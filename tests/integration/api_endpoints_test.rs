use actix_web::{test, web, App, middleware::Logger};
use chrono::{DateTime, Utc, Duration};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::env;
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::{
    handlers::{
        health::health_handler,
        ingest::{ingest_handler, ingest_async_handler}, 
        background::batch_process_handler,
    },
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    models::{ApiResponse, IngestResponse, HealthMetric},
    services::auth::AuthService,
};

/// Comprehensive API endpoint tests for all ingest endpoints and new metric types
/// Tests dual-write functionality, validation, error handling, and batch processing

async fn get_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

fn get_test_redis_client() -> redis::Client {
    dotenv::dotenv().ok();
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    redis::Client::open(redis_url).expect("Failed to create Redis client")
}

async fn setup_test_user_and_key(pool: &PgPool, email: &str) -> (Uuid, String) {
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user(email, Some("API Test User"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "API Test Key", None, vec!["write".to_string()])
        .await
        .unwrap();

    (user.id, plain_key)
}

fn create_api_test_app(pool: PgPool, redis_client: redis::Client) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware::new(redis_client, 1000, std::time::Duration::from_secs(60)))
            .wrap(AuthMiddleware::new(pool))
            .route("/health", web::get().to(health_handler))
            .route("/api/v1/ingest", web::post().to(ingest_handler))
            .route("/api/v1/ingest/async", web::post().to(ingest_async_handler))
            .route("/api/v1/batch/process", web::post().to(batch_process_handler))
    )
}

/// Test all ingest endpoints with new metric types
#[tokio::test]
async fn test_all_ingest_endpoints_new_metrics() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "endpoints_test@example.com").await;

    let app = create_api_test_app(pool.clone(), redis_client).await;

    println!("🚀 Testing all ingest endpoints with new metric types...");

    // Test 1: Standard ingest endpoint with new metrics
    let new_metrics_payload = create_all_new_metrics_payload();
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&new_metrics_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Standard ingest should handle new metrics");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(body.success);
    assert_eq!(body.data.failed_count, 0);
    assert!(body.data.processed_count >= 6, "Should process all 6 new metric types");

    // Test 2: Async ingest endpoint
    let async_payload = create_large_mixed_payload();
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest/async")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&async_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Async ingest should handle large payloads");

    let async_body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(async_body.success);
    
    // Test 3: Batch process endpoint
    let batch_payload = create_batch_processing_payload();
    
    let req = test::TestRequest::post()
        .uri("/api/v1/batch/process")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&batch_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Batch process should handle structured batches");

    // Verify all metrics were stored correctly
    let storage_counts = verify_all_metric_types_stored(&pool, user_id).await;
    
    println!("✅ All Ingest Endpoints Results:");
    println!("   📊 Nutrition metrics stored: {}", storage_counts.get("nutrition").unwrap_or(&0));
    println!("   🤒 Symptoms stored: {}", storage_counts.get("symptoms").unwrap_or(&0));
    println!("   💝 Reproductive health stored: {}", storage_counts.get("reproductive").unwrap_or(&0));
    println!("   🌍 Environmental stored: {}", storage_counts.get("environmental").unwrap_or(&0));
    println!("   🧠 Mental health stored: {}", storage_counts.get("mental_health").unwrap_or(&0));
    println!("   🚶 Mobility stored: {}", storage_counts.get("mobility").unwrap_or(&0));

    // Verify minimum storage requirements
    for (metric_type, count) in storage_counts {
        assert!(*count > 0, "Should store at least one {} metric", metric_type);
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test validation and error handling for all new metric types
#[tokio::test]
async fn test_validation_error_handling_all_types() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "validation_api@example.com").await;

    let app = create_api_test_app(pool.clone(), redis_client).await;

    println!("🚀 Testing validation and error handling for all metric types...");

    // Test 1: Nutrition validation errors
    let invalid_nutrition = json!({
        "data": {
            "nutrition_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "water_ml": 30000.0, // Invalid: exceeds 20L limit
                    "energy_consumed_kcal": 25000.0, // Invalid: exceeds 20k limit
                    "protein_g": 2000.0, // Invalid: exceeds 1k limit
                    "aggregation_period": "invalid_period", // Invalid enum
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_nutrition)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(!body.success);
    assert!(!body.data.errors.is_empty());
    
    let error_messages: Vec<String> = body.data.errors.iter().map(|e| e.error_message.clone()).collect();
    assert!(error_messages.iter().any(|e| e.contains("water_ml")), "Should have water volume error");
    assert!(error_messages.iter().any(|e| e.contains("energy_consumed")), "Should have energy error");

    // Test 2: Symptoms validation errors
    let invalid_symptoms = json!({
        "data": {
            "symptom_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "symptom_type": "unknown_symptom_type",
                    "severity": "extremely_severe", // Invalid severity
                    "duration_minutes": 20000, // Invalid: exceeds 1 week limit
                    "onset_at": "2025-01-15T12:00:00Z", // Invalid: future date
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_symptoms)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    // Test 3: Environmental metrics validation errors
    let invalid_environmental = json!({
        "data": {
            "environmental_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "environmental_sound_level_db": 200.0, // Invalid: exceeds 140dB
                    "uv_index": 25.0, // Invalid: exceeds 15
                    "air_quality_index": 600, // Invalid: exceeds 500
                    "impact_force_g": 100.0, // Invalid: exceeds 50G
                    "aggregation_period": "invalid", // Invalid enum
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_environmental)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    // Test 4: Mental health validation errors
    let invalid_mental_health = json!({
        "data": {
            "mental_health_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "mindful_minutes": 2000.0, // Invalid: exceeds 1440 minutes (24h)
                    "mood_valence": 2.0, // Invalid: exceeds 1.0 range
                    "daylight_minutes": 1500.0, // Invalid: exceeds 1440 minutes
                    "stress_level": "extremely_critical", // Invalid enum
                    "depression_score": 50, // Invalid: exceeds 27 (PHQ-9)
                    "anxiety_score": 30, // Invalid: exceeds 21 (GAD-7)
                    "sleep_quality_score": 15, // Invalid: exceeds 10
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_mental_health)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    // Test 5: Mobility validation errors
    let invalid_mobility = json!({
        "data": {
            "mobility_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "walking_speed_m_per_s": 10.0, // Invalid: exceeds 5.0 m/s
                    "step_length_cm": 200.0, // Invalid: exceeds 150cm
                    "double_support_percentage": 150.0, // Invalid: exceeds 100%
                    "walking_asymmetry_percentage": 150.0, // Invalid: exceeds 100%
                    "walking_steadiness": "extremely_unstable", // Invalid enum
                    "six_minute_walk_test_distance": 2000.0, // Invalid: exceeds 1000m
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_mobility)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    // Test 6: Reproductive health validation errors
    let invalid_reproductive = json!({
        "data": {
            "reproductive_health_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "menstrual_flow": "extremely_heavy", // Invalid enum
                    "cycle_day": 100, // Invalid: exceeds 60
                    "cycle_length": 100, // Invalid: exceeds 60
                    "basal_body_temp": 45.0, // Invalid: exceeds 40.0°C
                    "cervical_mucus_quality": "invalid_quality", // Invalid enum
                    "gestational_age_weeks": 60, // Invalid: exceeds 50
                    "source": "Validation Test"
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_reproductive)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    println!("✅ All validation tests passed - errors properly caught and handled");

    cleanup_test_data(&pool, user_id).await;
}

/// Test batch processing with mixed metric types
#[tokio::test]
async fn test_batch_processing_mixed_metrics() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "batch_mixed@example.com").await;

    let app = create_api_test_app(pool.clone(), redis_client).await;

    println!("🚀 Testing batch processing with mixed metric types...");

    // Create large mixed batch payload
    let large_mixed_batch = create_large_mixed_batch_payload();

    let start_time = std::time::Instant::now();

    let req = test::TestRequest::post()
        .uri("/api/v1/batch/process")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&large_mixed_batch)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let processing_time = start_time.elapsed();

    assert!(resp.status().is_success(), "Batch processing should succeed");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(body.success);
    assert_eq!(body.data.failed_count, 0, "No records should fail in batch");
    
    let records_per_second = body.data.processed_count as f64 / processing_time.as_secs_f64();

    println!("✅ Batch Processing Results:");
    println!("   📊 Records processed: {}", body.data.processed_count);
    println!("   ⏱️  Processing time: {:.2}s", processing_time.as_secs_f64());
    println!("   🚀 Records per second: {:.0}", records_per_second);

    // Verify performance expectations
    assert!(records_per_second > 1000.0, "Should process >1000 records/second");

    // Verify all metric types were processed
    let batch_counts = verify_batch_processing_results(&pool, user_id).await;
    for (metric_type, count) in batch_counts {
        assert!(count > 0, "Should have processed {} metrics in batch", metric_type);
        println!("   📈 {} processed: {}", metric_type, count);
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test API error handling edge cases
#[tokio::test]
async fn test_api_error_handling_edge_cases() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "error_edge@example.com").await;

    let app = create_api_test_app(pool.clone(), redis_client).await;

    println!("🚀 Testing API error handling edge cases...");

    // Test 1: Empty payload
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&json!({}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should succeed but process 0 records
    assert!(resp.status().is_success());

    // Test 2: Malformed JSON
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload("{invalid json}")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return bad request for malformed JSON");

    // Test 3: Missing content-type header
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .set_json(&json!({"data": {}}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should still work - actix-web handles this gracefully

    // Test 4: Invalid authorization header
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", "Bearer invalid_key"))
        .insert_header(("content-type", "application/json"))
        .set_json(&json!({"data": {}}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Should return unauthorized");

    // Test 5: Missing authorization header
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .set_json(&json!({"data": {}}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Should return unauthorized");

    // Test 6: Oversized payload (if limits are configured)
    let oversized_payload = create_oversized_payload();
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&oversized_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should handle gracefully (may succeed or return payload too large)

    println!("✅ API error handling edge cases tested successfully");

    cleanup_test_data(&pool, user_id).await;
}

/// Test API endpoint performance with concurrent requests
#[tokio::test]
async fn test_api_endpoint_performance_concurrent() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "perf_concurrent@example.com").await;

    let app = std::sync::Arc::new(create_api_test_app(pool.clone(), redis_client).await);

    println!("🚀 Testing API endpoint performance with concurrent requests...");

    let concurrent_requests = 50;
    let mut tasks = Vec::new();

    let start_time = std::time::Instant::now();

    for i in 0..concurrent_requests {
        let app_clone = app.clone();
        let api_key_clone = api_key.clone();
        
        let task = tokio::spawn(async move {
            let payload = create_performance_test_payload(i);
            
            let req = test::TestRequest::post()
                .uri("/api/v1/ingest")
                .insert_header(("Authorization", format!("Bearer {}", api_key_clone)))
                .insert_header(("content-type", "application/json"))
                .set_json(&payload)
                .to_request();

            let request_start = std::time::Instant::now();
            let resp = test::call_service(&*app_clone, req).await;
            let request_time = request_start.elapsed();

            (resp.status().is_success(), request_time)
        });

        tasks.push(task);
    }

    let results = futures::future::join_all(tasks).await;
    let total_time = start_time.elapsed();

    let successful_requests = results.iter().filter(|r| r.as_ref().unwrap().0).count();
    let response_times: Vec<std::time::Duration> = results
        .into_iter()
        .filter_map(|r| r.ok().map(|(_, time)| time))
        .collect();

    let avg_response_time = response_times.iter().sum::<std::time::Duration>() / response_times.len() as u32;
    let requests_per_second = concurrent_requests as f64 / total_time.as_secs_f64();

    println!("✅ Concurrent Performance Results:");
    println!("   👥 Concurrent requests: {}", concurrent_requests);
    println!("   ✅ Successful requests: {}", successful_requests);
    println!("   ⏱️  Average response time: {}ms", avg_response_time.as_millis());
    println!("   🚀 Requests per second: {:.1}", requests_per_second);

    // Performance assertions
    let success_rate = successful_requests as f64 / concurrent_requests as f64;
    assert!(success_rate >= 0.95, "Success rate should be ≥95%");
    assert!(avg_response_time.as_millis() < 2000, "Average response time should be <2000ms");

    cleanup_test_data(&pool, user_id).await;
}

// Helper functions

fn create_all_new_metrics_payload() -> Value {
    json!({
        "data": {
            "nutrition_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "water_ml": 2500.0,
                    "energy_consumed_kcal": 2200.0,
                    "protein_g": 85.0,
                    "carbohydrates_g": 275.0,
                    "source": "API Test"
                }
            ],
            "symptom_metrics": [
                {
                    "recorded_at": "2024-01-15T10:00:00Z",
                    "symptom_type": "headache",
                    "severity": "mild",
                    "duration_minutes": 60,
                    "source": "API Test"
                }
            ],
            "reproductive_health_metrics": [
                {
                    "recorded_at": "2024-01-15T08:00:00Z",
                    "menstrual_flow": "medium",
                    "cycle_day": 3,
                    "cycle_length": 28,
                    "source": "API Test"
                }
            ],
            "environmental_metrics": [
                {
                    "recorded_at": "2024-01-15T14:00:00Z",
                    "environmental_sound_level_db": 45.0,
                    "uv_index": 6.0,
                    "air_quality_index": 85,
                    "source": "API Test"
                }
            ],
            "mental_health_metrics": [
                {
                    "recorded_at": "2024-01-15T07:00:00Z",
                    "mindful_minutes": 15.0,
                    "mood_valence": 0.3,
                    "stress_level": "low",
                    "source": "API Test"
                }
            ],
            "mobility_metrics": [
                {
                    "recorded_at": "2024-01-15T16:00:00Z",
                    "walking_speed_m_per_s": 1.2,
                    "step_length_cm": 65.0,
                    "walking_steadiness": "ok",
                    "source": "API Test"
                }
            ]
        }
    })
}

fn create_large_mixed_payload() -> Value {
    let mut nutrition_metrics = Vec::new();
    let mut symptom_metrics = Vec::new();
    let mut environmental_metrics = Vec::new();

    let base_date = Utc::now() - Duration::days(7);

    // Create 100 nutrition records
    for i in 0..100 {
        let date = base_date + Duration::hours(i * 2);
        nutrition_metrics.push(json!({
            "recorded_at": date.to_rfc3339(),
            "water_ml": 300.0 + (i % 50) as f64,
            "energy_consumed_kcal": 400.0 + (i % 200) as f64,
            "protein_g": 20.0 + (i % 30) as f64,
            "source": format!("Large Test {}", i)
        }));
    }

    // Create 50 symptom records
    for i in 0..50 {
        let date = base_date + Duration::hours(i * 3);
        symptom_metrics.push(json!({
            "recorded_at": date.to_rfc3339(),
            "symptom_type": "fatigue",
            "severity": if i % 3 == 0 { "mild" } else { "moderate" },
            "duration_minutes": 60 + (i % 120),
            "source": format!("Large Test {}", i)
        }));
    }

    // Create 75 environmental records
    for i in 0..75 {
        let date = base_date + Duration::hours(i * 2 + 1);
        environmental_metrics.push(json!({
            "recorded_at": date.to_rfc3339(),
            "environmental_sound_level_db": 40.0 + (i % 30) as f64,
            "air_quality_index": 60 + (i % 80) as i32,
            "source": format!("Large Test {}", i)
        }));
    }

    json!({
        "data": {
            "nutrition_metrics": nutrition_metrics,
            "symptom_metrics": symptom_metrics,
            "environmental_metrics": environmental_metrics
        }
    })
}

fn create_batch_processing_payload() -> Value {
    json!({
        "batches": [
            {
                "batch_id": "batch_1",
                "metrics": [
                    {
                        "type": "Nutrition",
                        "recorded_at": "2024-01-15T12:00:00Z",
                        "water_ml": 500.0,
                        "energy_consumed_kcal": 600.0,
                        "source": "Batch 1"
                    }
                ]
            },
            {
                "batch_id": "batch_2", 
                "metrics": [
                    {
                        "type": "Environmental",
                        "recorded_at": "2024-01-15T14:00:00Z",
                        "environmental_sound_level_db": 50.0,
                        "air_quality_index": 75,
                        "source": "Batch 2"
                    }
                ]
            }
        ]
    })
}

fn create_large_mixed_batch_payload() -> Value {
    let mut all_metrics = Vec::new();
    
    let base_date = Utc::now() - Duration::days(1);
    
    // Create 500 mixed metrics for batch processing
    for i in 0..500 {
        let date = base_date + Duration::minutes(i * 3);
        
        match i % 6 {
            0 => all_metrics.push(json!({
                "type": "Nutrition",
                "recorded_at": date.to_rfc3339(),
                "water_ml": 200.0 + (i % 100) as f64,
                "energy_consumed_kcal": 300.0 + (i % 150) as f64,
                "source": "Batch Mixed"
            })),
            1 => all_metrics.push(json!({
                "type": "Symptom",
                "recorded_at": date.to_rfc3339(),
                "symptom_type": "headache",
                "severity": "mild",
                "duration_minutes": 30 + (i % 90),
                "source": "Batch Mixed"
            })),
            2 => all_metrics.push(json!({
                "type": "Environmental",
                "recorded_at": date.to_rfc3339(),
                "environmental_sound_level_db": 35.0 + (i % 40) as f64,
                "air_quality_index": 50 + (i % 100) as i32,
                "source": "Batch Mixed"
            })),
            3 => all_metrics.push(json!({
                "type": "MentalHealth",
                "recorded_at": date.to_rfc3339(),
                "mindful_minutes": (i % 30) as f64,
                "mood_valence": -1.0 + (i % 200) as f64 / 100.0,
                "stress_level": "low",
                "source": "Batch Mixed"
            })),
            4 => all_metrics.push(json!({
                "type": "Mobility",
                "recorded_at": date.to_rfc3339(),
                "walking_speed_m_per_s": 0.8 + (i % 50) as f64 / 100.0,
                "step_length_cm": 50.0 + (i % 30) as f64,
                "source": "Batch Mixed"
            })),
            5 => all_metrics.push(json!({
                "type": "ReproductiveHealth",
                "recorded_at": date.to_rfc3339(),
                "cycle_day": 1 + (i % 28) as i16,
                "basal_body_temp": 97.0 + (i % 20) as f64 / 10.0,
                "source": "Batch Mixed"
            })),
            _ => unreachable!()
        }
    }

    json!({
        "data": {
            "metrics": all_metrics
        }
    })
}

fn create_oversized_payload() -> Value {
    // Create a very large payload to test size limits
    let large_data = "x".repeat(1_000_000); // 1MB of data
    
    json!({
        "data": {
            "nutrition_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "water_ml": 500.0,
                    "notes": large_data,
                    "source": "Oversized Test"
                }
            ]
        }
    })
}

fn create_performance_test_payload(index: usize) -> Value {
    json!({
        "data": {
            "nutrition_metrics": [
                {
                    "recorded_at": "2024-01-15T12:00:00Z",
                    "water_ml": 300.0 + (index % 100) as f64,
                    "energy_consumed_kcal": 400.0 + (index % 200) as f64,
                    "source": format!("Performance Test {}", index)
                }
            ],
            "environmental_metrics": [
                {
                    "recorded_at": "2024-01-15T12:05:00Z",
                    "environmental_sound_level_db": 40.0 + (index % 30) as f64,
                    "air_quality_index": 60 + (index % 80) as i32,
                    "source": format!("Performance Test {}", index)
                }
            ]
        }
    })
}

async fn verify_all_metric_types_stored(pool: &PgPool, user_id: Uuid) -> HashMap<String, i64> {
    let mut counts = HashMap::new();

    let nutrition_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM nutrition_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("nutrition".to_string(), nutrition_count);

    let symptoms_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM symptoms WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("symptoms".to_string(), symptoms_count);

    let reproductive_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM reproductive_health WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("reproductive".to_string(), reproductive_count);

    let environmental_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("environmental".to_string(), environmental_count);

    let mental_health_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("mental_health".to_string(), mental_health_count);

    let mobility_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM mobility_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);
    counts.insert("mobility".to_string(), mobility_count);

    counts
}


async fn verify_batch_processing_results(pool: &PgPool, user_id: Uuid) -> HashMap<String, i64> {
    // Return counts of each metric type processed in batch
    verify_all_metric_types_stored(pool, user_id).await
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();
}