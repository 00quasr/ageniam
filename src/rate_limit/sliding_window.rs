use crate::errors::Result;
use redis::aio::ConnectionManager;
use std::time::{SystemTime, UNIX_EPOCH};

/// Sliding window rate limiter using Redis sorted sets
pub struct SlidingWindowRateLimiter {
    redis: ConnectionManager,
}

impl SlidingWindowRateLimiter {
    /// Create a new sliding window rate limiter
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Check if a request is allowed and increment the counter
    pub async fn check_and_increment(
        &mut self,
        key: &str,
        limit: u64,
        window_seconds: u64,
    ) -> Result<RateLimitResult> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| crate::errors::AppError::Internal(format!("Time error: {}", e)))?
            .as_secs();

        let window_start = now.saturating_sub(window_seconds);

        tracing::debug!(
            key = %key,
            limit = %limit,
            window_seconds = %window_seconds,
            "Checking rate limit"
        );

        // Use Lua script for atomic operation
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local window_start = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            local window_seconds = tonumber(ARGV[4])

            -- Remove entries outside the sliding window
            redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)

            -- Count current entries in the window
            local current = redis.call('ZCARD', key)

            -- Check if limit is exceeded
            if current < limit then
                -- Add new entry with current timestamp as both score and value
                -- Using timestamp with microsecond precision to ensure uniqueness
                local unique_score = now + (redis.call('TIME')[2] / 1000000)
                redis.call('ZADD', key, unique_score, unique_score)

                -- Set expiration to window size + buffer
                redis.call('EXPIRE', key, window_seconds + 60)

                current = current + 1
                return {1, current, limit - current, now + window_seconds}
            else
                -- Get the oldest timestamp in the window
                local oldest = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                local reset_time = window_start + window_seconds
                if #oldest > 0 then
                    reset_time = math.ceil(tonumber(oldest[2])) + window_seconds
                end

                return {0, current, 0, reset_time}
            end
            "#,
        );

        use redis::AsyncCommands;
        let result: Vec<i64> = script
            .key(key)
            .arg(now)
            .arg(window_start)
            .arg(limit)
            .arg(window_seconds)
            .invoke_async(&mut self.redis)
            .await?;

        let allowed = result[0] == 1;
        let current = result[1] as u64;
        let remaining = result[2] as u64;
        let reset_time = result[3] as u64;

        let rate_limit_result = RateLimitResult {
            allowed,
            limit,
            remaining,
            reset: reset_time,
            current,
        };

        tracing::debug!(
            key = %key,
            allowed = %allowed,
            current = %current,
            remaining = %remaining,
            "Rate limit check result"
        );

        Ok(rate_limit_result)
    }

    /// Get current count without incrementing
    pub async fn get_current_count(&mut self, key: &str, window_seconds: u64) -> Result<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| crate::errors::AppError::Internal(format!("Time error: {}", e)))?
            .as_secs();

        let window_start = now.saturating_sub(window_seconds);

        use redis::AsyncCommands;

        // Remove old entries
        let _: i64 = self
            .redis
            .zrembyscore(key, "-inf", window_start as i64)
            .await?;

        // Count current entries
        let count: u64 = self.redis.zcard(key).await?;

        Ok(count)
    }

    /// Reset rate limit for a specific key
    pub async fn reset(&mut self, key: &str) -> Result<()> {
        use redis::AsyncCommands;
        let _: () = self.redis.del(key).await?;

        tracing::info!(key = %key, "Rate limit reset");

        Ok(())
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// The rate limit (max requests)
    pub limit: u64,
    /// Number of requests remaining in the current window
    pub remaining: u64,
    /// Unix timestamp when the rate limit will reset
    pub reset: u64,
    /// Current number of requests in the window
    pub current: u64,
}

impl RateLimitResult {
    /// Get the number of seconds until the rate limit resets
    pub fn retry_after(&self) -> Option<u64> {
        if !self.allowed {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(self.reset.saturating_sub(now))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_sliding_window_basic() {
        let config = crate::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        };

        let redis = crate::redis::create_client(&config).await.unwrap();
        let mut limiter = SlidingWindowRateLimiter::new(redis);

        let test_key = "test:sliding_window:basic";

        // Clean up first
        limiter.reset(test_key).await.unwrap();

        // First request should be allowed
        let result = limiter.check_and_increment(test_key, 5, 60).await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.current, 1);
        assert_eq!(result.limit, 5);
        assert_eq!(result.remaining, 4);

        // Second request should be allowed
        let result = limiter.check_and_increment(test_key, 5, 60).await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.current, 2);
        assert_eq!(result.remaining, 3);

        // Clean up
        limiter.reset(test_key).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_sliding_window_limit_exceeded() {
        let config = crate::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        };

        let redis = crate::redis::create_client(&config).await.unwrap();
        let mut limiter = SlidingWindowRateLimiter::new(redis);

        let test_key = "test:sliding_window:exceeded";

        // Clean up first
        limiter.reset(test_key).await.unwrap();

        // Make requests up to the limit
        for i in 1..=3 {
            let result = limiter.check_and_increment(test_key, 3, 60).await.unwrap();
            assert!(result.allowed, "Request {} should be allowed", i);
        }

        // Next request should be denied
        let result = limiter.check_and_increment(test_key, 3, 60).await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.current, 3);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after().is_some());

        // Clean up
        limiter.reset(test_key).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_get_current_count() {
        let config = crate::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        };

        let redis = crate::redis::create_client(&config).await.unwrap();
        let mut limiter = SlidingWindowRateLimiter::new(redis);

        let test_key = "test:sliding_window:count";

        // Clean up first
        limiter.reset(test_key).await.unwrap();

        // Make some requests
        limiter.check_and_increment(test_key, 10, 60).await.unwrap();
        limiter.check_and_increment(test_key, 10, 60).await.unwrap();

        // Check count
        let count = limiter.get_current_count(test_key, 60).await.unwrap();
        assert_eq!(count, 2);

        // Clean up
        limiter.reset(test_key).await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_reset() {
        let config = crate::config::RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout_seconds: 5,
        };

        let redis = crate::redis::create_client(&config).await.unwrap();
        let mut limiter = SlidingWindowRateLimiter::new(redis);

        let test_key = "test:sliding_window:reset";

        // Make some requests
        limiter.check_and_increment(test_key, 5, 60).await.unwrap();
        limiter.check_and_increment(test_key, 5, 60).await.unwrap();

        // Reset
        limiter.reset(test_key).await.unwrap();

        // Count should be 0
        let count = limiter.get_current_count(test_key, 60).await.unwrap();
        assert_eq!(count, 0);

        // New request should start fresh
        let result = limiter.check_and_increment(test_key, 5, 60).await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.current, 1);

        // Clean up
        limiter.reset(test_key).await.unwrap();
    }
}
