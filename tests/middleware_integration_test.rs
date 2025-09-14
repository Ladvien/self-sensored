use actix_web::{test, web, App, HttpResponse};
use self_sensored::{
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    services::{auth::AuthService, rate_limiter::RateLimiter},
};
use sqlx::postgres::PgPoolOptions;

async fn protected_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"message": "Protected content"}))
}

async fn get_test_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
async fn test_full_auth_flow() {
    let pool = get_test_pool().await;
    let auth_service = web::Data::new(AuthService::new(pool));
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(10));

    // Create a test user and API key
    let user = auth_service
        .create_user(
            &format!("integration-test-{}@example.com", uuid::Uuid::new_v4()),
            Some(&format!("integration_test_user_{}", uuid::Uuid::new_v4())),
            Some(serde_json::json!({"name": "Integration Test User"})),
        )
        .await
        .unwrap();

    let (api_key, _) = auth_service
        .create_api_key(
            user.id,
            Some("Integration Test Key"),
            None,
            Some(serde_json::json!(["read", "write"])),
            None,
        )
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(auth_service.clone())
            .app_data(rate_limiter.clone())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/protected", web::get().to(protected_handler)),
    )
    .await;

    // Test successful authentication and authorization
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Verify rate limit headers are present
    let headers = resp.headers();
    assert!(headers.contains_key("X-RateLimit-Limit"));
    assert!(headers.contains_key("X-RateLimit-Remaining"));
    assert!(headers.contains_key("X-RateLimit-Reset"));

    // Test invalid API key
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Authorization", "Bearer invalid_key"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Test missing authorization header
    let req = test::TestRequest::get().uri("/protected").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_health_endpoint_bypass() {
    let pool = get_test_pool().await;
    let auth_service = web::Data::new(AuthService::new(pool));
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(0)); // No requests allowed

    let app = test::init_service(
        App::new()
            .app_data(auth_service)
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().json("healthy") }),
            ),
    )
    .await;

    // Health endpoint should work without authentication
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_rate_limiting() {
    let pool = get_test_pool().await;
    let auth_service = web::Data::new(AuthService::new(pool));
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(2)); // Only 2 requests allowed

    // Create a test user and API key
    let user = auth_service
        .create_user(
            &format!("rate-limit-test-{}@example.com", uuid::Uuid::new_v4()),
            Some(&format!("rate_limit_test_{}", uuid::Uuid::new_v4())),
            Some(serde_json::json!({"name": "Rate Limit Test User"})),
        )
        .await
        .unwrap();

    let (api_key, _) = auth_service
        .create_api_key(
            user.id,
            Some("Rate Limit Test Key"),
            None,
            Some(serde_json::json!(["read"])),
            None,
        )
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(auth_service.clone())
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(protected_handler)),
    )
    .await;

    let auth_header = format!("Bearer {api_key}");

    // First 2 requests should succeed
    for i in 0..2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", auth_header.clone()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let headers = resp.headers();
        let remaining = headers
            .get("X-RateLimit-Remaining")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(remaining, (1 - i).to_string());
    }

    // Third request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", auth_header))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429); // Too Many Requests

    let headers = resp.headers();
    assert!(headers.contains_key("Retry-After"));

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_api_key_expiration() {
    let pool = get_test_pool().await;
    let auth_service = web::Data::new(AuthService::new(pool));
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(10));

    // Create a test user and expired API key
    let user = auth_service
        .create_user(
            &format!("expiry-test-{}@example.com", uuid::Uuid::new_v4()),
            Some(&format!("expiry_test_{}", uuid::Uuid::new_v4())),
            Some(serde_json::json!({"name": "Expiry Test User"})),
        )
        .await
        .unwrap();

    let expired_time = chrono::Utc::now() - chrono::Duration::hours(1); // Expired 1 hour ago
    let (expired_api_key, _) = auth_service
        .create_api_key(
            user.id,
            Some("Expired Test Key"),
            Some(expired_time),
            Some(serde_json::json!(["read"])),
            None,
        )
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(auth_service.clone())
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(protected_handler)),
    )
    .await;

    // Request with expired API key should fail
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {expired_api_key}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_inactive_user() {
    let pool = get_test_pool().await;
    let auth_service = web::Data::new(AuthService::new(pool));
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(10));

    // Create a test user and API key
    let user = auth_service
        .create_user(
            &format!("inactive-test-{}@example.com", uuid::Uuid::new_v4()),
            Some(&format!("inactive_test_{}", uuid::Uuid::new_v4())),
            Some(serde_json::json!({"name": "Inactive Test User"})),
        )
        .await
        .unwrap();

    let (api_key, _) = auth_service
        .create_api_key(
            user.id,
            Some("Inactive Test Key"),
            None,
            Some(serde_json::json!(["read"])),
            None,
        )
        .await
        .unwrap();

    // Deactivate the user
    sqlx::query!("UPDATE users SET is_active = false WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(auth_service.clone())
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(protected_handler)),
    )
    .await;

    // Request with API key from inactive user should fail
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {api_key}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401); // Should be unauthorized due to inactive user

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();
}
