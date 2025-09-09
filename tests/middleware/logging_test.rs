use actix_web::{
    http::StatusCode, 
    test::{self, TestRequest},
    web, App, HttpResponse,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use uuid::Uuid;

use self_sensored::{
    middleware::{StructuredLogger, mask_sensitive_data, mask_sensitive_string, PerformanceTimer, get_request_id},
    config::LoggingConfig,
};

/// Test handler that generates various log events
async fn test_handler() -> HttpResponse {
    tracing::info!(
        event = "test_handler_called",
        message = "Test handler executed successfully"
    );
    HttpResponse::Ok().json(json!({"status": "ok", "timestamp": chrono::Utc::now()}))
}

/// Test handler that simulates an error
async fn error_handler() -> HttpResponse {
    tracing::error!(
        event = "test_error",
        error = "Simulated error for testing",
        message = "Test error handler executed"
    );
    HttpResponse::InternalServerError().json(json!({"error": "Test error"}))
}

/// Test handler with sensitive data
async fn sensitive_handler() -> HttpResponse {
    let sensitive_data = json!({
        "user": "testuser",
        "api_key": "secret123",
        "password": "supersecret",
        "data": {
            "token": "bearer_token_123",
            "value": 42
        }
    });
    
    let masked_data = mask_sensitive_data(sensitive_data.clone());
    tracing::info!(
        event = "sensitive_data_test",
        original_data = ?sensitive_data,
        masked_data = ?masked_data,
        message = "Testing sensitive data masking"
    );
    
    HttpResponse::Ok().json(masked_data)
}

#[actix_web::test]
async fn test_structured_logger_middleware() {
    // Initialize test logging
    let _ = env_logger::builder().is_test(true).try_init();
    
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/test", web::get().to(test_handler))
            .route("/error", web::get().to(error_handler))
    ).await;

    // Test successful request
    let req = TestRequest::get()
        .uri("/test")
        .insert_header(("user-agent", "test-agent"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Test error request  
    let req = TestRequest::get()
        .uri("/error")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[actix_web::test]
async fn test_request_id_propagation() {
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/test", web::get().to(|req: actix_web::HttpRequest| async move {
                let request_id = get_request_id(&req);
                HttpResponse::Ok().json(json!({
                    "request_id": request_id.map(|id| id.to_string())
                }))
            }))
    ).await;

    // Test with provided request ID
    let test_id = Uuid::new_v4();
    let req = TestRequest::get()
        .uri("/test")
        .insert_header(("x-request-id", test_id.to_string()))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Test without request ID (should generate one)
    let req = TestRequest::get()
        .uri("/test")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn test_sensitive_data_masking_json() {
    let sensitive_input = json!({
        "username": "testuser",
        "password": "secret123",
        "api_key": "abc123def456",
        "email": "user@example.com",
        "data": {
            "token": "sensitive_token",
            "secret": "hidden_secret",
            "public_info": "visible_data"
        },
        "items": [
            {
                "auth": "bearer_token", 
                "normal": "normal_value"
            }
        ],
        "nested": {
            "deeper": {
                "credential": "nested_secret",
                "safe": "safe_value"
            }
        }
    });

    let masked = mask_sensitive_data(sensitive_input.clone());
    
    // Check that sensitive fields are masked
    assert_eq!(masked["password"], json!("[MASKED]"));
    assert_eq!(masked["api_key"], json!("[MASKED]"));
    assert_eq!(masked["email"], json!("[MASKED]"));
    assert_eq!(masked["data"]["token"], json!("[MASKED]"));
    assert_eq!(masked["data"]["secret"], json!("[MASKED]"));
    assert_eq!(masked["items"][0]["auth"], json!("[MASKED]"));
    assert_eq!(masked["nested"]["deeper"]["credential"], json!("[MASKED]"));
    
    // Check that non-sensitive fields are preserved
    assert_eq!(masked["username"], json!("testuser"));
    assert_eq!(masked["data"]["public_info"], json!("visible_data"));
    assert_eq!(masked["items"][0]["normal"], json!("normal_value"));
    assert_eq!(masked["nested"]["deeper"]["safe"], json!("safe_value"));
}

#[test]
fn test_sensitive_data_masking_strings() {
    // Test URL parameters
    assert_eq!(
        mask_sensitive_string("https://api.example.com?api_key=secret123&other=value"),
        "https://api.example.com?api_key=[MASKED]&other=value"
    );

    // Test authorization headers
    assert_eq!(
        mask_sensitive_string("Authorization: Bearer token123"),
        "authorization: [MASKED]"
    );
    
    assert_eq!(
        mask_sensitive_string("authorization: Bearer token123"),
        "authorization: [MASKED]"
    );

    // Test password parameters
    assert_eq!(
        mask_sensitive_string("user=john&password=secret&role=admin"),
        "user=john&password=[MASKED]&role=admin"
    );

    // Test multiple sensitive fields
    assert_eq!(
        mask_sensitive_string("token=abc123&key=def456&data=safe"),
        "token=[MASKED]&key=def456&data=safe"
    );

    // Test non-sensitive strings
    assert_eq!(
        mask_sensitive_string("regular string with no secrets"),
        "regular string with no secrets"
    );
}

#[test]
fn test_performance_timer() {
    let timer = PerformanceTimer::new("test_operation", None);
    
    // Simulate some work
    std::thread::sleep(Duration::from_millis(10));
    
    let duration = timer.finish();
    assert!(duration >= 10, "Duration should be at least 10ms, got: {}ms", duration);
    assert!(duration <= 50, "Duration should be reasonable, got: {}ms", duration);
}

#[test]
fn test_performance_timer_with_request_id() {
    let request_id = Uuid::new_v4();
    let timer = PerformanceTimer::new("test_with_id", Some(request_id));
    
    std::thread::sleep(Duration::from_millis(5));
    
    let duration = timer.finish();
    assert!(duration >= 5);
}

#[actix_web::test] 
async fn test_logging_performance_impact() {
    // Test that logging middleware adds minimal overhead
    let app_without_logging = test::init_service(
        App::new()
            .route("/perf", web::get().to(test_handler))
    ).await;
    
    let app_with_logging = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/perf", web::get().to(test_handler))
    ).await;

    // Warm up
    for _ in 0..10 {
        let req = TestRequest::get().uri("/perf").to_request();
        let _ = test::call_service(&app_without_logging, req).await;
        
        let req = TestRequest::get().uri("/perf").to_request();
        let _ = test::call_service(&app_with_logging, req).await;
    }

    // Benchmark without logging
    let start = Instant::now();
    for _ in 0..100 {
        let req = TestRequest::get().uri("/perf").to_request();
        let resp = test::call_service(&app_without_logging, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let duration_without = start.elapsed();

    // Benchmark with logging
    let start = Instant::now();
    for _ in 0..100 {
        let req = TestRequest::get().uri("/perf").to_request();
        let resp = test::call_service(&app_with_logging, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let duration_with = start.elapsed();

    // Calculate overhead
    let overhead = duration_with.saturating_sub(duration_without);
    let overhead_per_request = overhead.as_micros() / 100;
    
    println!("Without logging: {:?}", duration_without);
    println!("With logging: {:?}", duration_with);
    println!("Overhead per request: {}μs", overhead_per_request);
    
    // Assert that overhead is less than 1ms per request (1000 microseconds)
    assert!(
        overhead_per_request < 1000,
        "Logging overhead too high: {}μs per request (should be < 1000μs)",
        overhead_per_request
    );
}

#[test]
fn test_logging_config_from_env() {
    // Test default configuration
    let config = LoggingConfig::default();
    assert!(matches!(config.level, tracing::Level::INFO));
    assert!(config.json_format);
    assert!(!config.pretty_print);
    assert_eq!(config.app_name, "health-export-api");

    // Test environment variable parsing would require setting env vars
    // This is tested in integration tests with different env setups
}

#[test]
fn test_mask_sensitive_edge_cases() {
    // Empty object
    let empty = json!({});
    let masked = mask_sensitive_data(empty.clone());
    assert_eq!(masked, empty);

    // Null values
    let with_nulls = json!({
        "password": null,
        "normal": null
    });
    let masked = mask_sensitive_data(with_nulls);
    assert_eq!(masked["password"], json!("[MASKED]"));
    assert_eq!(masked["normal"], json!(null));

    // Arrays with sensitive data
    let array_data = json!([
        {"api_key": "secret1", "data": "safe1"},
        {"api_key": "secret2", "data": "safe2"}
    ]);
    let masked = mask_sensitive_data(array_data);
    assert_eq!(masked[0]["api_key"], json!("[MASKED]"));
    assert_eq!(masked[1]["api_key"], json!("[MASKED]"));
    assert_eq!(masked[0]["data"], json!("safe1"));
    assert_eq!(masked[1]["data"], json!("safe2"));

    // Case insensitive matching
    let case_test = json!({
        "API_KEY": "secret",
        "Password": "secret",
        "TOKEN": "secret",
        "Normal": "safe"
    });
    let masked = mask_sensitive_data(case_test);
    assert_eq!(masked["API_KEY"], json!("[MASKED]"));
    assert_eq!(masked["Password"], json!("[MASKED]")); 
    assert_eq!(masked["TOKEN"], json!("[MASKED]"));
    assert_eq!(masked["Normal"], json!("safe"));
}

#[test]
fn test_mask_sensitive_partial_matches() {
    // Test partial field name matches
    let partial_data = json!({
        "user_password": "secret",
        "api_key_id": "keyid123",
        "backup_token": "token123",
        "email_address": "user@example.com",
        "username": "safe",
        "description": "safe text"
    });
    let masked = mask_sensitive_data(partial_data);
    assert_eq!(masked["user_password"], json!("[MASKED]"));
    assert_eq!(masked["api_key_id"], json!("[MASKED]"));
    assert_eq!(masked["backup_token"], json!("[MASKED]"));
    assert_eq!(masked["email_address"], json!("[MASKED]"));
    assert_eq!(masked["username"], json!("safe"));
    assert_eq!(masked["description"], json!("safe text"));
}

#[actix_web::test]
async fn test_complex_request_logging() {
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/sensitive", web::post().to(sensitive_handler))
    ).await;

    let sensitive_payload = json!({
        "username": "testuser",
        "password": "secret123",
        "profile": {
            "email": "test@example.com",
            "preferences": {
                "api_key": "user_api_key_123"
            }
        }
    });

    let req = TestRequest::post()
        .uri("/sensitive")
        .insert_header(("content-type", "application/json"))
        .insert_header(("authorization", "Bearer secret_token"))
        .insert_header(("user-agent", "test-client/1.0"))
        .set_json(&sensitive_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Integration test for full logging pipeline
#[actix_web::test]
async fn test_logging_integration() {
    // This test verifies the complete logging pipeline works end-to-end
    let _ = env_logger::builder().is_test(true).try_init();
    
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .route("/health", web::get().to(|| async {
                tracing::info!(
                    event = "health_check",
                    component = "api",
                    status = "healthy",
                    message = "Health check endpoint called"
                );
                HttpResponse::Ok().json(json!({
                    "status": "healthy",
                    "timestamp": chrono::Utc::now(),
                    "version": env!("CARGO_PKG_VERSION")
                }))
            }))
            .route("/metrics", web::get().to(|| async {
                tracing::info!(
                    event = "metrics_request",
                    component = "monitoring",
                    message = "Metrics endpoint called"
                );
                HttpResponse::Ok().json(json!({
                    "requests": 100,
                    "errors": 5,
                    "avg_duration": 250
                }))
            }))
    ).await;

    // Test health endpoint
    let req = TestRequest::get()
        .uri("/health")
        .insert_header(("x-request-id", Uuid::new_v4().to_string()))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Test metrics endpoint  
    let req = TestRequest::get()
        .uri("/metrics")
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}