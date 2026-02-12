use crate::{
    api::{auth, authz, health, identities, policies},
    observability::HealthChecker,
};
use axum::{
    routing::{get, post},
    Router,
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis_manager: ConnectionManager,
    pub health_checker: Arc<HealthChecker>,
}

pub fn create_router(db_pool: PgPool, redis_manager: ConnectionManager) -> Router {
    let health_checker = Arc::new(HealthChecker::new(db_pool.clone(), redis_manager.clone()));

    let state = AppState {
        db_pool,
        redis_manager,
        health_checker: health_checker.clone(),
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health endpoints
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .route("/health/startup", get(health::startup))
        .route("/metrics", get(health::metrics))
        // API v1 routes (to be implemented)
        .nest("/v1", v1_routes())
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        // Add state
        .with_state(state)
}

fn v1_routes() -> Router<AppState> {
    Router::new()
        // Placeholder routes (will be implemented in subsequent tasks)
        .route("/auth/login", post(|| async { "Auth login endpoint" }))
        .route("/auth/logout", post(|| async { "Auth logout endpoint" }))
        .route("/auth/refresh", post(|| async { "Auth refresh endpoint" }))
        .route("/identities", post(|| async { "Create identity endpoint" }))
        .route("/identities/:id", get(|| async { "Get identity endpoint" }))
        .route("/authz/check", post(|| async { "Check authorization endpoint" }))
        .route("/policies", get(|| async { "List policies endpoint" }))
}
