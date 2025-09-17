use actix_web::{test, web, App, HttpResponse};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::{ApiResponse, IngestResponse};
use self_sensored::services::auth::AuthService;

mod common;
use common::{cleanup_test_db, setup_test_db};

/// Test helper to create a user and return user_id
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Test helper to create an API key and return the key string
async fn create_test_api_key(pool: &PgPool, user_id: Uuid) -> String {
    let api_key = "test_api_key_12345678901234567890123456789012"; // 32 chars
    let api_key_id = Uuid::new_v4();

    // Hash the API key using Argon2 (same as production)
    let salt = b"test_salt_16byte"; // 16 bytes
    let config = argon2::Config::default();
    let hashed_key =
        argon2::hash_encoded(api_key.as_bytes(), salt, &config).expect("Failed to hash API key");

    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, permissions, created_at, last_used_at)
        VALUES ($1, $2, $3, 'Test Key', '{}', NOW(), NOW())
        "#,
        api_key_id,
        user_id,
        hashed_key
    )
    .execute(pool)
    .await
    .expect("Failed to create test API key");

    api_key.to_string()
}

/// Create a large JSON payload that exceeds the 10MB threshold for async processing
fn create_large_payload() -> Value {
    let mut metrics = Vec::new();

    // Create enough heart rate metrics to exceed 10MB
    // Each metric is roughly 200-300 bytes, so we need ~35,000-50,000 metrics
    for i in 0..45000 {
        metrics.push(json!({
            "metric_type": "HeartRate",
            "recorded_at": "2024-01-01T12:00:00Z",
            "heart_rate": 70 + (i % 30), // Vary between 70-100 BPM
            "resting_heart_rate": 60,
            "heart_rate_variability": 25.5,
            "context": "resting",
            "source_device": format!("Apple Watch Series 8 - Test Device {}", i)
        }));
    }

    json!({
        "data": {
            "metrics": metrics,
            "workouts": []
        }
    })
}

/// Create a smaller payload for synchronous processing
fn create_small_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "metric_type": "HeartRate",
                    "recorded_at": "2024-01-01T12:00:00Z",
                    "heart_rate": 75,
                    "resting_heart_rate": 60,
                    "heart_rate_variability": 25.5,
                    "context": "resting",
                    "source_device": "Apple Watch Series 8"
                }
            ],
            "workouts": []
        }
    })
}

#[actix_web::test]
async fn test_async_processing_response_fields_large_payload() {
    // Setup
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Create large payload that will trigger async processing (>10MB)
    let large_payload = create_large_payload();
    let payload_json = serde_json::to_string(&large_payload).unwrap();

    // Verify payload is large enough to trigger async processing
    let payload_size_mb = payload_json.len() as f64 / (1024.0 * 1024.0);
    assert!(
        payload_size_mb > 10.0,
        "Payload must be >10MB to trigger async processing, got {:.1}MB",
        payload_size_mb
    );

    // Create app with auth middleware
    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Make request with large payload
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(payload_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // CRITICAL TEST: Verify HTTP 202 Accepted status for async processing
    assert_eq!(
        resp.status(),
        202,
        "Large payload should return HTTP 202 Accepted"
    );

    // Parse response body
    let body_bytes = test::read_body(resp).await;
    let response_text = String::from_utf8(body_bytes.to_vec()).expect("Response should be valid UTF-8");
    let api_response: ApiResponse<IngestResponse> =
        serde_json::from_str(&response_text).expect("Response should be valid JSON");

    // CRITICAL TEST: Verify response fields for async processing
    assert!(
        api_response.success,
        "API response success should be true for accepted async processing"
    );

    let ingest_response = api_response.data.expect("Response should contain data");

    // STORY-EMERGENCY-003: Critical async processing response fields
    assert_eq!(
        ingest_response.success, true,
        "IngestResponse.success should be true - request was accepted"
    );
    assert_eq!(
        ingest_response.processed_count, 0,
        "processed_count should be 0 - no metrics processed yet"
    );
    assert_eq!(
        ingest_response.failed_count, 0,
        "failed_count should be 0 - processing hasn't started"
    );
    assert!(
        ingest_response.errors.is_empty(),
        "errors should be empty - processing hasn't started"
    );

    // Verify async-specific fields
    assert_eq!(
        ingest_response.processing_status.as_ref().unwrap(),
        "accepted_for_processing",
        "processing_status should indicate async processing"
    );
    assert!(
        ingest_response.raw_ingestion_id.is_some(),
        "raw_ingestion_id should be provided for status tracking"
    );

    // Verify processing time is very fast (< 5 seconds for acceptance)
    assert!(
        ingest_response.processing_time_ms < 5000,
        "Async acceptance should be fast, got {}ms",
        ingest_response.processing_time_ms
    );

    // Verify message is clear about async nature
    let message = api_response
        .error
        .expect("Response should contain informational message");
    assert!(
        message.contains("accepted for background processing"),
        "Message should clearly indicate background processing: {}",
        message
    );
    assert!(
        message.contains("raw_ingestion_id"),
        "Message should reference raw_ingestion_id for status checking: {}",
        message
    );

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_synchronous_processing_response_fields_small_payload() {
    // Setup
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Create small payload for synchronous processing
    let small_payload = create_small_payload();

    // Create app with auth middleware
    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Make request with small payload
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .set_json(&small_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Verify HTTP 200 OK for synchronous processing
    assert_eq!(
        resp.status(),
        200,
        "Small payload should return HTTP 200 OK"
    );

    // Parse response body
    let body_bytes = test::read_body(resp).await;
    let response_text = String::from_utf8(body_bytes.to_vec()).expect("Response should be valid UTF-8");
    let api_response: ApiResponse<IngestResponse> =
        serde_json::from_str(&response_text).expect("Response should be valid JSON");

    // Verify response fields for synchronous processing
    assert!(
        api_response.success,
        "API response success should be true for successful sync processing"
    );

    let ingest_response = api_response.data.expect("Response should contain data");

    // Synchronous processing response verification
    assert_eq!(
        ingest_response.success, true,
        "IngestResponse.success should be true - metrics were processed"
    );
    assert_eq!(
        ingest_response.processed_count, 1,
        "processed_count should be 1 - one metric processed"
    );
    assert_eq!(
        ingest_response.failed_count, 0,
        "failed_count should be 0 - no failures"
    );
    assert!(
        ingest_response.errors.is_empty(),
        "errors should be empty - no processing errors"
    );

    // Verify sync-specific fields
    assert_eq!(
        ingest_response.processing_status.as_ref().unwrap(),
        "processed",
        "processing_status should indicate completed processing"
    );
    assert!(
        ingest_response.raw_ingestion_id.is_some(),
        "raw_ingestion_id should be provided"
    );

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_async_vs_sync_response_difference() {
    // Setup
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Create app with auth middleware
    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Test 1: Small payload (synchronous)
    let small_payload = create_small_payload();
    let req_small = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
        .set_json(&small_payload)
        .to_request();

    let resp_small = test::call_service(&app, req_small).await;
    let body_small: actix_web::body::MessageBody = test::read_body(resp_small).await;
    let response_small: ApiResponse<IngestResponse> =
        serde_json::from_str(&String::from_utf8(body_small.to_vec()).unwrap()).unwrap();

    // Test 2: Large payload (asynchronous)
    let large_payload = create_large_payload();
    let payload_json = serde_json::to_string(&large_payload).unwrap();

    let req_large = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(payload_json)
        .to_request();

    let resp_large = test::call_service(&app, req_large).await;
    let body_large: actix_web::body::MessageBody = test::read_body(resp_large).await;
    let response_large: ApiResponse<IngestResponse> =
        serde_json::from_str(&String::from_utf8(body_large.to_vec()).unwrap()).unwrap();

    // CRITICAL COMPARISON: Verify different response patterns
    let small_data = response_small.data.unwrap();
    let large_data = response_large.data.unwrap();

    // Both should indicate success (request acceptance)
    assert_eq!(
        small_data.success, true,
        "Small payload: success should be true"
    );
    assert_eq!(
        large_data.success, true,
        "Large payload: success should be true"
    );

    // But processed_count should differ
    assert_eq!(
        small_data.processed_count, 1,
        "Small payload: should process 1 metric immediately"
    );
    assert_eq!(
        large_data.processed_count, 0,
        "Large payload: should not process any metrics yet"
    );

    // Processing status should differ
    assert_eq!(
        small_data.processing_status.as_ref().unwrap(),
        "processed",
        "Small payload: should be processed"
    );
    assert_eq!(
        large_data.processing_status.as_ref().unwrap(),
        "accepted_for_processing",
        "Large payload: should be accepted for processing"
    );

    // Both should have raw_ingestion_id
    assert!(
        small_data.raw_ingestion_id.is_some(),
        "Small payload: should have raw_ingestion_id"
    );
    assert!(
        large_data.raw_ingestion_id.is_some(),
        "Large payload: should have raw_ingestion_id"
    );

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_async_processing_database_state() {
    // Setup
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Create large payload that will trigger async processing
    let large_payload = create_large_payload();
    let payload_json = serde_json::to_string(&large_payload).unwrap();

    // Create app with auth middleware
    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Make request with large payload
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(payload_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Parse response to get raw_ingestion_id
    let body_bytes = test::read_body(resp).await;
    let response_text = String::from_utf8(body_bytes.to_vec()).expect("Response should be valid UTF-8");
    let api_response: ApiResponse<IngestResponse> =
        serde_json::from_str(&response_text).expect("Response should be valid JSON");

    let ingest_response = api_response.data.unwrap();
    let raw_ingestion_id = ingest_response.raw_ingestion_id.unwrap();

    // Verify raw_ingestion record was created immediately
    let raw_ingestion = sqlx::query!(
        "SELECT id, processing_status, processed_at FROM raw_ingestions WHERE id = $1",
        raw_ingestion_id
    )
    .fetch_one(&pool)
    .await
    .expect("Raw ingestion record should exist");

    assert_eq!(
        raw_ingestion.id, raw_ingestion_id,
        "Raw ingestion ID should match"
    );
    assert_eq!(
        raw_ingestion.processing_status, Some("parsing".to_string()),
        "Initial processing status should be 'parsing' for async processing"
    );
    assert!(
        raw_ingestion.processed_at.is_some(),
        "processed_at should be set when record is created"
    );

    // Verify no metrics have been processed yet (they should be processed in background)
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Should be able to query heart rate metrics");

    assert_eq!(
        heart_rate_count.count.unwrap_or(0),
        0,
        "No heart rate metrics should be processed immediately for async processing"
    );

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}
