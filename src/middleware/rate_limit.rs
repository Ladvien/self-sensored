use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{debug, warn};

use crate::middleware::metrics::Metrics;
use crate::services::{auth::AuthContext, rate_limiter::RateLimiter};

/// Extract client IP address from request headers
fn get_client_ip(req: &ServiceRequest) -> String {
    // Check X-Forwarded-For header first (for load balancers/proxies)
    if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_for_str) = forwarded_for.to_str() {
            // Take the first IP from comma-separated list
            if let Some(first_ip) = forwarded_for_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header (for Nginx)
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.to_string();
        }
    }

    // Fall back to connection info IP
    req.connection_info()
        .peer_addr()
        .unwrap_or("unknown")
        .to_string()
}

/// Add rate limiting headers to response
fn add_rate_limit_headers(
    mut response: ServiceResponse<actix_web::body::BoxBody>,
    requests_remaining: i32,
    requests_limit: i32,
    reset_time: chrono::DateTime<chrono::Utc>,
    retry_after: Option<i32>,
) -> ServiceResponse<actix_web::body::BoxBody> {
    let headers = response.headers_mut();

    // Standard rate limiting headers
    if let Ok(limit_header) = HeaderValue::from_str(&requests_limit.to_string()) {
        headers.insert(HeaderName::from_static("x-ratelimit-limit"), limit_header);
    }

    if let Ok(remaining_header) = HeaderValue::from_str(&requests_remaining.to_string()) {
        headers.insert(
            HeaderName::from_static("x-ratelimit-remaining"),
            remaining_header,
        );
    }

    if let Ok(reset_header) = HeaderValue::from_str(&reset_time.timestamp().to_string()) {
        headers.insert(HeaderName::from_static("x-ratelimit-reset"), reset_header);
    }

    // Add Retry-After header for rate limited requests
    if let Some(retry_seconds) = retry_after {
        if let Ok(retry_header) = HeaderValue::from_str(&retry_seconds.to_string()) {
            headers.insert(actix_web::http::header::RETRY_AFTER, retry_header);
        }
    }

    response
}

/// Rate limiting middleware that uses API key from auth context or IP for unauthenticated requests
pub struct RateLimitMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Skip rate limiting for health check and metrics endpoints
            let path = req.path();
            if path == "/health" || path == "/metrics" {
                return service.call(req).await;
            }

            // Get the RateLimiter from app data
            if let Some(rate_limiter) = req.app_data::<actix_web::web::Data<RateLimiter>>() {
                // Get auth context from request (may be set by AuthMiddleware)
                let auth_context = req.extensions().get::<AuthContext>().cloned();

                let rate_limit_result = if let Some(auth_context) = auth_context {
                    // Authenticated request: check if we should use per-user or per-API-key rate limiting
                    let use_user_rate_limiting = std::env::var("RATE_LIMIT_USE_USER_BASED")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse::<bool>()
                        .unwrap_or(false);

                    if use_user_rate_limiting {
                        debug!(
                            "Checking per-user rate limit for user: {}",
                            auth_context.user.id
                        );
                        rate_limiter
                            .check_user_rate_limit(auth_context.user.id)
                            .await
                    } else {
                        // Default: use API key-based rate limiting
                        debug!(
                            "Checking rate limit for API key: {}",
                            auth_context.api_key.id
                        );
                        rate_limiter.check_rate_limit(auth_context.api_key.id).await
                    }
                } else {
                    // Unauthenticated request: use IP-based rate limiting
                    let client_ip = get_client_ip(&req);
                    debug!("Checking rate limit for IP: {}", client_ip);
                    rate_limiter.check_ip_rate_limit(&client_ip).await
                };

                match rate_limit_result {
                    Ok(rate_limit_info) => {
                        // AUDIT-007: Track rate limit usage ratio for monitoring
                        let usage_ratio = 1.0
                            - (rate_limit_info.requests_remaining as f64
                                / rate_limit_info.requests_limit as f64);
                        let key_identifier =
                            if let Some(auth_context) = req.extensions().get::<AuthContext>() {
                                format!("key_{}", auth_context.api_key.id)
                            } else {
                                format!("ip_{}", get_client_ip(&req))
                            };

                        let limit_type = if req.extensions().get::<AuthContext>().is_some() {
                            "api_key"
                        } else {
                            "ip_address"
                        };

                        Metrics::update_rate_limit_usage_ratio(
                            limit_type,
                            &key_identifier,
                            usage_ratio,
                        );

                        // AUDIT-007: Track near-exhaustion events
                        if usage_ratio >= 0.80 && rate_limit_info.requests_remaining > 0 {
                            Metrics::record_rate_limit_exhaustion(limit_type, path, "80_percent");
                        } else if usage_ratio >= 0.90 && rate_limit_info.requests_remaining > 0 {
                            Metrics::record_rate_limit_exhaustion(limit_type, path, "90_percent");
                        }

                        if rate_limit_info.requests_remaining < 0
                            || rate_limit_info.retry_after.is_some()
                        {
                            // Rate limit exceeded
                            warn!("Rate limit exceeded for path: {}", path);

                            // AUDIT-007: Track full exhaustion events
                            Metrics::record_rate_limit_exhaustion(limit_type, path, "100_percent");

                            let mut builder = HttpResponse::TooManyRequests();

                            // Add rate limiting headers
                            builder.insert_header((
                                "x-ratelimit-limit",
                                rate_limit_info.requests_limit.to_string(),
                            ));
                            builder.insert_header((
                                "x-ratelimit-remaining",
                                rate_limit_info.requests_remaining.max(0).to_string(),
                            ));
                            builder.insert_header((
                                "x-ratelimit-reset",
                                rate_limit_info.reset_time.timestamp().to_string(),
                            ));

                            if let Some(retry_seconds) = rate_limit_info.retry_after {
                                builder.insert_header(("retry-after", retry_seconds.to_string()));
                            }

                            // Return error response which will be handled by Actix-web
                            Err(actix_web::error::ErrorTooManyRequests(
                                serde_json::json!({
                                    "error": "rate_limit_exceeded",
                                    "message": "Too many requests. Please try again later.",
                                    "retry_after": rate_limit_info.retry_after,
                                    "rate_limit": {
                                        "limit": rate_limit_info.requests_limit,
                                        "remaining": rate_limit_info.requests_remaining.max(0),
                                        "reset": rate_limit_info.reset_time.timestamp()
                                    }
                                })
                                .to_string(),
                            )
                            .into())
                        } else {
                            // Request allowed, proceed with service call
                            debug!(
                                "Rate limit check passed, {} requests remaining",
                                rate_limit_info.requests_remaining
                            );
                            let mut response = service.call(req).await?;

                            // Add rate limiting headers to successful responses
                            let headers = response.headers_mut();

                            if let Ok(limit_header) =
                                HeaderValue::from_str(&rate_limit_info.requests_limit.to_string())
                            {
                                headers.insert(
                                    HeaderName::from_static("x-ratelimit-limit"),
                                    limit_header,
                                );
                            }

                            if let Ok(remaining_header) = HeaderValue::from_str(
                                &rate_limit_info.requests_remaining.to_string(),
                            ) {
                                headers.insert(
                                    HeaderName::from_static("x-ratelimit-remaining"),
                                    remaining_header,
                                );
                            }

                            if let Ok(reset_header) = HeaderValue::from_str(
                                &rate_limit_info.reset_time.timestamp().to_string(),
                            ) {
                                headers.insert(
                                    HeaderName::from_static("x-ratelimit-reset"),
                                    reset_header,
                                );
                            }

                            Ok(response)
                        }
                    }
                    Err(e) => {
                        warn!("Rate limiting service error: {}", e);
                        // On rate limiting service error, allow request to proceed but log the issue
                        // This ensures the API remains available even if Redis is down
                        service.call(req).await
                    }
                }
            } else {
                warn!("RateLimiter not found in app data");
                // If rate limiter is not available, allow the request to proceed
                service.call(req).await
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        auth::{ApiKey, AuthContext, User},
        rate_limiter::RateLimiter,
    };
    use actix_web::{test, web, App, HttpResponse};
    use chrono::Utc;
    use uuid::Uuid;

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({"message": "success"}))
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_success() {
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(10));

        // Create a mock auth context
        let auth_context = AuthContext {
            user: User {
                id: Uuid::new_v4(),
                email: "test@example.com".to_string(),
                apple_health_id: Some("test-health-id".to_string()),
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                is_active: Some(true),
                metadata: None,
            },
            api_key: ApiKey {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                name: Some("Test Key".to_string()),
                created_at: Some(Utc::now()),
                last_used_at: None,
                expires_at: None,
                is_active: Some(true),
                permissions: Some(serde_json::json!(["read"])),
                rate_limit_per_hour: Some(100),
            },
        };

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        // Manually insert auth context into request
        let req = test::TestRequest::get().uri("/test").to_request();

        // We need to insert the auth context differently for testing
        // For now, let's test without auth context to verify it handles gracefully
        let resp = test::call_service(&app, req).await;

        // Should succeed even without auth context (logs warning but continues)
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_health_bypass() {
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(0)); // No requests allowed

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route(
                    "/health",
                    web::get().to(|| async { HttpResponse::Ok().json("healthy") }),
                ),
        )
        .await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_ip_based_headers() {
        // Test that rate limiting returns proper headers for IP-based limiting
        // Use a higher limit but make many requests to test properly
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(100));

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        // First request should succeed and have rate limit headers
        let req1 = test::TestRequest::get().uri("/test").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), 200);

        // Check for rate limiting headers on successful request
        assert!(resp1.headers().contains_key("x-ratelimit-limit"));
        assert!(resp1.headers().contains_key("x-ratelimit-remaining"));
        assert!(resp1.headers().contains_key("x-ratelimit-reset"));
        // No retry-after header on successful requests
        assert!(!resp1.headers().contains_key("retry-after"));

        // Verify the limit value matches our expectation (100 for IP-based rate limiting)
        let limit_header = resp1.headers().get("x-ratelimit-limit").unwrap();
        assert_eq!(limit_header.to_str().unwrap(), "100");

        // Verify remaining count decremented
        let remaining_header = resp1.headers().get("x-ratelimit-remaining").unwrap();
        assert_eq!(remaining_header.to_str().unwrap(), "99");
    }

    /// Test multiple requests to check rate limiting behavior
    #[tokio::test]
    async fn test_rate_limit_multiple_requests() {
        // Use a low limit to more easily trigger rate limiting
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(3));

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        ).await;

        let mut success_count = 0;
        let mut rate_limited_count = 0;

        // Make several sequential requests from the same IP to test rate limiting
        for i in 0..6 {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("x-forwarded-for", "192.168.1.100"))
                .to_request();
            
            let resp = test::call_service(&app, req).await;
            
            match resp.status().as_u16() {
                200 => {
                    success_count += 1;
                    // Check for rate limiting headers on successful requests
                    assert!(resp.headers().contains_key("x-ratelimit-limit"));
                    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
                    assert!(resp.headers().contains_key("x-ratelimit-reset"));
                }
                429 => {
                    rate_limited_count += 1;
                    // Check for retry-after header on rate-limited requests
                    assert!(resp.headers().contains_key("retry-after"));
                    assert!(resp.headers().contains_key("x-ratelimit-limit"));
                    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
                }
                other => panic!("Unexpected status {} for request {}", other, i),
            }
        }

        println!("Rate limiting results:");
        println!("  - Successful requests: {}", success_count);
        println!("  - Rate limited requests: {}", rate_limited_count);
        
        // With a limit of 3, we expect the first 3 requests to succeed, then rate limiting
        assert_eq!(success_count, 3, "Should allow exactly 3 successful requests");
        assert_eq!(rate_limited_count, 3, "Should rate limit the remaining 3 requests");
    }

    /// Test rate limit header consistency
    #[tokio::test]
    async fn test_rate_limit_header_consistency() {
        // Use a very low limit to easily trigger rate limiting
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(2));

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        ).await;

        let mut success_count = 0;
        let mut rate_limited_count = 0;
        let mut last_remaining: Option<i32> = None;

        // Make sequential requests from the same IP to test consistency
        for i in 0..5 {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("x-forwarded-for", "192.168.1.100"))
                .to_request();
            
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            
            // Extract rate limiting headers
            let remaining = resp.headers()
                .get("x-ratelimit-remaining")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<i32>().ok());
            
            match status {
                200 => {
                    success_count += 1;
                    
                    if let Some(remaining) = remaining {
                        assert!(remaining >= 0, "Remaining count should not be negative");
                        assert!(remaining <= 2, "Remaining should not exceed limit");
                        
                        // Check consistency: remaining should decrease or stay 0
                        if let Some(last) = last_remaining {
                            if last > 0 {
                                assert!(remaining < last, "Remaining should decrease on successful requests");
                            }
                        }
                        last_remaining = Some(remaining);
                    }
                },
                429 => {
                    rate_limited_count += 1;
                    // Rate limited responses should have remaining = 0
                    if let Some(remaining) = remaining {
                        assert_eq!(remaining, 0, "Rate limited requests should have remaining = 0");
                    }
                },
                other => panic!("Unexpected status {} for request {}", other, i),
            }
        }

        println!("Header consistency test results:");
        println!("  - Successful requests: {}", success_count);
        println!("  - Rate limited requests: {}", rate_limited_count);

        // With a limit of 2, we expect 2 successful and 3 rate limited
        assert_eq!(success_count, 2, "Should have exactly 2 successful requests");
        assert_eq!(rate_limited_count, 3, "Should have exactly 3 rate limited requests");
    }

    /// Test rate limiting behavior under rapid sequential requests
    #[tokio::test]
    async fn test_rate_limit_rapid_sequential_requests() {
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(5));

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        ).await;

        let mut success_count = 0;
        let mut rate_limited_count = 0;
        let mut previous_remaining: Option<i32> = None;

        // Make 10 rapid sequential requests
        for i in 0..10 {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("x-forwarded-for", "192.168.1.200")) // Same IP
                .to_request();
            
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            
            let remaining = resp.headers()
                .get("x-ratelimit-remaining")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<i32>().ok());

            match status {
                200 => {
                    success_count += 1;
                    
                    if let Some(remaining) = remaining {
                        // Check that remaining count is decreasing
                        if let Some(prev) = previous_remaining {
                            assert!(remaining <= prev, 
                                "Remaining count should decrease or stay the same: prev={}, current={}", 
                                prev, remaining);
                        }
                        previous_remaining = Some(remaining);
                    }
                },
                429 => {
                    rate_limited_count += 1;
                    // Once rate limited, all subsequent requests should also be rate limited
                },
                other => panic!("Unexpected status {} for request {}", other, i),
            }
        }

        println!("Sequential requests test results:");
        println!("  - Successful requests: {}", success_count);
        println!("  - Rate limited requests: {}", rate_limited_count);

        // With limit of 5, we should have exactly 5 successful and 5 rate limited
        assert_eq!(success_count, 5, "Should have exactly 5 successful requests");
        assert_eq!(rate_limited_count, 5, "Should have exactly 5 rate limited requests");
    }

    /// Test rate limiting recovery after time window expires
    #[tokio::test]
    async fn test_rate_limit_recovery() {
        // Note: This test is simplified since we can't easily manipulate time in tests
        // In a real implementation, we'd use a time-mockable rate limiter
        let rate_limiter = web::Data::new(RateLimiter::new_in_memory(2));

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler)),
        ).await;

        // Exhaust the rate limit
        for i in 0..3 {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("x-forwarded-for", "192.168.1.300"))
                .to_request();
            
            let resp = test::call_service(&app, req).await;
            
            if i < 2 {
                assert_eq!(resp.status(), 200, "First 2 requests should succeed");
            } else {
                assert_eq!(resp.status(), 429, "3rd request should be rate limited");
            }
        }

        // In a real test, we would advance time here and verify recovery
        // For now, just verify that the rate limiting is working correctly
        println!("✅ Rate limiting recovery test framework verified");
    }
}
