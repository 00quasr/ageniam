// Database queries for sessions

use crate::db::schema::Session;
use crate::errors::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new session
pub async fn create(
    pool: &PgPool,
    identity_id: Uuid,
    tenant_id: Uuid,
    token_id: String,
    token_type: &str,
    expires_at: DateTime<Utc>,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<Session> {
    let session = sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (
            identity_id, tenant_id, token_id, token_type,
            expires_at, ip_address, user_agent
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING
            id, identity_id, tenant_id, token_id, token_type,
            scope, delegation_chain, created_at, expires_at,
            revoked_at, last_used_at, ip_address, user_agent, metadata
        "#,
        identity_id,
        tenant_id,
        token_id,
        token_type,
        expires_at,
        ip_address.as_ref().map(|s| s.parse::<std::net::IpAddr>().ok()).flatten(),
        user_agent
    )
    .fetch_one(pool)
    .await?;

    tracing::info!(
        "Created session {} for identity {} (type: {})",
        session.id,
        identity_id,
        token_type
    );

    Ok(session)
}

/// Get a session by token ID
pub async fn get_by_token_id(pool: &PgPool, token_id: &str) -> Result<Option<Session>> {
    let session = sqlx::query_as!(
        Session,
        r#"
        SELECT
            id, identity_id, tenant_id, token_id, token_type,
            scope, delegation_chain, created_at, expires_at,
            revoked_at, last_used_at, ip_address, user_agent, metadata
        FROM sessions
        WHERE token_id = $1 AND revoked_at IS NULL
        "#,
        token_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(session)
}

/// Revoke a session by token ID
pub async fn revoke(pool: &PgPool, token_id: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE token_id = $1
        "#,
        token_id
    )
    .execute(pool)
    .await?;

    tracing::info!("Revoked session with token_id {}", token_id);

    Ok(())
}

/// Revoke all sessions for an identity
pub async fn revoke_all_for_identity(pool: &PgPool, identity_id: Uuid) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE identity_id = $1 AND revoked_at IS NULL
        "#,
        identity_id
    )
    .execute(pool)
    .await?;

    tracing::info!(
        "Revoked {} sessions for identity {}",
        result.rows_affected(),
        identity_id
    );

    Ok(result.rows_affected())
}

/// Update last used time for a session
pub async fn update_last_used(pool: &PgPool, token_id: &str) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE sessions
        SET last_used_at = NOW()
        WHERE token_id = $1
        "#,
        token_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Clean up expired sessions (older than retention period)
pub async fn cleanup_expired(pool: &PgPool, retention_days: i32) -> Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM sessions
        WHERE expires_at < NOW() - ($1 || ' days')::INTERVAL
        OR revoked_at < NOW() - ($1 || ' days')::INTERVAL
        "#,
        retention_days
    )
    .execute(pool)
    .await?;

    tracing::info!("Cleaned up {} expired sessions", result.rows_affected());

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use sqlx::postgres::PgPoolOptions;

    async fn create_test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/agent_iam_test".to_string());

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to create test pool")
    }

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_create_session() {
        let pool = create_test_pool().await;

        let identity_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let token_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(1);

        let result = create(
            &pool,
            identity_id,
            tenant_id,
            token_id.clone(),
            "jwt",
            expires_at,
            Some("127.0.0.1".to_string()),
            Some("test-agent".to_string()),
        )
        .await;

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.token_id, token_id);
    }
}
