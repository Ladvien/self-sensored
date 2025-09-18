use chrono::Utc;
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

use self_sensored::config::{BatchConfig, HEART_RATE_PARAMS_PER_RECORD, BLOOD_PRESSURE_PARAMS_PER_RECORD, SAFE_PARAM_LIMIT, SLEEP_PARAMS_PER_RECORD, ACTIVITY_PARAMS_PER_RECORD, WORKOUT_PARAMS_PER_RECORD};
use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, HealthMetric, HeartRateMetric, IngestData, IngestPayload,
    SleepMetric, WorkoutData,
};
use self_sensored::models::enums::{ActivityContext, WorkoutType};
use self_sensored::services::batch_processor::BatchProcessor;

/// Test helper to create a test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/health_export_test".to_string()
    });

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
        "heart_rate_metrics",
        "blood_pressure_metrics",
        "sleep_metrics",
        "activity_metrics",
        "workouts",
        "users",
    ];

    for table in tables {
        let query = format!("DELETE FROM {} WHERE user_id = $1", table);
        sqlx::query(&query)
            .bind(user_id)
            .execute(pool)
            .await
            .ok(); // Ignore errors for cleanup
    }

    // Also clean up users table
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Create sample heart rate metrics for testing
fn create_sample_heart_rate_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overwritten in processing
                recorded_at: Utc::now() - chrono::Duration::minutes(i as i64),
                heart_rate: Some(70 + (i % 30) as i16),
                resting_heart_rate: Some(60 + (i % 20) as i16),
                heart_rate_variability: Some(40.0 + (i % 20) as f64),
                walking_heart_rate_average: Some(90 + (i % 15) as i16),
                heart_rate_recovery_one_minute: Some(20 + (i % 10) as i16),
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch".to_string()),
                context: Some(ActivityContext::Resting),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create sample blood pressure metrics for testing
fn create_sample_blood_pressure_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::BloodPressure(BloodPressureMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overwritten in processing
                recorded_at: Utc::now() - chrono::Duration::hours(i as i64),
                systolic: 120 + (i % 20) as i16,
                diastolic: 80 + (i % 10) as i16,
                pulse: Some(70 + (i % 15) as i16),
                source_device: Some("Manual Entry".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create sample sleep metrics for testing
fn create_sample_sleep_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            let sleep_start = Utc::now() - chrono::Duration::hours(8 + i as i64 * 24);
            let sleep_end = sleep_start + chrono::Duration::hours(7);

            HealthMetric::Sleep(SleepMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overwritten in processing
                sleep_start,
                sleep_end,
                duration_minutes: Some(420), // 7 hours
                deep_sleep_minutes: Some(90), // 1.5 hours
                rem_sleep_minutes: Some(90),
                light_sleep_minutes: Some(240), // 4 hours
                awake_minutes: Some(30),
                efficiency: Some(85.0 + (i % 15) as f64),
                source_device: Some("Sleep Tracker".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create sample activity metrics for testing
fn create_sample_activity_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::Activity(ActivityMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overwritten in processing
                recorded_at: Utc::now() - chrono::Duration::hours(i as i64),
                step_count: Some(8000 + (i % 5000) as i32),
                distance_meters: Some(5500.0 + (i % 3000) as f64),
                flights_climbed: Some(10 + (i % 20) as i32),
                active_energy_burned_kcal: Some(250.0 + (i % 150) as f64),
                basal_energy_burned_kcal: Some(1200.0 + (i % 300) as f64),
                distance_cycling_meters: None,
                distance_swimming_meters: None,
                distance_wheelchair_meters: None,
                distance_downhill_snow_sports_meters: None,
                push_count: None,
                swimming_stroke_count: None,
                nike_fuel_points: None,
                apple_exercise_time_minutes: Some(30 + (i % 60) as i32),
                apple_stand_time_minutes: Some(12 + (i % 8) as i32),
                apple_move_time_minutes: Some(40 + (i % 30) as i32),
                apple_stand_hour_achieved: Some(i % 2 == 0),
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
            })
        })
        .collect()
}

/// Create sample workout data for testing
fn create_sample_workouts(count: usize) -> Vec<WorkoutData> {
    (0..count)
        .map(|i| {
            let start_time = Utc::now() - chrono::Duration::hours(i as i64);
            WorkoutData {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overwritten in processing
                workout_type: WorkoutType::Running,
                started_at: start_time,
                ended_at: start_time + chrono::Duration::minutes(30),
                total_energy_kcal: Some(300.0 + (i % 100) as f64),
                active_energy_kcal: Some(250.0 + (i % 80) as f64),
                distance_meters: Some(5000.0 + (i % 2000) as f64),
                avg_heart_rate: Some(150 + (i % 30) as i32),
                max_heart_rate: Some(180 + (i % 20) as i32),
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            }
        })
        .collect()
}

#[tokio::test]
async fn test_batch_processor_new() {
    let pool = create_test_pool().await;
    let _processor = BatchProcessor::new(pool.clone());

    // Test that the processor can be created without errors
    // Internal state is private, so we test behavior through the public API
}

#[tokio::test]
async fn test_batch_processor_with_config() {
    let pool = create_test_pool().await;
    let config = BatchConfig {
        max_retries: 5,
        enable_parallel_processing: false,
        heart_rate_chunk_size: 1000,
        ..BatchConfig::default()
    };

    let _processor = BatchProcessor::with_config(pool.clone(), config.clone());
    // Test that the processor can be created with custom config without errors
    // Configuration is private, so we test behavior through the public API
}

#[tokio::test]
async fn test_reset_counters() {
    let pool = create_test_pool().await;
    let processor = BatchProcessor::new(pool);

    // Test that reset_counters method exists and can be called
    processor.reset_counters();

    // Internal counters are private, so we can't test them directly
    // This test verifies the method exists and doesn't panic
}

#[tokio::test]
async fn test_process_batch_empty_payload() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 0);
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert!(result.processing_time_ms > 0);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_process_batch_single_metric_type() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    let metrics = create_sample_heart_rate_metrics(5);
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 5);
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert!(result.processing_time_ms > 0);
    assert!(result.deduplication_stats.is_some());

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_process_batch_multiple_metric_types() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    let mut metrics = create_sample_heart_rate_metrics(3);
    metrics.extend(create_sample_blood_pressure_metrics(2));
    metrics.extend(create_sample_sleep_metrics(1));

    let workouts = create_sample_workouts(2);

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 8); // 3 + 2 + 1 + 2
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert!(result.deduplication_stats.is_some());

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_process_batch_large_chunks() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Configure smaller chunk sizes to test chunking logic
    let config = BatchConfig {
        heart_rate_chunk_size: 3,
        blood_pressure_chunk_size: 2,
        enable_parallel_processing: false,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    let mut metrics = create_sample_heart_rate_metrics(10); // Will require multiple chunks
    metrics.extend(create_sample_blood_pressure_metrics(5)); // Will require multiple chunks

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 15);
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert!(result.processing_time_ms > 0);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_process_batch_parallel_vs_sequential() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let metrics = create_sample_heart_rate_metrics(50);

    // Test parallel processing
    let config_parallel = BatchConfig {
        enable_parallel_processing: true,
        ..BatchConfig::default()
    };
    let processor_parallel = BatchProcessor::with_config(pool.clone(), config_parallel);

    let payload_parallel = IngestPayload {
        data: IngestData {
            metrics: metrics.clone(),
            workouts: vec![],
        },
    };

    let _start_parallel = Instant::now();
    let result_parallel = processor_parallel.process_batch(user_id, payload_parallel).await;

    cleanup_test_data(&pool, user_id).await;

    // Test sequential processing
    let config_sequential = BatchConfig {
        enable_parallel_processing: false,
        ..BatchConfig::default()
    };
    let processor_sequential = BatchProcessor::with_config(pool.clone(), config_sequential);

    let payload_sequential = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let _start_sequential = Instant::now();
    let result_sequential = processor_sequential.process_batch(user_id, payload_sequential).await;

    // Both should process the same number of records successfully
    assert_eq!(result_parallel.processed_count, 50);
    assert_eq!(result_sequential.processed_count, 50);
    assert_eq!(result_parallel.failed_count, 0);
    assert_eq!(result_sequential.failed_count, 0);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_deduplication_functionality() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create duplicate metrics (same timestamp)
    let base_time = Utc::now();
    let duplicate_metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time,
            heart_rate: Some(70),
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(40.0),
            walking_heart_rate_average: Some(90),
            heart_rate_recovery_one_minute: Some(20),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time, // Same timestamp - should be deduplicated
            heart_rate: Some(71),
            resting_heart_rate: Some(61),
            heart_rate_variability: Some(41.0),
            walking_heart_rate_average: Some(91),
            heart_rate_recovery_one_minute: Some(21),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time + chrono::Duration::minutes(1), // Different timestamp
            heart_rate: Some(72),
            resting_heart_rate: Some(62),
            heart_rate_variability: Some(42.0),
            walking_heart_rate_average: Some(92),
            heart_rate_recovery_one_minute: Some(22),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics: duplicate_metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should process 2 unique records (1 duplicate removed)
    assert_eq!(result.processed_count, 2);
    assert_eq!(result.failed_count, 0);

    let dedup_stats = result.deduplication_stats.unwrap();
    assert_eq!(dedup_stats.heart_rate_duplicates, 1);
    assert_eq!(dedup_stats.total_duplicates, 1);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_chunk_size_validation() {
    let _pool = create_test_pool().await;

    // Test invalid chunk size that would exceed PostgreSQL parameter limits
    let invalid_config = BatchConfig {
        heart_rate_chunk_size: 10000, // This would exceed SAFE_PARAM_LIMIT
        ..BatchConfig::default()
    };

    // Validation should catch this
    let validation_result = invalid_config.validate();
    assert!(validation_result.is_err());

    // Test valid chunk size
    let valid_config = BatchConfig {
        heart_rate_chunk_size: 1000, // This should be safe
        ..BatchConfig::default()
    };

    let validation_result = valid_config.validate();
    assert!(validation_result.is_ok());
}

#[tokio::test]
async fn test_parameter_limit_edge_cases() {
    // Test chunk sizes right at the boundary
    let boundary_config = BatchConfig {
        heart_rate_chunk_size: SAFE_PARAM_LIMIT / HEART_RATE_PARAMS_PER_RECORD,
        blood_pressure_chunk_size: SAFE_PARAM_LIMIT / BLOOD_PRESSURE_PARAMS_PER_RECORD,
        sleep_chunk_size: SAFE_PARAM_LIMIT / SLEEP_PARAMS_PER_RECORD,
        activity_chunk_size: SAFE_PARAM_LIMIT / ACTIVITY_PARAMS_PER_RECORD,
        workout_chunk_size: SAFE_PARAM_LIMIT / WORKOUT_PARAMS_PER_RECORD,
        ..BatchConfig::default()
    };

    let validation_result = boundary_config.validate();
    assert!(validation_result.is_ok(), "Boundary chunk sizes should be valid");

    // Test chunk sizes just over the boundary
    let over_boundary_config = BatchConfig {
        heart_rate_chunk_size: (SAFE_PARAM_LIMIT / HEART_RATE_PARAMS_PER_RECORD) + 1,
        ..BatchConfig::default()
    };

    let validation_result = over_boundary_config.validate();
    assert!(validation_result.is_err(), "Over-boundary chunk sizes should be invalid");
}

#[tokio::test]
async fn test_memory_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    let metrics = create_sample_heart_rate_metrics(100);
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert!(result.memory_peak_mb.is_some());
    let memory_usage = result.memory_peak_mb.unwrap();
    // Memory usage should be non-negative
    assert!(memory_usage >= 0.0);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_processing_with_database_error() {
    let pool = create_test_pool().await;

    // Create user with invalid UUID to trigger database error
    let invalid_user_id = Uuid::nil(); // This user doesn't exist
    let processor = BatchProcessor::new(pool.clone());

    let metrics = create_sample_heart_rate_metrics(5);
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(invalid_user_id, payload).await;

    // Should handle the error gracefully
    assert!(result.failed_count > 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_configuration_from_environment() {
    // Set environment variables
    std::env::set_var("BATCH_MAX_RETRIES", "5");
    std::env::set_var("BATCH_ENABLE_PARALLEL", "false");
    std::env::set_var("BATCH_HEART_RATE_CHUNK_SIZE", "2000");

    let config = BatchConfig::from_env();

    assert_eq!(config.max_retries, 5);
    assert!(!config.enable_parallel_processing);
    assert_eq!(config.heart_rate_chunk_size, 2000);

    // Clean up environment variables
    std::env::remove_var("BATCH_MAX_RETRIES");
    std::env::remove_var("BATCH_ENABLE_PARALLEL");
    std::env::remove_var("BATCH_HEART_RATE_CHUNK_SIZE");
}

#[tokio::test]
async fn test_batch_config_performance_benchmark() {
    let config = BatchConfig::default();
    let report = config.performance_benchmark();

    // Should contain expected sections
    assert!(report.contains("STORY-OPTIMIZATION-001"));
    assert!(report.contains("OPTIMIZATION SUMMARY"));
    assert!(report.contains("POSTGRESQL PARAMETER USAGE"));
    assert!(report.contains("VALIDATION RESULTS"));
}

#[tokio::test]
async fn test_progress_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig {
        enable_progress_tracking: true,
        heart_rate_chunk_size: 2, // Small chunks to test progress
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    let metrics = create_sample_heart_rate_metrics(10);
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 10);
    assert!(result.chunk_progress.is_some());

    let progress = result.chunk_progress.unwrap();
    assert!(progress.total_chunks > 0);
    assert_eq!(progress.completed_chunks, progress.total_chunks);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_retry_logic_configuration() {
    let pool = create_test_pool().await;

    let config = BatchConfig {
        max_retries: 3,
        initial_backoff_ms: 50,
        max_backoff_ms: 200,
        ..BatchConfig::default()
    };

    let _processor = BatchProcessor::with_config(pool.clone(), config.clone());

    // Test that the processor can be created with retry configuration
    // The behavior of retry logic will be tested through process_batch
}

#[tokio::test]
async fn test_intra_batch_deduplication_configuration() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test with deduplication enabled
    let config_with_dedup = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };

    let processor_with_dedup = BatchProcessor::with_config(pool.clone(), config_with_dedup);

    let base_time = Utc::now();
    let duplicate_metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time,
            heart_rate: Some(70),
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(40.0),
            walking_heart_rate_average: Some(90),
            heart_rate_recovery_one_minute: Some(20),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time, // Duplicate
            heart_rate: Some(71),
            resting_heart_rate: Some(61),
            heart_rate_variability: Some(41.0),
            walking_heart_rate_average: Some(91),
            heart_rate_recovery_one_minute: Some(21),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics: duplicate_metrics,
            workouts: vec![],
        },
    };

    let result = processor_with_dedup.process_batch(user_id, payload).await;

    // Should deduplicate
    assert_eq!(result.processed_count, 1);
    let dedup_stats = result.deduplication_stats.unwrap();
    assert_eq!(dedup_stats.heart_rate_duplicates, 1);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_all_metric_types_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create a mix of all supported metric types
    let mut metrics = create_sample_heart_rate_metrics(2);
    metrics.extend(create_sample_blood_pressure_metrics(2));
    metrics.extend(create_sample_sleep_metrics(1));
    metrics.extend(create_sample_activity_metrics(2));

    let workouts = create_sample_workouts(2);

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should process all metric types successfully
    assert_eq!(result.processed_count, 9); // 2+2+1+2+2
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());

    let dedup_stats = result.deduplication_stats.unwrap();
    assert_eq!(dedup_stats.total_duplicates, 0); // No duplicates in this test

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create metrics with invalid data that might cause database errors
    let invalid_metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(-1), // Invalid heart rate
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(40.0),
            walking_heart_rate_average: Some(90),
            heart_rate_recovery_one_minute: Some(20),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics: invalid_metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should handle errors gracefully (depending on validation)
    // The result may have failures or errors, but shouldn't crash
    assert!(result.processing_time_ms > 0);

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_semaphore_concurrency_control() {
    let pool = create_test_pool().await;
    let _processor = BatchProcessor::new(pool.clone());

    // Test that the processor is created successfully with semaphore
    // Semaphore is private, so we test behavior indirectly through processing
}

#[tokio::test]
async fn test_default_batch_config_values() {
    let config = BatchConfig::default();

    // Test default values
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_backoff_ms, 100);
    assert_eq!(config.max_backoff_ms, 5000);
    assert!(config.enable_parallel_processing);
    assert_eq!(config.chunk_size, 1000);
    assert_eq!(config.memory_limit_mb, 500.0);
    assert!(config.enable_progress_tracking);
    assert!(config.enable_intra_batch_deduplication);

    // Test that chunk sizes are reasonable
    assert!(config.heart_rate_chunk_size > 0);
    assert!(config.blood_pressure_chunk_size > 0);
    assert!(config.sleep_chunk_size > 0);
    assert!(config.activity_chunk_size > 0);
    assert!(config.workout_chunk_size > 0);
}

#[tokio::test]
async fn test_deduplication_stats_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create multiple types of duplicates
    let base_time = Utc::now();
    let metrics = vec![
        // Heart rate duplicates
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time,
            heart_rate: Some(70),
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(40.0),
            walking_heart_rate_average: Some(90),
            heart_rate_recovery_one_minute: Some(20),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time, // Duplicate
            heart_rate: Some(71),
            resting_heart_rate: Some(61),
            heart_rate_variability: Some(41.0),
            walking_heart_rate_average: Some(91),
            heart_rate_recovery_one_minute: Some(21),
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        // Blood pressure duplicates
        HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time,
            systolic: 120,
            diastolic: 80,
            pulse: Some(70),
            source_device: Some("Manual".to_string()),
            created_at: Utc::now(),
        }),
        HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_time, // Duplicate
            systolic: 121,
            diastolic: 81,
            pulse: Some(71),
            source_device: Some("Manual".to_string()),
            created_at: Utc::now(),
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let dedup_stats = result.deduplication_stats.unwrap();
    assert_eq!(dedup_stats.heart_rate_duplicates, 1);
    assert_eq!(dedup_stats.blood_pressure_duplicates, 1);
    assert_eq!(dedup_stats.total_duplicates, 2);
    assert!(dedup_stats.deduplication_time_ms > 0);

    cleanup_test_data(&pool, user_id).await;
}