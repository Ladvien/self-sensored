use actix_web::{test, web, App};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;

use self_sensored::db::database::create_connection_pool;
// Note: Environmental handlers are not implemented in current codebase
// This test file contains examples for future implementation
// use self_sensored::models::{EnvironmentalMetric, AudioExposureMetric, SafetyEventMetric};
use self_sensored::services::auth::{AuthContext};
use self_sensored::db::models::User;

// Test helper functions
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .expect("DATABASE_URL or TEST_DATABASE_URL must be set for testing");

    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database connection pool")
}

async fn create_test_user(pool: &PgPool) -> User {
    let user_id = uuid::Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        email
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    User {
        id: user_id,
        email,
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: Some(serde_json::Value::Object(serde_json::Map::new())),
    }
}

fn create_auth_context(user: User) -> AuthContext {
    use self_sensored::services::auth::{User as AuthUser, ApiKey as AuthApiKey};

    AuthContext {
        user: AuthUser {
            id: user.id,
            email: user.email,
            apple_health_id: user.apple_health_id,
            created_at: user.created_at,
            updated_at: user.updated_at,
            is_active: user.is_active,
            metadata: user.metadata,
        },
        api_key: AuthApiKey {
            id: uuid::Uuid::new_v4(),
            user_id: user.id,
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

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_ingest_environmental_handler_success() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create test environmental metrics
    let environmental_metrics = vec![EnvironmentalMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        uv_index: Some(8.5),
        uv_exposure_minutes: Some(30),
        time_in_daylight_minutes: Some(480), // 8 hours
        ambient_temperature_celsius: Some(25.0),
        humidity_percent: Some(65.0),
        air_pressure_hpa: Some(1013.25),
        altitude_meters: Some(100.0),
        location_latitude: Some(40.7128),
        location_longitude: Some(-74.0060), // New York coordinates
        source_device: Some("iPhone".to_string()),
        created_at: Utc::now(),
    }];

    let payload = EnvironmentalIngestPayload {
        data: environmental_metrics,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/environmental", web::post().to(ingest_environmental_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/environmental")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert response
    assert!(resp.status().is_success(), "Expected success response");

    // Verify data was stored in database
    let stored_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query database")
    .count
    .unwrap_or(0);

    assert_eq!(stored_count, 1, "Expected one environmental metric to be stored");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_ingest_audio_exposure_handler_dangerous_levels() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create test audio exposure metrics with dangerous levels
    let audio_metrics = vec![AudioExposureMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        environmental_audio_exposure_db: Some(95.0), // Above safe threshold
        headphone_audio_exposure_db: Some(90.0), // Above safe threshold
        exposure_duration_minutes: 120, // 2 hours
        audio_exposure_event: true, // Should be true for high levels
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    }];

    let payload = AudioExposureIngestPayload {
        data: audio_metrics,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/audio-exposure", web::post().to(ingest_audio_exposure_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/audio-exposure")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert response
    assert!(resp.status().is_success(), "Expected success response");

    // Verify dangerous levels were logged (would need log capturing for full verification)
    // For now, just verify data storage
    let stored_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1 AND (environmental_audio_exposure_db IS NOT NULL OR headphone_audio_exposure_db IS NOT NULL)",
        user.id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query database")
    .count
    .unwrap_or(0);

    assert_eq!(stored_count, 1, "Expected one audio exposure metric to be stored");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_ingest_safety_events_handler_critical_events() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create test safety event metrics
    let safety_events = vec![SafetyEventMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        event_type: "fall_detected".to_string(),
        severity_level: Some(4), // High severity
        location_latitude: Some(40.7128),
        location_longitude: Some(-74.0060),
        emergency_contacts_notified: true,
        resolution_status: Some("pending".to_string()),
        notes: Some("Hard fall detected during workout".to_string()),
        source_device: Some("Apple Watch Series 9".to_string()),
        created_at: Utc::now(),
    }];

    let payload = SafetyEventIngestPayload {
        data: safety_events,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/safety-events", web::post().to(ingest_safety_events_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/safety-events")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert response
    assert!(resp.status().is_success(), "Expected success response");

    // Verify safety event was stored (in health_symptoms table for demo)
    let stored_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM health_symptoms WHERE user_id = $1 AND symptom_type = 'fall_detected'",
        user.id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to query database")
    .count
    .unwrap_or(0);

    assert_eq!(stored_count, 1, "Expected one safety event to be stored");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_get_environmental_data_handler() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Insert test environmental data directly to database
    sqlx::query!(
        r#"
        INSERT INTO environmental_metrics (
            id, user_id, recorded_at, uv_index, time_in_daylight_minutes,
            ambient_temperature_celsius, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        uuid::Uuid::new_v4(),
        user.id,
        Utc::now() - chrono::Duration::hours(2),
        6.5,
        300, // 5 hours
        22.5,
        "iPhone 15",
        Utc::now()
    )
    .execute(&pool)
    .await
    .expect("Failed to insert test data");

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/environmental", web::get().to(get_environmental_data_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::get()
        .uri("/environmental?limit=10")
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert response
    assert!(resp.status().is_success(), "Expected success response");

    // Parse response body
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Verify response structure
    assert!(body["data"].is_array(), "Expected data array in response");
    assert!(body["total_count"].is_number(), "Expected total_count in response");
    assert!(body["has_more"].is_boolean(), "Expected has_more in response");

    let data_array = body["data"].as_array().unwrap();
    assert_eq!(data_array.len(), 1, "Expected one environmental record");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_environmental_validation_errors() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create invalid environmental metric (UV index out of range)
    let invalid_metrics = vec![EnvironmentalMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        uv_index: Some(25.0), // Invalid: UV index should be 0-20
        uv_exposure_minutes: Some(-10), // Invalid: negative minutes
        time_in_daylight_minutes: Some(1500), // Invalid: more than 24 hours
        ambient_temperature_celsius: Some(70.0), // Invalid: too hot
        humidity_percent: Some(150.0), // Invalid: over 100%
        air_pressure_hpa: None,
        altitude_meters: None,
        location_latitude: Some(100.0), // Invalid: outside lat range
        location_longitude: Some(200.0), // Invalid: outside lng range
        source_device: Some("Test Device".to_string()),
        created_at: Utc::now(),
    }];

    let payload = EnvironmentalIngestPayload {
        data: invalid_metrics,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/environmental", web::post().to(ingest_environmental_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/environmental")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert validation errors
    assert!(resp.status().is_success(), "Should return success with validation errors");

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should have validation errors
    assert_eq!(body["processed_count"], 0, "No metrics should be processed");
    assert!(body["failed_count"].as_u64().unwrap() > 0, "Should have failed records");
    assert!(body["errors"].is_array(), "Should have error details");

    let errors = body["errors"].as_array().unwrap();
    assert!(!errors.is_empty(), "Should have validation error messages");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_audio_exposure_validation() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create invalid audio exposure metric
    let invalid_audio_metrics = vec![AudioExposureMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        environmental_audio_exposure_db: Some(200.0), // Invalid: too high
        headphone_audio_exposure_db: Some(-10.0), // Invalid: negative
        exposure_duration_minutes: -30, // Invalid: negative duration
        audio_exposure_event: false, // Inconsistent with high DB levels
        source_device: Some("Test Device".to_string()),
        created_at: Utc::now(),
    }];

    let payload = AudioExposureIngestPayload {
        data: invalid_audio_metrics,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/audio-exposure", web::post().to(ingest_audio_exposure_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/audio-exposure")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert validation errors
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["processed_count"], 0, "No invalid metrics should be processed");
    assert!(body["failed_count"].as_u64().unwrap() > 0, "Should have validation failures");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_safety_event_validation() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create invalid safety event
    let invalid_safety_events = vec![SafetyEventMetric {
        id: uuid::Uuid::new_v4(),
        user_id: user.id,
        recorded_at: Utc::now(),
        event_type: "".to_string(), // Invalid: empty event type
        severity_level: Some(10), // Invalid: out of range (1-5)
        location_latitude: Some(100.0), // Invalid: outside range
        location_longitude: Some(200.0), // Invalid: outside range
        emergency_contacts_notified: false,
        resolution_status: Some("unknown".to_string()),
        notes: None,
        source_device: Some("Test Device".to_string()),
        created_at: Utc::now(),
    }];

    let payload = SafetyEventIngestPayload {
        data: invalid_safety_events,
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/safety-events", web::post().to(ingest_safety_events_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/safety-events")
        .set_json(&payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Assert validation errors
    let body: serde_json::Value = test::read_body_json(resp).await;

    assert_eq!(body["processed_count"], 0, "No invalid events should be processed");
    assert!(body["failed_count"].as_u64().unwrap() > 0, "Should have validation failures");
}

#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_empty_payload_rejection() {
    let pool = create_test_pool().await;
    // Create test user
    let user = create_test_user(&pool).await;
    let auth = create_auth_context(user.clone());

    // Create empty payload
    let empty_payload = EnvironmentalIngestPayload {
        data: vec![],
    };

    // Setup test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/environmental", web::post().to(ingest_environmental_handler))
    ).await;

    // Create test request
    let req = test::TestRequest::post()
        .uri("/environmental")
        .set_json(&empty_payload)
        .to_request();

    // Execute request
    let resp = test::call_service(&app, req).await;

    // Should return bad request for empty payload
    assert_eq!(resp.status(), 400, "Empty payload should return 400 Bad Request");
}

// Integration test with iOS data parsing
#[tokio::test]
#[ignore = "Environmental handlers not yet implemented"]
async fn test_ios_environmental_data_conversion() {
    let pool = create_test_pool().await;
    use self_sensored::models::ios_models::{IosIngestPayload, IosIngestData, IosMetric, IosMetricData};

    // Create test user
    let user = create_test_user(&pool).await;

    // Create iOS-style environmental data payload
    let ios_payload = IosIngestPayload {
        data: IosIngestData {
            metrics: vec![
                IosMetric {
                    name: "uv_exposure".to_string(),
                    units: Some("UV Index".to_string()),
                    data: vec![IosMetricData {
                        source: Some("iPhone".to_string()),
                        date: Some("2025-01-15 14:30:00 +0000".to_string()),
                        start: None,
                        end: None,
                        qty: Some(7.5),
                        value: None,
                        extra: {
                            let mut map = HashMap::new();
                            map.insert("latitude".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(40.7128).unwrap()));
                            map.insert("longitude".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(-74.0060).unwrap()));
                            map
                        },
                    }],
                },
                IosMetric {
                    name: "time_in_daylight".to_string(),
                    units: Some("minutes".to_string()),
                    data: vec![IosMetricData {
                        source: Some("Apple Watch".to_string()),
                        date: Some("2025-01-15 14:30:00 +0000".to_string()),
                        start: None,
                        end: None,
                        qty: Some(420.0), // 7 hours
                        value: None,
                        extra: HashMap::new(),
                    }],
                },
                IosMetric {
                    name: "environmental_audio_exposure".to_string(),
                    units: Some("dB".to_string()),
                    data: vec![IosMetricData {
                        source: Some("Apple Watch".to_string()),
                        date: Some("2025-01-15 14:30:00 +0000".to_string()),
                        start: None,
                        end: None,
                        qty: Some(88.5), // Above safe threshold
                        value: None,
                        extra: {
                            let mut map = HashMap::new();
                            map.insert("duration_minutes".to_string(), serde_json::Value::Number(serde_json::Number::from(90)));
                            map
                        },
                    }],
                },
            ],
            workouts: vec![],
        },
    };

    // Convert iOS payload to internal format
    let internal_payload = ios_payload.to_internal_format(user.id);

    // Verify conversion results
    let environmental_metrics: Vec<_> = internal_payload.data.metrics
        .iter()
        .filter(|metric| matches!(metric, self_sensored::models::HealthMetric::Environmental(_)))
        .collect();

    let audio_metrics: Vec<_> = internal_payload.data.metrics
        .iter()
        .filter(|metric| matches!(metric, self_sensored::models::HealthMetric::AudioExposure(_)))
        .collect();

    assert_eq!(environmental_metrics.len(), 2, "Should have 2 environmental metrics (UV + daylight)");
    assert_eq!(audio_metrics.len(), 1, "Should have 1 audio exposure metric");

    // Test that the conversion preserved GPS coordinates
    if let self_sensored::models::HealthMetric::Environmental(env_metric) = &environmental_metrics[0] {
        assert_eq!(env_metric.location_latitude, Some(40.7128), "Should preserve latitude");
        assert_eq!(env_metric.location_longitude, Some(-74.0060), "Should preserve longitude");
    }

    // Test that audio exposure event was correctly detected
    if let self_sensored::models::HealthMetric::AudioExposure(audio_metric) = &audio_metrics[0] {
        assert!(audio_metric.audio_exposure_event, "Should detect dangerous audio level");
        assert_eq!(audio_metric.environmental_audio_exposure_db, Some(88.5), "Should preserve dB level");
    }
}