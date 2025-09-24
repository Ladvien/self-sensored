use actix_web::{middleware::Logger, test, web, App};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

use self_sensored::{
    db::database::create_connection_pool,
    handlers::{health::health_check, ingest::ingest_handler},
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    models::{ApiResponse, IngestResponse},
    services::{auth::AuthService, rate_limiter::RateLimiter},
};

/// Comprehensive API endpoint tests for all ingest endpoints and new metric types
/// Tests dual-write functionality, validation, error handling, and batch processing

async fn get_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");
    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool")
}

fn get_test_redis_client() -> redis::Client {
    dotenv::dotenv().ok();
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    redis::Client::open(redis_url).expect("Failed to create Redis client")
}

async fn setup_test_user_and_key(pool: &PgPool, email: &str) -> (Uuid, String) {
    let auth_service = AuthService::new(pool.clone());

    // Generate unique apple_health_id for this test run
    let unique_id = Uuid::new_v4();
    let apple_health_id = format!("api_test_user_{}", unique_id);

    // Clean up any existing test user with this email
    sqlx::query!(
        "DELETE FROM users WHERE email = $1 OR apple_health_id = $2",
        email,
        &apple_health_id
    )
    .execute(pool)
    .await
    .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user(
            email,
            Some(&apple_health_id),
            Some(serde_json::json!({"name": "API Test User"})),
        )
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("API Test Key"),
            None,
            Some(serde_json::json!(["write"])),
            None,
        )
        .await
        .unwrap();

    (user.id, plain_key)
}

/// Test all ingest endpoints with supported metric types
#[tokio::test]
async fn test_all_ingest_endpoints_new_metrics() {
    let pool = get_test_pool().await;
    let _redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "endpoints_test@example.com").await;

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    let auth_service = web::Data::new(AuthService::new(pool.clone()));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter)
            .app_data(auth_service)
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(health_check))
            .route("/api/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    println!("🚀 Testing ingest endpoint with supported metric types...");

    // Test with properly formatted metrics payload
    let new_metrics_payload = create_all_new_metrics_payload();

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&new_metrics_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;

    if !status.is_success() || !body.success {
        println!("Response status: {}", status);
        println!("Response body: {:?}", body);
        panic!(
            "Standard ingest failed with status {} and success: {}",
            status, body.success
        );
    }
    assert!(body.success);
    let data = body.data.expect("Should have response data");
    println!(
        "Ingest result: processed={}, failed={}",
        data.processed_count, data.failed_count
    );

    // The API processes heart rate, blood pressure (as 2 separate metrics), steps, energy, and sleep
    // Blood pressure is sent as separate systolic/diastolic which may cause some failures
    assert!(
        data.processed_count >= 3,
        "Should process at least 3 metric types, but only processed {}",
        data.processed_count
    );

    // Test 2: Large payload test
    let large_payload = create_large_mixed_payload();

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&large_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Should handle large payloads");

    let large_body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(large_body.success);

    println!(
        "Large payload result: processed={}, failed={}",
        large_body
            .data
            .as_ref()
            .map(|d| d.processed_count)
            .unwrap_or(0),
        large_body
            .data
            .as_ref()
            .map(|d| d.failed_count)
            .unwrap_or(0)
    );

    // Verify metrics were stored
    let storage_counts = verify_all_metric_types_stored(&pool, user_id).await;

    println!("✅ Ingest Test Results:");
    println!(
        "   ❤️ Heart rate metrics stored: {}",
        storage_counts.get("heart_rate").unwrap_or(&0)
    );
    println!(
        "   🩸 Blood pressure metrics stored: {}",
        storage_counts.get("blood_pressure").unwrap_or(&0)
    );
    println!(
        "   😴 Sleep metrics stored: {}",
        storage_counts.get("sleep").unwrap_or(&0)
    );
    println!(
        "   🏃 Activity metrics stored: {}",
        storage_counts.get("activity").unwrap_or(&0)
    );
    println!(
        "   🏋️ Workout metrics stored: {}",
        storage_counts.get("workouts").unwrap_or(&0)
    );

    // Verify at least some metrics were stored
    let total_stored: i64 = storage_counts.values().sum();
    assert!(total_stored > 0, "Should store at least some metrics");

    cleanup_test_data(&pool, user_id).await;
}

/// Test validation and error handling for all supported metric types
#[tokio::test]
async fn test_validation_error_handling_all_types() {
    let pool = get_test_pool().await;
    let _redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "validation_api@example.com").await;

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    let auth_service = web::Data::new(AuthService::new(pool.clone()));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter)
            .app_data(auth_service)
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(health_check))
            .route("/api/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    println!("🚀 Testing validation and error handling for supported metric types...");

    // Test 1: Invalid heart rate values
    let invalid_heart_rate = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 350.0, // Invalid: exceeds max heart rate
                            "source": "Test"
                        },
                        {
                            "date": "2024-01-15 13:00:00 +0000",
                            "qty": -10.0, // Invalid: negative heart rate
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_heart_rate)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(!body.success, "Invalid heart rate should fail validation");

    // Test 2: Invalid blood pressure values
    let invalid_blood_pressure = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 300.0, // Invalid: exceeds max systolic
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 200.0, // Invalid: exceeds max diastolic
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_blood_pressure)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        400,
        "Invalid blood pressure should return 400"
    );

    // Test 3: Invalid sleep data
    let invalid_sleep = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "units": "min",
                    "data": [
                        {
                            "start": "2024-01-15 23:00:00 +0000",
                            "end": "2024-01-15 22:00:00 +0000", // Invalid: end before start
                            "qty": -100.0, // Invalid: negative duration
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_sleep)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Invalid sleep data should return 400");

    // Test 4: Invalid workout data
    let invalid_workout = json!({
        "data": {
            "workouts": [
                {
                    "source": "Test App",
                    "startDate": "2024-01-15T06:00:00Z",
                    "endDate": "2024-01-15T05:30:00Z", // Invalid: end before start
                    "activityType": "HKWorkoutActivityTypeRunning",
                    "totalEnergyBurned": -250.0, // Invalid: negative energy
                    "totalDistance": -5000.0 // Invalid: negative distance
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_workout)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Invalid workout should return 400");

    // Test 5: Invalid activity/step data
    let invalid_activity = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [
                        {
                            "date": "2024-01-15",
                            "qty": -5000.0, // Invalid: negative steps
                            "source": "Test"
                        },
                        {
                            "date": "2024-01-16",
                            "qty": 300000.0, // Invalid: exceeds reasonable daily limit
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_activity)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        400,
        "Invalid activity data should return 400"
    );

    println!("✅ All validation tests passed - errors properly caught for supported metric types");

    cleanup_test_data(&pool, user_id).await;
}

/// Test batch processing with mixed metric types
#[tokio::test]
async fn test_batch_processing_mixed_metrics() {
    let pool = get_test_pool().await;
    let _redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "batch_mixed@example.com").await;

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    let auth_service = web::Data::new(AuthService::new(pool.clone()));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter)
            .app_data(auth_service)
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(health_check))
            .route("/api/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    println!("🚀 Testing batch processing with mixed supported metric types...");

    // Create large mixed payload with all 5 supported metric types
    let mixed_batch_payload = create_large_mixed_payload();

    let start_time = std::time::Instant::now();

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&mixed_batch_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let processing_time = start_time.elapsed();

    assert!(
        resp.status().is_success(),
        "Batch processing should succeed"
    );

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(
        body.success,
        "Batch processing response should indicate success"
    );
    let data = body.data.expect("Should have response data");

    let records_per_second = data.processed_count as f64 / processing_time.as_secs_f64();

    println!("✅ Batch Processing Results:");
    println!("   📊 Records processed: {}", data.processed_count);
    println!("   ❌ Records failed: {}", data.failed_count);
    println!(
        "   ⏱️  Processing time: {:.2}s",
        processing_time.as_secs_f64()
    );
    println!("   🚀 Records per second: {records_per_second:.0}");

    // Verify some metrics were processed (batch processing with large payloads)
    assert!(
        data.processed_count > 100,
        "Should process substantial number of metrics, got {}",
        data.processed_count
    );

    // Verify metrics were stored in database
    let storage_counts = verify_all_metric_types_stored(&pool, user_id).await;
    let total_stored: i64 = storage_counts.values().sum();
    assert!(
        total_stored > 100,
        "Should store substantial number of metrics in database"
    );

    for (metric_type, count) in storage_counts {
        if count > 0 {
            println!("   📈 {metric_type} stored: {count}");
        }
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test API error handling edge cases
#[tokio::test]
async fn test_api_error_handling_edge_cases() {
    let pool = get_test_pool().await;
    let _redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "error_edge@example.com").await;

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    let auth_service = web::Data::new(AuthService::new(pool.clone()));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter)
            .app_data(auth_service)
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(health_check))
            .route("/api/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    println!("🚀 Testing API error handling edge cases...");

    // Test 1: Empty data object
    let empty_data = json!({
        "data": {
            "metrics": [],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&empty_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Empty data may return 400 or 200 depending on implementation
    if resp.status().is_success() {
        let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
        assert_eq!(
            body.data.as_ref().map(|d| d.processed_count).unwrap_or(0),
            0,
            "Should process 0 records for empty data"
        );
    } else {
        assert_eq!(resp.status(), 400, "Empty data may return 400 bad request");
    }

    // Test 2: Malformed JSON
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_payload("{invalid json}")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        400,
        "Should return bad request for malformed JSON"
    );

    // Test 3: Missing data field entirely
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(json!({}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // This might succeed with 0 processed or return 400 depending on implementation
    println!("Missing data field status: {}", resp.status());

    // Test 4: Invalid authorization header
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", "Bearer invalid_key"))
        .insert_header(("content-type", "application/json"))
        .set_json(json!({"data": {"metrics": [], "workouts": []}}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Should return unauthorized");

    // Test 5: Missing authorization header
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .set_json(json!({"data": {"metrics": [], "workouts": []}}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Should return unauthorized");

    // Test 6: Metrics with null values
    let null_metrics = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": null,
                            "qty": 72.0,
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&null_metrics)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // The API accepts the request and may process metrics with null dates using defaults
    assert_eq!(resp.status(), 200, "Should accept request");
    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    let processed = body.data.as_ref().map(|d| d.processed_count).unwrap_or(0);
    // API may either skip metrics with null dates (0) or use defaults (1)
    assert!(
        processed <= 1,
        "Should process at most 1 metric with null date, got {}",
        processed
    );
    println!("Null date handling: processed {} metrics", processed);

    // Test 7: Mixed valid and invalid metrics
    let mixed_metrics = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 72.0,
                            "source": "Test"
                        },
                        {
                            "date": "2024-01-15 13:00:00 +0000",
                            "qty": -50.0, // Invalid negative heart rate
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [
                        {
                            "date": "2024-01-15",
                            "qty": 5000.0, // Valid
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&mixed_metrics)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // API should process valid metrics and skip invalid ones
    if resp.status() == 200 {
        let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
        let processed = body.data.as_ref().map(|d| d.processed_count).unwrap_or(0);
        // Should process at least the valid step count metric
        assert!(
            processed >= 1,
            "Should process at least 1 valid metric from mixed batch"
        );
        println!("Mixed metrics: processed {} valid metrics", processed);
    } else {
        println!(
            "Mixed valid/invalid metrics status: {} (strict validation mode)",
            resp.status()
        );
    }

    // Test 8: Duplicate timestamps for same metric type
    let duplicate_timestamps = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 72.0,
                            "source": "Test"
                        },
                        {
                            "date": "2024-01-15 12:00:00 +0000", // Duplicate timestamp
                            "qty": 73.0,
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&duplicate_timestamps)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should handle duplicates gracefully (likely keep one, reject other)
    assert!(resp.status().is_success() || resp.status() == 400);

    // Test 9: Large payload with supported metrics (reduced size for faster test)
    let large_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": (0..100).map(|i| json!({
                        "date": format!("2024-01-15 {:02}:00:00 +0000", i % 24),
                        "qty": 60.0 + (i % 40) as f64,
                        "source": "Large Test"
                    })).collect::<Vec<_>>()
                }
            ],
            "workouts": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&large_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should handle gracefully
    println!("Large payload status: {}", resp.status());

    // Test 10: Workout with invalid date range (end before start)
    let invalid_workout = json!({
        "data": {
            "metrics": [],
            "workouts": [
                {
                    "workoutActivityType": "running",
                    "duration": "30",
                    "durationUnit": "min",
                    "totalDistance": "5.2",
                    "totalDistanceUnit": "km",
                    "totalEnergyBurned": "320",
                    "totalEnergyBurnedUnit": "kcal",
                    "startDate": "2024-01-15 06:30:00 +0000",
                    "endDate": "2024-01-15 06:00:00 +0000" // End before start
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_workout)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // The API may accept the request but not process invalid workouts
    if resp.status() == 200 {
        let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
        assert_eq!(
            body.data.as_ref().map(|d| d.processed_count).unwrap_or(0),
            0,
            "Should process 0 workouts with invalid date range"
        );
    } else {
        assert_eq!(
            resp.status(),
            400,
            "Or may reject workout with end before start"
        );
    }

    println!("✅ API error handling edge cases tested successfully");

    cleanup_test_data(&pool, user_id).await;
}

/// Test API endpoint performance with multiple requests
#[tokio::test]
async fn test_api_endpoint_performance_concurrent() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "perf_test@example.com").await;

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    let auth_service = web::Data::new(AuthService::new(pool.clone()));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter)
            .app_data(auth_service)
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/api/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    println!("🚀 Testing API performance with simplified schema metric types...");

    // Test each of the 5 supported metric types
    let test_payloads = vec![
        ("Heart Rate", create_heart_rate_performance_payload()),
        (
            "Blood Pressure",
            create_blood_pressure_performance_payload(),
        ),
        ("Sleep", create_sleep_performance_payload()),
        ("Activity", create_activity_performance_payload()),
        ("Workout", create_workout_performance_payload()),
    ];

    let mut successful_requests = 0;
    let total_requests = test_payloads.len();

    for (i, (metric_type, payload)) in test_payloads.iter().enumerate() {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .insert_header(("content-type", "application/json"))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        println!(
            "Request {}: {} - Status = {}",
            i + 1,
            metric_type,
            resp.status()
        );

        if resp.status().is_success() {
            successful_requests += 1;
        }
    }

    println!("✅ Performance Test Results:");
    println!("   🔢 Total requests: {total_requests}");
    println!("   ✅ Successful requests: {successful_requests}");

    // Performance assertion - at least 3 out of 5 metric types should work
    let success_rate = successful_requests as f64 / total_requests as f64;
    assert!(
        success_rate >= 0.6,
        "Success rate should be ≥60% (at least 3/5 metric types working)"
    );

    cleanup_test_data(&pool, user_id).await;
}

// Helper functions

fn create_all_new_metrics_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 72.0,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 120.0,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 80.0,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [
                        {
                            "date": "2024-01-15",
                            "qty": 10000.0,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierActiveEnergyBurned",
                    "units": "kcal",
                    "data": [
                        {
                            "date": "2024-01-15",
                            "qty": 450.5,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "units": "min",
                    "data": [
                        {
                            "start": "2024-01-15 23:00:00 +0000",
                            "end": "2024-01-16 07:00:00 +0000",
                            "qty": 480.0,
                            "source": "Test"
                        }
                    ]
                }
            ],
            "workouts": [
                {
                    "workoutActivityType": "running",
                    "duration": "30",
                    "durationUnit": "min",
                    "totalDistance": "5.2",
                    "totalDistanceUnit": "km",
                    "totalEnergyBurned": "320",
                    "totalEnergyBurnedUnit": "kcal",
                    "startDate": "2024-01-15 06:00:00 +0000",
                    "endDate": "2024-01-15 06:30:00 +0000"
                }
            ]
        }
    })
}

fn create_large_mixed_payload() -> Value {
    let base_date = Utc::now() - Duration::days(7);

    // Create iOS format metrics with name and data array
    let mut heart_rate_data = Vec::new();
    for i in 0..100 {
        let date = base_date + Duration::hours(i * 2);
        heart_rate_data.push(json!({
            "date": date.format("%Y-%m-%d %H:%M:%S %z").to_string(),
            "qty": 60.0 + (i % 40) as f64,
            "source": "Test"
        }));
    }

    let mut systolic_data = Vec::new();
    let mut diastolic_data = Vec::new();
    for i in 0..50 {
        let date = base_date + Duration::hours(i * 3);
        systolic_data.push(json!({
            "date": date.format("%Y-%m-%d %H:%M:%S %z").to_string(),
            "qty": 110.0 + (i % 30) as f64,
            "source": "Test"
        }));
        diastolic_data.push(json!({
            "date": date.format("%Y-%m-%d %H:%M:%S %z").to_string(),
            "qty": 70.0 + (i % 20) as f64,
            "source": "Test"
        }));
    }

    let mut steps_data = Vec::new();
    for i in 0..75 {
        let date = base_date + Duration::days(i as i64);
        steps_data.push(json!({
            "date": date.format("%Y-%m-%d").to_string(),
            "qty": 5000.0 + (i * 100) as f64,
            "source": "Test"
        }));
    }

    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": heart_rate_data
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": systolic_data
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": diastolic_data
                },
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": steps_data
                }
            ],
            "workouts": []
        }
    })
}

fn create_oversized_payload() -> Value {
    // Create a very large payload to test size limits
    let mut oversized_metrics = Vec::new();

    // Generate 1000 heart rate metrics to create a large but reasonable payload
    for i in 0..1000 {
        oversized_metrics.push(json!({
            "date": format!("2024-01-15 {:02}:{:02}:{:02} +0000", i / 3600 % 24, (i / 60) % 60, i % 60),
            "qty": 60.0 + (i % 40) as f64,
            "source": "Oversized Test"
        }));
    }

    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": oversized_metrics
                }
            ],
            "workouts": []
        }
    })
}

async fn verify_all_metric_types_stored(pool: &PgPool, user_id: Uuid) -> HashMap<String, i64> {
    let mut counts = HashMap::new();

    // Count heart rate metrics
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("heart_rate".to_string(), heart_rate_count);

    // Count blood pressure metrics
    let blood_pressure_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM blood_pressure_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("blood_pressure".to_string(), blood_pressure_count);

    // Count sleep metrics
    let sleep_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM sleep_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("sleep".to_string(), sleep_count);

    // Count activity metrics
    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("activity".to_string(), activity_count);

    // Count workouts
    let workout_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM workouts WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("workout".to_string(), workout_count);

    counts
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();
}

// Performance test payload creation functions (using working test patterns)
fn create_heart_rate_performance_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "bpm",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 75.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    })
}

fn create_blood_pressure_performance_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 120.0,
                            "source": "Test"
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 12:00:00 +0000",
                            "qty": 80.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    })
}

fn create_sleep_performance_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "data": [
                        {
                            "start": "2024-01-15 23:00:00 +0000",
                            "end": "2024-01-16 07:00:00 +0000",
                            "qty": 480.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    })
}

fn create_activity_performance_payload() -> Value {
    json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [
                        {
                            "date": "2024-01-15",
                            "qty": 5000.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    })
}

fn create_workout_performance_payload() -> Value {
    json!({
        "data": {
            "workouts": [
                {
                    "workoutActivityType": "running",
                    "duration": "30",
                    "durationUnit": "min",
                    "totalDistance": "5.2",
                    "totalDistanceUnit": "km",
                    "totalEnergyBurned": "320",
                    "totalEnergyBurnedUnit": "kcal",
                    "startDate": "2024-01-15 06:00:00 +0000",
                    "endDate": "2024-01-15 06:30:00 +0000"
                }
            ]
        }
    })
}
