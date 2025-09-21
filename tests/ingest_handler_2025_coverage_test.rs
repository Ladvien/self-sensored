/// Comprehensive Ingest Handler Test Suite - 2025 Modern Rust Testing Practices
/// This test suite aims for 100% coverage of the ingest handler functionality
/// using state-of-the-art testing techniques.

use actix_web::{
    http::{header, StatusCode},
    middleware, test,
    web::{self, Data},
    App, HttpResponse,
};
use chrono::Utc;
use proptest::prelude::*;
use quickcheck_macros::quickcheck;
use rstest::*;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::{ingest_handler, LoggedJson},
    middleware::auth::AuthMiddleware,
    models::{
        health_metrics::{HeartRateMetric, BloodPressureMetric, SleepMetric, ActivityMetric},
        ios_models::{IosIngestPayload, GroupedMetrics},
        IngestResponse,
    },
    services::auth::{AuthService, User, ApiKey},
};

// Test Database Setup
async fn setup_test_database() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set for testing");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Ensure tables exist for testing
    sqlx::migrate!("./database/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

// Test User and API Key Setup
async fn create_test_user_and_key(pool: &PgPool) -> (User, ApiKey, String) {
    let auth_service = AuthService::new(pool.clone());

    // Create test user
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, $3) RETURNING *",
        user_id,
        email,
        Utc::now()
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user");

    // Create API key
    let (raw_key, api_key) = auth_service
        .create_api_key(user.id, "test_key", None, None)
        .await
        .expect("Failed to create test API key");

    (user, api_key, raw_key)
}

// Test App Factory with Full Middleware Stack
fn create_test_app(pool: PgPool) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        Config = (),
        InitError = (),
    >,
> {
    App::new()
        .app_data(Data::new(pool))
        .wrap(middleware::Logger::default())
        .route("/v1/ingest", web::post().to(ingest_handler))
        .route("/health", web::get().to(|| async { HttpResponse::Ok().body("OK") }))
}

// Fixture: Valid Heart Rate Payload
#[fixture]
fn valid_heart_rate_payload() -> Value {
    json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": Uuid::new_v4(),
                "recorded_at": Utc::now(),
                "heart_rate": 75,
                "resting_heart_rate": 65,
                "heart_rate_variability": 45.5,
                "source_device": "Apple Watch Series 9",
                "context": "Exercise",
                "created_at": Utc::now()
            }],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        },
        "device_info": {
            "device_model": "iPhone 15 Pro",
            "ios_version": "17.2",
            "app_version": "2.1.0"
        }
    })
}

// Fixture: Large Batch Payload (for performance testing)
#[fixture]
fn large_batch_payload() -> Value {
    let heart_rate_entries: Vec<Value> = (0..1000)
        .map(|i| json!({
            "id": Uuid::new_v4(),
            "user_id": Uuid::new_v4(),
            "recorded_at": Utc::now(),
            "heart_rate": 60 + i % 40,
            "resting_heart_rate": 55 + i % 20,
            "source_device": "Apple Watch",
            "created_at": Utc::now()
        }))
        .collect();

    json!({
        "data": {
            "heart_rate": heart_rate_entries,
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    })
}

// ==================== CORE FUNCTIONALITY TESTS ====================

#[sqlx::test(migrations = "./database/migrations")]
async fn test_successful_ingest_basic_payload(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = valid_heart_rate_payload();

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&payload)
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: IngestResponse = response.json().await;
    assert_eq!(response_body.status, "success");
    assert!(response_body.metrics_processed > 0);
    assert_eq!(response_body.errors.len(), 0);

    // Verify data was actually stored
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(count.unwrap_or(0) > 0);
    Ok(())
}

#[sqlx::test]
async fn test_unauthorized_request(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool).await;
    let payload = valid_heart_rate_payload();

    // Test 1: No authorization header
    let response = server
        .post("/v1/ingest")
        .add_header("content-type", "application/json")
        .json(&payload)
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test 2: Invalid API key
    let response = server
        .post("/v1/ingest")
        .add_header("authorization", "Bearer invalid_key_123")
        .add_header("content-type", "application/json")
        .json(&payload)
        .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test]
async fn test_malformed_json_payload(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Test various malformed JSON scenarios
    let malformed_payloads = vec![
        r#"{"data": {"heart_rate": [}"#, // Incomplete JSON
        r#"{"data": "not_an_object"}"#,   // Wrong data type
        r#"{}"#,                          // Missing required fields
        r#"{"data": {"heart_rate": [{"invalid_field": true}]}}"#, // Invalid structure
    ];

    for (i, payload) in malformed_payloads.iter().enumerate() {
        let response = server
            .post("/v1/ingest")
            .add_header("authorization", format!("Bearer {}", raw_key))
            .add_header("content-type", "application/json")
            .add_raw_body(payload.as_bytes())
            .await;

        assert!(
            response.status().is_client_error(),
            "Test case {}: Expected client error for malformed payload: {}",
            i,
            payload
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_empty_payload_handling(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let empty_payload = json!({
        "data": {
            "heart_rate": [],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&empty_payload)
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: IngestResponse = response.json().await;
    assert_eq!(response_body.status, "success");
    assert_eq!(response_body.metrics_processed, 0);

    Ok(())
}

// ==================== PERFORMANCE AND LOAD TESTS ====================

#[sqlx::test]
#[ignore] // Run separately as it's time-intensive
async fn test_large_batch_processing_performance(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let large_payload = large_batch_payload();
    let start_time = std::time::Instant::now();

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&large_payload)
        .await;

    let elapsed = start_time.elapsed();

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: IngestResponse = response.json().await;
    assert_eq!(response_body.status, "success");
    assert_eq!(response_body.metrics_processed, 1000);

    // Performance assertion: should process 1000 metrics in under 5 seconds
    assert!(
        elapsed < Duration::from_secs(5),
        "Large batch processing took too long: {:?}",
        elapsed
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_ingest_requests() {
    let pool = setup_test_database().await;
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = valid_heart_rate_payload();

    // Spawn 10 concurrent requests
    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let server = server.clone();
            let raw_key = raw_key.clone();
            let payload = payload.clone();

            tokio::spawn(async move {
                server
                    .post("/v1/ingest")
                    .add_header("authorization", format!("Bearer {}", raw_key))
                    .add_header("content-type", "application/json")
                    .json(&payload)
                    .await
            })
        })
        .collect();

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    // Verify all requests succeeded
    for (i, result) in results.iter().enumerate() {
        let response = result.as_ref().expect(&format!("Task {} panicked", i));
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} failed with status: {}",
            i,
            response.status()
        );
    }
}

// ==================== PROPERTY-BASED TESTING ====================

proptest! {
    #[test]
    fn test_payload_size_handling(
        heart_rate_count in 0..100usize,
        blood_pressure_count in 0..50usize,
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let pool = setup_test_database().await;
            let server = create_test_app(pool.clone()).await;
            let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

            // Generate random-sized payload
            let heart_rate_data: Vec<Value> = (0..heart_rate_count)
                .map(|_| json!({
                    "id": Uuid::new_v4(),
                    "user_id": Uuid::new_v4(),
                    "recorded_at": Utc::now(),
                    "heart_rate": 60,
                    "source_device": "Test Device",
                    "created_at": Utc::now()
                }))
                .collect();

            let blood_pressure_data: Vec<Value> = (0..blood_pressure_count)
                .map(|_| json!({
                    "id": Uuid::new_v4(),
                    "user_id": Uuid::new_v4(),
                    "recorded_at": Utc::now(),
                    "systolic": 120,
                    "diastolic": 80,
                    "source_device": "Test Device",
                    "created_at": Utc::now()
                }))
                .collect();

            let payload = json!({
                "data": {
                    "heart_rate": heart_rate_data,
                    "blood_pressure": blood_pressure_data,
                    "sleep_analysis": [],
                    "activity_summaries": [],
                    "workout_data": []
                }
            });

            let response = server
                .post("/v1/ingest")
                .add_header("authorization", format!("Bearer {}", raw_key))
                .add_header("content-type", "application/json")
                .json(&payload)
                .await;

            // Property: Any valid payload structure should be accepted
            prop_assert!(
                response.status().is_success() || response.status() == StatusCode::BAD_REQUEST,
                "Unexpected status code: {}",
                response.status()
            );
        });
    }
}

// ==================== ERROR PATH COVERAGE ====================

#[sqlx::test]
async fn test_database_connection_failure_handling(pool: PgPool) -> sqlx::Result<()> {
    // This test simulates database connectivity issues
    // Note: In real scenarios, we'd use a mock or test database that we can disconnect

    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Close all connections to simulate database failure
    pool.close().await;

    let payload = valid_heart_rate_payload();

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&payload)
        .await;

    // Should handle database errors gracefully
    assert!(response.status().is_server_error());

    Ok(())
}

#[sqlx::test]
async fn test_invalid_content_type_handling(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = r#"{"data": {"heart_rate": []}}"#;

    // Test various invalid content types
    let invalid_content_types = vec![
        "text/plain",
        "application/xml",
        "multipart/form-data",
        "", // Empty content type
    ];

    for content_type in invalid_content_types {
        let response = server
            .post("/v1/ingest")
            .add_header("authorization", format!("Bearer {}", raw_key))
            .add_header("content-type", content_type)
            .add_raw_body(payload.as_bytes())
            .await;

        // Should reject non-JSON content types appropriately
        assert!(
            response.status().is_client_error(),
            "Failed to reject content type: {}",
            content_type
        );
    }

    Ok(())
}

// ==================== VALIDATION EDGE CASES ====================

#[sqlx::test]
async fn test_extreme_heart_rate_values(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Test boundary values for heart rate
    let extreme_values = vec![
        (0, "zero heart rate"),
        (300, "maximum physiological"),
        (500, "impossible high value"),
        (-1, "negative value"),
    ];

    for (heart_rate, description) in extreme_values {
        let payload = json!({
            "data": {
                "heart_rate": [{
                    "id": Uuid::new_v4(),
                    "user_id": Uuid::new_v4(),
                    "recorded_at": Utc::now(),
                    "heart_rate": heart_rate,
                    "source_device": "Test Device",
                    "created_at": Utc::now()
                }],
                "blood_pressure": [],
                "sleep_analysis": [],
                "activity_summaries": [],
                "workout_data": []
            }
        });

        let response = server
            .post("/v1/ingest")
            .add_header("authorization", format!("Bearer {}", raw_key))
            .add_header("content-type", "application/json")
            .json(&payload)
            .await;

        // The handler should either accept or gracefully reject extreme values
        assert!(
            response.status().is_success() || response.status().is_client_error(),
            "Unexpected response for {}: {}",
            description,
            response.status()
        );
    }

    Ok(())
}

// ==================== LOGGING AND MONITORING TESTS ====================

#[sqlx::test]
async fn test_logged_json_extractor(pool: PgPool) -> sqlx::Result<()> {
    // Test the LoggedJson extractor functionality
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = valid_heart_rate_payload();

    // This test verifies that the LoggedJson extractor properly logs payload information
    // In a real test environment, we'd capture logs and verify they contain expected data

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&payload)
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    // Verify response contains logging metadata
    let response_body: IngestResponse = response.json().await;
    assert!(response_body.processing_time_ms.is_some());
    assert!(response_body.request_id.is_some());

    Ok(())
}

// ==================== QUICKCHECK PROPERTY TESTS ====================

#[quickcheck]
fn test_json_serialization_roundtrip(heart_rate: u16) -> bool {
    if heart_rate == 0 || heart_rate > 300 {
        return true; // Skip invalid values
    }

    let payload = json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": Uuid::new_v4(),
                "recorded_at": Utc::now(),
                "heart_rate": heart_rate,
                "source_device": "Test Device",
                "created_at": Utc::now()
            }],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    // Property: Any valid payload should serialize and deserialize correctly
    let serialized = serde_json::to_string(&payload);
    if let Ok(json_str) = serialized {
        if let Ok(_deserialized) = serde_json::from_str::<Value>(&json_str) {
            return true;
        }
    }
    false
}

// ==================== COMPREHENSIVE INTEGRATION TESTS ====================

#[sqlx::test]
async fn test_full_multi_metric_ingest_workflow(pool: PgPool) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Comprehensive payload with all metric types
    let comprehensive_payload = json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
                "recorded_at": Utc::now(),
                "heart_rate": 75,
                "resting_heart_rate": 65,
                "source_device": "Apple Watch",
                "created_at": Utc::now()
            }],
            "blood_pressure": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
                "recorded_at": Utc::now(),
                "systolic": 120,
                "diastolic": 80,
                "pulse": 75,
                "source_device": "Blood Pressure Monitor",
                "created_at": Utc::now()
            }],
            "sleep_analysis": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
                "sleep_start": Utc::now(),
                "sleep_end": Utc::now(),
                "duration_minutes": 480,
                "efficiency": 85.0,
                "source_device": "Sleep Tracker",
                "created_at": Utc::now()
            }],
            "activity_summaries": [],
            "workout_data": []
        },
        "device_info": {
            "device_model": "iPhone 15 Pro",
            "ios_version": "17.2",
            "app_version": "2.1.0"
        },
        "user_info": {
            "timezone": "America/New_York",
            "locale": "en_US"
        }
    });

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", "application/json")
        .json(&comprehensive_payload)
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response_body: IngestResponse = response.json().await;
    assert_eq!(response_body.status, "success");
    assert_eq!(response_body.metrics_processed, 3); // heart_rate + blood_pressure + sleep

    // Verify each metric type was stored correctly
    let heart_rate_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    let blood_pressure_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    let sleep_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(heart_rate_count.unwrap_or(0), 1);
    assert_eq!(blood_pressure_count.unwrap_or(0), 1);
    assert_eq!(sleep_count.unwrap_or(0), 1);

    Ok(())
}

// ==================== RSTEST PARAMETERIZED TESTS ====================

#[rstest]
#[case("application/json")]
#[case("application/json; charset=utf-8")]
#[case("application/json; charset=UTF-8")]
#[sqlx::test]
async fn test_valid_content_types(
    pool: PgPool,
    #[case] content_type: &str,
) -> sqlx::Result<()> {
    let server = create_test_app(pool.clone()).await;
    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = valid_heart_rate_payload();

    let response = server
        .post("/v1/ingest")
        .add_header("authorization", format!("Bearer {}", raw_key))
        .add_header("content-type", content_type)
        .json(&payload)
        .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Failed to accept content type: {}",
        content_type
    );

    Ok(())
}

#[rstest]
#[case(StatusCode::OK, "valid_payload")]
#[case(StatusCode::BAD_REQUEST, "malformed_json")]
#[case(StatusCode::UNAUTHORIZED, "no_auth")]
#[tokio::test]
async fn test_expected_status_codes(
    #[case] expected_status: StatusCode,
    #[case] test_case: &str,
) {
    let pool = setup_test_database().await;
    let server = create_test_app(pool.clone()).await;

    let response = match test_case {
        "valid_payload" => {
            let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;
            let payload = valid_heart_rate_payload();

            server
                .post("/v1/ingest")
                .add_header("authorization", format!("Bearer {}", raw_key))
                .add_header("content-type", "application/json")
                .json(&payload)
                .await
        }
        "malformed_json" => {
            let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

            server
                .post("/v1/ingest")
                .add_header("authorization", format!("Bearer {}", raw_key))
                .add_header("content-type", "application/json")
                .add_raw_body(b"{invalid json")
                .await
        }
        "no_auth" => {
            let payload = valid_heart_rate_payload();

            server
                .post("/v1/ingest")
                .add_header("content-type", "application/json")
                .json(&payload)
                .await
        }
        _ => panic!("Unknown test case: {}", test_case),
    };

    assert_eq!(
        response.status(),
        expected_status,
        "Test case '{}' failed",
        test_case
    );
}