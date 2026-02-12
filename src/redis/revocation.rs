// Token revocation list using Redis

use crate::errors::Result;
use redis::{aio::ConnectionManager, AsyncCommands};

const REVOCATION_PREFIX: &str = "revoked:";

/// Add a token to the revocation list
pub async fn revoke_token(
    manager: &mut ConnectionManager,
    token_id: &str,
    ttl_seconds: i64,
) -> Result<()> {
    let key = format!("{}{}", REVOCATION_PREFIX, token_id);
    manager.set_ex(&key, "1", ttl_seconds as u64).await?;
    Ok(())
}

/// Check if a token is revoked
pub async fn is_token_revoked(
    manager: &mut ConnectionManager,
    token_id: &str,
) -> Result<bool> {
    let key = format!("{}{}", REVOCATION_PREFIX, token_id);
    let exists: bool = manager.exists(&key).await?;
    Ok(exists)
}

/// Remove a token from the revocation list (when it expires naturally)
pub async fn unrevoke_token(
    manager: &mut ConnectionManager,
    token_id: &str,
) -> Result<()> {
    let key = format!("{}{}", REVOCATION_PREFIX, token_id);
    manager.del(&key).await?;
    Ok(())
}
