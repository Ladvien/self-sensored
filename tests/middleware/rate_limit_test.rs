use actix_web::{test, web, App, HttpResponse, Result};
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

use self_sensored::{
    middleware::{auth::AuthMiddleware, rate_limit::RateLimitMiddleware},
    services::auth::AuthService,
};

async fn get_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

fn get_test_redis_client() -> RedisClient {
    dotenv::dotenv().ok();
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    RedisClient::open(redis_url).expect("Failed to create Redis client")
}

async fn test_handler() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "ok"})))
}

async fn setup_test_user_and_key(pool: &PgPool, email: &str) -> (uuid::Uuid, String) {
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user(email, Some("Rate Limit Test"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Rate Limit Test Key", None, vec!["write".to_string()])
        .await
        .unwrap();

    (user.id, plain_key)
}

#[tokio::test]
async fn test_rate_limit_allows_requests_under_limit() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "under_limit_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 10, Duration::from_secs(60))) // 10 requests per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Make several requests under the limit
    for i in 0..5 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Request {} should succeed", i);

        // Check rate limit headers
        let headers = resp.headers();
        assert!(headers.contains_key("x-ratelimit-limit"));
        assert!(headers.contains_key("x-ratelimit-remaining"));
        assert!(headers.contains_key("x-ratelimit-reset"));
    }

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_blocks_requests_over_limit() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "over_limit_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 3, Duration::from_secs(60))) // 3 requests per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Make requests up to the limit
    for i in 0..3 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Request {} should succeed", i);
    }

    // The next request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429, "Request should be rate limited");

    // Check rate limit headers
    let headers = resp.headers();
    assert!(headers.contains_key("x-ratelimit-limit"));
    assert!(headers.contains_key("x-ratelimit-remaining"));
    assert!(headers.contains_key("x-ratelimit-reset"));
    assert_eq!(headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap(), "0");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_per_api_key_isolation() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();

    // Create two different users and API keys
    let (user1_id, api_key1) = setup_test_user_and_key(&pool, "isolation_test1@example.com").await;
    let (user2_id, api_key2) = setup_test_user_and_key(&pool, "isolation_test2@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 2, Duration::from_secs(60))) // 2 requests per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Use up the rate limit for API key 1
    for i in 0..2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", api_key1)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "User1 request {} should succeed", i);
    }

    // API key 1 should be rate limited now
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key1)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429, "User1 should be rate limited");

    // But API key 2 should still work
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key2)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "User2 should not be affected by User1's rate limit");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user1_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("DELETE FROM users WHERE id = $1", user2_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_window_reset() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "window_reset_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 2, Duration::from_secs(2))) // 2 requests per 2 seconds
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Use up the rate limit
    for i in 0..2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success(), "Request {} should succeed", i);
    }

    // Next request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 429, "Request should be rate limited");

    // Wait for window to reset
    sleep(Duration::from_secs(3)).await;

    // Should work again after window reset
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Request should succeed after window reset");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_with_invalid_api_key() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 10, Duration::from_secs(60)))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Request with invalid API key should fail at auth, not rate limiting
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", "Bearer invalid_key"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Should fail authentication, not rate limiting");

    // Rate limit headers should not be present for unauthenticated requests
    let headers = resp.headers();
    assert!(!headers.contains_key("x-ratelimit-limit"));
}

#[tokio::test]
async fn test_rate_limit_headers() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "headers_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 5, Duration::from_secs(60))) // 5 requests per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // First request
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let headers = resp.headers();
    
    // Check all required headers are present
    assert!(headers.contains_key("x-ratelimit-limit"));
    assert!(headers.contains_key("x-ratelimit-remaining"));
    assert!(headers.contains_key("x-ratelimit-reset"));

    // Check values
    let limit = headers.get("x-ratelimit-limit").unwrap().to_str().unwrap();
    let remaining = headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap();
    
    assert_eq!(limit, "5");
    assert_eq!(remaining, "4"); // Started with 5, used 1

    // Second request
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();
    let remaining = headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap();
    assert_eq!(remaining, "3"); // Now used 2

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_concurrent_rate_limiting() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "concurrent_rate_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 5, Duration::from_secs(60))) // 5 requests per minute
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Run 10 concurrent requests (should only allow 5)
    let mut handles = vec![];
    
    for i in 0..10 {
        let app_clone = app.clone();
        let key_clone = api_key.clone();
        
        let handle = tokio::spawn(async move {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("Authorization", format!("Bearer {}", key_clone)))
                .to_request();

            let resp = test::call_service(&app_clone, req).await;
            (i, resp.status().as_u16())
        });
        
        handles.push(handle);
    }

    // Collect results
    let mut success_count = 0;
    let mut rate_limited_count = 0;
    
    for handle in handles {
        let (i, status) = handle.await.unwrap();
        match status {
            200 => success_count += 1,
            429 => rate_limited_count += 1,
            other => panic!("Unexpected status {} for request {}", other, i),
        }
    }

    // Should have exactly 5 successes and 5 rate limited
    assert_eq!(success_count, 5, "Should have exactly 5 successful requests");
    assert_eq!(rate_limited_count, 5, "Should have exactly 5 rate limited requests");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test] 
async fn test_bandwidth_limiting() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "bandwidth_test@example.com").await;

    // This test would require implementing bandwidth limiting in addition to request rate limiting
    // For now, we'll test with large payloads to ensure the system handles them properly
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 10, Duration::from_secs(60)))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::post().to(test_handler)),
    )
    .await;

    // Large payload (1MB)
    let large_payload = "x".repeat(1024 * 1024);

    let req = test::TestRequest::post()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(large_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    
    // Should succeed but count towards any bandwidth limits
    // (Actual bandwidth limiting would be implemented in the rate limiter)
    assert!(resp.status().is_success());

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_rate_limit_performance() {
    let pool = get_test_pool().await;
    let redis_client = get_test_redis_client();
    let (user_id, api_key) = setup_test_user_and_key(&pool, "perf_rate_test@example.com").await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(RateLimitMiddleware::new(redis_client.clone(), 1000, Duration::from_secs(60))) // High limit to avoid blocking
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let start = std::time::Instant::now();
    
    // Make 100 requests to test rate limiting performance
    for _ in 0..100 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
    
    let duration = start.elapsed();
    let avg_per_request = duration.as_millis() / 100;

    // Performance requirement: rate limiting should add < 5ms overhead per request
    assert!(avg_per_request < 15, // Including auth overhead
           "Rate limiting took {}ms per request, expected < 15ms", avg_per_request);

    println!("✓ Rate limiting performance: {}ms average per request", avg_per_request);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .unwrap();
}