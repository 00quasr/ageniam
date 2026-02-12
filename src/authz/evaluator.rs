// Authorization decision logic
use crate::errors::Result;
use cedar_policy::{Context, Entities, EntityId, EntityTypeName, EntityUid, Request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, warn};
use uuid::Uuid;

/// Builder for creating authorization requests
pub struct AuthorizationRequestBuilder {
    principal: Option<String>,
    action: Option<String>,
    resource: Option<String>,
    context: HashMap<String, Value>,
}

impl AuthorizationRequestBuilder {
    pub fn new() -> Self {
        Self {
            principal: None,
            action: None,
            resource: None,
            context: HashMap::new(),
        }
    }

    pub fn principal(mut self, principal: String) -> Self {
        self.principal = Some(principal);
        self
    }

    pub fn action(mut self, action: String) -> Self {
        self.action = Some(action);
        self
    }

    pub fn resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn add_context(mut self, key: String, value: Value) -> Self {
        self.context.insert(key, value);
        self
    }

    pub fn build(self) -> Result<Request> {
        let principal = self
            .principal
            .ok_or_else(|| anyhow::anyhow!("Principal is required"))?;
        let action = self
            .action
            .ok_or_else(|| anyhow::anyhow!("Action is required"))?;
        let resource = self
            .resource
            .ok_or_else(|| anyhow::anyhow!("Resource is required"))?;

        let principal_uid = parse_entity_uid(&principal)?;
        let action_uid = parse_action_uid(&action)?;
        let resource_uid = parse_entity_uid(&resource)?;

        let context = Context::from_json_value(
            serde_json::to_value(&self.context)?,
            None,
        )?;

        Ok(Request::new(
            principal_uid,
            action_uid,
            resource_uid,
            context,
            None,
        )?)
    }
}

impl Default for AuthorizationRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse an entity UID from a string like "User::\"alice\""
fn parse_entity_uid(s: &str) -> Result<EntityUid> {
    // Expected format: EntityType::"id"
    let parts: Vec<&str> = s.splitn(2, "::").collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid entity UID format: {}. Expected 'Type::\"id\"'",
            s
        )
        .into());
    }

    let type_name = EntityTypeName::from_str(parts[0])?;
    let id_part = parts[1].trim_matches('"');
    let entity_id = EntityId::from_str(id_part)?;

    Ok(EntityUid::from_type_name_and_id(type_name, entity_id))
}

/// Parse an action UID from a string like "read" or "Action::\"read\""
fn parse_action_uid(s: &str) -> Result<EntityUid> {
    // If it doesn't contain "::", assume it's just the action name
    if !s.contains("::") {
        let action_str = format!("Action::\"{}\"", s);
        return parse_entity_uid(&action_str);
    }

    parse_entity_uid(s)
}

/// Create an empty entities set
pub fn create_empty_entities() -> Result<Entities> {
    Ok(Entities::empty())
}

// ============================================================================
// Authorization Evaluator
// ============================================================================

/// Authorization decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzDecision {
    pub allowed: bool,
    pub reason: Option<String>,
}

/// High-level authorization evaluator that wraps Cedar engine
pub struct AuthzEvaluator {
    pool: PgPool,
}

impl AuthzEvaluator {
    /// Create a new authorization evaluator
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Evaluate an authorization request
    pub async fn evaluate(
        &self,
        identity_id: &Uuid,
        tenant_id: &Uuid,
        resource_type: &str,
        resource_id: Option<&str>,
        action: &str,
    ) -> Result<AuthzDecision> {
        // For now, use a simple permission-based authorization
        // TODO: Integrate with Cedar engine for policy-based authorization

        // Check if identity has permission for this action on resource type
        let has_permission = self.check_permission(
            identity_id,
            tenant_id,
            resource_type,
            action,
        ).await?;

        debug!(
            identity_id = %identity_id,
            tenant_id = %tenant_id,
            resource_type = %resource_type,
            resource_id = ?resource_id,
            action = %action,
            allowed = has_permission,
            "Authorization decision"
        );

        Ok(AuthzDecision {
            allowed: has_permission,
            reason: if has_permission {
                Some("Permission granted".to_string())
            } else {
                Some("Permission denied".to_string())
            },
        })
    }

    /// Check if identity has permission for action on resource type
    async fn check_permission(
        &self,
        identity_id: &Uuid,
        tenant_id: &Uuid,
        resource_type: &str,
        action: &str,
    ) -> Result<bool> {
        // Simple role-based check
        // In a real system, this would query the policies table and use Cedar

        // For now, allow all authenticated users to perform read operations
        // and require specific permissions for write operations
        match action {
            "read" | "list" | "get" => Ok(true),
            _ => {
                // Check if user has admin role or specific permission
                // This is a placeholder - in production this would check actual roles
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity_uid() {
        let result = parse_entity_uid("User::\"alice\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_action_uid_simple() {
        let result = parse_action_uid("read");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_action_uid_full() {
        let result = parse_action_uid("Action::\"read\"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_entity_uid() {
        let result = parse_entity_uid("invalid_format");
        assert!(result.is_err());
    }

    #[test]
    fn test_request_builder() {
        let result = AuthorizationRequestBuilder::new()
            .principal("User::\"alice\"".to_string())
            .action("read".to_string())
            .resource("File::\"file1\"".to_string())
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_request_builder_missing_principal() {
        let result = AuthorizationRequestBuilder::new()
            .action("read".to_string())
            .resource("File::\"file1\"".to_string())
            .build();

        assert!(result.is_err());
    }
}
