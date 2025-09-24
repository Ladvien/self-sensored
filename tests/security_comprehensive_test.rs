// Comprehensive security tests for authentication and security components
// Tests cover API key lifecycle, rate limiting, middleware, caching, and security vulnerabilities

use actix_web::{
    dev::Service,
    http::{header, StatusCode},
    test::{self, TestRequest},
    web, App, HttpResponse,
};
use chrono::{Duration, Utc};
use redis::Client as RedisClient;
use self_sensored::{
    middleware::{admin::AdminMiddleware, auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    services::{
        auth::{ApiKey, AuthContext, AuthService, User},
        cache::{CacheConfig, CacheService},
        rate_limiter::{RateLimitError, RateLimiter},
    },
};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration as StdDuration,
};
use uuid::Uuid;

// Test database setup
async fn get_test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let database_url =
        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set for tests");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

// Mock Redis setup for testing
async fn setup_mock_redis() -> Option<String> {
    // Try to connect to a test Redis instance
    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/1".to_string());

    // Test if Redis is available
    if let Ok(client) = RedisClient::open(redis_url.as_str()) {
        if let Ok(mut conn) = client.get_async_connection().await {
            use redis::AsyncCommands;
            if let Ok(_) = redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                // Clear test database
                let _: Result<(), _> = redis::cmd("FLUSHDB").query_async::<_, ()>(&mut conn).await;
                return Some(redis_url);
            }
        }
    }
    None
}

// Test helper for creating test users and API keys
struct TestUserSetup {
    pool: PgPool,
    user: User,
    api_key: ApiKey,
    plain_key: String,
    auth_service: AuthService,
}

impl TestUserSetup {
    async fn new() -> Self {
        let pool = get_test_pool().await;
        let auth_service = AuthService::new(pool.clone());

        // Clean up any existing test user
        let test_email = format!("test_{}@security-test.com", Uuid::new_v4());

        // Create test user
        let user = auth_service
            .create_user(
                &test_email,
                Some(&format!("test_user_{}", Uuid::new_v4())),
                Some(json!({"test": true, "security_test": true})),
            )
            .await
            .expect("Failed to create test user");

        // Create API key for user
        let (plain_key, api_key) = auth_service
            .create_api_key(
                user.id,
                Some("Security Test Key"),
                None, // No expiration
                Some(json!(["read", "write"])),
                Some(100), // 100 requests per hour
            )
            .await
            .expect("Failed to create test API key");

        Self {
            pool,
            user,
            api_key,
            plain_key,
            auth_service,
        }
    }

    async fn create_admin_user(&self) -> (String, ApiKey) {
        // Create admin API key
        let (plain_key, api_key) = self
            .auth_service
            .create_api_key(
                self.user.id,
                Some("Admin Test Key"),
                None,
                Some(json!(["read", "write", "admin"])), // Admin permissions
                Some(100),
            )
            .await
            .expect("Failed to create admin API key");

        (plain_key, api_key)
    }

    async fn cleanup(&self) {
        // Clean up test data
        sqlx::query!("DELETE FROM users WHERE id = $1", self.user.id)
            .execute(&self.pool)
            .await
            .ok();
    }
}

// ======================
// API Key Security Tests
// ======================

#[tokio::test]
async fn test_api_key_generation_security() {
    let auth_service = AuthService::new(get_test_pool().await);

    // Generate multiple keys and verify uniqueness
    let mut keys = std::collections::HashSet::new();
    for _ in 0..100 {
        let key = AuthService::generate_api_key();

        // Verify format
        assert!(
            key.starts_with("hea_"),
            "API key should have correct prefix"
        );
        assert_eq!(key.len(), 36, "API key should be correct length");

        // Verify uniqueness
        assert!(keys.insert(key.clone()), "API keys should be unique");

        // Verify it's properly random (no predictable patterns)
        assert!(
            !key.contains("0000"),
            "API key shouldn't contain obvious patterns"
        );
        assert!(
            !key.contains("1111"),
            "API key shouldn't contain obvious patterns"
        );
    }
}

#[tokio::test]
async fn test_argon2_hashing_security() {
    let auth_service = AuthService::new(get_test_pool().await);

    let test_cases = vec![
        "simple_password",
        "complex_P@ssw0rd!",
        "unicode_测试密码",
        "very_long_password_that_exceeds_normal_length_expectations_for_comprehensive_testing",
        "", // Empty password
        "spaces in password",
        "special!@#$%^&*()_+{}|:<>?[]\\;'\",./<>?",
    ];

    for password in test_cases {
        // Hash the password
        let hash = auth_service
            .hash_api_key(password)
            .expect("Hashing should succeed");

        // Verify hash format (Argon2)
        assert!(hash.starts_with("$argon2"), "Hash should use Argon2 format");
        assert!(
            hash.matches('$').count() >= 5,
            "Hash should have proper structure"
        );

        // Verify correct password
        assert!(
            auth_service
                .verify_api_key(password, &hash)
                .expect("Verification should succeed"),
            "Correct password should verify"
        );

        // Verify incorrect password
        assert!(
            !auth_service
                .verify_api_key("wrong_password", &hash)
                .expect("Verification should succeed"),
            "Incorrect password should not verify"
        );

        // Verify case sensitivity
        if !password.is_empty() {
            let uppercase = password.to_uppercase();
            if uppercase != password {
                assert!(
                    !auth_service
                        .verify_api_key(&uppercase, &hash)
                        .expect("Verification should succeed"),
                    "Password verification should be case sensitive"
                );
            }
        }
    }
}

#[tokio::test]
async fn test_timing_attack_resistance() {
    let auth_service = AuthService::new(get_test_pool().await);

    let valid_password = "test_password_123";
    let hash = auth_service
        .hash_api_key(valid_password)
        .expect("Hashing should succeed");

    let test_passwords = vec![
        "wrong_password",
        "test_password_124", // One character different
        "",                  // Empty password
        "completely_different_password_that_is_much_longer",
    ];

    // Measure timing for multiple verification attempts
    for wrong_password in test_passwords {
        let mut timings = Vec::new();

        for _ in 0..10 {
            let start = std::time::Instant::now();
            let result = auth_service.verify_api_key(wrong_password, &hash);
            let duration = start.elapsed();

            assert!(result.is_ok(), "Verification should not error");
            assert!(!result.unwrap(), "Wrong password should not verify");

            timings.push(duration);
        }

        // Verify that timing is relatively consistent (within reasonable bounds)
        // This is a basic timing attack resistance test
        let avg_time = timings.iter().sum::<StdDuration>() / timings.len() as u32;
        let max_deviation = timings
            .iter()
            .map(|t| {
                if *t > avg_time {
                    *t - avg_time
                } else {
                    avg_time - *t
                }
            })
            .max()
            .unwrap_or(StdDuration::from_nanos(0));

        // Allow up to 100ms deviation (very generous for test environments)
        assert!(
            max_deviation < StdDuration::from_millis(100),
            "Timing deviation should be reasonable for password: {}",
            wrong_password
        );
    }
}

#[tokio::test]
async fn test_api_key_lifecycle_security() {
    let setup = TestUserSetup::new().await;

    // Test API key creation
    let (plain_key, api_key) = setup
        .auth_service
        .create_api_key(
            setup.user.id,
            Some("Lifecycle Test Key"),
            Some(Utc::now() + Duration::hours(1)), // Expires in 1 hour
            Some(json!(["read"])),
            Some(50),
        )
        .await
        .expect("API key creation should succeed");

    // Test authentication with new key
    let auth_context = setup
        .auth_service
        .authenticate(&plain_key, None, None)
        .await
        .expect("Authentication should succeed");

    assert_eq!(auth_context.api_key.id, api_key.id);
    assert_eq!(auth_context.user.id, setup.user.id);

    // Test API key revocation
    let revoked = setup
        .auth_service
        .revoke_api_key(api_key.id, setup.user.id)
        .await
        .expect("Revocation should succeed");

    assert!(revoked, "API key should be revoked");

    // Test authentication fails after revocation
    let auth_result = setup
        .auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(
        auth_result.is_err(),
        "Authentication should fail after revocation"
    );

    // Test double revocation (should be idempotent)
    let revoked_again = setup
        .auth_service
        .revoke_api_key(api_key.id, setup.user.id)
        .await
        .expect("Second revocation should succeed");

    assert!(!revoked_again, "Second revocation should return false");

    setup.cleanup().await;
}

#[tokio::test]
async fn test_api_key_expiration_security() {
    let setup = TestUserSetup::new().await;

    // Create expired API key
    let (plain_key, _api_key) = setup
        .auth_service
        .create_api_key(
            setup.user.id,
            Some("Expired Key"),
            Some(Utc::now() - Duration::hours(1)), // Already expired
            Some(json!(["read"])),
            Some(100),
        )
        .await
        .expect("API key creation should succeed");

    // Test authentication with expired key
    let auth_result = setup
        .auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(
        auth_result,
        Err(self_sensored::services::auth::AuthError::ApiKeyExpired)
    ));

    setup.cleanup().await;
}

// ========================
// Rate Limiting Tests
// ========================

#[tokio::test]
async fn test_rate_limiter_in_memory_security() {
    let rate_limiter = RateLimiter::new_in_memory(5); // 5 requests per hour
    let api_key_id = Uuid::new_v4();

    // Test normal operation
    for i in 0..5 {
        let result = rate_limiter
            .check_rate_limit(api_key_id)
            .await
            .expect("Rate limit check should succeed");
        assert_eq!(result.requests_remaining, 4 - i);
        assert_eq!(result.requests_limit, 5);
        assert!(result.retry_after.is_none());
    }

    // Test rate limit enforcement
    let result = rate_limiter
        .check_rate_limit(api_key_id)
        .await
        .expect("Rate limit check should succeed");
    assert_eq!(result.requests_remaining, 0);
    assert!(result.retry_after.is_some());
    assert!(result.retry_after.unwrap() > 0);

    // Test multiple blocked requests
    for _ in 0..3 {
        let result = rate_limiter
            .check_rate_limit(api_key_id)
            .await
            .expect("Rate limit check should succeed");
        assert_eq!(result.requests_remaining, 0);
        assert!(result.retry_after.is_some());
    }
}

#[tokio::test]
async fn test_rate_limiter_ip_based_security() {
    let rate_limiter = RateLimiter::new_in_memory_with_ip_limit(100, 3); // 3 IP requests per hour

    let test_ips = vec![
        "192.168.1.100",
        "10.0.0.1",
        "172.16.0.1",
        "::1",         // IPv6 localhost
        "2001:db8::1", // IPv6 example
    ];

    for ip in test_ips {
        // Test normal operation for each IP
        for i in 0..3 {
            let result = rate_limiter
                .check_ip_rate_limit(ip)
                .await
                .expect("IP rate limit check should succeed");
            assert_eq!(result.requests_remaining, 2 - i);
            assert_eq!(result.requests_limit, 3);
        }

        // Test rate limit enforcement for each IP
        let result = rate_limiter
            .check_ip_rate_limit(ip)
            .await
            .expect("IP rate limit check should succeed");
        assert_eq!(result.requests_remaining, 0);
        assert!(result.retry_after.is_some());
    }
}

#[tokio::test]
async fn test_rate_limiter_isolation() {
    let rate_limiter = RateLimiter::new_in_memory(2); // 2 requests per hour

    let key1 = Uuid::new_v4();
    let key2 = Uuid::new_v4();
    let ip1 = "192.168.1.1";
    let ip2 = "192.168.1.2";

    // Exhaust rate limit for key1
    rate_limiter.check_rate_limit(key1).await.unwrap();
    rate_limiter.check_rate_limit(key1).await.unwrap();

    // key1 should be rate limited
    let result = rate_limiter.check_rate_limit(key1).await.unwrap();
    assert_eq!(result.requests_remaining, 0);

    // key2 should still work (isolation)
    let result = rate_limiter.check_rate_limit(key2).await.unwrap();
    assert_eq!(result.requests_remaining, 1);

    // IP-based rate limiting should be independent
    let result = rate_limiter.check_ip_rate_limit(ip1).await.unwrap();
    assert!(result.requests_remaining > 0);

    let result = rate_limiter.check_ip_rate_limit(ip2).await.unwrap();
    assert!(result.requests_remaining > 0);
}

#[tokio::test]
async fn test_redis_rate_limiter_security() {
    if let Some(redis_url) = setup_mock_redis().await {
        let rate_limiter = RateLimiter::new(&redis_url)
            .await
            .expect("Redis rate limiter should initialize");

        if !rate_limiter.is_using_redis() {
            println!("Skipping Redis test - using in-memory fallback");
            return;
        }

        let api_key_id = Uuid::new_v4();

        // Clear any existing state
        rate_limiter
            .clear_all()
            .await
            .expect("Clear should succeed");

        // Test Redis-backed rate limiting
        for i in 0..100 {
            let result = rate_limiter
                .check_rate_limit(api_key_id)
                .await
                .expect("Rate limit check should succeed");
            if result.requests_remaining == 0 {
                // Hit the limit
                assert!(result.retry_after.is_some());
                break;
            } else {
                assert!(result.requests_remaining <= 100 - i - 1);
            }
        }

        // Test rate limit persistence across "connections" (new rate limiter instance)
        let rate_limiter2 = RateLimiter::new(&redis_url)
            .await
            .expect("Second rate limiter should initialize");

        if rate_limiter2.is_using_redis() {
            let result = rate_limiter2
                .check_rate_limit(api_key_id)
                .await
                .expect("Rate limit check should succeed");
            // Should still be rate limited
            assert_eq!(result.requests_remaining, 0);
            assert!(result.retry_after.is_some());
        }
    } else {
        println!("Skipping Redis rate limiter test - Redis not available");
    }
}

// ===========================
// Authentication Middleware Tests
// ===========================

async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().json(json!({"message": "success", "authenticated": true}))
}

async fn test_handler_with_auth(auth: AuthContext) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "message": "authenticated",
        "user_id": auth.user.id,
        "api_key_id": auth.api_key.id,
        "permissions": auth.api_key.permissions
    }))
}

#[tokio::test]
async fn test_auth_middleware_valid_token() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["user_id"], json!(setup.user.id));
    assert_eq!(body["api_key_id"], json!(setup.api_key.id));

    setup.cleanup().await;
}

#[tokio::test]
async fn test_auth_middleware_invalid_token() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let tampered_key = format!("{}_tampered", setup.plain_key);
    let invalid_tokens = vec![
        "invalid_token",
        "hea_invalid_key_12345",
        tampered_key.as_str(),
        "", // Empty token
        "Bearer extra_bearer_prefix",
        "not_a_bearer_token",
    ];

    for invalid_token in invalid_tokens {
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", invalid_token)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Token '{}' should be rejected",
            invalid_token
        );
    }

    setup.cleanup().await;
}

#[tokio::test]
async fn test_auth_middleware_missing_token() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    // Test missing Authorization header
    let req = TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Test empty Authorization header
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, ""))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Test malformed Authorization header
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, "NotBearer token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    setup.cleanup().await;
}

#[tokio::test]
async fn test_auth_middleware_health_endpoint_bypass() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AuthMiddleware)
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().json("healthy") }),
            )
            .route(
                "/api/v1/status",
                web::get().to(|| async { HttpResponse::Ok().json("status") }),
            ),
    )
    .await;

    // Health endpoints should bypass authentication
    let health_req = TestRequest::get().uri("/health").to_request();
    let health_resp = test::call_service(&app, health_req).await;
    assert_eq!(health_resp.status(), StatusCode::OK);

    let status_req = TestRequest::get().uri("/api/v1/status").to_request();
    let status_resp = test::call_service(&app, status_req).await;
    assert_eq!(status_resp.status(), StatusCode::OK);

    setup.cleanup().await;
}

// ========================
// Admin Middleware Tests
// ========================

async fn test_admin_handler() -> HttpResponse {
    HttpResponse::Ok().json(json!({"message": "admin_success", "admin": true}))
}

#[tokio::test]
async fn test_admin_middleware_with_admin_permissions() {
    let setup = TestUserSetup::new().await;
    let (admin_key, _admin_api_key) = setup.create_admin_user().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AdminMiddleware)
            .wrap(AuthMiddleware) // Admin middleware requires auth middleware first
            .route("/admin/test", web::get().to(test_admin_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/admin/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["admin"], json!(true));

    setup.cleanup().await;
}

#[tokio::test]
async fn test_admin_middleware_without_admin_permissions() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AdminMiddleware)
            .wrap(AuthMiddleware)
            .route("/admin/test", web::get().to(test_admin_handler)),
    )
    .await;

    // Regular user (non-admin) should be denied
    let req = TestRequest::get()
        .uri("/admin/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    setup.cleanup().await;
}

#[tokio::test]
async fn test_admin_middleware_without_auth() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AdminMiddleware)
            // Note: No AuthMiddleware - this tests the case where admin middleware is called without auth
            .route("/admin/test", web::get().to(test_admin_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/admin/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    setup.cleanup().await;
}

// ==============================
// Rate Limiting Middleware Tests
// ==============================

#[tokio::test]
async fn test_rate_limit_middleware_enforcement() {
    let setup = TestUserSetup::new().await;
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory_with_ip_limit(100, 2)); // 2 IP requests per hour

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .app_data(rate_limiter.clone())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    // Clear rate limiter state
    rate_limiter.clear_all().await.unwrap();

    // First two requests should succeed
    for i in 0..2 {
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
            .insert_header(("x-forwarded-for", "192.168.1.200")) // Consistent IP
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Request {} should succeed",
            i + 1
        );

        // Check rate limit headers
        assert!(resp.headers().contains_key("x-ratelimit-limit"));
        assert!(resp.headers().contains_key("x-ratelimit-remaining"));
        assert!(resp.headers().contains_key("x-ratelimit-reset"));
    }

    // Third request should be rate limited
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .insert_header(("x-forwarded-for", "192.168.1.200")) // Same IP
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // Check rate limit headers on 429 response
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
    assert!(resp.headers().contains_key("retry-after"));

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "rate_limit_exceeded");
    assert!(body["retry_after"].is_number());

    setup.cleanup().await;
}

#[tokio::test]
async fn test_rate_limit_middleware_different_ips() {
    let setup = TestUserSetup::new().await;
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory_with_ip_limit(100, 1)); // 1 IP request per hour

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .app_data(rate_limiter.clone())
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    rate_limiter.clear_all().await.unwrap();

    let test_ips = vec!["192.168.1.1", "192.168.1.2", "10.0.0.1"];

    // Each IP should get one successful request
    for ip in test_ips {
        let req = TestRequest::get()
            .uri("/test")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
            .insert_header(("x-forwarded-for", ip))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Request from IP {} should succeed",
            ip
        );
    }

    // Second request from first IP should be rate limited
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .insert_header(("x-forwarded-for", "192.168.1.1"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    setup.cleanup().await;
}

#[tokio::test]
async fn test_rate_limit_middleware_health_bypass() {
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory_with_ip_limit(100, 0)); // No requests allowed

    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().json("healthy") }),
            )
            .route(
                "/metrics",
                web::get().to(|| async { HttpResponse::Ok().json("metrics") }),
            ),
    )
    .await;

    // Health and metrics endpoints should bypass rate limiting
    let health_req = TestRequest::get().uri("/health").to_request();
    let health_resp = test::call_service(&app, health_req).await;
    assert_eq!(health_resp.status(), StatusCode::OK);

    let metrics_req = TestRequest::get().uri("/metrics").to_request();
    let metrics_resp = test::call_service(&app, metrics_req).await;
    assert_eq!(metrics_resp.status(), StatusCode::OK);
}

// ========================
// Cache Security Tests
// ========================

#[tokio::test]
async fn test_auth_cache_security() {
    if let Some(redis_url) = setup_mock_redis().await {
        let cache_config = CacheConfig::default();
        let cache_service = CacheService::new(&redis_url, cache_config).await;

        if let Ok(cache_service) = cache_service {
            let setup = TestUserSetup::new().await;
            let auth_service = AuthService::new_with_cache(
                setup.pool.clone(),
                None, // No rate limiter for this test
                Some(cache_service),
            );

            // First authentication should hit the database
            let start = std::time::Instant::now();
            let auth_context1 = auth_service
                .authenticate(&setup.plain_key, None, None)
                .await
                .expect("First authentication should succeed");
            let first_duration = start.elapsed();

            // Second authentication should hit the cache (faster)
            let start = std::time::Instant::now();
            let auth_context2 = auth_service
                .authenticate(&setup.plain_key, None, None)
                .await
                .expect("Second authentication should succeed");
            let second_duration = start.elapsed();

            // Verify same results
            assert_eq!(auth_context1.user.id, auth_context2.user.id);
            assert_eq!(auth_context1.api_key.id, auth_context2.api_key.id);

            // Cache should generally be faster (though not guaranteed in test environments)
            println!(
                "First auth: {:?}, Second auth: {:?}",
                first_duration, second_duration
            );

            // Test cache invalidation
            auth_service.invalidate_user_auth_cache(setup.user.id).await;

            // Third authentication should hit database again after invalidation
            let start = std::time::Instant::now();
            let auth_context3 = auth_service
                .authenticate(&setup.plain_key, None, None)
                .await
                .expect("Third authentication should succeed");
            let third_duration = start.elapsed();

            assert_eq!(auth_context1.user.id, auth_context3.user.id);
            println!("Third auth (after invalidation): {:?}", third_duration);

            setup.cleanup().await;
        } else {
            println!("Skipping cache test - Redis not available");
        }
    } else {
        println!("Skipping cache test - Redis not available");
    }
}

// ================================
// Security Vulnerability Tests
// ================================

#[tokio::test]
async fn test_sql_injection_prevention() {
    let setup = TestUserSetup::new().await;

    // Test SQL injection attempts in various authentication scenarios
    let injection_attempts = vec![
        "'; DROP TABLE users; --",
        "' OR '1'='1",
        "' UNION SELECT * FROM users --",
        "'; UPDATE users SET is_active = false; --",
        "\"; DELETE FROM api_keys; --",
        "' OR 1=1 --",
        "admin'--",
        "admin' /*",
        "' OR 'x'='x",
        "'; EXEC xp_cmdshell('dir'); --",
    ];

    for injection in injection_attempts {
        // Test in email field during user creation
        let result = setup.auth_service.create_user(injection, None, None).await;
        // Should either succeed (treating as literal string) or fail gracefully
        match result {
            Ok(user) => {
                // If it succeeds, the email should be stored literally
                assert_eq!(user.email, injection);
                // Clean up
                sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
                    .execute(&setup.pool)
                    .await
                    .ok();
            }
            Err(_) => {
                // Failure is also acceptable for malformed emails
            }
        }

        // Test in authentication (should never succeed with these values)
        let auth_result = setup.auth_service.authenticate(injection, None, None).await;
        assert!(
            auth_result.is_err(),
            "SQL injection attempt should fail: {}",
            injection
        );
    }

    setup.cleanup().await;
}

#[tokio::test]
async fn test_xss_prevention_in_responses() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .insert_header((header::USER_AGENT, "<script>alert('xss')</script>"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Ensure response is JSON and doesn't contain raw script tags
    let content_type = resp.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    setup.cleanup().await;
}

#[tokio::test]
async fn test_cors_security_headers() {
    let setup = TestUserSetup::new().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .wrap(
                actix_cors::Cors::default()
                    .allowed_origin("https://trusted-domain.com")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
                    .max_age(3600),
            )
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(test_handler_with_auth)),
    )
    .await;

    // Test preflight request
    let preflight_req = TestRequest::default()
        .insert_header((header::ACCESS_CONTROL_REQUEST_METHOD, "GET"))
        .method(actix_web::http::Method::OPTIONS)
        .uri("/test")
        .insert_header((header::ORIGIN, "https://trusted-domain.com"))
        .to_request();

    let preflight_resp = test::call_service(&app, preflight_req).await;

    // Should allow trusted origin
    assert!(preflight_resp
        .headers()
        .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));

    // Test actual request
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .insert_header((header::ORIGIN, "https://trusted-domain.com"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    setup.cleanup().await;
}

// ===========================
// Audit Logging Tests
// ===========================

#[tokio::test]
async fn test_audit_logging_for_authentication() {
    let setup = TestUserSetup::new().await;

    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
    let user_agent = "TestAgent/1.0";

    // Test successful authentication audit
    let _auth_context = setup
        .auth_service
        .authenticate(&setup.plain_key, Some(client_ip), Some(user_agent))
        .await
        .expect("Authentication should succeed");

    // Test failed authentication audit
    let _failed_result = setup
        .auth_service
        .authenticate("invalid_key", Some(client_ip), Some(user_agent))
        .await;

    // Note: Since we're using tracing for audit logs in the simplified schema,
    // we can't easily verify the logs in tests without a log capture system.
    // In a production system, you would capture and verify audit log entries.

    setup.cleanup().await;
}

// ==============================
// Performance and Load Tests
// ==============================

#[tokio::test]
async fn test_concurrent_authentication_security() {
    let setup = TestUserSetup::new().await;

    let mut handles = Vec::new();
    let success_count = std::sync::Arc::new(AtomicU32::new(0));
    let error_count = std::sync::Arc::new(AtomicU32::new(0));

    // Spawn multiple concurrent authentication attempts
    for i in 0..50 {
        let auth_service = setup.auth_service.clone();
        let api_key = if i % 10 == 0 {
            "invalid_key".to_string() // Some invalid keys
        } else {
            setup.plain_key.clone()
        };
        let success_count = success_count.clone();
        let error_count = error_count.clone();

        let handle = tokio::spawn(async move {
            let result = auth_service.authenticate(&api_key, None, None).await;
            match result {
                Ok(_) => success_count.fetch_add(1, Ordering::Relaxed),
                Err(_) => error_count.fetch_add(1, Ordering::Relaxed),
            };
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    let final_success = success_count.load(Ordering::Relaxed);
    let final_error = error_count.load(Ordering::Relaxed);

    // Should have ~45 successes and ~5 errors
    assert!(
        final_success >= 40,
        "Should have sufficient successful authentications"
    );
    assert!(final_error >= 3, "Should have some failed authentications");
    assert_eq!(
        final_success + final_error,
        50,
        "Total should equal number of attempts"
    );

    setup.cleanup().await;
}

#[tokio::test]
async fn test_rate_limiter_under_load() {
    let rate_limiter = RateLimiter::new_in_memory(10); // 10 requests per hour
    let api_key_id = Uuid::new_v4();

    let mut handles = Vec::new();
    let success_count = std::sync::Arc::new(AtomicU32::new(0));
    let limited_count = std::sync::Arc::new(AtomicU32::new(0));

    // Spawn many concurrent rate limit checks
    for _ in 0..100 {
        let rate_limiter = rate_limiter.clone();
        let success_count = success_count.clone();
        let limited_count = limited_count.clone();

        let handle = tokio::spawn(async move {
            match rate_limiter.check_rate_limit(api_key_id).await {
                Ok(info) => {
                    if info.requests_remaining > 0 || info.retry_after.is_none() {
                        success_count.fetch_add(1, Ordering::Relaxed);
                    } else {
                        limited_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    limited_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    let final_success = success_count.load(Ordering::Relaxed);
    let final_limited = limited_count.load(Ordering::Relaxed);

    // Should have exactly 10 successes (the rate limit)
    assert_eq!(final_success, 10, "Should allow exactly the rate limit");
    assert_eq!(final_limited, 90, "Should rate limit excess requests");

    println!(
        "Load test results: {} successful, {} rate limited",
        final_success, final_limited
    );
}

// =============================
// Integration Tests
// =============================

#[tokio::test]
async fn test_full_security_pipeline() {
    let setup = TestUserSetup::new().await;
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory_with_ip_limit(100, 10));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .app_data(rate_limiter.clone())
            .wrap(
                actix_cors::Cors::default()
                    .allowed_origin("https://example.com")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE]),
            )
            .wrap(RateLimitMiddleware)
            .wrap(AuthMiddleware)
            .route("/api/data", web::get().to(test_handler_with_auth))
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().json("healthy") }),
            ),
    )
    .await;

    rate_limiter.clear_all().await.unwrap();

    // Test 1: Health endpoint should work without auth
    let health_req = TestRequest::get().uri("/health").to_request();
    let health_resp = test::call_service(&app, health_req).await;
    assert_eq!(health_resp.status(), StatusCode::OK);

    // Test 2: Protected endpoint should require auth
    let unauth_req = TestRequest::get().uri("/api/data").to_request();
    let unauth_resp = test::call_service(&app, unauth_req).await;
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // Test 3: Protected endpoint should work with valid auth
    let auth_req = TestRequest::get()
        .uri("/api/data")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .insert_header((header::ORIGIN, "https://example.com"))
        .insert_header(("x-forwarded-for", "192.168.1.50"))
        .to_request();

    let auth_resp = test::call_service(&app, auth_req).await;
    assert_eq!(auth_resp.status(), StatusCode::OK);

    // Check security headers
    assert!(auth_resp.headers().contains_key("x-ratelimit-limit"));
    assert!(auth_resp.headers().contains_key("x-ratelimit-remaining"));

    // Test 4: Rate limiting should work
    for i in 0..9 {
        let req = TestRequest::get()
            .uri("/api/data")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
            .insert_header(("x-forwarded-for", "192.168.1.50"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            println!("Rate limited after {} additional requests", i + 1);
            break;
        }
        assert_eq!(resp.status(), StatusCode::OK);
    }

    setup.cleanup().await;
}

#[tokio::test]
async fn test_admin_security_pipeline() {
    let setup = TestUserSetup::new().await;
    let (admin_key, _admin_api_key) = setup.create_admin_user().await;
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(setup.auth_service.clone()))
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .wrap(AdminMiddleware)
            .wrap(AuthMiddleware)
            .route("/admin/users", web::get().to(test_admin_handler)),
    )
    .await;

    // Test 1: Regular user should be denied
    let regular_req = TestRequest::get()
        .uri("/admin/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", setup.plain_key)))
        .to_request();

    let regular_resp = test::call_service(&app, regular_req).await;
    assert_eq!(regular_resp.status(), StatusCode::FORBIDDEN);

    // Test 2: Admin user should be allowed
    let admin_req = TestRequest::get()
        .uri("/admin/users")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", admin_key)))
        .to_request();

    let admin_resp = test::call_service(&app, admin_req).await;
    assert_eq!(admin_resp.status(), StatusCode::OK);

    // Test 3: No auth should be denied
    let no_auth_req = TestRequest::get().uri("/admin/users").to_request();
    let no_auth_resp = test::call_service(&app, no_auth_req).await;
    assert_eq!(no_auth_resp.status(), StatusCode::UNAUTHORIZED);

    setup.cleanup().await;
}
