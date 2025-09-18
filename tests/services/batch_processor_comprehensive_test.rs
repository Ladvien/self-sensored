///! Comprehensive test suite for batch processor functionality
///!
///! This test suite covers:
///! - PostgreSQL parameter limit validation
///! - All metric type batch processing
///! - Chunk size safety
///! - Data loss prevention
///! - Integration testing for all metric types

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::config::{
    BatchConfig, SAFE_PARAM_LIMIT,
    HEART_RATE_PARAMS_PER_RECORD, BLOOD_PRESSURE_PARAMS_PER_RECORD,
    SLEEP_PARAMS_PER_RECORD, ACTIVITY_PARAMS_PER_RECORD,
    BODY_MEASUREMENT_PARAMS_PER_RECORD, WORKOUT_PARAMS_PER_RECORD,
    METABOLIC_PARAMS_PER_RECORD, SAFETY_EVENT_PARAMS_PER_RECORD,
    MINDFULNESS_PARAMS_PER_RECORD, MENTAL_HEALTH_PARAMS_PER_RECORD,
    ENVIRONMENTAL_PARAMS_PER_RECORD, AUDIO_EXPOSURE_PARAMS_PER_RECORD,
};
use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, BodyMeasurementMetric,
    HeartRateMetric, HealthMetric, IngestData, IngestPayload,
    SleepMetric, WorkoutData, MetabolicMetric, SafetyEventMetric,
    MindfulnessMetric, MentalHealthMetric, EnvironmentalMetric,
    AudioExposureMetric,
};
use self_sensored::models::enums::{
    WorkoutType, SafetyEventType, MindfulnessSessionType,
    MoodRating, AudioExposureEvent,
};
use self_sensored::services::batch_processor::{BatchProcessor, BatchProcessingResult};

/// Test helper to create a test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/health_export_test".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Test helper to create a test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        format!("test+{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Test helper to clean up test data
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let tables = vec![
        "heart_rate_metrics", "blood_pressure_metrics", "sleep_metrics",
        "activity_metrics", "body_measurement_metrics", "workout_metrics",
        "metabolic_metrics", "safety_event_metrics", "mindfulness_metrics",
        "mental_health_metrics", "environmental_metrics", "audio_exposure_metrics",
        "users"
    ];

    for table in tables {
        sqlx::query(&format!("DELETE FROM {} WHERE user_id = $1", table))
            .bind(user_id)
            .execute(pool)
            .await
            .ok(); // Ignore errors for tables that don't have user_id
    }

    // Clean up users table
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Test PostgreSQL parameter limit validation for all metric types
#[tokio::test]
async fn test_postgresql_parameter_limits() {
    // Test that all chunk sizes stay under PostgreSQL 65,535 parameter limit

    let test_cases = vec![
        ("HeartRate", HEART_RATE_PARAMS_PER_RECORD, 5242), // 10 params -> 5,242 max safe
        ("BloodPressure", BLOOD_PRESSURE_PARAMS_PER_RECORD, 8738), // 6 params -> 8,738 max safe
        ("Sleep", SLEEP_PARAMS_PER_RECORD, 5242), // 10 params -> 5,242 max safe
        ("Activity", ACTIVITY_PARAMS_PER_RECORD, 2759), // 19 params -> 2,759 max safe
        ("BodyMeasurement", BODY_MEASUREMENT_PARAMS_PER_RECORD, 3276), // 16 params -> 3,276 max safe
        ("Workout", WORKOUT_PARAMS_PER_RECORD, 5242), // 10 params -> 5,242 max safe
        ("Metabolic", METABOLIC_PARAMS_PER_RECORD, 8738), // 6 params -> 8,738 max safe
        ("SafetyEvent", SAFETY_EVENT_PARAMS_PER_RECORD, 6553), // 8 params -> 6,553 max safe
        ("Mindfulness", MINDFULNESS_PARAMS_PER_RECORD, 5825), // 9 params -> 5,825 max safe
        ("MentalHealth", MENTAL_HEALTH_PARAMS_PER_RECORD, 5242), // 10 params -> 5,242 max safe
        ("Environmental", ENVIRONMENTAL_PARAMS_PER_RECORD, 3744), // 14 params -> 3,744 max safe
        ("AudioExposure", AUDIO_EXPOSURE_PARAMS_PER_RECORD, 7489), // 7 params -> 7,489 max safe
    ];

    for (metric_type, params_per_record, expected_max_safe) in test_cases {
        let theoretical_max = 65535 / params_per_record;
        let calculated_safe = (theoretical_max as f64 * 0.8) as usize;

        assert_eq!(calculated_safe, expected_max_safe,
            "{} metric: Expected max safe chunk size of {}, got {}",
            metric_type, expected_max_safe, calculated_safe);

        // Verify parameters don't exceed safe limit
        assert!(calculated_safe * params_per_record <= SAFE_PARAM_LIMIT,
            "{} metric: {} params * {} chunk = {} exceeds safe limit of {}",
            metric_type, params_per_record, calculated_safe,
            calculated_safe * params_per_record, SAFE_PARAM_LIMIT);
    }
}

/// Test batch configuration parameter validation
#[tokio::test]
async fn test_batch_config_validation() {
    let valid_config = BatchConfig::from_env();

    // Test that config validation passes for valid configuration
    assert!(valid_config.validate().is_ok(), "Valid configuration should pass validation");

    // Test invalid configurations
    let mut invalid_config = valid_config.clone();
    invalid_config.activity_chunk_size = 7000; // Too large: 7000 * 19 = 133,000 > 65,535

    let validation_result = invalid_config.validate();
    assert!(validation_result.is_err(), "Invalid activity chunk size should fail validation");

    if let Err(error) = validation_result {
        assert!(error.contains("activity"),
            "Error should mention activity: {}", error);
        assert!(error.contains("PostgreSQL parameter limit") || error.contains("parameter"),
            "Error should mention PostgreSQL parameter limit: {}", error);
    }
}

/// Test chunk size safety calculations
#[tokio::test]
async fn test_chunk_size_safety() {
    let config = BatchConfig::from_env();

    // Verify all chunk sizes are safe
    assert!(config.heart_rate_chunk_size * HEART_RATE_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Heart rate chunk size is unsafe");
    assert!(config.blood_pressure_chunk_size * BLOOD_PRESSURE_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Blood pressure chunk size is unsafe");
    assert!(config.sleep_chunk_size * SLEEP_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Sleep chunk size is unsafe");
    assert!(config.activity_chunk_size * ACTIVITY_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Activity chunk size is unsafe");
    assert!(config.body_measurement_chunk_size * BODY_MEASUREMENT_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Body measurement chunk size is unsafe");
    assert!(config.workout_chunk_size * WORKOUT_PARAMS_PER_RECORD <= SAFE_PARAM_LIMIT,
        "Workout chunk size is unsafe");
}

/// Test data loss prevention for all metric types
#[tokio::test]
async fn test_data_loss_prevention() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig::from_env();
    let batch_processor = BatchProcessor::new(pool.clone(), config);

    // Create comprehensive test metrics
    let test_metrics = create_comprehensive_test_metrics(user_id);
    let original_count = test_metrics.len();

    // Process metrics
    let payload = IngestPayload {
        user_id,
        data: IngestData {
            metrics: test_metrics,
        },
    };

    let result = batch_processor.process_health_metrics(&payload).await;
    assert!(result.is_ok(), "Batch processing should succeed: {:?}", result);

    let processing_result = result.unwrap();

    // Verify no data loss
    assert_eq!(processing_result.total_processed, original_count,
        "Should process all {} metrics without data loss. Processed: {}",
        original_count, processing_result.total_processed);

    assert_eq!(processing_result.failed, 0,
        "Should have no failed metrics. Failed: {}", processing_result.failed);

    // Verify data is actually in database
    let db_metrics_count = count_all_metrics_in_db(&pool, user_id).await;
    assert_eq!(db_metrics_count, original_count,
        "Database should contain all {} metrics. Found: {}",
        original_count, db_metrics_count);

    cleanup_test_data(&pool, user_id).await;
}

/// Test all metric type batch processing
#[tokio::test]
async fn test_all_metric_types_batch_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig::from_env();
    let batch_processor = BatchProcessor::new(pool.clone(), config);

    // Test each metric type individually
    let metric_tests = vec![
        ("HeartRate", vec![create_heart_rate_metric(user_id)]),
        ("BloodPressure", vec![create_blood_pressure_metric(user_id)]),
        ("Sleep", vec![create_sleep_metric(user_id)]),
        ("Activity", vec![create_activity_metric(user_id)]),
        ("BodyMeasurement", vec![create_body_measurement_metric(user_id)]),
        ("Workout", vec![create_workout_metric(user_id)]),
        ("Metabolic", vec![create_metabolic_metric(user_id)]),
        ("SafetyEvent", vec![create_safety_event_metric(user_id)]),
        ("Mindfulness", vec![create_mindfulness_metric(user_id)]),
        ("MentalHealth", vec![create_mental_health_metric(user_id)]),
        ("Environmental", vec![create_environmental_metric(user_id)]),
        ("AudioExposure", vec![create_audio_exposure_metric(user_id)]),
    ];

    for (metric_type, metrics) in metric_tests {
        let payload = IngestPayload {
            user_id,
            data: IngestData { metrics },
        };

        let result = batch_processor.process_health_metrics(&payload).await;
        assert!(result.is_ok(), "{} metric processing failed: {:?}", metric_type, result);

        let processing_result = result.unwrap();
        assert_eq!(processing_result.total_processed, 1,
            "{} metric should process 1 metric", metric_type);
        assert_eq!(processing_result.failed, 0,
            "{} metric should have no failures", metric_type);
    }

    cleanup_test_data(&pool, user_id).await;
}

/// Test large batch processing with chunking
#[tokio::test]
async fn test_large_batch_chunking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig::from_env();
    let batch_processor = BatchProcessor::new(pool.clone(), config.clone());

    // Create large batch of activity metrics (most parameter-heavy)
    let batch_size = config.activity_chunk_size * 3; // 3 chunks worth
    let large_batch: Vec<HealthMetric> = (0..batch_size)
        .map(|i| {
            let mut metric = create_activity_metric(user_id);
            // Make each metric unique by adjusting timestamp
            if let HealthMetric::Activity(ref mut activity) = metric {
                activity.recorded_at = Utc::now() + chrono::Duration::milliseconds(i as i64);
            }
            metric
        })
        .collect();

    let payload = IngestPayload {
        user_id,
        data: IngestData { metrics: large_batch },
    };

    let result = batch_processor.process_health_metrics(&payload).await;
    assert!(result.is_ok(), "Large batch processing should succeed: {:?}", result);

    let processing_result = result.unwrap();
    assert_eq!(processing_result.total_processed, batch_size,
        "Should process all {} metrics in large batch", batch_size);
    assert_eq!(processing_result.failed, 0,
        "Large batch should have no failures");

    cleanup_test_data(&pool, user_id).await;
}

/// Test memory limit protection
#[tokio::test]
async fn test_memory_limit_protection() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let mut config = BatchConfig::from_env();
    config.memory_limit_mb = 1.0; // Very low memory limit

    let batch_processor = BatchProcessor::new(pool.clone(), config);

    // Create metrics that would exceed memory limit
    let large_batch: Vec<HealthMetric> = (0..10000)
        .map(|_| create_heart_rate_metric(user_id))
        .collect();

    let payload = IngestPayload {
        user_id,
        data: IngestData { metrics: large_batch },
    };

    let result = batch_processor.process_health_metrics(&payload).await;
    // Should either succeed with chunking or fail gracefully
    match result {
        Ok(processing_result) => {
            // If it succeeds, it should process all metrics via chunking
            assert_eq!(processing_result.total_processed, 10000);
        }
        Err(error) => {
            // If it fails, it should be due to memory limit
            assert!(error.to_string().contains("memory") || error.to_string().contains("limit"));
        }
    }

    cleanup_test_data(&pool, user_id).await;
}

// Helper functions to create test metrics

fn create_comprehensive_test_metrics(user_id: Uuid) -> Vec<HealthMetric> {
    vec![
        create_heart_rate_metric(user_id),
        create_blood_pressure_metric(user_id),
        create_sleep_metric(user_id),
        create_activity_metric(user_id),
        create_body_measurement_metric(user_id),
        create_workout_metric(user_id),
        create_metabolic_metric(user_id),
        create_safety_event_metric(user_id),
        create_mindfulness_metric(user_id),
        create_mental_health_metric(user_id),
        create_environmental_metric(user_id),
        create_audio_exposure_metric(user_id),
    ]
}

fn create_heart_rate_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::HeartRate(HeartRateMetric {
        user_id,
        recorded_at: Utc::now(),
        heart_rate: Some(72),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.0),
        walking_heart_rate_average: Some(85),
        heart_rate_recovery_one_minute: Some(25),
        atrial_fibrillation_burden_percentage: Some(0.1),
        vo2_max_ml_kg_min: Some(35.5),
        context: Some("test".to_string()),
        source_device: "TestDevice".to_string(),
    })
}

fn create_blood_pressure_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::BloodPressure(BloodPressureMetric {
        user_id,
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
        source_device: "TestDevice".to_string(),
    })
}

fn create_sleep_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Sleep(SleepMetric {
        user_id,
        sleep_start: Utc::now() - chrono::Duration::hours(8),
        sleep_end: Utc::now(),
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(90),
        light_sleep_minutes: Some(240),
        awake_minutes: Some(30),
        efficiency: Some(87.5),
        source_device: "TestDevice".to_string(),
    })
}

fn create_activity_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Activity(ActivityMetric {
        user_id,
        recorded_at: Utc::now(),
        step_count: Some(8500),
        distance_meters: Some(6800.0),
        active_energy_burned_kcal: Some(450.0),
        basal_energy_burned_kcal: Some(1800.0),
        flights_climbed: Some(12),
        distance_cycling_meters: Some(0.0),
        distance_swimming_meters: Some(0.0),
        distance_wheelchair_meters: Some(0.0),
        distance_downhill_snow_sports_meters: Some(0.0),
        push_count: Some(0),
        swimming_stroke_count: Some(0),
        nike_fuel_points: Some(2400),
        apple_exercise_time_minutes: Some(30),
        apple_stand_time_minutes: Some(540),
        apple_move_time_minutes: Some(720),
        apple_stand_hour_achieved: Some(12),
        source_device: "TestDevice".to_string(),
    })
}

fn create_body_measurement_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::BodyMeasurement(BodyMeasurementMetric {
        user_id,
        recorded_at: Utc::now(),
        body_weight_kg: Some(70.5),
        body_mass_index: Some(22.5),
        body_fat_percentage: Some(15.0),
        lean_body_mass_kg: Some(59.9),
        height_cm: Some(175.0),
        waist_circumference_cm: Some(85.0),
        hip_circumference_cm: Some(95.0),
        chest_circumference_cm: Some(100.0),
        arm_circumference_cm: Some(32.0),
        thigh_circumference_cm: Some(55.0),
        body_temperature_celsius: Some(36.7),
        basal_body_temperature_celsius: Some(36.5),
        measurement_source: Some("scale".to_string()),
        source_device: "TestDevice".to_string(),
    })
}

fn create_workout_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Workout(WorkoutData {
        user_id,
        workout_type: WorkoutType::Running,
        started_at: Utc::now() - chrono::Duration::minutes(30),
        ended_at: Utc::now(),
        total_energy_kcal: Some(250.0),
        active_energy_kcal: Some(200.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: Some(140),
        max_heart_rate: Some(165),
        source_device: "TestDevice".to_string(),
    })
}

fn create_metabolic_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Metabolic(MetabolicMetric {
        user_id,
        recorded_at: Utc::now(),
        blood_alcohol_content: Some(0.0),
        insulin_delivery_units: Some(5.0),
        delivery_method: Some("injection".to_string()),
        source_device: "TestDevice".to_string(),
    })
}

fn create_safety_event_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::SafetyEvent(SafetyEventMetric {
        user_id,
        recorded_at: Utc::now(),
        event_type: SafetyEventType::Fall,
        severity_level: 3,
        location: Some("home".to_string()),
        description: Some("test fall event".to_string()),
        emergency_contact_notified: Some(false),
        source_device: "TestDevice".to_string(),
    })
}

fn create_mindfulness_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Mindfulness(MindfulnessMetric {
        user_id,
        recorded_at: Utc::now(),
        session_type: MindfulnessSessionType::Meditation,
        duration_minutes: 15,
        stress_level_before: Some(7),
        stress_level_after: Some(4),
        focus_rating: Some(8),
        notes: Some("good session".to_string()),
        source_device: "TestDevice".to_string(),
    })
}

fn create_mental_health_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::MentalHealth(MentalHealthMetric {
        user_id,
        recorded_at: Utc::now(),
        mood_rating: MoodRating::Good,
        anxiety_level: Some(3),
        stress_level: Some(4),
        energy_level: Some(7),
        sleep_quality_perception: Some(8),
        medication_taken: Some(false),
        therapy_session: Some(false),
        notes: Some("feeling good today".to_string()),
        source_device: "TestDevice".to_string(),
    })
}

fn create_environmental_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::Environmental(EnvironmentalMetric {
        user_id,
        recorded_at: Utc::now(),
        environmental_audio_exposure_db: Some(65.0),
        headphone_audio_exposure_db: Some(75.0),
        uv_index: Some(5.0),
        uv_exposure_minutes: Some(30),
        ambient_temperature_celsius: Some(22.0),
        humidity_percent: Some(45.0),
        air_pressure_hpa: Some(1013.25),
        altitude_meters: Some(100.0),
        time_in_daylight_minutes: Some(480),
        location_latitude: Some(37.7749),
        location_longitude: Some(-122.4194),
        source_device: "TestDevice".to_string(),
    })
}

fn create_audio_exposure_metric(user_id: Uuid) -> HealthMetric {
    HealthMetric::AudioExposure(AudioExposureMetric {
        user_id,
        recorded_at: Utc::now(),
        environmental_audio_exposure_db: Some(70.0),
        headphone_audio_exposure_db: Some(80.0),
        exposure_duration_minutes: 60,
        audio_exposure_event: AudioExposureEvent::LoudEnvironment,
        source_device: "TestDevice".to_string(),
    })
}

/// Count all metrics in database for verification
async fn count_all_metrics_in_db(pool: &PgPool, user_id: Uuid) -> usize {
    let mut total = 0;

    let tables = vec![
        "heart_rate_metrics", "blood_pressure_metrics", "sleep_metrics",
        "activity_metrics", "body_measurement_metrics", "workout_metrics",
        "metabolic_metrics", "safety_event_metrics", "mindfulness_metrics",
        "mental_health_metrics", "environmental_metrics", "audio_exposure_metrics",
    ];

    for table in tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {} WHERE user_id = $1", table))
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        total += count as usize;
    }

    total
}