// Database schema types and query helpers

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Tenant
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Identity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Identity {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub identity_type: String,
    pub name: String,
    pub email: Option<String>,
    pub status: String,
    pub parent_identity_id: Option<Uuid>,
    pub task_id: Option<String>,
    pub task_scope: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
    pub password_hash: Option<String>,
    pub api_key_hash: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdentityType {
    User,
    Service,
    Agent,
}

impl IdentityType {
    pub fn as_str(&self) -> &str {
        match self {
            IdentityType::User => "user",
            IdentityType::Service => "service",
            IdentityType::Agent => "agent",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(IdentityType::User),
            "service" => Some(IdentityType::Service),
            "agent" => Some(IdentityType::Agent),
            _ => None,
        }
    }
}

// ============================================================================
// Role
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub parent_role_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Permission
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub resource_type: String,
    pub action: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Session
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub tenant_id: Uuid,
    pub token_id: String,
    pub token_type: String,
    pub scope: Option<serde_json::Value>,
    pub delegation_chain: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
}

// ============================================================================
// Policy
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Policy {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub policy_cedar: String,
    pub resource_type: Option<String>,
    pub priority: i32,
    pub effect: String,
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Audit Log
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub actor_identity_id: Option<Uuid>,
    pub delegation_chain: Option<serde_json::Value>,
    pub event_type: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub decision: Option<String>,
    pub decision_reason: Option<String>,
    pub request_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>,
    pub previous_event_hash: Option<String>,
}

// ============================================================================
// Rate Limit
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RateLimit {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub target_type: String,
    pub target_id: Uuid,
    pub limit_type: String,
    pub max_count: i32,
    pub window_seconds: i32,
    pub resource_type: Option<String>,
    pub action: Option<String>,
    pub created_at: DateTime<Utc>,
}
