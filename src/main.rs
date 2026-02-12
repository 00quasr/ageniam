use agent_iam::{
    api::create_router,
    config::Config,
    db::{create_pool, run_migrations},
    observability::init_tracing,
    redis::create_client,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load()?;
    config.validate()?;

    // Initialize tracing/logging
    init_tracing(&config.observability);

    tracing::info!("Starting Agent IAM service");
    tracing::info!("Configuration loaded: {:?}", config.server);

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;
    tracing::info!("Database connection pool created");

    // Run database migrations
    run_migrations(&db_pool).await?;
    tracing::info!("Database migrations completed");

    // Create Redis connection
    let redis_manager = create_client(&config.redis).await?;
    tracing::info!("Redis connection established");

    // Create router
    let app = create_router(db_pool.clone(), redis_manager.clone());

    // Bind server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Listening on http://{}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Agent IAM service is ready to accept requests");

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
