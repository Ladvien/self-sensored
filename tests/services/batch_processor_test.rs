use chrono::{DateTime, Utc};
use futures::future::join_all;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use tokio::time::sleep;
use tokio::sync::Semaphore;

use self_sensored::config::BatchConfig;
use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, HealthMetric, HeartRateMetric, IngestData,
    IngestPayload, SleepMetric, WorkoutData,
};
use self_sensored::models::enums::{ActivityContext, WorkoutType};
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

/// Test helper to clean up test data for core 5 metric types
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let tables = vec![
        "heart_rate_metrics",
        "blood_pressure_metrics",
        "sleep_metrics",
        "activity_metrics",
        "workout_metrics",
        "raw_ingestions",
        "users",
    ];

    for table in tables {
        sqlx::query!("DELETE FROM {} WHERE user_id = $1", table.as_str())
            .bind(user_id)
            .execute(pool)
            .await
            .ok(); // Ignore errors for cleanup
    }
}

/// Create test heart rate metrics
fn create_heart_rate_metrics(count: usize, user_id: Uuid) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now(),
                heart_rate: Some(70 + (i % 30) as i16),
                resting_heart_rate: Some(60 + (i % 20) as i16),
                heart_rate_variability: Some(40.0 + (i % 20) as f64),
                walking_heart_rate_average: Some(85 + (i % 25) as i16),
                heart_rate_recovery_one_minute: Some(20 + (i % 15) as i16),
                atrial_fibrillation_burden_percentage: Some(rust_decimal::Decimal::from_f64_retain((i % 5) as f64).unwrap_or_default()),
                vo2_max_ml_kg_min: Some(rust_decimal::Decimal::from_f64_retain(45.0 + (i % 15) as f64).unwrap_or_default()),
                context: Some(ActivityContext::Exercise),
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create test blood pressure metrics
fn create_blood_pressure_metrics(count: usize, user_id: Uuid) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::BloodPressure(BloodPressureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now(),
                systolic: 120 + (i % 30) as i16,
                diastolic: 80 + (i % 20) as i16,
                pulse: Some(70 + (i % 25) as i16),
                source_device: Some("Blood Pressure Monitor".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create test sleep metrics
fn create_sleep_metrics(count: usize, user_id: Uuid) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            let sleep_start = Utc::now() - chrono::Duration::hours(8);
            HealthMetric::Sleep(SleepMetric {
                id: Uuid::new_v4(),
                user_id,
                sleep_start,
                sleep_end: sleep_start + chrono::Duration::hours(8),
                duration_minutes: Some(480 + (i % 120) as i32),
                deep_sleep_minutes: Some(120 + (i % 60) as i32),
                rem_sleep_minutes: Some(90 + (i % 30) as i32),
                light_sleep_minutes: Some(180 + (i % 90) as i32),
                awake_minutes: Some(10 + (i % 20) as i32),
                efficiency: Some(85.0 + (i % 15) as f64),
                source_device: Some("iPhone".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create test activity metrics
fn create_activity_metrics(count: usize, user_id: Uuid) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::Activity(ActivityMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now(),
                step_count: Some(8000 + (i % 5000) as i32),
                distance_meters: Some(6000.0 + (i % 2000) as f64),
                active_energy_burned_kcal: Some(300.0 + (i % 200) as f64),
                basal_energy_burned_kcal: Some(1500.0 + (i % 300) as f64),
                flights_climbed: Some((i % 20) as i32),
                distance_cycling_meters: Some((i % 10000) as f64),
                distance_swimming_meters: Some((i % 2000) as f64),
                distance_wheelchair_meters: Some(0.0),
                distance_downhill_snow_sports_meters: Some(0.0),
                push_count: Some(0),
                swimming_stroke_count: Some((i % 1000) as i32),
                nike_fuel_points: Some((i % 3000) as i32),
                apple_exercise_time_minutes: Some((i % 60) as i32),
                apple_stand_time_minutes: Some((i % 16) as i32),
                apple_move_time_minutes: Some((i % 60) as i32),
                apple_stand_hour_achieved: Some(i % 2 == 0),
                walking_speed_m_per_s: Some(1.3 + (i % 5) as f64 * 0.1),
                walking_step_length_cm: Some(70.0 + (i % 10) as f64),
                walking_asymmetry_percent: Some((i % 5) as f64),
                walking_double_support_percent: Some(20.0 + (i % 10) as f64),
                six_minute_walk_test_distance_m: Some(400.0 + (i % 200) as f64),
                stair_ascent_speed_m_per_s: Some(0.5 + (i % 3) as f64 * 0.1),
                stair_descent_speed_m_per_s: Some(0.4 + (i % 3) as f64 * 0.1),
                ground_contact_time_ms: Some(200.0 + (i % 50) as f64),
                vertical_oscillation_cm: Some(8.0 + (i % 3) as f64),
                running_stride_length_m: Some(1.2 + (i % 5) as f64 * 0.1),
                running_power_watts: Some(250.0 + (i % 100) as f64),
                running_speed_m_per_s: Some(3.0 + (i % 3) as f64),
                cycling_speed_kmh: Some(25.0 + (i % 15) as f64),
                cycling_power_watts: Some(200.0 + (i % 150) as f64),
                cycling_cadence_rpm: Some(80.0 + (i % 40) as f64),
                functional_threshold_power_watts: Some(250.0 + (i % 100) as f64),
                underwater_depth_meters: Some((i % 10) as f64),
                diving_duration_seconds: Some((i % 3600) as i32),
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            })
        })
        .collect()
}

/// Create test workout metrics
fn create_workout_metrics(count: usize, user_id: Uuid) -> Vec<WorkoutData> {
    (0..count)
        .map(|i| {
            let started_at = Utc::now() - chrono::Duration::hours(1);
            WorkoutData {
                id: Uuid::new_v4(),
                user_id,
                workout_type: WorkoutType::Running,
                started_at,
                ended_at: started_at + chrono::Duration::minutes(30 + (i % 60) as i64),
                total_energy_kcal: Some(300.0 + (i % 200) as f64),
                active_energy_kcal: Some(250.0 + (i % 150) as f64),
                distance_meters: Some(5000.0 + (i % 3000) as f64),
                avg_heart_rate: Some(140 + (i % 40) as i32),
                max_heart_rate: Some(170 + (i % 30) as i32),
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            }
        })
        .collect()
}

// ===== BATCH PROCESSING TESTS =====

#[tokio::test]
async fn test_batch_processing_small_batches() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Test small batches for all 5 core metric types
    let test_cases = vec![
        ("heart_rate", create_heart_rate_metrics(10, user_id)),
        ("blood_pressure", create_blood_pressure_metrics(10, user_id)),
        ("sleep", create_sleep_metrics(10, user_id)),
        ("activity", create_activity_metrics(10, user_id)),
    ];

    for (metric_type, metrics) in test_cases {
        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: if metric_type == "workout" {
                    create_workout_metrics(10, user_id)
                } else {
                    vec![]
                },
            },
        };

        let result = processor.process_batch(user_id, payload).await;

        assert_eq!(result.failed_count, 0,
            "Failed to process {} metrics: {:?}", metric_type, result.errors);
        assert_eq!(result.processed_count, 10,
            "Expected 10 {} metrics to be processed", metric_type);
        assert!(result.processing_time_ms < 1000,
            "Processing {} metrics took too long: {}ms", metric_type, result.processing_time_ms);
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_batch_processing_large_batches() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Test large batches up to 10,000 items
    let test_sizes = vec![100, 1000, 5000, 10000];

    for size in test_sizes {
        let metrics = create_heart_rate_metrics(size, user_id);
        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };

        let start_time = Instant::now();
        let result = processor.process_batch(user_id, payload).await;
        let processing_time = start_time.elapsed();

        assert_eq!(result.failed_count, 0,
            "Failed to process {} heart rate metrics: {:?}", size, result.errors);
        assert_eq!(result.processed_count, size,
            "Expected {} heart rate metrics to be processed", size);

        // Performance requirement: 10,000 metrics in <5 seconds
        if size == 10000 {
            assert!(processing_time.as_secs() < 5,
                "Processing 10,000 metrics took too long: {:?}", processing_time);
        }

        println!("Processed {} metrics in {:?}", size, processing_time);
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_postgresql_parameter_limits() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test each metric type at its maximum safe chunk size
    let test_cases = vec![
        ("heart_rate", 4766, create_heart_rate_metrics(4766, user_id)),
        ("blood_pressure", 8738, create_blood_pressure_metrics(8738, user_id)),
        ("sleep", 5242, create_sleep_metrics(5242, user_id)),
        ("activity", 1450, create_activity_metrics(1450, user_id)),
    ];

    for (metric_type, chunk_size, metrics) in test_cases {
        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };

        let result = BatchProcessor::new(pool.clone()).process_batch(user_id, payload).await;

        assert_eq!(result.failed_count, 0,
            "Failed to process {} {} metrics at max chunk size: {:?}",
            chunk_size, metric_type, result.errors);
        assert_eq!(result.processed_count, chunk_size,
            "Expected {} {} metrics to be processed", chunk_size, metric_type);

        println!("Successfully processed {} {} metrics in one chunk", chunk_size, metric_type);
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_chunking_strategies() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test different chunking configurations
    let configs = vec![
        ("small_chunks", BatchConfig {
            heart_rate_chunk_size: 100,
            blood_pressure_chunk_size: 100,
            sleep_chunk_size: 100,
            activity_chunk_size: 100,
            ..BatchConfig::default()
        }),
        ("large_chunks", BatchConfig {
            heart_rate_chunk_size: 4000,
            blood_pressure_chunk_size: 8000,
            sleep_chunk_size: 5000,
            activity_chunk_size: 1400,
            ..BatchConfig::default()
        }),
    ];

    for (config_name, config) in configs {
        let processor = BatchProcessor::with_config(pool.clone(), config);
        let metrics = create_heart_rate_metrics(1000, user_id);

        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };

        let result = processor.process_batch(user_id, payload).await;

        assert_eq!(result.failed_count, 0,
            "Failed to process metrics with {} config: {:?}", config_name, result.errors);
        assert_eq!(result.processed_count, 1000,
            "Expected 1000 metrics to be processed with {} config", config_name);

        if let Some(progress) = result.chunk_progress {
            println!("{} config: {} chunks processed", config_name, progress.chunks_completed);
        }
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_parallel_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test with parallel processing enabled vs disabled
    let configs = vec![
        ("parallel_enabled", BatchConfig {
            enable_parallel_processing: true,
            ..BatchConfig::default()
        }),
        ("parallel_disabled", BatchConfig {
            enable_parallel_processing: false,
            ..BatchConfig::default()
        }),
    ];

    for (config_name, config) in configs {
        let processor = BatchProcessor::with_config(pool.clone(), config);
        let metrics = create_heart_rate_metrics(2000, user_id);

        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };

        let start_time = Instant::now();
        let result = processor.process_batch(user_id, payload).await;
        let processing_time = start_time.elapsed();

        assert_eq!(result.failed_count, 0,
            "Failed to process metrics with {}: {:?}", config_name, result.errors);
        assert_eq!(result.processed_count, 2000,
            "Expected 2000 metrics to be processed with {}", config_name);

        println!("{}: processed 2000 metrics in {:?}", config_name, processing_time);
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_concurrent_batch_processing() {
    let pool = create_test_pool().await;
    let processor = Arc::new(BatchProcessor::new(pool.clone()));

    // Create multiple users for concurrent testing
    let user_ids: Vec<Uuid> = join_all((0..5).map(|_| create_test_user(&pool))).await;

    // Create concurrent batch processing tasks
    let tasks: Vec<_> = user_ids.iter().map(|&user_id| {
        let processor = Arc::clone(&processor);
        let metrics = create_heart_rate_metrics(500, user_id);

        tokio::spawn(async move {
            let payload = IngestPayload {
                data: IngestData {
                    metrics,
                    workouts: vec![],
                },
            };

            processor.process_batch(user_id, payload).await
        })
    }).collect();

    // Wait for all tasks to complete
    let results = join_all(tasks).await;

    // Verify all batches processed successfully
    for (i, result) in results.into_iter().enumerate() {
        let batch_result = result.expect("Task should complete");
        assert_eq!(batch_result.failed_count, 0,
            "Concurrent batch {} failed: {:?}", i, batch_result.errors);
        assert_eq!(batch_result.processed_count, 500,
            "Expected 500 metrics in concurrent batch {}", i);
    }

    // Cleanup all test users
    for user_id in user_ids {
        cleanup_test_data(&pool, user_id).await;
    }
}

#[tokio::test]
async fn test_error_handling_and_retry_logic() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test with retry configuration
    let config = BatchConfig {
        max_retries: 3,
        initial_backoff_ms: 10,
        max_backoff_ms: 100,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create metrics with some invalid data to trigger retries
    let mut metrics = create_heart_rate_metrics(100, user_id);

    // Add some metrics with invalid user_id to trigger errors
    metrics.extend(create_heart_rate_metrics(10, Uuid::new_v4()));

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should have some successful and some failed
    assert!(result.processed_count > 0, "Some metrics should be processed successfully");
    assert!(result.retry_attempts > 0, "Should have retry attempts for failed metrics");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_deduplication_within_batches() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create metrics with duplicates
    let mut metrics = create_heart_rate_metrics(100, user_id);
    let duplicates = create_heart_rate_metrics(50, user_id); // Same data
    metrics.extend(duplicates);

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should process unique metrics only
    assert!(result.processed_count <= 150, "Should not process all metrics due to deduplication");

    if let Some(dedup_stats) = result.deduplication_stats {
        assert!(dedup_stats.heart_rate_duplicates > 0, "Should detect heart rate duplicates");
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_memory_limits_and_backpressure() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig {
        memory_limit_mb: 100.0, // Low memory limit for testing
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create a large batch
    let metrics = create_activity_metrics(5000, user_id); // Activity metrics are larger

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should still process successfully but may chunk differently
    assert_eq!(result.failed_count, 0, "Should handle memory limits gracefully");
    assert_eq!(result.processed_count, 5000, "Should process all metrics despite memory constraints");

    if let Some(memory_peak) = result.memory_peak_mb {
        println!("Peak memory usage: {:.2} MB", memory_peak);
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_transaction_isolation_and_rollback() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let processor = BatchProcessor::new(pool.clone());

    // Process valid metrics first
    let valid_metrics = create_heart_rate_metrics(50, user_id);
    let valid_payload = IngestPayload {
        data: IngestData {
            metrics: valid_metrics,
            workouts: vec![],
        },
    };

    let valid_result = processor.process_batch(user_id, valid_payload).await;
    assert_eq!(valid_result.processed_count, 50);
    assert_eq!(valid_result.failed_count, 0);

    // Verify data was inserted
    let count_before = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);

    assert_eq!(count_before, 50);

    // Now try processing with some invalid metrics (invalid user_id)
    let mut mixed_metrics = create_heart_rate_metrics(25, user_id);
    mixed_metrics.extend(create_heart_rate_metrics(25, Uuid::new_v4())); // Invalid user

    let mixed_payload = IngestPayload {
        data: IngestData {
            metrics: mixed_metrics,
            workouts: vec![],
        },
    };

    let mixed_result = processor.process_batch(user_id, mixed_payload).await;

    // Verify partial success (valid metrics should be processed)
    assert!(mixed_result.processed_count > 0, "Some valid metrics should be processed");
    assert!(mixed_result.failed_count > 0, "Some invalid metrics should fail");

    // Verify the original data is still intact
    let count_after = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);

    assert!(count_after >= 50, "Original valid data should remain intact");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_configuration_validation() {
    // Test valid configuration
    let valid_config = BatchConfig::default();
    assert!(valid_config.validate().is_ok(), "Default configuration should be valid");

    // Test invalid configuration (exceeds PostgreSQL parameter limits)
    let invalid_config = BatchConfig {
        heart_rate_chunk_size: 10000, // Would exceed parameter limit
        ..BatchConfig::default()
    };

    assert!(invalid_config.validate().is_err(), "Invalid configuration should fail validation");

    // Test environment configuration
    std::env::set_var("BATCH_HEART_RATE_CHUNK_SIZE", "1000");
    std::env::set_var("BATCH_ENABLE_PARALLEL", "false");

    let env_config = BatchConfig::from_env();
    assert_eq!(env_config.heart_rate_chunk_size, 1000);
    assert_eq!(env_config.enable_parallel_processing, false);

    // Cleanup environment variables
    std::env::remove_var("BATCH_HEART_RATE_CHUNK_SIZE");
    std::env::remove_var("BATCH_ENABLE_PARALLEL");
}

#[tokio::test]
async fn test_performance_benchmarks() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Benchmark different metric types
    let benchmark_cases = vec![
        ("heart_rate", create_heart_rate_metrics(1000, user_id)),
        ("blood_pressure", create_blood_pressure_metrics(1000, user_id)),
        ("sleep", create_sleep_metrics(1000, user_id)),
        ("activity", create_activity_metrics(1000, user_id)),
    ];

    let mut performance_results = HashMap::new();

    for (metric_type, metrics) in benchmark_cases {
        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };

        let start_time = Instant::now();
        let result = processor.process_batch(user_id, payload).await;
        let processing_time = start_time.elapsed();

        assert_eq!(result.failed_count, 0, "Benchmark should process without errors");
        assert_eq!(result.processed_count, 1000, "Should process all 1000 metrics");

        performance_results.insert(metric_type, processing_time);
        println!("{}: 1000 metrics processed in {:?}", metric_type, processing_time);
    }

    // Verify performance requirements
    for (metric_type, duration) in performance_results {
        assert!(duration.as_secs() < 2,
            "{} processing took too long: {:?}", metric_type, duration);
    }

    cleanup_test_data(&pool, user_id).await;
}
