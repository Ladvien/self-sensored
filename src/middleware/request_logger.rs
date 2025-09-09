use actix_web::{
    body::MessageBody,
    dev::{forward_ready, ServiceRequest, ServiceResponse, Transform},
    http::Method,
    Error,
};
use futures_util::{future::LocalBoxFuture, stream::StreamExt};
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::debug;

pub struct RequestLogger;

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestLoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Only log POST requests to /api/v1/ingest
            if req.method() == Method::POST && req.path() == "/api/v1/ingest" {
                let (http_req, mut payload) = req.into_parts();

                // Collect the payload
                let mut body_bytes = actix_web::web::BytesMut::new();
                while let Some(chunk) = payload.next().await {
                    let chunk = chunk?;
                    body_bytes.extend_from_slice(&chunk);
                }

                // Log the raw payload
                let body_str = String::from_utf8_lossy(&body_bytes);
                debug!("Raw request body for /api/v1/ingest: {}", body_str);
                debug!(
                    "Body length: {} bytes, Content-Type: {:?}",
                    body_bytes.len(),
                    http_req.headers().get("content-type")
                );

                // Create a new payload from the captured bytes
                let new_payload = actix_web::dev::Payload::from(body_bytes.freeze());
                req = ServiceRequest::from_parts(http_req, new_payload);
            }

            // Call the next service
            service.call(req).await
        })
    }
}
