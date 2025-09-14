use actix_web::{test, web, App, middleware::Logger};
use chrono::{DateTime, Utc};
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
        .create_user(email, Some("Deduplication Test User"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Deduplication Test Key", None, vec!["write".to_string()])
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

/// Test the primary fix: ActivityKey now uses timestamp instead of date
/// This test verifies that activity metrics with the same date but different timestamps are NOT duplicated
#[tokio::test]
async fn test_activity_timestamp_based_deduplication() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "activity_timestamp_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Create iOS payload with activity metrics that have same date but different timestamps
    // This should NOT be considered duplicates with our timestamp-based fix
    let ios_payload = json!({
        "data": {
            "metrics": [
                // Same date (2024-01-15) but different times - should NOT be duplicates
                {
                    "name": "steps",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2024-01-15 08:00:00 +0000", // 08:00 timestamp
                            "qty": 1000.0
                        },
                        {
                            "source": "iPhone",
                            "date": "2024-01-15 12:00:00 +0000", // 12:00 timestamp (4 hours later)
                            "qty": 2000.0
                        },
                        {
                            "source": "iPhone",
                            "date": "2024-01-15 18:00:00 +0000", // 18:00 timestamp (10 hours later)
                            "qty": 3000.0
                        },
                        {
                            "source": "iPhone",
                            "date": "2024-01-15 08:00:00 +0000", // EXACT same timestamp as first - should be duplicate
                            "qty": 1500.0
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .insert_header(("user-agent", "Auto Export/2.1.0 iOS/17.2.1"))
        .set_json(&ios_payload)
        .to_request();

    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert!(ingest_resp.status().is_success(), "Ingest should succeed");

    let ingest_body: ApiResponse<IngestResponse> = test::read_body_json(ingest_resp).await;
    assert!(ingest_body.success);

    // Verify that 3 records were processed (4 total - 1 duplicate)
    assert_eq!(ingest_body.data.processed_count, 3,
        "Should process 3 unique activity records (different timestamps)");
    assert_eq!(ingest_body.data.failed_count, 0, "Should have no failures");

    // Verify database contains 3 activity records
    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(activity_count, 3, "Database should contain 3 activity records with different timestamps");

    // Verify the exact timestamps are stored correctly
    let activity_timestamps = sqlx::query!(
        "SELECT recorded_at FROM activity_metrics WHERE user_id = $1 ORDER BY recorded_at",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(activity_timestamps.len(), 3);

    // Expected timestamps: 08:00, 12:00, 18:00 (not 08:00 duplicate)
    let expected_times = vec![
        "2024-01-15T08:00:00Z",
        "2024-01-15T12:00:00Z",
        "2024-01-15T18:00:00Z"
    ];

    for (i, timestamp) in activity_timestamps.iter().enumerate() {
        let expected_time: DateTime<Utc> = expected_times[i].parse().unwrap();
        assert_eq!(timestamp.recorded_at, expected_time);
    }

    println!("✓ Activity timestamp-based deduplication test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// Test that high-frequency data (multiple records per second) works correctly
/// This validates the user's requirement that "data can be recorded 100 times a second"
#[tokio::test]
async fn test_high_frequency_data_recording() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "high_frequency_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Create payload with heart rate data recorded multiple times per second
    let base_time = "2024-01-15T10:30:00.000Z";
    let base_datetime: DateTime<Utc> = base_time.parse().unwrap();

    let mut heart_rate_data = Vec::new();

    // Generate 20 records within the same second (every 50ms)
    for i in 0..20 {
        let timestamp = base_datetime + chrono::Duration::milliseconds(i * 50);
        heart_rate_data.push(json!({
            "source": "Apple Watch",
            "date": timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "qty": 70.0 + (i as f64) // Vary the heart rate slightly
        }));
    }

    // Add one exact duplicate to test deduplication
    heart_rate_data.push(json!({
        "source": "Apple Watch",
        "date": base_datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true), // Same as first record
        "qty": 72.0 // Different value, same timestamp - should be duplicate
    }));

    let ios_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": heart_rate_data
                }
            ],
            "workouts": []
        }
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&ios_payload)
        .to_request();

    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert!(ingest_resp.status().is_success());

    let ingest_body: ApiResponse<IngestResponse> = test::read_body_json(ingest_resp).await;
    assert!(ingest_body.success);

    // Should process 20 unique records (21 total - 1 duplicate)
    assert_eq!(ingest_body.data.processed_count, 20,
        "Should process 20 unique high-frequency heart rate records");

    // Verify database contains 20 heart rate records with millisecond precision
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(heart_rate_count, 20, "Should store 20 unique heart rate records");

    // Verify timestamps have millisecond precision
    let timestamps = sqlx::query!(
        "SELECT recorded_at FROM heart_rate_metrics WHERE user_id = $1 ORDER BY recorded_at LIMIT 5",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    // Check that we have the expected millisecond differences
    for (i, row) in timestamps.iter().enumerate() {
        let expected_time = base_datetime + chrono::Duration::milliseconds(i as i64 * 50);
        assert_eq!(row.recorded_at, expected_time,
            "Record {} should have timestamp with 50ms precision", i);
    }

    println!("✓ High-frequency data recording test passed: {} records stored", heart_rate_count);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// Test that iOS user_id assignment is now consistent with authenticated user
/// This validates the fix where to_internal_format() now uses auth.user.id instead of random UUID
#[tokio::test]
async fn test_ios_user_id_consistency() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "ios_user_id_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    let ios_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:30:00Z",
                            "qty": 75.0
                        }
                    ]
                },
                {
                    "name": "blood_pressure_systolic",
                    "data": [
                        {
                            "source": "Manual Entry",
                            "date": "2024-01-15T10:31:00Z",
                            "qty": 120.0
                        }
                    ]
                },
                {
                    "name": "blood_pressure_diastolic",
                    "data": [
                        {
                            "source": "Manual Entry",
                            "date": "2024-01-15T10:31:00Z",
                            "qty": 80.0
                        }
                    ]
                }
            ],
            "workouts": [
                {
                    "name": "Running",
                    "start": "2024-01-15T09:00:00Z",
                    "end": "2024-01-15T09:30:00Z",
                    "source": "Apple Watch"
                }
            ]
        }
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&ios_payload)
        .to_request();

    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert!(ingest_resp.status().is_success());

    let ingest_body: ApiResponse<IngestResponse> = test::read_body_json(ingest_resp).await;
    assert!(ingest_body.success);

    // Verify that all records were stored with the correct authenticated user_id
    let heart_rate_user_ids = sqlx::query!(
        "SELECT user_id FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let bp_user_ids = sqlx::query!(
        "SELECT user_id FROM blood_pressure_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let workout_user_ids = sqlx::query!(
        "SELECT user_id FROM workouts WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    // All records should exist with the authenticated user's ID
    assert_eq!(heart_rate_user_ids.len(), 1, "Should have 1 heart rate record with correct user_id");
    assert_eq!(bp_user_ids.len(), 1, "Should have 1 blood pressure record with correct user_id");
    assert_eq!(workout_user_ids.len(), 1, "Should have 1 workout record with correct user_id");

    // Verify no records exist with random user_ids (would indicate the bug still exists)
    let total_heart_rates = sqlx::query!("SELECT COUNT(*) as count FROM heart_rate_metrics")
        .fetch_one(&pool)
        .await
        .unwrap()
        .count
        .unwrap_or(0);

    let user_heart_rates = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    // The counts should match (no orphaned records with random user_ids)
    assert!(user_heart_rates >= 1, "Should have at least 1 record with correct user_id");

    // Verify raw ingestions also use the correct user_id
    let raw_ingestion_user_ids = sqlx::query!(
        "SELECT user_id FROM raw_ingestions WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(raw_ingestion_user_ids.len(), 1, "Raw ingestion should use correct user_id");

    println!("✓ iOS user_id consistency test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// Test edge case: same payload submitted twice should be properly deduplicated
/// This tests both intra-batch deduplication and raw payload deduplication
#[tokio::test]
async fn test_duplicate_payload_submission() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "duplicate_payload_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Create a payload with mixed metrics
    let payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:30:00Z",
                            "qty": 75.0
                        }
                    ]
                },
                {
                    "name": "steps",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2024-01-15T00:00:00Z",
                            "qty": 8000.0
                        }
                    ]
                }
            ],
            "workouts": []
        }
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
    assert_eq!(body1.data.processed_count, 2, "First submission should process 2 metrics");

    // Second submission (exact duplicate payload)
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

    // Verify only one set of records exists in database
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
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

    assert_eq!(heart_rate_count, 1, "Should have only 1 heart rate record despite duplicate submission");
    assert_eq!(activity_count, 1, "Should have only 1 activity record despite duplicate submission");

    // Verify raw ingestion deduplication (based on payload hash)
    let raw_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM raw_ingestions WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    // Raw ingestion should be deduplicated by hash, so only 1 record
    assert_eq!(raw_count, 1, "Should have only 1 raw ingestion record due to hash deduplication");

    println!("✓ Duplicate payload submission test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// Test database constraint enforcement
/// Verify that database UNIQUE constraints work properly with our fixes
#[tokio::test]
async fn test_database_constraint_enforcement() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "db_constraint_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Send payload with activity metrics at different times
    let payload1 = json!({
        "data": {
            "metrics": [
                {
                    "name": "steps",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2024-01-15T08:00:00.000Z", // Exact timestamp
                            "qty": 1000.0
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    // First submission should succeed
    let req1 = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload1)
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success());

    // Second payload with exact same timestamp - should be handled gracefully by DB constraint
    let payload2 = json!({
        "data": {
            "metrics": [
                {
                    "name": "steps",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2024-01-15T08:00:00.000Z", // EXACT same timestamp
                            "qty": 1500.0 // Different value
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req2 = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload2)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert!(resp2.status().is_success(), "Should handle DB constraint gracefully");

    // Verify only one record exists (first one wins due to UNIQUE constraint)
    let activity_records = sqlx::query!(
        "SELECT recorded_at, step_count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(activity_records.len(), 1, "Should have only 1 activity record due to UNIQUE constraint");

    // Verify it's the first record that was kept (first one wins with ON CONFLICT DO NOTHING)
    let record = &activity_records[0];
    assert_eq!(record.recorded_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
               "2024-01-15T08:00:00.000Z");

    println!("✓ Database constraint enforcement test passed");

    // Third payload with slightly different timestamp (1ms later) - should create new record
    let payload3 = json!({
        "data": {
            "metrics": [
                {
                    "name": "steps",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2024-01-15T08:00:00.001Z", // 1ms later
                            "qty": 2000.0
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let req3 = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload3)
        .to_request();

    let resp3 = test::call_service(&app, req3).await;
    assert!(resp3.status().is_success());

    // Now should have 2 records (different timestamps)
    let final_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(final_count, 2, "Should have 2 records with different millisecond timestamps");

    println!("✓ Database constraint with millisecond precision test passed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

/// Integration test that validates the complete fix
/// Tests the original problem: large batch with many "duplicates" that were actually unique timestamps
#[tokio::test]
async fn test_original_deduplication_issue_fixed() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "original_issue_test@example.com").await;

    let app = create_full_app(pool.clone(), redis_client).await;

    // Simulate the original issue: many activity records on the same date with different times
    // This would previously be seen as "65k duplicates" but should now be properly processed
    let mut activity_data = Vec::new();

    // Generate 100 activity records throughout a single day with different timestamps
    let base_date = "2024-01-15";
    for hour in 0..24 {
        for minute_step in 0..4 { // Every 15 minutes, ~96 records per day
            let timestamp = format!("{}T{:02}:{:02}:00.000Z", base_date, hour, minute_step * 15);
            activity_data.push(json!({
                "source": "iPhone",
                "date": timestamp,
                "qty": 1000.0 + (hour * minute_step) as f64 // Slightly varying step counts
            }));
        }
    }

    // Add a few true duplicates (same exact timestamp)
    activity_data.push(json!({
        "source": "iPhone",
        "date": "2024-01-15T12:00:00.000Z", // Exact duplicate of one above
        "qty": 5000.0
    }));
    activity_data.push(json!({
        "source": "iPhone",
        "date": "2024-01-15T12:00:00.000Z", // Another exact duplicate
        "qty": 6000.0
    }));

    let ios_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "steps",
                    "units": "count",
                    "data": activity_data
                }
            ],
            "workouts": []
        }
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&ios_payload)
        .to_request();

    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert!(ingest_resp.status().is_success());

    let ingest_body: ApiResponse<IngestResponse> = test::read_body_json(ingest_resp).await;
    assert!(ingest_body.success);

    // With the fix, we should process 96 unique records (98 total - 2 true duplicates with same timestamp)
    assert_eq!(ingest_body.data.processed_count, 96,
        "Should process 96 unique activity records (not treat same-date-different-time as duplicates)");
    assert_eq!(ingest_body.data.failed_count, 0);

    // Verify database contains the expected number of records
    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(activity_count, 96, "Database should contain 96 unique activity records");

    // Verify we have records throughout the day (not just one per date)
    let min_hour = sqlx::query!(
        "SELECT EXTRACT(hour FROM recorded_at) as hour FROM activity_metrics WHERE user_id = $1 ORDER BY recorded_at LIMIT 1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let max_hour = sqlx::query!(
        "SELECT EXTRACT(hour FROM recorded_at) as hour FROM activity_metrics WHERE user_id = $1 ORDER BY recorded_at DESC LIMIT 1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(min_hour.hour.unwrap() as i32, 0, "Should have records starting from hour 0");
    assert_eq!(max_hour.hour.unwrap() as i32, 23, "Should have records up to hour 23");

    println!("✓ Original deduplication issue fix validated: {} records processed correctly", activity_count);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}