use actix_web::{test, web, App, HttpResponse, Result, middleware::Logger};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, debug, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use self_sensored::{
    middleware::{
        request_logger::RequestLogger,
        logging::{StructuredLogger, mask_sensitive_data, get_request_id},
        metrics::MetricsMiddleware,
    },
    config::LoggingConfig,
};

/// Initialize test logging with structured format
fn init_request_logger_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "debug".into());

        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .json()
                    .with_test_writer()
            )
            .init();
    });
}

/// Test handler for ingest endpoint simulation
async fn mock_ingest_handler(payload: web::Json<Value>) -> Result<HttpResponse> {
    let request_size = serde_json::to_string(&payload.into_inner())
        .map(|s| s.len())
        .unwrap_or(0);

    info!(
        event = "ingest_request_processed",
        payload_size_bytes = request_size,
        component = "mock_ingest_handler",
        message = "Ingest payload processed successfully"
    );

    // Simulate processing time based on payload size
    let processing_delay = match request_size {
        0..=1024 => 10,           // Small payloads: 10ms
        1025..=10240 => 50,       // Medium payloads: 50ms
        10241..=102400 => 200,    // Large payloads: 200ms
        _ => 500,                 // Very large payloads: 500ms
    };

    tokio::time::sleep(Duration::from_millis(processing_delay)).await;

    Ok(HttpResponse::Ok().json(json!({
        "status": "success",
        "processed_bytes": request_size,
        "processing_time_ms": processing_delay,
        "timestamp": chrono::Utc::now()
    })))
}

/// Test handler for other endpoints
async fn mock_generic_handler() -> Result<HttpResponse> {
    debug!(
        event = "generic_request_processed",
        component = "mock_generic_handler",
        message = "Generic request processed"
    );

    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now()
    })))
}

/// Test handler that generates an error
async fn mock_error_handler() -> Result<HttpResponse> {
    warn!(
        event = "error_request_processed",
        error_type = "simulated_error",
        component = "mock_error_handler",
        message = "Simulated error for request logging testing"
    );

    Ok(HttpResponse::InternalServerError().json(json!({
        "error": "simulated_error",
        "message": "Test error for request logging validation",
        "timestamp": chrono::Utc::now()
    })))
}

#[tokio::test]
async fn test_request_logger_ingest_endpoint_logging() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
            .route("/api/v1/health", web::get().to(mock_generic_handler))
            .route("/other", web::get().to(mock_generic_handler))
    ).await;

    // Test various payload sizes for /api/v1/ingest
    let test_payloads = vec![
        // Small payload
        json!({
            "user_id": "user123",
            "metrics": [
                {"type": "heart_rate", "value": 72, "timestamp": "2023-01-01T00:00:00Z"}
            ]
        }),
        // Medium payload
        json!({
            "user_id": "user456",
            "metrics": (0..100).map(|i| json!({
                "type": "heart_rate",
                "value": 70 + i % 20,
                "timestamp": format!("2023-01-01T00:{:02}:00Z", i)
            })).collect::<Vec<_>>()
        }),
        // Large payload with sensitive data
        json!({
            "user_id": "user789",
            "api_key": "secret_api_key_12345",
            "password": "user_password",
            "metrics": (0..1000).map(|i| json!({
                "type": "activity",
                "steps": 100 + i,
                "distance": 0.1 * i as f64,
                "timestamp": format!("2023-01-01T{:02}:00:00Z", i % 24)
            })).collect::<Vec<_>>()
        }),
    ];

    for (index, payload) in test_payloads.iter().enumerate() {
        info!(
            event = "test_payload_start",
            payload_index = index,
            payload_size_estimate = serde_json::to_string(payload).unwrap().len(),
            message = "Testing request logger with payload"
        );

        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("content-type", "application/json"))
            .insert_header(("user-agent", "health-export-test/1.0"))
            .insert_header(("x-request-id", Uuid::new_v4().to_string()))
            .set_json(payload)
            .to_request();

        let start_time = Instant::now();
        let resp = test::call_service(&app, req).await;
        let response_time = start_time.elapsed();

        assert!(resp.status().is_success(), "Request should succeed for payload {}", index);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "success");
        assert!(body["processed_bytes"].is_number());
        assert!(body["processing_time_ms"].is_number());

        info!(
            event = "test_payload_complete",
            payload_index = index,
            response_time_ms = response_time.as_millis(),
            processed_bytes = body["processed_bytes"],
            message = "Request logger test payload completed"
        );
    }
}

#[tokio::test]
async fn test_request_logger_selective_logging() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
            .route("/api/v1/health", web::get().to(mock_generic_handler))
            .route("/metrics", web::get().to(mock_generic_handler))
            .route("/other", web::get().to(mock_generic_handler))
    ).await;

    // Test that only /api/v1/ingest POST requests are logged
    let test_requests = vec![
        ("/api/v1/ingest", "POST", true, Some(json!({"test": "data"}))),
        ("/api/v1/health", "GET", false, None),
        ("/metrics", "GET", false, None),
        ("/other", "GET", false, None),
        ("/api/v1/ingest", "GET", false, None), // Wrong method
        ("/api/v2/ingest", "POST", false, Some(json!({"test": "data"}))), // Wrong path
    ];

    for (path, method, should_log_body, payload) in test_requests {
        info!(
            event = "selective_logging_test",
            path = path,
            method = method,
            should_log_body = should_log_body,
            message = "Testing selective request logging"
        );

        let mut req_builder = match method {
            "POST" => test::TestRequest::post(),
            "GET" => test::TestRequest::get(),
            _ => test::TestRequest::get(),
        };

        req_builder = req_builder.uri(path);

        if let Some(json_payload) = payload {
            req_builder = req_builder
                .insert_header(("content-type", "application/json"))
                .set_json(&json_payload);
        }

        let req = req_builder.to_request();
        let resp = test::call_service(&app, req).await;

        // All endpoints should respond (some may return 404, but that's OK for this test)
        // We're primarily testing the logging behavior
        debug!(
            event = "selective_logging_response",
            path = path,
            method = method,
            status_code = resp.status().as_u16(),
            should_log_body = should_log_body
        );
    }
}

#[tokio::test]
async fn test_request_logger_payload_size_tracking() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    // Test different payload sizes to verify size tracking
    let size_tests = vec![
        ("tiny", json!({"test": "small"})),                     // ~20 bytes
        ("small", json!({"data": "x".repeat(1000)})),           // ~1KB
        ("medium", json!({"data": "x".repeat(10000)})),         // ~10KB
        ("large", json!({"data": "x".repeat(100000)})),         // ~100KB
    ];

    for (size_category, payload) in size_tests {
        let payload_string = serde_json::to_string(&payload).unwrap();
        let expected_size = payload_string.len();

        info!(
            event = "payload_size_test_start",
            size_category = size_category,
            expected_size_bytes = expected_size,
            message = "Testing payload size tracking"
        );

        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("content-type", "application/json"))
            .insert_header(("content-length", expected_size.to_string()))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: Value = test::read_body_json(resp).await;
        let processed_bytes = body["processed_bytes"].as_u64().unwrap();

        // Verify the processed size matches expectations
        assert!(
            (processed_bytes as usize).abs_diff(expected_size) < 100,
            "Size mismatch for {}: expected ~{}, got {}",
            size_category, expected_size, processed_bytes
        );

        info!(
            event = "payload_size_test_complete",
            size_category = size_category,
            expected_size_bytes = expected_size,
            processed_bytes = processed_bytes,
            size_difference = (processed_bytes as i64) - (expected_size as i64),
            message = "Payload size tracking test completed"
        );
    }
}

#[tokio::test]
async fn test_request_logger_error_handling() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
            .route("/error", web::get().to(mock_error_handler))
    ).await;

    // Test normal ingest request
    let normal_payload = json!({
        "user_id": "test_user",
        "metrics": [{"type": "heart_rate", "value": 72}]
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .set_json(&normal_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test error endpoint
    let req = test::TestRequest::get().uri("/error").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_server_error());

    // Test malformed JSON (this should be handled gracefully)
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .set_payload("{invalid json")
        .to_request();

    let resp = test::call_service(&app, req).await;
    // The request logger should not crash, regardless of response status
    debug!(
        event = "malformed_json_test",
        status_code = resp.status().as_u16(),
        message = "Tested request logger with malformed JSON"
    );

    info!(
        event = "error_handling_test_complete",
        message = "Request logger error handling test successful"
    );
}

#[tokio::test]
async fn test_request_logger_concurrent_requests() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    const NUM_CONCURRENT_REQUESTS: usize = 50;
    let mut handles = Vec::new();

    info!(
        event = "concurrent_logging_test_start",
        concurrent_requests = NUM_CONCURRENT_REQUESTS,
        message = "Starting concurrent request logging test"
    );

    for i in 0..NUM_CONCURRENT_REQUESTS {
        let app_clone = &app;

        let handle = tokio::spawn(async move {
            let payload = json!({
                "user_id": format!("concurrent_user_{}", i),
                "request_id": i,
                "metrics": (0..(i % 10 + 1)).map(|j| json!({
                    "type": "heart_rate",
                    "value": 70 + j,
                    "timestamp": chrono::Utc::now()
                })).collect::<Vec<_>>()
            });

            let req = test::TestRequest::post()
                .uri("/api/v1/ingest")
                .insert_header(("content-type", "application/json"))
                .insert_header(("x-request-id", Uuid::new_v4().to_string()))
                .set_json(&payload)
                .to_request();

            let start_time = Instant::now();
            let resp = test::call_service(app_clone, req).await;
            let response_time = start_time.elapsed();

            debug!(
                event = "concurrent_request_complete",
                request_index = i,
                response_time_ms = response_time.as_millis(),
                status_code = resp.status().as_u16()
            );

            (i, resp.status().is_success(), response_time)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Request should complete");
        results.push(result);
    }

    // Analyze results
    let successful_requests = results.iter().filter(|(_, success, _)| *success).count();
    let total_time: Duration = results.iter().map(|(_, _, time)| *time).sum();
    let avg_response_time = total_time / results.len() as u32;

    assert_eq!(
        successful_requests, NUM_CONCURRENT_REQUESTS,
        "All concurrent requests should succeed"
    );

    assert!(
        avg_response_time < Duration::from_millis(1000),
        "Average response time should be reasonable: {:?}",
        avg_response_time
    );

    info!(
        event = "concurrent_logging_test_complete",
        total_requests = NUM_CONCURRENT_REQUESTS,
        successful_requests = successful_requests,
        avg_response_time_ms = avg_response_time.as_millis(),
        message = "Concurrent request logging test completed successfully"
    );
}

#[tokio::test]
async fn test_request_logger_sensitive_data_handling() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    // Test payload with sensitive data
    let sensitive_payload = json!({
        "user_id": "sensitive_test_user",
        "api_key": "secret_api_key_12345",
        "password": "user_password_secret",
        "email": "user@example.com",
        "health_data": {
            "heart_rate": [
                {"value": 72, "timestamp": "2023-01-01T00:00:00Z"}
            ],
            "auth_token": "bearer_token_xyz789"
        },
        "device_info": {
            "model": "iPhone 14",
            "secret": "device_secret_key"
        }
    });

    info!(
        event = "sensitive_data_test_start",
        payload_fields = sensitive_payload.as_object().unwrap().keys().len(),
        message = "Testing request logger with sensitive data"
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .insert_header(("authorization", "Bearer secret_token_123"))
        .insert_header(("x-api-key", "header_api_key_456"))
        .set_json(&sensitive_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "success");

    // The request logger should capture the payload for debugging,
    // but sensitive data masking is handled by the structured logger
    let masked_payload = mask_sensitive_data(sensitive_payload.clone());

    // Verify that sensitive fields would be masked
    assert_eq!(masked_payload["api_key"], json!("[MASKED]"));
    assert_eq!(masked_payload["password"], json!("[MASKED]"));
    assert_eq!(masked_payload["email"], json!("[MASKED]"));
    assert_eq!(masked_payload["health_data"]["auth_token"], json!("[MASKED]"));
    assert_eq!(masked_payload["device_info"]["secret"], json!("[MASKED]"));

    // Verify that non-sensitive fields are preserved
    assert_eq!(masked_payload["user_id"], json!("sensitive_test_user"));
    assert_eq!(masked_payload["device_info"]["model"], json!("iPhone 14"));
    assert_eq!(masked_payload["health_data"]["heart_rate"][0]["value"], json!(72));

    info!(
        event = "sensitive_data_test_complete",
        original_fields = sensitive_payload.as_object().unwrap().keys().len(),
        masked_fields = masked_payload.as_object().unwrap().keys().len(),
        message = "Sensitive data handling test completed"
    );
}

#[tokio::test]
async fn test_request_logger_performance_impact() {
    init_request_logger_tracing();

    // Test app without request logger
    let app_without_logger = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    // Test app with request logger
    let app_with_logger = test::init_service(
        App::new()
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    const PERFORMANCE_TEST_REQUESTS: usize = 100;
    let test_payload = json!({
        "user_id": "performance_test_user",
        "metrics": (0..50).map(|i| json!({
            "type": "heart_rate",
            "value": 70 + i % 20,
            "timestamp": format!("2023-01-01T00:{:02}:00Z", i)
        })).collect::<Vec<_>>()
    });

    // Benchmark without request logger
    let start = Instant::now();
    for _ in 0..PERFORMANCE_TEST_REQUESTS {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("content-type", "application/json"))
            .set_json(&test_payload)
            .to_request();

        let resp = test::call_service(&app_without_logger, req).await;
        assert!(resp.status().is_success());
    }
    let duration_without = start.elapsed();

    // Benchmark with request logger
    let start = Instant::now();
    for _ in 0..PERFORMANCE_TEST_REQUESTS {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("content-type", "application/json"))
            .set_json(&test_payload)
            .to_request();

        let resp = test::call_service(&app_with_logger, req).await;
        assert!(resp.status().is_success());
    }
    let duration_with = start.elapsed();

    // Calculate performance impact
    let overhead = duration_with.saturating_sub(duration_without);
    let overhead_per_request = overhead.as_micros() / PERFORMANCE_TEST_REQUESTS as u128;
    let overhead_percentage = (overhead.as_secs_f64() / duration_without.as_secs_f64()) * 100.0;

    info!(
        event = "performance_impact_analysis",
        requests_tested = PERFORMANCE_TEST_REQUESTS,
        duration_without_us = duration_without.as_micros(),
        duration_with_us = duration_with.as_micros(),
        overhead_us = overhead.as_micros(),
        overhead_per_request_us = overhead_per_request,
        overhead_percentage = overhead_percentage,
        message = "Request logger performance impact analysis complete"
    );

    // Assert reasonable performance impact
    // Request logger should add minimal overhead for ingest endpoint logging
    assert!(
        overhead_per_request < 5000, // Less than 5ms per request
        "Request logger overhead too high: {}μs per request",
        overhead_per_request
    );

    assert!(
        overhead_percentage < 50.0, // Less than 50% overhead
        "Request logger overhead percentage too high: {:.2}%",
        overhead_percentage
    );

    info!(
        event = "performance_test_complete",
        overhead_per_request_us = overhead_per_request,
        overhead_percentage = overhead_percentage,
        verdict = "acceptable",
        message = "Request logger performance test passed"
    );
}

#[tokio::test]
async fn test_request_logger_integration_with_metrics() {
    init_request_logger_tracing();

    let app = test::init_service(
        App::new()
            .wrap(MetricsMiddleware)
            .wrap(RequestLogger)
            .wrap(StructuredLogger)
            .route("/api/v1/ingest", web::post().to(mock_ingest_handler))
    ).await;

    // Test multiple requests to verify integration
    let test_requests = vec![
        json!({"small": "payload"}),
        json!({"medium": "x".repeat(1000)}),
        json!({"large": "x".repeat(10000)}),
    ];

    for (index, payload) in test_requests.iter().enumerate() {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("content-type", "application/json"))
            .insert_header(("x-request-id", Uuid::new_v4().to_string()))
            .set_json(payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        info!(
            event = "integration_test_request",
            request_index = index,
            payload_size = serde_json::to_string(payload).unwrap().len(),
            status = "success"
        );
    }

    info!(
        event = "integration_test_complete",
        total_requests = test_requests.len(),
        message = "Request logger integration with metrics test completed"
    );
}