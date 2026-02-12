// Rate limiting counters using Redis sliding window algorithm

use crate::errors::Result;
use redis::{aio::ConnectionManager, AsyncCommands, Script};
use std::time::{SystemTime, UNIX_EPOCH};

const RATE_LIMIT_PREFIX: &str = "ratelimit:";

/// Sliding window rate limiter
pub struct SlidingWindowLimiter {
    manager: ConnectionManager,
}

impl SlidingWindowLimiter {
    pub fn new(manager: ConnectionManager) -> Self {
        Self { manager }
    }

    /// Check and increment rate limit counter
    /// Returns (allowed, current_count, limit)
    pub async fn check_and_increment(
        &mut self,
        key: &str,
        limit: u64,
        window_seconds: u64,
    ) -> Result<(bool, u64, u64)> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - window_seconds;
        let redis_key = format!("{}{}", RATE_LIMIT_PREFIX, key);

        // Lua script for atomic sliding window check
        // This removes old entries, counts current entries, and adds new entry
        let script = Script::new(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])

            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)

            -- Count current entries
            local current = redis.call('ZCARD', key)

            if current < limit then
                -- Add new entry
                redis.call('ZADD', key, now, now)
                redis.call('EXPIRE', key, ARGV[4])
                return {1, current + 1, limit}
            else
                return {0, current, limit}
            end
            "#,
        );

        let result: Vec<u64> = script
            .key(&redis_key)
            .arg(now)
            .arg(window_start)
            .arg(limit)
            .arg(window_seconds)
            .invoke_async(&mut self.manager)
            .await?;

        let allowed = result[0] == 1;
        let current_count = result[1];

        Ok((allowed, current_count, limit))
    }

    /// Get current count without incrementing
    pub async fn get_count(&mut self, key: &str, window_seconds: u64) -> Result<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let window_start = now - window_seconds;
        let redis_key = format!("{}{}", RATE_LIMIT_PREFIX, key);

        // Remove old entries
        self.manager
            .zrembyscore(&redis_key, "-inf", window_start as i64)
            .await?;

        // Count current entries
        let count: u64 = self.manager.zcard(&redis_key).await?;

        Ok(count)
    }

    /// Reset rate limit counter for a key
    pub async fn reset(&mut self, key: &str) -> Result<()> {
        let redis_key = format!("{}{}", RATE_LIMIT_PREFIX, key);
        self.manager.del(&redis_key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Redis instance
    // Run with: cargo test --features integration-tests
    #[tokio::test]
    #[ignore]
    async fn test_sliding_window_limiter() {
        let config = crate::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        };

        let manager = crate::redis::create_client(&config).await.unwrap();
        let mut limiter = SlidingWindowLimiter::new(manager);

        // Test rate limit
        let (allowed, count, limit) = limiter
            .check_and_increment("test_key", 5, 60)
            .await
            .unwrap();

        assert!(allowed);
        assert_eq!(count, 1);
        assert_eq!(limit, 5);

        // Clean up
        limiter.reset("test_key").await.unwrap();
    }
}
