use reqwest::Client;
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    domain::{models::GatewayError, provider::ProviderKind},
    infrastructure::provider_client::{normalize_openai_message_response, ProviderForwardResult, ProviderResponseBody},
};

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";

pub async fn forward_chat_completion(
    client: &Client,
    api_key: &str,
    body: &Value,
) -> Result<ProviderForwardResult, GatewayError> {
    if body.get("stream").and_then(|value| value.as_bool()).unwrap_or(false) {
        return Err(GatewayError::Routing("Anthropic streaming is not implemented yet".to_string()));
    }

    let model = body
        .get("model")
        .and_then(|value| value.as_str())
        .ok_or_else(|| GatewayError::InvalidRequest("model is required".to_string()))?;

    let payload = json!({
        "model": model,
        "max_tokens": body.get("max_tokens").and_then(|value| value.as_u64()).unwrap_or(1024),
        "temperature": body.get("temperature").and_then(|value| value.as_f64()).unwrap_or(0.7),
        "messages": to_anthropic_messages(body),
    });

    info!(model, "Forwarding request to Anthropic");

    let response = client
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|error| {
            error!(error = %error, "Failed to reach Anthropic upstream");
            GatewayError::UpstreamUnreachable(error)
        })?;

    let status = response.status().as_u16();
    let response_json: Value = response.json().await.map_err(|error| {
        error!(error = %error, "Failed to parse Anthropic response");
        GatewayError::ResponseBuild(error.to_string())
    })?;

    if status >= 400 {
        return Ok(error_result(status, response_json));
    }

    let content = response_json
        .get("content")
        .and_then(|value| value.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let prompt_tokens = response_json
        .get("usage")
        .and_then(|value| value.get("input_tokens"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let completion_tokens = response_json
        .get("usage")
        .and_then(|value| value.get("output_tokens"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;

    let normalized = normalize_openai_message_response(
        ProviderKind::Anthropic,
        model,
        content,
        prompt_tokens,
        completion_tokens,
    );

    Ok(ProviderForwardResult {
        status,
        content_type: "application/json".to_string(),
        body: ProviderResponseBody::Buffered(normalized),
    })
}

fn to_anthropic_messages(body: &Value) -> Vec<Value> {
    body.get("messages")
        .and_then(|value| value.as_array())
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| {
                    let role = message.get("role")?.as_str()?;
                    let content = extract_text_content(message.get("content")?)?;
                    Some(json!({
                        "role": role,
                        "content": [{ "type": "text", "text": content }]
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_text_content(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }

    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(|item| item.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn error_result(status: u16, response_json: Value) -> ProviderForwardResult {
    ProviderForwardResult {
        status,
        content_type: "application/json".to_string(),
        body: ProviderResponseBody::Buffered(serde_json::to_vec(&response_json).unwrap_or_default()),
    }
}
