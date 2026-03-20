use reqwest::Client;
use serde_json::Value;
use tracing::{error, info};

use crate::{domain::models::GatewayError, infrastructure::provider_client::{ProviderForwardResult, ProviderResponseBody}};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

pub async fn forward_chat_completion(
    client: &Client,
    api_key: &str,
    body: &Value,
    accept_header: &str,
) -> Result<ProviderForwardResult, GatewayError> {
    info!("Forwarding request to OpenAI");

    let response = client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("Accept", accept_header)
        .json(body)
        .send()
        .await
        .map_err(|error| {
            error!(error = %error, "Failed to reach OpenAI upstream");
            GatewayError::UpstreamUnreachable(error)
        })?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    Ok(ProviderForwardResult {
        status,
        content_type,
        body: ProviderResponseBody::Stream(response),
    })
}
