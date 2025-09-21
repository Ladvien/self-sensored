/// Comprehensive tests for authentication services and middleware
///
/// This test suite covers:
/// - Auth service validation, caching, and rate limiting
/// - Auth middleware Bearer token processing
/// - API key validation paths (UUID, hashed, invalid, expired)
/// - Rate limiting scenarios and isolation
/// - Security edge cases and error handling
/// - Authentication performance and concurrency
///
/// Test coverage goals:
/// - 100% auth service method coverage
/// - 100% middleware path coverage
/// - All authentication error scenarios
/// - Rate limiting behaviors
/// - Caching and performance validation

use actix_web::{test, web, App, HttpRequest, HttpResponse, Result};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::net::IpAddr;
use uuid::Uuid;

use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::services::auth::{AuthService, AuthContext, User, ApiKey, AuthError};
use self_sensored::services::rate_limiter::RateLimiter;

async fn get_test_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(10)
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

async fn create_test_user_and_key(pool: &sqlx::PgPool) -> (User, ApiKey, String) {
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("test_handler_{}@example.com", chrono::Utc::now().timestamp_micros());
    let user = auth_service
        .create_user(&test_email, Some("Test Handler User"), Some(json!("test-handler")))
        .await
        .unwrap();

    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Handler Key"),
            None,
            Some(json!(["read", "write"])),
            None
        )
        .await
        .unwrap();

    (user, api_key, plain_key)
}

async fn create_test_user_with_auth_service(auth_service: &AuthService, email: &str) -> User {
    // Add timestamp to apple_health_id to ensure uniqueness across tests
    let unique_id = format!("Test-{}-{}",
        email.split('@').next().unwrap_or("user"),
        chrono::Utc::now().timestamp_micros()
    );

    auth_service
        .create_user(email, Some("Test User"), Some(json!(unique_id)))
        .await
        .unwrap()
}


// =============================================================================
// AUTH SERVICE TESTS
// =============================================================================

#[tokio::test]
async fn test_api_key_generation_and_hashing() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Test API key generation uniqueness
    let key1 = AuthService::generate_api_key();
    let key2 = AuthService::generate_api_key();
    let key3 = AuthService::generate_api_key();

    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);

    // Keys should have correct prefix
    assert!(key1.starts_with("hea_"));
    assert!(key2.starts_with("hea_"));
    assert!(key3.starts_with("hea_"));

    // Keys should be proper length
    assert_eq!(key1.len(), 36); // hea_ + 32 hex chars
    assert_eq!(key2.len(), 36);
    assert_eq!(key3.len(), 36);

    // Test hashing
    let api_key = "test_api_key_12345";
    let hash1 = auth_service.hash_api_key(api_key).unwrap();
    let hash2 = auth_service.hash_api_key(api_key).unwrap();

    // Hashes should be different due to salt
    assert_ne!(hash1, hash2);

    // Both hashes should verify correctly
    assert!(auth_service.verify_api_key(api_key, &hash1).unwrap());
    assert!(auth_service.verify_api_key(api_key, &hash2).unwrap());

    // Wrong key should not verify
    assert!(!auth_service.verify_api_key("wrong_key", &hash1).unwrap());
    assert!(!auth_service.verify_api_key("wrong_key", &hash2).unwrap());

    // Hashes should start with $argon2
    assert!(hash1.starts_with("$argon2"));
    assert!(hash2.starts_with("$argon2"));
}

#[tokio::test]
async fn test_uuid_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up any existing test user
    let test_email = format!("uuid_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;
    let test_ip: IpAddr = "127.0.0.1".parse().unwrap();
    let test_user_agent = "TestAgent/1.0";

    // Insert a UUID-based API key directly (simulating Auto Export key)
    let api_key_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, is_active)
        VALUES ($1, $2, $3, $4)
        "#,
        api_key_id,
        user.id,
        "Auto Export Key",
        true
    )
    .execute(&pool)
    .await
    .unwrap();

    // Authenticate using UUID directly (Auto Export format)
    let auth_context = auth_service
        .authenticate(&api_key_id.to_string(), Some(test_ip), Some(test_user_agent))
        .await
        .unwrap();

    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.name, Some("Auto Export Key".to_string()));
    assert_eq!(auth_context.api_key.id, api_key_id);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_hashed_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("hashed_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;
    let test_ip: IpAddr = "192.168.1.100".parse().unwrap();
    let test_user_agent = "TestClient/2.0";

    // Create API key using the service (generates hashed key)
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Hashed Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Authenticate using the plain key
    let auth_context = auth_service
        .authenticate(&plain_key, Some(test_ip), Some(test_user_agent))
        .await
        .unwrap();

    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.name, Some("Test Hashed Key".to_string()));
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;

    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), Some(rate_limiter));

    let test_email = format!("rate_limit_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Rate Limited Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_expired_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("expired_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;
    let test_ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Create expired API key
    let expired_time = Utc::now() - Duration::hours(1);
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Expired Key"),
            Some(expired_time),
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Try to authenticate with expired key
    let result = auth_service
        .authenticate(&plain_key, Some(test_ip), Some("ExpiredTest/1.0"))
        .await;

    assert!(matches!(result, Err(AuthError::ApiKeyExpired)));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_invalid_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_ip: IpAddr = "172.16.0.1".parse().unwrap();
    let test_user_agent = "InvalidTest/1.0";

    // Try to authenticate with completely invalid key
    let result = auth_service
        .authenticate("totally_invalid_key", Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Try to authenticate with valid UUID format but non-existent key
    let fake_uuid = Uuid::new_v4();
    let result = auth_service
        .authenticate(&fake_uuid.to_string(), Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Try to authenticate with valid hea_ format but non-existent key
    let fake_hea_key = "hea_nonexistent12345678901234567890";
    let result = auth_service
        .authenticate(fake_hea_key, Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));
}

#[tokio::test]
async fn test_inactive_user_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("inactive_user_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Deactivate the user
    sqlx::query!("UPDATE users SET is_active = false WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();

    // Try to authenticate - should fail because user is inactive
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_inactive_api_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("inactive_key_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // First authenticate successfully
    let auth_context = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Deactivate the API key
    sqlx::query!(
        "UPDATE api_keys SET is_active = false WHERE id = $1",
        api_key.id
    )
    .execute(&pool)
    .await
    .unwrap();

    // Try to authenticate - should fail because API key is inactive
    let result = auth_service.authenticate(&plain_key, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_api_key_revocation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("revocation_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (_plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Revoke the API key
    let revoked = auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();
    assert!(revoked);

    // Try to revoke non-existent key
    let fake_key_id = Uuid::new_v4();
    let not_revoked = auth_service
        .revoke_api_key(fake_key_id, user.id)
        .await
        .unwrap();
    assert!(!not_revoked);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_list_api_keys() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("list_keys_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create multiple API keys
    let (_key1, api_key1) = auth_service
        .create_api_key(
            user.id,
            Some("Key 1"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    let (_key2, api_key2) = auth_service
        .create_api_key(
            user.id,
            Some("Key 2"),
            Some(Utc::now() + Duration::days(30)),
            Some(json!(["read", "write"])),
            None
        )
        .await
        .unwrap();

    // List API keys
    let api_keys = auth_service.list_api_keys(user.id).await.unwrap();

    assert_eq!(api_keys.len(), 2);
    assert!(api_keys.iter().any(|k| k.id == api_key1.id));
    assert!(api_keys.iter().any(|k| k.id == api_key2.id));

    // Keys should be ordered by created_at DESC (newest first)
    assert!(api_keys[0].created_at >= api_keys[1].created_at);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_last_used_timestamp_update() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("last_used_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Initially, last_used_at should be None
    assert!(api_key.last_used_at.is_none());

    // Authenticate - this should update last_used_at
    let _auth_context = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    // Check that last_used_at was updated
    let updated_keys = auth_service.list_api_keys(user.id).await.unwrap();
    let updated_key = updated_keys.iter().find(|k| k.id == api_key.id).unwrap();

    assert!(updated_key.last_used_at.is_some());
    assert!(updated_key.last_used_at.unwrap() > api_key.created_at.unwrap());

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

// =============================================================================
// AUTH MIDDLEWARE TESTS
// =============================================================================

async fn test_handler_with_auth(_auth_context: AuthContext) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({"status": "authenticated"})))
}

async fn test_handler_without_auth() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({"status": "public"})))
}

#[tokio::test]
async fn test_auth_middleware_valid_token() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    let test_email = format!("middleware_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create test user and API key
    let user = auth_service
        .create_user(&test_email, Some("Middleware Test"), Some(json!("middleware-test")))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Middleware Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "authenticated");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_auth_middleware_missing_header() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_auth_middleware_malformed_header() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", "InvalidFormat"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_auth_middleware_invalid_token() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", "Bearer invalid_key_12345"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_auth_middleware_health_check_bypass() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(test_handler_without_auth))
            .route("/api/v1/status", web::get().to(test_handler_without_auth)),
    )
    .await;

    // Health check should bypass auth
    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Status check should bypass auth
    let req = test::TestRequest::get()
        .uri("/api/v1/status")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

// =============================================================================
// PERFORMANCE TESTS
// =============================================================================

#[tokio::test]
async fn test_authentication_performance() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("perf_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Perf Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Test multiple authentication calls to ensure consistent performance
    let mut total_duration = std::time::Duration::from_nanos(0);
    let iterations = 10;

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let _auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        let duration = start.elapsed();
        total_duration += duration;
    }

    let average_duration = total_duration / iterations;

    // Requirement: <10ms for authentication check
    assert!(
        average_duration.as_millis() < 10,
        "Average authentication time {}ms exceeds 10ms requirement",
        average_duration.as_millis()
    );

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_concurrent_authentication_requests() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("concurrent_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Concurrent Test Key"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Run multiple concurrent authentication calls directly on the service
    let mut handles = vec![];

    for i in 0..10 {
        let auth_service_clone = AuthService::new(pool.clone());
        let key_clone = plain_key.clone();

        let handle = tokio::spawn(async move {
            let result = auth_service_clone
                .authenticate(&key_clone, None, None)
                .await;
            (i, result.is_ok())
        });

        handles.push(handle);
    }

    // Collect results
    let mut success_count = 0;
    for handle in handles {
        let (_i, is_success) = handle.await.unwrap();
        if is_success {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All concurrent authentication requests should succeed");

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

// =============================================================================
// SECURITY TESTS
// =============================================================================

#[tokio::test]
async fn test_permission_validation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = format!("permission_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Test admin permission check
    let admin_context = AuthContext::new_for_testing(user.id);
    let mut admin_context_with_perms = admin_context.clone();
    admin_context_with_perms.api_key.permissions = Some(json!(["admin"]));

    assert!(AuthService::has_admin_permission(&admin_context_with_perms));
    assert!(!AuthService::has_admin_permission(&admin_context));

    // Test specific permission check
    let read_context = AuthContext::new_for_testing(user.id);
    let mut read_context_with_perms = read_context.clone();
    read_context_with_perms.api_key.permissions = Some(json!(["read"]));

    assert!(AuthService::has_permission(&read_context_with_perms, "read"));
    assert!(!AuthService::has_permission(&read_context, "read"));

    // Admin should have all permissions
    assert!(AuthService::has_permission(&admin_context_with_perms, "read"));
    assert!(AuthService::has_permission(&admin_context_with_perms, "write"));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;

    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), Some(rate_limiter));

    let test_email = format!("isolation_test_{}@example.com", chrono::Utc::now().timestamp_micros());
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(&test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user_with_auth_service(&auth_service, &test_email).await;

    // Create two different API keys for the same user
    let (key1, _api_key1) = auth_service
        .create_api_key(
            user.id,
            Some("Key 1"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(
            user.id,
            Some("Key 2"),
            None,
            Some(json!(["read"])),
            None
        )
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}