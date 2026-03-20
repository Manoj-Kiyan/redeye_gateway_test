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

    let mut attempt = 0;
    let response = loop {
        let request = client
            .post(OPENAI_CHAT_URL)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .header("Accept", accept_header)
            .json(body);

        match request.send().await {
            Ok(response) => break response,
            Err(error) if attempt < 2 && is_retryable_transport_error(&error) => {
                attempt += 1;
                tracing::warn!(attempt, error = %error, "Transient OpenAI transport error; retrying");
                tokio::time::sleep(std::time::Duration::from_millis(150 * attempt)).await;
            }
            Err(error) => {
                error!(error = %error, "Failed to reach OpenAI upstream");
                return Err(GatewayError::UpstreamUnreachable(error));
            }
        }
    };

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

fn is_retryable_transport_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout() || error.is_request()
}

#[cfg(test)]
mod tests {
    use super::is_retryable_transport_error;

    #[test]
    fn retry_helper_compiles_and_is_callable() {
        let _ = is_retryable_transport_error;
    }
}
