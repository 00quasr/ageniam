use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge, TextEncoder,
};

// Metrics registry
static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    )
    .unwrap()
});

static HTTP_REQUEST_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request latency in seconds",
        &["method", "path"],
        vec![0.001, 0.005, 0.010, 0.050, 0.100, 0.500, 1.0, 5.0]
    )
    .unwrap()
});

static AUTHZ_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "authz_requests_total",
        "Total number of authorization requests",
        &["decision", "resource_type"]
    )
    .unwrap()
});

static AUTHZ_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "authz_latency_seconds",
        "Authorization decision latency in seconds",
        &["decision"],
        vec![0.001, 0.002, 0.005, 0.010, 0.020, 0.050, 0.100]
    )
    .unwrap()
});

static AUTHZ_ERRORS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "authz_policy_evaluation_errors_total",
        "Total number of policy evaluation errors",
        &["error_type"]
    )
    .unwrap()
});

static ACTIVE_SESSIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!("active_sessions", "Number of active sessions").unwrap()
});

static RATE_LIMIT_EXCEEDED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "rate_limit_exceeded_total",
        "Total number of rate limit violations",
        &["tenant_id", "limit_type"]
    )
    .unwrap()
});

pub struct MetricsRecorder;

impl MetricsRecorder {
    pub fn record_http_request(method: &str, path: &str, status: u16) {
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, path, &status.to_string()])
            .inc();
    }

    pub fn record_http_duration(method: &str, path: &str, duration: f64) {
        HTTP_REQUEST_DURATION
            .with_label_values(&[method, path])
            .observe(duration);
    }

    pub fn record_authz_request(decision: &str, resource_type: &str) {
        AUTHZ_REQUESTS_TOTAL
            .with_label_values(&[decision, resource_type])
            .inc();
    }

    pub fn record_authz_latency(decision: &str, duration: f64) {
        AUTHZ_LATENCY.with_label_values(&[decision]).observe(duration);
    }

    pub fn record_authz_error(error_type: &str) {
        AUTHZ_ERRORS_TOTAL.with_label_values(&[error_type]).inc();
    }

    pub fn set_active_sessions(count: i64) {
        ACTIVE_SESSIONS.set(count);
    }

    pub fn record_rate_limit_exceeded(tenant_id: &str, limit_type: &str) {
        RATE_LIMIT_EXCEEDED_TOTAL
            .with_label_values(&[tenant_id, limit_type])
            .inc();
    }

    /// Export all metrics in Prometheus format
    pub fn export() -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        encoder.encode_to_string(&metric_families)
    }
}
