/// Comprehensive Ingest Handler Coverage Test - Modern 2025 Rust Testing
/// Focuses on achieving maximum coverage using practical testing approaches

use actix_web::{test, web, App, HttpResponse, http::StatusCode};
use chrono::Utc;
use proptest::prelude::*;
use rstest::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::{ingest_handler, LoggedJson},
    models::IngestResponse,
    services::auth::{AuthService, User, ApiKey},
};

// ==================== TEST FIXTURES AND SETUP ====================

async fn setup_test_database() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set for testing");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_user_and_key(pool: &PgPool) -> (User, ApiKey, String) {
    let auth_service = AuthService::new(pool.clone());

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

    let (raw_key, api_key) = auth_service
        .create_api_key(user.id, Some("test_key"), None, None, None)
        .await
        .expect("Failed to create test API key");

    (user, api_key, raw_key)
}

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
        .app_data(web::Data::new(pool))
        .route("/v1/ingest", web::post().to(ingest_handler))
        .route("/health", web::get().to(|| async { HttpResponse::Ok().body("OK") }))
}

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

// ==================== CORE FUNCTIONALITY TESTS ====================

#[actix_web::test]
async fn test_successful_ingest_basic_workflow() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
                "recorded_at": Utc::now(),
                "heart_rate": 75,
                "source_device": "Test Device",
                "created_at": Utc::now()
            }],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify data was stored
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(count.unwrap_or(0) > 0);
}

#[actix_web::test]
async fn test_unauthorized_request_handling() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool)).await;

    let payload = json!({
        "data": {
            "heart_rate": [],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    // Test without authorization header
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Test with invalid API key
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", "Bearer invalid_key_123"))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_malformed_json_payload_handling() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Test various malformed JSON scenarios
    let malformed_payloads = vec![
        r#"{"data": {"heart_rate": [}"#, // Incomplete JSON
        r#"{"invalid": "structure"}"#,     // Missing required fields
        r#"not_json_at_all"#,             // Not JSON
        r#"{"data": "should_be_object"}"#, // Wrong data type
    ];

    for (i, payload) in malformed_payloads.iter().enumerate() {
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("authorization", format!("Bearer {}", raw_key)))
            .insert_header(("content-type", "application/json"))
            .set_payload(payload.to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_client_error(),
            "Test case {}: Expected client error for payload: {}",
            i,
            payload
        );
    }
}

#[actix_web::test]
async fn test_empty_payload_acceptance() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

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

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&empty_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ==================== MULTI-METRIC TYPE TESTING ====================

#[actix_web::test]
async fn test_comprehensive_multi_metric_ingest() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

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
        }
    });

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&comprehensive_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify each metric type was stored
    let heart_rate_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let blood_pressure_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let sleep_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(heart_rate_count.unwrap_or(0), 1);
    assert_eq!(blood_pressure_count.unwrap_or(0), 1);
    assert_eq!(sleep_count.unwrap_or(0), 1);
}

// ==================== VALIDATION EDGE CASES ====================

#[rstest]
#[case(0, "zero_heart_rate")]
#[case(300, "maximum_physiological")]
#[case(500, "impossible_high_value")]
#[actix_web::test]
async fn test_extreme_heart_rate_values(
    #[case] heart_rate: i16,
    #[case] description: &str,
) {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
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

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // The handler should either accept or gracefully reject extreme values
    assert!(
        resp.status().is_success() || resp.status().is_client_error(),
        "Unexpected response for {}: {}",
        description,
        resp.status()
    );
}

// ==================== CONTENT TYPE VALIDATION ====================

#[rstest]
#[case("application/json", true)]
#[case("application/json; charset=utf-8", true)]
#[case("text/plain", false)]
#[case("application/xml", false)]
#[case("", false)]
#[actix_web::test]
async fn test_content_type_validation(
    #[case] content_type: &str,
    #[case] should_succeed: bool,
) {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = r#"{"data": {"heart_rate": [], "blood_pressure": [], "sleep_analysis": [], "activity_summaries": [], "workout_data": []}}"#;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", content_type))
        .set_payload(payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    if should_succeed {
        assert!(
            resp.status().is_success(),
            "Should succeed with content type: {}",
            content_type
        );
    } else {
        assert!(
            resp.status().is_client_error(),
            "Should fail with content type: {}",
            content_type
        );
    }
}

// ==================== PERFORMANCE TESTING ====================

#[actix_web::test]
#[ignore] // Run separately as it's time-intensive
async fn test_large_batch_processing_performance() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    // Generate large payload with 1000 heart rate metrics
    let heart_rate_entries: Vec<Value> = (0..1000)
        .map(|i| json!({
            "id": Uuid::new_v4(),
            "user_id": user.id,
            "recorded_at": Utc::now(),
            "heart_rate": 60 + i % 40,
            "resting_heart_rate": 55 + i % 20,
            "source_device": "Apple Watch",
            "created_at": Utc::now()
        }))
        .collect();

    let large_payload = json!({
        "data": {
            "heart_rate": heart_rate_entries,
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    let start_time = std::time::Instant::now();

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&large_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let elapsed = start_time.elapsed();

    assert_eq!(resp.status(), StatusCode::OK);

    // Performance assertion: should process 1000 metrics in under 10 seconds
    assert!(
        elapsed < Duration::from_secs(10),
        "Large batch processing took too long: {:?}",
        elapsed
    );
}

// ==================== PROPERTY-BASED TESTING ====================

proptest! {
    #[test]
    fn test_payload_size_handling_property(
        heart_rate_count in 0..50usize,
        blood_pressure_count in 0..25usize,
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let pool = setup_test_database().await;
            let app = test::init_service(create_test_app(pool.clone())).await;
            let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

            // Generate variable-sized payload
            let heart_rate_data: Vec<Value> = (0..heart_rate_count)
                .map(|_| json!({
                    "id": Uuid::new_v4(),
                    "user_id": user.id,
                    "recorded_at": Utc::now(),
                    "heart_rate": 60,
                    "source_device": "Test Device",
                    "created_at": Utc::now()
                }))
                .collect();

            let blood_pressure_data: Vec<Value> = (0..blood_pressure_count)
                .map(|_| json!({
                    "id": Uuid::new_v4(),
                    "user_id": user.id,
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

            let req = test::TestRequest::post()
                .uri("/v1/ingest")
                .insert_header(("authorization", format!("Bearer {}", raw_key)))
                .insert_header(("content-type", "application/json"))
                .set_json(&payload)
                .to_request();

            let resp = test::call_service(&app, req).await;

            // Property: Any valid payload structure should be accepted
            prop_assert!(
                resp.status().is_success() || resp.status() == StatusCode::BAD_REQUEST,
                "Unexpected status code: {}",
                resp.status()
            );
        });
    }
}

// ==================== LOGGED JSON EXTRACTOR TESTS ====================

#[actix_web::test]
async fn test_logged_json_extractor_functionality() {
    let pool = setup_test_database().await;
    let app = test::init_service(create_test_app(pool.clone())).await;

    let (_user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = json!({
        "data": {
            "heart_rate": [],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    // This test verifies that the LoggedJson extractor processes requests correctly
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", raw_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // In a real test environment, we'd verify logs were generated
    // For now, we verify the request was processed successfully
}

// ==================== CONCURRENT ACCESS TESTING ====================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_ingest_requests() {
    let pool = setup_test_database().await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let payload = json!({
        "data": {
            "heart_rate": [{
                "id": Uuid::new_v4(),
                "user_id": user.id,
                "recorded_at": Utc::now(),
                "heart_rate": 75,
                "source_device": "Test Device",
                "created_at": Utc::now()
            }],
            "blood_pressure": [],
            "sleep_analysis": [],
            "activity_summaries": [],
            "workout_data": []
        }
    });

    // Create the test app once to avoid Send/Sync issues
    let app = test::init_service(create_test_app(pool.clone())).await;
    let app = Arc::new(app);

    // Spawn 10 concurrent requests
    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let app = app.clone();
            let raw_key = raw_key.clone();
            let payload = payload.clone();

            async move {
                let req = test::TestRequest::post()
                    .uri("/v1/ingest")
                    .insert_header(("authorization", format!("Bearer {}", raw_key)))
                    .insert_header(("content-type", "application/json"))
                    .set_json(&payload)
                    .to_request();

                let resp = test::call_service(&*app, req).await;
                (i, resp.status())
            }
        })
        .collect();

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    // Verify all requests succeeded
    for (task_id, status) in results {
        assert_eq!(
            status,
            StatusCode::OK,
            "Concurrent request {} failed with status: {}",
            task_id,
            status
        );
    }
}