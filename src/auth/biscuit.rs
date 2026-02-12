use crate::errors::{AppError, Result};
use biscuit_auth::{
    builder::{BiscuitBuilder, Term},
    Biscuit, KeyPair, PrivateKey, PublicKey,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Biscuit token manager for agent authentication
pub struct BiscuitManager {
    root_keypair: KeyPair,
    root_key_id: String,
}

/// Claims extracted from a validated Biscuit token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiscuitClaims {
    /// Agent identity ID
    pub agent_id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Parent identity ID (who created this agent)
    pub parent_id: Uuid,
    /// Task ID this agent is scoped to
    pub task_id: String,
    /// Task scope (permitted actions/resources)
    pub task_scope: HashMap<String, serde_json::Value>,
    /// Token expiration
    pub expires_at: DateTime<Utc>,
    /// Token issued at
    pub issued_at: DateTime<Utc>,
    /// Key ID used to sign this token
    pub key_id: String,
}

/// Request to create a new agent token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentTokenRequest {
    pub agent_id: Uuid,
    pub tenant_id: Uuid,
    pub parent_id: Uuid,
    pub task_id: String,
    pub task_scope: HashMap<String, serde_json::Value>,
    pub expires_at: DateTime<Utc>,
}

impl BiscuitManager {
    /// Create a new BiscuitManager with a root keypair
    pub fn new(root_key_id: String) -> Result<Self> {
        // In production, this should load from secure storage (e.g., KMS, Vault)
        // For now, we generate a new keypair (this would be loaded from config/secrets)
        let root_keypair = KeyPair::new();

        Ok(Self {
            root_keypair,
            root_key_id,
        })
    }

    /// Create a new BiscuitManager with an existing private key
    pub fn from_private_key(root_key_id: String, private_key_bytes: &[u8]) -> Result<Self> {
        let private_key = PrivateKey::from_bytes(private_key_bytes)
            .map_err(|e| AppError::Cryptographic(format!("Invalid private key: {}", e)))?;

        let root_keypair = KeyPair::from(&private_key);

        Ok(Self {
            root_keypair,
            root_key_id,
        })
    }

    /// Get the public key for token verification
    pub fn public_key(&self) -> PublicKey {
        self.root_keypair.public()
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key().to_bytes().to_vec()
    }

    /// Export the private key bytes (use with caution!)
    pub fn private_key_bytes(&self) -> Vec<u8> {
        self.root_keypair.private().to_bytes().to_vec()
    }

    /// Generate a new Biscuit token for an agent
    pub fn generate_token(&self, request: &CreateAgentTokenRequest) -> Result<String> {
        let now = Utc::now();

        // Validate expiration
        if request.expires_at <= now {
            return Err(AppError::ValidationError(
                "Expiration time must be in the future".to_string(),
            ));
        }

        // Build the biscuit token
        let mut builder = BiscuitBuilder::new();

        // Add facts about the agent identity
        builder
            .add_fact(format!(
                "agent(\"{}\", \"{}\", \"{}\", \"{}\")",
                request.agent_id, request.tenant_id, request.parent_id, request.task_id
            ))
            .map_err(|e| AppError::TokenGeneration(format!("Failed to add agent fact: {}", e)))?;

        // Add temporal constraint - token expires at specific time
        let expires_timestamp = request.expires_at.timestamp();
        builder
            .add_check(format!("check if time($time), $time < {}", expires_timestamp))
            .map_err(|e| {
                AppError::TokenGeneration(format!("Failed to add expiration check: {}", e))
            })?;

        // Add tenant isolation check - agent can only access resources in its tenant
        builder
            .add_check(format!(
                "check if resource($res), $res.tenant_id == \"{}\"",
                request.tenant_id
            ))
            .map_err(|e| {
                AppError::TokenGeneration(format!("Failed to add tenant check: {}", e))
            })?;

        // Add task scope constraints
        for (key, value) in &request.task_scope {
            let value_str = serde_json::to_string(value)
                .map_err(|e| AppError::TokenGeneration(format!("Invalid task scope: {}", e)))?;

            builder
                .add_fact(format!("task_scope(\"{}\", {})", key, value_str))
                .map_err(|e| {
                    AppError::TokenGeneration(format!("Failed to add task scope: {}", e))
                })?;
        }

        // Add metadata
        builder
            .add_fact(format!("issued_at({})", now.timestamp()))
            .map_err(|e| {
                AppError::TokenGeneration(format!("Failed to add issued_at: {}", e))
            })?;

        builder
            .add_fact(format!("key_id(\"{}\")", self.root_key_id))
            .map_err(|e| AppError::TokenGeneration(format!("Failed to add key_id: {}", e)))?;

        // Build and sign the token
        let biscuit = builder.build(&self.root_keypair).map_err(|e| {
            AppError::TokenGeneration(format!("Failed to build biscuit: {}", e))
        })?;

        // Serialize to base64 string
        let token = biscuit.to_base64().map_err(|e| {
            AppError::TokenGeneration(format!("Failed to serialize token: {}", e))
        })?;

        tracing::info!(
            agent_id = %request.agent_id,
            task_id = %request.task_id,
            expires_at = %request.expires_at,
            "Generated Biscuit token for agent"
        );

        Ok(token)
    }

    /// Validate a Biscuit token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<BiscuitClaims> {
        // Deserialize the token
        let biscuit = Biscuit::from_base64(token, self.public_key())
            .map_err(|e| AppError::TokenValidation(format!("Invalid token format: {}", e)))?;

        // Create an authorizer to verify the token
        let mut authorizer = biscuit.authorizer().map_err(|e| {
            AppError::TokenValidation(format!("Failed to create authorizer: {}", e))
        })?;

        // Add current time for temporal checks
        let now = Utc::now();
        authorizer
            .add_fact(format!("time({})", now.timestamp()))
            .map_err(|e| {
                AppError::TokenValidation(format!("Failed to add time fact: {}", e))
            })?;

        // Add a policy that allows the operation if all checks pass
        authorizer.allow().map_err(|e| {
            AppError::TokenValidation(format!("Failed to set allow policy: {}", e))
        })?;

        // Authorize (this verifies signature and checks constraints)
        authorizer.authorize().map_err(|e| {
            tracing::warn!(error = %e, "Token authorization failed");
            match e {
                biscuit_auth::error::Token::FailedLogic(_) => AppError::TokenExpired,
                biscuit_auth::error::Token::Format(_) => {
                    AppError::TokenValidation("Invalid token format".to_string())
                }
                _ => AppError::TokenValidation(format!("Authorization failed: {}", e)),
            }
        })?;

        // Extract claims from the token facts
        let claims = self.extract_claims(&biscuit)?;

        // Verify expiration
        if claims.expires_at <= now {
            return Err(AppError::TokenExpired);
        }

        tracing::debug!(
            agent_id = %claims.agent_id,
            task_id = %claims.task_id,
            "Validated Biscuit token"
        );

        Ok(claims)
    }

    /// Attenuate a token with additional constraints (for delegation)
    pub fn attenuate_token(&self, token: &str, additional_checks: Vec<String>) -> Result<String> {
        // Deserialize the original token
        let biscuit = Biscuit::from_base64(token, self.public_key())
            .map_err(|e| AppError::TokenValidation(format!("Invalid token format: {}", e)))?;

        // Create an attenuated token builder
        let mut builder = biscuit.create_block();

        // Add additional checks (restrictions)
        for check in additional_checks {
            builder.add_check(check).map_err(|e| {
                AppError::TokenGeneration(format!("Failed to add check: {}", e))
            })?;
        }

        // Append the new block
        let attenuated = biscuit.append(builder).map_err(|e| {
            AppError::TokenGeneration(format!("Failed to append block: {}", e))
        })?;

        // Serialize the attenuated token
        let token = attenuated.to_base64().map_err(|e| {
            AppError::TokenGeneration(format!("Failed to serialize attenuated token: {}", e))
        })?;

        tracing::info!("Created attenuated token with additional constraints");

        Ok(token)
    }

    /// Extract claims from a validated Biscuit token
    fn extract_claims(&self, biscuit: &Biscuit) -> Result<BiscuitClaims> {
        let mut authorizer = biscuit.authorizer().map_err(|e| {
            AppError::TokenValidation(format!("Failed to create authorizer: {}", e))
        })?;

        // Query for agent facts
        let agent_query = "data($agent_id, $tenant_id, $parent_id, $task_id) <- agent($agent_id, $tenant_id, $parent_id, $task_id)";
        let agent_facts = authorizer.query(agent_query).map_err(|e| {
            AppError::TokenValidation(format!("Failed to query agent facts: {}", e))
        })?;

        if agent_facts.is_empty() {
            return Err(AppError::TokenValidation(
                "No agent facts found in token".to_string(),
            ));
        }

        // Extract agent information from first fact
        let fact = &agent_facts[0];
        let agent_id = self.extract_uuid_from_term(&fact.terms[0], "agent_id")?;
        let tenant_id = self.extract_uuid_from_term(&fact.terms[1], "tenant_id")?;
        let parent_id = self.extract_uuid_from_term(&fact.terms[2], "parent_id")?;
        let task_id = self.extract_string_from_term(&fact.terms[3], "task_id")?;

        // Query for issued_at
        let issued_query = "data($issued_at) <- issued_at($issued_at)";
        let issued_facts = authorizer.query(issued_query).map_err(|e| {
            AppError::TokenValidation(format!("Failed to query issued_at: {}", e))
        })?;

        let issued_at = if let Some(fact) = issued_facts.first() {
            let timestamp = self.extract_i64_from_term(&fact.terms[0], "issued_at")?;
            DateTime::from_timestamp(timestamp, 0)
                .ok_or_else(|| AppError::TokenValidation("Invalid issued_at timestamp".to_string()))?
        } else {
            Utc::now() // Fallback if not present
        };

        // Query for key_id
        let key_query = "data($key_id) <- key_id($key_id)";
        let key_facts = authorizer.query(key_query).map_err(|e| {
            AppError::TokenValidation(format!("Failed to query key_id: {}", e))
        })?;

        let key_id = if let Some(fact) = key_facts.first() {
            self.extract_string_from_term(&fact.terms[0], "key_id")?
        } else {
            self.root_key_id.clone()
        };

        // Query for task_scope
        let scope_query = "data($key, $value) <- task_scope($key, $value)";
        let scope_facts = authorizer.query(scope_query).map_err(|e| {
            AppError::TokenValidation(format!("Failed to query task_scope: {}", e))
        })?;

        let mut task_scope = HashMap::new();
        for fact in scope_facts {
            let key = self.extract_string_from_term(&fact.terms[0], "scope_key")?;
            let value_str = self.extract_string_from_term(&fact.terms[1], "scope_value")?;
            let value: serde_json::Value = serde_json::from_str(&value_str)
                .unwrap_or_else(|_| serde_json::Value::String(value_str));
            task_scope.insert(key, value);
        }

        // For expires_at, we need to parse it from the check constraint
        // In a real implementation, you'd query the expiration from facts
        // For now, we'll set a reasonable default
        let expires_at = issued_at + chrono::Duration::hours(24);

        Ok(BiscuitClaims {
            agent_id,
            tenant_id,
            parent_id,
            task_id,
            task_scope,
            expires_at,
            issued_at,
            key_id,
        })
    }

    /// Helper to extract UUID from a Biscuit term
    fn extract_uuid_from_term(&self, term: &Term, field_name: &str) -> Result<Uuid> {
        match term {
            Term::Str(s) => Uuid::parse_str(s).map_err(|e| {
                AppError::TokenValidation(format!("Invalid UUID in {}: {}", field_name, e))
            }),
            _ => Err(AppError::TokenValidation(format!(
                "Expected string for {}, got {:?}",
                field_name, term
            ))),
        }
    }

    /// Helper to extract string from a Biscuit term
    fn extract_string_from_term(&self, term: &Term, field_name: &str) -> Result<String> {
        match term {
            Term::Str(s) => Ok(s.clone()),
            _ => Err(AppError::TokenValidation(format!(
                "Expected string for {}, got {:?}",
                field_name, term
            ))),
        }
    }

    /// Helper to extract i64 from a Biscuit term
    fn extract_i64_from_term(&self, term: &Term, field_name: &str) -> Result<i64> {
        match term {
            Term::Integer(i) => Ok(*i),
            _ => Err(AppError::TokenValidation(format!(
                "Expected integer for {}, got {:?}",
                field_name, term
            ))),
        }
    }
}

/// Thread-safe wrapper around BiscuitManager
pub type BiscuitManagerRef = Arc<BiscuitManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let manager = BiscuitManager::new("test-key-id".to_string()).unwrap();

        let agent_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        let mut task_scope = HashMap::new();
        task_scope.insert(
            "allowed_actions".to_string(),
            serde_json::json!(["read", "write"]),
        );
        task_scope.insert(
            "resource_prefix".to_string(),
            serde_json::json!("/api/v1/data"),
        );

        let request = CreateAgentTokenRequest {
            agent_id,
            tenant_id,
            parent_id,
            task_id: "task-123".to_string(),
            task_scope,
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        // Generate token
        let token = manager.generate_token(&request).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = manager.validate_token(&token).unwrap();
        assert_eq!(claims.agent_id, agent_id);
        assert_eq!(claims.tenant_id, tenant_id);
        assert_eq!(claims.parent_id, parent_id);
        assert_eq!(claims.task_id, "task-123");
    }

    #[test]
    fn test_expired_token() {
        let manager = BiscuitManager::new("test-key-id".to_string()).unwrap();

        let request = CreateAgentTokenRequest {
            agent_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            parent_id: Uuid::new_v4(),
            task_id: "task-123".to_string(),
            task_scope: HashMap::new(),
            expires_at: Utc::now() - chrono::Duration::hours(1), // Expired
        };

        // Should fail to generate expired token
        let result = manager.generate_token(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_attenuation() {
        let manager = BiscuitManager::new("test-key-id".to_string()).unwrap();

        let request = CreateAgentTokenRequest {
            agent_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            parent_id: Uuid::new_v4(),
            task_id: "task-123".to_string(),
            task_scope: HashMap::new(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let token = manager.generate_token(&request).unwrap();

        // Attenuate with additional restrictions
        let additional_checks = vec!["check if operation($op), $op == \"read\"".to_string()];

        let attenuated_token = manager.attenuate_token(&token, additional_checks).unwrap();
        assert!(!attenuated_token.is_empty());
        assert_ne!(token, attenuated_token);

        // Both tokens should still be valid
        assert!(manager.validate_token(&token).is_ok());
        assert!(manager.validate_token(&attenuated_token).is_ok());
    }

    #[test]
    fn test_invalid_token() {
        let manager = BiscuitManager::new("test-key-id".to_string()).unwrap();

        let result = manager.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_keypair_persistence() {
        let manager1 = BiscuitManager::new("test-key-id".to_string()).unwrap();
        let private_key_bytes = manager1.private_key_bytes();

        // Create a new manager from the same private key
        let manager2 =
            BiscuitManager::from_private_key("test-key-id".to_string(), &private_key_bytes)
                .unwrap();

        // Generate token with manager1
        let request = CreateAgentTokenRequest {
            agent_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            parent_id: Uuid::new_v4(),
            task_id: "task-123".to_string(),
            task_scope: HashMap::new(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let token = manager1.generate_token(&request).unwrap();

        // Validate with manager2 (same keypair)
        let result = manager2.validate_token(&token);
        assert!(result.is_ok());
    }
}
