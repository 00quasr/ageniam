use crate::config::ObservabilityConfig;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing/logging
pub fn init_tracing(config: &ObservabilityConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let registry = tracing_subscriber::registry().with(filter);

    match config.log_format.as_str() {
        "json" => {
            registry
                .with(fmt::layer().json().flatten_event(true))
                .init();
        }
        _ => {
            // Pretty format for development
            registry.with(fmt::layer().pretty()).init();
        }
    }

    tracing::info!(
        "Tracing initialized (level: {}, format: {})",
        config.log_level,
        config.log_format
    );
}
