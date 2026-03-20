use serde::{Deserialize, Serialize};

use crate::config::GatewayConfig;

/// Application-wide shared state passed to every handler via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
    pub redis_pool: deadpool_redis::Pool,
    pub db_pool: sqlx::PgPool,
    pub config: GatewayConfig,
}

/// Trace context propagated through every request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub session_id: String,
    pub parent_trace_id: Option<String>,
}

/// Typed errors for the gateway.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Upstream LLM API unreachable: {0}")]
    UpstreamUnreachable(#[from] reqwest::Error),

    #[error("Failed to build proxy response: {0}")]
    ResponseBuild(String),

    #[error("Failed to execute internal proxy request: {0}")]
    Proxy(reqwest::Error),

    #[error("Invalid gateway request: {0}")]
    InvalidRequest(String),

    #[error("Provider routing failed: {0}")]
    Routing(String),

    #[error("Provider circuit breaker is open: {0}")]
    CircuitOpen(String),
}

impl axum::response::IntoResponse for GatewayError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;

        let (status, message) = match &self {
            GatewayError::UpstreamUnreachable(_) => (
                StatusCode::BAD_GATEWAY,
                "The upstream LLM service is currently unreachable.",
            ),
            GatewayError::ResponseBuild(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while building the response.",
            ),
            GatewayError::Proxy(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred while communicating with backend cluster services.",
            ),
            GatewayError::InvalidRequest(_) => (
                StatusCode::BAD_REQUEST,
                "The gateway request was invalid.",
            ),
            GatewayError::Routing(_) => (
                StatusCode::BAD_REQUEST,
                "The requested model or provider is not available for this tenant.",
            ),
            GatewayError::CircuitOpen(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "The upstream provider is temporarily unavailable. Please retry shortly.",
            ),
        };

        tracing::error!(error = %self, "Returning error response to client");

        let body = Json(serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
