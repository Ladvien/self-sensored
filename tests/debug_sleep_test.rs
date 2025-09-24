use actix_web::{middleware::Logger, test, web, App};
use self_sensored::handlers::health::health_check;
use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::middleware::rate_limit::RateLimitMiddleware;
use self_sensored::services::auth::AuthService;
use self_sensored::services::rate_limiter::RateLimiter;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "common/mod.rs"]
mod common;
use common::{cleanup_test_data, get_test_pool, setup_test_user_and_key};

#[tokio::test]
async fn test_sleep_processing_isolated() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "sleep_test@example.com").await;

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

    println!("Testing sleep data processing...");

    let sleep_payload = json!({
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
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_json(&sleep_payload)
        .to_request();

    println!("Sending sleep data request...");
    let resp = test::call_service(&app, req).await;
    println!("Response status: {}", resp.status());

    assert!(
        resp.status().is_success(),
        "Sleep data should process successfully"
    );

    cleanup_test_data(&pool, user_id).await;
    println!("Test completed successfully");
}
