use crate::{config::DatabaseConfig, errors::Result};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

/// Create a PostgreSQL connection pool
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool> {
    tracing::info!("Creating database connection pool");

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
        .connect(&config.url)
        .await?;

    tracing::info!(
        "Database connection pool created with {} max connections",
        config.max_connections
    );

    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations");
    sqlx::migrate!("./src/db/migrations").run(pool).await?;
    tracing::info!("Database migrations completed");
    Ok(())
}

/// Health check for database connection
pub async fn health_check(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
