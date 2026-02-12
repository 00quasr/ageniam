// Authentication endpoints

use crate::api::routes::AppState;
use crate::auth::{jwt::{JwtManager, TokenPair}, password};
use crate::errors::{AppError, Result};
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl From<TokenPair> for LoginResponse {
    fn from(pair: TokenPair) -> Self {
        Self {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            token_type: pair.token_type,
            expires_in: pair.expires_in,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /v1/auth/login
///
/// Authenticate a user with email and password
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
    tracing::info!("Login attempt for email: {}", req.email);

    // Validate input
    if req.email.is_empty() {
        return Err(AppError::ValidationError("Email is required".to_string()));
    }
    if req.password.is_empty() {
        return Err(AppError::ValidationError("Password is required".to_string()));
    }

    // Get identity by email
    let identity = sqlx::query!(
        r#"
        SELECT
            id, tenant_id, identity_type, password_hash, status
        FROM identities
        WHERE email = $1
        "#,
        req.email
    )
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or(AppError::InvalidCredentials)?;

    // Check if identity is active
    if identity.status != "active" {
        tracing::warn!("Login attempt for inactive identity: {}", identity.id);
        return Err(AppError::InvalidCredentials);
    }

    // Verify password
    let password_hash = identity
        .password_hash
        .ok_or(AppError::InvalidCredentials)?;

    let is_valid = password::verify_password(&req.password, &password_hash)?;

    if !is_valid {
        tracing::warn!("Invalid password for identity: {}", identity.id);
        return Err(AppError::InvalidCredentials);
    }

    // Generate JWT tokens
    let config = crate::config::Config::load().map_err(|e| {
        tracing::error!("Failed to load config: {}", e);
        AppError::Internal("Configuration error".to_string())
    })?;

    let jwt_manager = JwtManager::new(&config)?;

    let access_token = jwt_manager.generate_access_token(
        identity.id,
        identity.tenant_id,
        &identity.identity_type,
    )?;

    let refresh_token = jwt_manager.generate_refresh_token(
        identity.id,
        identity.tenant_id,
        None, // First token, no family ID yet
    )?;

    // Extract token IDs for session storage
    let access_token_id = jwt_manager.extract_token_id(&access_token)?;
    let refresh_token_id = jwt_manager.extract_token_id(&refresh_token)?;

    // Get token expiration from config
    let expires_in = config.auth.jwt_expiration_seconds;
    let refresh_expires_in = config.auth.refresh_token_expiration_seconds;

    // Create sessions in database
    let now = chrono::Utc::now();
    let access_expires_at = now + chrono::Duration::seconds(expires_in);
    let refresh_expires_at = now + chrono::Duration::seconds(refresh_expires_in);

    // Store access token session
    sqlx::query!(
        r#"
        INSERT INTO sessions (
            identity_id, tenant_id, token_id, token_type, expires_at
        )
        VALUES ($1, $2, $3, 'jwt', $4)
        "#,
        identity.id,
        identity.tenant_id,
        access_token_id,
        access_expires_at
    )
    .execute(&state.db_pool)
    .await?;

    // Store refresh token session
    sqlx::query!(
        r#"
        INSERT INTO sessions (
            identity_id, tenant_id, token_id, token_type, expires_at
        )
        VALUES ($1, $2, $3, 'refresh', $4)
        "#,
        identity.id,
        identity.tenant_id,
        refresh_token_id,
        refresh_expires_at
    )
    .execute(&state.db_pool)
    .await?;

    // Update last login time
    sqlx::query!(
        r#"
        UPDATE identities
        SET last_login_at = NOW()
        WHERE id = $1
        "#,
        identity.id
    )
    .execute(&state.db_pool)
    .await?;

    tracing::info!("Successful login for identity: {}", identity.id);

    let token_pair = TokenPair::new(access_token, refresh_token, expires_in);

    Ok(Json(token_pair.into()))
}

/// POST /v1/auth/logout
///
/// Invalidate the current access token
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>> {
    tracing::info!("Logout request received");

    // Extract token from Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // Check Bearer prefix
    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized);
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Load config and create JWT manager
    let config = crate::config::Config::load().map_err(|e| {
        tracing::error!("Failed to load config: {}", e);
        AppError::Internal("Configuration error".to_string())
    })?;

    let jwt_manager = JwtManager::new(&config)?;

    // Validate and extract token ID
    let claims = jwt_manager.validate_access_token(token)?;
    let token_id = claims.token_id();

    // Revoke the token in the database
    sqlx::query!(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE token_id = $1 AND revoked_at IS NULL
        "#,
        token_id
    )
    .execute(&state.db_pool)
    .await?;

    // Add token to Redis revocation list (for fast validation)
    let mut redis_conn = state.redis_manager.clone();
    let ttl_seconds = (claims.exp - chrono::Utc::now().timestamp()).max(0) as i64;
    
    if ttl_seconds > 0 {
        crate::redis::revocation::revoke_token(
            &mut redis_conn,
            token_id,
            ttl_seconds,
        )
        .await?;
    }

    tracing::info!("Successfully logged out token: {}", token_id);

    Ok(Json(LogoutResponse {
        message: "Successfully logged out".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database and full setup
    async fn test_login_endpoint() {
        // Integration test requires full setup
    }

    #[tokio::test]
    #[ignore]
    async fn test_logout_endpoint() {
        // Integration test requires full setup
    }
}
