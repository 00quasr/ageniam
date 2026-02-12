use crate::errors::Result;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    pub database: ComponentStatus,
    pub redis: ComponentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

pub struct HealthChecker {
    db_pool: PgPool,
    redis_manager: ConnectionManager,
}

impl HealthChecker {
    pub fn new(db_pool: PgPool, redis_manager: ConnectionManager) -> Self {
        Self {
            db_pool,
            redis_manager,
        }
    }

    /// Liveness check - is the service running?
    pub async fn liveness(&self) -> HealthStatus {
        HealthStatus {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: HealthChecks {
                database: ComponentStatus {
                    status: "unknown".to_string(),
                    message: None,
                },
                redis: ComponentStatus {
                    status: "unknown".to_string(),
                    message: None,
                },
            },
        }
    }

    /// Readiness check - can the service handle requests?
    pub async fn readiness(&self) -> HealthStatus {
        let db_status = self.check_database().await;
        let redis_status = self.check_redis().await;

        let overall_status = if db_status.status == "ok" && redis_status.status == "ok" {
            "ok"
        } else {
            "degraded"
        };

        HealthStatus {
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks: HealthChecks {
                database: db_status,
                redis: redis_status,
            },
        }
    }

    /// Startup check - has the service finished initializing?
    pub async fn startup(&self) -> HealthStatus {
        self.readiness().await
    }

    async fn check_database(&self) -> ComponentStatus {
        match crate::db::health_check(&self.db_pool).await {
            Ok(_) => ComponentStatus {
                status: "ok".to_string(),
                message: None,
            },
            Err(e) => ComponentStatus {
                status: "error".to_string(),
                message: Some(format!("Database check failed: {}", e)),
            },
        }
    }

    async fn check_redis(&self) -> ComponentStatus {
        let mut manager = self.redis_manager.clone();
        match crate::redis::health_check(&mut manager).await {
            Ok(_) => ComponentStatus {
                status: "ok".to_string(),
                message: None,
            },
            Err(e) => ComponentStatus {
                status: "error".to_string(),
                message: Some(format!("Redis check failed: {}", e)),
            },
        }
    }
}
