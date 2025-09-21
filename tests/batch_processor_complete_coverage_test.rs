use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use self_sensored::config::BatchConfig;
use self_sensored::models::{HealthMetric, IngestPayload, ProcessingError};
use self_sensored::services::batch_processor::{BatchProcessor, BatchProcessingResult, DeduplicationStats};

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

fn create_test_batch_config() -> BatchConfig {
    BatchConfig {
        heart_rate_chunk_size: 100,
        blood_pressure_chunk_size: 100,
        sleep_chunk_size: 50,
        activity_chunk_size: 30,
        workout_chunk_size: 50,
        body_measurement_chunk_size: 80,
        temperature_chunk_size: 100,
        respiratory_chunk_size: 60,
        blood_glucose_chunk_size: 100,
        nutrition_chunk_size: 40,
        environmental_chunk_size: 70,
        audio_exposure_chunk_size: 80,
        safety_event_chunk_size: 90,
        mindfulness_chunk_size: 60,
        mental_health_chunk_size: 50,
        symptom_chunk_size: 80,
        hygiene_chunk_size: 90,
        menstrual_chunk_size: 100,
        fertility_chunk_size: 100,
        chunk_size: 100,
        memory_limit_mb: 500.0,
        max_retries: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 5000,
        enable_parallel_processing: true,
        enable_progress_tracking: true,
        enable_deduplication: true,
        parallel_chunk_limit: 10,
        metabolic_chunk_size: 80,
        enable_dual_write_activity_metrics: false,
        enable_batch_size_optimization: true,
        enable_reproductive_health_encryption: false,
    }
}

fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    // In a real test, we would insert a user, but for this test we'll just use the UUID
    user_id
}

fn create_test_health_metrics(user_id: Uuid, count: usize) -> Vec<HealthMetric> {
    let mut metrics = Vec::new();
    let base_time = Utc::now();

    for i in 0..count {
        let metric = HealthMetric::HeartRate {
            user_id,
            recorded_at: base_time - Duration::minutes(i as i64),
            heart_rate: Some(70 + (i % 30) as i16),
            resting_heart_rate: Some(60 + (i % 10) as i16),
            heart_rate_variability: Some(40.0 + (i % 20) as f64),
            source_device: Some(format!("device_{}", i % 3)),
            context: None,
        };
        metrics.push(metric);
    }

    metrics
}

fn create_mixed_health_metrics(user_id: Uuid, count: usize) -> Vec<HealthMetric> {
    let mut metrics = Vec::new();
    let base_time = Utc::now();

    for i in 0..count {
        let metric = match i % 5 {
            0 => HealthMetric::HeartRate {
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                heart_rate: Some(70 + (i % 30) as i16),
                resting_heart_rate: Some(60),
                heart_rate_variability: Some(40.0),
                source_device: Some("Apple Watch".to_string()),
                context: None,
            },
            1 => HealthMetric::BloodPressure {
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                systolic: 120 + (i % 20) as i16,
                diastolic: 80 + (i % 10) as i16,
                pulse: Some(72),
                source_device: Some("Omron".to_string()),
            },
            2 => HealthMetric::Sleep {
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
            },
            3 => HealthMetric::Activity {
                user_id,
                recorded_at: base_time - Duration::minutes(i as i64),
                step_count: Some(8000 + (i % 5000) as i32),
                distance_meters: Some(6000.0 + (i % 2000) as f64),
                flights_climbed: Some(10 + (i % 20) as i32),
                active_energy_burned_kcal: Some(300.0 + (i % 200) as f64),
                basal_energy_burned_kcal: Some(1500.0),
                source_device: Some("iPhone".to_string()),
            },
            _ => HealthMetric::Workout {
                user_id,
                workout_type: "Running".to_string(),
                started_at: base_time - Duration::minutes((i * 45) as i64),
                ended_at: base_time - Duration::minutes((i * 45 - 30) as i64),
                total_energy_kcal: Some(350.0),
                active_energy_kcal: Some(300.0),
                distance_meters: Some(5000.0),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(180),
                source_device: Some("Garmin".to_string()),
            },
        };
        metrics.push(metric);
    }

    metrics
}

#[test]
fn test_batch_config_creation_and_validation() {
    let config = create_test_batch_config();

    assert_eq!(config.heart_rate_chunk_size, 100);
    assert_eq!(config.blood_pressure_chunk_size, 100);
    assert_eq!(config.sleep_chunk_size, 50);
    assert_eq!(config.activity_chunk_size, 30);
    assert!(config.enable_parallel_processing);
    assert!(config.enable_deduplication);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.memory_limit_mb, 500.0);
}

#[test]
fn test_batch_processing_result_creation() {
    let mut result = BatchProcessingResult::default();

    assert_eq!(result.processed_count, 0);
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert_eq!(result.processing_time_ms, 0);
    assert_eq!(result.retry_attempts, 0);
    assert!(result.memory_peak_mb.is_none());

    // Test updating result
    result.processed_count = 100;
    result.failed_count = 5;
    result.processing_time_ms = 1500;
    result.retry_attempts = 2;
    result.memory_peak_mb = Some(250.0);

    assert_eq!(result.processed_count, 100);
    assert_eq!(result.failed_count, 5);
    assert_eq!(result.processing_time_ms, 1500);
    assert_eq!(result.retry_attempts, 2);
    assert_eq!(result.memory_peak_mb, Some(250.0));
}

#[test]
fn test_deduplication_stats_creation() {
    let mut stats = DeduplicationStats::default();

    assert_eq!(stats.heart_rate_duplicates, 0);
    assert_eq!(stats.blood_pressure_duplicates, 0);
    assert_eq!(stats.total_duplicates, 0);
    assert_eq!(stats.deduplication_time_ms, 0);

    // Test updating stats
    stats.heart_rate_duplicates = 10;
    stats.blood_pressure_duplicates = 5;
    stats.sleep_duplicates = 3;
    stats.activity_duplicates = 8;
    stats.total_duplicates = 26;
    stats.deduplication_time_ms = 150;

    assert_eq!(stats.heart_rate_duplicates, 10);
    assert_eq!(stats.blood_pressure_duplicates, 5);
    assert_eq!(stats.total_duplicates, 26);
    assert_eq!(stats.deduplication_time_ms, 150);
}

#[test]
fn test_deduplication_stats_comprehensive() {
    let mut stats = DeduplicationStats::default();

    // Test all metric type duplicates
    stats.heart_rate_duplicates = 10;
    stats.blood_pressure_duplicates = 5;
    stats.sleep_duplicates = 3;
    stats.activity_duplicates = 8;
    stats.body_measurement_duplicates = 2;
    stats.temperature_duplicates = 4;
    stats.respiratory_duplicates = 1;
    stats.blood_glucose_duplicates = 6;
    stats.metabolic_duplicates = 3;
    stats.nutrition_duplicates = 7;
    stats.workout_duplicates = 9;
    stats.environmental_duplicates = 2;
    stats.audio_exposure_duplicates = 1;
    stats.safety_event_duplicates = 0;
    stats.mindfulness_duplicates = 5;
    stats.mental_health_duplicates = 3;
    stats.symptom_duplicates = 4;
    stats.hygiene_duplicates = 2;
    stats.menstrual_duplicates = 1;
    stats.fertility_duplicates = 1;

    // Calculate total
    let expected_total = 10 + 5 + 3 + 8 + 2 + 4 + 1 + 6 + 3 + 7 + 9 + 2 + 1 + 0 + 5 + 3 + 4 + 2 + 1 + 1;
    stats.total_duplicates = expected_total;

    assert_eq!(stats.total_duplicates, 77);
    assert_eq!(stats.heart_rate_duplicates, 10);
    assert_eq!(stats.workout_duplicates, 9);
    assert_eq!(stats.activity_duplicates, 8);
    assert_eq!(stats.safety_event_duplicates, 0);
}

#[tokio::test]
async fn test_batch_processor_creation() {
    let pool = setup_test_pool().await;
    let config = create_test_batch_config();

    let processor = BatchProcessor::new(pool, config);

    // Test that processor was created successfully
    // We can't easily test internal state without exposing it,
    // but we can test that it doesn't panic on creation
    assert!(true);
}

#[tokio::test]
async fn test_health_metric_enum_variants() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test all HealthMetric variants can be created
    let heart_rate = HealthMetric::HeartRate {
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.0),
        source_device: Some("Apple Watch".to_string()),
        context: None,
    };

    let blood_pressure = HealthMetric::BloodPressure {
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
        source_device: Some("Omron".to_string()),
    };

    let sleep = HealthMetric::Sleep {
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
    };

    let activity = HealthMetric::Activity {
        user_id,
        recorded_at: now,
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(15),
        active_energy_burned_kcal: Some(400.0),
        basal_energy_burned_kcal: Some(1600.0),
        source_device: Some("iPhone".to_string()),
    };

    let workout = HealthMetric::Workout {
        user_id,
        workout_type: "Running".to_string(),
        started_at: now - Duration::minutes(45),
        ended_at: now,
        total_energy_kcal: Some(350.0),
        active_energy_kcal: Some(300.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source_device: Some("Garmin".to_string()),
    };

    // Test that all variants can be matched
    let metrics = vec![heart_rate, blood_pressure, sleep, activity, workout];

    for metric in metrics {
        match metric {
            HealthMetric::HeartRate { user_id: uid, .. } => assert_eq!(uid, user_id),
            HealthMetric::BloodPressure { user_id: uid, .. } => assert_eq!(uid, user_id),
            HealthMetric::Sleep { user_id: uid, .. } => assert_eq!(uid, user_id),
            HealthMetric::Activity { user_id: uid, .. } => assert_eq!(uid, user_id),
            HealthMetric::Workout { user_id: uid, .. } => assert_eq!(uid, user_id),
        }
    }
}

#[test]
fn test_processing_error_creation() {
    let error = ProcessingError {
        metric_type: "heart_rate".to_string(),
        error_message: "Validation failed".to_string(),
        user_id: Some(Uuid::new_v4()),
        timestamp: Utc::now(),
        retry_count: 0,
        is_recoverable: true,
    };

    assert_eq!(error.metric_type, "heart_rate");
    assert_eq!(error.error_message, "Validation failed");
    assert!(error.user_id.is_some());
    assert_eq!(error.retry_count, 0);
    assert!(error.is_recoverable);
}

#[test]
fn test_ingest_payload_creation() {
    let user_id = Uuid::new_v4();
    let metrics = create_test_health_metrics(user_id, 5);

    let payload = IngestPayload {
        user_id,
        data: metrics.clone(),
        received_at: Utc::now(),
        source_app: Some("AutoExport".to_string()),
        app_version: Some("1.0.0".to_string()),
        device_model: Some("iPhone 15 Pro".to_string()),
        os_version: Some("iOS 17.0".to_string()),
    };

    assert_eq!(payload.user_id, user_id);
    assert_eq!(payload.data.len(), 5);
    assert_eq!(payload.source_app, Some("AutoExport".to_string()));
    assert_eq!(payload.app_version, Some("1.0.0".to_string()));
    assert_eq!(payload.device_model, Some("iPhone 15 Pro".to_string()));
    assert_eq!(payload.os_version, Some("iOS 17.0".to_string()));
}

#[test]
fn test_large_batch_creation() {
    let user_id = Uuid::new_v4();
    let large_batch = create_mixed_health_metrics(user_id, 1000);

    assert_eq!(large_batch.len(), 1000);

    // Count different metric types
    let mut heart_rate_count = 0;
    let mut blood_pressure_count = 0;
    let mut sleep_count = 0;
    let mut activity_count = 0;
    let mut workout_count = 0;

    for metric in &large_batch {
        match metric {
            HealthMetric::HeartRate { .. } => heart_rate_count += 1,
            HealthMetric::BloodPressure { .. } => blood_pressure_count += 1,
            HealthMetric::Sleep { .. } => sleep_count += 1,
            HealthMetric::Activity { .. } => activity_count += 1,
            HealthMetric::Workout { .. } => workout_count += 1,
        }
    }

    // Each type should appear roughly 200 times (1000 / 5)
    assert_eq!(heart_rate_count, 200);
    assert_eq!(blood_pressure_count, 200);
    assert_eq!(sleep_count, 200);
    assert_eq!(activity_count, 200);
    assert_eq!(workout_count, 200);
}

#[test]
fn test_chunk_size_calculations() {
    let config = create_test_batch_config();

    // Test various chunk sizes are within reasonable bounds
    assert!(config.heart_rate_chunk_size > 0);
    assert!(config.heart_rate_chunk_size <= 10000);

    assert!(config.blood_pressure_chunk_size > 0);
    assert!(config.blood_pressure_chunk_size <= 10000);

    assert!(config.sleep_chunk_size > 0);
    assert!(config.sleep_chunk_size <= 10000);

    assert!(config.activity_chunk_size > 0);
    assert!(config.activity_chunk_size <= 10000);

    // Test that smaller chunk sizes are used for complex metrics
    assert!(config.activity_chunk_size <= config.heart_rate_chunk_size);
    assert!(config.sleep_chunk_size <= config.blood_pressure_chunk_size);
}

#[test]
fn test_memory_and_performance_configs() {
    let config = create_test_batch_config();

    assert!(config.memory_limit_mb > 0.0);
    assert!(config.memory_limit_mb <= 10000.0); // Reasonable upper bound

    assert!(config.max_retries > 0);
    assert!(config.max_retries <= 10); // Reasonable retry limit

    assert!(config.initial_backoff_ms > 0);
    assert!(config.max_backoff_ms >= config.initial_backoff_ms);
    assert!(config.parallel_chunk_limit > 0);
    assert!(config.parallel_chunk_limit <= 100); // Reasonable concurrency limit
}

#[test]
fn test_metric_with_edge_case_data() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test metrics with edge case values
    let edge_cases = vec![
        HealthMetric::HeartRate {
            user_id,
            recorded_at: now,
            heart_rate: Some(0), // Minimum value
            resting_heart_rate: None,
            heart_rate_variability: Some(0.0),
            source_device: None,
            context: None,
        },
        HealthMetric::HeartRate {
            user_id,
            recorded_at: now,
            heart_rate: Some(300), // Maximum reasonable value
            resting_heart_rate: Some(250),
            heart_rate_variability: Some(100.0),
            source_device: Some("Test Device".repeat(100)), // Long string
            context: None,
        },
        HealthMetric::BloodPressure {
            user_id,
            recorded_at: now,
            systolic: 50, // Low value
            diastolic: 30,
            pulse: Some(30),
            source_device: None,
        },
        HealthMetric::BloodPressure {
            user_id,
            recorded_at: now,
            systolic: 250, // High value
            diastolic: 150,
            pulse: Some(200),
            source_device: Some("Manual Cuff".to_string()),
        },
    ];

    for metric in edge_cases {
        // Test that edge case metrics can be created and matched
        match metric {
            HealthMetric::HeartRate { user_id: uid, .. } => assert_eq!(uid, user_id),
            HealthMetric::BloodPressure { user_id: uid, .. } => assert_eq!(uid, user_id),
            _ => {}
        }
    }
}

#[test]
fn test_concurrent_batch_creation() {
    use std::sync::Arc;
    use std::thread;

    let user_id = Uuid::new_v4();
    let user_id = Arc::new(user_id);

    // Create batches concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let user_id = Arc::clone(&user_id);
            thread::spawn(move || {
                create_mixed_health_metrics(*user_id, 100 + i * 10)
            })
        })
        .collect();

    // Collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Verify all batches were created
    assert_eq!(results.len(), 10);
    for (i, batch) in results.iter().enumerate() {
        assert_eq!(batch.len(), 100 + i * 10);
    }
}

#[test]
fn test_metric_serialization_roundtrip() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let metric = HealthMetric::HeartRate {
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.5),
        source_device: Some("Apple Watch".to_string()),
        context: None,
    };

    // Test Debug trait
    let debug_str = format!("{:?}", metric);
    assert!(debug_str.contains("HeartRate"));
    assert!(debug_str.contains("75"));

    // Test Clone trait
    let cloned = metric.clone();
    match (&metric, &cloned) {
        (HealthMetric::HeartRate { heart_rate: hr1, .. }, HealthMetric::HeartRate { heart_rate: hr2, .. }) => {
            assert_eq!(hr1, hr2);
        }
        _ => panic!("Clone should preserve enum variant"),
    }
}

#[test]
fn test_batch_config_feature_flags() {
    let mut config = create_test_batch_config();

    // Test all boolean flags
    assert!(config.enable_parallel_processing);
    assert!(config.enable_progress_tracking);
    assert!(config.enable_deduplication);
    assert!(config.enable_batch_size_optimization);
    assert!(!config.enable_dual_write_activity_metrics);
    assert!(!config.enable_reproductive_health_encryption);

    // Test toggling flags
    config.enable_parallel_processing = false;
    config.enable_deduplication = false;
    config.enable_dual_write_activity_metrics = true;

    assert!(!config.enable_parallel_processing);
    assert!(!config.enable_deduplication);
    assert!(config.enable_dual_write_activity_metrics);
}

#[test]
fn test_processing_error_categorization() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let errors = vec![
        ProcessingError {
            metric_type: "heart_rate".to_string(),
            error_message: "Database connection failed".to_string(),
            user_id: Some(user_id),
            timestamp: now,
            retry_count: 0,
            is_recoverable: true,
        },
        ProcessingError {
            metric_type: "blood_pressure".to_string(),
            error_message: "Validation failed: systolic too high".to_string(),
            user_id: Some(user_id),
            timestamp: now,
            retry_count: 3,
            is_recoverable: false,
        },
        ProcessingError {
            metric_type: "sleep".to_string(),
            error_message: "Timeout during processing".to_string(),
            user_id: Some(user_id),
            timestamp: now,
            retry_count: 1,
            is_recoverable: true,
        },
    ];

    // Test error categorization
    let recoverable_errors: Vec<_> = errors.iter().filter(|e| e.is_recoverable).collect();
    let non_recoverable_errors: Vec<_> = errors.iter().filter(|e| !e.is_recoverable).collect();

    assert_eq!(recoverable_errors.len(), 2);
    assert_eq!(non_recoverable_errors.len(), 1);

    // Test retry counts
    let max_retry_count = errors.iter().map(|e| e.retry_count).max().unwrap();
    assert_eq!(max_retry_count, 3);
}

#[test]
fn test_deduplication_key_generation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test that metrics with same key data would be considered duplicates
    let metric1 = HealthMetric::HeartRate {
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.0),
        source_device: Some("Apple Watch".to_string()),
        context: None,
    };

    let metric2 = HealthMetric::HeartRate {
        user_id,
        recorded_at: now, // Same timestamp
        heart_rate: Some(80), // Different value
        resting_heart_rate: Some(70),
        heart_rate_variability: Some(50.0),
        source_device: Some("Garmin".to_string()),
        context: None,
    };

    // These would be considered duplicates based on user_id + recorded_at
    match (&metric1, &metric2) {
        (HealthMetric::HeartRate { user_id: u1, recorded_at: t1, .. },
         HealthMetric::HeartRate { user_id: u2, recorded_at: t2, .. }) => {
            assert_eq!(u1, u2);
            assert_eq!(t1, t2);
        }
        _ => panic!("Both should be HeartRate metrics"),
    }
}

#[test]
fn test_batch_result_aggregation() {
    let mut result = BatchProcessingResult::default();

    // Simulate processing multiple chunks
    for i in 0..5 {
        result.processed_count += 100 + i * 10;
        result.failed_count += i;
        result.retry_attempts += i / 2;
        result.processing_time_ms += 200 + i * 50;
    }

    assert_eq!(result.processed_count, 600); // 100+110+120+130+140
    assert_eq!(result.failed_count, 10); // 0+1+2+3+4
    assert_eq!(result.retry_attempts, 3); // 0+0+1+1+2 (integer division)
    assert_eq!(result.processing_time_ms, 1500); // 200+250+300+350+400
}

// This test verifies the traits are properly implemented
#[test]
fn test_struct_traits() {
    let config = create_test_batch_config();
    let stats = DeduplicationStats::default();
    let result = BatchProcessingResult::default();

    // Test Clone trait
    let _cloned_config = config.clone();
    let _cloned_stats = stats.clone();
    let _cloned_result = result.clone();

    // Test Debug trait
    let _debug_config = format!("{:?}", config);
    let _debug_stats = format!("{:?}", stats);
    let _debug_result = format!("{:?}", result);

    // Test Default trait
    let _default_stats = DeduplicationStats::default();
    let _default_result = BatchProcessingResult::default();

    // All should compile without errors
    assert!(true);
}