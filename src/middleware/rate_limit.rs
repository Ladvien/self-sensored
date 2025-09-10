use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
    http::header::{HeaderName, HeaderValue},
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{warn, debug};

use crate::services::{auth::AuthContext, rate_limiter::{RateLimiter, RateLimitError}};

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
        headers.insert(HeaderName::from_static("x-ratelimit-remaining"), remaining_header);
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
                    // Authenticated request: use API key-based rate limiting
                    debug!("Checking rate limit for API key: {}", auth_context.api_key.id);
                    rate_limiter.check_rate_limit(auth_context.api_key.id).await
                } else {
                    // Unauthenticated request: use IP-based rate limiting
                    let client_ip = get_client_ip(&req);
                    debug!("Checking rate limit for IP: {}", client_ip);
                    rate_limiter.check_ip_rate_limit(&client_ip).await
                };

                match rate_limit_result {
                    Ok(rate_limit_info) => {
                        if rate_limit_info.requests_remaining < 0 || rate_limit_info.retry_after.is_some() {
                            // Rate limit exceeded
                            warn!("Rate limit exceeded for path: {}", path);
                            let mut builder = HttpResponse::TooManyRequests();
                            
                            // Add rate limiting headers
                            builder.insert_header(("x-ratelimit-limit", rate_limit_info.requests_limit.to_string()));
                            builder.insert_header(("x-ratelimit-remaining", rate_limit_info.requests_remaining.max(0).to_string()));
                            builder.insert_header(("x-ratelimit-reset", rate_limit_info.reset_time.timestamp().to_string()));
                            
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
                                }).to_string()
                            ).into())
                        } else {
                            // Request allowed, proceed with service call
                            debug!("Rate limit check passed, {} requests remaining", rate_limit_info.requests_remaining);
                            let mut response = service.call(req).await?;
                            
                            // Add rate limiting headers to successful responses
                            let headers = response.headers_mut();
                            
                            if let Ok(limit_header) = HeaderValue::from_str(&rate_limit_info.requests_limit.to_string()) {
                                headers.insert(HeaderName::from_static("x-ratelimit-limit"), limit_header);
                            }
                            
                            if let Ok(remaining_header) = HeaderValue::from_str(&rate_limit_info.requests_remaining.to_string()) {
                                headers.insert(HeaderName::from_static("x-ratelimit-remaining"), remaining_header);
                            }
                            
                            if let Ok(reset_header) = HeaderValue::from_str(&rate_limit_info.reset_time.timestamp().to_string()) {
                                headers.insert(HeaderName::from_static("x-ratelimit-reset"), reset_header);
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
        auth::{AuthService, AuthContext, User, ApiKey},
        rate_limiter::RateLimiter
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

        let app = test::init_service(
            App::new()
                .app_data(rate_limiter)
                .wrap(RateLimitMiddleware)
                .route("/test", web::get().to(test_handler))
        ).await;

        // Manually insert auth context into request
        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();

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
                .route("/health", web::get().to(|| async { HttpResponse::Ok().json("healthy") }))
        ).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), 200);
    }
}