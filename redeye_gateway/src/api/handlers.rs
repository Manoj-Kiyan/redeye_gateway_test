//! api/handlers.rs - Thin Axum handlers that extract, delegate to use cases, and respond.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Extension, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use tracing::{error, info, instrument};

use crate::domain::{models::{AppState, GatewayError, TraceContext}, provider::ProviderKind, routing::TenantRouteConfig};
use crate::infrastructure::{audit_repository, routing_repository};
use crate::usecases::{proxy, routing};

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRoutesPayload {
    pub routes: Vec<UpdateTenantRouteItem>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRouteItem {
    pub provider: String,
    pub model: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct TenantRoutesResponse {
    pub tenant_id: String,
    pub routes: Vec<TenantRouteConfig>,
}

#[derive(Debug, Serialize)]
pub struct ProviderCatalogEntry {
    pub provider: String,
    pub suggested_models: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderCatalogResponse {
    pub providers: Vec<ProviderCatalogEntry>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub tenant_id: String,
    pub entries: Vec<audit_repository::AuditLogEntry>,
}

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "redeye_gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[instrument(skip(state, body))]
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(trace_ctx): Extension<TraceContext>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, GatewayError> {
    info!("Received chat completion request");

    let routing = routing::resolve_routing_decision(state.as_ref(), &headers, &body).await?;
    let raw_prompt = serde_json::to_string(&body).unwrap_or_default();
    let accept = headers
        .get("accept")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json");

    let result = proxy::execute_proxy(&state, &body, &routing, &raw_prompt, accept, &trace_ctx).await?;

    let cache_header = if result.cache_hit { "HIT" } else { "MISS" };

    match result.body {
        proxy::ProxyBody::Buffered(body_bytes) => {
            let response = Response::builder()
                .status(result.status)
                .header("content-type", &result.content_type)
                .header("X-Cache", cache_header)
                .body(Body::from(body_bytes))
                .map_err(|build_error| {
                    error!(error = %build_error, "Failed to construct proxy response");
                    GatewayError::ResponseBuild(build_error.to_string())
                })?;

            Ok(response)
        }
        proxy::ProxyBody::Stream(upstream_response) => {
            let stream = upstream_response.bytes_stream();

            let response = Response::builder()
                .status(result.status)
                .header("content-type", &result.content_type)
                .header("X-Cache", cache_header)
                .body(Body::from_stream(stream))
                .map_err(|build_error| {
                    error!(error = %build_error, "Failed to construct streaming proxy response");
                    GatewayError::ResponseBuild(build_error.to_string())
                })?;

            Ok(response)
        }
    }
}

#[instrument(skip(state))]
pub async fn admin_metrics(State(state): State<Arc<AppState>>) -> Result<Json<Value>, GatewayError> {
    info!("Fetching live metrics from ClickHouse");

    let query = "
        SELECT 
            count() as total_requests,
            avg(latency_ms) as avg_latency_ms,
            sum(tokens) as total_tokens,
            countIf(status = 429) as rate_limited_requests
        FROM RedEye_telemetry.request_logs
        FORMAT JSON
    ";

    let response = state
        .http_client
        .post(&state.config.service_urls.clickhouse_url)
        .body(query)
        .send()
        .await
        .map_err(|error| {
            error!(error = %error, "ClickHouse metrics failed");
            GatewayError::Proxy(error)
        })?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!(error = %error_text, "ClickHouse metrics query failed");
        return Err(GatewayError::ResponseBuild("Metrics query failed".to_string()));
    }

    let mut clickhouse_json: Value = response.json().await.map_err(|error| {
        error!(error = %error, "Failed to parse ClickHouse JSON");
        GatewayError::ResponseBuild(error.to_string())
    })?;

    let row = clickhouse_json
        .get_mut("data")
        .and_then(|data| data.as_array_mut())
        .and_then(|rows| rows.pop())
        .unwrap_or_else(|| {
            json!({
                "total_requests": "0",
                "avg_latency_ms": 0.0,
                "total_tokens": "0",
                "rate_limited_requests": "0"
            })
        });

    Ok(Json(row))
}

pub async fn get_tenant_routes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<TenantRoutesResponse>, GatewayError> {
    let tenant_id = extract_tenant_id(&headers)?;
    let routes = routing_repository::fetch_tenant_routes(&state.db_pool, &tenant_id).await?;

    Ok(Json(TenantRoutesResponse {
        tenant_id,
        routes: routes.into_iter().map(|route| TenantRouteConfig {
            provider: route.provider,
            model: route.requested_model,
            is_default: route.is_default,
        }).collect(),
    }))
}

pub async fn get_provider_catalog() -> Json<ProviderCatalogResponse> {
    let providers = [ProviderKind::OpenAi, ProviderKind::Anthropic, ProviderKind::Gemini]
        .into_iter()
        .map(|provider| ProviderCatalogEntry {
            provider: provider.as_str().to_string(),
            suggested_models: provider.catalog_models().iter().map(|model| (*model).to_string()).collect(),
        })
        .collect();

    Json(ProviderCatalogResponse { providers })
}

pub async fn get_tenant_audit_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AuditLogResponse>, GatewayError> {
    let tenant_id = extract_tenant_id(&headers)?;
    let entries = audit_repository::fetch_tenant_audit_logs(&state.db_pool, &tenant_id, 25).await?;

    Ok(Json(AuditLogResponse { tenant_id, entries }))
}

pub async fn update_tenant_routes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateTenantRoutesPayload>,
) -> Result<Json<TenantRoutesResponse>, GatewayError> {
    let tenant_id = extract_tenant_id(&headers)?;
    let actor_user_id = extract_optional_user_id(&headers);
    validate_route_payload(&payload)?;

    let next_routes = payload.routes.into_iter().map(|route| {
        let provider = ProviderKind::from_db_value(&route.provider)
            .ok_or_else(|| GatewayError::InvalidRequest(format!("unsupported provider '{}'", route.provider)))?;

        if !provider.supports_model(&route.model) {
            return Err(GatewayError::InvalidRequest(format!(
                "model '{}' is not compatible with provider '{}'",
                route.model,
                provider.as_str()
            )));
        }

        Ok(TenantRouteConfig {
            provider,
            model: route.model,
            is_default: route.is_default,
        })
    }).collect::<Result<Vec<_>, GatewayError>>()?;

    let routes = routing_repository::replace_tenant_routes(&state.db_pool, &tenant_id, &next_routes).await?;
    audit_repository::insert_audit_log(
        &state.db_pool,
        &tenant_id,
        actor_user_id.as_deref(),
        "gateway",
        "tenant_routes_updated",
        "llm_routes",
        json!({
            "route_count": routes.len(),
            "routes": routes.iter().map(|route| {
                json!({
                    "provider": route.provider.as_str(),
                    "model": route.requested_model,
                    "is_default": route.is_default,
                })
            }).collect::<Vec<_>>(),
        }),
    ).await?;

    Ok(Json(TenantRoutesResponse {
        tenant_id,
        routes: routes.into_iter().map(|route| TenantRouteConfig {
            provider: route.provider,
            model: route.requested_model,
            is_default: route.is_default,
        }).collect(),
    }))
}

fn extract_tenant_id(headers: &HeaderMap) -> Result<String, GatewayError> {
    headers
        .get("x-tenant-id")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
        .ok_or_else(|| GatewayError::InvalidRequest("tenant context is missing".to_string()))
}

fn extract_optional_user_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-user-id")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn validate_route_payload(payload: &UpdateTenantRoutesPayload) -> Result<(), GatewayError> {
    if payload.routes.is_empty() {
        return Err(GatewayError::InvalidRequest("at least one route is required".to_string()));
    }

    let default_count = payload.routes.iter().filter(|route| route.is_default).count();
    if default_count != 1 {
        return Err(GatewayError::InvalidRequest("exactly one route must be marked as default".to_string()));
    }

    if payload.routes.iter().any(|route| route.model.trim().is_empty()) {
        return Err(GatewayError::InvalidRequest("route model names cannot be empty".to_string()));
    }

    let mut seen = std::collections::HashSet::new();
    for route in &payload.routes {
        let key = format!("{}::{}", route.provider.trim(), route.model.trim().to_lowercase());
        if !seen.insert(key) {
            return Err(GatewayError::InvalidRequest("duplicate provider/model routes are not allowed".to_string()));
        }
    }

    Ok(())
}
