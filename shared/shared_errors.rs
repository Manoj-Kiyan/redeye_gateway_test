//! Unified error handling framework for all RedEye microservices
//! Implements error context, correlation IDs, and standardized HTTP responses

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Structured error context propagated across all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Unique request/trace identifier for correlation
    pub correlation_id: String,
    /// Session ID for user context
    pub session_id: Option<String>,
    /// Tenant ID for multi-tenancy
    pub tenant_id: Option<String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            correlation_id: uuid::Uuid::new_v4().to_string(),
            session_id: None,
            tenant_id: None,
        }
    }
}

/// Unified error type for all RedEye services
#[derive(Error, Debug)]
pub enum AppError {
    /// Invalid request input (400)
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Missing authentication credentials (401)
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Insufficient permissions (403)
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Resource not found (404)
    #[error("Not found: {0}")]
    NotFound(String),

    /// Request validation/conflict (409)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Rate limit exceeded (429)
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    /// Upstream service error (502)
    #[error("Upstream service unavailable: {0}")]
    UpstreamError(String),

    /// Database error (500)
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Redis/cache error (500)
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Configuration error (500)
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Internal server error (500)
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::UpstreamError(_) => StatusCode::BAD_GATEWAY,
            Self::DatabaseError(_) | Self::CacheError(_) | Self::ConfigError(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Create error with additional context
    pub fn with_context(self, context: &ErrorContext) -> Self {
        // Log the error with context
        tracing::error!(
            error = %self,
            correlation_id = %context.correlation_id,
            session_id = ?context.session_id,
            tenant_id = ?context.tenant_id,
        );
        self
    }
}

/// API error response structure
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// Error code (machine-readable)
    pub code: String,
    /// Correlation ID for debugging
    pub correlation_id: String,
    /// Optional hint for retry-ability
    pub retry_after: Option<u64>,
}

impl ErrorResponse {
    pub fn new(error: &AppError, correlation_id: String) -> Self {
        let code = match error {
            AppError::BadRequest(_) => "INVALID_REQUEST",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Conflict(_) => "CONFLICT",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::UpstreamError(_) => "UPSTREAM_ERROR",
            AppError::DatabaseError(_) => "DATABASE_ERROR",
            AppError::CacheError(_) => "CACHE_ERROR",
            AppError::ConfigError(_) => "CONFIG_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        };

        let retry_after = match error {
            AppError::RateLimited(_) => Some(60), // Retry after 60 seconds
            AppError::UpstreamError(_) => Some(5), // Retry after 5 seconds
            _ => None,
        };

        Self {
            error: error.to_string(),
            code: code.to_string(),
            correlation_id,
            retry_after,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let correlation_id = uuid::Uuid::new_v4().to_string();
        let status = self.status_code();
        let body = ErrorResponse::new(&self, correlation_id);

        (status, Json(body)).into_response()
    }
}

/// Trait for converting external errors into AppError
pub trait IntoAppError<T> {
    fn map_app_err(self, msg: &str) -> Result<T, AppError>;
}

// SQLx database errors
impl<T> IntoAppError<T> for Result<T, sqlx::Error> {
    fn map_app_err(self, msg: &str) -> Result<T, AppError> {
        self.map_err(|e| {
            tracing::error!("Database error: {}", e);
            AppError::DatabaseError(format!("{}: {}", msg, e))
        })
    }
}

// Redis errors
impl<T> IntoAppError<T> for Result<T, redis::RedisError> {
    fn map_app_err(self, msg: &str) -> Result<T, AppError> {
        self.map_err(|e| {
            tracing::error!("Redis error: {}", e);
            AppError::CacheError(format!("{}: {}", msg, e))
        })
    }
}

// Reqwest HTTP client errors
impl<T> IntoAppError<T> for Result<T, reqwest::Error> {
    fn map_app_err(self, msg: &str) -> Result<T, AppError> {
        self.map_err(|e| {
            tracing::error!("HTTP error: {}", e);
            if e.is_timeout() || e.is_connect() {
                AppError::UpstreamError(format!("{}: connection failed", msg))
            } else {
                AppError::UpstreamError(format!("{}: {}", msg, e))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AppError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized("test".to_string()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::RateLimited("test".to_string()).status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_error_response_structure() {
        let err = AppError::NotFound("user".to_string());
        let resp = ErrorResponse::new(&err, "test-id".to_string());
        assert_eq!(resp.code, "NOT_FOUND");
        assert_eq!(resp.correlation_id, "test-id");
    }
}
