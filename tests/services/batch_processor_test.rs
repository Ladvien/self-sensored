use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::time::{Duration, Instant};
use uuid::Uuid;
use tokio::time::sleep;

use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, BodyMeasurementMetric, HealthMetric, HeartRateMetric, IngestData,
    IngestPayload, RespiratoryMetric, SleepMetric, WorkoutData,
};
use self_sensored::services::batch_processor::{BatchConfig, BatchProcessor, ProcessingStatus};

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
        "heart_rate_metrics",
        "blood_pressure_metrics",
        "sleep_metrics",
        "activity_metrics",
        "body_metrics",
        "respiratory_metrics",
        "workouts",
        "users"
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

/// Create sample heart rate metrics
fn create_sample_heart_rate_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::HeartRate(HeartRateMetric {
                recorded_at: Utc::now() - chrono::Duration::minutes(i as i64),
                min_bpm: Some(60 + (i % 20) as i16),
                avg_bpm: Some(70 + (i % 30) as i16),
                max_bpm: Some(80 + (i % 40) as i16),
                source: Some("Apple Watch".to_string()),
                context: Some("resting".to_string()),
            })
        })
        .collect()
}

/// Create sample blood pressure metrics
fn create_sample_blood_pressure_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            HealthMetric::BloodPressure(BloodPressureMetric {
                recorded_at: Utc::now() - chrono::Duration::hours(i as i64),
                systolic: 120 + (i % 20) as i16,
                diastolic: 80 + (i % 10) as i16,
                pulse: Some(70 + (i % 15) as i16),
                source: Some("Manual Entry".to_string()),
            })
        })
        .collect()
}

/// Create sample sleep metrics
fn create_sample_sleep_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            let sleep_start = Utc::now() - chrono::Duration::days(i as i64 + 1) + chrono::Duration::hours(22);
            let sleep_end = sleep_start + chrono::Duration::hours(8);
            
            HealthMetric::Sleep(SleepMetric {
                recorded_at: sleep_end,
                sleep_start,
                sleep_end,
                total_sleep_minutes: 480 - (i % 60) as i32,
                deep_sleep_minutes: Some(120 - (i % 30) as i32),
                rem_sleep_minutes: Some(90 - (i % 20) as i32),
                awake_minutes: Some(10 + (i % 15) as i32),
                efficiency_percentage: Some(85.0 + (i % 10) as f32),
                source: Some("Sleep Tracker".to_string()),
            })
        })
        .collect()
}

/// Create sample activity metrics
fn create_sample_activity_metrics(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            let date = chrono::Utc::now().naive_utc().date() - chrono::Duration::days(i as i64);
            
            HealthMetric::Activity(ActivityMetric {
                date,
                steps: Some(10000 + (i * 100) as i32),
                distance_meters: Some(8000.0 + (i as f64 * 50.0)),
                calories_burned: Some(2500.0 + (i as f64 * 25.0)),
                active_minutes: Some(60 + (i % 30) as i32),
                flights_climbed: Some(10 + (i % 5) as i32),
                source: Some("Fitness Tracker".to_string()),
            })
        })
        .collect()
}

/// Create sample workout data
fn create_sample_workouts(count: usize) -> Vec<WorkoutData> {
    (0..count)
        .map(|i| {
            let start_time = Utc::now() - chrono::Duration::days(i as i64);
            
            WorkoutData {
                workout_type: if i % 3 == 0 { "Running" } else if i % 3 == 1 { "Cycling" } else { "Swimming" }.to_string(),
                start_time,
                end_time: start_time + chrono::Duration::minutes(30 + (i % 60) as i64),
                total_energy_kcal: Some(300.0 + (i as f64 * 10.0)),
                distance_meters: Some(5000.0 + (i as f64 * 100.0)),
                avg_heart_rate: Some(140 + (i % 20) as i16),
                max_heart_rate: Some(160 + (i % 30) as i16),
                source: Some("Workout App".to_string()),
            }
        })
        .collect()
}

#[tokio::test]
async fn test_batch_processor_basic_functionality() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Create batch processor
    let processor = BatchProcessor::new(pool.clone());
    
    // Create test payload with mixed metrics
    let mut metrics = Vec::new();
    metrics.extend(create_sample_heart_rate_metrics(5));
    metrics.extend(create_sample_blood_pressure_metrics(3));
    metrics.extend(create_sample_sleep_metrics(2));
    metrics.extend(create_sample_activity_metrics(4));
    
    let workouts = create_sample_workouts(3);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };
    
    // Process the batch
    let result = processor.process_batch(user_id, payload).await;
    
    // Verify results
    assert!(result.processed_count > 0, "Should have processed some metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(result.errors.is_empty(), "Should have no errors");
    assert!(result.processing_time_ms > 0, "Should have positive processing time");
    
    // Verify data was actually inserted
    let heart_rate_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    let blood_pressure_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    let sleep_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    let activity_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    let workout_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workouts WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert_eq!(heart_rate_count, 5);
    assert_eq!(blood_pressure_count, 3);
    assert_eq!(sleep_count, 2);
    assert_eq!(activity_count, 4);
    assert_eq!(workout_count, 3);
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_parallel_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Create config with parallel processing enabled
    let config = BatchConfig {
        enable_parallel_processing: true,
        ..BatchConfig::default()
    };
    
    let processor = BatchProcessor::with_config(pool.clone(), config);
    
    // Create larger payload to test parallel processing
    let mut metrics = Vec::new();
    metrics.extend(create_sample_heart_rate_metrics(50));
    metrics.extend(create_sample_blood_pressure_metrics(50));
    metrics.extend(create_sample_sleep_metrics(50));
    metrics.extend(create_sample_activity_metrics(50));
    
    let workouts = create_sample_workouts(50);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };
    
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();
    
    // Verify results
    assert!(result.processed_count > 0, "Should have processed metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(result.errors.is_empty(), "Should have no errors");
    
    // Parallel processing should be reasonably fast
    assert!(elapsed < Duration::from_secs(5), "Parallel processing should be fast");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_sequential_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Create config with parallel processing disabled
    let config = BatchConfig {
        enable_parallel_processing: false,
        ..BatchConfig::default()
    };
    
    let processor = BatchProcessor::with_config(pool.clone(), config);
    
    // Create test payload
    let mut metrics = Vec::new();
    metrics.extend(create_sample_heart_rate_metrics(10));
    metrics.extend(create_sample_blood_pressure_metrics(10));
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    
    // Verify results
    assert!(result.processed_count > 0, "Should have processed metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(result.errors.is_empty(), "Should have no errors");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test] 
async fn test_duplicate_handling() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Create identical heart rate metrics (same recorded_at timestamp)
    let recorded_at = Utc::now();
    let heart_rate = HeartRateMetric {
        recorded_at,
        min_bpm: Some(60),
        avg_bpm: Some(70),
        max_bpm: Some(80),
        source: Some("Test".to_string()),
        context: Some("resting".to_string()),
    };
    
    let metrics = vec![
        HealthMetric::HeartRate(heart_rate.clone()),
        HealthMetric::HeartRate(heart_rate.clone()),
        HealthMetric::HeartRate(heart_rate),
    ];
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    
    // Should process successfully (duplicates are handled by ON CONFLICT)
    assert!(result.processed_count >= 0, "Should handle duplicates gracefully");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    
    // Verify only one record was actually inserted
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert_eq!(count, 1, "Should only have one record after duplicate handling");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_retry_logic() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Create config with retry settings
    let config = BatchConfig {
        max_retries: 2,
        initial_backoff_ms: 50,
        max_backoff_ms: 200,
        ..BatchConfig::default()
    };
    
    let processor = BatchProcessor::with_config(pool.clone(), config);
    
    // Create metrics with valid data (retries will only be tested in error scenarios)
    let metrics = create_sample_heart_rate_metrics(5);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    
    // With valid data, should succeed without retries
    assert!(result.processed_count > 0, "Should have processed metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert_eq!(result.retry_attempts, 0, "Should have no retries for valid data");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_empty_payload() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Create empty payload
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    
    // Should handle empty payload gracefully
    assert_eq!(result.processed_count, 0, "Should process zero metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(result.errors.is_empty(), "Should have no errors");
    assert!(result.processing_time_ms >= 0, "Should have valid processing time");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_large_batch_performance() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Create large batch (1000 metrics)
    let mut metrics = Vec::new();
    metrics.extend(create_sample_heart_rate_metrics(250));
    metrics.extend(create_sample_blood_pressure_metrics(250));
    metrics.extend(create_sample_sleep_metrics(250));
    metrics.extend(create_sample_activity_metrics(250));
    
    let workouts = create_sample_workouts(100);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };
    
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();
    
    // Verify performance requirements (1000+ metrics should complete in reasonable time)
    assert!(result.processed_count >= 1000, "Should process all metrics");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(elapsed < Duration::from_secs(30), "Large batch should complete within 30 seconds");
    
    println!("Processed {} metrics in {:?}", result.processed_count, elapsed);
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test] 
async fn test_batch_config_options() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Test various configuration options
    let configs = vec![
        BatchConfig {
            enable_parallel_processing: true,
            chunk_size: 500,
            memory_limit_mb: 250.0,
            ..BatchConfig::default()
        },
        BatchConfig {
            enable_parallel_processing: false,
            chunk_size: 100,
            memory_limit_mb: 100.0,
            ..BatchConfig::default()
        },
    ];
    
    for (i, config) in configs.into_iter().enumerate() {
        let processor = BatchProcessor::with_config(pool.clone(), config);
        
        let metrics = create_sample_heart_rate_metrics(50);
        let payload = IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![],
            },
        };
        
        let result = processor.process_batch(user_id, payload).await;
        
        assert!(result.processed_count > 0, "Config {} should process metrics", i);
        assert_eq!(result.failed_count, 0, "Config {} should have no failures", i);
        
        // Clean up between configs
        sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_memory_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Create moderate batch to test memory tracking
    let metrics = create_sample_heart_rate_metrics(100);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    
    // Memory tracking should be present (even if zero for our simple implementation)
    assert!(result.memory_peak_mb.is_some(), "Should track memory usage");
    assert!(result.memory_peak_mb.unwrap() >= 0.0, "Memory usage should be non-negative");
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_processing_status_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Test that ProcessingStatus enum works correctly
    let statuses = [
        ProcessingStatus::Pending,
        ProcessingStatus::InProgress,
        ProcessingStatus::Completed,
        ProcessingStatus::Failed,
        ProcessingStatus::Retrying,
    ];
    
    // Verify all status variants are available
    for status in &statuses {
        match status {
            ProcessingStatus::Pending => assert_eq!(*status, ProcessingStatus::Pending),
            ProcessingStatus::InProgress => assert_eq!(*status, ProcessingStatus::InProgress),
            ProcessingStatus::Completed => assert_eq!(*status, ProcessingStatus::Completed),
            ProcessingStatus::Failed => assert_eq!(*status, ProcessingStatus::Failed),
            ProcessingStatus::Retrying => assert_eq!(*status, ProcessingStatus::Retrying),
        }
    }
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_batch_processor_counter_reset() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    let processor = BatchProcessor::new(pool.clone());
    
    // Reset counters (should not fail)
    processor.reset_counters();
    
    // Process some data
    let metrics = create_sample_heart_rate_metrics(5);
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let result = processor.process_batch(user_id, payload).await;
    assert!(result.processed_count > 0, "Should process metrics after counter reset");
    
    cleanup_test_data(&pool, user_id).await;
}

/// Benchmark test for 10K metrics requirement
#[tokio::test]
async fn benchmark_10k_metrics() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    
    // Use parallel processing for best performance
    let config = BatchConfig {
        enable_parallel_processing: true,
        chunk_size: 1000,
        ..BatchConfig::default()
    };
    
    let processor = BatchProcessor::with_config(pool.clone(), config);
    
    // Create exactly 10,000 metrics as per requirement
    let mut metrics = Vec::new();
    metrics.extend(create_sample_heart_rate_metrics(2500));
    metrics.extend(create_sample_blood_pressure_metrics(2500));
    metrics.extend(create_sample_sleep_metrics(2500));
    metrics.extend(create_sample_activity_metrics(2500));
    
    let workouts = create_sample_workouts(1000);
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };
    
    // Measure processing time
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();
    
    // Verify performance requirement: 10K metrics in < 10 seconds
    assert_eq!(result.processed_count, 11000, "Should process exactly 11,000 items"); // 10K metrics + 1K workouts
    assert_eq!(result.failed_count, 0, "Should have no failures");
    assert!(elapsed < Duration::from_secs(10), 
        "10K metrics should process in <10s, took {:?}", elapsed);
    
    println!("BENCHMARK: Processed 10,000 metrics + 1,000 workouts in {:?}", elapsed);
    println!("Performance: {:.2} items/sec", 11000.0 / elapsed.as_secs_f64());
    
    cleanup_test_data(&pool, user_id).await;
}
/// Test respiratory metrics batch processing with medical validation
#[tokio::test]
async fn test_respiratory_metrics_batch_processing() {
    use self_sensored::models::RespiratoryMetric;
    use self_sensored::config::ValidationConfig;

    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig {
        enable_parallel_processing: true,
        respiratory_chunk_size: 7000, // Optimized for 7 params per record
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create sample respiratory metrics with medical-grade data
    let mut respiratory_metrics = Vec::new();
    let now = Utc::now();

    for i in 0..1000 {
        let metric = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: now - chrono::Duration::minutes(i),
            respiratory_rate: Some(12 + (i % 8) as i32), // Normal range: 12-20 BPM
            oxygen_saturation: Some(95.0 + (i % 5) as f64), // Normal range: 95-100%
            forced_vital_capacity: Some(3.0 + (i % 3) as f64), // FVC in liters
            forced_expiratory_volume_1: Some(2.4 + (i % 2) as f64), // FEV1 in liters
            peak_expiratory_flow_rate: Some(400.0 + (i % 200) as f64), // PEFR in L/min
            inhaler_usage: if i % 10 == 0 { Some(1) } else { None }, // Occasional inhaler use
            source_device: Some("Pulse Oximeter".to_string()),
            created_at: now,
        };

        // Validate the metric using our validation logic
        let validation_config = ValidationConfig::default();
        assert!(metric.validate_with_config(&validation_config).is_ok(),
            "Respiratory metric should pass medical validation");

        // Test critical SpO2 detection
        if let Some(spo2) = metric.oxygen_saturation {
            let is_critical = metric.is_critical(&validation_config);
            if spo2 < 90.0 {
                assert!(is_critical, "SpO2 below 90% should be flagged as critical");
            }
        }

        respiratory_metrics.push(HealthMetric::Respiratory(metric));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics: respiratory_metrics,
            workouts: vec![],
        },
    };

    // Process the batch
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();

    // Verify processing results
    assert_eq!(result.processed_count, 1000, "Should process all respiratory metrics");
    assert_eq!(result.failed_count, 0, "Should have no processing failures");
    assert!(elapsed < Duration::from_secs(5),
        "1K respiratory metrics should process quickly, took {:?}", elapsed);

    println!("RESPIRATORY BATCH TEST: Processed 1,000 respiratory metrics in {:?}", elapsed);
    println!("Performance: {:.2} respiratory metrics/sec", 1000.0 / elapsed.as_secs_f64());

    cleanup_test_data(&pool, user_id).await;
}
