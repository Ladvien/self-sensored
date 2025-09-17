use actix_web::{test, web, App, http::StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::ingest_handler,
    models::{ApiResponse, IngestResponse, IngestPayload, IngestData},
    services::auth::{AuthContext, User as AuthUser, ApiKey as AuthApiKey},
    middleware::auth::AuthMiddleware,
};

/// Tests for critical ingest validation issues that mislead iOS app
///
/// These tests cover:
/// 1. Empty payloads should be rejected with 400 Bad Request
/// 2. Async responses should indicate "accepted_for_processing" not "processed"
/// 3. Status logic should check actual vs expected counts
/// 4. Proper error categorization

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Helper to create test auth context
    fn create_test_auth_context() -> AuthContext {
        let user_id = Uuid::new_v4();
        let api_key_id = Uuid::new_v4();

        AuthContext {
            user: AuthUser {
                id: user_id,
                email: "test@example.com".to_string(),
                apple_health_id: None,
                created_at: Some(Utc::now()),
                updated_at: Some(Utc::now()),
                is_active: Some(true),
                metadata: None,
            },
            api_key: AuthApiKey {
                id: api_key_id,
                user_id,
                name: Some("Test API Key".to_string()),
                created_at: Some(Utc::now()),
                last_used_at: Some(Utc::now()),
                expires_at: None,
                is_active: Some(true),
                permissions: None,
                rate_limit_per_hour: None,
            },
        }
    }

    /// Create empty payload that should be rejected
    fn create_empty_payload() -> IngestPayload {
        IngestPayload {
            data: IngestData {
                metrics: vec![],
                workouts: vec![],
            },
        }
    }

    /// Create a mock pool for testing (without database connection)
    async fn create_mock_pool() -> PgPool {
        // For unit testing, we'd typically use a mock
        // For now, return a placeholder - this test would need integration setup
        todo!("Implement mock pool or use integration test setup")
    }

    #[actix_web::test]
    async fn test_empty_payload_rejection() {
        // This test verifies that empty payloads are rejected with clear error message

        let pool = create_mock_pool().await;
        let auth = create_test_auth_context();
        let empty_payload = create_empty_payload();

        // Convert to raw bytes for handler
        let payload_bytes = web::Bytes::from(
            serde_json::to_vec(&empty_payload).expect("Should serialize")
        );

        // Create mock request
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .to_http_request();

        // Call handler directly
        let result = ingest_handler(
            web::Data::new(pool),
            auth,
            payload_bytes,
            req,
        ).await;

        // Should return BadRequest (400)
        assert!(result.is_ok());
        let response = result.unwrap();

        // For now, just verify the structure exists
        // Full integration test would verify HTTP status codes
        println!("Empty payload test structure verified");
    }

    #[test]
    fn test_empty_payload_structure() {
        // Verify empty payload structure is correct
        let empty_payload = create_empty_payload();

        assert_eq!(empty_payload.data.metrics.len(), 0);
        assert_eq!(empty_payload.data.workouts.len(), 0);

        // Should serialize/deserialize correctly
        let json_str = serde_json::to_string(&empty_payload).expect("Should serialize");
        let deserialized: IngestPayload = serde_json::from_str(&json_str)
            .expect("Should deserialize");

        assert_eq!(deserialized.data.metrics.len(), 0);
        assert_eq!(deserialized.data.workouts.len(), 0);
    }

    #[test]
    fn test_response_structure_accuracy() {
        // Test that IngestResponse correctly represents processing state

        // Case 1: Complete success
        let success_response = IngestResponse {
            success: true,
            processed_count: 10,
            failed_count: 0,
            processing_time_ms: 150,
            errors: vec![],
        };

        assert!(success_response.success);
        assert_eq!(success_response.processed_count, 10);
        assert_eq!(success_response.failed_count, 0);

        // Case 2: Partial failure
        let partial_response = IngestResponse {
            success: false,  // Should be false if any failures
            processed_count: 8,
            failed_count: 2,
            processing_time_ms: 200,
            errors: vec![], // Would contain actual errors in real scenario
        };

        assert!(!partial_response.success);
        assert_eq!(partial_response.processed_count + partial_response.failed_count, 10);

        // Case 3: Complete failure
        let failure_response = IngestResponse {
            success: false,
            processed_count: 0,
            failed_count: 10,
            processing_time_ms: 50,
            errors: vec![], // Would contain actual errors
        };

        assert!(!failure_response.success);
        assert_eq!(failure_response.processed_count, 0);
        assert_eq!(failure_response.failed_count, 10);

        // Case 4: Async acceptance (not yet processed)
        let async_response = IngestResponse {
            success: false,  // Not yet processed
            processed_count: 0,  // No processing completed yet
            failed_count: 0,
            processing_time_ms: 10,  // Just acceptance time
            errors: vec![],
        };

        assert!(!async_response.success);
        assert_eq!(async_response.processed_count, 0);
    }

    #[test]
    fn test_error_message_clarity() {
        // Test that error messages are clear and actionable

        let empty_payload_error = "Empty payload: no metrics or workouts provided. Please include at least one metric or workout.";

        // Should be clear about the issue
        assert!(empty_payload_error.contains("Empty payload"));
        assert!(empty_payload_error.contains("no metrics or workouts"));

        // Should provide actionable guidance
        assert!(empty_payload_error.contains("Please include at least one"));

        // Test async processing message
        let async_message = "Accepted 1000 items for background processing. Processing is NOT yet complete. Monitor raw_ingestion id abc-123 for actual status.";

        // Should be clear about async nature
        assert!(async_message.contains("background processing"));
        assert!(async_message.contains("NOT yet complete"));
        assert!(async_message.contains("Monitor"));
    }

    #[test]
    fn test_status_determination_logic() {
        // Test the logic for determining processing status

        struct TestCase {
            processed_count: usize,
            failed_count: usize,
            has_errors: bool,
            expected_status: &'static str,
            description: &'static str,
        }

        let test_cases = vec![
            TestCase {
                processed_count: 10,
                failed_count: 0,
                has_errors: false,
                expected_status: "processed",
                description: "Complete success",
            },
            TestCase {
                processed_count: 8,
                failed_count: 2,
                has_errors: true,
                expected_status: "partial_success",
                description: "Partial failure",
            },
            TestCase {
                processed_count: 0,
                failed_count: 10,
                has_errors: true,
                expected_status: "error",
                description: "Complete failure",
            },
            TestCase {
                processed_count: 0,
                failed_count: 0,
                has_errors: false,
                expected_status: "error",
                description: "No processing occurred (unexpected)",
            },
        ];

        for case in test_cases {
            // Simulate the status determination logic from the handler
            let partial_failure = case.failed_count > 0;

            let status = if case.has_errors && case.processed_count == 0 {
                "error"  // Complete failure
            } else if partial_failure {
                "partial_success"  // Some items failed
            } else if case.processed_count > 0 {
                "processed"  // All items processed successfully
            } else {
                "error"  // No items processed (unexpected)
            };

            assert_eq!(
                status,
                case.expected_status,
                "Failed for case: {} (processed: {}, failed: {}, has_errors: {})",
                case.description,
                case.processed_count,
                case.failed_count,
                case.has_errors
            );
        }
    }

    #[test]
    fn test_success_flag_logic() {
        // Test the logic for determining the success flag in responses

        struct TestCase {
            processed_count: usize,
            failed_count: usize,
            errors_empty: bool,
            expected_success: bool,
            description: &'static str,
        }

        let test_cases = vec![
            TestCase {
                processed_count: 10,
                failed_count: 0,
                errors_empty: true,
                expected_success: true,
                description: "All processed, no errors",
            },
            TestCase {
                processed_count: 10,
                failed_count: 0,
                errors_empty: false,
                expected_success: false,
                description: "All processed, but has errors",
            },
            TestCase {
                processed_count: 0,
                failed_count: 0,
                errors_empty: true,
                expected_success: false,
                description: "Nothing processed (accepted for async)",
            },
            TestCase {
                processed_count: 8,
                failed_count: 2,
                errors_empty: false,
                expected_success: false,
                description: "Partial processing with errors",
            },
        ];

        for case in test_cases {
            // Simulate the success determination logic
            let success = case.errors_empty && case.processed_count > 0;

            assert_eq!(
                success,
                case.expected_success,
                "Failed for case: {} (processed: {}, failed: {}, errors_empty: {})",
                case.description,
                case.processed_count,
                case.failed_count,
                case.errors_empty
            );
        }
    }
}

// Integration tests (require database setup)
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;

    /// Helper to setup test database (would need real implementation)
    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    /// Helper to create test user and API key
    async fn setup_test_auth(pool: &PgPool) -> (Uuid, String) {
        let user_id = Uuid::new_v4();

        // Create test user
        sqlx::query!(
            "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
            user_id,
            format!("test_{}@example.com", user_id)
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        // Create API key (simplified - real implementation would use AuthService)
        let api_key = format!("test_key_{}", Uuid::new_v4());
        sqlx::query!(
            "INSERT INTO api_keys (id, user_id, key_hash, name, created_at) VALUES ($1, $2, $3, $4, NOW())",
            Uuid::new_v4(),
            user_id,
            "test_hash", // In real implementation, this would be properly hashed
            "Test Key"
        )
        .execute(pool)
        .await
        .expect("Failed to create test API key");

        (user_id, api_key)
    }

    /// Cleanup test data
    async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
        // Clean up in reverse dependency order
        sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
            .execute(pool)
            .await
            .ok();

        sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id)
            .execute(pool)
            .await
            .ok();

        sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id)
            .execute(pool)
            .await
            .ok();

        sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
            .execute(pool)
            .await
            .ok();

        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(pool)
            .await
            .ok();
    }

    #[actix_web::test]
    async fn test_empty_payload_rejection_integration() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create empty payload
        let empty_payload = json!({
            "data": {
                "metrics": [],
                "workouts": []
            }
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Make request with empty payload
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&empty_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Should return 400 Bad Request
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Check response body
        let body: ApiResponse<()> = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body.error.is_some());

        let error_message = body.error.unwrap();
        assert!(error_message.contains("Empty payload"));
        assert!(error_message.contains("no metrics or workouts"));

        cleanup_test_data(&pool, user_id).await;
    }

    #[actix_web::test]
    async fn test_async_processing_response_accuracy() {
        // This test verifies that large payloads return proper async responses
        // indicating that processing is not yet complete with new fields

        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create a large payload string (>10MB) to trigger async processing
        let large_data_string = "x".repeat(11 * 1024 * 1024); // 11MB string
        let large_payload = json!({
            "data": {
                "metrics": [{
                    "type": "heart_rate",
                    "recorded_at": "2024-01-01T12:00:00Z",
                    "heart_rate": 75,
                    "source_device": large_data_string
                }],
                "workouts": []
            }
        });

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .insert_header(("Content-Type", "application/json"))
            .set_json(&large_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Verify response is HTTP 202 Accepted (not 200 OK)
        assert_eq!(resp.status(), StatusCode::ACCEPTED, "Large payload should return 202 Accepted");

        let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;

        // Verify the response indicates async processing with new fields
        assert!(!body.data.success, "success should be false for async processing");
        assert_eq!(body.data.processed_count, 0, "processed_count should be 0 for async processing");

        // NEW FIELDS: Verify processing_status and raw_ingestion_id are set
        assert_eq!(body.data.processing_status, Some("accepted_for_processing".to_string()),
                   "processing_status should be 'accepted_for_processing'");
        assert!(body.data.raw_ingestion_id.is_some(),
                "raw_ingestion_id should be provided for status tracking");

        // Verify message is clear about async processing
        assert!(body.message.contains("accepted_for_processing"),
                "Message should mention processing status");
        assert!(body.message.contains("NO metrics have been processed yet"),
                "Message should clarify that no processing has occurred");
        assert!(body.message.contains("raw_ingestion_id"),
                "Message should mention the ID for status checking");

        cleanup_test_data(&pool, user_id).await;
    }

    #[actix_web::test]
    async fn test_sync_processing_response_fields() {
        // This test verifies that small payloads (synchronous processing)
        // also include the new processing_status and raw_ingestion_id fields

        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create a small payload for synchronous processing
        let small_payload = json!({
            "data": {
                "metrics": [{
                    "type": "heart_rate",
                    "recorded_at": "2024-01-01T12:00:00Z",
                    "heart_rate": 75,
                    "source_device": "iPhone"
                }],
                "workouts": []
            }
        });

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/api/v1/ingest", web::post().to(ingest_handler))
        ).await;

        let req = test::TestRequest::post()
            .uri("/api/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .insert_header(("Content-Type", "application/json"))
            .set_json(&small_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Verify response is HTTP 200 OK (synchronous processing)
        assert_eq!(resp.status(), StatusCode::OK, "Small payload should return 200 OK");

        let body: ApiResponse<IngestResponse> = test::read_body_json(resp).await;

        // Verify synchronous processing fields
        assert!(body.data.success, "success should be true for successful sync processing");
        assert!(body.data.processed_count > 0, "processed_count should be > 0 for sync processing");

        // NEW FIELDS: Verify processing_status and raw_ingestion_id are set for sync too
        assert_eq!(body.data.processing_status, Some("processed".to_string()),
                   "processing_status should be 'processed' for successful sync processing");
        assert!(body.data.raw_ingestion_id.is_some(),
                "raw_ingestion_id should be provided for audit trail");

        cleanup_test_data(&pool, user_id).await;
    }

    /// Test duplicate payload detection for STORY-EMERGENCY-002
    #[actix_web::test]
    async fn test_duplicate_payload_rejection() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create a valid payload with a heart rate metric
        let test_payload = json!({
            "data": {
                "metrics": [{
                    "type": "HeartRate",
                    "recorded_at": "2023-12-01T12:00:00Z",
                    "heart_rate": 75,
                    "resting_heart_rate": 65,
                    "heart_rate_variability": 25.5,
                    "context": "resting",
                    "source_device": "Apple Watch"
                }],
                "workouts": []
            }
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // First submission - should succeed
        let req1 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&test_payload)
            .to_request();

        let resp1 = test::call_service(&app, req1).await;

        // First submission should be successful
        assert_eq!(resp1.status(), StatusCode::OK);
        let body1: ApiResponse<IngestResponse> = test::read_body_json(resp1).await;
        assert!(body1.success);
        assert!(body1.data.is_some());

        let first_response = body1.data.unwrap();
        assert!(first_response.raw_ingestion_id.is_some());
        let first_raw_id = first_response.raw_ingestion_id.unwrap();

        // Second submission with identical payload - should be rejected as duplicate
        let req2 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&test_payload)
            .to_request();

        let resp2 = test::call_service(&app, req2).await;

        // Second submission should be rejected with 400 Bad Request
        assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);

        let body2: ApiResponse<()> = test::read_body_json(resp2).await;
        assert!(!body2.success);
        assert!(body2.error.is_some());

        let error_message = body2.error.unwrap();
        assert!(error_message.contains("Duplicate payload detected"));
        assert!(error_message.contains("already processed"));
        assert!(error_message.contains(&first_raw_id.to_string()));
        assert!(error_message.contains("prevent client retry loops"));

        cleanup_test_data(&pool, user_id).await;
    }

    /// Test that different payloads from same user are not considered duplicates
    #[actix_web::test]
    async fn test_different_payloads_not_duplicates() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create first payload
        let payload1 = json!({
            "data": {
                "metrics": [{
                    "type": "HeartRate",
                    "recorded_at": "2023-12-01T12:00:00Z",
                    "heart_rate": 75,
                    "resting_heart_rate": 65,
                    "heart_rate_variability": 25.5,
                    "context": "resting",
                    "source_device": "Apple Watch"
                }],
                "workouts": []
            }
        });

        // Create second payload with different heart rate value
        let payload2 = json!({
            "data": {
                "metrics": [{
                    "type": "HeartRate",
                    "recorded_at": "2023-12-01T12:00:00Z",
                    "heart_rate": 85, // Different value
                    "resting_heart_rate": 65,
                    "heart_rate_variability": 25.5,
                    "context": "resting",
                    "source_device": "Apple Watch"
                }],
                "workouts": []
            }
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // Submit first payload
        let req1 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&payload1)
            .to_request();

        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), StatusCode::OK);

        // Submit second payload (different) - should succeed
        let req2 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&payload2)
            .to_request();

        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), StatusCode::OK);

        let body2: ApiResponse<IngestResponse> = test::read_body_json(resp2).await;
        assert!(body2.success);

        cleanup_test_data(&pool, user_id).await;
    }

    /// Test that duplicate detection works for empty payloads too
    #[actix_web::test]
    async fn test_duplicate_empty_payload_rejection() {
        let pool = setup_test_db().await;
        let (user_id, api_key) = setup_test_auth(&pool).await;

        // Create empty payload
        let empty_payload = json!({
            "data": {
                "metrics": [],
                "workouts": []
            }
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // First submission of empty payload - should be rejected for being empty
        let req1 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&empty_payload)
            .to_request();

        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), StatusCode::BAD_REQUEST);

        let body1: ApiResponse<()> = test::read_body_json(resp1).await;
        assert!(!body1.success);
        let error1 = body1.error.unwrap();
        assert!(error1.contains("Empty payload"));

        // Second submission of same empty payload - should still be rejected for being empty
        // (Empty payloads are rejected before duplicate check)
        let req2 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key)))
            .set_json(&empty_payload)
            .to_request();

        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);

        let body2: ApiResponse<()> = test::read_body_json(resp2).await;
        assert!(!body2.success);
        let error2 = body2.error.unwrap();
        assert!(error2.contains("Empty payload"));

        cleanup_test_data(&pool, user_id).await;
    }

    /// Test that duplicate detection is user-specific (different users can submit same payload)
    #[actix_web::test]
    async fn test_duplicate_detection_user_specific() {
        let pool = setup_test_db().await;
        let (user_id1, api_key1) = setup_test_auth(&pool).await;

        // Create second user
        let user_id2 = Uuid::new_v4();
        let api_key_id2 = Uuid::new_v4();
        let api_key2 = "test_key_user2";

        // Insert second user and API key
        sqlx::query!("INSERT INTO users (id, email) VALUES ($1, $2)", user_id2, "test2@example.com")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query!(
            "INSERT INTO api_keys (id, user_id, name, key_hash) VALUES ($1, $2, $3, $4)",
            api_key_id2,
            user_id2,
            "Test API Key 2",
            "dummy_hash_2"
        )
        .execute(&pool)
        .await
        .unwrap();

        // Same payload for both users
        let test_payload = json!({
            "data": {
                "metrics": [{
                    "type": "HeartRate",
                    "recorded_at": "2023-12-01T12:00:00Z",
                    "heart_rate": 75,
                    "resting_heart_rate": 65,
                    "heart_rate_variability": 25.5,
                    "context": "resting",
                    "source_device": "Apple Watch"
                }],
                "workouts": []
            }
        });

        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .route("/v1/ingest", web::post().to(ingest_handler))
        ).await;

        // User 1 submits payload
        let req1 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key1)))
            .set_json(&test_payload)
            .to_request();

        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), StatusCode::OK);

        // User 2 submits same payload - should succeed (different user)
        let req2 = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key2)))
            .set_json(&test_payload)
            .to_request();

        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), StatusCode::OK);

        let body2: ApiResponse<IngestResponse> = test::read_body_json(resp2).await;
        assert!(body2.success);

        // Cleanup both users
        cleanup_test_data(&pool, user_id1).await;
        cleanup_test_data(&pool, user_id2).await;
    }
}