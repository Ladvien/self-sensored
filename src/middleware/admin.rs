use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, Result,
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};

use crate::services::auth::{AuthContext, AuthService};

/// Admin-only middleware that ensures only users with admin permissions can access protected routes
pub struct AdminMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AdminMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AdminMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AdminMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AdminMiddlewareService<S>
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
            // Extract auth context from request extensions (should be set by AuthMiddleware)
            let auth_context = req.extensions().get::<AuthContext>().cloned();

            match auth_context {
                Some(ref auth) => {
                    // Check if user has admin permissions
                    if AuthService::has_admin_permission(auth) {
                        tracing::info!(
                            event = "admin_access_granted",
                            user_id = %auth.user.id,
                            api_key_id = %auth.api_key.id,
                            path = req.path(),
                            method = %req.method(),
                            client_ip = req.connection_info().peer_addr().unwrap_or("unknown"),
                            message = "Admin access granted for protected endpoint"
                        );
                        service.call(req).await
                    } else {
                        tracing::warn!(
                            event = "admin_access_denied",
                            user_id = %auth.user.id,
                            api_key_id = %auth.api_key.id,
                            path = req.path(),
                            method = %req.method(),
                            client_ip = req.connection_info().peer_addr().unwrap_or("unknown"),
                            permissions = ?auth.api_key.permissions,
                            message = "Non-admin user attempted to access admin endpoint"
                        );
                        Err(actix_web::error::ErrorForbidden(
                            "Admin privileges required to access this endpoint",
                        ))
                    }
                }
                None => {
                    tracing::error!(
                        event = "admin_middleware_no_auth",
                        path = req.path(),
                        method = %req.method(),
                        client_ip = req.connection_info().peer_addr().unwrap_or("unknown"),
                        message = "Admin middleware called without authentication context"
                    );
                    Err(actix_web::error::ErrorUnauthorized(
                        "Authentication required for admin endpoints",
                    ))
                }
            }
        })
    }
}
