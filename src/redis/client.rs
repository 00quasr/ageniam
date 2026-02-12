use crate::{config::RedisConfig, errors::Result};
use redis::{aio::ConnectionManager, Client};
use std::time::Duration;

/// Create a Redis client and connection manager
pub async fn create_client(config: &RedisConfig) -> Result<ConnectionManager> {
    tracing::info!("Creating Redis client");

    let client = Client::open(config.url.as_str())?;

    let manager = ConnectionManager::new(client).await?;

    tracing::info!("Redis client connected");

    Ok(manager)
}

/// Health check for Redis connection
pub async fn health_check(manager: &mut ConnectionManager) -> Result<()> {
    use redis::AsyncCommands;

    let _: String = manager.ping().await?;
    Ok(())
}
