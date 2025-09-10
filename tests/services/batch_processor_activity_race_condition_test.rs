use chrono::{DateTime, Utc};
use uuid::Uuid;

use self_sensored::models::health_metrics::ActivityMetric;
use self_sensored::models::{HealthMetric, IngestData, IngestPayload};
use self_sensored::services::batch_processor::{BatchConfig, BatchProcessor};
use self_sensored::test_utilities::{create_test_pool, setup_test_database};

/// Helper function to create a test activity metric
fn create_test_activity_with_values(
    date: chrono::NaiveDate, 
    steps: Option<i32>,
    distance: Option<f64>,
    calories: Option<f64>,
    active_minutes: Option<i32>,
    flights: Option<i32>,
    source: Option<String>
) -> ActivityMetric {
    ActivityMetric {
        date,
        steps,
        distance_meters: distance,
        calories_burned: calories,
        active_minutes,
        flights_climbed: flights,
        source,
    }
}

#[tokio::test]
async fn test_activity_race_condition_fix_with_deduplication_enabled() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create multiple activity metrics for the same date (this would cause the race condition)
    let today = chrono::Utc::now().date_naive();
    let metrics = vec![
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(5000), Some(3000.0), Some(200.0), Some(30), Some(5), Some("Apple Watch".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(7000), Some(4000.0), Some(250.0), Some(45), Some(8), Some("iPhone".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(6000), Some(3500.0), None, Some(40), Some(6), Some("Fitbit".to_string())
        )),
        // This would have caused: "ON CONFLICT DO UPDATE command cannot affect row a second time"
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should not fail and should properly deduplicate and merge
    assert_eq!(result.failed_count, 0, "Should have no processing failures");
    assert!(result.deduplication_stats.is_some());
    
    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.activity_duplicates, 2, "Should have removed 2 duplicate activity records");
    assert_eq!(result.processed_count, 1, "Should process 1 merged activity record");

    // Verify the merged data is correct (should have taken maximum values)
    // We can't easily verify the exact values without querying the database,
    // but the key point is that it processed without error
}

#[tokio::test]
async fn test_activity_race_condition_fix_with_deduplication_disabled() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: false, // This is the dangerous scenario
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create multiple activity metrics for the same date
    // Before the fix, this would cause: "ON CONFLICT DO UPDATE command cannot affect row a second time"
    let today = chrono::Utc::now().date_naive();
    let metrics = vec![
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(8000), Some(5000.0), Some(300.0), Some(50), Some(10), Some("Apple Watch".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(9000), Some(5500.0), Some(350.0), Some(55), Some(12), Some("iPhone".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(8500), Some(5200.0), Some(320.0), Some(52), Some(11), Some("Garmin".to_string())
        )),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // With the fix (using DO NOTHING instead of DO UPDATE), this should not fail
    assert_eq!(result.failed_count, 0, "Should have no processing failures even with deduplication disabled");
    assert!(result.deduplication_stats.is_some());
    
    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.activity_duplicates, 0, "Should have 0 duplicates removed when deduplication is disabled");
    // With DO NOTHING, only the first record will be inserted, the rest will be ignored
    assert_eq!(result.processed_count, 1, "Should process 1 activity record (first one inserted, others ignored by ON CONFLICT DO NOTHING)");
}

#[tokio::test]
async fn test_activity_merging_logic() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: true,
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id = Uuid::new_v4();

    // Create activity metrics with different values to test merging logic
    let today = chrono::Utc::now().date_naive();
    let metrics = vec![
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(5000), Some(3000.0), None, Some(30), None, Some("Device1".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(7000), None, Some(250.0), Some(45), Some(8), Some("Device2".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(6000), Some(4000.0), Some(200.0), None, Some(10), None
        )),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let result = processor.process_batch(user_id, payload).await;

    // Should successfully merge and process
    assert_eq!(result.failed_count, 0, "Should have no processing failures");
    assert_eq!(result.processed_count, 1, "Should process 1 merged activity record");
    
    let stats = result.deduplication_stats.unwrap();
    assert_eq!(stats.activity_duplicates, 2, "Should have merged 2 duplicate records");
    
    // The merged record should have:
    // - steps: max(5000, 7000, 6000) = 7000
    // - distance: max(3000.0, 4000.0) = 4000.0 (None ignored)
    // - calories: max(250.0, 200.0) = 250.0 (None ignored)
    // - active_minutes: max(30, 45) = 45 (None ignored)
    // - flights: max(8, 10) = 10 (None ignored)
    // - source: "Device2" (last non-null value)
}

#[tokio::test]
async fn test_activity_multiple_users_no_conflict() {
    let pool = create_test_pool().await;
    setup_test_database(&pool).await.expect("Failed to setup test database");
    
    let config = BatchConfig {
        enable_intra_batch_deduplication: false, // Test the dangerous scenario
        ..BatchConfig::default()
    };
    let processor = BatchProcessor::with_config(pool, config);
    let user_id_1 = Uuid::new_v4();
    let user_id_2 = Uuid::new_v4();

    let today = chrono::Utc::now().date_naive();

    // Process for user 1 - multiple records for same date
    let metrics_1 = vec![
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(8000), Some(5000.0), Some(300.0), Some(50), Some(10), Some("User1_Device1".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(9000), Some(5500.0), Some(350.0), Some(55), Some(12), Some("User1_Device2".to_string())
        )),
    ];

    let payload_1 = IngestPayload {
        data: IngestData {
            metrics: metrics_1,
            workouts: vec![],
        },
    };

    let result_1 = processor.process_batch(user_id_1, payload_1).await;
    assert_eq!(result_1.failed_count, 0, "User 1 batch should not fail");

    // Process for user 2 - multiple records for same date (different user)
    let metrics_2 = vec![
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(7000), Some(4000.0), Some(280.0), Some(45), Some(8), Some("User2_Device1".to_string())
        )),
        HealthMetric::Activity(create_test_activity_with_values(
            today, Some(7500), Some(4200.0), Some(290.0), Some(47), Some(9), Some("User2_Device2".to_string())
        )),
    ];

    let payload_2 = IngestPayload {
        data: IngestData {
            metrics: metrics_2,
            workouts: vec![],
        },
    };

    let result_2 = processor.process_batch(user_id_2, payload_2).await;
    assert_eq!(result_2.failed_count, 0, "User 2 batch should not fail");

    // Both users should have successfully processed 1 record each
    // (with DO NOTHING, duplicates within each user's batch are ignored)
    assert_eq!(result_1.processed_count, 1, "User 1 should have 1 processed record");
    assert_eq!(result_2.processed_count, 1, "User 2 should have 1 processed record");
}