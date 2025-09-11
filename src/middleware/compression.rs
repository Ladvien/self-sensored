use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue, CACHE_CONTROL, ETAG, EXPIRES},
    Error, Result,
};
use futures_util::future::{ready, Ready};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::SystemTime,
};
use tracing::debug;

/// Compression and caching headers middleware
pub struct CompressionAndCaching;

impl<S, B> Transform<S, ServiceRequest> for CompressionAndCaching
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
    type Transform = CompressionAndCachingMiddleware<S>;
    type InitError = ();

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CompressionAndCachingMiddleware { service }))
    }
}

pub struct CompressionAndCachingMiddleware<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for CompressionAndCachingMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut response = fut.await?;

            // Add caching headers based on endpoint type
            add_caching_headers(&mut response, &path);

            // Add performance headers
            add_performance_headers(&mut response);

            debug!("Added caching and performance headers for {}", path);
            Ok(response)
        })
    }
}

/// Add appropriate caching headers based on the endpoint
fn add_caching_headers<B>(response: &mut ServiceResponse<B>, path: &str) {
    let headers = response.headers_mut();

    if path.starts_with("/api/v1/export/") {
        // Export endpoints - cache for 5 minutes
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=300, must-revalidate"),
        );
    } else if path.starts_with("/api/v1/data/") {
        // Data query endpoints - cache for 1 minute
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=60, must-revalidate"),
        );
    } else if path == "/health" || path == "/api/v1/status" {
        // Health endpoints - no cache
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
    } else if path.starts_with("/api/v1/ingest") {
        // Ingest endpoints - never cache
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
        headers.insert(EXPIRES, HeaderValue::from_static("0"));
    } else {
        // Default caching for other endpoints
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=300"),
        );
    }

    // Add ETags for data endpoints to enable conditional requests
    if path.starts_with("/api/v1/data/") || path.starts_with("/api/v1/export/") {
        let etag = format!("\"{}\"", generate_simple_etag());
        if let Ok(etag_value) = HeaderValue::from_str(&etag) {
            headers.insert(ETAG, etag_value);
        }
    }
}

/// Add performance-related headers
fn add_performance_headers<B>(response: &mut ServiceResponse<B>) {
    let headers = response.headers_mut();

    // Add server timing information
    if let Ok(header_value) = HeaderValue::from_str("app;dur=0") {
        headers.insert(HeaderName::from_static("server-timing"), header_value);
    }

    // Add security headers that can impact performance
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    // Add compression hint
    headers.insert(
        HeaderName::from_static("vary"),
        HeaderValue::from_static("Accept-Encoding"),
    );
}

/// Generate a simple ETag based on current timestamp
fn generate_simple_etag() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Use a hash of the timestamp rounded to nearest minute for cache efficiency
    let rounded_time = (now / 60) * 60;
    format!("{:x}", rounded_time)
}
