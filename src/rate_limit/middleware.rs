use crate::errors::AppError;
use crate::rate_limit::limiter::RateLimiter;
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: Arc<Mutex<RateLimiter>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract identifier (IP address, user ID, or API key)
    let identifier = extract_identifier(&headers);

    // Check rate limit
    let mut limiter_guard = limiter.lock().await;
    let result = limiter_guard.check_default_rate_limit(&identifier).await?;
    drop(limiter_guard);

    if !result.allowed {
        tracing::warn!(
            identifier = %identifier,
            limit = %result.limit,
            current = %result.current,
            "Rate limit exceeded"
        );

        return Err(AppError::RateLimitExceeded);
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    add_rate_limit_headers(response.headers_mut(), &result);

    Ok(response)
}

/// Extract identifier from request headers
fn extract_identifier(headers: &HeaderMap) -> String {
    // Try to get user ID from auth header first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            // For now, use the auth header as identifier
            // In a real implementation, we'd extract the user ID from the token
            return format!("user:{}", auth_str.chars().take(20).collect::<String>());
        }
    }

    // Fall back to IP address
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(ip) = forwarded_for.to_str() {
            return format!("ip:{}", ip.split(',').next().unwrap_or("unknown").trim());
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            return format!("ip:{}", ip);
        }
    }

    // Default identifier
    "ip:unknown".to_string()
}

/// Add rate limit headers to response
fn add_rate_limit_headers(headers: &mut HeaderMap, result: &crate::rate_limit::sliding_window::RateLimitResult) {
    use axum::http::header::HeaderName;
    use axum::http::HeaderValue;

    // X-RateLimit-Limit: Maximum number of requests allowed in the window
    if let Ok(value) = HeaderValue::from_str(&result.limit.to_string()) {
        headers.insert(
            HeaderName::from_static("x-ratelimit-limit"),
            value,
        );
    }

    // X-RateLimit-Remaining: Number of requests remaining
    if let Ok(value) = HeaderValue::from_str(&result.remaining.to_string()) {
        headers.insert(
            HeaderName::from_static("x-ratelimit-remaining"),
            value,
        );
    }

    // X-RateLimit-Reset: Unix timestamp when the rate limit resets
    if let Ok(value) = HeaderValue::from_str(&result.reset.to_string()) {
        headers.insert(
            HeaderName::from_static("x-ratelimit-reset"),
            value,
        );
    }

    // Retry-After: Seconds until reset (only if limit exceeded)
    if !result.allowed {
        if let Some(retry_after) = result.retry_after() {
            if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
                headers.insert(
                    HeaderName::from_static("retry-after"),
                    value,
                );
            }
        }
    }
}

/// Auth-specific rate limiting middleware
pub async fn auth_rate_limit_middleware(
    limiter: Arc<Mutex<RateLimiter>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let identifier = extract_identifier(&headers);

    let mut limiter_guard = limiter.lock().await;
    let result = limiter_guard.check_auth_rate_limit(&identifier).await?;
    drop(limiter_guard);

    if !result.allowed {
        tracing::warn!(
            identifier = %identifier,
            limit = %result.limit,
            current = %result.current,
            "Auth rate limit exceeded"
        );

        return Err(AppError::RateLimitExceeded);
    }

    let mut response = next.run(request).await;
    add_rate_limit_headers(response.headers_mut(), &result);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_identifier_from_auth() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer test_token_12345"));

        let identifier = extract_identifier(&headers);
        assert!(identifier.starts_with("user:Bearer test_token"));
    }

    #[test]
    fn test_extract_identifier_from_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.1, 10.0.0.1"));

        let identifier = extract_identifier(&headers);
        assert_eq!(identifier, "ip:192.168.1.1");
    }

    #[test]
    fn test_extract_identifier_from_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("203.0.113.42"));

        let identifier = extract_identifier(&headers);
        assert_eq!(identifier, "ip:203.0.113.42");
    }

    #[test]
    fn test_extract_identifier_default() {
        let headers = HeaderMap::new();
        let identifier = extract_identifier(&headers);
        assert_eq!(identifier, "ip:unknown");
    }
}
