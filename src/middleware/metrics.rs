use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::Method,
    Error, HttpResponse, Result,
    web::Bytes,
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

// Payload monitoring metrics for security analysis
static REQUEST_SIZE_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_request_size_bytes",
        "Distribution of HTTP request payload sizes in bytes",
        &["method", "endpoint"],
        // Buckets optimized for health data payloads: 1KB to 200MB
        vec![
            1024.0,      // 1KB - small requests
            10240.0,     // 10KB - typical single metric
            102400.0,    // 100KB - small batch
            1048576.0,   // 1MB - medium batch
            10485760.0,  // 10MB - large batch
            52428800.0,  // 50MB - very large batch
            104857600.0, // 100MB - extremely large batch
            209715200.0, // 200MB - maximum allowed
        ],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create request size histogram")
});

static PROCESSING_DURATION_BY_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec_with_registry!(
        "health_export_processing_duration_seconds",
        "HTTP request processing duration in seconds, labeled by payload size bucket",
        &["method", "endpoint", "size_bucket"],
        // Buckets optimized for processing times based on payload size: 1ms to 300s
        vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create processing duration by size histogram")
});

static LARGE_REQUEST_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_large_request_total",
        "Total number of requests larger than 10MB, for security monitoring",
        &["endpoint", "size_bucket"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create large request counter")
});

static SECURITY_EVENTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec_with_registry!(
        "health_export_security_events_total",
        "Total security events detected during request processing",
        &["event_type", "endpoint", "severity"],
        METRICS_REGISTRY.clone()
    )
    .expect("Failed to create security events counter")
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
        
        // Extract content length for payload monitoring
        let content_length = req.headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        let normalized_endpoint = normalize_endpoint(&path);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            // Calculate request duration with minimal overhead
            let duration = start_time.elapsed();
            let status_code = res.status().as_u16().to_string();

            // Record HTTP metrics - this should be <1ms overhead
            HTTP_REQUESTS_TOTAL
                .with_label_values(&[method.as_str(), normalized_endpoint.as_str(), status_code.as_str()])
                .inc();

            HTTP_REQUEST_DURATION_SECONDS
                .with_label_values(&[method.as_str(), normalized_endpoint.as_str(), status_code.as_str()])
                .observe(duration.as_secs_f64());

            // Record payload size monitoring metrics
            if content_length > 0 {
                // Record request size distribution
                REQUEST_SIZE_BYTES
                    .with_label_values(&[method.as_str(), normalized_endpoint.as_str()])
                    .observe(content_length as f64);

                // Classify payload size and record processing duration
                let size_bucket = classify_payload_size(content_length);
                PROCESSING_DURATION_BY_SIZE
                    .with_label_values(&[method.as_str(), normalized_endpoint.as_str(), size_bucket])
                    .observe(duration.as_secs_f64());

                // Monitor large requests for security analysis
                if content_length > 10 * 1024 * 1024 {  // 10MB threshold
                    LARGE_REQUEST_TOTAL
                        .with_label_values(&[normalized_endpoint.as_str(), size_bucket])
                        .inc();
                    
                    // Log security event for large payloads
                    tracing::warn!(
                        method = %method,
                        endpoint = %normalized_endpoint,
                        content_length = content_length,
                        duration_ms = duration.as_millis(),
                        status_code = %status_code,
                        "Large payload detected - monitoring for potential DoS"
                    );
                    
                    SECURITY_EVENTS_TOTAL
                        .with_label_values(&["large_payload", &normalized_endpoint, "medium"])
                        .inc();
                }

                // Monitor extremely large requests (>100MB)
                if content_length > 100 * 1024 * 1024 {
                    tracing::error!(
                        method = %method,
                        endpoint = %normalized_endpoint,
                        content_length = content_length,
                        duration_ms = duration.as_millis(),
                        status_code = %status_code,
                        "Extremely large payload detected - potential DoS attack"
                    );
                    
                    SECURITY_EVENTS_TOTAL
                        .with_label_values(&["extremely_large_payload", &normalized_endpoint, "high"])
                        .inc();
                }

                // Monitor slow processing of large payloads
                if content_length > 1024 * 1024 && duration.as_secs() > 30 {  // 1MB+ taking >30s
                    tracing::warn!(
                        method = %method,
                        endpoint = %normalized_endpoint,
                        content_length = content_length,
                        duration_ms = duration.as_millis(),
                        status_code = %status_code,
                        "Slow processing of large payload detected"
                    );
                    
                    SECURITY_EVENTS_TOTAL
                        .with_label_values(&["slow_large_payload", &normalized_endpoint, "medium"])
                        .inc();
                }
            }

            Ok(res.map_into_left_body())
        })
    }
}

/// Classify payload size into buckets for metrics labeling
fn classify_payload_size(size_bytes: u64) -> &'static str {
    match size_bytes {
        0..=1024 => "tiny",         // 0-1KB
        1025..=10240 => "small",    // 1KB-10KB
        10241..=102400 => "medium", // 10KB-100KB
        102401..=1048576 => "large", // 100KB-1MB
        1048577..=10485760 => "xlarge", // 1MB-10MB
        10485761..=52428800 => "xxlarge", // 10MB-50MB
        52428801..=104857600 => "huge", // 50MB-100MB
        _ => "massive", // >100MB
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

    /// Record request payload size for monitoring
    #[instrument(skip_all)]
    pub fn record_request_size(method: &str, endpoint: &str, size_bytes: u64) {
        REQUEST_SIZE_BYTES
            .with_label_values(&[method, endpoint])
            .observe(size_bytes as f64);
    }

    /// Record processing duration by payload size bucket
    #[instrument(skip_all)]
    pub fn record_processing_duration_by_size(
        method: &str,
        endpoint: &str,
        size_bytes: u64,
        duration: std::time::Duration,
    ) {
        let size_bucket = classify_payload_size(size_bytes);
        PROCESSING_DURATION_BY_SIZE
            .with_label_values(&[method, endpoint, size_bucket])
            .observe(duration.as_secs_f64());
    }

    /// Record large request for security monitoring
    #[instrument(skip_all)]
    pub fn record_large_request(endpoint: &str, size_bytes: u64) {
        let size_bucket = classify_payload_size(size_bytes);
        LARGE_REQUEST_TOTAL
            .with_label_values(&[endpoint, size_bucket])
            .inc();
    }

    /// Record security event
    #[instrument(skip_all)]
    pub fn record_security_event(event_type: &str, endpoint: &str, severity: &str) {
        SECURITY_EVENTS_TOTAL
            .with_label_values(&[event_type, endpoint, severity])
            .inc();
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
    fn test_classify_payload_size() {
        assert_eq!(classify_payload_size(512), "tiny");
        assert_eq!(classify_payload_size(5120), "small");
        assert_eq!(classify_payload_size(51200), "medium");
        assert_eq!(classify_payload_size(512000), "large");
        assert_eq!(classify_payload_size(5120000), "xlarge");
        assert_eq!(classify_payload_size(26214400), "xxlarge"); // 25MB
        assert_eq!(classify_payload_size(78643200), "huge");    // 75MB
        assert_eq!(classify_payload_size(157286400), "massive"); // 150MB
    }

    #[test]
    fn test_metrics_recording() {
        // Test that metrics can be recorded without panicking
        Metrics::record_ingest_request();
        Metrics::record_metrics_processed("heart_rate", 10, "success");
        Metrics::record_error("validation", "/api/v1/ingest", "warning");
        Metrics::update_active_users_24h(42);

        // Test new payload monitoring metrics
        Metrics::record_request_size("POST", "/api/v1/ingest", 1048576);
        Metrics::record_processing_duration_by_size(
            "POST",
            "/api/v1/ingest",
            1048576,
            std::time::Duration::from_millis(500),
        );
        Metrics::record_large_request("/api/v1/ingest", 52428800);
        Metrics::record_security_event("large_payload", "/api/v1/ingest", "medium");

        // Verify metrics registry is accessible
        let registry = get_metrics_registry();
        assert!(!registry.gather().is_empty());
    }
}
