use crate::errors::{AppError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Hash chain implementation for tamper-proof audit logs
///
/// Each audit event includes a hash of the previous event, creating a chain
/// where any modification to a past event would break the integrity of all
/// subsequent events.
#[derive(Debug, Clone)]
pub struct HashChain {
    /// The hash algorithm used (SHA-256)
    algorithm: HashAlgorithm,
}

/// Hash algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    #[serde(rename = "sha256")]
    Sha256,
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "sha256"),
        }
    }
}

/// Represents a hashable audit event
///
/// This is a simplified representation containing the essential fields
/// needed for hash chain computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashableEvent {
    /// Unique event ID
    pub id: uuid::Uuid,
    /// Tenant ID for multi-tenancy isolation
    pub tenant_id: uuid::Uuid,
    /// Actor identity ID (nullable)
    pub actor_identity_id: Option<uuid::Uuid>,
    /// Event type (e.g., "authentication.login")
    pub event_type: String,
    /// Action performed (e.g., "create", "read", "update", "delete")
    pub action: String,
    /// Resource type (e.g., "identity", "session")
    pub resource_type: String,
    /// Resource ID (nullable)
    pub resource_id: Option<String>,
    /// Decision for authorization events (nullable)
    pub decision: Option<String>,
    /// Timestamp in RFC3339 format
    pub timestamp: String,
    /// Previous event hash (nullable for first event in chain)
    pub previous_hash: Option<String>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl HashChain {
    /// Create a new hash chain with SHA-256
    pub fn new() -> Self {
        Self {
            algorithm: HashAlgorithm::Sha256,
        }
    }

    /// Compute the hash of an audit event
    ///
    /// The hash is computed over a canonical representation of the event
    /// to ensure consistency. The hash includes:
    /// - Event ID
    /// - Tenant ID
    /// - Actor identity ID
    /// - Event type
    /// - Action
    /// - Resource type
    /// - Resource ID
    /// - Decision
    /// - Timestamp
    /// - Previous hash
    /// - Metadata (sorted JSON)
    ///
    /// Returns a hex-encoded SHA-256 hash (64 characters)
    pub fn compute_hash(&self, event: &HashableEvent) -> Result<String> {
        let canonical = self.canonicalize(event)?;
        let hash = self.hash_bytes(canonical.as_bytes());
        Ok(hash)
    }

    /// Verify that an event's hash matches the computed hash
    pub fn verify_hash(&self, event: &HashableEvent, expected_hash: &str) -> Result<bool> {
        let computed = self.compute_hash(event)?;

        // Use constant-time comparison to prevent timing attacks
        Ok(constant_time_compare(&computed, expected_hash))
    }

    /// Verify the integrity of a chain of events
    ///
    /// Returns Ok(true) if:
    /// - The first event has no previous_hash
    /// - Each subsequent event's previous_hash matches the hash of the previous event
    /// - All event hashes are valid
    ///
    /// Returns Ok(false) if any verification fails
    pub fn verify_chain(&self, events: &[HashableEvent]) -> Result<bool> {
        if events.is_empty() {
            return Ok(true);
        }

        // First event should have no previous hash
        if events[0].previous_hash.is_some() {
            tracing::warn!(
                "First event in chain has a previous_hash, expected None"
            );
            return Ok(false);
        }

        let mut previous_hash: Option<String> = None;

        for (idx, event) in events.iter().enumerate() {
            // Verify previous hash linkage
            if event.previous_hash != previous_hash {
                tracing::warn!(
                    event_id = %event.id,
                    index = idx,
                    expected = ?previous_hash,
                    actual = ?event.previous_hash,
                    "Hash chain broken: previous_hash mismatch"
                );
                return Ok(false);
            }

            // Compute and store this event's hash for the next iteration
            previous_hash = Some(self.compute_hash(event)?);
        }

        Ok(true)
    }

    /// Find the index where a chain was broken (if any)
    ///
    /// Returns None if the chain is valid
    /// Returns Some(index) of the first event where the chain is broken
    pub fn find_chain_break(&self, events: &[HashableEvent]) -> Result<Option<usize>> {
        if events.is_empty() {
            return Ok(None);
        }

        if events[0].previous_hash.is_some() {
            return Ok(Some(0));
        }

        let mut previous_hash: Option<String> = None;

        for (idx, event) in events.iter().enumerate() {
            if event.previous_hash != previous_hash {
                return Ok(Some(idx));
            }
            previous_hash = Some(self.compute_hash(event)?);
        }

        Ok(None)
    }

    /// Canonicalize an event into a deterministic string representation
    ///
    /// This ensures that the same event data always produces the same hash,
    /// regardless of field ordering or formatting.
    fn canonicalize(&self, event: &HashableEvent) -> Result<String> {
        // Create a canonical representation using pipe-separated fields
        // Format: field_name=value|field_name=value|...
        let mut parts = Vec::new();

        parts.push(format!("id={}", event.id));
        parts.push(format!("tenant_id={}", event.tenant_id));

        if let Some(actor_id) = &event.actor_identity_id {
            parts.push(format!("actor_identity_id={}", actor_id));
        } else {
            parts.push("actor_identity_id=null".to_string());
        }

        parts.push(format!("event_type={}", event.event_type));
        parts.push(format!("action={}", event.action));
        parts.push(format!("resource_type={}", event.resource_type));

        if let Some(resource_id) = &event.resource_id {
            parts.push(format!("resource_id={}", resource_id));
        } else {
            parts.push("resource_id=null".to_string());
        }

        if let Some(decision) = &event.decision {
            parts.push(format!("decision={}", decision));
        } else {
            parts.push("decision=null".to_string());
        }

        parts.push(format!("timestamp={}", event.timestamp));

        if let Some(prev_hash) = &event.previous_hash {
            parts.push(format!("previous_hash={}", prev_hash));
        } else {
            parts.push("previous_hash=null".to_string());
        }

        // Serialize metadata to canonical JSON (sorted keys)
        let metadata_canonical = serde_json::to_string(&event.metadata)
            .map_err(|e| AppError::Internal(format!("Failed to serialize metadata: {}", e)))?;
        parts.push(format!("metadata={}", metadata_canonical));

        Ok(parts.join("|"))
    }

    /// Hash bytes using SHA-256 and return hex-encoded string
    fn hash_bytes(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant-time string comparison to prevent timing attacks
///
/// This is important for security-sensitive comparisons like hash verification
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut result = 0u8;
    for i in 0..a_bytes.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event(
        id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        event_type: &str,
        previous_hash: Option<String>,
    ) -> HashableEvent {
        HashableEvent {
            id,
            tenant_id,
            actor_identity_id: Some(uuid::Uuid::new_v4()),
            event_type: event_type.to_string(),
            action: "test".to_string(),
            resource_type: "test_resource".to_string(),
            resource_id: Some("test-id".to_string()),
            decision: Some("allow".to_string()),
            timestamp: "2026-02-12T10:00:00Z".to_string(),
            previous_hash,
            metadata: serde_json::json!({"test": "value"}),
        }
    }

    #[test]
    fn test_hash_computation() {
        let chain = HashChain::new();
        let event = create_test_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "test.event",
            None,
        );

        let hash = chain.compute_hash(&event).unwrap();

        // SHA-256 produces 64 hex characters
        assert_eq!(hash.len(), 64);

        // Hash should be deterministic
        let hash2 = chain.compute_hash(&event).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_changes_with_data() {
        let chain = HashChain::new();
        let tenant_id = uuid::Uuid::new_v4();

        let event1 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event1",
            None,
        );

        let event2 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event2",
            None,
        );

        let hash1 = chain.compute_hash(&event1).unwrap();
        let hash2 = chain.compute_hash(&event2).unwrap();

        assert_ne!(hash1, hash2, "Different events should produce different hashes");
    }

    #[test]
    fn test_verify_hash() {
        let chain = HashChain::new();
        let event = create_test_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "test.event",
            None,
        );

        let hash = chain.compute_hash(&event).unwrap();

        assert!(chain.verify_hash(&event, &hash).unwrap());
        assert!(!chain.verify_hash(&event, "invalid_hash").unwrap());
    }

    #[test]
    fn test_verify_empty_chain() {
        let chain = HashChain::new();
        let events: Vec<HashableEvent> = vec![];

        assert!(chain.verify_chain(&events).unwrap());
    }

    #[test]
    fn test_verify_single_event_chain() {
        let chain = HashChain::new();
        let event = create_test_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "test.event",
            None, // First event should have no previous hash
        );

        let events = vec![event];
        assert!(chain.verify_chain(&events).unwrap());
    }

    #[test]
    fn test_verify_valid_chain() {
        let chain = HashChain::new();
        let tenant_id = uuid::Uuid::new_v4();

        // Create first event
        let event1 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event1",
            None,
        );
        let hash1 = chain.compute_hash(&event1).unwrap();

        // Create second event with previous hash
        let event2 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event2",
            Some(hash1.clone()),
        );
        let hash2 = chain.compute_hash(&event2).unwrap();

        // Create third event
        let event3 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event3",
            Some(hash2),
        );

        let events = vec![event1, event2, event3];
        assert!(chain.verify_chain(&events).unwrap());
    }

    #[test]
    fn test_verify_broken_chain() {
        let chain = HashChain::new();
        let tenant_id = uuid::Uuid::new_v4();

        // Create first event
        let event1 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event1",
            None,
        );

        // Create second event with WRONG previous hash
        let event2 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event2",
            Some("invalid_hash".to_string()),
        );

        let events = vec![event1, event2];
        assert!(!chain.verify_chain(&events).unwrap());
    }

    #[test]
    fn test_find_chain_break() {
        let chain = HashChain::new();
        let tenant_id = uuid::Uuid::new_v4();

        // Create valid chain
        let event1 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event1",
            None,
        );
        let hash1 = chain.compute_hash(&event1).unwrap();

        let event2 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event2",
            Some(hash1),
        );

        // Third event with wrong hash
        let event3 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event3",
            Some("wrong_hash".to_string()),
        );

        let events = vec![event1, event2, event3];
        let break_index = chain.find_chain_break(&events).unwrap();

        assert_eq!(break_index, Some(2));
    }

    #[test]
    fn test_find_no_break() {
        let chain = HashChain::new();
        let tenant_id = uuid::Uuid::new_v4();

        let event1 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event1",
            None,
        );
        let hash1 = chain.compute_hash(&event1).unwrap();

        let event2 = create_test_event(
            uuid::Uuid::new_v4(),
            tenant_id,
            "test.event2",
            Some(hash1),
        );

        let events = vec![event1, event2];
        let break_index = chain.find_chain_break(&events).unwrap();

        assert_eq!(break_index, None);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc", "abc"));
        assert!(!constant_time_compare("abc", "abd"));
        assert!(!constant_time_compare("abc", "ab"));
        assert!(!constant_time_compare("abc", "abcd"));
    }

    #[test]
    fn test_first_event_with_previous_hash_invalid() {
        let chain = HashChain::new();
        let event = create_test_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "test.event",
            Some("should_not_have_this".to_string()),
        );

        let events = vec![event];
        assert!(!chain.verify_chain(&events).unwrap());
    }

    #[test]
    fn test_canonicalize_is_deterministic() {
        let chain = HashChain::new();
        let event = create_test_event(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "test.event",
            None,
        );

        let canonical1 = chain.canonicalize(&event).unwrap();
        let canonical2 = chain.canonicalize(&event).unwrap();

        assert_eq!(canonical1, canonical2);
    }

    #[test]
    fn test_null_fields_handled() {
        let chain = HashChain::new();
        let event = HashableEvent {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            actor_identity_id: None, // null
            event_type: "test".to_string(),
            action: "test".to_string(),
            resource_type: "test".to_string(),
            resource_id: None, // null
            decision: None, // null
            timestamp: "2026-02-12T10:00:00Z".to_string(),
            previous_hash: None,
            metadata: serde_json::json!({}),
        };

        let hash = chain.compute_hash(&event).unwrap();
        assert_eq!(hash.len(), 64);
    }
}
