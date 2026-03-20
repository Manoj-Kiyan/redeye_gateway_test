use serde_json::{json, Value};

use crate::{
    config::ProviderRegistry,
    domain::{models::GatewayError, provider::ProviderKind},
    infrastructure::{anthropic_client, gemini_client, openai_client},
};

pub enum ProviderResponseBody {
    Buffered(Vec<u8>),
    Stream(reqwest::Response),
}

pub struct ProviderForwardResult {
    pub status: u16,
    pub content_type: String,
    pub body: ProviderResponseBody,
}

pub async fn forward_chat_completion(
    client: &reqwest::Client,
    providers: &ProviderRegistry,
    provider: ProviderKind,
    api_key: &str,
    body: &Value,
    accept_header: &str,
) -> Result<ProviderForwardResult, GatewayError> {
    match provider {
        ProviderKind::OpenAi => openai_client::forward_chat_completion(client, api_key, body, accept_header).await,
        ProviderKind::Anthropic => {
            let anthropic_api_key = if api_key.trim().is_empty() {
                providers
                    .anthropic_api_key
                    .as_deref()
                    .ok_or_else(|| GatewayError::Routing("ANTHROPIC_API_KEY is not configured".to_string()))?
            } else {
                api_key
            };

            anthropic_client::forward_chat_completion(client, anthropic_api_key, body).await
        }
        ProviderKind::Gemini => {
            let gemini_api_key = if api_key.trim().is_empty() {
                providers
                    .gemini_api_key
                    .as_deref()
                    .ok_or_else(|| GatewayError::Routing("GEMINI_API_KEY is not configured".to_string()))?
            } else {
                api_key
            };

            gemini_client::forward_chat_completion(client, gemini_api_key, body).await
        }
    }
}

pub fn normalize_openai_message_response(
    provider: ProviderKind,
    model: &str,
    message_content: String,
    prompt_tokens: u32,
    completion_tokens: u32,
) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "id": format!("{}-normalized", provider.as_str()),
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": message_content,
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens
        }
    }))
    .unwrap_or_default()
}
