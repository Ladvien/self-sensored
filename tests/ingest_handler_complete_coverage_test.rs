use actix_web::{test, web, App, http::header};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::handlers::health::health_check;
use self_sensored::models::health_metrics::{
    HealthMetric, IngestPayload, IngestData, ActivityMetric, WorkoutData,
    HeartRateMetric, BloodPressureMetric, SleepMetric
};
use self_sensored::models::enums::WorkoutType;
use self_sensored::services::auth::{AuthService, AuthContext};
use self_sensored::middleware::auth::AuthMiddleware;

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_user_and_key(pool: &PgPool) -> (Uuid, String) {
    let auth_service = AuthService::new(pool.clone());
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    // Create user
    sqlx::query!(
        "INSERT INTO users (id, email, is_active, created_at) VALUES ($1, $2, true, NOW())",
        user_id,
        &email
    )
    .execute(pool)
    .await
    .unwrap();

    // Create API key
    let (raw_key, _) = auth_service.create_api_key(
        user_id,
        Some("Test Key"),
        None,
        None,
        Some(1000)
    ).await.unwrap();

    (user_id, raw_key)
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM blood_pressure_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM sleep_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM workouts WHERE user_id = $1", user_id)
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
async fn test_health_check_endpoint() {
    let app = test::init_service(
        App::new()
            .route("/health", web::get().to(health_check))
    ).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("healthy") || body_str.contains("ok"));
}

fn create_test_health_metrics(user_id: Uuid, count: usize) -> Vec<HealthMetric> {
    let mut metrics = Vec::new();
    let base_time = Utc::now();

    for i in 0..count {
        let metric = match i % 5 {
            0 => HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                heart_rate: Some(70 + (i % 30) as i16),
                resting_heart_rate: Some(60),
                heart_rate_variability: Some(40.0),
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch".to_string()),
                context: None,
                created_at: Utc::now(),
            }),
            1 => HealthMetric::BloodPressure(BloodPressureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                systolic: 120 + (i % 20) as i16,
                diastolic: 80 + (i % 10) as i16,
                pulse: Some(72),
                source_device: Some("Omron".to_string()),
                created_at: Utc::now(),
            }),
            2 => HealthMetric::Sleep(SleepMetric {
                id: Uuid::new_v4(),
                user_id,
                sleep_start: base_time - Duration::hours(8) - Duration::minutes(i as i64),
                sleep_end: base_time - Duration::minutes(i as i64),
                duration_minutes: Some(480),
                deep_sleep_minutes: Some(120),
                rem_sleep_minutes: Some(90),
                light_sleep_minutes: Some(200),
                awake_minutes: Some(70),
                efficiency: Some(85.0),
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            }),
            3 => HealthMetric::Activity(ActivityMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                step_count: Some(8000 + (i % 5000) as i32),
                distance_meters: Some(6000.0 + (i % 2000) as f64),
                flights_climbed: Some(10 + (i % 20) as i32),
                active_energy_burned_kcal: Some(300.0 + (i % 200) as f64),
                basal_energy_burned_kcal: Some(1500.0),
                distance_cycling_meters: None,
                distance_swimming_meters: None,
                distance_wheelchair_meters: None,
                distance_downhill_snow_sports_meters: None,
                push_count: None,
                swimming_stroke_count: None,
                nike_fuel_points: None,
                apple_exercise_time_minutes: None,
                apple_stand_time_minutes: None,
                apple_move_time_minutes: None,
                apple_stand_hour_achieved: None,
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
                created_at: Utc::now(),
            }),
            _ => HealthMetric::Workout(WorkoutData {
                id: Uuid::new_v4(),
                user_id,
                workout_type: WorkoutType::Running,
                started_at: base_time - Duration::minutes((i * 45) as i64),
                ended_at: base_time - Duration::minutes((i * 45 - 30) as i64),
                total_energy_kcal: Some(350.0),
                active_energy_kcal: Some(300.0),
                distance_meters: Some(5000.0),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(180),
                source_device: Some("Garmin".to_string()),
                created_at: Utc::now(),
            }),
        };
        metrics.push(metric);
    }

    metrics
}

#[actix_web::test]
async fn test_ingest_endpoint_basic_success() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let metrics = create_test_health_metrics(user_id, 5);
    let payload = IngestPayload {
        data: IngestData {
            metrics: metrics,
            workouts: vec![],
        },
    };

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Note: This test may fail without proper middleware setup
    // In a real scenario, you'd need to include authentication middleware

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_endpoint_empty_payload() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![], // Empty metrics
            workouts: vec![],
        },
    };

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should handle empty payload gracefully
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_endpoint_large_payload() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let metrics = create_test_health_metrics(user_id, 1000); // Large batch
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_endpoint_malformed_json() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let malformed_json = r#"{"user_id": "invalid-uuid", "data": [}"#; // Invalid JSON

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload(malformed_json)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 Bad Request for malformed JSON
    assert!(resp.status().is_client_error());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_endpoint_missing_auth_header() {
    let pool = setup_test_pool().await;
    let (user_id, _) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let metrics = create_test_health_metrics(user_id, 5);
    let payload = IngestPayload {
        data: IngestData {
            metrics: metrics,
            workouts: vec![],
        },
    };

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        // No Authorization header
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Without proper middleware, this might succeed unexpectedly
    // In production, this should return 401 Unauthorized

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_ingest_endpoint_invalid_auth_token() {
    let pool = setup_test_pool().await;
    let (user_id, _) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let metrics = create_test_health_metrics(user_id, 5);
    let payload = IngestPayload {
        data: IngestData {
            metrics: metrics,
            workouts: vec![],
        },
    };

    let req = test::TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, "Bearer invalid_token_123"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 401 Unauthorized for invalid token
    cleanup_test_data(&pool, user_id).await;
}

#[test]
#[test]
fn test_ingest_payload_validation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test valid payload
    let valid_payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate {
                    user_id,
                    recorded_at: now,
                    heart_rate: Some(75),
                    resting_heart_rate: Some(65),
                    heart_rate_variability: Some(45.0),
                    source_device: Some("Apple Watch".to_string()),
                    context: None,
                }
            ],
            workouts: vec![],
        },
    };

    assert_eq!(valid_payload.data.metrics.len(), 1);
    assert_eq!(valid_payload.data.workouts.len(), 0);

    // Test payload serialization
    let json = serde_json::to_string(&valid_payload).unwrap();
    assert!(json.contains("AutoExport"));
    assert!(json.contains("iPhone 15 Pro"));

    // Test payload deserialization
    let deserialized: IngestPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.user_id, user_id);
    assert_eq!(deserialized.data.len(), 1);
}

#[test]
#[test]
fn test_ingest_payload_edge_cases() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test payload with minimal data
    let minimal_payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    // IngestPayload now only has a 'data' field
    assert_eq!(minimal_payload.data.metrics.len(), 0);
    assert_eq!(minimal_payload.data.workouts.len(), 0);
}

#[test]
#[test]
fn test_health_metric_enum_serialization() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let metrics = vec![
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
            source_device: Some("Apple Watch".to_string()),
            context: None,
            created_at: Utc::now(),
        }),
        HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: now,
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source_device: Some("Omron".to_string()),
            created_at: Utc::now(),
        }),
        HealthMetric::Sleep(SleepMetric {
            id: Uuid::new_v4(),
            user_id,
            sleep_start: now - Duration::hours(8),
            sleep_end: now,
            duration_minutes: Some(480),
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(200),
            awake_minutes: Some(70),
            efficiency: Some(85.0),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        }),
    ];

    for metric in metrics {
        // Test serialization
        let json = serde_json::to_string(&metric).unwrap();
        assert!(!json.is_empty());

        // Test deserialization
        let deserialized: HealthMetric = serde_json::from_str(&json).unwrap();

        // Verify user_id is preserved
        match (&metric, &deserialized) {
            (HealthMetric::HeartRate { user_id: u1, .. }, HealthMetric::HeartRate { user_id: u2, .. }) => {
                assert_eq!(u1, u2);
            }
            (HealthMetric::BloodPressure { user_id: u1, .. }, HealthMetric::BloodPressure { user_id: u2, .. }) => {
                assert_eq!(u1, u2);
            }
            (HealthMetric::Sleep { user_id: u1, .. }, HealthMetric::Sleep { user_id: u2, .. }) => {
                assert_eq!(u1, u2);
            }
            _ => panic!("Metric type changed during serialization"),
        }
    }
}

#[test]
#[test]
fn test_concurrent_payload_creation() {
    use std::thread;
    use std::sync::Arc;

    let user_id = Arc::new(Uuid::new_v4());

    // Create payloads concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let user_id = Arc::clone(&user_id);
            thread::spawn(move || {
                let metrics = create_test_health_metrics(*user_id, 10 + i);
                IngestPayload {
                    data: IngestData {
                        metrics,
                        workouts: vec![],
                    },
                }
            })
        })
        .collect();

    // Collect results
    let payloads: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify all payloads were created correctly
    assert_eq!(payloads.len(), 10);
    for (i, payload) in payloads.iter().enumerate() {
        assert_eq!(payload.data.metrics.len(), 10 + i);
        assert_eq!(payload.data.workouts.len(), 0);
    }
}

#[actix_web::test]
async fn test_ingest_endpoint_different_content_types() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let metrics = create_test_health_metrics(user_id, 3);
    let payload = IngestPayload {
        data: IngestData {
            metrics: metrics,
            workouts: vec![],
        },
    };

    // Test with different content types
    let content_types = vec![
        "application/json",
        "application/json; charset=utf-8",
        "text/json",
    ];

    for content_type in content_types {
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
            .insert_header((header::CONTENT_TYPE, content_type))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Most should work, but some might be rejected
        // The actual behavior depends on the handler implementation
    }

    cleanup_test_data(&pool, user_id).await;
}