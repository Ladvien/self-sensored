use actix_web::{middleware::Logger, test, web, App};
use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::middleware::rate_limit::RateLimitMiddleware;
use self_sensored::services::auth::AuthService;
use self_sensored::services::rate_limiter::RateLimiter;
use serde_json::json;

#[path = "common/mod.rs"]
mod common;
use common::{cleanup_test_data, get_test_pool, setup_test_user_and_key};

#[tokio::test]
async fn test_debug_performance_issue() {
    println!("Starting debug test...");

    let pool = get_test_pool().await;
    println!("Got test pool");

    let (user_id, api_key) = setup_test_user_and_key(&pool, "debug_perf@example.com").await;
    println!("Created user: {} with key: {}", user_id, api_key);

    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(1000));
    println!("Created rate limiter");

    let auth_service = web::Data::new(AuthService::new(pool.clone()));
    println!("Created auth service");

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
    println!("App initialized");

    // Test just heart rate first
    let heart_rate_payload = json!({
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
                }
            ]
        }
    });

    println!("Sending heart rate request...");
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&heart_rate_payload)
        .to_request();

    println!("About to call service...");
    let resp = test::call_service(&app, req).await;
    println!("Heart Rate Response status: {}", resp.status());

    assert!(
        resp.status().is_success(),
        "Heart rate request should succeed"
    );

    // Now test blood pressure
    let bp_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "date": "2024-01-15 14:00:00 +0000",
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
                            "date": "2024-01-15 14:00:00 +0000",
                            "qty": 80.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    });

    println!("Sending blood pressure request...");
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&bp_payload)
        .to_request();

    println!("About to call service for BP...");
    let resp = test::call_service(&app, req).await;
    println!("Blood Pressure Response status: {}", resp.status());

    assert!(
        resp.status().is_success(),
        "Blood pressure request should succeed"
    );

    cleanup_test_data(&pool, user_id).await;
    println!("Test completed successfully!");
}
