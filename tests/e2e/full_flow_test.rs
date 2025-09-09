use actix_web::{test, web, App, middleware::Logger};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use self_sensored::{
    handlers::{health::health_handler, ingest::ingest_handler},
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    models::{ApiResponse, IngestResponse},
    services::auth::AuthService,
};

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
        .create_user(email, Some("E2E Test User"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "E2E Test Key", None, vec!["write".to_string()])
        .await
        .unwrap();

    (user.id, plain_key)
}

fn create_full_app(pool: PgPool, redis_client: redis::Client) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(Logger::default())
            .wrap(RateLimitMiddleware::new(redis_client, 100, std::time::Duration::from_secs(60)))
            .wrap(AuthMiddleware::new(pool))
            .route("/health", web::get().to(health_handler))
            .route("/api/v1/ingest", web::post().to(ingest_handler))
    )
}

#[tokio::test]
async fn test_complete_auto_export_workflow() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "auto_export_e2e@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Step 1: Check health endpoint
    let health_req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let health_resp = test::call_service(&app, health_req).await;
    assert!(health_resp.status().is_success());

    // Step 2: Send real Auto Export payload
    let auto_export_payload = json!({
        "data": [
            {
                "type": "HeartRate",
                "unit": "count/min",
                "value": 75.0,
                "date": "2024-01-15T10:30:00Z",
                "source": "Apple Watch Series 9",
                "metadata": {
                    "device": "Watch7,1",
                    "context": "Active",
                    "confidence": 0.95
                }
            },
            {
                "type": "BloodPressureSystolic",
                "unit": "mmHg",
                "value": 125.0,
                "date": "2024-01-15T10:31:00Z",
                "source": "Manual Entry"
            },
            {
                "type": "BloodPressureDiastolic",
                "unit": "mmHg", 
                "value": 82.0,
                "date": "2024-01-15T10:31:00Z",
                "source": "Manual Entry"
            },
            {
                "type": "SleepAnalysis",
                "unit": "min",
                "value": 480.0,
                "date": "2024-01-15T06:00:00Z",
                "endDate": "2024-01-15T14:00:00Z",
                "source": "iPhone",
                "metadata": {
                    "stage": "deep",
                    "efficiency": 0.85
                }
            },
            {
                "type": "ActiveEnergyBurned",
                "unit": "kcal",
                "value": 250.0,
                "date": "2024-01-15T10:00:00Z",
                "endDate": "2024-01-15T11:00:00Z",
                "source": "Apple Watch Series 9",
                "metadata": {
                    "activity": "running",
                    "distance": 5000
                }
            },
            {
                "type": "Workout",
                "unit": "min",
                "value": 45.0,
                "date": "2024-01-15T10:00:00Z",
                "endDate": "2024-01-15T10:45:00Z",
                "source": "Apple Watch Series 9",
                "metadata": {
                    "workoutType": "Running",
                    "totalDistance": 8000,
                    "totalEnergyBurned": 450,
                    "route": [
                        {"latitude": 37.7749, "longitude": -122.4194, "timestamp": "2024-01-15T10:00:00Z"},
                        {"latitude": 37.7849, "longitude": -122.4094, "timestamp": "2024-01-15T10:15:00Z"},
                        {"latitude": 37.7949, "longitude": -122.3994, "timestamp": "2024-01-15T10:30:00Z"},
                        {"latitude": 37.8049, "longitude": -122.3894, "timestamp": "2024-01-15T10:45:00Z"}
                    ]
                }
            }
        ],
        "device": {
            "model": "iPhone15,2",
            "systemVersion": "17.2.1",
            "appVersion": "2.1.0"
        },
        "exportDate": "2024-01-15T15:00:00Z"
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .insert_header(("user-agent", "Auto Export/2.1.0 iOS/17.2.1"))
        .set_json(&auto_export_payload)
        .to_request();

    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert!(ingest_resp.status().is_success(), "Ingest should succeed");

    let ingest_body: ApiResponse<IngestResponse> = test::read_body_json(ingest_resp).await;
    assert!(ingest_body.success);
    assert!(ingest_body.data.processed_count > 0);
    assert_eq!(ingest_body.data.failed_count, 0);
    assert!(ingest_body.data.errors.is_empty());
    
    println!("✓ Processed {} metrics successfully", ingest_body.data.processed_count);

    // Step 3: Verify data was stored in database
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let bp_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM blood_pressure_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let sleep_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM sleep_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let workout_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM workouts WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    // Verify expected data counts
    assert_eq!(heart_rate_count, 1, "Should have 1 heart rate metric");
    assert_eq!(bp_count, 1, "Should have 1 blood pressure metric (combined systolic/diastolic)");
    assert_eq!(sleep_count, 1, "Should have 1 sleep metric");
    assert_eq!(activity_count, 1, "Should have 1 activity metric");
    assert_eq!(workout_count, 1, "Should have 1 workout");

    // Step 4: Verify PostGIS geometry was stored for workout
    let workout_with_route = sqlx::query!(
        "SELECT route_geometry FROM workouts WHERE user_id = $1 AND route_geometry IS NOT NULL",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(workout_with_route.route_geometry.is_some(), "Workout should have route geometry");

    // Step 5: Verify raw ingestion was stored
    let raw_ingestion_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM raw_ingestions WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(raw_ingestion_count, 1, "Should have 1 raw ingestion record");

    // Step 6: Verify audit log entries
    let audit_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM audit_log WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert!(audit_count >= 1, "Should have audit log entries");

    println!("✓ Complete Auto Export workflow test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_duplicate_submission_handling() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "duplicate_e2e@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    let payload = json!({
        "data": [
            {
                "type": "HeartRate",
                "unit": "count/min",
                "value": 72.0,
                "date": "2024-01-15T12:00:00Z",
                "source": "Apple Watch"
            }
        ],
        "device": {
            "model": "iPhone15,2",
            "appVersion": "2.1.0"
        },
        "exportDate": "2024-01-15T15:00:00Z"
    });

    // First submission
    let req1 = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success());

    let body1: ApiResponse<IngestResponse> = test::read_body_json(resp1).await;
    assert!(body1.success);
    assert_eq!(body1.data.processed_count, 1);

    // Duplicate submission (same payload)
    let req2 = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert!(resp2.status().is_success());

    let body2: ApiResponse<IngestResponse> = test::read_body_json(resp2).await;
    assert!(body2.success);

    // Verify only one record was stored
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(heart_rate_count, 1, "Should only have 1 heart rate record despite duplicate submission");

    // Verify raw ingestions (should be deduplicated)
    let raw_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM raw_ingestions WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(raw_count, 1, "Should only have 1 raw ingestion record");

    println!("✓ Duplicate submission handling test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_large_batch_auto_export() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "large_batch_e2e@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Create a large batch simulating a week of data
    let mut data_points = Vec::new();
    
    // Generate heart rate data (7 days, every 15 minutes while awake = ~70 per day)
    let base_date = chrono::DateTime::parse_from_rfc3339("2024-01-15T06:00:00Z").unwrap().with_timezone(&chrono::Utc);
    
    for day in 0..7 {
        for hour in 6..22 { // Awake hours
            for quarter in 0..4 {
                let timestamp = base_date + chrono::Duration::days(day) + 
                               chrono::Duration::hours(hour) + chrono::Duration::minutes(quarter * 15);
                
                data_points.push(json!({
                    "type": "HeartRate",
                    "unit": "count/min",
                    "value": 70.0 + (day * hour + quarter) as f64 % 40.0, // Varying heart rates
                    "date": timestamp.to_rfc3339(),
                    "source": "Apple Watch"
                }));
            }
        }
    }

    // Add some daily sleep data
    for day in 0..7 {
        let sleep_start = base_date + chrono::Duration::days(day) + chrono::Duration::hours(23);
        data_points.push(json!({
            "type": "SleepAnalysis",
            "unit": "min",
            "value": 480.0 + (day as f64 * 10.0), // Varying sleep duration
            "date": sleep_start.to_rfc3339(),
            "endDate": (sleep_start + chrono::Duration::hours(8)).to_rfc3339(),
            "source": "iPhone",
            "metadata": {
                "stage": "deep",
                "efficiency": 0.80 + (day as f64 * 0.02)
            }
        }));
    }

    let large_payload = json!({
        "data": data_points,
        "device": {
            "model": "iPhone15,2",
            "systemVersion": "17.2.1",
            "appVersion": "2.1.0"
        },
        "exportDate": "2024-01-22T00:00:00Z"
    });

    println!("Testing large batch with {} data points", data_points.len());

    let start_time = std::time::Instant::now();
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&large_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let processing_time = start_time.elapsed();
    
    assert!(resp.status().is_success());

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(body.success);
    assert_eq!(body.data.failed_count, 0);
    assert!(body.data.processed_count > 0);

    // Performance check: should process large batch in reasonable time
    assert!(processing_time.as_secs() < 30, 
           "Large batch took {}s, expected < 30s", processing_time.as_secs());

    println!("✓ Large batch processed in {}ms: {} metrics", 
            processing_time.as_millis(), body.data.processed_count);

    // Verify data counts in database
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let sleep_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM sleep_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert!(heart_rate_count > 400, "Should have many heart rate metrics");
    assert_eq!(sleep_count, 7, "Should have 7 sleep records");

    println!("✓ Large batch E2E test passed: {} HR, {} sleep metrics", 
            heart_rate_count, sleep_count);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limiting_in_full_flow() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "rate_limit_e2e@example.com").await;

    // Create app with stricter rate limits for testing
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client, 3, std::time::Duration::from_secs(60))) // 3 per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/api/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = json!({
        "data": [
            {
                "type": "HeartRate",
                "unit": "count/min",
                "value": 75.0,
                "date": "2024-01-15T10:30:00Z",
                "source": "Apple Watch"
            }
        ]
    });

    // Make requests up to the limit
    for i in 0..3 {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .insert_header(("content-type", "application/json"))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Request {} should succeed", i);
        
        // Verify rate limit headers are present
        let headers = resp.headers();
        assert!(headers.contains_key("x-ratelimit-limit"));
        assert!(headers.contains_key("x-ratelimit-remaining"));
    }

    // Next request should be rate limited
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429, "Should be rate limited");

    println!("✓ Rate limiting in full flow test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "error_recovery_e2e@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Test with mixed valid/invalid data
    let mixed_payload = json!({
        "data": [
            {
                "type": "HeartRate",
                "unit": "count/min",
                "value": 75.0,
                "date": "2024-01-15T10:30:00Z",
                "source": "Apple Watch"
            },
            {
                "type": "HeartRate",
                "unit": "count/min", 
                "value": 500.0, // Invalid - too high
                "date": "2024-01-15T10:31:00Z",
                "source": "Apple Watch"
            },
            {
                "type": "BloodPressureSystolic",
                "unit": "mmHg",
                "value": 120.0,
                "date": "2024-01-15T10:32:00Z",
                "source": "Manual Entry"
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&mixed_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return validation error");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(!body.success);
    assert!(!body.data.errors.is_empty());
    assert!(body.data.errors[0].error_message.contains("heart rate"));

    // Verify raw ingestion was still stored (for debugging)
    let raw_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM raw_ingestions WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(raw_count, 1, "Raw payload should be stored even on validation error");

    // Test with completely invalid JSON
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload("{invalid json}")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "Should return JSON parsing error");

    println!("✓ Error handling and recovery test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_uuid_api_key_full_flow() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let auth_service = AuthService::new(pool.clone());

    // Clean up and create test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "uuid_e2e@example.com")
        .execute(&pool)
        .await
        .unwrap();

    let user = auth_service
        .create_user("uuid_e2e@example.com", Some("UUID E2E Test"))
        .await
        .unwrap();

    // Create UUID-based API key (Auto Export format)
    let uuid_key = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, key_hash, key_type, is_active, scopes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        uuid::Uuid::new_v4(),
        user.id,
        "Auto Export UUID Key",
        uuid_key,
        "uuid",
        true,
        Some(vec!["write".to_string()])
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = create_full_app(pool.clone(), redis_client).await;

    // Test with UUID key
    let payload = json!({
        "data": [
            {
                "type": "HeartRate",
                "unit": "count/min",
                "value": 78.0,
                "date": "2024-01-15T11:00:00Z",
                "source": "Apple Watch"
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", uuid_key)))
        .insert_header(("content-type", "application/json"))
        .insert_header(("user-agent", "Auto Export/2.1.0"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "UUID key should work");

    let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;
    assert!(body.success);
    assert_eq!(body.data.processed_count, 1);

    // Verify data was stored
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(heart_rate_count, 1);

    println!("✓ UUID API key full flow test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}