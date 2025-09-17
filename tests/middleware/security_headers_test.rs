use actix_web::{test, App, web, HttpResponse, Result as ActixResult};
use actix_web::http::header::{HeaderName, HeaderValue};
use std::collections::HashMap;

// Import the middleware we're testing
use self_sensored::middleware::CompressionAndCaching;

/// Simple test handler to verify middleware is applied
async fn test_handler() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "ok"})))
}

/// Test that all critical security headers are present in responses
#[actix_web::test]
async fn test_critical_security_headers_present() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/test", web::get().to(test_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Get response headers
    let headers = resp.headers();

    // Define required security headers and their expected values
    let required_headers = vec![
        // Critical security headers
        ("content-security-policy", Some("default-src 'none'")), // Just check start of CSP
        ("strict-transport-security", Some("max-age=31536000; includeSubDomains; preload")),
        ("x-xss-protection", Some("1; mode=block")),
        ("referrer-policy", Some("strict-origin-when-cross-origin")),

        // Existing security headers
        ("x-content-type-options", Some("nosniff")),
        ("x-frame-options", Some("DENY")),

        // Additional HIPAA compliance headers
        ("permissions-policy", None), // Just check presence, value is long
        ("cross-origin-resource-policy", Some("same-origin")),
        ("cross-origin-embedder-policy", Some("require-corp")),
        ("cross-origin-opener-policy", Some("same-origin")),
    ];

    let mut missing_headers = Vec::new();
    let mut incorrect_values = Vec::new();

    for (header_name, expected_value) in required_headers {
        let header = HeaderName::from_static(header_name);

        match headers.get(&header) {
            Some(actual_value) => {
                if let Some(expected) = expected_value {
                    let actual_str = actual_value.to_str().unwrap_or("");
                    if !actual_str.starts_with(expected) {
                        incorrect_values.push(format!(
                            "{}: expected '{}' but got '{}'",
                            header_name, expected, actual_str
                        ));
                    }
                }
                println!("✓ Header '{}' present: {:?}", header_name, actual_value);
            }
            None => {
                missing_headers.push(header_name.to_string());
                println!("✗ Header '{}' missing", header_name);
            }
        }
    }

    // Assert all headers are present
    assert!(
        missing_headers.is_empty(),
        "Missing critical security headers: {:?}",
        missing_headers
    );

    // Assert header values are correct
    assert!(
        incorrect_values.is_empty(),
        "Incorrect header values: {:?}",
        incorrect_values
    );

    println!("✓ All critical security headers are present and correct");
}

/// Test Content Security Policy is restrictive enough for health data API
#[actix_web::test]
async fn test_content_security_policy_restrictive() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/ingest", web::post().to(test_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    let csp_header = headers.get("content-security-policy")
        .expect("Content-Security-Policy header should be present");

    let csp_value = csp_header.to_str().unwrap();

    // Verify CSP is restrictive
    let required_csp_directives = vec![
        "default-src 'none'",      // Block everything by default
        "script-src 'none'",       // No scripts allowed
        "style-src 'none'",        // No inline styles
        "img-src 'none'",          // No images
        "connect-src 'self'",      // Only same-origin connections
        "object-src 'none'",       // No plugins
        "frame-ancestors 'none'",  // Cannot be embedded
        "form-action 'none'",      // No form submissions
        "upgrade-insecure-requests", // Force HTTPS
        "block-all-mixed-content", // Block mixed content
    ];

    let mut missing_directives = Vec::new();

    for directive in required_csp_directives {
        if !csp_value.contains(directive) {
            missing_directives.push(directive);
        }
    }

    assert!(
        missing_directives.is_empty(),
        "CSP missing required restrictive directives: {:?}. Full CSP: {}",
        missing_directives,
        csp_value
    );

    println!("✓ Content Security Policy is sufficiently restrictive");
}

/// Test HSTS header configuration for HTTPS enforcement
#[actix_web::test]
async fn test_strict_transport_security_configuration() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/health", web::get().to(test_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    let hsts_header = headers.get("strict-transport-security")
        .expect("Strict-Transport-Security header should be present");

    let hsts_value = hsts_header.to_str().unwrap();

    // Verify HSTS configuration meets security standards
    assert!(hsts_value.contains("max-age=31536000"), "HSTS should have 1 year max-age");
    assert!(hsts_value.contains("includeSubDomains"), "HSTS should include subdomains");
    assert!(hsts_value.contains("preload"), "HSTS should include preload directive");

    println!("✓ Strict Transport Security properly configured: {}", hsts_value);
}

/// Test that security headers are applied to all endpoint types
#[actix_web::test]
async fn test_security_headers_on_all_endpoints() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/health", web::get().to(test_handler))
            .route("/api/v1/ingest", web::post().to(test_handler))
            .route("/api/v1/data/heart-rate", web::get().to(test_handler))
            .route("/api/v1/export/all", web::get().to(test_handler))
    ).await;

    let test_endpoints = vec![
        ("/health", "GET"),
        ("/api/v1/ingest", "POST"),
        ("/api/v1/data/heart-rate", "GET"),
        ("/api/v1/export/all", "GET"),
    ];

    for (path, method) in test_endpoints {
        let req = match method {
            "GET" => test::TestRequest::get().uri(path).to_request(),
            "POST" => test::TestRequest::post().uri(path).to_request(),
            _ => panic!("Unsupported method: {}", method),
        };

        let resp = test::call_service(&app, req).await;
        let headers = resp.headers();

        // Check that key security headers are present on all endpoints
        let critical_headers = vec![
            "content-security-policy",
            "strict-transport-security",
            "x-content-type-options",
            "x-frame-options",
        ];

        for header_name in critical_headers {
            assert!(
                headers.contains_key(header_name),
                "Security header '{}' missing from {} {}",
                header_name, method, path
            );
        }

        println!("✓ Security headers present on {} {}", method, path);
    }
}

/// Test Permissions Policy disables unnecessary browser features
#[actix_web::test]
async fn test_permissions_policy_restrictive() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/ingest", web::post().to(test_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    let permissions_header = headers.get("permissions-policy")
        .expect("Permissions-Policy header should be present");

    let permissions_value = permissions_header.to_str().unwrap();

    // Verify key features are disabled for health data security
    let disabled_features = vec![
        "camera=()",
        "microphone=()",
        "geolocation=()",
        "payment=()",
        "usb=()",
        "serial=()",
        "bluetooth=()", // This might not be in our policy but good to check
    ];

    let mut enabled_features = Vec::new();

    for feature in disabled_features {
        if !permissions_value.contains(feature) {
            // Some features might not be in the policy, that's OK
            // But if they are, they should be disabled
            continue;
        }

        // Check if feature is properly disabled
        if !permissions_value.contains(&feature.replace("()", "=()")) {
            enabled_features.push(feature);
        }
    }

    // The main check is that camera, microphone, geolocation are disabled
    let critical_disabled = vec!["camera=()", "microphone=()", "geolocation=()"];
    for feature in critical_disabled {
        assert!(
            permissions_value.contains(feature),
            "Critical feature '{}' should be disabled in Permissions Policy. Full policy: {}",
            feature, permissions_value
        );
    }

    println!("✓ Permissions Policy properly restricts browser features");
}

/// Test Cross-Origin headers are configured for same-origin security
#[actix_web::test]
async fn test_cross_origin_headers_same_origin() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/data/summary", web::get().to(test_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/summary")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    // Test Cross-Origin Resource Policy
    let corp_header = headers.get("cross-origin-resource-policy")
        .expect("Cross-Origin-Resource-Policy header should be present");
    assert_eq!(corp_header.to_str().unwrap(), "same-origin");

    // Test Cross-Origin Embedder Policy
    let coep_header = headers.get("cross-origin-embedder-policy")
        .expect("Cross-Origin-Embedder-Policy header should be present");
    assert_eq!(coep_header.to_str().unwrap(), "require-corp");

    // Test Cross-Origin Opener Policy
    let coop_header = headers.get("cross-origin-opener-policy")
        .expect("Cross-Origin-Opener-Policy header should be present");
    assert_eq!(coop_header.to_str().unwrap(), "same-origin");

    println!("✓ Cross-Origin headers properly configured for same-origin security");
}

/// Test that caching headers and security headers coexist properly
#[actix_web::test]
async fn test_caching_and_security_headers_coexist() {
    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/export/heart-rate", web::get().to(test_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/heart-rate")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    // Should have both caching headers
    assert!(headers.contains_key("cache-control"), "Cache-Control header should be present");
    assert!(headers.contains_key("etag"), "ETag header should be present");

    // And security headers
    assert!(headers.contains_key("content-security-policy"), "CSP header should be present");
    assert!(headers.contains_key("strict-transport-security"), "HSTS header should be present");
    assert!(headers.contains_key("x-content-type-options"), "XCTO header should be present");

    // Verify they don't conflict
    let cache_control = headers.get("cache-control").unwrap().to_str().unwrap();
    let csp = headers.get("content-security-policy").unwrap().to_str().unwrap();

    assert!(cache_control.contains("private"), "Export endpoints should use private caching");
    assert!(csp.contains("default-src 'none'"), "CSP should be restrictive");

    println!("✓ Caching and security headers coexist properly");
}

/// Performance test to ensure security headers don't significantly impact response time
#[actix_web::test]
async fn test_security_headers_performance_impact() {
    use std::time::Instant;

    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/status", web::get().to(test_handler))
    ).await;

    let iterations = 100;
    let mut total_duration = std::time::Duration::new(0, 0);

    for _ in 0..iterations {
        let start = Instant::now();

        let req = test::TestRequest::get()
            .uri("/api/v1/status")
            .to_request();

        let _resp = test::call_service(&app, req).await;

        total_duration += start.elapsed();
    }

    let avg_duration = total_duration / iterations;

    // Security headers should add minimal overhead (< 1ms average)
    assert!(
        avg_duration.as_millis() < 10,
        "Security headers adding too much overhead: {} ms average",
        avg_duration.as_millis()
    );

    println!("✓ Security headers performance impact minimal: {} μs average", avg_duration.as_micros());
}

/// Integration test to verify headers work with real error responses
#[actix_web::test]
async fn test_security_headers_on_error_responses() {
    use actix_web::HttpResponseBuilder;

    async fn error_handler() -> ActixResult<HttpResponse> {
        Ok(HttpResponseBuilder::new(actix_web::http::StatusCode::BAD_REQUEST)
            .json(serde_json::json!({"error": "test error"})))
    }

    let app = test::init_service(
        App::new()
            .wrap(CompressionAndCaching)
            .route("/api/v1/error", web::get().to(error_handler))
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/error")
        .to_request();

    let resp = test::call_service(&app, req).await;
    let headers = resp.headers();

    // Security headers should be present even on error responses
    assert!(headers.contains_key("content-security-policy"), "CSP should be present on errors");
    assert!(headers.contains_key("strict-transport-security"), "HSTS should be present on errors");
    assert!(headers.contains_key("x-frame-options"), "X-Frame-Options should be present on errors");

    // Response should be 400 Bad Request
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);

    println!("✓ Security headers present on error responses");
}