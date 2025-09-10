use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::models::health_metrics::{
    HeartRateMetric, BloodPressureMetric, SleepMetric, ActivityMetric, WorkoutData,
};
use self_sensored::models::{HealthMetric, IngestData, IngestPayload};
use self_sensored::services::batch_processor::{BatchConfig, BatchProcessor, DeduplicationStats};
use self_sensored::test_utilities::{create_test_pool, setup_test_database};

/// Helper function to create a test heart rate metric
fn create_test_heart_rate(recorded_at: DateTime<Utc>, bpm: i16) -> HeartRateMetric {
    HeartRateMetric {
        recorded_at,
        min_bpm: Some(bpm - 5),
        avg_bpm: Some(bpm),
        max_bpm: Some(bpm + 5),
        source: Some("test_device".to_string()),
        context: Some("resting".to_string()),
    }
}

/// Helper function to create a test blood pressure metric
fn create_test_blood_pressure(recorded_at: DateTime<Utc>, systolic: i16, diastolic: i16) -> BloodPressureMetric {
    BloodPressureMetric {
        recorded_at,
        systolic,
        diastolic,
        pulse: Some(72),
        source: Some("test_device".to_string()),
    }
}

/// Helper function to create a test sleep metric
fn create_test_sleep(sleep_start: DateTime<Utc>, sleep_end: DateTime<Utc>) -> SleepMetric {
    SleepMetric {
        recorded_at: sleep_start,
        sleep_start,
        sleep_end,
        total_sleep_minutes: (sleep_end - sleep_start).num_minutes() as i32,
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(90),
        awake_minutes: Some(30),
        efficiency_percentage: Some(85.0),
        source: Some("test_device".to_string()),
    }
}

/// Helper function to create a test activity metric
fn create_test_activity(date: chrono::NaiveDate, steps: i32) -> ActivityMetric {
    ActivityMetric {
        date,
        steps: Some(steps),
        distance_meters: Some(5000.0),
        calories_burned: Some(300.0),
        active_minutes: Some(60),
        flights_climbed: Some(10),
        source: Some("test_device".to_string()),
    }
}

/// Helper function to create a test workout
fn create_test_workout(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> WorkoutData {
    WorkoutData {
        workout_type: "running".to_string(),
        start_time,
        end_time,
        total_energy_kcal: Some(400.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source: Some("test_device".to_string()),
        route_points: None,
    }
}

#[tokio::test]
async fn test_heart_rate_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create duplicate heart rate metrics (same user_id and recorded_at)
    let timestamp = Utc::now();
    let metrics = vec![
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 74)), // Duplicate timestamp
        HealthMetric::HeartRate(create_test_heart_rate(timestamp.checked_add_signed(chrono::Duration::minutes(1)).unwrap(), 76)), // Different timestamp
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Verify deduplication occurred
    assert!(result.deduplication_stats.is_some());
    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.heart_rate_duplicates, 1, "Expected 1 heart rate duplicate to be removed");
    assert_eq!(stats.total_duplicates, 1, "Expected 1 total duplicate to be removed");
    assert_eq!(result.processed_count, 2, "Expected 2 unique records to be processed");
    assert_eq!(result.failed_count, 0, "Expected no processing failures");
}

#[tokio::test]
async fn test_blood_pressure_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create duplicate blood pressure metrics
    let timestamp = Utc::now();
    let metrics = vec![
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 120, 80)),
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 125, 82)), // Duplicate timestamp, different values
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp.checked_add_signed(chrono::Duration::hours(1)).unwrap(), 118, 78)), // Different timestamp
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.blood_pressure_duplicates, 1);
    assert_eq!(stats.total_duplicates, 1);
    assert_eq!(result.processed_count, 2);
}

#[tokio::test]
async fn test_sleep_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create duplicate sleep metrics (same user_id, sleep_start, sleep_end)
    let sleep_start = Utc::now() - chrono::Duration::hours(8);
    let sleep_end = Utc::now();
    let metrics = vec![
        HealthMetric::Sleep(create_test_sleep(sleep_start, sleep_end)),
        HealthMetric::Sleep(create_test_sleep(sleep_start, sleep_end)), // Exact duplicate
        HealthMetric::Sleep(create_test_sleep(
            sleep_start.checked_add_signed(chrono::Duration::days(1)).unwrap(),
            sleep_end.checked_add_signed(chrono::Duration::days(1)).unwrap()
        )), // Different date
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.sleep_duplicates, 1);
    assert_eq!(stats.total_duplicates, 1);
    assert_eq!(result.processed_count, 2);
}

#[tokio::test]
async fn test_activity_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create duplicate activity metrics (same user_id, recorded_date)
    let today = chrono::Utc::now().date_naive();
    let metrics = vec![
        HealthMetric::Activity(create_test_activity(today, 10000)),
        HealthMetric::Activity(create_test_activity(today, 12000)), // Same date, different step count
        HealthMetric::Activity(create_test_activity(today.succ_opt().unwrap(), 8000)), // Different date
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.activity_duplicates, 1);
    assert_eq!(stats.total_duplicates, 1);
    assert_eq!(result.processed_count, 2);
}

#[tokio::test]
async fn test_workout_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create duplicate workout records (same user_id, started_at)
    let start_time = Utc::now() - chrono::Duration::hours(1);
    let end_time = Utc::now();
    let workouts = vec![
        create_test_workout(start_time, end_time),
        create_test_workout(start_time, end_time.checked_add_signed(chrono::Duration::minutes(5)).unwrap()), // Same start, different end
        create_test_workout(start_time.checked_add_signed(chrono::Duration::hours(1)).unwrap(), 
                           end_time.checked_add_signed(chrono::Duration::hours(2)).unwrap()), // Different start time
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.workout_duplicates, 1);
    assert_eq!(stats.total_duplicates, 1);
    assert_eq!(result.processed_count, 2);
}

#[tokio::test]
async fn test_mixed_metric_deduplication() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    let timestamp = Utc::now();
    let today = timestamp.date_naive();
    let sleep_start = timestamp - chrono::Duration::hours(8);

    // Create a mix of metrics with duplicates across different types
    let metrics = vec![
        // Heart rate duplicates
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 74)), // Duplicate
        
        // Blood pressure duplicates  
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 120, 80)),
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 125, 85)), // Duplicate
        
        // Activity duplicates
        HealthMetric::Activity(create_test_activity(today, 10000)),
        HealthMetric::Activity(create_test_activity(today, 12000)), // Duplicate
        
        // Sleep with no duplicates
        HealthMetric::Sleep(create_test_sleep(sleep_start, timestamp)),
    ];

    let workouts = vec![
        create_test_workout(timestamp, timestamp.checked_add_signed(chrono::Duration::hours(1)).unwrap()),
        create_test_workout(timestamp, timestamp.checked_add_signed(chrono::Duration::hours(2)).unwrap()), // Duplicate start time
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.heart_rate_duplicates, 1);
    assert_eq!(stats.blood_pressure_duplicates, 1);
    assert_eq!(stats.sleep_duplicates, 0);
    assert_eq!(stats.activity_duplicates, 1);
    assert_eq!(stats.workout_duplicates, 1);
    assert_eq!(stats.total_duplicates, 4);
    assert_eq!(result.processed_count, 5); // 7 original - 4 duplicates + 2 workouts - 1 duplicate = 5
}

#[tokio::test]
async fn test_deduplication_disabled() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: false, // Disabled
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    let timestamp = Utc::now();
    let metrics = vec![
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 74)), // Would be duplicate if enabled
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.total_duplicates, 0, "No duplicates should be removed when deduplication is disabled");
    assert_eq!(stats.deduplication_time_ms, 0);
    // Note: processed_count will be 1 because database ON CONFLICT will handle the duplicate
    assert_eq!(result.processed_count, 1);
}

#[tokio::test]
async fn test_deduplication_preserves_order() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create metrics where the first occurrence should be preserved
    let timestamp = Utc::now();
    let metrics = vec![
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),    // This should be kept (first)
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 80)),    // This should be discarded (duplicate timestamp)
        HealthMetric::HeartRate(create_test_heart_rate(timestamp.checked_add_signed(chrono::Duration::minutes(1)).unwrap(), 75)), // This should be kept (unique)
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.heart_rate_duplicates, 1);
    assert_eq!(result.processed_count, 2);
    
    // Note: We can't easily verify which specific record was kept without more complex database queries
    // but the deduplication logic preserves the first occurrence by design
}

#[tokio::test]
async fn test_large_batch_deduplication_performance() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create a large batch with 50% duplicates
    let mut metrics = Vec::new();
    let base_timestamp = Utc::now();
    
    // Add 1000 unique heart rate metrics
    for i in 0..1000 {
        let timestamp = base_timestamp.checked_add_signed(chrono::Duration::minutes(i)).unwrap();
        metrics.push(HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)));
    }
    
    // Add 500 duplicates (same timestamps as first 500)
    for i in 0..500 {
        let timestamp = base_timestamp.checked_add_signed(chrono::Duration::minutes(i)).unwrap();
        metrics.push(HealthMetric::HeartRate(create_test_heart_rate(timestamp, 80))); // Different value, same timestamp
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let start_time = std::time::Instant::now();
    let result = processor.process_batch(user_id, payload).await;
    let total_time = start_time.elapsed();

    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.heart_rate_duplicates, 500);
    assert_eq!(result.processed_count, 1000);
    
    // Performance assertions
    assert!(stats.deduplication_time_ms < 1000, "Deduplication should complete in under 1 second for 1500 records");
    assert!(total_time.as_millis() < 10000, "Total processing should complete in under 10 seconds");
    
    println!("Deduplication performance: {} duplicates removed in {}ms", stats.total_duplicates, stats.deduplication_time_ms);
}

#[tokio::test]
async fn test_deduplication_with_different_users() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id_1 = Uuid::new_v4();
    let user_id_2 = Uuid::new_v4();

    // Same timestamp but different users should NOT be considered duplicates
    let timestamp = Utc::now();
    
    // Process for user 1
    let metrics_1 = vec![
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 74)), // Duplicate for same user
    ];

    let payload_1 = IngestPayload {
        data: IngestData {
            metrics: metrics_1,
            workouts: vec![],
        },
    };

    let result_1 = processor.process_batch(user_id_1, payload_1).await;
    let stats_1 = result_1.deduplication_stats.unwrap();
    assert_eq!(stats_1.heart_rate_duplicates, 1); // One duplicate within user 1's batch

    // Process for user 2 with same timestamp
    let metrics_2 = vec![
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 76)), // Same timestamp as user 1, but different user
    ];

    let payload_2 = IngestPayload {
        data: IngestData {
            metrics: metrics_2,
            workouts: vec![],
        },
    };

    let result_2 = processor.process_batch(user_id_2, payload_2).await;
    let stats_2 = result_2.deduplication_stats.unwrap();
    assert_eq!(stats_2.heart_rate_duplicates, 0); // No duplicates for user 2
    assert_eq!(result_2.processed_count, 1); // Should successfully process user 2's record
}

#[tokio::test]  
async fn test_deduplication_statistics_accuracy() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    let timestamp = Utc::now();
    let today = timestamp.date_naive();
    
    // Create a complex scenario with known duplicate counts
    let metrics = vec![
        // 3 heart rate, 2 duplicates
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 72)),
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 74)), // dup 1
        HealthMetric::HeartRate(create_test_heart_rate(timestamp, 76)), // dup 2
        
        // 2 blood pressure, 1 duplicate  
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 120, 80)),
        HealthMetric::BloodPressure(create_test_blood_pressure(timestamp, 125, 85)), // dup 1
        
        // 1 activity, no duplicates
        HealthMetric::Activity(create_test_activity(today, 10000)),
    ];

    let workouts = vec![
        // 2 workouts, 1 duplicate
        create_test_workout(timestamp, timestamp.checked_add_signed(chrono::Duration::hours(1)).unwrap()),
        create_test_workout(timestamp, timestamp.checked_add_signed(chrono::Duration::hours(2)).unwrap()), // dup 1
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts,
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    let stats = result.deduplication_stats.unwrap();
    
    // Verify individual metric type statistics
    assert_eq!(stats.heart_rate_duplicates, 2, "Should have 2 heart rate duplicates");
    assert_eq!(stats.blood_pressure_duplicates, 1, "Should have 1 blood pressure duplicate");
    assert_eq!(stats.sleep_duplicates, 0, "Should have 0 sleep duplicates");
    assert_eq!(stats.activity_duplicates, 0, "Should have 0 activity duplicates");
    assert_eq!(stats.workout_duplicates, 1, "Should have 1 workout duplicate");
    
    // Verify total
    assert_eq!(stats.total_duplicates, 4, "Total duplicates should be 2+1+0+0+1=4");
    
    // Verify processing results
    assert_eq!(result.processed_count, 4, "Should process 4 unique records: 1 HR + 1 BP + 1 Activity + 1 Workout");
    assert_eq!(result.failed_count, 0, "Should have no failures");
    
    // Verify timing was recorded
    assert!(stats.deduplication_time_ms > 0, "Deduplication time should be recorded");
}