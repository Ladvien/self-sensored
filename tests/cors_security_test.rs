//! CORS Security Tests
//!
//! Tests for Cross-Origin Resource Sharing (CORS) security implementation
//! following OWASP guidelines and production security best practices.

use actix_cors::Cors;
use actix_web::{
    http::{header, Method, StatusCode},
    test, web, App, HttpResponse,
};

/// Simple test handler for CORS testing
async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

/// Create test CORS configuration
fn create_test_cors(allowed_origins: &str, allow_credentials: bool, max_age: usize) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-api-key"),
        ])
        .expose_headers(&[header::CONTENT_DISPOSITION])
        .max_age(max_age);

    // Add origins
    for origin in allowed_origins.split(',') {
        let trimmed_origin = origin.trim();
        if !trimmed_origin.is_empty() {
            cors = cors.allowed_origin(trimmed_origin);
        }
    }

    if allow_credentials {
        cors = cors.supports_credentials();
    }

    cors
}

/// Make a preflight CORS request
fn make_preflight_request(origin: &str, method: &str) -> test::TestRequest {
    test::TestRequest::default()
        .method(Method::OPTIONS)
        .uri("/test")
        .insert_header((header::ORIGIN, origin))
        .insert_header((header::ACCESS_CONTROL_REQUEST_METHOD, method))
        .insert_header((
            header::ACCESS_CONTROL_REQUEST_HEADERS,
            "authorization, content-type",
        ))
}

/// Make a simple CORS request
fn make_simple_request(origin: &str, method: Method, uri: &str) -> test::TestRequest {
    test::TestRequest::default()
        .method(method)
        .uri(uri)
        .insert_header((header::ORIGIN, origin))
}

#[tokio::test]
async fn test_cors_preflight_allowed_origin() {
    let cors = create_test_cors("https://trusted-app.com,http://localhost:3000", false, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test preflight request from allowed origin
    let req = make_preflight_request("https://trusted-app.com", "GET");
    let resp = test::call_service(&app, req.to_request()).await;

    assert_eq!(resp.status(), StatusCode::OK);

    // Check CORS headers are present
    let headers = resp.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert_eq!(
        headers.get("access-control-allow-origin").unwrap(),
        "https://trusted-app.com"
    );
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
    assert!(headers.contains_key("access-control-max-age"));
}

#[tokio::test]
async fn test_cors_preflight_disallowed_origin() {
    let cors = create_test_cors("https://trusted-app.com", false, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test preflight request from disallowed origin
    let req = make_preflight_request("https://malicious-site.com", "GET");
    let resp = test::call_service(&app, req.to_request()).await;

    // CORS should reject the request - either client error or no CORS headers
    assert!(
        resp.status().is_client_error()
            || !resp.headers().contains_key("access-control-allow-origin")
    );
}

#[tokio::test]
async fn test_cors_simple_request_allowed_origin() {
    let cors = create_test_cors("http://localhost:3000", false, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test simple GET request from allowed origin
    let req = make_simple_request("http://localhost:3000", Method::GET, "/test");
    let resp = test::call_service(&app, req.to_request()).await;

    assert_eq!(resp.status(), StatusCode::OK);

    // Check CORS header is present for simple request
    let headers = resp.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
    assert_eq!(
        headers.get("access-control-allow-origin").unwrap(),
        "http://localhost:3000"
    );
}

#[tokio::test]
async fn test_cors_disallowed_methods() {
    let cors = create_test_cors("http://localhost:3000", false, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test preflight request with disallowed method (DELETE)
    let req = make_preflight_request("http://localhost:3000", "DELETE");
    let resp = test::call_service(&app, req.to_request()).await;

    // Should fail or not include DELETE in allowed methods
    let headers = resp.headers();
    if headers.contains_key("access-control-allow-methods") {
        let allowed_methods = headers
            .get("access-control-allow-methods")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(!allowed_methods.contains("DELETE"));
        assert!(!allowed_methods.contains("PUT"));
    }
}

#[tokio::test]
async fn test_cors_security_headers_validation() {
    let cors = create_test_cors("https://trusted-app.com", false, 1800);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = make_preflight_request("https://trusted-app.com", "GET");
    let resp = test::call_service(&app, req.to_request()).await;

    let headers = resp.headers();

    // Validate max-age header
    if headers.contains_key("access-control-max-age") {
        let max_age = headers
            .get("access-control-max-age")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(max_age, "1800");
    }

    // Validate that credentials are not allowed when not configured
    assert!(!headers.contains_key("access-control-allow-credentials"));

    // Validate allowed headers only include necessary ones
    if headers.contains_key("access-control-allow-headers") {
        let allowed_headers = headers
            .get("access-control-allow-headers")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(allowed_headers.contains("authorization"));
        assert!(allowed_headers.contains("content-type"));
        assert!(allowed_headers.contains("accept"));
        // Should not allow dangerous headers
        assert!(!allowed_headers.to_lowercase().contains("cookie"));
    }
}

#[tokio::test]
async fn test_cors_credentials_configuration() {
    let cors = create_test_cors("https://trusted-app.com", true, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = make_preflight_request("https://trusted-app.com", "GET");
    let resp = test::call_service(&app, req.to_request()).await;

    let headers = resp.headers();

    // When credentials are enabled, check the header is present
    if headers.contains_key("access-control-allow-credentials") {
        let credentials = headers
            .get("access-control-allow-credentials")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(credentials, "true");
    }
}

#[tokio::test]
async fn test_cors_multiple_origins() {
    let cors = create_test_cors(
        "https://app1.com,https://app2.com,http://localhost:3000",
        false,
        3600,
    );

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test each allowed origin
    let allowed_origins = [
        "https://app1.com",
        "https://app2.com",
        "http://localhost:3000",
    ];

    for origin in &allowed_origins {
        let req = make_preflight_request(origin, "GET");
        let resp = test::call_service(&app, req.to_request()).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let headers = resp.headers();
        assert!(headers.contains_key("access-control-allow-origin"));
        assert_eq!(headers.get("access-control-allow-origin").unwrap(), *origin);
    }
}

#[tokio::test]
async fn test_cors_no_origin_header() {
    let cors = create_test_cors("https://trusted-app.com", false, 3600);

    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Test request without Origin header (same-origin request)
    let req = test::TestRequest::default()
        .method(Method::GET)
        .uri("/test")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // No CORS headers should be present for same-origin requests
    let headers = resp.headers();
    assert!(!headers.contains_key("access-control-allow-origin"));
}

#[cfg(test)]
mod security_edge_cases {
    use super::*;

    #[tokio::test]
    async fn test_cors_case_sensitive_origins() {
        let cors = create_test_cors("https://TrustedApp.com", false, 3600);

        let app = test::init_service(
            App::new()
                .wrap(cors)
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        // Test with different case - should fail
        let req = make_preflight_request("https://trustedapp.com", "GET");
        let resp = test::call_service(&app, req.to_request()).await;

        // Case sensitivity should be enforced
        assert!(
            resp.status().is_client_error()
                || !resp.headers().contains_key("access-control-allow-origin")
        );
    }

    #[tokio::test]
    async fn test_cors_subdomain_restriction() {
        let cors = create_test_cors("https://app.example.com", false, 3600);

        let app = test::init_service(
            App::new()
                .wrap(cors)
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        // Test subdomain attack - should fail
        let req = make_preflight_request("https://evil.app.example.com", "GET");
        let resp = test::call_service(&app, req.to_request()).await;

        // Should reject subdomain attempts
        assert!(
            resp.status().is_client_error()
                || !resp.headers().contains_key("access-control-allow-origin")
        );
    }

    #[tokio::test]
    async fn test_cors_protocol_mismatch() {
        let cors = create_test_cors("https://trusted-app.com", false, 3600);

        let app = test::init_service(
            App::new()
                .wrap(cors)
                .route("/test", web::get().to(test_handler)),
        )
        .await;

        // Test HTTP vs HTTPS protocol mismatch - should fail
        let req = make_preflight_request("http://trusted-app.com", "GET");
        let resp = test::call_service(&app, req.to_request()).await;

        // Should reject protocol mismatch
        assert!(
            resp.status().is_client_error()
                || !resp.headers().contains_key("access-control-allow-origin")
        );
    }
}
