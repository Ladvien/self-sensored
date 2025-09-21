use chrono::Utc;
use futures::future::join_all;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

use self_sensored::config::BatchConfig;
use self_sensored::models::{
    HealthMetric, IngestData, IngestPayload, HeartRateMetric, BloodPressureMetric,
    SleepMetric, ActivityMetric, WorkoutMetric, BodyMeasurementMetric, TemperatureMetric,
    RespiratoryMetric, BloodGlucoseMetric, ProcessingError,
};
use self_sensored::services::batch_processor::{
    BatchProcessor, BatchProcessingResult, DeduplicationStats, ChunkProgress,
};

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up all test data for user
    let tables = vec![
        "heart_rate_metrics",
        "blood_pressure_metrics",
        "sleep_metrics",
        "activity_metrics",
        "workout_metrics",
        "body_measurement_metrics",
        "temperature_metrics",
        "respiratory_metrics",
        "blood_glucose_metrics",
    ];

    for table in tables {
        let query = format!("DELETE FROM {} WHERE user_id = $1", table);
        sqlx::query(&query)
            .bind(user_id)
            .execute(pool)
            .await
            .ok();
    }

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

fn create_test_user_id() -> Uuid {
    Uuid::new_v4()
}

#[tokio::test]
async fn test_batch_processor_creation() {
    let pool = setup_test_pool().await;
    let processor = BatchProcessor::new(pool.clone());

    // Test with custom config
    let config = BatchConfig {
        chunk_size: 100,
        enable_parallel_processing: true,
        max_retries: 5,
        initial_backoff_ms: 200,
        max_backoff_ms: 10000,
        memory_limit_mb: 1000.0,
        enable_progress_tracking: true,
        enable_deduplication: true,
    };

    let custom_processor = BatchProcessor::with_config(pool.clone(), config);
    assert!(custom_processor.config.enable_parallel_processing);
}

#[sqlx::test]
async fn test_empty_batch_processing(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();
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

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_heart_rate_batch_processing(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_hr_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let processor = BatchProcessor::new(pool.clone());

    // Create heart rate metrics
    let mut metrics = vec![];
    for i in 0..10 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: Utc::now() - chrono::Duration::minutes(i),
            heart_rate: Some(60 + i as i32),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(42.5),
            context: Some("resting".to_string()),
            source_device: Some("Apple Watch".to_string()),
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;
    assert_eq!(result.processed_count, 10);
    assert_eq!(result.failed_count, 0);

    // Verify data was inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, Some(10));

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_duplicate_detection(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_dup_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let config = BatchConfig {
        enable_deduplication: true,
        ..Default::default()
    };
    let processor = BatchProcessor::with_config(pool.clone(), config);

    let timestamp = Utc::now();

    // Create duplicate heart rate metrics
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: timestamp,
            heart_rate: Some(70),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: timestamp, // Same timestamp - duplicate
            heart_rate: Some(71),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should have deduplicated
    assert!(result.deduplication_stats.is_some());
    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.heart_rate_duplicates, 1);
    assert_eq!(stats.total_duplicates, 1);

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_chunking_large_batch(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_chunk_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let config = BatchConfig {
        chunk_size: 10, // Small chunk size to test chunking
        enable_progress_tracking: true,
        ..Default::default()
    };
    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create large batch of metrics
    let mut metrics = vec![];
    for i in 0..100 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: Utc::now() - chrono::Duration::seconds(i),
            heart_rate: Some(60 + (i % 40) as i32),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 100);
    assert!(result.chunk_progress.is_some());

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_parallel_processing(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_parallel_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let config = BatchConfig {
        enable_parallel_processing: true,
        chunk_size: 20,
        ..Default::default()
    };
    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create mixed metrics
    let mut metrics = vec![];

    // Add various metric types
    for i in 0..50 {
        if i % 3 == 0 {
            metrics.push(HealthMetric::HeartRate(HeartRateMetric {
                user_id,
                recorded_at: Utc::now() - chrono::Duration::seconds(i),
                heart_rate: Some(70),
                resting_heart_rate: None,
                heart_rate_variability: None,
                context: None,
                source_device: None,
            }));
        } else if i % 3 == 1 {
            metrics.push(HealthMetric::BloodPressure(BloodPressureMetric {
                user_id,
                recorded_at: Utc::now() - chrono::Duration::seconds(i),
                systolic: 120,
                diastolic: 80,
                pulse: Some(72),
                source_device: None,
            }));
        } else {
            metrics.push(HealthMetric::Activity(ActivityMetric {
                user_id,
                recorded_at: Utc::now() - chrono::Duration::seconds(i),
                step_count: Some(1000 + i as i32),
                distance_meters: Some(800.0 + i as f64),
                active_energy_burned_kcal: Some(50.0),
                basal_energy_burned_kcal: Some(100.0),
                flights_climbed: None,
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
                source_device: None,
            }));
        }
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let start = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let duration = start.elapsed();

    assert_eq!(result.processed_count, 50);
    assert_eq!(result.failed_count, 0);

    // Parallel processing should be reasonably fast
    assert!(duration < Duration::from_secs(5));

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_error_handling_invalid_data(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();
    // Don't create user - should cause foreign key violations

    let processor = BatchProcessor::new(pool.clone());

    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: Utc::now(),
            heart_rate: Some(70),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should fail due to missing user
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(!result.errors.is_empty());

    Ok(())
}

#[sqlx::test]
async fn test_workout_processing(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_workout_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let processor = BatchProcessor::new(pool.clone());

    let workouts = vec![
        WorkoutMetric {
            user_id,
            workout_type: "Running".to_string(),
            started_at: Utc::now() - chrono::Duration::hours(2),
            ended_at: Utc::now() - chrono::Duration::hours(1),
            total_energy_kcal: Some(500.0),
            active_energy_kcal: Some(450.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(180),
            source_device: Some("Apple Watch".to_string()),
        },
        WorkoutMetric {
            user_id,
            workout_type: "Cycling".to_string(),
            started_at: Utc::now() - chrono::Duration::hours(4),
            ended_at: Utc::now() - chrono::Duration::hours(3),
            total_energy_kcal: Some(400.0),
            active_energy_kcal: Some(350.0),
            distance_meters: Some(15000.0),
            avg_heart_rate: Some(140),
            max_heart_rate: Some(170),
            source_device: Some("Garmin".to_string()),
        },
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 2);
    assert_eq!(result.failed_count, 0);

    // Verify workouts were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workout_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, Some(2));

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_retry_mechanism(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    let config = BatchConfig {
        max_retries: 3,
        initial_backoff_ms: 10,
        max_backoff_ms: 100,
        ..Default::default()
    };
    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Use invalid data that will fail
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id, // No user exists, will fail
            recorded_at: Utc::now(),
            heart_rate: Some(70),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should have attempted retries
    assert!(result.retry_attempts > 0);
    assert!(result.retry_attempts <= 3);

    Ok(())
}

#[tokio::test]
async fn test_memory_limit_tracking() {
    let pool = setup_test_pool().await;
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_memory_{}@example.com", user_id)
    )
    .execute(&pool)
    .await
    .unwrap();

    let config = BatchConfig {
        memory_limit_mb: 100.0,
        enable_progress_tracking: true,
        ..Default::default()
    };
    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create a moderate batch
    let mut metrics = vec![];
    for i in 0..50 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: Utc::now() - chrono::Duration::seconds(i),
            heart_rate: Some(70),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should track memory usage
    assert!(result.memory_peak_mb.is_some());

    cleanup_test_data(&pool, user_id).await;
}

#[sqlx::test]
async fn test_all_metric_types(pool: PgPool) -> sqlx::Result<()> {
    let user_id = create_test_user_id();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_all_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let processor = BatchProcessor::new(pool.clone());

    // Create one of each metric type
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id,
            recorded_at: Utc::now(),
            heart_rate: Some(70),
            resting_heart_rate: None,
            heart_rate_variability: None,
            context: None,
            source_device: None,
        }),
        HealthMetric::BloodPressure(BloodPressureMetric {
            user_id,
            recorded_at: Utc::now() - chrono::Duration::minutes(1),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source_device: None,
        }),
        HealthMetric::Sleep(SleepMetric {
            user_id,
            sleep_start: Utc::now() - chrono::Duration::hours(8),
            sleep_end: Utc::now() - chrono::Duration::hours(1),
            duration_minutes: 420,
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(180),
            awake_minutes: Some(30),
            efficiency: Some(85.7),
            source_device: None,
        }),
        HealthMetric::Activity(ActivityMetric {
            user_id,
            recorded_at: Utc::now() - chrono::Duration::minutes(2),
            step_count: Some(5000),
            distance_meters: Some(4000.0),
            active_energy_burned_kcal: Some(200.0),
            basal_energy_burned_kcal: Some(1500.0),
            flights_climbed: Some(10),
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
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 4);
    assert_eq!(result.failed_count, 0);

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_batch_processing() {
    let pool = setup_test_pool().await;

    // Create multiple users
    let mut handles = vec![];

    for i in 0..5 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let user_id = create_test_user_id();

            sqlx::query!(
                "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
                user_id,
                format!("test_concurrent_{}_{:?}@example.com", i, user_id)
            )
            .execute(&pool_clone)
            .await
            .unwrap();

            let processor = BatchProcessor::new(pool_clone.clone());

            let metrics = vec![
                HealthMetric::HeartRate(HeartRateMetric {
                    user_id,
                    recorded_at: Utc::now() - chrono::Duration::seconds(i),
                    heart_rate: Some(60 + i as i32),
                    resting_heart_rate: None,
                    heart_rate_variability: None,
                    context: None,
                    source_device: None,
                }),
            ];

            let payload = IngestPayload {
                data: IngestData {
                    metrics,
                    workouts: vec![],
                },
            };

            let result = processor.process_batch(user_id, payload).await;

            cleanup_test_data(&pool_clone, user_id).await;

            result
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;

    // All should succeed
    for result in results {
        let batch_result = result.unwrap();
        assert_eq!(batch_result.processed_count, 1);
        assert_eq!(batch_result.failed_count, 0);
    }
}

#[test]
fn test_deduplication_stats_default() {
    let stats = DeduplicationStats::default();
    assert_eq!(stats.total_duplicates, 0);
    assert_eq!(stats.heart_rate_duplicates, 0);
    assert_eq!(stats.deduplication_time_ms, 0);
}

#[test]
fn test_batch_processing_result_default() {
    let result = BatchProcessingResult::default();
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.failed_count, 0);
    assert!(result.errors.is_empty());
    assert_eq!(result.processing_time_ms, 0);
    assert_eq!(result.retry_attempts, 0);
    assert!(result.memory_peak_mb.is_none());
}