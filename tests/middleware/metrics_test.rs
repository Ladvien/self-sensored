use std::time::{Duration, Instant};
use actix_web::{test, web, App, HttpResponse, Result};
use prometheus::{Encoder, TextEncoder};
use self_sensored::middleware::metrics::{Metrics, MetricsMiddleware, get_metrics_registry, metrics_handler};

/// Helper function to create a test app with metrics middleware
fn create_test_app() -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
        Config = (),
        InitError = (),
    >,
> {
    App::new()
        .wrap(MetricsMiddleware)
        .route("/test", web::get().to(test_handler))
        .route("/slow", web::get().to(slow_handler))
        .route("/error", web::get().to(error_handler))
        .route("/metrics", web::get().to(metrics_handler))
}

async fn test_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json("success"))
}

async fn slow_handler() -> Result<HttpResponse> {
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(HttpResponse::Ok().json("slow response"))
}

async fn error_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json("error"))
}

#[tokio::test]
async fn test_http_request_metrics_collection() {
    let app = test::init_service(create_test_app()).await;
    
    // Make a successful request
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Make an error request
    let req = test::TestRequest::get().uri("/error").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_server_error());
    
    // Check that metrics were recorded
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    // Find HTTP request metrics
    let http_requests = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_http_requests_total")
        .expect("Should have HTTP requests metric");
    
    let http_duration = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_http_request_duration_seconds")
        .expect("Should have HTTP duration metric");
    
    assert!(!http_requests.get_metric().is_empty());
    assert!(!http_duration.get_metric().is_empty());
    
    // Verify we have metrics for both success and error
    let success_metrics = http_requests
        .get_metric()
        .iter()
        .filter(|m| {
            m.get_label()
                .iter()
                .any(|l| l.get_name() == "status_code" && l.get_value() == "200")
        })
        .count();
    
    let error_metrics = http_requests
        .get_metric()
        .iter()
        .filter(|m| {
            m.get_label()
                .iter()
                .any(|l| l.get_name() == "status_code" && l.get_value() == "500")
        })
        .count();
    
    assert!(success_metrics > 0, "Should have success metrics");
    assert!(error_metrics > 0, "Should have error metrics");
}

#[tokio::test]
async fn test_metrics_endpoint_response() {
    let app = test::init_service(create_test_app()).await;
    
    // Make some requests to generate metrics
    let req = test::TestRequest::get().uri("/test").to_request();
    let _ = test::call_service(&app, req).await;
    
    // Request metrics endpoint
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    
    // Check content type
    let headers = resp.headers();
    let content_type = headers
        .get("content-type")
        .expect("Should have content-type header")
        .to_str()
        .unwrap();
    
    assert_eq!(content_type, "text/plain; version=0.0.4; charset=utf-8");
    
    // Convert response to string and check format
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    
    // Should contain Prometheus format metrics
    assert!(body_str.contains("health_export_http_requests_total"));
    assert!(body_str.contains("health_export_http_request_duration_seconds"));
    assert!(body_str.contains("# HELP"));
    assert!(body_str.contains("# TYPE"));
}

#[tokio::test]
async fn test_middleware_performance_overhead() {
    let app_with_metrics = test::init_service(
        App::new()
            .wrap(MetricsMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;
    
    let app_without_metrics = test::init_service(
        App::new()
            .route("/test", web::get().to(test_handler))
    ).await;
    
    const NUM_REQUESTS: usize = 100;
    
    // Test with metrics middleware
    let start = Instant::now();
    for _ in 0..NUM_REQUESTS {
        let req = test::TestRequest::get().uri("/test").to_request();
        let _ = test::call_service(&app_with_metrics, req).await;
    }
    let with_metrics_duration = start.elapsed();
    
    // Test without metrics middleware
    let start = Instant::now();
    for _ in 0..NUM_REQUESTS {
        let req = test::TestRequest::get().uri("/test").to_request();
        let _ = test::call_service(&app_without_metrics, req).await;
    }
    let without_metrics_duration = start.elapsed();
    
    // Calculate overhead per request
    let overhead_per_request = (with_metrics_duration - without_metrics_duration) / NUM_REQUESTS as u32;
    
    println!(
        "Overhead per request: {:?} (with: {:?}, without: {:?})",
        overhead_per_request, with_metrics_duration, without_metrics_duration
    );
    
    // Verify overhead is less than 1ms per request (requirement from story)
    assert!(
        overhead_per_request < Duration::from_millis(1),
        "Metrics overhead {} is greater than 1ms requirement",
        format!("{:?}", overhead_per_request)
    );
}

#[tokio::test]
async fn test_business_metrics_recording() {
    // Test ingest request recording
    Metrics::record_ingest_request();
    Metrics::record_metrics_processed("heart_rate", 10, "success");
    Metrics::record_metrics_processed("blood_pressure", 5, "failed");
    
    // Test error recording
    Metrics::record_error("validation", "/api/v1/ingest", "warning");
    Metrics::record_error("database", "/api/v1/ingest", "error");
    
    // Test business metrics
    Metrics::update_active_users_24h(42);
    Metrics::record_data_volume("health_data", "ingested", 1024 * 1024); // 1MB
    Metrics::record_health_metrics_stored("sleep", 15);
    
    // Test database metrics
    Metrics::update_db_connection_metrics(20, 5);
    Metrics::record_db_connection_wait_time("ingest", Duration::from_millis(50));
    
    // Test batch processing metrics
    Metrics::record_batch_processing_duration("heart_rate", 100, Duration::from_millis(500));
    
    // Verify metrics were recorded
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    let metric_names: Vec<&str> = metric_families
        .iter()
        .map(|mf| mf.get_name())
        .collect();
    
    // Check that all expected metrics are present
    assert!(metric_names.contains(&"health_export_ingest_requests_total"));
    assert!(metric_names.contains(&"health_export_ingest_metrics_processed_total"));
    assert!(metric_names.contains(&"health_export_errors_total"));
    assert!(metric_names.contains(&"health_export_active_users_24h"));
    assert!(metric_names.contains(&"health_export_data_volume_bytes_total"));
    assert!(metric_names.contains(&"health_export_health_metrics_stored_total"));
    assert!(metric_names.contains(&"health_export_db_connections_active"));
    assert!(metric_names.contains(&"health_export_db_connections_idle"));
    assert!(metric_names.contains(&"health_export_db_connection_wait_time_seconds"));
    assert!(metric_names.contains(&"health_export_batch_processing_duration_seconds"));
}

#[tokio::test]
async fn test_endpoint_normalization() {
    let app = test::init_service(create_test_app()).await;
    
    // Make requests to various endpoints
    let endpoints = vec!["/test", "/unknown-endpoint", "/api/v1/data/heart-rate"];
    
    for endpoint in endpoints {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let _ = test::call_service(&app, req).await;
    }
    
    // Check that metrics were recorded with normalized endpoints
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    let http_requests = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_http_requests_total")
        .expect("Should have HTTP requests metric");
    
    // Check for endpoint normalization
    let endpoints_found: Vec<String> = http_requests
        .get_metric()
        .iter()
        .filter_map(|m| {
            m.get_label()
                .iter()
                .find(|l| l.get_name() == "endpoint")
                .map(|l| l.get_value().to_string())
        })
        .collect();
    
    // Should normalize unknown endpoints to /other
    assert!(endpoints_found.contains(&"/other".to_string()));
    assert!(endpoints_found.contains(&"/api/v1/data/heart-rate".to_string()));
}

#[tokio::test]
async fn test_metric_value_accuracy() {
    // Reset any existing metrics by creating fresh instances
    Metrics::record_ingest_request();
    Metrics::record_ingest_request();
    Metrics::record_ingest_request();
    
    Metrics::record_metrics_processed("test_metric", 25, "success");
    Metrics::record_metrics_processed("test_metric", 10, "failed");
    
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    // Check ingest requests counter
    let ingest_counter = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_ingest_requests_total")
        .expect("Should have ingest requests metric");
    
    let ingest_value = ingest_counter.get_metric()[0].get_counter().get_value();
    assert!(ingest_value >= 3.0, "Should have at least 3 ingest requests");
    
    // Check metrics processed counter
    let processed_counter = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_ingest_metrics_processed_total")
        .expect("Should have metrics processed counter");
    
    let success_metrics = processed_counter
        .get_metric()
        .iter()
        .filter(|m| {
            m.get_label()
                .iter()
                .any(|l| l.get_name() == "status" && l.get_value() == "success")
        })
        .map(|m| m.get_counter().get_value())
        .sum::<f64>();
    
    let failed_metrics = processed_counter
        .get_metric()
        .iter()
        .filter(|m| {
            m.get_label()
                .iter()
                .any(|l| l.get_name() == "status" && l.get_value() == "failed")
        })
        .map(|m| m.get_counter().get_value())
        .sum::<f64>();
    
    assert!(success_metrics >= 25.0, "Should have at least 25 successful metrics");
    assert!(failed_metrics >= 10.0, "Should have at least 10 failed metrics");
}

#[tokio::test] 
async fn test_histogram_buckets() {
    // Record some durations with known values
    Metrics::record_ingest_duration(Duration::from_millis(5), "success");
    Metrics::record_ingest_duration(Duration::from_millis(100), "success");
    Metrics::record_ingest_duration(Duration::from_secs(2), "timeout");
    
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    let duration_histogram = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_ingest_duration_seconds")
        .expect("Should have ingest duration histogram");
    
    // Verify histogram has buckets
    let histogram_metrics: Vec<_> = duration_histogram
        .get_metric()
        .iter()
        .filter(|m| m.has_histogram())
        .collect();
    
    assert!(!histogram_metrics.is_empty(), "Should have histogram metrics");
    
    for metric in histogram_metrics {
        let histogram = metric.get_histogram();
        assert!(histogram.get_bucket().len() > 0, "Should have histogram buckets");
        assert!(histogram.get_sample_count() > 0, "Should have samples in histogram");
    }
}

#[tokio::test]
async fn test_concurrent_metrics_access() {
    use std::sync::Arc;
    use tokio::sync::Barrier;
    
    const NUM_THREADS: usize = 10;
    const REQUESTS_PER_THREAD: usize = 50;
    
    let barrier = Arc::new(Barrier::new(NUM_THREADS));
    let mut handles = Vec::new();
    
    for i in 0..NUM_THREADS {
        let barrier = barrier.clone();
        let handle = tokio::spawn(async move {
            barrier.wait().await;
            
            for j in 0..REQUESTS_PER_THREAD {
                Metrics::record_ingest_request();
                Metrics::record_metrics_processed("concurrent_test", 1, "success");
                Metrics::record_error("test_error", "/test", "info");
                Metrics::record_data_volume("test_data", "concurrent", j as u64);
                
                // Small delay to simulate real work
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.await.expect("Thread should complete successfully");
    }
    
    // Verify metrics were recorded from all threads
    let registry = get_metrics_registry();
    let metric_families = registry.gather().unwrap();
    
    let ingest_counter = metric_families
        .iter()
        .find(|mf| mf.get_name() == "health_export_ingest_requests_total")
        .expect("Should have ingest requests metric");
    
    let total_requests = ingest_counter.get_metric()[0].get_counter().get_value();
    let expected_min = (NUM_THREADS * REQUESTS_PER_THREAD) as f64;
    
    assert!(
        total_requests >= expected_min,
        "Should have at least {} requests, got {}",
        expected_min,
        total_requests
    );
}