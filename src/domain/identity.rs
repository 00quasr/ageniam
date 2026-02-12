// Identity domain model and JIT provisioning logic

use crate::db::schema::{Identity, IdentityType};
use crate::errors::{AppError, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Domain Types
// ============================================================================

/// Builder for creating new identities
#[derive(Debug, Clone)]
pub struct IdentityBuilder {
    tenant_id: Uuid,
    identity_type: IdentityType,
    name: String,
    email: Option<String>,
    parent_identity_id: Option<Uuid>,
    task_id: Option<String>,
    task_scope: Option<serde_json::Value>,
    expires_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

impl IdentityBuilder {
    /// Create a new identity builder
    pub fn new(tenant_id: Uuid, identity_type: IdentityType, name: String) -> Self {
        Self {
            tenant_id,
            identity_type,
            name,
            email: None,
            parent_identity_id: None,
            task_id: None,
            task_scope: None,
            expires_at: None,
            metadata: json!({}),
        }
    }

    /// Set email (required for users)
    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    /// Set parent identity (required for agents)
    pub fn parent_identity_id(mut self, parent_id: Uuid) -> Self {
        self.parent_identity_id = Some(parent_id);
        self
    }

    /// Set task ID (for agent task scoping)
    pub fn task_id(mut self, task_id: String) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Set task scope (permissions/resources for this task)
    pub fn task_scope(mut self, scope: serde_json::Value) -> Self {
        self.task_scope = Some(scope);
        self
    }

    /// Set expiration time
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Validate the identity configuration
    fn validate(&self) -> Result<()> {
        // Users must have email
        if matches!(self.identity_type, IdentityType::User) && self.email.is_none() {
            return Err(AppError::ValidationError(
                "Users must have an email address".to_string(),
            ));
        }

        // Agents must have parent
        if matches!(self.identity_type, IdentityType::Agent) && self.parent_identity_id.is_none() {
            return Err(AppError::ValidationError(
                "Agents must have a parent identity".to_string(),
            ));
        }

        // Validate email format if provided
        if let Some(ref email) = self.email {
            if !email.contains('@') || email.len() < 3 {
                return Err(AppError::ValidationError(
                    "Invalid email format".to_string(),
                ));
            }
        }

        // Validate name
        if self.name.trim().is_empty() {
            return Err(AppError::ValidationError(
                "Identity name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Build and validate the identity
    pub async fn build(self, pool: &PgPool) -> Result<Identity> {
        self.validate()?;

        // For agents, validate parent exists and is in same tenant
        if let Some(parent_id) = self.parent_identity_id {
            let parent = get_identity_by_id(pool, parent_id).await?;
            if parent.tenant_id != self.tenant_id {
                return Err(AppError::ValidationError(
                    "Parent identity must be in same tenant".to_string(),
                ));
            }
        }

        // Create the identity record
        create_identity(pool, self).await
    }
}

// ============================================================================
// JIT Agent Provisioning
// ============================================================================

/// Parameters for JIT agent provisioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProvisionRequest {
    pub parent_identity_id: Uuid,
    pub task_id: String,
    pub task_scope: serde_json::Value,
    pub name: String,
    pub ttl_seconds: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

/// Result of agent provisioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProvisionResult {
    pub agent_identity: Identity,
    pub delegation_depth: i32,
}

/// Provision a new agent identity just-in-time for a task
///
/// This function implements JIT provisioning logic:
/// 1. Validates the parent identity exists and is active
/// 2. Checks delegation depth limits (max 10 levels)
/// 3. Calculates appropriate expiration time
/// 4. Creates the agent identity with proper delegation chain
/// 5. Returns the agent identity for token generation
pub async fn provision_agent(
    pool: &PgPool,
    tenant_id: Uuid,
    request: AgentProvisionRequest,
) -> Result<AgentProvisionResult> {
    tracing::info!(
        "Provisioning agent for task {} under parent {}",
        request.task_id,
        request.parent_identity_id
    );

    // 1. Validate parent identity
    let parent = get_identity_by_id(pool, request.parent_identity_id).await?;

    // Check tenant isolation
    if parent.tenant_id != tenant_id {
        return Err(AppError::ValidationError(
            "Parent identity must be in same tenant".to_string(),
        ));
    }

    // Check parent is active
    if parent.status != "active" {
        return Err(AppError::ValidationError(
            "Parent identity is not active".to_string(),
        ));
    }

    // 2. Calculate delegation depth
    let delegation_depth = calculate_delegation_depth(pool, parent.id).await?;

    const MAX_DELEGATION_DEPTH: i32 = 10;
    if delegation_depth >= MAX_DELEGATION_DEPTH {
        return Err(AppError::ValidationError(
            format!("Maximum delegation depth of {} exceeded", MAX_DELEGATION_DEPTH),
        ));
    }

    // 3. Calculate expiration time
    let ttl_seconds = request.ttl_seconds.unwrap_or(3600); // Default 1 hour
    const MAX_TTL_SECONDS: i64 = 86400; // 24 hours
    const MIN_TTL_SECONDS: i64 = 60; // 1 minute

    if ttl_seconds < MIN_TTL_SECONDS || ttl_seconds > MAX_TTL_SECONDS {
        return Err(AppError::ValidationError(
            format!("TTL must be between {} and {} seconds", MIN_TTL_SECONDS, MAX_TTL_SECONDS),
        ));
    }

    let expires_at = Utc::now() + Duration::seconds(ttl_seconds);

    // If parent has expiration, agent cannot exceed it
    let expires_at = if let Some(parent_expires) = parent.expires_at {
        if expires_at > parent_expires {
            parent_expires
        } else {
            expires_at
        }
    } else {
        expires_at
    };

    // 4. Build agent identity
    let metadata = request.metadata.unwrap_or_else(|| {
        json!({
            "provisioned_via": "jit",
            "delegation_depth": delegation_depth + 1,
        })
    });

    let agent_identity = IdentityBuilder::new(
        tenant_id,
        IdentityType::Agent,
        request.name,
    )
    .parent_identity_id(request.parent_identity_id)
    .task_id(request.task_id.clone())
    .task_scope(request.task_scope.clone())
    .expires_at(expires_at)
    .metadata(metadata)
    .build(pool)
    .await?;

    tracing::info!(
        "Successfully provisioned agent {} with depth {} expiring at {}",
        agent_identity.id,
        delegation_depth + 1,
        expires_at
    );

    Ok(AgentProvisionResult {
        agent_identity,
        delegation_depth: delegation_depth + 1,
    })
}

/// Calculate the delegation depth of an identity
/// Returns 0 for root identities (users/services), N for agents
async fn calculate_delegation_depth(pool: &PgPool, identity_id: Uuid) -> Result<i32> {
    let result = sqlx::query!(
        r#"
        WITH RECURSIVE delegation_chain AS (
            SELECT id, parent_identity_id, 0 as depth
            FROM identities
            WHERE id = $1

            UNION ALL

            SELECT i.id, i.parent_identity_id, dc.depth + 1
            FROM identities i
            INNER JOIN delegation_chain dc ON i.id = dc.parent_identity_id
            WHERE dc.depth < 100  -- Safety limit to prevent infinite loops
        )
        SELECT MAX(depth) as max_depth
        FROM delegation_chain
        "#,
        identity_id
    )
    .fetch_one(pool)
    .await?;

    Ok(result.max_depth.unwrap_or(0))
}

/// Get the full delegation chain for an identity
pub async fn get_delegation_chain(pool: &PgPool, identity_id: Uuid) -> Result<Vec<Identity>> {
    let identities = sqlx::query_as!(
        Identity,
        r#"
        WITH RECURSIVE delegation_chain AS (
            SELECT id, tenant_id, identity_type, name, email, status,
                   parent_identity_id, task_id, task_scope, expires_at,
                   password_hash, api_key_hash, metadata,
                   created_at, updated_at, last_login_at, 0 as depth
            FROM identities
            WHERE id = $1

            UNION ALL

            SELECT i.id, i.tenant_id, i.identity_type, i.name, i.email, i.status,
                   i.parent_identity_id, i.task_id, i.task_scope, i.expires_at,
                   i.password_hash, i.api_key_hash, i.metadata,
                   i.created_at, i.updated_at, i.last_login_at, dc.depth + 1
            FROM identities i
            INNER JOIN delegation_chain dc ON i.id = dc.parent_identity_id
            WHERE dc.depth < 100
        )
        SELECT id, tenant_id, identity_type, name, email, status,
               parent_identity_id, task_id, task_scope, expires_at,
               password_hash, api_key_hash, metadata,
               created_at, updated_at, last_login_at
        FROM delegation_chain
        ORDER BY depth
        "#,
        identity_id
    )
    .fetch_all(pool)
    .await?;

    Ok(identities)
}

// ============================================================================
// Database Operations
// ============================================================================

/// Create a new identity in the database
async fn create_identity(pool: &PgPool, builder: IdentityBuilder) -> Result<Identity> {
    let identity = sqlx::query_as!(
        Identity,
        r#"
        INSERT INTO identities (
            tenant_id, identity_type, name, email, status,
            parent_identity_id, task_id, task_scope, expires_at, metadata
        )
        VALUES ($1, $2, $3, $4, 'active', $5, $6, $7, $8, $9)
        RETURNING id, tenant_id, identity_type, name, email, status,
                  parent_identity_id, task_id, task_scope, expires_at,
                  password_hash, api_key_hash, metadata,
                  created_at, updated_at, last_login_at
        "#,
        builder.tenant_id,
        builder.identity_type.as_str(),
        builder.name,
        builder.email,
        builder.parent_identity_id,
        builder.task_id,
        builder.task_scope,
        builder.expires_at,
        builder.metadata,
    )
    .fetch_one(pool)
    .await?;

    Ok(identity)
}

/// Get an identity by ID
pub async fn get_identity_by_id(pool: &PgPool, id: Uuid) -> Result<Identity> {
    let identity = sqlx::query_as!(
        Identity,
        r#"
        SELECT id, tenant_id, identity_type, name, email, status,
               parent_identity_id, task_id, task_scope, expires_at,
               password_hash, api_key_hash, metadata,
               created_at, updated_at, last_login_at
        FROM identities
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::IdentityNotFound)?;

    Ok(identity)
}

/// Get an identity by email (for user lookup)
pub async fn get_identity_by_email(pool: &PgPool, tenant_id: Uuid, email: &str) -> Result<Identity> {
    let identity = sqlx::query_as!(
        Identity,
        r#"
        SELECT id, tenant_id, identity_type, name, email, status,
               parent_identity_id, task_id, task_scope, expires_at,
               password_hash, api_key_hash, metadata,
               created_at, updated_at, last_login_at
        FROM identities
        WHERE tenant_id = $1 AND email = $2
        "#,
        tenant_id,
        email
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::IdentityNotFound)?;

    Ok(identity)
}

/// Update identity status
pub async fn update_identity_status(
    pool: &PgPool,
    identity_id: Uuid,
    status: &str,
) -> Result<Identity> {
    // Validate status
    if !["active", "suspended", "deleted"].contains(&status) {
        return Err(AppError::ValidationError(
            "Invalid status value".to_string(),
        ));
    }

    let identity = sqlx::query_as!(
        Identity,
        r#"
        UPDATE identities
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, tenant_id, identity_type, name, email, status,
                  parent_identity_id, task_id, task_scope, expires_at,
                  password_hash, api_key_hash, metadata,
                  created_at, updated_at, last_login_at
        "#,
        identity_id,
        status
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::IdentityNotFound)?;

    Ok(identity)
}

/// Update last login timestamp
pub async fn update_last_login(pool: &PgPool, identity_id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE identities
        SET last_login_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
        identity_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// List identities for a tenant with optional filters
#[derive(Debug, Clone, Default)]
pub struct IdentityListFilter {
    pub tenant_id: Uuid,
    pub identity_type: Option<String>,
    pub status: Option<String>,
    pub parent_identity_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_identities(pool: &PgPool, filter: IdentityListFilter) -> Result<Vec<Identity>> {
    let limit = filter.limit.unwrap_or(100).min(1000); // Max 1000
    let offset = filter.offset.unwrap_or(0);

    let identities = sqlx::query_as!(
        Identity,
        r#"
        SELECT id, tenant_id, identity_type, name, email, status,
               parent_identity_id, task_id, task_scope, expires_at,
               password_hash, api_key_hash, metadata,
               created_at, updated_at, last_login_at
        FROM identities
        WHERE tenant_id = $1
            AND ($2::text IS NULL OR identity_type = $2)
            AND ($3::text IS NULL OR status = $3)
            AND ($4::uuid IS NULL OR parent_identity_id = $4)
        ORDER BY created_at DESC
        LIMIT $5 OFFSET $6
        "#,
        filter.tenant_id,
        filter.identity_type,
        filter.status,
        filter.parent_identity_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(identities)
}

/// Delete expired agent identities (cleanup job)
pub async fn delete_expired_agents(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE identities
        SET status = 'deleted', updated_at = NOW()
        WHERE identity_type = 'agent'
          AND status = 'active'
          AND expires_at IS NOT NULL
          AND expires_at < NOW()
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_builder_validation() {
        // User without email should fail
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::User,
            "Test User".to_string(),
        );
        assert!(builder.validate().is_err());

        // User with email should pass
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::User,
            "Test User".to_string(),
        )
        .email("test@example.com".to_string());
        assert!(builder.validate().is_ok());

        // Agent without parent should fail
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::Agent,
            "Test Agent".to_string(),
        );
        assert!(builder.validate().is_err());

        // Agent with parent should pass
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::Agent,
            "Test Agent".to_string(),
        )
        .parent_identity_id(Uuid::new_v4());
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_email_validation() {
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::User,
            "Test User".to_string(),
        )
        .email("invalid".to_string());
        assert!(builder.validate().is_err());

        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::User,
            "Test User".to_string(),
        )
        .email("valid@example.com".to_string());
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_empty_name_validation() {
        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::Service,
            "".to_string(),
        );
        assert!(builder.validate().is_err());

        let builder = IdentityBuilder::new(
            Uuid::new_v4(),
            IdentityType::Service,
            "   ".to_string(),
        );
        assert!(builder.validate().is_err());
    }
}
