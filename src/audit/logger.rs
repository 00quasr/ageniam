use crate::domain::audit::{AuditEvent, PersistedAuditEvent};
use crate::errors::Result;
use crate::audit::storage::AuditStorage;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Configuration for the audit logger
#[derive(Debug, Clone)]
pub struct AuditLoggerConfig {
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub channel_buffer_size: usize,
}

impl Default for AuditLoggerConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            batch_timeout_ms: 1000,
            channel_buffer_size: 10000,
        }
    }
}

/// Async audit logger with batching for high-performance event logging
pub struct AuditLogger {
    sender: mpsc::Sender<AuditEvent>,
}

impl AuditLogger {
    /// Create a new audit logger with the given storage backend and configuration
    pub fn new(storage: Arc<dyn AuditStorage>, config: AuditLoggerConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.channel_buffer_size);

        // Spawn the background batch processor
        tokio::spawn(batch_processor(receiver, storage, config));

        Self { sender }
    }

    /// Log an audit event asynchronously
    /// Returns immediately after queuing the event
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        self.sender
            .send(event)
            .await
            .map_err(|e| crate::errors::AppError::Internal(format!("Failed to queue audit event: {}", e)))?;
        Ok(())
    }

    /// Log an audit event with a blocking call (for tests or critical operations)
    pub fn log_blocking(&self, event: AuditEvent) -> Result<()> {
        self.sender
            .try_send(event)
            .map_err(|e| crate::errors::AppError::Internal(format!("Failed to queue audit event: {}", e)))?;
        Ok(())
    }

    /// Get the current queue size (for monitoring)
    pub fn queue_size(&self) -> usize {
        self.sender.capacity() - self.sender.max_capacity()
    }
}

/// Background batch processor that accumulates events and writes them in batches
async fn batch_processor(
    mut receiver: mpsc::Receiver<AuditEvent>,
    storage: Arc<dyn AuditStorage>,
    config: AuditLoggerConfig,
) {
    let mut batch: Vec<AuditEvent> = Vec::with_capacity(config.batch_size);
    let mut flush_interval = interval(Duration::from_millis(config.batch_timeout_ms));

    info!(
        "Audit logger batch processor started (batch_size={}, timeout_ms={})",
        config.batch_size, config.batch_timeout_ms
    );

    loop {
        tokio::select! {
            // Receive events from the channel
            Some(event) = receiver.recv() => {
                batch.push(event);

                // Flush if batch is full
                if batch.len() >= config.batch_size {
                    if let Err(e) = flush_batch(&mut batch, &storage).await {
                        error!("Failed to flush audit batch: {:?}", e);
                    }
                }
            }

            // Flush on timeout even if batch is not full
            _ = flush_interval.tick() => {
                if !batch.is_empty() {
                    if let Err(e) = flush_batch(&mut batch, &storage).await {
                        error!("Failed to flush audit batch on timeout: {:?}", e);
                    }
                }
            }

            // Channel closed, flush remaining events and exit
            else => {
                warn!("Audit logger channel closed, flushing remaining events");
                if !batch.is_empty() {
                    if let Err(e) = flush_batch(&mut batch, &storage).await {
                        error!("Failed to flush final audit batch: {:?}", e);
                    }
                }
                break;
            }
        }
    }

    info!("Audit logger batch processor stopped");
}

/// Flush a batch of events to storage
async fn flush_batch(
    batch: &mut Vec<AuditEvent>,
    storage: &Arc<dyn AuditStorage>,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let count = batch.len();
    let start = std::time::Instant::now();

    // Convert events to persisted events (without tamper-proofing for now)
    let persisted_events: Vec<PersistedAuditEvent> = batch
        .iter()
        .map(|event| PersistedAuditEvent {
            id: Uuid::new_v4(),
            event: event.clone(),
            signature: None,
            previous_event_hash: None,
        })
        .collect();

    // Write batch to storage
    storage.write_batch(persisted_events).await?;

    let duration = start.elapsed();
    info!(
        "Flushed {} audit events to storage in {:?}",
        count, duration
    );

    // Record metrics
    metrics::counter!("audit_events_written_total", count as u64);
    metrics::histogram!("audit_batch_write_duration_seconds", duration.as_secs_f64());

    // Clear the batch
    batch.clear();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::audit::AuditEventType;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockStorage {
        events: Arc<Mutex<Vec<PersistedAuditEvent>>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_events(&self) -> Vec<PersistedAuditEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl AuditStorage for MockStorage {
        async fn write_batch(&self, events: Vec<PersistedAuditEvent>) -> Result<()> {
            self.events.lock().unwrap().extend(events);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_audit_logger_batching() {
        let storage = Arc::new(MockStorage::new());
        let config = AuditLoggerConfig {
            batch_size: 5,
            batch_timeout_ms: 100,
            channel_buffer_size: 100,
        };

        let logger = AuditLogger::new(storage.clone(), config);

        // Log 3 events (should not trigger batch flush yet)
        for i in 0..3 {
            let event = AuditEvent::new(
                Uuid::new_v4(),
                AuditEventType::SystemEvent,
                format!("test_action_{}", i),
                "test_resource".to_string(),
            );
            logger.log(event).await.unwrap();
        }

        // Wait a bit to ensure they're processed
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(storage.get_events().len(), 0, "Events should not be flushed yet");

        // Log 2 more events (should trigger batch flush at 5)
        for i in 3..5 {
            let event = AuditEvent::new(
                Uuid::new_v4(),
                AuditEventType::SystemEvent,
                format!("test_action_{}", i),
                "test_resource".to_string(),
            );
            logger.log(event).await.unwrap();
        }

        // Wait for batch to be flushed
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(storage.get_events().len(), 5, "All 5 events should be flushed");
    }

    #[tokio::test]
    async fn test_audit_logger_timeout_flush() {
        let storage = Arc::new(MockStorage::new());
        let config = AuditLoggerConfig {
            batch_size: 100,
            batch_timeout_ms: 100,
            channel_buffer_size: 100,
        };

        let logger = AuditLogger::new(storage.clone(), config);

        // Log 2 events (not enough to trigger batch size)
        for i in 0..2 {
            let event = AuditEvent::new(
                Uuid::new_v4(),
                AuditEventType::SystemEvent,
                format!("test_action_{}", i),
                "test_resource".to_string(),
            );
            logger.log(event).await.unwrap();
        }

        // Wait for timeout to trigger flush
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(storage.get_events().len(), 2, "Events should be flushed after timeout");
    }
}
