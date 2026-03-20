//! usecases/proxy.rs - Core proxy orchestration logic.
//! This is the heart of the gateway: cache check -> upstream call -> async telemetry.

use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

use crate::domain::{models::{AppState, GatewayError, TraceContext}, routing::RoutingDecision};
use crate::infrastructure::{cache_client, clickhouse_logger, provider_client};
use crate::infrastructure::provider_client::ProviderResponseBody;

pub enum ProxyBody {
    Buffered(Vec<u8>),
    Stream(reqwest::Response),
}

pub struct ProxyResult {
    pub status: u16,
    pub content_type: String,
    pub body: ProxyBody,
    pub cache_hit: bool,
}

pub async fn execute_proxy(
    state: &Arc<AppState>,
    body: &Value,
    routing: &RoutingDecision,
    raw_prompt: &str,
    accept_header: &str,
    trace_ctx: &TraceContext,
) -> Result<ProxyResult, GatewayError> {
    let start_time = std::time::Instant::now();

    if let Some(cached_content) = cache_client::lookup_cache(
        &state.http_client,
        &state.config.service_urls.cache_url,
        &routing.tenant_id,
        &routing.effective_model,
        raw_prompt,
    )
    .await
    {
        let mock_response = json!({
            "id": "chatcmpl-cached",
            "object": "chat.completion",
            "created": 0,
            "model": routing.effective_model,
            "choices": [{"index": 0, "message": {"role": "assistant", "content": cached_content}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });

        let bytes = serde_json::to_vec(&mock_response).unwrap_or_default();

        fire_async_telemetry(
            state,
            &routing.tenant_id,
            &routing.effective_model,
            raw_prompt,
            trace_ctx,
            200,
            start_time.elapsed().as_millis() as u32,
            0,
            true,
            None,
        );

        return Ok(ProxyResult {
            status: 200,
            content_type: "application/json".to_string(),
            body: ProxyBody::Buffered(bytes),
            cache_hit: true,
        });
    }

    let upstream_response = provider_client::forward_chat_completion(
        &state.http_client,
        &state.config.providers,
        routing.provider,
        &routing.upstream_api_key,
        body,
        accept_header,
    )
    .await?;

    info!(provider = routing.provider.as_str(), status = upstream_response.status, "Received upstream response");

    match upstream_response.body {
        ProviderResponseBody::Stream(response) => handle_streaming_or_buffered_openai(
            state,
            body,
            routing,
            raw_prompt,
            trace_ctx,
            upstream_response.status,
            upstream_response.content_type,
            response,
            start_time,
        ).await,
        ProviderResponseBody::Buffered(body_bytes) => handle_buffered_response(
            state,
            routing,
            raw_prompt,
            trace_ctx,
            upstream_response.status,
            upstream_response.content_type,
            body_bytes,
            start_time,
        ),
    }
}

async fn handle_streaming_or_buffered_openai(
    state: &Arc<AppState>,
    body: &Value,
    routing: &RoutingDecision,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    status: u16,
    content_type: String,
    response: reqwest::Response,
    start_time: std::time::Instant,
) -> Result<ProxyResult, GatewayError> {
    let is_streaming = body
        .get("stream")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if is_streaming {
        let latency_ms = start_time.elapsed().as_millis() as u32;

        fire_async_telemetry(
            state,
            &routing.tenant_id,
            &routing.effective_model,
            raw_prompt,
            trace_ctx,
            status,
            latency_ms,
            0,
            false,
            None,
        );

        return Ok(ProxyResult {
            status,
            content_type,
            body: ProxyBody::Stream(response),
            cache_hit: false,
        });
    }

    let body_bytes = response.bytes().await.unwrap_or_default().to_vec();
    Ok(finalize_buffered_response(
        state,
        routing,
        raw_prompt,
        trace_ctx,
        status,
        content_type,
        body_bytes,
        start_time,
    ))
}

fn handle_buffered_response(
    state: &Arc<AppState>,
    routing: &RoutingDecision,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    status: u16,
    content_type: String,
    body_bytes: Vec<u8>,
    start_time: std::time::Instant,
) -> Result<ProxyResult, GatewayError> {
    Ok(finalize_buffered_response(
        state,
        routing,
        raw_prompt,
        trace_ctx,
        status,
        content_type,
        body_bytes,
        start_time,
    ))
}

fn finalize_buffered_response(
    state: &Arc<AppState>,
    routing: &RoutingDecision,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    status: u16,
    content_type: String,
    body_bytes: Vec<u8>,
    start_time: std::time::Instant,
) -> ProxyResult {
    let latency_ms = start_time.elapsed().as_millis() as u32;

    let tokens = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|value| value["usage"]["total_tokens"].as_u64())
        .unwrap_or(0) as u32;

    fire_async_telemetry(
        state,
        &routing.tenant_id,
        &routing.effective_model,
        raw_prompt,
        trace_ctx,
        status,
        latency_ms,
        tokens,
        false,
        Some(body_bytes.clone()),
    );

    ProxyResult {
        status,
        content_type,
        body: ProxyBody::Buffered(body_bytes),
        cache_hit: false,
    }
}

fn fire_async_telemetry(
    state: &Arc<AppState>,
    tenant_id: &str,
    model_name: &str,
    raw_prompt: &str,
    trace_ctx: &TraceContext,
    status_code: u16,
    latency_ms: u32,
    tokens: u32,
    cache_hit: bool,
    response_bytes: Option<Vec<u8>>,
) {
    let state = state.clone();
    let tenant_id = tenant_id.to_string();
    let model_name = model_name.to_string();
    let raw_prompt = raw_prompt.to_string();
    let trace_ctx = trace_ctx.clone();

    tokio::spawn(async move {
        clickhouse_logger::log_request(
            &state.http_client,
            &state.config.service_urls.clickhouse_url,
            &tenant_id,
            &trace_ctx.trace_id,
            &trace_ctx.session_id,
            status_code,
            latency_ms,
            &model_name,
            tokens,
            cache_hit,
        )
        .await;

        let response_content = response_bytes
            .as_ref()
            .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok())
            .and_then(|value| value["choices"][0]["message"]["content"].as_str().map(str::to_string))
            .unwrap_or_default();

        let trace_payload = json!({
            "trace_id": trace_ctx.trace_id,
            "session_id": trace_ctx.session_id,
            "parent_trace_id": trace_ctx.parent_trace_id,
            "tenant_id": tenant_id,
            "model": model_name,
            "status": status_code,
            "latency_ms": latency_ms,
            "total_tokens": tokens,
            "cache_hit": cache_hit,
            "prompt_content": raw_prompt,
            "response_content": response_content
        });

        clickhouse_logger::send_trace_to_tracer(
            &state.http_client,
            &state.config.service_urls.tracer_url,
            &trace_payload,
        )
        .await;

        if !cache_hit && status_code == 200 && !response_content.is_empty() {
            cache_client::store_in_cache(
                &state.http_client,
                &state.config.service_urls.cache_url,
                &tenant_id,
                &model_name,
                &raw_prompt,
                &response_content,
            )
            .await;
        }
    });
}
