use reqwest::Client;
use serde_json::{json, Value};
use tracing::{error, info};

use crate::{
    domain::{models::GatewayError, provider::ProviderKind},
    infrastructure::provider_client::{normalize_openai_message_response, ProviderForwardResult, ProviderResponseBody},
};

pub async fn forward_chat_completion(
    client: &Client,
    api_key: &str,
    body: &Value,
) -> Result<ProviderForwardResult, GatewayError> {
    if body.get("stream").and_then(|value| value.as_bool()).unwrap_or(false) {
        return Err(GatewayError::Routing("Gemini streaming is not implemented yet".to_string()));
    }

    let model = body
        .get("model")
        .and_then(|value| value.as_str())
        .ok_or_else(|| GatewayError::InvalidRequest("model is required".to_string()))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}"
    );

    let payload = json!({
        "contents": to_gemini_contents(body),
        "generationConfig": {
            "temperature": body.get("temperature").and_then(|value| value.as_f64()).unwrap_or(0.7),
            "maxOutputTokens": body.get("max_tokens").and_then(|value| value.as_u64()).unwrap_or(1024),
        }
    });

    info!(model, "Forwarding request to Gemini");

    let response = client
        .post(url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|error| {
            error!(error = %error, "Failed to reach Gemini upstream");
            GatewayError::UpstreamUnreachable(error)
        })?;

    let status = response.status().as_u16();
    let response_json: Value = response.json().await.map_err(|error| {
        error!(error = %error, "Failed to parse Gemini response");
        GatewayError::ResponseBuild(error.to_string())
    })?;

    if status >= 400 {
        return Ok(ProviderForwardResult {
            status,
            content_type: "application/json".to_string(),
            body: ProviderResponseBody::Buffered(serde_json::to_vec(&response_json).unwrap_or_default()),
        });
    }

    let content = response_json
        .get("candidates")
        .and_then(|value| value.as_array())
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.as_array())
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    let prompt_tokens = response_json
        .get("usageMetadata")
        .and_then(|value| value.get("promptTokenCount"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let completion_tokens = response_json
        .get("usageMetadata")
        .and_then(|value| value.get("candidatesTokenCount"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;

    let normalized = normalize_openai_message_response(
        ProviderKind::Gemini,
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

fn to_gemini_contents(body: &Value) -> Vec<Value> {
    body.get("messages")
        .and_then(|value| value.as_array())
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| {
                    let role = match message.get("role")?.as_str()? {
                        "assistant" => "model",
                        _ => "user",
                    };
                    let content = extract_text_content(message.get("content")?)?;
                    Some(json!({
                        "role": role,
                        "parts": [{ "text": content }]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{extract_text_content, to_gemini_contents};

    #[test]
    fn gemini_message_conversion_maps_assistant_to_model() {
        let body = json!({
            "messages": [
                {"role": "user", "content": "hello"},
                {"role": "assistant", "content": "world"}
            ]
        });

        let contents = to_gemini_contents(&body);
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
    }

    #[test]
    fn gemini_extract_text_content_supports_string_payloads() {
        let content = extract_text_content(&json!("plain text")).unwrap();
        assert_eq!(content, "plain text");
    }
}
