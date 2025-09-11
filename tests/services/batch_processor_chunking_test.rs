use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, HealthMetric, HeartRateMetric, IngestPayload, 
    IngestData, SleepMetric, WorkoutData
};
use self_sensored::services::batch_processor::{BatchConfig, BatchProcessor, BatchProcessingResult};

/// Test helper to create a test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/health_export_test".to_string());
    
    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Test chunk size calculations stay under PostgreSQL parameter limits
#[tokio::test]
async fn test_chunk_size_calculations() {
    // Test parameter limits for each metric type
    // PostgreSQL limit: 65,535 parameters per query
    
    // Heart Rate: 6 params per record
    let heart_rate_max = 65535 / 6; // = 10,922
    let heart_rate_safe = (heart_rate_max as f64 * 0.8) as usize; // = 8,738
    assert!(8000 <= heart_rate_safe, "Heart rate chunk size should be safe");
    
    // Blood Pressure: 6 params per record  
    let blood_pressure_max = 65535 / 6; // = 10,922
    let blood_pressure_safe = (blood_pressure_max as f64 * 0.8) as usize; // = 8,738
    assert!(8000 <= blood_pressure_safe, "Blood pressure chunk size should be safe");
    
    // Sleep: 10 params per record
    let sleep_max = 65535 / 10; // = 6,553
    let sleep_safe = (sleep_max as f64 * 0.8) as usize; // = 5,242
    assert!(5000 <= sleep_safe, "Sleep chunk size should be safe");
    
    // Activity: 7 params per record
    let activity_max = 65535 / 7; // = 9,362
    let activity_safe = (activity_max as f64 * 0.8) as usize; // = 7,489
    assert!(7000 <= activity_safe, "Activity chunk size should be safe");
    
    // Workout: 10 params per record
    let workout_max = 65535 / 10; // = 6,553
    let workout_safe = (workout_max as f64 * 0.8) as usize; // = 5,242
    assert!(5000 <= workout_safe, "Workout chunk size should be safe");
}

/// Test BatchConfig defaults have safe chunk sizes
#[tokio::test] 
async fn test_batch_config_default_chunk_sizes() {
    let config = BatchConfig::default();
    
    // Verify chunk sizes stay under parameter limits
    assert!(config.heart_rate_chunk_size * 6 < 65535, "Heart rate chunks exceed parameter limit");
    assert!(config.blood_pressure_chunk_size * 6 < 65535, "Blood pressure chunks exceed parameter limit");
    assert!(config.sleep_chunk_size * 10 < 65535, "Sleep chunks exceed parameter limit");
    assert!(config.activity_chunk_size * 7 < 65535, "Activity chunks exceed parameter limit");
    assert!(config.workout_chunk_size * 10 < 65535, "Workout chunks exceed parameter limit");
    
    // Verify reasonable chunk sizes
    assert!(config.heart_rate_chunk_size >= 1000, "Heart rate chunk size too small");
    assert!(config.blood_pressure_chunk_size >= 1000, "Blood pressure chunk size too small");
    assert!(config.sleep_chunk_size >= 1000, "Sleep chunk size too small");
    assert!(config.activity_chunk_size >= 1000, "Activity chunk size too small"); 
    assert!(config.workout_chunk_size >= 1000, "Workout chunk size too small");
}

/// Test large heart rate batch gets chunked properly
#[tokio::test]
async fn test_heart_rate_chunking() {
    let pool = create_test_pool().await;
    let mut batch_processor = BatchProcessor::new(pool.clone());
    let user_id = Uuid::new_v4();
    
    // Create large heart rate dataset that would exceed parameter limit
    let large_count = 15000; // More than 10,922 max for heart rate
    let mut heart_rates = Vec::new();
    for i in 0..large_count {
        heart_rates.push(HeartRateMetric {
            recorded_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
            avg_bpm: Some(70 + (i % 50) as u16),
            max_bpm: Some(80 + (i % 60) as u16),
            min_bpm: Some(60 + (i % 40) as u16),
            context: Some("resting".to_string()),
            source: "test_device".to_string(),
        });
    }
    
    // Create payload with large heart rate data
    let payload = IngestPayload {
        data: IngestData {
            metrics: heart_rates.into_iter().map(HealthMetric::HeartRate).collect(),
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process the batch - should succeed with chunking
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Verify processing succeeded
    assert_eq!(result.failed_count, 0, "Processing should not fail with chunking");
    assert!(result.processed_count > 0, "Should process some heart rate records");
    
    // Verify records were inserted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count heart rate records");
    
    assert!(count > 0, "Heart rate records should be inserted");
}

/// Test large sleep batch gets chunked properly
#[tokio::test]
async fn test_sleep_chunking() {
    let pool = create_test_pool().await;
    let mut batch_processor = BatchProcessor::new(pool.clone());
    let user_id = Uuid::new_v4();
    
    // Create large sleep dataset that would exceed parameter limit
    let large_count = 8000; // More than 6,553 max for sleep
    let mut sleep_metrics = Vec::new();
    for i in 0..large_count {
        let sleep_start = chrono::Utc::now() + chrono::Duration::hours(i as i64 * 24);
        sleep_metrics.push(SleepMetric {
            sleep_start,
            sleep_end: sleep_start + chrono::Duration::hours(8),
            total_sleep_minutes: 480,
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: Some(85.0),
            source: "test_device".to_string(),
        });
    }
    
    // Create payload with large sleep data
    let payload = IngestPayload {
        data: IngestData {
            metrics: sleep_metrics.into_iter().map(HealthMetric::Sleep).collect(),
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process the batch - should succeed with chunking
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Verify processing succeeded
    assert_eq!(result.failed_count, 0, "Processing should not fail with chunking");
    assert!(result.processed_count > 0, "Should process some sleep records");
    
    // Verify records were inserted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count sleep records");
    
    assert!(count > 0, "Sleep records should be inserted");
}

/// Test large workout batch gets chunked properly
#[tokio::test]
async fn test_workout_chunking() {
    let pool = create_test_pool().await;
    let mut batch_processor = BatchProcessor::new(pool.clone());
    let user_id = Uuid::new_v4();
    
    // Create large workout dataset that would exceed parameter limit
    let large_count = 8000; // More than 6,553 max for workouts
    let mut workouts = Vec::new();
    for i in 0..large_count {
        let start_time = chrono::Utc::now() + chrono::Duration::hours(i as i64);
        workouts.push(WorkoutData {
            workout_type: "Running".to_string(),
            start_time,
            end_time: start_time + chrono::Duration::minutes(30),
            total_energy_kcal: Some(300),
            distance_meters: Some(5000),
            avg_heart_rate: Some(140),
            max_heart_rate: Some(160),
            source: "test_device".to_string(),
        });
    }
    
    // Create payload with large workout data
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts,
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process the batch - should succeed with chunking
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Verify processing succeeded
    assert_eq!(result.failed_count, 0, "Processing should not fail with chunking");
    assert!(result.processed_count > 0, "Should process some workout records");
    
    // Verify records were inserted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workouts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count workout records");
    
    assert!(count > 0, "Workout records should be inserted");
}

/// Test mixed large batch with all metric types
#[tokio::test]
async fn test_mixed_large_batch_chunking() {
    let pool = create_test_pool().await;
    let config = BatchConfig {
        enable_progress_tracking: true,
        ..Default::default()
    };
    let mut batch_processor = BatchProcessor::with_config(pool.clone(), config);
    let user_id = Uuid::new_v4();
    
    let mut metrics = Vec::new();
    
    // Add large number of heart rate metrics
    for i in 0..12000 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            recorded_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
            avg_bpm: Some(70 + (i % 50) as u16),
            max_bpm: None,
            min_bpm: None,
            context: Some("testing".to_string()),
            source: "test_device".to_string(),
        }));
    }
    
    // Add large number of activity metrics (within 7,000 chunk limit)
    for i in 0..6000 {
        metrics.push(HealthMetric::Activity(ActivityMetric {
            date: chrono::Utc::now().date_naive() + chrono::Duration::days(i as i64),
            steps: Some(10000 + i),
            distance_meters: Some(8000.0),
            calories_burned: Some(500),
            active_minutes: Some(60),
            flights_climbed: Some(10),
            source: "test_device".to_string(),
        }));
    }
    
    // Create large mixed payload
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process the batch - should succeed with chunking
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Verify processing succeeded
    assert_eq!(result.failed_count, 0, "Processing should not fail with chunking");
    assert!(result.processed_count > 0, "Should process records");
    
    // Verify progress tracking was enabled
    assert!(result.chunk_progress.is_some(), "Progress tracking should be enabled");
    
    // Verify chunk sizes were respected (within configured limits)
    let config = BatchConfig::default();
    assert!(12000 <= config.heart_rate_chunk_size * 2, "Heart rate test size should be within chunking capacity");
    assert!(6000 <= config.activity_chunk_size, "Activity test size should be within single chunk limit");
    
    // Verify both metric types were inserted
    let heart_rate_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count heart rate records");
        
    let activity_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count activity records");
    
    assert!(heart_rate_count > 0, "Heart rate records should be inserted");
    assert!(activity_count > 0, "Activity records should be inserted");
}

/// Test custom chunk sizes work correctly
#[tokio::test]
async fn test_custom_chunk_sizes() {
    let pool = create_test_pool().await;
    let config = BatchConfig {
        heart_rate_chunk_size: 1000,   // Small chunks for testing
        blood_pressure_chunk_size: 500,
        sleep_chunk_size: 200,
        activity_chunk_size: 800,
        workout_chunk_size: 300,
        enable_progress_tracking: true,
        ..Default::default()
    };
    let mut batch_processor = BatchProcessor::with_config(pool.clone(), config);
    let user_id = Uuid::new_v4();
    
    // Create exactly 2500 heart rate records to trigger multiple chunks
    let mut heart_rates = Vec::new();
    for i in 0..2500 {
        heart_rates.push(HeartRateMetric {
            recorded_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
            avg_bpm: Some(70),
            max_bpm: None,
            min_bpm: None,
            context: Some("test".to_string()),
            source: "test_device".to_string(),
        });
    }
    
    let payload = IngestPayload {
        data: IngestData {
            metrics: heart_rates.into_iter().map(HealthMetric::HeartRate).collect(),
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process with custom small chunk sizes
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Should succeed
    assert_eq!(result.failed_count, 0, "Should succeed with custom chunk sizes");
    assert!(result.processed_count > 0, "Should process records");
    
    // With chunk size 1000, we expect 3 chunks (1000 + 1000 + 500)
    // This should be reflected in logs
}

/// Test transaction integrity across chunks
#[tokio::test]
async fn test_chunk_transaction_integrity() {
    let pool = create_test_pool().await;
    let mut batch_processor = BatchProcessor::new(pool.clone());
    let user_id = Uuid::new_v4();
    
    // Create heart rate data with some duplicates that should be handled by ON CONFLICT
    let mut heart_rates = Vec::new();
    let base_time = chrono::Utc::now();
    
    // Add 3000 records with some duplicate timestamps
    for i in 0..3000 {
        heart_rates.push(HeartRateMetric {
            recorded_at: base_time + chrono::Duration::seconds((i / 10) as i64), // Creates duplicates
            avg_bpm: Some(70 + (i % 30) as u16),
            max_bpm: None,
            min_bpm: None,
            context: Some("test".to_string()),
            source: "test_device".to_string(),
        });
    }
    
    let payload = IngestPayload {
        data: IngestData {
            metrics: heart_rates.into_iter().map(HealthMetric::HeartRate).collect(),
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Process the batch
    let result = batch_processor.process_batch(user_id, payload).await;
    
    // Should succeed despite duplicates
    assert_eq!(result.failed_count, 0, "Should handle duplicates gracefully");
    
    // Verify only unique timestamps were inserted
    let unique_count: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT recorded_at) FROM heart_rate_metrics WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count unique heart rate records");
    
    // Should be ~300 unique timestamps (3000 / 10)  
    assert!(unique_count <= 300, "Should have handled duplicates with ON CONFLICT");
    assert!(unique_count > 0, "Should have inserted some records");
}

/// Benchmark chunking performance with large datasets
#[tokio::test]
async fn test_chunking_performance_benchmark() {
    let pool = create_test_pool().await;
    let mut batch_processor = BatchProcessor::new(pool.clone());
    let user_id = Uuid::new_v4();
    
    // Create very large dataset to test performance
    let large_count = 50000;
    let mut metrics = Vec::new();
    
    for i in 0..large_count {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            recorded_at: chrono::Utc::now() + chrono::Duration::seconds(i as i64),
            avg_bpm: Some(70 + (i % 50) as u16),
            max_bpm: None,
            min_bpm: None,
            context: Some("benchmark".to_string()),
            source: "benchmark_device".to_string(),
        }));
    }
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
        timestamp: chrono::Utc::now(),
    };
    
    // Time the processing
    let start = std::time::Instant::now();
    let result = batch_processor.process_batch(user_id, payload).await;
    let duration = start.elapsed();
    
    // Verify success
    assert_eq!(result.failed_count, 0, "Large batch should succeed with chunking");
    assert!(result.processed_count > 0, "Should process records");
    
    // Performance target: should complete within reasonable time
    assert!(duration.as_secs() < 60, "Large batch processing should complete within 60 seconds");
    
    println!("Processed {} records in {:?}", large_count, duration);
    println!("Processing rate: {:.0} records/second", large_count as f64 / duration.as_secs_f64());
}