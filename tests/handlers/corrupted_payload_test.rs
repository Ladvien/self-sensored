use actix_web::{test as actix_test, web, App};
use self_sensored::db;
use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod corrupted_payload_tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        // Get test database URL from environment or use default
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/health_export_test".to_string());

        // Create connection pool
        let pool = PgPool::connect(&database_url).await
            .expect("Failed to connect to test database");

        // Clean up any existing test data
        sqlx::query("DELETE FROM raw_ingestions WHERE processing_status = 'error'")
            .execute(&pool)
            .await
            .expect("Failed to clean up test data");

        pool
    }

    async fn create_test_user_with_api_key(pool: &PgPool) -> (Uuid, String) {
        let user_id = Uuid::new_v4();
        let email = format!("test_{}@example.com", user_id);

        // Create user
        sqlx::query!(
            "INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            user_id,
            email
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        // Create API key
        let api_key = format!("test_key_{}", Uuid::new_v4());
        let key_hash = format!("hash_{}", api_key); // In real implementation, this should be properly hashed

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

    #[actix_web::test]
    async fn test_save_corrupted_payload_invalid_json() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = create_test_user_with_api_key(&pool).await;

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(AuthMiddleware)
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Send invalid JSON payload
        let invalid_json = r#"{"data": {"metrics": [{"invalid": "json"#; // Incomplete JSON

        let req = actix_test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("X-API-Key", api_key.as_str()))
            .insert_header(("Content-Type", "application/json"))
            .set_payload(invalid_json)
            .to_request();

        let resp = actix_test::call_service(&app, req).await;

        // Should return bad request
        assert_eq!(resp.status(), 400);

        // Verify corrupted payload was saved
        let saved_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM raw_ingestions
             WHERE user_id = $1 AND processing_status = 'error'",
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count saved corrupted payloads");

        assert_eq!(saved_count, 1, "Corrupted payload should be saved");

        // Verify error details are stored
        let error_details = sqlx::query!(
            "SELECT processing_errors FROM raw_ingestions
             WHERE user_id = $1 AND processing_status = 'error'",
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch error details");

        assert!(error_details.processing_errors.is_some());
        let errors_json = error_details.processing_errors.unwrap();
        assert!(errors_json.to_string().contains("json_parse_error"));
    }

    #[actix_web::test]
    async fn test_save_corrupted_payload_malformed_structure() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = create_test_user_with_api_key(&pool).await;

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(AuthMiddleware)
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Send JSON with wrong structure
        let malformed_json = r#"{"wrong_field": "value", "no_data": true}"#;

        let req = actix_test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("X-API-Key", api_key.as_str()))
            .insert_header(("Content-Type", "application/json"))
            .set_payload(malformed_json)
            .to_request();

        let resp = actix_test::call_service(&app, req).await;

        // Should return bad request
        assert_eq!(resp.status(), 400);

        // Verify corrupted payload was saved
        let saved = sqlx::query!(
            "SELECT id, raw_payload, payload_hash FROM raw_ingestions
             WHERE user_id = $1 AND processing_status = 'error'
             ORDER BY created_at DESC LIMIT 1",
            user_id
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to fetch saved corrupted payload");

        assert!(saved.is_some(), "Corrupted payload should be saved");

        let saved_data = saved.unwrap();
        assert!(saved_data.raw_payload.is_some());

        // Verify the raw payload contains the original malformed data
        let raw_json = saved_data.raw_payload.unwrap();
        assert!(raw_json.to_string().contains("corrupted"));
        assert!(raw_json.to_string().contains("parse_error"));
    }

    #[actix_web::test]
    async fn test_save_corrupted_payload_with_hash() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = create_test_user_with_api_key(&pool).await;

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(AuthMiddleware)
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Send specific invalid payload to test hash generation
        let invalid_payload = r#"{"data": {"metrics": null}}"#;

        let req = actix_test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("X-API-Key", api_key.as_str()))
            .insert_header(("Content-Type", "application/json"))
            .set_payload(invalid_payload)
            .to_request();

        let resp = actix_test::call_service(&app, req).await;

        // Should return bad request
        assert_eq!(resp.status(), 400);

        // Verify hash was calculated and stored
        let saved = sqlx::query!(
            "SELECT payload_hash, payload_size_bytes FROM raw_ingestions
             WHERE user_id = $1 AND processing_status = 'error'
             ORDER BY created_at DESC LIMIT 1",
            user_id
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to fetch saved corrupted payload");

        assert!(saved.is_some(), "Corrupted payload should be saved");

        let saved_data = saved.unwrap();
        assert!(!saved_data.payload_hash.is_empty(), "Hash should be calculated");
        assert_eq!(saved_data.payload_size_bytes as usize, invalid_payload.len(), "Size should match");
    }

    #[actix_web::test]
    async fn test_corrupted_payload_recovery_info() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = create_test_user_with_api_key(&pool).await;

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(AuthMiddleware)
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Send invalid JSON with specific error
        let invalid_json = r#"{"data": {"metrics": [1,2,3]}}"#; // Wrong type for metrics

        let req = actix_test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("X-API-Key", api_key.as_str()))
            .insert_header(("Content-Type", "application/json"))
            .set_payload(invalid_json)
            .to_request();

        let resp = actix_test::call_service(&app, req).await;

        // Should return bad request
        assert_eq!(resp.status(), 400);

        // Verify recovery information is stored
        let saved = sqlx::query!(
            "SELECT raw_payload, processing_errors FROM raw_ingestions
             WHERE user_id = $1 AND processing_status = 'error'
             ORDER BY created_at DESC LIMIT 1",
            user_id
        )
        .fetch_optional(&pool)
        .await
        .expect("Failed to fetch saved corrupted payload");

        assert!(saved.is_some(), "Corrupted payload should be saved");

        let saved_data = saved.unwrap();

        // Verify raw payload contains original text for recovery
        let raw_json = saved_data.raw_payload.unwrap();
        assert!(raw_json.to_string().contains("raw_text"));

        // Verify error information is comprehensive
        let errors = saved_data.processing_errors.unwrap();
        assert!(errors.to_string().contains("error_type"));
        assert!(errors.to_string().contains("error_message"));
        assert!(errors.to_string().contains("severity"));
        assert!(errors.to_string().contains("timestamp"));
    }
}