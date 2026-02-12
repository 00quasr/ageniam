// JWT token generation and validation

use crate::config::Config;
use crate::errors::{AppError, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// JWT Claims
// ============================================================================

/// Standard JWT claims for access tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (identity ID)
    pub sub: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Identity type (user, service, agent)
    pub identity_type: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// JWT ID (unique token identifier)
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: Vec<String>,
    /// Optional custom claims
    #[serde(flatten)]
    pub custom: Option<serde_json::Value>,
}

impl JwtClaims {
    /// Create new JWT claims
    pub fn new(
        identity_id: Uuid,
        tenant_id: Uuid,
        identity_type: &str,
        duration_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::seconds(duration_seconds);

        Self {
            sub: identity_id.to_string(),
            tenant_id: tenant_id.to_string(),
            identity_type: identity_type.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: Uuid::new_v4().to_string(),
            iss: "agent-iam".to_string(),
            aud: vec!["agent-iam-api".to_string()],
            custom: None,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp <= now
    }

    /// Get token ID
    pub fn token_id(&self) -> &str {
        &self.jti
    }

    /// Get identity ID
    pub fn identity_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub).map_err(|e| AppError::TokenValidation(format!("Invalid subject UUID: {}", e)))
    }

    /// Get tenant ID
    pub fn tenant_id_uuid(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.tenant_id).map_err(|e| AppError::TokenValidation(format!("Invalid tenant UUID: {}", e)))
    }

    /// Get expiration as DateTime
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(Utc::now)
    }
}

// ============================================================================
// Refresh Token Claims
// ============================================================================

/// Claims for refresh tokens (simpler than access tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (identity ID)
    pub sub: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// JWT ID (unique token identifier)
    pub jti: String,
    /// Token family ID (for rotation tracking)
    pub family_id: String,
    /// Issuer
    pub iss: String,
}

impl RefreshTokenClaims {
    /// Create new refresh token claims
    pub fn new(
        identity_id: Uuid,
        tenant_id: Uuid,
        duration_seconds: i64,
        family_id: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::seconds(duration_seconds);

        Self {
            sub: identity_id.to_string(),
            tenant_id: tenant_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            jti: Uuid::new_v4().to_string(),
            family_id: family_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            iss: "agent-iam".to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp <= now
    }

    /// Get token ID
    pub fn token_id(&self) -> &str {
        &self.jti
    }

    /// Get identity ID
    pub fn identity_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub).map_err(|e| AppError::TokenValidation(format!("Invalid subject UUID: {}", e)))
    }

    /// Get tenant ID
    pub fn tenant_id_uuid(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.tenant_id).map_err(|e| AppError::TokenValidation(format!("Invalid tenant UUID: {}", e)))
    }

    /// Get expiration as DateTime
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(Utc::now)
    }
}

// ============================================================================
// JWT Manager
// ============================================================================

/// JWT token manager for generation and validation
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_expiration: i64,
    refresh_token_expiration: i64,
}

impl JwtManager {
    /// Create new JWT manager from configuration
    pub fn new(config: &Config) -> Result<Self> {
        // Get JWT secret from environment variable (required for security)
        let secret = std::env::var("AGENT_IAM__AUTH__JWT_SECRET")
            .map_err(|_| AppError::Configuration(
                "JWT_SECRET must be set via AGENT_IAM__AUTH__JWT_SECRET environment variable".to_string()
            ))?;

        if secret.len() < 32 {
            return Err(AppError::Configuration(
                "JWT secret must be at least 32 characters long".to_string()
            ));
        }

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_expiration: config.auth.jwt_expiration_seconds,
            refresh_token_expiration: config.auth.refresh_token_expiration_seconds,
        })
    }

    /// Generate access token (JWT)
    pub fn generate_access_token(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        identity_type: &str,
    ) -> Result<String> {
        let claims = JwtClaims::new(
            identity_id,
            tenant_id,
            identity_type,
            self.access_token_expiration,
        );

        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| AppError::TokenGeneration(format!("Failed to encode JWT: {}", e)))
    }

    /// Generate refresh token
    pub fn generate_refresh_token(
        &self,
        identity_id: Uuid,
        tenant_id: Uuid,
        family_id: Option<String>,
    ) -> Result<String> {
        let claims = RefreshTokenClaims::new(
            identity_id,
            tenant_id,
            self.refresh_token_expiration,
            family_id,
        );

        let header = Header::new(Algorithm::HS256);

        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| AppError::TokenGeneration(format!("Failed to encode refresh token: {}", e)))
    }

    /// Validate and decode access token
    pub fn validate_access_token(&self, token: &str) -> Result<JwtClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["agent-iam"]);
        validation.set_audience(&["agent-iam-api"]);

        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AppError::TokenValidation(format!("Failed to decode JWT: {}", e)))?;

        let claims = token_data.claims;

        // Additional validation
        if claims.is_expired() {
            return Err(AppError::TokenExpired);
        }

        Ok(claims)
    }

    /// Validate and decode refresh token
    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["agent-iam"]);
        // Refresh tokens don't have audience requirement
        validation.set_required_spec_claims(&["exp", "iat", "iss", "jti", "sub"]);

        let token_data = decode::<RefreshTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| AppError::TokenValidation(format!("Failed to decode refresh token: {}", e)))?;

        let claims = token_data.claims;

        // Additional validation
        if claims.is_expired() {
            return Err(AppError::TokenExpired);
        }

        Ok(claims)
    }

    /// Extract token ID from any token without full validation
    /// Useful for revocation checks
    pub fn extract_token_id(&self, token: &str) -> Result<String> {
        // Decode without validation to get JTI
        let mut validation = Validation::new(Algorithm::HS256);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;

        let token_data = decode::<serde_json::Value>(token, &self.decoding_key, &validation)
            .map_err(|e| AppError::TokenValidation(format!("Failed to extract token ID: {}", e)))?;

        token_data.claims
            .get("jti")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::TokenValidation("Missing jti claim".to_string()))
    }
}

// ============================================================================
// Token Pair
// ============================================================================

/// A pair of access token and refresh token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl TokenPair {
    /// Create new token pair
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        // Set test JWT secret in environment
        std::env::set_var("AGENT_IAM__AUTH__JWT_SECRET", "test-secret-key-for-jwt-signing-minimum-length-requirement");

        let mut config = Config::default();
        config.auth.jwt_expiration_seconds = 900; // 15 minutes
        config.auth.refresh_token_expiration_seconds = 2592000; // 30 days
        config
    }

    #[test]
    fn test_jwt_claims_creation() {
        let identity_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let claims = JwtClaims::new(identity_id, tenant_id, "user", 900);

        assert_eq!(claims.sub, identity_id.to_string());
        assert_eq!(claims.tenant_id, tenant_id.to_string());
        assert_eq!(claims.identity_type, "user");
        assert_eq!(claims.iss, "agent-iam");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_access_token_generation_and_validation() {
        let config = create_test_config();
        let manager = JwtManager::new(&config).unwrap();

        let identity_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = manager.generate_access_token(identity_id, tenant_id, "user").unwrap();
        assert!(!token.is_empty());

        let claims = manager.validate_access_token(&token).unwrap();
        assert_eq!(claims.identity_id().unwrap(), identity_id);
        assert_eq!(claims.tenant_id_uuid().unwrap(), tenant_id);
        assert_eq!(claims.identity_type, "user");
    }
}
