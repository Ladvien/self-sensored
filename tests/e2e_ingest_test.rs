use actix_web::{test, web, App};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::services::auth::AuthService;

mod common;
use common::{cleanup_test_data, load_test_fixture, setup_test_db};

#[actix_web::test]
async fn test_ingest_small_payload() {
    // Setup
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Load small test fixture
    let payload = load_test_fixture("small").await;

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

    // Make request
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Assertions
    assert!(resp.status().is_success(), "Response: {:?}", resp.status());

    // Verify data was stored
    let metrics_count = verify_metrics_stored(&pool, user_id).await;
    assert!(metrics_count > 0, "Expected metrics to be stored");

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_medium_payload() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let payload = load_test_fixture("medium").await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("X-API-Key", api_key))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let metrics_count = verify_metrics_stored(&pool, user_id).await;
    assert!(
        metrics_count >= 50,
        "Expected at least 50 metrics, got {}",
        metrics_count
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_large_payload() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let payload = load_test_fixture("large").await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("X-API-Key", api_key))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let metrics_count = verify_metrics_stored(&pool, user_id).await;
    assert!(
        metrics_count >= 200,
        "Expected at least 200 metrics, got {}",
        metrics_count
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_duplicate_handling() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let payload = load_test_fixture("small").await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // First request
    let req1 = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
        .set_json(&payload)
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success());

    let first_count = verify_metrics_stored(&pool, user_id).await;

    // Second request with same data
    let req2 = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("X-API-Key", api_key))
        .set_json(&payload)
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert!(resp2.status().is_success());

    let second_count = verify_metrics_stored(&pool, user_id).await;

    // Should handle duplicates gracefully
    assert_eq!(
        first_count, second_count,
        "Duplicate data should not be inserted"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_invalid_auth() {
    let pool = setup_test_db().await;

    let payload = load_test_fixture("small").await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Request without API key
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401, "Should return unauthorized");
}

#[actix_web::test]
async fn test_ingest_rate_limiting() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let payload = load_test_fixture("small").await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Make multiple rapid requests
    let mut exceeded = false;
    for i in 0..150 {
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        if resp.status() == 429 {
            exceeded = true;
            println!("Rate limit exceeded at request {}", i + 1);
            break;
        }
    }

    assert!(exceeded, "Rate limit should be enforced");

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_partial_success() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Create payload with some invalid data
    let mut payload = load_test_fixture("medium").await;

    // Add some invalid metrics
    if let Some(metrics) = payload["data"]["metrics"].as_array_mut() {
        metrics.push(json!({
            "type": "HeartRate",
            "heart_rate": -10,  // Invalid negative heart rate
            "recorded_at": "2025-01-01T00:00:00Z"
        }));

        metrics.push(json!({
            "type": "BloodPressure",
            "systolic": 500,  // Invalid high blood pressure
            "diastolic": 400,
            "recorded_at": "2025-01-01T00:00:00Z"
        }));
    }

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("X-API-Key", api_key))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should still succeed with partial data
    assert!(resp.status().is_success());

    // Valid metrics should be stored
    let metrics_count = verify_metrics_stored(&pool, user_id).await;
    assert!(metrics_count > 0, "Valid metrics should be stored");

    cleanup_test_data(&pool, user_id).await;
}

// Helper functions
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

async fn create_test_api_key(pool: &PgPool, user_id: Uuid) -> String {
    let auth_service = AuthService::new(pool.clone());
    let (api_key, _) = auth_service
        .create_api_key(user_id, Some("Test Key"), None, None, None)
        .await
        .expect("Failed to create API key");
    api_key
}

async fn verify_metrics_stored(pool: &PgPool, user_id: Uuid) -> i64 {
    let heart_rate = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map(|r| r.count.unwrap_or(0))
    .unwrap_or(0);

    let activity = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map(|r| r.count.unwrap_or(0))
    .unwrap_or(0);

    let body = sqlx::query!(
        "SELECT COUNT(*) as count FROM body_measurements WHERE user_id = $1",
        user_id
    )
    .fetch_one(pool)
    .await
    .map(|r| r.count.unwrap_or(0))
    .unwrap_or(0);

    heart_rate + activity + body
}
