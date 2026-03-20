use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Gemini => "gemini",
        }
    }

    pub fn supports_model(self, model: &str) -> bool {
        match self {
            ProviderKind::OpenAi => {
                model.starts_with("gpt-4")
                    || model.starts_with("gpt-5")
                    || model.starts_with("o1")
                    || model.starts_with("o3")
            }
            ProviderKind::Anthropic => model.starts_with("claude"),
            ProviderKind::Gemini => model.starts_with("gemini"),
        }
    }

    pub fn catalog_models(self) -> &'static [&'static str] {
        match self {
            ProviderKind::OpenAi => &["gpt-4o-mini", "gpt-4o", "gpt-5-mini", "gpt-5", "o1-mini", "o3-mini"],
            ProviderKind::Anthropic => &["claude-3-5-haiku-latest", "claude-3-5-sonnet-latest", "claude-sonnet-4-20250514"],
            ProviderKind::Gemini => &["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash"],
        }
    }

    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "openai" => Some(ProviderKind::OpenAi),
            "anthropic" => Some(ProviderKind::Anthropic),
            "gemini" => Some(ProviderKind::Gemini),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProviderCredentials {
    pub api_key: String,
}
