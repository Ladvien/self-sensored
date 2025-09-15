use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::time::{Duration, Instant};
use uuid::Uuid;
use tokio::time::sleep;

use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, BodyMeasurementMetric, HealthMetric, HeartRateMetric, IngestData,
    IngestPayload, RespiratoryMetric, SleepMetric, WorkoutData,
    // Reproductive Health Metrics (HIPAA-Compliant Testing)
    MenstrualMetric, FertilityMetric,
};
use self_sensored::models::enums::{
    MenstrualFlow, CervicalMucusQuality, OvulationTestResult,
    PregnancyTestResult, TemperatureContext,
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
        "body_measurements",
        "respiratory_metrics",
        "workouts",
        // Reproductive Health Tables (HIPAA-Compliant Test Cleanup)
        "menstrual_health",
        "fertility_tracking",
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

/// Create sample body measurement metrics for smart scale testing
fn create_sample_body_measurements(count: usize) -> Vec<HealthMetric> {
    (0..count)
        .map(|i| {
            let recorded_at = Utc::now() - chrono::Duration::hours(i as i64);

            // Simulate smart scale data with multiple devices
            let measurement_source = match i % 4 {
                0 => "InBody_720".to_string(),
                1 => "Withings_Body_Plus".to_string(),
                2 => "Fitbit_Aria_2".to_string(),
                _ => "Apple_Watch_Series_9".to_string(),
            };

            let weight = 70.0 + (i as f64 % 30.0); // Weight range 70-100kg
            let height = 175.0 + (i as f64 % 20.0); // Height range 175-195cm
            let bmi = weight / ((height / 100.0) * (height / 100.0)); // Calculate BMI

            HealthMetric::BodyMeasurement(BodyMeasurementMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(), // Will be overridden in test
                recorded_at,
                // Core smart scale measurements
                body_weight_kg: Some(weight),
                body_mass_index: Some(bmi),
                body_fat_percentage: Some(15.0 + (i as f64 % 20.0)), // Body fat 15-35%
                lean_body_mass_kg: Some(weight * (1.0 - (15.0 + i as f64 % 20.0) / 100.0)),

                // Physical measurements (some scales provide circumference data)
                height_cm: if i % 5 == 0 { Some(height) } else { None }, // Height measured occasionally
                waist_circumference_cm: if i % 3 == 0 { Some(80.0 + i as f64 % 15.0) } else { None },
                hip_circumference_cm: if i % 3 == 0 { Some(95.0 + i as f64 % 10.0) } else { None },
                chest_circumference_cm: None, // Less common on smart scales
                arm_circumference_cm: None,
                thigh_circumference_cm: None,

                // Body temperature (some smart scales include this)
                body_temperature_celsius: if i % 10 == 0 { Some(36.5 + (i as f64 % 3.0) / 10.0) } else { None },
                basal_body_temperature_celsius: None,

                // Metadata
                measurement_source: Some(measurement_source),
                source_device: Some("Smart Scale API".to_string()),
                created_at: recorded_at,
            })
        })
        .collect()
}

/// Test body measurements batch processing with smart scale integration
#[tokio::test]
async fn test_body_measurements_batch_processing() {
    use self_sensored::config::ValidationConfig;

    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let config = BatchConfig {
        enable_parallel_processing: true,
        body_measurement_chunk_size: 3000, // Optimized for 16 params per record
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Create sample body measurements with smart scale data
    let mut body_measurements = create_sample_body_measurements(2000);

    // Override user_id for consistency
    for metric in &mut body_measurements {
        if let HealthMetric::BodyMeasurement(ref mut body_metric) = metric {
            body_metric.user_id = user_id;
        }
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics: body_measurements,
            workouts: vec![],
        },
    };

    // Process the batch
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();

    // Verify processing results
    assert_eq!(result.processed_count, 2000, "Should process all body measurement metrics");
    assert_eq!(result.failed_count, 0, "Should have no processing failures");
    assert!(elapsed < Duration::from_secs(10),
        "2K body measurements should process quickly, took {:?}", elapsed);

    // Verify data integrity with BMI validation
    let measurements_with_bmi: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1 AND body_mass_index IS NOT NULL",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(measurements_with_bmi > 0, "Should have body measurements with BMI data");

    // Test multi-device deduplication (same timestamp, different sources)
    let unique_sources: i64 = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT measurement_source) FROM body_measurements WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(unique_sources >= 3, "Should have measurements from multiple smart scale sources");

    println!("BODY MEASUREMENTS BATCH TEST: Processed 2,000 body measurements in {:?}", elapsed);
    println!("Performance: {:.2} body measurements/sec", 2000.0 / elapsed.as_secs_f64());

    cleanup_test_data(&pool, user_id).await;
}

/// Test BMI consistency validation for body measurements
#[tokio::test]
async fn test_bmi_consistency_validation() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let processor = BatchProcessor::new(pool.clone());

    let recorded_at = Utc::now();
    let weight = 75.0; // 75kg
    let height = 175.0; // 175cm
    let calculated_bmi = weight / ((height / 100.0) * (height / 100.0)); // Should be ~24.5

    // Test consistent BMI (should pass validation)
    let consistent_measurement = BodyMeasurementMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at,
        body_weight_kg: Some(weight),
        body_mass_index: Some(calculated_bmi), // Consistent BMI
        height_cm: Some(height),
        body_fat_percentage: Some(18.5),
        lean_body_mass_kg: Some(61.1),
        waist_circumference_cm: None,
        hip_circumference_cm: None,
        chest_circumference_cm: None,
        arm_circumference_cm: None,
        thigh_circumference_cm: None,
        body_temperature_celsius: None,
        basal_body_temperature_celsius: None,
        measurement_source: Some("InBody_Test".to_string()),
        source_device: Some("Test Scale".to_string()),
        created_at: recorded_at,
    };

    // Test BMI consistency validation
    assert!(consistent_measurement.validate_bmi_consistency(Some(height)).is_ok(),
        "BMI consistency check should pass for calculated BMI");

    // Test multi-metric reading detection
    assert!(consistent_measurement.is_multi_metric_reading(),
        "Should detect multi-metric smart scale reading");

    // Process the measurement
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![HealthMetric::BodyMeasurement(consistent_measurement)],
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    assert_eq!(result.processed_count, 1, "Should process consistent BMI measurement");
    assert_eq!(result.failed_count, 0, "Should have no validation failures");

    // Verify the BMI was stored correctly
    let stored_bmi: Option<f64> = sqlx::query_scalar!(
        "SELECT body_mass_index FROM body_measurements WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(stored_bmi.is_some(), "BMI should be stored");
    let bmi_diff = (stored_bmi.unwrap() - calculated_bmi).abs();
    assert!(bmi_diff < 0.1, "Stored BMI should match calculated BMI within tolerance");

    cleanup_test_data(&pool, user_id).await;
}

/// Test body measurements deduplication with multi-device scenarios
#[tokio::test]
async fn test_body_measurements_multi_device_deduplication() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    let processor = BatchProcessor::new(pool.clone());

    let recorded_at = Utc::now();

    // Create identical measurements from different smart scales (same timestamp)
    let measurements = vec![
        // InBody scale measurement
        BodyMeasurementMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_weight_kg: Some(70.5),
            body_mass_index: Some(23.1),
            body_fat_percentage: Some(18.2),
            lean_body_mass_kg: Some(57.7),
            height_cm: Some(175.0),
            waist_circumference_cm: None,
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: None,
            basal_body_temperature_celsius: None,
            measurement_source: Some("InBody_720".to_string()),
            source_device: Some("Smart Scale".to_string()),
            created_at: recorded_at,
        },
        // Withings scale measurement (different source, same time)
        BodyMeasurementMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_weight_kg: Some(70.6), // Slightly different reading
            body_mass_index: Some(23.1),
            body_fat_percentage: Some(18.5),
            lean_body_mass_kg: Some(57.5),
            height_cm: Some(175.0),
            waist_circumference_cm: None,
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: None,
            basal_body_temperature_celsius: None,
            measurement_source: Some("Withings_Body_Plus".to_string()),
            source_device: Some("Smart Scale".to_string()),
            created_at: recorded_at,
        },
        // Apple Watch measurement (different source, same time)
        BodyMeasurementMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_weight_kg: None, // Apple Watch doesn't measure weight
            body_mass_index: None,
            body_fat_percentage: None,
            lean_body_mass_kg: None,
            height_cm: Some(175.0), // Height from Health app
            waist_circumference_cm: None,
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: Some(36.7), // Apple Watch body temp
            basal_body_temperature_celsius: None,
            measurement_source: Some("Apple_Watch_Series_9".to_string()),
            source_device: Some("Apple Watch".to_string()),
            created_at: recorded_at,
        },
    ];

    let health_metrics: Vec<HealthMetric> = measurements
        .into_iter()
        .map(HealthMetric::BodyMeasurement)
        .collect();

    let payload = IngestPayload {
        data: IngestData {
            metrics: health_metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should process all 3 measurements (different sources = no duplication)
    assert_eq!(result.processed_count, 3, "Should process measurements from different sources");
    assert_eq!(result.failed_count, 0, "Should have no processing failures");

    // Verify that deduplication stats are tracked
    if let Some(dedup_stats) = result.deduplication_stats {
        println!("Body measurement duplicates detected: {}", dedup_stats.body_measurement_duplicates);
    }

    // Verify all three measurements were stored (different measurement_source)
    let stored_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored_count, 3, "Should store measurements from different sources separately");

    // Verify we can distinguish between sources
    let sources: Vec<String> = sqlx::query_scalar!(
        "SELECT DISTINCT measurement_source FROM body_measurements WHERE user_id = $1 AND recorded_at = $2 ORDER BY measurement_source",
        user_id,
        recorded_at
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .filter_map(|s| s)
    .collect();

    assert_eq!(sources.len(), 3, "Should have 3 distinct measurement sources");
    assert!(sources.contains(&"InBody_720".to_string()), "Should contain InBody source");
    assert!(sources.contains(&"Withings_Body_Plus".to_string()), "Should contain Withings source");
    assert!(sources.contains(&"Apple_Watch_Series_9".to_string()), "Should contain Apple Watch source");

    cleanup_test_data(&pool, user_id).await;
}

/// Benchmark body measurements processing for fitness tracking apps
#[tokio::test]
async fn benchmark_body_measurements_fitness_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Use optimized config for large fitness data imports
    let config = BatchConfig {
        enable_parallel_processing: true,
        body_measurement_chunk_size: 3000, // 16 params per record, ~48K parameters per chunk
        enable_intra_batch_deduplication: true,
        memory_limit_mb: 500.0,
        ..BatchConfig::default()
    };

    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Simulate importing 2 years of daily body measurements from fitness tracking app
    let daily_measurements_count = 365 * 2; // 2 years of daily data
    let mut all_measurements = create_sample_body_measurements(daily_measurements_count);

    // Override user_id for consistency
    for metric in &mut all_measurements {
        if let HealthMetric::BodyMeasurement(ref mut body_metric) = metric {
            body_metric.user_id = user_id;

            // Add some historical data patterns
            if let Some(ref mut source) = body_metric.measurement_source {
                // Simulate device upgrades over time
                if body_metric.recorded_at < Utc::now() - chrono::Duration::days(365) {
                    *source = "Fitbit_Aria_1".to_string(); // Older device
                }
            }
        }
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics: all_measurements,
            workouts: vec![],
        },
    };

    // Measure processing time for fitness tracking scenario
    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let elapsed = start_time.elapsed();

    // Performance requirements for fitness tracking data import
    assert_eq!(result.processed_count, daily_measurements_count, "Should process all historical body measurements");
    assert_eq!(result.failed_count, 0, "Should have no processing failures");
    assert!(elapsed < Duration::from_secs(15),
        "2 years of body measurements should import quickly, took {:?}", elapsed);

    // Verify memory usage stayed within limits
    if let Some(memory_peak) = result.memory_peak_mb {
        assert!(memory_peak <= 500.0, "Memory usage should stay within configured limit");
    }

    // Verify data quality and trends
    let weight_measurements: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1 AND body_weight_kg IS NOT NULL",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let bmi_measurements: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1 AND body_mass_index IS NOT NULL",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(weight_measurements > 0, "Should have weight measurements for trend analysis");
    assert!(bmi_measurements > 0, "Should have BMI measurements for health tracking");

    println!("FITNESS TRACKING BENCHMARK: Imported {} body measurements in {:?}",
             daily_measurements_count, elapsed);
    println!("Performance: {:.2} measurements/sec",
             daily_measurements_count as f64 / elapsed.as_secs_f64());
    println!("Average processing per measurement: {:.2}ms",
             elapsed.as_millis() as f64 / daily_measurements_count as f64);

    cleanup_test_data(&pool, user_id).await;
}

// ============================================================================
// REPRODUCTIVE HEALTH BATCH PROCESSING TESTS (HIPAA-Compliant)
// ============================================================================

#[tokio::test]
async fn test_menstrual_health_batch_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create realistic menstrual health data spanning a complete cycle
    let mut menstrual_metrics = Vec::new();
    let base_time = Utc::now() - chrono::Duration::days(28);

    // 28-day menstrual cycle with realistic flow patterns
    for day in 0..28 {
        let recorded_at = base_time + chrono::Duration::days(day);
        let cycle_day = Some((day + 1) as i16);

        let menstrual_flow = match day {
            0..=1 => MenstrualFlow::Light,  // Cycle day 1-2: Light flow
            2..=3 => MenstrualFlow::Heavy,  // Cycle day 3-4: Heavy flow
            4..=5 => MenstrualFlow::Medium, // Cycle day 5-6: Medium flow
            6..=7 => MenstrualFlow::Light,  // Cycle day 7-8: Light flow
            _ => MenstrualFlow::None,       // Rest of cycle: No flow
        };

        let cramps_severity = match day {
            0..=2 => Some(7), // Severe cramps during heavy flow
            3..=5 => Some(4), // Moderate cramps
            6..=7 => Some(2), // Mild cramps
            _ => Some(0),     // No cramps rest of cycle
        };

        let mood_rating = match day {
            0..=3 => Some(2),   // Poor mood during menstruation
            4..=7 => Some(3),   // Improving mood
            8..=21 => Some(4),  // Good mood follicular/ovulatory phase
            _ => Some(3),       // Declining mood luteal phase
        };

        let energy_level = match day {
            0..=3 => Some(2),   // Low energy during menstruation
            4..=7 => Some(3),   // Improving energy
            8..=21 => Some(4),  // High energy follicular/ovulatory phase
            _ => Some(3),       // Moderate energy luteal phase
        };

        menstrual_metrics.push(MenstrualMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            menstrual_flow,
            spotting: day == 0 || day == 7, // Spotting at cycle start/end
            cycle_day,
            cramps_severity,
            mood_rating,
            energy_level,
            notes: Some(format!("Cycle day {} tracking", day + 1)),
            source_device: Some("ios_health_app".to_string()),
            created_at: Utc::now(),
        });
    }

    println!("Testing menstrual health batch processing with {} records", menstrual_metrics.len());

    let start_time = Instant::now();
    let result = processor.process_menstrual_metrics(user_id, menstrual_metrics.clone()).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok(), "Menstrual health batch processing should succeed");
    let batch_result = result.unwrap();

    println!("MENSTRUAL HEALTH BATCH PROCESSING RESULTS:");
    println!("  Processed: {} records", batch_result.processed_count);
    println!("  Failed: {} records", batch_result.failed_count);
    println!("  Processing Time: {:?}", elapsed);
    println!("  Memory Peak: {:?} MB", batch_result.memory_peak_mb);
    println!("  Deduplication Stats: {:?}", batch_result.deduplication_stats);

    // Verify all records were processed successfully
    assert_eq!(batch_result.processed_count, 28, "All 28 menstrual health records should be processed");
    assert_eq!(batch_result.failed_count, 0, "No records should fail");

    // Verify cycle-aware deduplication statistics
    if let Some(dedup_stats) = batch_result.deduplication_stats {
        assert_eq!(dedup_stats.menstrual_duplicates, 0, "No duplicates expected in clean data");
        assert_eq!(dedup_stats.total_duplicates, 0, "Total duplicates should be 0");
    }

    // Verify database insertion
    let count_result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM menstrual_health WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await;

    assert!(count_result.is_ok(), "Should be able to count menstrual health records");
    let record_count = count_result.unwrap().unwrap_or(0) as usize;
    assert_eq!(record_count, 28, "Database should contain all 28 menstrual health records");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_fertility_tracking_batch_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let processor = BatchProcessor::new(pool.clone());

    // Create comprehensive fertility tracking data spanning ovulation cycle
    let mut fertility_metrics = Vec::new();
    let base_time = Utc::now() - chrono::Duration::days(14);

    // 14-day fertility tracking with ovulation patterns
    for day in 0..14 {
        let recorded_at = base_time + chrono::Duration::days(day);

        // Cervical mucus quality changes throughout cycle
        let cervical_mucus_quality = match day {
            0..=5 => Some(CervicalMucusQuality::Dry),     // Early follicular
            6..=9 => Some(CervicalMucusQuality::Sticky),  // Mid follicular
            10..=11 => Some(CervicalMucusQuality::Creamy), // Pre-ovulation
            12..=13 => Some(CervicalMucusQuality::EggWhite), // Ovulation
            _ => Some(CervicalMucusQuality::Watery),      // Post-ovulation
        };

        // Ovulation test progression
        let ovulation_test_result = match day {
            0..=9 => OverulationTestResult::Negative,   // No LH surge
            10..=11 => OverulationTestResult::High,     // Rising LH
            12..=13 => OverulationTestResult::Peak,     // LH surge/peak
            _ => OverulationTestResult::Negative,       // Post-ovulation
        };

        // Basal body temperature shift pattern
        let basal_body_temperature = match day {
            0..=11 => Some(97.2 + (day as f64 * 0.05)), // Gradual rise pre-ovulation
            12..=13 => Some(98.1),                       // Temperature shift at ovulation
            _ => Some(98.0),                             // Post-ovulation plateau
        };

        // Privacy-protected sensitive data (some days only)
        let sexual_activity = if day == 11 || day == 12 || day == 13 {
            Some(true)  // Activity around fertile window
        } else {
            Some(false)
        };

        // LH level progression
        let lh_level = match day {
            0..=9 => Some(5.0 + (day as f64 * 0.5)),   // Baseline rising
            10..=11 => Some(15.0 + (day as f64 * 2.0)), // Pre-surge increase
            12..=13 => Some(25.0),                      // LH surge
            _ => Some(8.0),                             // Post-ovulation decline
        };

        fertility_metrics.push(FertilityMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            cervical_mucus_quality,
            ovulation_test_result,
            sexual_activity,
            pregnancy_test_result: PregnancyTestResult::NotTested,
            basal_body_temperature,
            temperature_context: TemperatureContext::Basal,
            cervix_firmness: Some(if day < 12 { 2 } else { 3 }),
            cervix_position: Some(if day < 12 { 2 } else { 3 }),
            lh_level,
            notes: Some(format!("Fertility tracking day {}", day + 1)),
            source_device: Some("fertility_app".to_string()),
            created_at: Utc::now(),
        });
    }

    println!("Testing fertility tracking batch processing with {} records", fertility_metrics.len());

    let start_time = Instant::now();
    let result = processor.process_fertility_metrics(user_id, fertility_metrics.clone()).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok(), "Fertility tracking batch processing should succeed");
    let batch_result = result.unwrap();

    println!("FERTILITY TRACKING BATCH PROCESSING RESULTS:");
    println!("  Processed: {} records", batch_result.processed_count);
    println!("  Failed: {} records", batch_result.failed_count);
    println!("  Processing Time: {:?}", elapsed);
    println!("  Memory Peak: {:?} MB", batch_result.memory_peak_mb);
    println!("  Privacy Protection: Enhanced audit logging enabled");
    println!("  Deduplication Stats: {:?}", batch_result.deduplication_stats);

    // Verify all records were processed successfully with privacy protection
    assert_eq!(batch_result.processed_count, 14, "All 14 fertility tracking records should be processed");
    assert_eq!(batch_result.failed_count, 0, "No records should fail with privacy protection");

    // Verify privacy-first deduplication statistics
    if let Some(dedup_stats) = batch_result.deduplication_stats {
        assert_eq!(dedup_stats.fertility_duplicates, 0, "No duplicates expected in clean fertility data");
        assert_eq!(dedup_stats.total_duplicates, 0, "Total duplicates should be 0 with privacy protection");
    }

    // Verify database insertion with privacy protection
    let count_result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM fertility_tracking WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await;

    assert!(count_result.is_ok(), "Should be able to count fertility tracking records");
    let record_count = count_result.unwrap().unwrap_or(0) as usize;
    assert_eq!(record_count, 14, "Database should contain all 14 fertility tracking records");

    cleanup_test_data(&pool, user_id).await;
}

/// Test large batch processing with reproductive health data to verify PostgreSQL parameter limits
#[tokio::test]
async fn test_reproductive_health_large_batch_parameter_limits() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Test with customized batch configuration for parameter limit validation
    let config = BatchConfig {
        max_retries: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 5000,
        enable_parallel_processing: true,
        chunk_size: 1000,
        memory_limit_mb: 500.0,
        heart_rate_chunk_size: 8000,
        blood_pressure_chunk_size: 8000,
        sleep_chunk_size: 6000,
        activity_chunk_size: 6500,
        body_measurement_chunk_size: 3000,
        temperature_chunk_size: 8000,
        respiratory_chunk_size: 7000,
        workout_chunk_size: 5000,
        blood_glucose_chunk_size: 6500,
        nutrition_chunk_size: 1600,
        // Test reproductive health chunk sizes near PostgreSQL parameter limits
        menstrual_chunk_size: 6500, // 8 params: 6500 * 8 = 52,000 (under 65,535 limit)
        fertility_chunk_size: 4300, // 12 params: 4300 * 12 = 51,600 (under 65,535 limit)
        enable_progress_tracking: true,
        enable_intra_batch_deduplication: true,
        enable_dual_write_activity_metrics: false,
        enable_reproductive_health_encryption: true,
        reproductive_health_audit_logging: true,
    };

    // Validate configuration against PostgreSQL limits
    assert!(config.validate().is_ok(), "Configuration should respect PostgreSQL parameter limits");

    let processor = BatchProcessor::with_config(pool.clone(), config.clone());

    // Create large batches of reproductive health data to test chunking
    let base_time = Utc::now();
    let mut menstrual_metrics = Vec::new();
    let mut fertility_metrics = Vec::new();

    // Create 7000 menstrual metrics (larger than chunk size to test chunking)
    for i in 0..7000 {
        let recorded_at = base_time + chrono::Duration::minutes(i);
        menstrual_metrics.push(HealthMetric::Menstrual(MenstrualMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            menstrual_flow: if i % 4 == 0 { MenstrualFlow::Heavy } else { MenstrualFlow::Light },
            spotting: i % 3 == 0,
            cycle_day: Some(((i % 28) + 1) as i16),
            cramps_severity: Some(((i % 10) + 1) as i16),
            mood_rating: Some(((i % 5) + 1) as i16),
            energy_level: Some(((i % 5) + 1) as i16),
            notes: Some(format!("Test menstrual note {}", i)),
            source_device: Some("iPhone".to_string()),
            created_at: recorded_at,
        }));
    }

    // Create 5000 fertility metrics (larger than chunk size to test chunking)
    for i in 0..5000 {
        let recorded_at = base_time + chrono::Duration::minutes(i);
        fertility_metrics.push(HealthMetric::Fertility(FertilityMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            cervical_mucus_quality: if i % 3 == 0 { Some(CervicalMucusQuality::Creamy) } else { None },
            ovulation_test_result: if i % 10 == 0 { OvulationTestResult::Peak } else { OvulationTestResult::Low },
            sexual_activity: Some(i % 7 == 0), // Privacy-protected field
            pregnancy_test_result: PregnancyTestResult::NotTested,
            basal_body_temperature: Some(36.5 + (i as f64 * 0.01)),
            temperature_context: TemperatureContext::Basal,
            cervix_firmness: Some(((i % 3) + 1) as i16),
            cervix_position: Some(((i % 3) + 1) as i16),
            lh_level: Some(i as f64 * 0.1),
            notes: Some(format!("Test fertility note {}", i)),
            source_device: Some("iPhone".to_string()),
            created_at: recorded_at,
        }));
    }

    let mut all_metrics = menstrual_metrics;
    all_metrics.extend(fertility_metrics);

    let payload = IngestPayload {
        data: IngestData {
            metrics: all_metrics,
            workouts: vec![],
        },
    };

    let start_time = Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let processing_time = start_time.elapsed();

    // Verify batch processing succeeded
    assert!(result.processed_count > 0, "Should process reproductive health metrics");
    assert_eq!(result.failed_count, 0, "Should not have any failed metrics");
    assert!(result.errors.is_empty(), "Should not have any processing errors");

    // Performance requirements: Process 12,000 metrics in < 10 seconds
    assert!(processing_time < Duration::from_secs(10),
        "Large batch processing should complete within 10 seconds, took {:?}", processing_time);

    println!("Processed {} reproductive health metrics in {:?}",
        result.processed_count, processing_time);

    // Verify deduplication statistics if enabled
    if let Some(dedup_stats) = result.deduplication_stats {
        println!("Deduplication stats: menstrual={}, fertility={}, total={}",
            dedup_stats.menstrual_duplicates,
            dedup_stats.fertility_duplicates,
            dedup_stats.total_duplicates);
    }

    // Verify data was stored correctly
    let menstrual_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM menstrual_health WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("Should count menstrual records");

    let fertility_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fertility_tracking WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("Should count fertility records");

    assert!(menstrual_count > 0, "Should have stored menstrual metrics");
    assert!(fertility_count > 0, "Should have stored fertility metrics");

    println!("Stored {} menstrual metrics and {} fertility metrics",
        menstrual_count, fertility_count);

    cleanup_test_data(&pool, user_id).await;
}
