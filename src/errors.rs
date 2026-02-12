use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// Application-wide error type
#[derive(Debug)]
pub enum AppError {
    // Database errors
    Database(sqlx::Error),
    DatabaseMigration(sqlx::migrate::MigrateError),

    // Redis errors
    Redis(redis::RedisError),

    // Authentication errors
    InvalidCredentials,
    TokenGeneration(String),
    TokenValidation(String),
    TokenExpired,
    TokenRevoked,
    Unauthorized,

    // Authorization errors
    Forbidden,
    PolicyEvaluation(String),

    // Identity errors
    IdentityNotFound,
    IdentityAlreadyExists,
    InvalidIdentityType,

    // Session errors
    SessionNotFound,
    SessionExpired,

    // Rate limiting
    RateLimitExceeded,

    // Validation errors
    ValidationError(String),

    // Configuration errors
    Configuration(String),

    // Cryptographic errors
    Cryptographic(String),

    // Internal errors
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::DatabaseMigration(e) => write!(f, "Database migration error: {}", e),
            AppError::Redis(e) => write!(f, "Redis error: {}", e),
            AppError::InvalidCredentials => write!(f, "Invalid credentials"),
            AppError::TokenGeneration(msg) => write!(f, "Token generation failed: {}", msg),
            AppError::TokenValidation(msg) => write!(f, "Token validation failed: {}", msg),
            AppError::TokenExpired => write!(f, "Token has expired"),
            AppError::TokenRevoked => write!(f, "Token has been revoked"),
            AppError::Unauthorized => write!(f, "Unauthorized"),
            AppError::Forbidden => write!(f, "Forbidden"),
            AppError::PolicyEvaluation(msg) => write!(f, "Policy evaluation error: {}", msg),
            AppError::IdentityNotFound => write!(f, "Identity not found"),
            AppError::IdentityAlreadyExists => write!(f, "Identity already exists"),
            AppError::InvalidIdentityType => write!(f, "Invalid identity type"),
            AppError::SessionNotFound => write!(f, "Session not found"),
            AppError::SessionExpired => write!(f, "Session has expired"),
            AppError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            AppError::Cryptographic(msg) => write!(f, "Cryptographic error: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Convert from various error types
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        AppError::DatabaseMigration(err)
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Redis(err)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AppError::TokenExpired,
            ErrorKind::InvalidToken => AppError::TokenValidation("Invalid token".to_string()),
            _ => AppError::TokenValidation(err.to_string()),
        }
    }
}

// Implement IntoResponse for Axum
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Database(_) | AppError::DatabaseMigration(_) => {
                tracing::error!("Database error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Redis(_) => {
                tracing::error!("Redis error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AppError::TokenGeneration(_) => {
                tracing::error!("Token generation error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::TokenValidation(_) => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
            AppError::TokenRevoked => (StatusCode::UNAUTHORIZED, "Token revoked"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            AppError::PolicyEvaluation(_) => {
                tracing::error!("Policy evaluation error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::IdentityNotFound => (StatusCode::NOT_FOUND, "Identity not found"),
            AppError::IdentityAlreadyExists => (StatusCode::CONFLICT, "Identity already exists"),
            AppError::InvalidIdentityType => (StatusCode::BAD_REQUEST, "Invalid identity type"),
            AppError::SessionNotFound => (StatusCode::NOT_FOUND, "Session not found"),
            AppError::SessionExpired => (StatusCode::UNAUTHORIZED, "Session expired"),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string().as_str()),
            AppError::Configuration(_) => {
                tracing::error!("Configuration error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Cryptographic(_) => {
                tracing::error!("Cryptographic error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Internal(_) => {
                tracing::error!("Internal error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

/// Result type alias for the application
pub type Result<T> = std::result::Result<T, AppError>;
