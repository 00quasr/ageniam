use crate::observability::{HealthChecker, HealthStatus, MetricsRecorder};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;

/// GET /health/live - Liveness probe
#[tracing::instrument(skip(health_checker))]
pub async fn liveness(
    State(health_checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    let status = health_checker.liveness().await;
    Json(status)
}

/// GET /health/ready - Readiness probe
#[tracing::instrument(skip(health_checker))]
pub async fn readiness(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<impl IntoResponse, StatusCode> {
    let status = health_checker.readiness().await;

    if status.status == "ok" {
        Ok(Json(status))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// GET /health/startup - Startup probe
#[tracing::instrument(skip(health_checker))]
pub async fn startup(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<impl IntoResponse, StatusCode> {
    let status = health_checker.startup().await;

    if status.status == "ok" {
        Ok(Json(status))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// GET /metrics - Prometheus metrics
pub async fn metrics() -> Result<String, StatusCode> {
    MetricsRecorder::export().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
