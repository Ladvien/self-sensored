use actix_web::{http::StatusCode, test, App};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use self_sensored::{
    middleware::{AdminMiddleware, AuthMiddleware},
    handlers::admin,
    services::{auth::AuthService, rate_limiter::RateLimiter},
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn get_test_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_user(pool: &sqlx::PgPool, user_id: Uuid) {
    // Clean up API keys first (foreign key constraint)
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    
    // Clean up user
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Create a test user with specified permissions
async fn create_test_user_with_permissions(
    auth_service: &AuthService,
    email: &str,
    permissions: Option<serde_json::Value>,
) -> (String, self_sensored::services::auth::User, self_sensored::services::auth::ApiKey) {
    let user = auth_service
        .create_user(email, Some("test_health_id"), None)
        .await
        .expect("Failed to create test user");

    let (api_key, api_key_record) = auth_service
        .create_api_key(
            user.id,
            Some("Test API Key"),
            Some(Utc::now() + Duration::hours(24)),
            permissions,
            Some(1000),
        )
        .await
        .expect("Failed to create API key");

    (api_key, user, api_key_record)
}

/// Create test app with middleware configured
fn create_test_app(
    auth_service: actix_web::web::Data<AuthService>,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(auth_service)
        .service(
            actix_web::web::scope("/api/v1")
                .wrap(AuthMiddleware) // Apply authentication middleware
                .service(
                    actix_web::web::scope("/admin")
                        .wrap(AdminMiddleware) // Apply admin authorization middleware
                        .route(
                            "/logging/level",
                            actix_web::web::get().to(admin::get_log_level),
                        )
                        .route(
                            "/logging/level",
                            actix_web::web::put().to(admin::set_log_level),
                        )
                        .route(
                            "/logging/stats",
                            actix_web::web::get().to(admin::get_logging_stats),
                        )
                        .route(
                            "/logging/test",
                            actix_web::web::post().to(admin::generate_test_logs),
                        ),
                ),
        )
}

#[tokio::test]
async fn test_admin_access_with_admin_permissions() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create user with admin permissions (array format)
    let admin_permissions = json!(["read", "write", "admin"]);
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "admin@test.com",
        Some(admin_permissions),
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test GET /admin/logging/level
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/logging/level")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["current_level"], "info");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_admin_access_with_admin_permissions_object_format() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create user with admin permissions (object format)
    let admin_permissions = json!({"read": true, "write": true, "admin": true});
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "admin2@test.com",
        Some(admin_permissions),
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test PUT /admin/logging/level
    let req = test::TestRequest::put()
        .uri("/api/v1/admin/logging/level")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .append_header(("Content-Type", "application/json"))
        .set_payload(r#"{"level": "debug"}"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["current_level"], "debug");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_admin_access_denied_without_admin_permissions() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create user with regular permissions (no admin)
    let regular_permissions = json!(["read", "write"]);
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "regular@test.com",
        Some(regular_permissions),
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test GET /admin/logging/level - should be forbidden
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/logging/level")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "access_denied");
    assert_eq!(body["required_permission"], "admin");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_admin_access_denied_with_no_permissions() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create user with no permissions
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "noperms@test.com",
        None,
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test POST /admin/logging/test - should be forbidden
    let req = test::TestRequest::post()
        .uri("/api/v1/admin/logging/test")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .append_header(("Content-Type", "application/json"))
        .set_payload(r#"{"count": 3, "level": "info"}"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "access_denied");
    assert_eq!(body["message"], "Admin privileges required to generate test logs");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_admin_access_denied_without_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test without authentication header - should be unauthorized
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/logging/stats")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_access_denied_with_invalid_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test with invalid API key - should be unauthorized
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/logging/stats")
        .append_header(("Authorization", "Bearer invalid_key_12345"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_set_log_level_validation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create admin user
    let admin_permissions = json!(["admin"]);
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "admin3@test.com",
        Some(admin_permissions),
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test invalid log level
    let req = test::TestRequest::put()
        .uri("/api/v1/admin/logging/level")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .append_header(("Content-Type", "application/json"))
        .set_payload(r#"{"level": "invalid"}"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("Invalid log level"));

    // Test valid log level
    let req = test::TestRequest::put()
        .uri("/api/v1/admin/logging/level")
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .append_header(("Content-Type", "application/json"))
        .set_payload(r#"{"level": "warn"}"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["current_level"], "warn");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_all_admin_endpoints_require_admin_permission() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Create regular user without admin permissions
    let regular_permissions = json!(["read", "write"]);
    let (api_key, user, _) = create_test_user_with_permissions(
        &auth_service,
        "regular2@test.com",
        Some(regular_permissions),
    )
    .await;

    let app = test::init_service(create_test_app(actix_web::web::Data::new(auth_service))).await;

    // Test all admin endpoints should return 403
    let endpoints = vec![
        ("GET", "/api/v1/admin/logging/level"),
        ("PUT", "/api/v1/admin/logging/level"),
        ("GET", "/api/v1/admin/logging/stats"),
        ("POST", "/api/v1/admin/logging/test"),
    ];

    for (method, uri) in endpoints {
        let req = match method {
            "GET" => test::TestRequest::get().uri(uri),
            "PUT" => test::TestRequest::put().uri(uri).set_payload(r#"{"level": "info"}"#),
            "POST" => test::TestRequest::post().uri(uri).set_payload(r#"{"count": 1}"#),
            _ => panic!("Unsupported method"),
        }
        .append_header(("Authorization", format!("Bearer {}", api_key)))
        .append_header(("Content-Type", "application/json"))
        .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "Endpoint {} {} should require admin permission",
            method,
            uri
        );

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "access_denied", "Wrong error type for {} {}", method, uri);
    }

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}