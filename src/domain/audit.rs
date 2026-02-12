use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audit event builder for creating audit log entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub tenant_id: Uuid,
    pub actor_identity_id: Option<Uuid>,
    pub delegation_chain: Option<serde_json::Value>,
    pub event_type: AuditEventType,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub decision: Option<Decision>,
    pub decision_reason: Option<String>,
    pub request_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl AuditEvent {
    /// Create a new audit event builder
    pub fn new(
        tenant_id: Uuid,
        event_type: AuditEventType,
        action: String,
        resource_type: String,
    ) -> Self {
        Self {
            tenant_id,
            actor_identity_id: None,
            delegation_chain: None,
            event_type,
            action,
            resource_type,
            resource_id: None,
            decision: None,
            decision_reason: None,
            request_id: None,
            ip_address: None,
            user_agent: None,
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
        }
    }

    pub fn with_actor(mut self, actor_id: Uuid) -> Self {
        self.actor_identity_id = Some(actor_id);
        self
    }

    pub fn with_delegation_chain(mut self, chain: serde_json::Value) -> Self {
        self.delegation_chain = Some(chain);
        self
    }

    pub fn with_resource_id(mut self, resource_id: String) -> Self {
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_decision(mut self, decision: Decision, reason: Option<String>) -> Self {
        self.decision = Some(decision);
        self.decision_reason = reason;
        self
    }

    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_context(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.ip_address = ip;
        self.user_agent = user_agent;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Audit event types for categorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    Authentication,
    Authorization,
    IdentityCreated,
    IdentityUpdated,
    IdentityDeleted,
    RoleAssigned,
    RoleRevoked,
    PolicyCreated,
    PolicyUpdated,
    PolicyDeleted,
    SessionCreated,
    SessionExpired,
    SessionRevoked,
    TokenGenerated,
    TokenRefreshed,
    TokenRevoked,
    RateLimitExceeded,
    ConfigurationChanged,
    SystemEvent,
}

impl AuditEventType {
    pub fn as_str(&self) -> &str {
        match self {
            AuditEventType::Authentication => "authentication",
            AuditEventType::Authorization => "authorization",
            AuditEventType::IdentityCreated => "identity_created",
            AuditEventType::IdentityUpdated => "identity_updated",
            AuditEventType::IdentityDeleted => "identity_deleted",
            AuditEventType::RoleAssigned => "role_assigned",
            AuditEventType::RoleRevoked => "role_revoked",
            AuditEventType::PolicyCreated => "policy_created",
            AuditEventType::PolicyUpdated => "policy_updated",
            AuditEventType::PolicyDeleted => "policy_deleted",
            AuditEventType::SessionCreated => "session_created",
            AuditEventType::SessionExpired => "session_expired",
            AuditEventType::SessionRevoked => "session_revoked",
            AuditEventType::TokenGenerated => "token_generated",
            AuditEventType::TokenRefreshed => "token_refreshed",
            AuditEventType::TokenRevoked => "token_revoked",
            AuditEventType::RateLimitExceeded => "rate_limit_exceeded",
            AuditEventType::ConfigurationChanged => "configuration_changed",
            AuditEventType::SystemEvent => "system_event",
        }
    }
}

/// Authorization decision for audit logs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub fn as_str(&self) -> &str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        }
    }
}

/// Persisted audit log with tamper-proofing fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuditEvent {
    pub id: Uuid,
    pub event: AuditEvent,
    pub signature: Option<String>,
    pub previous_event_hash: Option<String>,
}
