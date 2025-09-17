// STORY-EMERGENCY-001: Integration Tests for API Status Reporting False Positives Fix
// Testing the enhanced update_processing_status() function to prevent false positives

use actix_web::{test, web, App};
use serde_json::json;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::ingest_handler,
    models::{ApiResponse, IngestResponse},
    services::auth::AuthService,
    middleware::auth::AuthMiddleware,
};

// Import test utilities
mod common;
use common::{get_test_pool, setup_test_user_and_key, cleanup_test_data};

/// Test that comprehensive metadata is stored for processing status tracking
#[actix_web::test]
async fn test_status_metadata_comprehensive_tracking() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "metadata_test@example.com").await;

    // Test with a small valid payload
    let test_payload = json!({
        "data": {
            "metrics": [
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:00:00Z",
                    "value": 75,
                    "unit": "bpm",
                    "source": "Apple Watch"
                },
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:01:00Z",
                    "value": 78,
                    "unit": "bpm",
                    "source": "Apple Watch"
                }
            ],
            "workouts": []
        }
    });

    // Create test app with middleware
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    // Make request
    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(test_payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Parse response
    let body = test::read_body(resp).await;
    let api_response: ApiResponse<IngestResponse> = serde_json::from_slice(&body).unwrap();
    let ingest_response = api_response.data.unwrap();

    // Verify response includes raw_ingestion_id
    assert!(ingest_response.raw_ingestion_id.is_some());
    let raw_id = ingest_response.raw_ingestion_id.unwrap();

    // Check database record for comprehensive metadata
    let record = sqlx::query!(
        "SELECT processing_status, processing_errors FROM raw_ingestions WHERE id = $1",
        raw_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Verify processing status is correctly determined
    assert_eq!(record.processing_status.unwrap(), "processed");

    // Verify processing_errors contains comprehensive metadata
    if let Some(processing_errors) = record.processing_errors {
        let metadata = processing_errors.get("processing_metadata");
        assert!(metadata.is_some(), "processing_metadata should be present");

        let metadata = metadata.unwrap();

        // Verify all required tracking fields are present
        assert!(metadata.get("expected_count").is_some());
        assert!(metadata.get("actual_count").is_some());
        assert!(metadata.get("failed_count").is_some());
        assert!(metadata.get("silent_failures").is_some());
        assert!(metadata.get("loss_percentage").is_some());
        assert!(metadata.get("has_silent_failures").is_some());
        assert!(metadata.get("significant_loss").is_some());
        assert!(metadata.get("postgresql_param_limit_violation").is_some());
        assert!(metadata.get("analysis_timestamp").is_some());

        // Verify detection logic thresholds are documented
        let detection_logic = metadata.get("detection_logic").unwrap();
        assert_eq!(detection_logic["silent_failure_threshold"], 1);
        assert_eq!(detection_logic["loss_percentage_threshold"], 1.0);
        assert_eq!(detection_logic["param_limit_threshold"], 50);

        // For this successful case, verify no data loss detected
        assert_eq!(metadata["expected_count"], 2); // 2 heart rate metrics
        assert_eq!(metadata["actual_count"], 2);   // All processed
        assert_eq!(metadata["silent_failures"], 0); // No silent failures
        assert_eq!(metadata["loss_percentage"], 0.0); // No loss
        assert_eq!(metadata["has_silent_failures"], false);
        assert_eq!(metadata["significant_loss"], false);
        assert_eq!(metadata["postgresql_param_limit_violation"], false);
    } else {
        panic!("processing_errors should contain metadata");
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test status reporting for empty payloads (STORY-EMERGENCY-002 related)
#[actix_web::test]
async fn test_empty_payload_rejection() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "empty_payload_test@example.com").await;

    // Test with empty payload
    let empty_payload = json!({
        "data": {
            "metrics": [],
            "workouts": []
        }
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(empty_payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should be rejected with 400 Bad Request
    assert_eq!(resp.status(), 400);

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Empty payload"));
    assert!(body_str.contains("no metrics or workouts provided"));

    cleanup_test_data(&pool, user_id).await;
}

/// Test async processing status response (STORY-EMERGENCY-003 related)
#[actix_web::test]
async fn test_large_payload_async_processing_response() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "large_payload_test@example.com").await;

    // Create a large payload (>10MB) by repeating metrics
    let single_metric = json!({
        "type": "HeartRate",
        "timestamp": "2025-01-15T10:00:00Z",
        "value": 75,
        "unit": "bpm",
        "source": "Apple Watch"
    });

    // Create a large array to exceed 10MB threshold
    let large_metrics = vec![single_metric; 100000]; // Should exceed 10MB

    let large_payload = json!({
        "data": {
            "metrics": large_metrics,
            "workouts": []
        }
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(large_payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should be accepted with 202 Accepted for async processing
    assert_eq!(resp.status(), 202);

    let body = test::read_body(resp).await;
    let api_response: ApiResponse<IngestResponse> = serde_json::from_slice(&body).unwrap();
    let ingest_response = api_response.data.unwrap();

    // Verify async processing response fields (STORY-EMERGENCY-003 fix)
    assert_eq!(ingest_response.processed_count, 0); // No metrics processed yet
    assert_eq!(ingest_response.failed_count, 0);    // No failures yet
    assert!(ingest_response.processing_status.is_some());
    assert_eq!(ingest_response.processing_status.unwrap(), "accepted_for_processing");
    assert!(ingest_response.raw_ingestion_id.is_some());

    // Verify the response message is clear about async nature
    let message = api_response.message.unwrap();
    assert!(message.contains("background processing"));
    assert!(message.contains("status API"));

    cleanup_test_data(&pool, user_id).await;
}

/// Test duplicate payload detection (STORY-EMERGENCY-002 related)
#[actix_web::test]
async fn test_duplicate_payload_detection() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "duplicate_test@example.com").await;

    let test_payload = json!({
        "data": {
            "metrics": [
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:00:00Z",
                    "value": 75,
                    "unit": "bpm",
                    "source": "Apple Watch"
                }
            ],
            "workouts": []
        }
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    // First request - should succeed
    let req1 = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(test_payload.to_string())
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert!(resp1.status().is_success());

    let body1 = test::read_body(resp1).await;
    let api_response1: ApiResponse<IngestResponse> = serde_json::from_slice(&body1).unwrap();
    let first_raw_id = api_response1.data.unwrap().raw_ingestion_id.unwrap();

    // Second identical request - should be rejected as duplicate
    let req2 = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(test_payload.to_string())
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 400); // Should be rejected

    let body2 = test::read_body(resp2).await;
    let body_str = String::from_utf8(body2.to_vec()).unwrap();
    assert!(body_str.contains("Duplicate payload detected"));
    assert!(body_str.contains(&first_raw_id.to_string()));

    cleanup_test_data(&pool, user_id).await;
}

/// Simulate a scenario where PostgreSQL parameter limits would be violated
/// This tests the data loss detection logic in update_processing_status
#[actix_web::test]
async fn test_parameter_limit_violation_detection() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "param_limit_test@example.com").await;

    // Create a payload with many metrics that could trigger parameter limits
    let mut large_metrics = Vec::new();
    for i in 0..1000 {
        large_metrics.push(json!({
            "type": "HeartRate",
            "timestamp": format!("2025-01-15T{:02}:{:02}:00Z", i / 60, i % 60),
            "value": 70 + (i % 30),
            "unit": "bpm",
            "source": "Apple Watch"
        }));
    }

    let large_payload = json!({
        "data": {
            "metrics": large_metrics,
            "workouts": []
        }
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(large_payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let api_response: ApiResponse<IngestResponse> = serde_json::from_slice(&body).unwrap();
    let ingest_response = api_response.data.unwrap();
    let raw_id = ingest_response.raw_ingestion_id.unwrap();

    // Check that processing metadata was properly recorded
    let record = sqlx::query!(
        "SELECT processing_status, processing_errors FROM raw_ingestions WHERE id = $1",
        raw_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    if let Some(processing_errors) = record.processing_errors {
        let metadata = processing_errors.get("processing_metadata").unwrap();

        // Verify that our data loss detection logic is working
        let expected_count = metadata["expected_count"].as_u64().unwrap();
        let actual_count = metadata["actual_count"].as_u64().unwrap();
        let silent_failures = metadata["silent_failures"].as_u64().unwrap();
        let loss_percentage = metadata["loss_percentage"].as_f64().unwrap();

        // Log the results for verification
        println!("Expected: {}, Actual: {}, Silent failures: {}, Loss: {}%",
            expected_count, actual_count, silent_failures, loss_percentage);

        // The status should reflect whether there was any data loss
        let status = record.processing_status.unwrap();
        if silent_failures > 0 {
            assert!(status == "error" || status == "partial_success");
        } else {
            assert_eq!(status, "processed");
        }
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test that validation errors are properly counted and don't cause false positives
#[actix_web::test]
async fn test_validation_errors_vs_data_loss() {
    let pool = get_test_pool().await;
    let (user_id, api_key) = setup_test_user_and_key(&pool, "validation_test@example.com").await;

    // Create payload with some invalid data that should be filtered out during validation
    let mixed_payload = json!({
        "data": {
            "metrics": [
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:00:00Z",
                    "value": 75, // Valid
                    "unit": "bpm",
                    "source": "Apple Watch"
                },
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:01:00Z",
                    "value": 350, // Invalid - out of range
                    "unit": "bpm",
                    "source": "Apple Watch"
                },
                {
                    "type": "HeartRate",
                    "timestamp": "2025-01-15T10:02:00Z",
                    "value": 80, // Valid
                    "unit": "bpm",
                    "source": "Apple Watch"
                }
            ],
            "workouts": []
        }
    });

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new())
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header(("authorization", format!("Bearer {}", api_key)))
        .insert_header(("content-type", "application/json"))
        .set_payload(mixed_payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let api_response: ApiResponse<IngestResponse> = serde_json::from_slice(&body).unwrap();
    let ingest_response = api_response.data.unwrap();

    // Should have processed 2 valid metrics and failed 1 invalid metric
    assert_eq!(ingest_response.processed_count, 2);
    assert_eq!(ingest_response.failed_count, 1);
    assert!(ingest_response.errors.len() > 0);

    // Status should be partial_success (not error) because validation failures are accounted for
    assert_eq!(ingest_response.processing_status.unwrap(), "partial_success");

    cleanup_test_data(&pool, user_id).await;
}