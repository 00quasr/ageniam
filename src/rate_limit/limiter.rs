use crate::config::RateLimitConfig;
use crate::errors::Result;
use crate::rate_limit::sliding_window::{RateLimitResult, SlidingWindowRateLimiter};
use redis::aio::ConnectionManager;

/// Rate limiter for different contexts
pub struct RateLimiter {
    limiter: SlidingWindowRateLimiter,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(redis: ConnectionManager, config: RateLimitConfig) -> Self {
        Self {
            limiter: SlidingWindowRateLimiter::new(redis),
            config,
        }
    }

    /// Check rate limit for authentication endpoints
    pub async fn check_auth_rate_limit(&mut self, identifier: &str) -> Result<RateLimitResult> {
        let key = format!("auth:{}", identifier);
        let limit = self.config.auth_requests_per_minute;
        self.limiter.check_and_increment(&key, limit, 60).await
    }

    /// Check default rate limit (per minute)
    pub async fn check_default_rate_limit(&mut self, identifier: &str) -> Result<RateLimitResult> {
        let key = format!("default:{}", identifier);
        let limit = self.config.default_requests_per_minute;
        self.limiter.check_and_increment(&key, limit, 60).await
    }

    /// Check hourly rate limit
    pub async fn check_hourly_rate_limit(&mut self, identifier: &str) -> Result<RateLimitResult> {
        let key = format!("hourly:{}", identifier);
        let limit = self.config.default_requests_per_hour;
        self.limiter
            .check_and_increment(&key, limit, 3600)
            .await
    }

    /// Check daily rate limit
    pub async fn check_daily_rate_limit(&mut self, identifier: &str) -> Result<RateLimitResult> {
        let key = format!("daily:{}", identifier);
        let limit = self.config.default_requests_per_day;
        self.limiter
            .check_and_increment(&key, limit, 86400)
            .await
    }

    /// Check custom rate limit
    pub async fn check_custom_rate_limit(
        &mut self,
        identifier: &str,
        limit: u64,
        window_seconds: u64,
    ) -> Result<RateLimitResult> {
        self.limiter
            .check_and_increment(identifier, limit, window_seconds)
            .await
    }

    /// Get current count for a key
    pub async fn get_count(&mut self, identifier: &str, window_seconds: u64) -> Result<u64> {
        self.limiter
            .get_current_count(identifier, window_seconds)
            .await
    }

    /// Reset rate limit for a specific identifier
    pub async fn reset(&mut self, identifier: &str) -> Result<()> {
        self.limiter.reset(identifier).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_auth_rate_limit() {
        let config = crate::config::Config::load().unwrap();
        let redis = crate::redis::create_client(&config.redis).await.unwrap();
        let mut limiter = RateLimiter::new(redis, config.rate_limit);

        let result = limiter.check_auth_rate_limit("user@example.com").await.unwrap();
        assert!(result.allowed);

        limiter.reset("auth:user@example.com").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_default_rate_limit() {
        let config = crate::config::Config::load().unwrap();
        let redis = crate::redis::create_client(&config.redis).await.unwrap();
        let mut limiter = RateLimiter::new(redis, config.rate_limit);

        let result = limiter.check_default_rate_limit("test_user").await.unwrap();
        assert!(result.allowed);

        limiter.reset("default:test_user").await.unwrap();
    }
}
