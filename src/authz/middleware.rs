use crate::{
    api::routes::AppState,
    authz::evaluator::AuthzEvaluator,
    errors::{AppError, Result},
};
use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Principal information extracted from authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub identity_id: Uuid,
    pub tenant_id: Uuid,
    pub identity_type: String,
    pub roles: Vec<String>,
}

/// Resource information for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub tenant_id: Option<Uuid>,
}

/// Action being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action: String,
}

/// Authorization context for middleware
#[derive(Debug, Clone)]
pub struct AuthzContext {
    pub principal: Principal,
    pub resource: Resource,
    pub action: Action,
}

impl AuthzContext {
    /// Create a new authorization context
    pub fn new(principal: Principal, resource: Resource, action: Action) -> Self {
        Self {
            principal,
            resource,
            action,
        }
    }
}

/// Extract principal from request extensions (set by auth middleware)
fn extract_principal(request: &Request) -> Result<Principal> {
    request
        .extensions()
        .get::<Principal>()
        .cloned()
        .ok_or(AppError::Unauthorized)
}

/// Derive resource from request path and method
fn derive_resource(request: &Request) -> Resource {
    let path = request.uri().path();
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Extract resource type and ID from path
    // Format: /v1/{resource_type}/{resource_id}
    let (resource_type, resource_id) = match parts.as_slice() {
        ["v1", resource, id, ..] => (resource.to_string(), Some(id.to_string())),
        ["v1", resource, ..] => (resource.to_string(), None),
        _ => ("unknown".to_string(), None),
    };

    Resource {
        resource_type,
        resource_id,
        tenant_id: None, // Will be set from principal's tenant
    }
}

/// Derive action from HTTP method and path
fn derive_action(request: &Request) -> Action {
    let method = request.method();
    let path = request.uri().path();

    let action_name = if path.contains("/authz/check") {
        "check".to_string()
    } else if path.contains("/authz/bulk-check") {
        "bulk_check".to_string()
    } else {
        match *method {
            Method::GET => "read".to_string(),
            Method::POST => "create".to_string(),
            Method::PUT | Method::PATCH => "update".to_string(),
            Method::DELETE => "delete".to_string(),
            _ => "unknown".to_string(),
        }
    };

    Action {
        action: action_name,
    }
}

/// Authorization middleware that checks Cedar policies
pub async fn authorize_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    // Extract principal from request (set by auth middleware)
    let principal = extract_principal(&request)?;

    // Derive resource and action from request
    let mut resource = derive_resource(&request);
    resource.tenant_id = Some(principal.tenant_id);

    let action = derive_action(&request);

    // Create authorization context
    let authz_context = AuthzContext::new(principal.clone(), resource.clone(), action.clone());

    // Store context in request extensions for downstream handlers
    request.extensions_mut().insert(authz_context.clone());

    // Create evaluator
    let evaluator = AuthzEvaluator::new(state.db_pool.clone());

    // Evaluate authorization
    let decision = evaluator
        .evaluate(
            &principal.identity_id,
            &principal.tenant_id,
            &resource.resource_type,
            resource.resource_id.as_deref(),
            &action.action,
        )
        .await?;

    // Log authorization decision
    tracing::info!(
        identity_id = %principal.identity_id,
        tenant_id = %principal.tenant_id,
        resource_type = %resource.resource_type,
        resource_id = ?resource.resource_id,
        action = %action.action,
        decision = %decision.allowed,
        "Authorization decision"
    );

    // Return 403 if not allowed
    if !decision.allowed {
        tracing::warn!(
            identity_id = %principal.identity_id,
            resource_type = %resource.resource_type,
            action = %action.action,
            reason = ?decision.reason,
            "Authorization denied"
        );
        return Err(AppError::Forbidden);
    }

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Builder for resource-specific authorization
pub struct AuthzRequirement {
    resource_type: String,
    action: String,
}

impl AuthzRequirement {
    /// Create a new authorization requirement
    pub fn new(resource_type: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            action: action.into(),
        }
    }

    /// Middleware function for this specific requirement
    pub async fn check(
        self,
        State(state): State<AppState>,
        request: Request,
        next: Next,
    ) -> Result<Response> {
        // Extract principal from request
        let principal = extract_principal(&request)?;

        // Create evaluator
        let evaluator = AuthzEvaluator::new(state.db_pool.clone());

        // Evaluate with specific resource type and action
        let decision = evaluator
            .evaluate(
                &principal.identity_id,
                &principal.tenant_id,
                &self.resource_type,
                None,
                &self.action,
            )
            .await?;

        if !decision.allowed {
            tracing::warn!(
                identity_id = %principal.identity_id,
                resource_type = %self.resource_type,
                action = %self.action,
                "Authorization denied for specific requirement"
            );
            return Err(AppError::Forbidden);
        }

        Ok(next.run(request).await)
    }
}

/// Helper macro for creating authorization requirements
#[macro_export]
macro_rules! require_authz {
    ($resource:expr, $action:expr) => {
        axum::middleware::from_fn_with_state(
            state.clone(),
            move |state, req, next| {
                $crate::authz::middleware::AuthzRequirement::new($resource, $action)
                    .check(state, req, next)
            },
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn test_derive_resource() {
        let request = Request::builder()
            .uri("/v1/identities/123")
            .body(())
            .unwrap();

        let resource = derive_resource(&request);
        assert_eq!(resource.resource_type, "identities");
        assert_eq!(resource.resource_id, Some("123".to_string()));
    }

    #[test]
    fn test_derive_resource_no_id() {
        let request = Request::builder()
            .uri("/v1/policies")
            .body(())
            .unwrap();

        let resource = derive_resource(&request);
        assert_eq!(resource.resource_type, "policies");
        assert_eq!(resource.resource_id, None);
    }

    #[test]
    fn test_derive_action_get() {
        let request = Request::builder()
            .method(Method::GET)
            .uri("/v1/identities/123")
            .body(())
            .unwrap();

        let action = derive_action(&request);
        assert_eq!(action.action, "read");
    }

    #[test]
    fn test_derive_action_post() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/identities")
            .body(())
            .unwrap();

        let action = derive_action(&request);
        assert_eq!(action.action, "create");
    }

    #[test]
    fn test_derive_action_put() {
        let request = Request::builder()
            .method(Method::PUT)
            .uri("/v1/identities/123")
            .body(())
            .unwrap();

        let action = derive_action(&request);
        assert_eq!(action.action, "update");
    }

    #[test]
    fn test_derive_action_delete() {
        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/v1/identities/123")
            .body(())
            .unwrap();

        let action = derive_action(&request);
        assert_eq!(action.action, "delete");
    }

    #[test]
    fn test_derive_action_authz_check() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/authz/check")
            .body(())
            .unwrap();

        let action = derive_action(&request);
        assert_eq!(action.action, "check");
    }
}
