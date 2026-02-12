use crate::domain::audit::PersistedAuditEvent;
use crate::errors::{AppError, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::{error, info};

/// Trait for audit event storage backends
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Write a batch of audit events to storage
    async fn write_batch(&self, events: Vec<PersistedAuditEvent>) -> Result<()>;
}

/// PostgreSQL storage backend for audit logs
pub struct PostgresAuditStorage {
    pool: PgPool,
}

impl PostgresAuditStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditStorage for PostgresAuditStorage {
    async fn write_batch(&self, events: Vec<PersistedAuditEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for event in events {
            let e = &event.event;

            // Convert Option<String> to Option<std::net::IpAddr> for ip_address
            let ip_addr: Option<std::net::IpAddr> = e
                .ip_address
                .as_ref()
                .and_then(|ip_str| ip_str.parse().ok());

            sqlx::query!(
                r#"
                INSERT INTO audit_logs (
                    id, tenant_id, actor_identity_id, delegation_chain,
                    event_type, action, resource_type, resource_id,
                    decision, decision_reason,
                    request_id, ip_address, user_agent, metadata, timestamp,
                    signature, previous_event_hash
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                "#,
                event.id,
                e.tenant_id,
                e.actor_identity_id,
                e.delegation_chain,
                e.event_type.as_str(),
                e.action,
                e.resource_type,
                e.resource_id,
                e.decision.map(|d| d.as_str()),
                e.decision_reason,
                e.request_id,
                ip_addr.map(|ip| ip.to_string()), // Convert back to string for INET type
                e.user_agent,
                e.metadata,
                e.timestamp,
                event.signature,
                event.previous_event_hash,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!("Failed to insert audit log: {:?}", e);
                AppError::Database(e)
            })?;
        }

        tx.commit().await?;

        Ok(())
    }
}

/// Multi-backend storage that can write to multiple destinations
pub struct MultiBackendStorage {
    backends: Vec<Box<dyn AuditStorage>>,
}

impl MultiBackendStorage {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    pub fn add_backend(mut self, backend: Box<dyn AuditStorage>) -> Self {
        self.backends.push(backend);
        self
    }
}

impl Default for MultiBackendStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditStorage for MultiBackendStorage {
    async fn write_batch(&self, events: Vec<PersistedAuditEvent>) -> Result<()> {
        if self.backends.is_empty() {
            return Err(AppError::Internal(
                "No storage backends configured".to_string(),
            ));
        }

        // Write to all backends in parallel
        let mut handles = Vec::new();

        for backend in &self.backends {
            let events_clone = events.clone();
            // Note: We can't easily spawn due to trait object limitations
            // In production, this would use Arc and spawn individual tasks
            // For now, we write sequentially but log errors instead of failing fast
        }

        // For now, write to each backend sequentially
        let mut errors = Vec::new();
        for (idx, backend) in self.backends.iter().enumerate() {
            if let Err(e) = backend.write_batch(events.clone()).await {
                error!("Backend {} failed to write audit batch: {:?}", idx, e);
                errors.push(e);
            }
        }

        // If at least one backend succeeded, we're OK
        if !errors.is_empty() && errors.len() == self.backends.len() {
            return Err(AppError::Internal(
                "All storage backends failed to write audit logs".to_string(),
            ));
        }

        Ok(())
    }
}

/// In-memory storage backend (for testing)
#[cfg(test)]
pub struct InMemoryAuditStorage {
    events: std::sync::Arc<tokio::sync::Mutex<Vec<PersistedAuditEvent>>>,
}

#[cfg(test)]
impl InMemoryAuditStorage {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    pub async fn get_events(&self) -> Vec<PersistedAuditEvent> {
        self.events.lock().await.clone()
    }
}

#[cfg(test)]
#[async_trait]
impl AuditStorage for InMemoryAuditStorage {
    async fn write_batch(&self, events: Vec<PersistedAuditEvent>) -> Result<()> {
        self.events.lock().await.extend(events);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::audit::{AuditEvent, AuditEventType};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryAuditStorage::new();

        let event = PersistedAuditEvent {
            id: Uuid::new_v4(),
            event: AuditEvent::new(
                Uuid::new_v4(),
                AuditEventType::SystemEvent,
                "test_action".to_string(),
                "test_resource".to_string(),
            ),
            signature: None,
            previous_event_hash: None,
        };

        storage.write_batch(vec![event.clone()]).await.unwrap();

        let stored = storage.get_events().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, event.id);
    }

    #[tokio::test]
    async fn test_multi_backend_storage() {
        let storage1 = InMemoryAuditStorage::new();
        let storage2 = InMemoryAuditStorage::new();

        let multi = MultiBackendStorage::new()
            .add_backend(Box::new(storage1.clone()))
            .add_backend(Box::new(storage2.clone()));

        let event = PersistedAuditEvent {
            id: Uuid::new_v4(),
            event: AuditEvent::new(
                Uuid::new_v4(),
                AuditEventType::SystemEvent,
                "test_action".to_string(),
                "test_resource".to_string(),
            ),
            signature: None,
            previous_event_hash: None,
        };

        multi.write_batch(vec![event.clone()]).await.unwrap();

        assert_eq!(storage1.get_events().await.len(), 1);
        assert_eq!(storage2.get_events().await.len(), 1);
    }
}
