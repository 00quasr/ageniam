// Database queries for identities

use crate::db::schema::Identity;
use crate::errors::{AppError, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// Get an identity by email
pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<Identity>> {
    let identity = sqlx::query_as!(
        Identity,
        r#"
        SELECT
            id, tenant_id, identity_type, name, email, status,
            parent_identity_id, task_id, task_scope, expires_at,
            password_hash, api_key_hash, metadata, created_at,
            updated_at, last_login_at
        FROM identities
        WHERE email = $1 AND status = 'active'
        "#,
        email
    )
    .fetch_optional(pool)
    .await?;

    Ok(identity)
}

/// Get an identity by ID
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Identity>> {
    let identity = sqlx::query_as!(
        Identity,
        r#"
        SELECT
            id, tenant_id, identity_type, name, email, status,
            parent_identity_id, task_id, task_scope, expires_at,
            password_hash, api_key_hash, metadata, created_at,
            updated_at, last_login_at
        FROM identities
        WHERE id = $1 AND status = 'active'
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(identity)
}

/// Update last login time for an identity
pub async fn update_last_login(pool: &PgPool, id: Uuid) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE identities
        SET last_login_at = NOW()
        WHERE id = $1
        "#,
        id
    )
    .execute(pool)
    .await?;

    tracing::debug!("Updated last login for identity {}", id);

    Ok(())
}

/// Check if an identity exists by email
pub async fn exists_by_email(pool: &PgPool, email: &str) -> Result<bool> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(SELECT 1 FROM identities WHERE email = $1) as "exists!"
        "#,
        email
    )
    .fetch_one(pool)
    .await?;

    Ok(result.exists)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_get_by_email() {
        let pool = create_test_pool().await;
        let result = get_by_email(&pool, "test@example.com").await;
        assert!(result.is_ok());
    }
}
