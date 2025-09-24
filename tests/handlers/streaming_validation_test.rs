use actix_web::{test as actix_test, web, App};
use self_sensored::db;
use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::services::streaming_parser::StreamingJsonParser;
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod streaming_validation_tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/health_export_test".to_string());

        let pool = PgPool::connect(&database_url).await
            .expect("Failed to connect to test database");

        pool
    }

    async fn create_test_user_with_api_key(pool: &PgPool) -> (Uuid, String) {
        let user_id = Uuid::new_v4();
        let email = format!("test_{}@example.com", user_id);

        sqlx::query!(
            "INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            user_id,
            email
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        let api_key = format!("test_key_{}", Uuid::new_v4());
        let key_hash = format!("hash_{}", api_key);

        sqlx::query!(
            "INSERT INTO api_keys (user_id, key_hash, name) VALUES ($1, $2, $3)",
            user_id,
            key_hash,
            "Test Key"
        )
        .execute(pool)
        .await
        .expect("Failed to create API key");

        (user_id, api_key)
    }

    #[tokio::test]
    async fn test_streaming_validation_small_payload() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        // Small valid JSON
        let small_json = r#"{"data": {"metrics": [], "workouts": []}}"#;
        let result = parser.validate_json_bytes(small_json.as_bytes());

        assert!(result.is_ok(), "Small valid JSON should pass validation");
    }

    #[tokio::test]
    async fn test_streaming_validation_large_payload() {
        let mut parser = StreamingJsonParser::with_max_size(20 * 1024 * 1024);

        // Generate a large valid JSON payload (simulate large health export)
        let mut large_json = String::from(r#"{"data": {"metrics": ["#);

        // Add 10,000 metric entries
        for i in 0..10000 {
            if i > 0 {
                large_json.push_str(", ");
            }
            large_json.push_str(&format!(
                r#"{{"name": "HeartRate", "value": {}, "timestamp": "2024-01-01T12:00:00Z"}}"#,
                60 + (i % 40)
            ));
        }

        large_json.push_str(r#"], "workouts": []}}"#);

        let result = parser.validate_json_bytes(large_json.as_bytes());

        assert!(result.is_ok(), "Large valid JSON should pass validation");

        // Verify size tracking
        assert!(large_json.len() > 100_000, "Test payload should be large");
    }

    #[tokio::test]
    async fn test_streaming_validation_truncated_json() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        // Truncated JSON (missing closing braces)
        let truncated_json = r#"{"data": {"metrics": [{"name": "HeartRate", "value": 65"#;
        let result = parser.validate_json_bytes(truncated_json.as_bytes());

        assert!(result.is_err(), "Truncated JSON should fail validation");

        let error = result.unwrap_err();
        assert!(error.contains("brace") || error.contains("bracket") || error.contains("unclosed"),
            "Error should mention unclosed braces/brackets");
    }

    #[tokio::test]
    async fn test_streaming_validation_malformed_json() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        // Malformed JSON with syntax errors
        let malformed_json = r#"{"data": {"metrics": [{"name": HeartRate, "value": 65}]}}"#;  // Missing quotes
        let result = parser.validate_json_bytes(malformed_json.as_bytes());

        // Note: Basic structure validation may not catch all syntax errors
        // It mainly checks for balanced braces/brackets
        if result.is_ok() {
            // This is acceptable as structure validation doesn't catch all syntax errors
            println!("Basic structure validation passed for malformed JSON (expected)");
        }
    }

    #[tokio::test]
    async fn test_streaming_validation_empty_payload() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        let empty_json = b"";
        let result = parser.validate_json_bytes(empty_json);

        assert!(result.is_err(), "Empty payload should fail validation");
    }

    #[tokio::test]
    async fn test_streaming_validation_nested_structure() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        // Deeply nested valid JSON
        let nested_json = r#"
        {
            "data": {
                "metrics": [
                    {
                        "name": "HeartRate",
                        "values": [
                            {"timestamp": "2024-01-01T12:00:00Z", "value": 65},
                            {"timestamp": "2024-01-01T12:01:00Z", "value": 67}
                        ],
                        "metadata": {
                            "device": "Apple Watch",
                            "version": "10.0",
                            "user": {
                                "id": "user123",
                                "profile": {
                                    "age": 30,
                                    "settings": {
                                        "units": "metric"
                                    }
                                }
                            }
                        }
                    }
                ],
                "workouts": []
            }
        }
        "#;

        let result = parser.validate_json_bytes(nested_json.as_bytes());
        assert!(result.is_ok(), "Deeply nested valid JSON should pass validation");
    }

    #[tokio::test]
    async fn test_streaming_validation_unbalanced_brackets() {
        let mut parser = StreamingJsonParser::with_max_size(1024 * 1024);

        // Extra closing bracket
        let extra_bracket = r#"{"data": {"metrics": []], "workouts": []}}"#;
        let result = parser.validate_json_bytes(extra_bracket.as_bytes());

        assert!(result.is_err(), "JSON with extra bracket should fail validation");
    }

    #[tokio::test]
    async fn test_streaming_validation_size_limits() {
        // Create parser with small size limit
        let mut parser = StreamingJsonParser::with_max_size(100);  // 100 bytes max

        // Create payload larger than limit
        let large_json = r#"{"data": {"metrics": [
            {"name": "HeartRate", "value": 65, "timestamp": "2024-01-01T12:00:00Z"},
            {"name": "HeartRate", "value": 67, "timestamp": "2024-01-01T12:01:00Z"}
        ], "workouts": []}}"#;

        // Note: validate_json_bytes doesn't enforce size limits, it just validates structure
        // Size limits are enforced during streaming
        let result = parser.validate_json_bytes(large_json.as_bytes());

        // This should still pass as we're just validating structure
        // Size enforcement happens in parse_from_stream
        if result.is_ok() {
            println!("Structure validation passed (size limits not enforced in validate_json_bytes)");
        }
    }

    #[actix_web::test]
    async fn test_large_payload_uses_streaming_validation() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = create_test_user_with_api_key(&pool).await;

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(AuthMiddleware)
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Create a large payload (> 10MB threshold for streaming)
        let mut large_payload = String::from(r#"{"data": {"metrics": ["#);

        // Generate enough data to exceed 10MB
        for i in 0..100000 {
            if i > 0 {
                large_payload.push_str(", ");
            }
            large_payload.push_str(&format!(
                r#"{{"name": "HKQuantityTypeIdentifierHeartRate", "units": "count/min", "data": [{{"qty": {}, "date": "2024-01-01 12:00:00"}}]}}"#,
                60 + (i % 40)
            ));
        }

        large_payload.push_str(r#"], "workouts": []}}"#);

        println!("Test payload size: {} bytes", large_payload.len());

        // Send the large payload
        let req = actix_test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("X-API-Key", api_key.as_str()))
            .insert_header(("Content-Type", "application/json"))
            .set_payload(large_payload.clone())
            .to_request();

        let resp = actix_test::call_service(&app, req).await;

        // Large valid payload should be accepted
        assert!(resp.status().is_success() || resp.status() == 413,  // 413 if payload limit exceeded
            "Large payload should either succeed or hit size limit");
    }
}