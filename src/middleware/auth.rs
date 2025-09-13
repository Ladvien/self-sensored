use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse, Result,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use crate::services::auth::{AuthContext, AuthService};

/// Authentication middleware that validates Bearer tokens
pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Skip auth for health check endpoints
            if req.path() == "/health" || req.path() == "/api/v1/status" {
                return service.call(req).await.map(|res| {
                    res.map_into_left_body()
                });
            }

            // Extract client IP and user agent for audit logging
            let client_ip_str = req
                .connection_info()
                .peer_addr()
                .unwrap_or("unknown")
                .to_string();

            let client_ip = client_ip_str.parse::<std::net::IpAddr>().ok();
            let user_agent = req
                .headers()
                .get("user-agent")
                .and_then(|h| h.to_str().ok());

            // Log all request details for debugging
            tracing::debug!(
                "Auth middleware processing request: method={} path={} client_ip={} user_agent={:?} content_type={:?}",
                req.method(),
                req.path(),
                client_ip_str,
                user_agent,
                req.headers().get("content-type")
            );

            // Get the Authorization header
            let auth_header = req.headers().get("Authorization");

            tracing::debug!("Authorization header present: {}", auth_header.is_some());

            if let Some(auth_value) = auth_header {
                if let Ok(auth_str) = auth_value.to_str() {
                    tracing::debug!("Authorization header value length: {}", auth_str.len());
                    tracing::debug!(
                        "Authorization header starts with 'Bearer ': {}",
                        auth_str.starts_with("Bearer ")
                    );

                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        // Remove "Bearer " prefix

                        tracing::debug!("Extracted API key: {}...", &token[..token.len().min(10)]);

                        // Get the AuthService from app data
                        if let Some(auth_service) =
                            req.app_data::<actix_web::web::Data<AuthService>>()
                        {
                            match auth_service
                                .authenticate(token, client_ip, user_agent)
                                .await
                            {
                                Ok(auth_context) => {
                                    tracing::info!(
                                        "Authentication successful: user_id={} api_key_id={} client_ip={}",
                                        auth_context.user.id,
                                        auth_context.api_key.id,
                                        client_ip_str
                                    );
                                    // Store auth context in request extensions for use by handlers
                                    req.extensions_mut().insert(auth_context);
                                    return service.call(req).await.map(|res| {
                                        res.map_into_left_body()
                                    });
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Authentication failed: {} - client_ip={} token_prefix={}",
                                        e,
                                        client_ip_str,
                                        &token[..token.len().min(10)]
                                    );
                                    let (req, _) = req.into_parts();
                                    let response = HttpResponse::Unauthorized()
                                        .insert_header(("content-type", "text/plain"))
                                        .body("Invalid API key");
                                    return Ok(ServiceResponse::new(req, response.map_into_right_body()));
                                }
                            }
                        } else {
                            tracing::error!("AuthService not found in app data");
                            let (req, _) = req.into_parts();
                            let response = HttpResponse::InternalServerError()
                                .insert_header(("content-type", "text/plain"))
                                .body("Authentication service unavailable");
                            return Ok(ServiceResponse::new(req, response.map_into_right_body()));
                        }
                    } else {
                        tracing::warn!(
                            "Authorization header doesn't start with 'Bearer ': client_ip={}",
                            client_ip_str
                        );
                    }
                } else {
                    tracing::warn!(
                        "Authorization header contains invalid UTF-8: client_ip={}",
                        client_ip_str
                    );
                }
            } else {
                tracing::warn!("Missing Authorization header: client_ip={}", client_ip_str);
            }

            // No valid Bearer token found
            let (req, _) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .insert_header(("content-type", "text/plain"))
                .body("Missing or invalid authorization header");
            Ok(ServiceResponse::new(req, response.map_into_right_body()))
        })
    }
}

/// Helper function for use in handlers to get auth context from request extensions
pub fn extract_auth_context(req: &actix_web::HttpRequest) -> Option<AuthContext> {
    req.extensions().get::<AuthContext>().cloned()
}

// Simple extractor for auth context
use actix_web::{FromRequest, HttpRequest};
use std::pin::Pin;

impl FromRequest for AuthContext {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn futures_util::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            req.extensions()
                .get::<AuthContext>()
                .cloned()
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("Authentication required"))
        })
    }
}
