use std::env;

use crate::domain::provider::ProviderKind;

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub service_urls: ServiceUrls,
    pub rate_limit: RateLimitConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub providers: ProviderRegistry,
}

#[derive(Debug, Clone)]
pub struct ServiceUrls {
    pub cache_url: String,
    pub clickhouse_url: String,
    pub tracer_url: String,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_secs: u32,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub open_window_secs: u64,
}

#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    pub default_provider: ProviderKind,
    pub openai_api_key: String,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            port: required_env_parse("GATEWAY_PORT", 8080u16, "GATEWAY_PORT must be a valid port number")?,
            database_url: required_env("DATABASE_URL")?,
            redis_url: required_env("REDIS_URL")?,
            service_urls: ServiceUrls {
                cache_url: env::var("CACHE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
                clickhouse_url: required_env("CLICKHOUSE_URL")?,
                tracer_url: env::var("TRACER_URL").unwrap_or_else(|_| "http://localhost:8082".to_string()),
            },
            rate_limit: RateLimitConfig {
                max_requests: required_env_parse("RATE_LIMIT_MAX_REQUESTS", 60u32, "RATE_LIMIT_MAX_REQUESTS must be a valid integer")?,
                window_secs: required_env_parse("RATE_LIMIT_WINDOW_SECS", 60u32, "RATE_LIMIT_WINDOW_SECS must be a valid integer")?,
            },
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: required_env_parse("CIRCUIT_BREAKER_FAILURE_THRESHOLD", 3u32, "CIRCUIT_BREAKER_FAILURE_THRESHOLD must be a valid integer")?,
                open_window_secs: required_env_parse("CIRCUIT_BREAKER_OPEN_WINDOW_SECS", 30u64, "CIRCUIT_BREAKER_OPEN_WINDOW_SECS must be a valid integer")?,
            },
            providers: ProviderRegistry {
                default_provider: ProviderKind::OpenAi,
                openai_api_key: required_env("OPENAI_API_KEY")?,
                anthropic_api_key: optional_env("ANTHROPIC_API_KEY"),
                gemini_api_key: optional_env("GEMINI_API_KEY"),
            },
        })
    }
}

fn required_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("{key} environment variable not set"))
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn required_env_parse<T>(key: &str, default: T, error_message: &str) -> Result<T, String>
where
    T: std::str::FromStr + ToString,
{
    env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .parse()
        .map_err(|_| error_message.to_string())
}
