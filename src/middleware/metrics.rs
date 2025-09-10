use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::Method,
    Error, HttpResponse, Result,
};
use futures::future::{ok, Ready};
use futures::Future;
use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec_with_registry, register_counter_with_registry,
    register_gauge_vec_with_registry, register_gauge_with_registry,
    register_histogram_vec_with_registry, Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram,
    HistogramVec, Registry, TextEncoder,
};
use tracing::{error, instrument};

// Global metrics registry
static METRICS_REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());

// HTTP request metrics
static HTTP_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_http_requests_total",
        "Total number of HTTP requests",
        &["method", "endpoint", "status_code"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create HTTP requests counter")
});

static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint", "status_code"],
        // Buckets optimized for API response times: 1ms to 10s
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create HTTP request duration histogram")
});

// Processing pipeline metrics
static INGEST_REQUESTS_TOTAL: Lazy<Counter> = Lazy::new(|| {
    register_counter_with_registry!(
        "health_export_ingest_requests_total",
        "Total number of ingest requests processed",
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create ingest requests counter")
});

static INGEST_METRICS_PROCESSED_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_ingest_metrics_processed_total",
        "Total number of health metrics processed",
        &["metric_type", "status"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create ingest metrics processed counter")
});

static INGEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_ingest_duration_seconds",
        "Duration of ingest operations in seconds",
        &["status"],
        // Buckets optimized for batch processing: 1ms to 60s
        vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create ingest duration histogram")
});

static BATCH_PROCESSING_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_batch_processing_duration_seconds",
        "Duration of batch processing operations in seconds",
        &["metric_type", "batch_size_bucket"],
        // Buckets optimized for batch processing: 1ms to 60s
        vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create batch processing duration histogram")
});

// Database connection pool metrics
static DB_CONNECTIONS_ACTIVE: Lazy<Gauge> = Lazy::new(|| {
    register_gauge_with_registry!(
        "health_export_db_connections_active",
        "Number of active database connections",
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create active database connections gauge")
});

static DB_CONNECTIONS_IDLE: Lazy<Gauge> = Lazy::new(|| {
    register_gauge_with_registry!(
        "health_export_db_connections_idle",
        "Number of idle database connections",
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create idle database connections gauge")
});

static DB_CONNECTION_WAIT_TIME_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_db_connection_wait_time_seconds",
        "Time waiting to acquire database connection in seconds",
        &["operation"],
        // Buckets for connection wait times: 1μs to 10s
        vec![0.000001, 0.00001, 0.0001, 0.001, 0.01, 0.1, 1.0, 10.0],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create database connection wait time histogram")
});

// Error tracking metrics
static ERRORS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_errors_total",
        "Total number of errors by type and endpoint",
        &["error_type", "endpoint", "severity"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create errors counter")
});

// Custom business metrics
static ACTIVE_USERS_24H: Lazy<Gauge> = Lazy::new(|| {
    register_gauge_with_registry!(
        "health_export_active_users_24h",
        "Number of active users in the last 24 hours",
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create active users gauge")
});

static DATA_VOLUME_BYTES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_data_volume_bytes_total",
        "Total volume of data processed in bytes",
        &["data_type", "operation"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create data volume counter")
});

static HEALTH_METRICS_STORED_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_health_metrics_stored_total",
        "Total number of health metrics stored by type",
        &["metric_type"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create health metrics stored counter")
});

// Rate limiting metrics
static RATE_LIMITED_REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_rate_limited_requests_total",
        "Total number of rate limited requests",
        &["endpoint", "user_id"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create rate limited requests counter")
});

// Authentication metrics
static AUTH_ATTEMPTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_auth_attempts_total",
        "Total authentication attempts",
        &["result", "key_type"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create auth attempts counter")
});

// Batch processing deduplication metrics
static DUPLICATES_REMOVED_TOTAL: Lazy<Counter> = Lazy::new(|| {
    register_counter_with_registry!(
        "health_export_duplicates_removed_total",
        "Total number of duplicate records removed during batch processing",
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create duplicates removed counter")
});

/// Metrics collection middleware for Prometheus monitoring
pub struct MetricsMiddleware;

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = MetricsMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(MetricsMiddlewareService { service })
    }
}

pub struct MetricsMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for MetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            // Calculate request duration with minimal overhead
            let duration = start_time.elapsed();
            let status_code = res.status().as_u16().to_string();

            // Record HTTP metrics - this should be <1ms overhead
            HTTP_REQUESTS_TOTAL
                .with_label_values(&[&method, &normalize_endpoint(&path), &status_code])
                .inc();

            HTTP_REQUEST_DURATION_SECONDS
                .with_label_values(&[&method, &normalize_endpoint(&path), &status_code])
                .observe(duration.as_secs_f64());

            Ok(res.map_into_left_body())
        })
    }
}

/// Normalize endpoint paths to reduce cardinality
fn normalize_endpoint(path: &str) -> String {
    // Replace UUIDs and IDs with placeholders to prevent high cardinality
    let uuid_regex =
        regex::Regex::new(r"/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .unwrap();
    let id_regex = regex::Regex::new(r"/\d+").unwrap();

    let normalized = uuid_regex.replace_all(path, "/{uuid}");
    let normalized = id_regex.replace_all(&normalized, "/{id}").to_string();

    // Limit to key endpoints to prevent metric explosion
    match normalized.as_str() {
        path if path.starts_with("/api/v1/ingest") => "/api/v1/ingest".to_string(),
        path if path.starts_with("/api/v1/data/") => path.to_string(),
        path if path.starts_with("/api/v1/export/") => path.to_string(),
        path if path.starts_with("/health") => "/health".to_string(),
        path if path.starts_with("/metrics") => "/metrics".to_string(),
        _ => "/other".to_string(),
    }
}

/// Metrics collection functions for other components to use
pub struct Metrics;

impl Metrics {
    /// Record ingest request processing
    #[instrument(skip_all)]
    pub fn record_ingest_request() {
        INGEST_REQUESTS_TOTAL.inc();
    }

    /// Record metrics processed during ingest
    #[instrument(skip_all)]
    pub fn record_metrics_processed(metric_type: &str, count: u64, status: &str) {
        INGEST_METRICS_PROCESSED_TOTAL
            .with_label_values(&[metric_type, status])
            .inc_by(count as f64);
    }

    /// Record ingest processing duration
    #[instrument(skip_all)]
    pub fn record_ingest_duration(duration: std::time::Duration, status: &str) {
        INGEST_DURATION_SECONDS
            .with_label_values(&[status])
            .observe(duration.as_secs_f64());
    }

    /// Record batch processing duration
    #[instrument(skip_all)]
    pub fn record_batch_processing_duration(
        metric_type: &str,
        batch_size: usize,
        duration: std::time::Duration,
    ) {
        let batch_size_bucket = match batch_size {
            0..=10 => "small",
            11..=100 => "medium",
            101..=1000 => "large",
            _ => "xlarge",
        };

        BATCH_PROCESSING_DURATION_SECONDS
            .with_label_values(&[metric_type, batch_size_bucket])
            .observe(duration.as_secs_f64());
    }

    /// Update database connection pool metrics
    #[instrument(skip_all)]
    pub fn update_db_connection_metrics(active: u64, idle: u64) {
        DB_CONNECTIONS_ACTIVE.set(active as f64);
        DB_CONNECTIONS_IDLE.set(idle as f64);
    }

    /// Record database connection wait time
    #[instrument(skip_all)]
    pub fn record_db_connection_wait_time(operation: &str, duration: std::time::Duration) {
        DB_CONNECTION_WAIT_TIME_SECONDS
            .with_label_values(&[operation])
            .observe(duration.as_secs_f64());
    }

    /// Record error occurrence
    #[instrument(skip_all)]
    pub fn record_error(error_type: &str, endpoint: &str, severity: &str) {
        ERRORS_TOTAL
            .with_label_values(&[error_type, endpoint, severity])
            .inc();
    }

    /// Update active users count
    #[instrument(skip_all)]
    pub fn update_active_users_24h(count: u64) {
        ACTIVE_USERS_24H.set(count as f64);
    }

    /// Record data volume processed
    #[instrument(skip_all)]
    pub fn record_data_volume(data_type: &str, operation: &str, bytes: u64) {
        DATA_VOLUME_BYTES
            .with_label_values(&[data_type, operation])
            .inc_by(bytes as f64);
    }

    /// Record health metrics stored
    #[instrument(skip_all)]
    pub fn record_health_metrics_stored(metric_type: &str, count: u64) {
        HEALTH_METRICS_STORED_TOTAL
            .with_label_values(&[metric_type])
            .inc_by(count as f64);
    }

    /// Record rate limited request
    #[instrument(skip_all)]
    pub fn record_rate_limited_request(endpoint: &str, user_id: &str) {
        RATE_LIMITED_REQUESTS_TOTAL
            .with_label_values(&[endpoint, user_id])
            .inc();
    }

    /// Record authentication attempt
    #[instrument(skip_all)]
    pub fn record_auth_attempt(result: &str, key_type: &str) {
        AUTH_ATTEMPTS_TOTAL
            .with_label_values(&[result, key_type])
            .inc();
    }

    /// Record duplicates removed during batch processing
    #[instrument(skip_all)]
    pub fn record_duplicates_removed(count: u64) {
        DUPLICATES_REMOVED_TOTAL.inc_by(count as f64);
    }
}

/// Handler for Prometheus metrics endpoint
#[instrument]
pub async fn metrics_handler() -> Result<HttpResponse> {
    let encoder = TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();

    match encoder.encode_to_string(&metric_families) {
        Ok(output) => Ok(HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(output)),
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to encode metrics",
                "message": e.to_string()
            })))
        }
    }
}

/// Get metrics registry for testing
#[cfg(test)]
pub fn get_metrics_registry() -> &'static Registry {
    &METRICS_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_endpoint() {
        assert_eq!(normalize_endpoint("/api/v1/ingest"), "/api/v1/ingest");
        assert_eq!(
            normalize_endpoint("/api/v1/data/heart-rate"),
            "/api/v1/data/heart-rate"
        );
        assert_eq!(normalize_endpoint("/health"), "/health");
        assert_eq!(normalize_endpoint("/metrics"), "/metrics");
        assert_eq!(normalize_endpoint("/unknown/endpoint"), "/other");
    }

    #[test]
    fn test_metrics_recording() {
        // Test that metrics can be recorded without panicking
        Metrics::record_ingest_request();
        Metrics::record_metrics_processed("heart_rate", 10, "success");
        Metrics::record_error("validation", "/api/v1/ingest", "warning");
        Metrics::update_active_users_24h(42);

        // Verify metrics registry is accessible
        let registry = get_metrics_registry();
        assert!(!registry.gather().is_empty());
    }
}
