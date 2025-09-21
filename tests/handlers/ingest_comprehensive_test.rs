use actix_web::{test, web, App, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::ingest_handler,
    models::{
        health_metrics::{
            HealthMetric, HeartRateMetric, BloodPressureMetric, SleepMetric,
            ActivityMetric, WorkoutData, IngestPayload, IngestData, IngestResponse,
        },
        ios_models::{IosIngestPayload, IosIngestData, IosMetric, IosMetricData, IosWorkout},
        enums::{ActivityContext, WorkoutType}
    },
    services::auth::{AuthContext, User as AuthUser, ApiKey as AuthApiKey, AuthService},
    db::models::{User, ApiKey},
    middleware::metrics::Metrics,
};

mod common;
use common::{setup_test_db, cleanup_test_data, create_test_heart_rate_metric, create_test_blood_pressure_metric, create_test_sleep_metric, create_test_activity_metric, create_test_workout_metric};

/// Test fixtures for various payload formats and scenarios
pub struct IngestTestFixtures;

impl IngestTestFixtures {
    /// Create a standard format payload with comprehensive health data
    pub fn standard_payload_comprehensive(user_id: Uuid) -> IngestPayload {
        let now = Utc::now();
        
        IngestPayload {
            data: IngestData {
                metrics: vec![
                    // Heart rate metrics
                    HealthMetric::HeartRate(HeartRateMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now,
                        heart_rate: Some(75),
                        resting_heart_rate: Some(65),
                        heart_rate_variability: Some(45.0),
                        walking_heart_rate_average: None,
                        heart_rate_recovery_one_minute: None,
                        atrial_fibrillation_burden_percentage: None,
                        vo2_max_ml_kg_min: None,
                        context: Some(ActivityContext::Resting),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    HealthMetric::HeartRate(HeartRateMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now - chrono::Duration::hours(1),
                        heart_rate: Some(120),
                        resting_heart_rate: None,
                        heart_rate_variability: Some(35.2),
                        walking_heart_rate_average: None,
                        heart_rate_recovery_one_minute: None,
                        atrial_fibrillation_burden_percentage: None,
                        vo2_max_ml_kg_min: None,
                        context: Some(ActivityContext::Exercise),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    // Blood pressure metric
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now - chrono::Duration::hours(2),
                        systolic: 120,
                        diastolic: 80,
                        pulse: Some(70),
                        source_device: Some("Blood Pressure Monitor".to_string()),
                        created_at: now,
                    }),
                    // Sleep metric
                    HealthMetric::Sleep(SleepMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        sleep_start: now - chrono::Duration::hours(10),
                        sleep_end: now - chrono::Duration::hours(2),
                        duration_minutes: Some(480),
                        deep_sleep_minutes: Some(120),
                        rem_sleep_minutes: Some(100),
                        light_sleep_minutes: Some(240),
                        awake_minutes: Some(20),
                        efficiency: Some(95.0),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    // Activity metric  
                    HealthMetric::Activity(ActivityMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
                        step_count: Some(10000),
                        distance_meters: Some(8000.0),
                        flights_climbed: Some(10),
                        active_energy_burned_kcal: Some(400.0),
                        basal_energy_burned_kcal: Some(1600.0),
                        distance_cycling_meters: None,
                        distance_swimming_meters: None,
                        distance_wheelchair_meters: None,
                        distance_downhill_snow_sports_meters: None,
                        push_count: None,
                        swimming_stroke_count: None,
                        nike_fuel_points: None,
                        apple_exercise_time_minutes: Some(30),
                        apple_stand_time_minutes: Some(12),
                        apple_move_time_minutes: Some(45),
                        apple_stand_hour_achieved: Some(true),
                        walking_speed_m_per_s: None,
                        walking_step_length_cm: None,
                        walking_asymmetry_percent: None,
                        walking_double_support_percent: None,
                        six_minute_walk_test_distance_m: None,
                        stair_ascent_speed_m_per_s: None,
                        stair_descent_speed_m_per_s: None,
                        ground_contact_time_ms: None,
                        vertical_oscillation_cm: None,
                        running_stride_length_m: None,
                        running_power_watts: None,
                        running_speed_m_per_s: None,
                        cycling_speed_kmh: None,
                        cycling_power_watts: None,
                        cycling_cadence_rpm: None,
                        functional_threshold_power_watts: None,
                        underwater_depth_meters: None,
                        diving_duration_seconds: None,
                        source_device: Some("iPhone".to_string()),
                        created_at: now,
                    }),
                ],
                workouts: vec![
                    WorkoutData {
                        id: Uuid::new_v4(),
                        user_id,
                        workout_type: WorkoutType::Running,
                        started_at: now - chrono::Duration::hours(3),
                        ended_at: now - chrono::Duration::hours(2) - chrono::Duration::minutes(30),
                        total_energy_kcal: Some(500.0),
                        active_energy_kcal: Some(450.0),
                        distance_meters: Some(5000.0),
                        avg_heart_rate: Some(145),
                        max_heart_rate: Some(175),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }
                ],
            },
        }
    }

    /// Create an iOS format payload with basic metrics
    pub fn ios_payload_basic(user_id: Uuid) -> IosIngestPayload {
        let now = Utc::now();
        
        IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HeartRate".to_string(),
                        units: Some("BPM".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(72.0),
                                date: Some(now.to_rfc3339()),
                                start: None,
                                end: None,
                                source: Some("Apple Watch".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            }
                        ],
                    },
                    IosMetric {
                        name: "BloodPressureSystolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(120.0),
                                date: Some((now - chrono::Duration::hours(1)).to_rfc3339()),
                                start: None,
                                end: None,
                                source: Some("Blood Pressure Monitor".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            }
                        ],
                    },
                ],
                workouts: vec![
                    IosWorkout {
                        name: "Running".to_string(),
                        start: (now - chrono::Duration::hours(2)).to_rfc3339(),
                        end: (now - chrono::Duration::hours(1) - chrono::Duration::minutes(30)).to_rfc3339(),
                        total_energy: Some(400.0),
                        total_distance: Some(5000.0),
                        source: Some("Apple Watch".to_string()),
                        extra: HashMap::new(),
                    }
                ],
            },
        }
    }

    /// Create a payload with validation errors
    pub fn payload_with_validation_errors(user_id: Uuid) -> IngestPayload {
        let now = Utc::now();
        
        IngestPayload {
            data: IngestData {
                metrics: vec![
                    // Invalid heart rate - too high
                    HealthMetric::HeartRate(HeartRateMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now,
                        heart_rate: Some(350), // Invalid - over 300
                        resting_heart_rate: Some(65),
                        heart_rate_variability: Some(45.0),
                        walking_heart_rate_average: None,
                        heart_rate_recovery_one_minute: None,
                        atrial_fibrillation_burden_percentage: None,
                        vo2_max_ml_kg_min: None,
                        context: Some(ActivityContext::Resting),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    // Invalid blood pressure - too low
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now,
                        systolic: 20, // Invalid - too low
                        diastolic: 10, // Invalid - too low
                        pulse: Some(70),
                        source_device: Some("Blood Pressure Monitor".to_string()),
                        created_at: now,
                    }),
                    // Valid metric to ensure partial processing works
                    HealthMetric::HeartRate(HeartRateMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        recorded_at: now - chrono::Duration::hours(1),
                        heart_rate: Some(75), // Valid
                        resting_heart_rate: Some(65),
                        heart_rate_variability: Some(45.0),
                        walking_heart_rate_average: None,
                        heart_rate_recovery_one_minute: None,
                        atrial_fibrillation_burden_percentage: None,
                        vo2_max_ml_kg_min: None,
                        context: Some(ActivityContext::Resting),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                ],
                workouts: vec![
                    // Invalid workout - end before start
                    WorkoutData {
                        id: Uuid::new_v4(),
                        user_id,
                        workout_type: WorkoutType::Running,
                        started_at: now,
                        ended_at: now - chrono::Duration::hours(1), // Invalid - end before start
                        total_energy_kcal: Some(500.0),
                        active_energy_kcal: Some(450.0),
                        distance_meters: Some(5000.0),
                        avg_heart_rate: Some(145),
                        max_heart_rate: Some(175),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }
                ],
            },
        }
    }

    /// Create an empty payload
    pub fn empty_payload() -> IngestPayload {
        IngestPayload {
            data: IngestData {
                metrics: vec![],
                workouts: vec![],
            },
        }
    }

    /// Create a large payload for testing performance
    pub fn large_payload(user_id: Uuid, metric_count: usize) -> IngestPayload {
        let now = Utc::now();
        let mut metrics = Vec::new();
        
        for i in 0..metric_count {
            metrics.push(HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: now - chrono::Duration::minutes(i as i64),
                heart_rate: Some(70 + (i % 50) as i32), // Vary between 70-120
                resting_heart_rate: Some(60),
                heart_rate_variability: Some(45.0),
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                context: Some("resting".to_string()),
                source_device: Some("Apple Watch".to_string()),
                created_at: now,
            }));
        }
        
        IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        }
    }
}

/// Helper function to create test user and API key
async fn create_test_user_and_key(pool: &PgPool) -> (User, ApiKey, AuthContext) {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    
    // Create test user
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW()) RETURNING *",
        user_id,
        format!("test-user-{}@example.com", user_id)
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user");
    
    // Create test API key (using a dummy hash since we're bypassing auth middleware)
    let api_key = sqlx::query_as!(
        ApiKey,
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW()) RETURNING *",
        api_key_id,
        user_id,
        "dummy_hash_for_testing",
        "Test API Key"
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test API key");
    
    let auth_context = AuthContext {
        user: AuthUser {
            id: user.id,
            email: user.email.clone(),
            created_at: user.created_at,
        },
        api_key: AuthApiKey {
            id: api_key.id,
            user_id: api_key.user_id,
            name: api_key.name.clone(),
            is_active: api_key.is_active,
            created_at: api_key.created_at,
            expires_at: api_key.expires_at,
            permissions: api_key.permissions.clone(),
            rate_limit_per_hour: api_key.rate_limit_per_hour,
        },
    };
    
    (user, api_key, auth_context)
}

#[sqlx::test]
async fn test_ingest_standard_payload_success(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::standard_payload_comprehensive(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Verify data was stored in database
    let heart_rate_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(heart_rate_count, 2, "Should have stored 2 heart rate metrics");
    
    let blood_pressure_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(blood_pressure_count, 1, "Should have stored 1 blood pressure metric");
    
    let workout_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workouts WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(workout_count, 1, "Should have stored 1 workout");
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_ios_payload_success(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let ios_payload = IngestTestFixtures::ios_payload_basic(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&ios_payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Verify iOS payload was converted and stored
    let heart_rate_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(heart_rate_count, 1, "Should have stored 1 heart rate metric from iOS payload");
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_validation_errors_partial_success(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::payload_with_validation_errors(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200); // Partial success still returns 200
    
    // Only valid metrics should be stored
    let heart_rate_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(heart_rate_count, 1, "Should have stored only 1 valid heart rate metric");
    
    let blood_pressure_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(blood_pressure_count, 0, "Should not have stored invalid blood pressure metric");
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_empty_payload_error(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::empty_payload();
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400); // Should reject empty payload
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_malformed_json_error(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    
    let malformed_json = b"{\"data\": {\"metrics\": [}}";
    let payload_bytes = web::Bytes::from(malformed_json.to_vec());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400); // Should reject malformed JSON
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_duplicate_payload_detection(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::standard_payload_comprehensive(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    // First request should succeed
    let result1 = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context.clone(),
        payload_bytes.clone(),
        req.clone(),
    )
    .await;
    
    assert!(result1.is_ok());
    let response1 = result1.unwrap();
    assert_eq!(response1.status(), 200);
    
    // Second identical request should be rejected as duplicate
    let result2 = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result2.is_ok());
    let response2 = result2.unwrap();
    assert_eq!(response2.status(), 400); // Should reject duplicate
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_large_payload_async_processing(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    
    // Create a large payload that should trigger async processing (>10MB)
    let large_metric_count = 50000; // This should create a payload > 10MB
    let payload = IngestTestFixtures::large_payload(user.id, large_metric_count);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    // Large payloads should return 202 Accepted for async processing
    assert_eq!(response.status(), 202);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_missing_user_id_consistency(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let mut payload = IngestTestFixtures::standard_payload_comprehensive(user.id);
    
    // Change user_id in one metric to a different ID (inconsistent with auth context)
    if let Some(HealthMetric::HeartRate(hr_metric)) = payload.data.metrics.first_mut() {
        hr_metric.user_id = Uuid::new_v4(); // Different user ID
    }
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    // Should still process - batch processor corrects user_id based on auth context
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_raw_payload_storage(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::standard_payload_comprehensive(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    assert!(result.is_ok());
    
    // Verify raw payload was stored
    let raw_ingestion_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM raw_ingestions WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(raw_ingestion_count, 1, "Should have stored raw payload for audit");
    
    // Verify processing status was updated
    let processing_status = sqlx::query_scalar!(
        "SELECT processing_status FROM raw_ingestions WHERE user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert!(processing_status.is_some());
    assert!(matches!(processing_status.as_deref(), Some("processed") | Some("partial_success")));
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_ingest_performance_metrics(pool: PgPool) {
    let (user, _api_key, auth_context) = create_test_user_and_key(&pool).await;
    let payload = IngestTestFixtures::standard_payload_comprehensive(user.id);
    
    let payload_bytes = web::Bytes::from(serde_json::to_vec(&payload).unwrap());
    
    let req = test::TestRequest::post()
        .uri("/api/v1/ingest")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let start_time = std::time::Instant::now();
    
    let result = ingest_handler(
        web::Data::new(pool.clone()),
        auth_context,
        payload_bytes,
        req,
    )
    .await;
    
    let processing_time = start_time.elapsed();
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Verify processing time is reasonable (under 1 second for small payload)
    assert!(processing_time.as_millis() < 1000, 
        "Processing should be under 1000ms for small payload, was {}ms", 
        processing_time.as_millis());
    
    cleanup_test_data(&pool, user.id).await;
}
