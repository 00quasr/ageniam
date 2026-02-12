use crate::errors::{AppError, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub audit: AuditConfig,
    pub crypto: CryptoConfig,
    pub observability: ObservabilityConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
    pub connection_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_expiration_seconds: i64,
    pub refresh_token_expiration_seconds: i64,
    pub biscuit_root_key_id: String,
    pub password_min_length: usize,
    pub password_require_uppercase: bool,
    pub password_require_lowercase: bool,
    pub password_require_digit: bool,
    pub password_require_special: bool,
    pub max_login_attempts: u32,
    pub lockout_duration_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub default_requests_per_minute: u64,
    pub default_requests_per_hour: u64,
    pub default_requests_per_day: u64,
    pub auth_requests_per_minute: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub async_batch_size: usize,
    pub async_flush_interval_seconds: u64,
    pub storage_backends: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CryptoConfig {
    pub key_rotation_days: u32,
    pub key_overlap_days: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub log_format: String,
    pub metrics_enabled: bool,
    pub tracing_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub tls_enabled: bool,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub cors_enabled: bool,
    pub cors_allowed_origins: Vec<String>,
    pub cors_allowed_methods: Vec<String>,
    pub cors_allowed_headers: Vec<String>,
    pub cors_max_age_seconds: usize,
}

impl Config {
    /// Load configuration from files and environment variables
    pub fn load() -> Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        // Determine environment
        let environment = env::var("AGENT_IAM_ENV").unwrap_or_else(|_| "development".to_string());

        // Build configuration
        let config = config::Config::builder()
            // Start with default config
            .add_source(config::File::with_name("config/default"))
            // Add environment-specific config
            .add_source(
                config::File::with_name(&format!("config/{}", environment)).required(false),
            )
            // Add environment variables with prefix AGENT_IAM
            // e.g., AGENT_IAM__SERVER__PORT=8080
            .add_source(
                config::Environment::with_prefix("AGENT_IAM")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| AppError::Configuration(e.to_string()))?;

        // Deserialize into our Config struct
        config
            .try_deserialize()
            .map_err(|e| AppError::Configuration(e.to_string()))
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(AppError::Configuration("Invalid port number".to_string()));
        }

        // Validate database config
        if self.database.url.is_empty() {
            return Err(AppError::Configuration(
                "Database URL is required".to_string(),
            ));
        }

        // Validate Redis config
        if self.redis.url.is_empty() {
            return Err(AppError::Configuration(
                "Redis URL is required".to_string(),
            ));
        }

        // Validate auth config
        if self.auth.password_min_length < 8 {
            return Err(AppError::Configuration(
                "Password min length must be at least 8".to_string(),
            ));
        }

        // Validate TLS config
        if self.security.tls_enabled {
            if self.security.tls_cert_path.is_empty() || self.security.tls_key_path.is_empty() {
                return Err(AppError::Configuration(
                    "TLS cert and key paths are required when TLS is enabled".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = Config::load().expect("Failed to load config");
        assert!(config.validate().is_ok());

        // Test invalid port
        config.server.port = 0;
        assert!(config.validate().is_err());
    }
}
