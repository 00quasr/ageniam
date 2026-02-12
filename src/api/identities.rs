// Identity management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::routes::AppState,
    errors::{AppError, Result},
};

/// Response for the delegation chain endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct DelegationChainResponse {
    pub identity_id: Uuid,
    pub chain: Vec<DelegationChainNode>,
}

/// Represents a node in the delegation chain
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DelegationChainNode {
    pub id: Uuid,
    pub identity_type: String,
    pub name: String,
    pub email: Option<String>,
    pub status: String,
    pub parent_identity_id: Option<Uuid>,
    pub task_id: Option<String>,
    pub task_scope: Option<serde_json::Value>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub depth: i32,
}

/// GET /v1/identities/:id/delegation-chain
/// Returns the full delegation chain for an identity, from the requested identity up to the root
#[tracing::instrument(skip(state))]
pub async fn get_delegation_chain(
    State(state): State<AppState>,
    Path(identity_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    tracing::info!("Fetching delegation chain for identity: {}", identity_id);

    // For now, use a hardcoded tenant_id for demonstration
    // In a real implementation, this would come from the authenticated user's context
    // TODO: Extract tenant_id from authentication middleware
    let tenant_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000")
        .map_err(|e| AppError::Internal(format!("Invalid tenant ID: {}", e)))?;

    // Get the delegation chain
    let chain = get_delegation_chain_query(&state.db_pool, identity_id, tenant_id).await?;

    if chain.is_empty() {
        return Err(AppError::IdentityNotFound);
    }

    let response = DelegationChainResponse {
        identity_id,
        chain,
    };

    Ok(Json(response))
}

/// Database query to get the full delegation chain for an identity
/// Uses a recursive CTE to traverse from the given identity up to the root
async fn get_delegation_chain_query(
    pool: &PgPool,
    identity_id: Uuid,
    tenant_id: Uuid,
) -> Result<Vec<DelegationChainNode>> {
    let nodes = sqlx::query_as!(
        DelegationChainNode,
        r#"
        WITH RECURSIVE chain AS (
            -- Start with the requested identity
            SELECT
                id,
                identity_type,
                name,
                email,
                status,
                parent_identity_id,
                task_id,
                task_scope,
                expires_at,
                created_at,
                0 as depth
            FROM identities
            WHERE id = $1 AND tenant_id = $2

            UNION ALL

            -- Recursively join with parent identities
            SELECT
                i.id,
                i.identity_type,
                i.name,
                i.email,
                i.status,
                i.parent_identity_id,
                i.task_id,
                i.task_scope,
                i.expires_at,
                i.created_at,
                c.depth + 1 as depth
            FROM identities i
            INNER JOIN chain c ON i.id = c.parent_identity_id
            WHERE i.tenant_id = $2
        )
        SELECT
            id,
            identity_type,
            name,
            email,
            status,
            parent_identity_id,
            task_id,
            task_scope,
            expires_at,
            created_at,
            depth as "depth!"
        FROM chain
        ORDER BY depth ASC
        "#,
        identity_id,
        tenant_id
    )
    .fetch_all(pool)
    .await?;

    Ok(nodes)
}
