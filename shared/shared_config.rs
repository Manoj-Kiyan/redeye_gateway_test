//! Configuration framework for RedEye microservices
//! Supports environment-based configuration with validation and defaults

use serde::{Deserialize, Serialize};
use std::env;

/// Represents configuration loading errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingVariable(String),
    #[error("Invalid value for {0}: {1}")]
    InvalidValue(String, String),
    #[error("Configuration error: {0}")]
    Other(String),
}

/// Trait for loading and validating configuration
pub trait LoadConfig: Sized {
    fn load() -> Result<Self, ConfigError>;
}

/// Global application configuration (shared across all services)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Service-specific configuration
    pub service: ServiceConfig,
    /// Database connection settings
    pub database: DatabaseConfig,
    /// Cache/Redis settings
    pub cache: CacheConfig,
    /// Logging settings
    pub logging: LoggingConfig,
    /// Security settings
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name (for distributed tracing)
    pub name: String,
    /// Service port
    pub port: u16,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Enable debug logging
    pub debug: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,
    /// Maximum number of connections in pool
    pub pool_size: u32,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Enable SSL/TLS
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Redis connection URL
    pub url: Option<String>,
    /// Redis connection timeout in seconds
    pub timeout: u64,
    /// TTL for cached items in seconds
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable JSON logging
    pub json: bool,
    /// Enable console output
    pub console: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret (minimum 32 bytes)
    pub jwt_secret: String,
    /// AES encryption key (32 bytes)
    pub aes_key: String,
    /// API key salt
    pub api_key_salt: String,
}

impl SecurityConfig {
    /// Validate security configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.jwt_secret.len() < 32 {
            return Err(ConfigError::InvalidValue(
                "JWT_SECRET".to_string(),
                "must be at least 32 bytes".to_string(),
            ));
        }

        if self.aes_key.len() != 32 {
            return Err(ConfigError::InvalidValue(
                "AES_MASTER_KEY".to_string(),
                "must be exactly 32 bytes".to_string(),
            ));
        }

        if self.api_key_salt.is_empty() {
            return Err(ConfigError::InvalidValue(
                "API_KEY_SALT".to_string(),
                "must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

impl LoadConfig for AppConfig {
    fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        // Service configuration
        let service = ServiceConfig {
            name: env_var("SERVICE_NAME")?,
            port: env_var_parsed("SERVICE_PORT", 8080),
            environment: env_var_default("ENVIRONMENT", "development"),
            debug: env_var_bool("DEBUG", false),
        };

        // Database configuration
        let database = DatabaseConfig {
            url: env_var("DATABASE_URL")?,
            pool_size: env_var_parsed("DB_POOL_SIZE", 10),
            timeout: env_var_parsed("DB_TIMEOUT", 30),
            use_tls: env_var_bool("DB_USE_TLS", matches!(service.environment.as_str(), "production")),
        };

        // Cache configuration
        let cache = CacheConfig {
            url: Ok(env_var("REDIS_URL")).ok(),
            timeout: env_var_parsed("REDIS_TIMEOUT", 5),
            ttl: env_var_parsed("CACHE_TTL", 3600),
        };

        // Logging configuration
        let logging = LoggingConfig {
            level: env_var_default("RUST_LOG", "info"),
            json: env_var_bool("LOG_JSON", matches!(service.environment.as_str(), "production")),
            console: env_var_bool("LOG_CONSOLE", true),
        };

        // Security configuration
        let security = SecurityConfig {
            jwt_secret: env_var("JWT_SECRET")?,
            aes_key: env_var("AES_MASTER_KEY")?,
            api_key_salt: env_var_default("API_KEY_SALT", "redeye-default-salt"),
        };

        // Validate critical settings
        security.validate()?;

        Ok(Self {
            service,
            database,
            cache,
            logging,
            security,
        })
    }
}

// Helper functions for environment variable loading

fn env_var(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingVariable(key.to_string()))
}

fn env_var_default(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_var_parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_var_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_validation() {
        let config = SecurityConfig {
            jwt_secret: "x".repeat(32),
            aes_key: "x".repeat(32),
            api_key_salt: "salt".to_string(),
        };
        assert!(config.validate().is_ok());

        let invalid = SecurityConfig {
            jwt_secret: "short".to_string(),
            aes_key: "x".repeat(32),
            api_key_salt: "salt".to_string(),
        };
        assert!(invalid.validate().is_err());
    }
}
