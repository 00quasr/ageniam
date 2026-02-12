// Authorization endpoints
use crate::api::routes::AppState;
use crate::authz::engine::{AuthorizationDecision, CedarEngine};
use crate::authz::evaluator::{create_empty_entities, AuthorizationRequestBuilder};
use crate::db::schema::PolicyRow;
use crate::errors::{AppError, Result};
use crate::observability::metrics;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

// Global Cedar engine instance
static CEDAR_ENGINE: OnceCell<Arc<CedarEngine>> = OnceCell::const_new();

async fn get_cedar_engine() -> Arc<CedarEngine> {
    CEDAR_ENGINE
        .get_or_init(|| async {
            Arc::new(CedarEngine::new())
        })
        .await
        .clone()
}

/// Request body for authorization check
#[derive(Debug, Deserialize)]
pub struct AuthzCheckRequest {
    /// Principal entity (e.g., "User::\"alice\"")
    pub principal: String,
    /// Action (e.g., "read" or "Action::\"read\"")
    pub action: String,
    /// Resource entity (e.g., "File::\"file1\"")
    pub resource: String,
    /// Optional context data
    #[serde(default)]
    pub context: serde_json::Value,
}

/// Response body for authorization check
#[derive(Debug, Serialize)]
pub struct AuthzCheckResponse {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Policy IDs that contributed to the decision
    pub reasons: Vec<String>,
    /// Any errors encountered during evaluation
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Request body for bulk authorization check
#[derive(Debug, Deserialize)]
pub struct BulkAuthzCheckRequest {
    /// List of authorization requests to check
    pub requests: Vec<AuthzCheckRequest>,
}

/// Single result in bulk authorization response
#[derive(Debug, Serialize)]
pub struct BulkAuthzCheckResult {
    /// Index of the request in the input array
    pub index: usize,
    /// Whether the request is allowed
    pub allowed: bool,
    /// Policy IDs that contributed to the decision
    pub reasons: Vec<String>,
    /// Any errors encountered during evaluation
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Response body for bulk authorization check
#[derive(Debug, Serialize)]
pub struct BulkAuthzCheckResponse {
    /// Results for each request
    pub results: Vec<BulkAuthzCheckResult>,
    /// Total number of requests processed
    pub total: usize,
    /// Number of allowed requests
    pub allowed_count: usize,
    /// Number of denied requests
    pub denied_count: usize,
}

/// POST /v1/authz/check - Check a single authorization request
#[instrument(skip(state))]
pub async fn check_authorization(
    State(state): State<AppState>,
    Json(req): Json<AuthzCheckRequest>,
) -> Result<Json<AuthzCheckResponse>> {
    info!(
        principal = %req.principal,
        action = %req.action,
        resource = %req.resource,
        "Authorization check requested"
    );

    // Get the Cedar engine
    let engine = get_cedar_engine().await;

    // Load policies from database
    let policies = load_policies_from_db(&state).await?;
    if !policies.is_empty() {
        engine.load_policies(policies).await?;
    }

    // Build the authorization request
    let cedar_request = AuthorizationRequestBuilder::new()
        .principal(req.principal.clone())
        .action(req.action.clone())
        .resource(req.resource.clone())
        .build()?;

    // Create empty entities (in a real system, you'd load these from DB)
    let entities = create_empty_entities()?;

    // Evaluate the request
    let start = std::time::Instant::now();
    let decision = engine.is_authorized(cedar_request, entities).await?;
    let duration = start.elapsed();

    // Record metrics
    metrics::observe_authz_decision_duration(duration);
    if decision.is_allowed() {
        metrics::increment_authz_allow();
    } else {
        metrics::increment_authz_deny();
    }

    debug!(
        allowed = decision.is_allowed(),
        duration_ms = duration.as_millis(),
        "Authorization decision made"
    );

    Ok(Json(AuthzCheckResponse {
        allowed: decision.is_allowed(),
        reasons: decision.reasons,
        errors: decision.errors,
    }))
}

/// POST /v1/authz/bulk-check - Check multiple authorization requests in batch
#[instrument(skip(state))]
pub async fn bulk_check_authorization(
    State(state): State<AppState>,
    Json(req): Json<BulkAuthzCheckRequest>,
) -> Result<Json<BulkAuthzCheckResponse>> {
    info!(count = req.requests.len(), "Bulk authorization check requested");

    if req.requests.is_empty() {
        return Err(AppError::BadRequest("No requests provided".to_string()));
    }

    // Limit bulk requests to prevent abuse
    const MAX_BULK_REQUESTS: usize = 100;
    if req.requests.len() > MAX_BULK_REQUESTS {
        return Err(AppError::BadRequest(format!(
            "Too many requests. Maximum is {}",
            MAX_BULK_REQUESTS
        )));
    }

    // Get the Cedar engine
    let engine = get_cedar_engine().await;

    // Load policies from database (once for all requests)
    let policies = load_policies_from_db(&state).await?;
    if !policies.is_empty() {
        engine.load_policies(policies).await?;
    }

    // Process each request
    let mut results = Vec::with_capacity(req.requests.len());
    let mut allowed_count = 0;
    let mut denied_count = 0;

    let overall_start = std::time::Instant::now();

    for (index, check_req) in req.requests.into_iter().enumerate() {
        // Build the authorization request
        let cedar_request = match AuthorizationRequestBuilder::new()
            .principal(check_req.principal.clone())
            .action(check_req.action.clone())
            .resource(check_req.resource.clone())
            .build()
        {
            Ok(req) => req,
            Err(e) => {
                // If building the request fails, record as denied with error
                error!(
                    index = index,
                    error = ?e,
                    "Failed to build authorization request"
                );
                denied_count += 1;
                results.push(BulkAuthzCheckResult {
                    index,
                    allowed: false,
                    reasons: vec![],
                    errors: vec![e.to_string()],
                });
                continue;
            }
        };

        // Create empty entities (in a real system, you'd load these from DB)
        let entities = match create_empty_entities() {
            Ok(e) => e,
            Err(e) => {
                error!(index = index, error = ?e, "Failed to create entities");
                denied_count += 1;
                results.push(BulkAuthzCheckResult {
                    index,
                    allowed: false,
                    reasons: vec![],
                    errors: vec![e.to_string()],
                });
                continue;
            }
        };

        // Evaluate the request
        let start = std::time::Instant::now();
        match engine.is_authorized(cedar_request, entities).await {
            Ok(decision) => {
                let duration = start.elapsed();
                metrics::observe_authz_decision_duration(duration);

                let allowed = decision.is_allowed();
                if allowed {
                    allowed_count += 1;
                    metrics::increment_authz_allow();
                } else {
                    denied_count += 1;
                    metrics::increment_authz_deny();
                }

                results.push(BulkAuthzCheckResult {
                    index,
                    allowed,
                    reasons: decision.reasons,
                    errors: decision.errors,
                });
            }
            Err(e) => {
                error!(index = index, error = ?e, "Authorization evaluation failed");
                denied_count += 1;
                results.push(BulkAuthzCheckResult {
                    index,
                    allowed: false,
                    reasons: vec![],
                    errors: vec![e.to_string()],
                });
            }
        }
    }

    let overall_duration = overall_start.elapsed();

    info!(
        total = results.len(),
        allowed = allowed_count,
        denied = denied_count,
        duration_ms = overall_duration.as_millis(),
        "Bulk authorization check completed"
    );

    Ok(Json(BulkAuthzCheckResponse {
        results,
        total: allowed_count + denied_count,
        allowed_count,
        denied_count,
    }))
}

/// Load policies from the database
async fn load_policies_from_db(state: &AppState) -> Result<Vec<(Uuid, String)>> {
    let policies = sqlx::query_as!(
        PolicyRow,
        r#"
        SELECT id, tenant_id, name, description, policy_cedar, version, is_active,
               created_at, updated_at
        FROM policies
        WHERE is_active = TRUE
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    debug!(count = policies.len(), "Loaded policies from database");

    Ok(policies
        .into_iter()
        .map(|p| (p.id, p.policy_cedar))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authz_check_request_deserialize() {
        let json = r#"{
            "principal": "User::\"alice\"",
            "action": "read",
            "resource": "File::\"file1\""
        }"#;

        let req: AuthzCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.principal, "User::\"alice\"");
        assert_eq!(req.action, "read");
        assert_eq!(req.resource, "File::\"file1\"");
    }

    #[test]
    fn test_bulk_authz_check_request_deserialize() {
        let json = r#"{
            "requests": [
                {
                    "principal": "User::\"alice\"",
                    "action": "read",
                    "resource": "File::\"file1\""
                },
                {
                    "principal": "User::\"bob\"",
                    "action": "write",
                    "resource": "File::\"file2\""
                }
            ]
        }"#;

        let req: BulkAuthzCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.requests.len(), 2);
    }

    #[test]
    fn test_authz_check_response_serialize() {
        let response = AuthzCheckResponse {
            allowed: true,
            reasons: vec!["policy1".to_string()],
            errors: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"allowed\":true"));
        assert!(json.contains("policy1"));
    }

    #[test]
    fn test_bulk_authz_check_response_serialize() {
        let response = BulkAuthzCheckResponse {
            results: vec![
                BulkAuthzCheckResult {
                    index: 0,
                    allowed: true,
                    reasons: vec!["policy1".to_string()],
                    errors: vec![],
                },
                BulkAuthzCheckResult {
                    index: 1,
                    allowed: false,
                    reasons: vec![],
                    errors: vec![],
                },
            ],
            total: 2,
            allowed_count: 1,
            denied_count: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":2"));
        assert!(json.contains("\"allowed_count\":1"));
        assert!(json.contains("\"denied_count\":1"));
    }
}
