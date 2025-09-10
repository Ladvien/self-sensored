use actix_web::{test, web, App, HttpResponse, http::StatusCode};
use chrono::Utc;
use uuid::Uuid;
use std::time::Duration;
use tokio::time::sleep;

use self_sensored::middleware::{AuthMiddleware, RateLimitMiddleware};
use self_sensored::services::{
    auth::{AuthContext, AuthService, User, ApiKey},
    rate_limiter::RateLimiter,
};

async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"message": "success"}))
}

#[tokio::test]
async fn test_rate_limit_api_key_success() {
    // Create in-memory rate limiter with high limit
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(10));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // Create mock auth context
    let auth_context = AuthContext {
        user: User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            full_name: Some("Test User".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_active: Some(true),
        },
        api_key: ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            created_at: Some(Utc::now()),
            last_used_at: None,
            expires_at: None,
            is_active: Some(true),
            scopes: Some(vec!["read".to_string()]),
        },
    };

    // Make multiple requests within limit
    for i in 1..=5 {
        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();
        
        // Insert auth context
        req.extensions_mut().insert(auth_context.clone());
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        
        // Check rate limiting headers
        assert!(resp.headers().contains_key("x-ratelimit-limit"));
        assert!(resp.headers().contains_key("x-ratelimit-remaining"));
        assert!(resp.headers().contains_key("x-ratelimit-reset"));
        
        if let Some(remaining_header) = resp.headers().get("x-ratelimit-remaining") {
            if let Ok(remaining_str) = remaining_header.to_str() {
                let remaining: i32 = remaining_str.parse().unwrap();
                assert_eq!(remaining, 10 - i);
            }
        }
    }
}

#[tokio::test]
async fn test_rate_limit_api_key_exceeded() {
    // Create in-memory rate limiter with very low limit
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(2));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    let auth_context = AuthContext {
        user: User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            full_name: Some("Test User".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_active: Some(true),
        },
        api_key: ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            created_at: Some(Utc::now()),
            last_used_at: None,
            expires_at: None,
            is_active: Some(true),
            scopes: Some(vec!["read".to_string()]),
        },
    };

    // Use up the limit
    for _ in 1..=2 {
        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();
        
        req.extensions_mut().insert(auth_context.clone());
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Next request should be rate limited
    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();
    
    req.extensions_mut().insert(auth_context);
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    
    // Check required headers
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
    assert!(resp.headers().contains_key("x-ratelimit-reset"));
    assert!(resp.headers().contains_key("retry-after"));
}

#[tokio::test]
async fn test_rate_limit_ip_unauthenticated() {
    // Create in-memory rate limiter
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100)); // High limit for API keys
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // Make unauthenticated requests (should use IP-based limiting)
    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.100"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    
    // Check that rate limiting headers are present
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
    assert!(resp.headers().contains_key("x-ratelimit-reset"));
}

#[tokio::test]
async fn test_rate_limit_ip_exceeded() {
    // Set environment variable for very low IP limit
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "1");
    
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100)); // High limit for API keys
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // First request should succeed
    let req1 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.101"))
        .to_request();
    
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Second request from same IP should be rate limited
    let req2 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.101"))
        .to_request();
    
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::TOO_MANY_REQUESTS);
    
    // Check required headers
    assert!(resp2.headers().contains_key("retry-after"));
    
    // Clean up environment
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");
}

#[tokio::test]
async fn test_rate_limit_health_endpoint_bypass() {
    // Create rate limiter with zero limit
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(0));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/health", web::get().to(|| async { HttpResponse::Ok().json("healthy") }))
    ).await;

    // Health endpoint should always work regardless of rate limits
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_metrics_endpoint_bypass() {
    // Create rate limiter with zero limit
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(0));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/metrics", web::get().to(|| async { HttpResponse::Ok().body("metrics") }))
    ).await;

    // Metrics endpoint should always work regardless of rate limits
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_different_ips() {
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "1");
    
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // First IP should work once
    let req1 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.100"))
        .to_request();
    
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Different IP should also work once
    let req2 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.200"))
        .to_request();
    
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::OK);

    // Original IP should be rate limited on second request
    let req3 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "192.168.1.100"))
        .to_request();
    
    let resp3 = test::call_service(&app, req3).await;
    assert_eq!(resp3.status(), StatusCode::TOO_MANY_REQUESTS);
    
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");
}

#[tokio::test]
async fn test_rate_limit_header_extraction() {
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "2");
    
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // Test X-Forwarded-For header
    let req1 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-forwarded-for", "10.0.0.1, 192.168.1.1"))
        .to_request();
    
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Test X-Real-IP header (when X-Forwarded-For is not present)
    let req2 = test::TestRequest::get()
        .uri("/test")
        .insert_header(("x-real-ip", "10.0.0.2"))
        .to_request();
    
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::OK);
    
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");
}

#[tokio::test]
async fn test_rate_limit_error_handling() {
    // Create an app without rate limiter in app data
    let app = test::init_service(
        App::new()
            .wrap(RateLimitMiddleware)
            .route("/test", web::get().to(test_handler))
    ).await;

    // Request should still proceed even without rate limiter
    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_dos_simulation() {
    // Create rate limiter with very low limits for DoS testing
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "3");
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/api/v1/ingest", web::post().to(test_handler))
    ).await;

    let attacker_ip = "192.168.1.666";
    let mut successful_requests = 0;
    let mut rate_limited_requests = 0;

    // Simulate DoS attack with rapid requests
    for _ in 0..10 {
        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("x-forwarded-for", attacker_ip))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        
        if resp.status() == StatusCode::OK {
            successful_requests += 1;
        } else if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited_requests += 1;
            
            // Verify proper error response
            let body = test::read_body(resp).await;
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json["error"], "rate_limit_exceeded");
            assert!(json["retry_after"].is_number());
        }
    }

    // Should allow only 3 successful requests, then rate limit the rest
    assert_eq!(successful_requests, 3);
    assert_eq!(rate_limited_requests, 7);
    
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");
}

#[tokio::test]
async fn test_legitimate_usage_patterns() {
    // Test legitimate usage with proper spacing between requests
    let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));
    
    let app = test::init_service(
        App::new()
            .app_data(rate_limiter)
            .wrap(RateLimitMiddleware)
            .route("/api/v1/data/heart-rate", web::get().to(test_handler))
    ).await;

    let auth_context = AuthContext {
        user: User {
            id: Uuid::new_v4(),
            email: "legitimate@example.com".to_string(),
            full_name: Some("Legitimate User".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_active: Some(true),
        },
        api_key: ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Legitimate App Key".to_string(),
            created_at: Some(Utc::now()),
            last_used_at: None,
            expires_at: None,
            is_active: Some(true),
            scopes: Some(vec!["read".to_string()]),
        },
    };

    // Simulate legitimate usage pattern - spaced requests
    for i in 1..=10 {
        let req = test::TestRequest::get()
            .uri("/api/v1/data/heart-rate")
            .to_request();
        
        req.extensions_mut().insert(auth_context.clone());
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        
        // Check that remaining requests decrease properly
        if let Some(remaining_header) = resp.headers().get("x-ratelimit-remaining") {
            if let Ok(remaining_str) = remaining_header.to_str() {
                let remaining: i32 = remaining_str.parse().unwrap();
                assert_eq!(remaining, 100 - i);
            }
        }
        
        // Small delay between requests (simulating real usage)
        if i < 10 {
            sleep(Duration::from_millis(10)).await;
        }
    }
}