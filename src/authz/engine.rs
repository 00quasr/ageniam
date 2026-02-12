// Cedar policy engine wrapper
use crate::errors::Result;
use cedar_policy::{Authorizer, Decision, Entities, Policy, PolicySet, Request, Response};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Cedar policy engine that evaluates authorization requests
#[derive(Clone)]
pub struct CedarEngine {
    authorizer: Arc<Authorizer>,
    policies: Arc<RwLock<PolicySet>>,
}

impl CedarEngine {
    /// Create a new Cedar engine instance
    pub fn new() -> Self {
        info!("Initializing Cedar policy engine");
        Self {
            authorizer: Arc::new(Authorizer::new()),
            policies: Arc::new(RwLock::new(PolicySet::new())),
        }
    }

    /// Load policies from Cedar policy strings
    /// Returns the number of policies loaded
    pub async fn load_policies(&self, policy_texts: Vec<(Uuid, String)>) -> Result<usize> {
        let mut policy_set = PolicySet::new();
        let mut loaded_count = 0;

        for (policy_id, policy_text) in policy_texts {
            match Policy::parse(Some(policy_id.to_string()), policy_text.clone()) {
                Ok(policy) => {
                    policy_set.add(policy)?;
                    loaded_count += 1;
                    debug!(policy_id = %policy_id, "Loaded Cedar policy");
                }
                Err(e) => {
                    error!(policy_id = %policy_id, error = ?e, "Failed to parse Cedar policy");
                    return Err(anyhow::anyhow!("Failed to parse policy {}: {}", policy_id, e).into());
                }
            }
        }

        // Replace the policy set atomically
        let mut policies = self.policies.write().await;
        *policies = policy_set;

        info!(count = loaded_count, "Loaded Cedar policies");
        Ok(loaded_count)
    }

    /// Add a single policy to the engine
    pub async fn add_policy(&self, policy_id: Uuid, policy_text: String) -> Result<()> {
        let policy = Policy::parse(Some(policy_id.to_string()), policy_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse policy: {}", e))?;

        let mut policies = self.policies.write().await;
        policies.add(policy)?;

        debug!(policy_id = %policy_id, "Added Cedar policy");
        Ok(())
    }

    /// Remove a policy from the engine
    pub async fn remove_policy(&self, policy_id: Uuid) -> Result<()> {
        let mut policies = self.policies.write().await;
        policies.remove(&policy_id.to_string().parse()?);

        debug!(policy_id = %policy_id, "Removed Cedar policy");
        Ok(())
    }

    /// Evaluate an authorization request
    pub async fn is_authorized(
        &self,
        request: Request,
        entities: Entities,
    ) -> Result<AuthorizationDecision> {
        let policies = self.policies.read().await;

        let response = self
            .authorizer
            .is_authorized(&request, &policies, &entities);

        let decision = AuthorizationDecision::from_cedar_response(response);

        debug!(
            decision = ?decision.decision,
            principal = ?request.principal(),
            action = ?request.action(),
            resource = ?request.resource(),
            "Authorization decision made"
        );

        Ok(decision)
    }

    /// Get the number of loaded policies
    pub async fn policy_count(&self) -> usize {
        self.policies.read().await.policies().count()
    }
}

impl Default for CedarEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Authorization decision result
#[derive(Debug, Clone)]
pub struct AuthorizationDecision {
    pub decision: Decision,
    pub reasons: Vec<String>,
    pub errors: Vec<String>,
}

impl AuthorizationDecision {
    /// Check if the decision is to allow the request
    pub fn is_allowed(&self) -> bool {
        matches!(self.decision, Decision::Allow)
    }

    /// Create from Cedar response
    fn from_cedar_response(response: Response) -> Self {
        let decision = response.decision();
        let reasons = response
            .diagnostics()
            .reason()
            .map(|p| p.id().to_string())
            .collect();
        let errors = response
            .diagnostics()
            .errors()
            .map(|e| e.to_string())
            .collect();

        Self {
            decision,
            reasons,
            errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = CedarEngine::new();
        assert_eq!(engine.policy_count().await, 0);
    }

    #[tokio::test]
    async fn test_load_simple_policy() {
        let engine = CedarEngine::new();
        let policy_id = Uuid::new_v4();

        let policy_text = r#"
            permit(
                principal,
                action == Action::"read",
                resource
            );
        "#.to_string();

        let result = engine.add_policy(policy_id, policy_text).await;
        assert!(result.is_ok());
        assert_eq!(engine.policy_count().await, 1);
    }

    #[tokio::test]
    async fn test_invalid_policy_rejected() {
        let engine = CedarEngine::new();
        let policy_id = Uuid::new_v4();

        let invalid_policy = "this is not valid Cedar syntax".to_string();

        let result = engine.add_policy(policy_id, invalid_policy).await;
        assert!(result.is_err());
        assert_eq!(engine.policy_count().await, 0);
    }
}
