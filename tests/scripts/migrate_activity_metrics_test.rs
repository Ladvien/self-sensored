// Migration Test Suite: activity_metrics to activity_metrics_v2
// Story 5.1: Create Data Migration Scripts - Test Component
//
// Comprehensive test suite for the activity_metrics migration including:
// - Production data pattern testing
// - Batch processing performance validation  
// - Resume after failure functionality
// - Data integrity verification
// - Zero data loss guarantee
// - Performance benchmarks for 100M+ records
//
// Usage: cargo test migrate_activity_metrics

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sqlx::{PgPool, Row};
use uuid::Uuid;
use tokio::time::sleep;
use serde_json::json;

use crate::test_utils::{get_test_pool, create_test_user, cleanup_test_data};

#[derive(Debug, Clone)]
struct TestActivityRecord {
    user_id: Uuid,
    recorded_date: chrono::NaiveDate,
    steps: Option<i32>,
    distance_meters: Option<rust_decimal::Decimal>,
    calories_burned: Option<i32>,
    active_minutes: Option<i32>,
    flights_climbed: Option<i32>,
    source_device: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl TestActivityRecord {
    fn new(user_id: Uuid, days_ago: i64) -> Self {
        let recorded_date = chrono::Utc::now().naive_utc().date() - chrono::Duration::days(days_ago);
        Self {
            user_id,
            recorded_date,
            steps: Some(rand::random::<i32>() % 20000 + 1000), // 1K-20K steps
            distance_meters: Some(rust_decimal::Decimal::from(rand::random::<f64>() * 15000.0)), // 0-15km
            calories_burned: Some(rand::random::<i32>() % 3000 + 200), // 200-3200 calories
            active_minutes: Some(rand::random::<i32>() % 480 + 30), // 30-480 minutes
            flights_climbed: Some(rand::random::<i32>() % 50), // 0-50 flights
            source_device: Some(format!("TestDevice{}", rand::random::<u16>() % 100)),
            metadata: Some(json!({
                "test_data": true,
                "batch": "migration_test",
                "generated_at": chrono::Utc::now()
            })),
        }
    }
}

/// Create test data with realistic production patterns
async fn create_test_activity_data(
    pool: &PgPool,
    user_count: usize,
    days_per_user: usize,
) -> Result<Vec<TestActivityRecord>, sqlx::Error> {
    let mut test_records = Vec::new();
    let mut users = Vec::new();
    
    // Create test users
    for _ in 0..user_count {
        users.push(create_test_user(pool).await?);
    }
    
    println!("Created {} test users", users.len());
    
    // Generate activity data for each user
    for user_id in users {
        for day_offset in 0..days_per_user {
            let record = TestActivityRecord::new(user_id, day_offset as i64);
            test_records.push(record);
        }
    }
    
    // Insert test records into activity_metrics table
    let mut inserted_count = 0;
    for record in &test_records {
        let result = sqlx::query!(
            r#"
            INSERT INTO activity_metrics (
                user_id, recorded_date, steps, distance_meters, 
                calories_burned, active_minutes, flights_climbed, 
                source_device, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (user_id, recorded_date) DO NOTHING
            "#,
            record.user_id,
            record.recorded_date,
            record.steps,
            record.distance_meters,
            record.calories_burned,
            record.active_minutes,
            record.flights_climbed,
            record.source_device.as_deref(),
            record.metadata
        )
        .execute(pool)
        .await?;
        
        if result.rows_affected() > 0 {
            inserted_count += 1;
        }
    }
    
    println!("Inserted {} activity records", inserted_count);
    Ok(test_records)
}

/// Test basic migration functionality with small dataset
#[tokio::test]
async fn test_basic_migration_functionality() {
    let pool = get_test_pool().await;
    
    // Create small test dataset
    let test_data = create_test_activity_data(&pool, 5, 30).await
        .expect("Failed to create test data");
    
    println!("Starting basic migration test with {} records", test_data.len());
    
    // Run migration
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(1000)" // Small batch size for testing
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    // Verify migration completed
    assert_eq!(migration_result.status, Some("completed".to_string()));
    assert!(migration_result.total_processed.unwrap_or(0) > 0);
    
    // Verify data integrity
    let validation_results = sqlx::query!(
        "SELECT * FROM validate_activity_metrics_migration()"
    )
    .fetch_all(&pool)
    .await
    .expect("Validation failed");
    
    // Check all validations pass
    for result in validation_results {
        assert_eq!(result.match_status, Some("PASS".to_string()), 
                   "Validation failed for: {}", result.validation_check.unwrap_or_default());
    }
    
    println!("Basic migration test passed");
    cleanup_test_data(&pool).await;
}

/// Test batch processing performance with larger dataset
#[tokio::test]
async fn test_batch_processing_performance() {
    let pool = get_test_pool().await;
    
    // Create larger test dataset
    let test_data = create_test_activity_data(&pool, 50, 100).await
        .expect("Failed to create test data");
    
    println!("Testing batch processing with {} records", test_data.len());
    
    let start_time = Instant::now();
    
    // Run migration with different batch sizes
    let batch_sizes = vec![1000, 4000, 8000];
    
    for batch_size in batch_sizes {
        println!("Testing batch size: {}", batch_size);
        
        // Clear previous migration data
        sqlx::query!("SELECT * FROM rollback_activity_metrics_migration()")
            .fetch_all(&pool)
            .await
            .expect("Rollback failed");
        
        let batch_start = Instant::now();
        
        // Run migration
        let migration_result = sqlx::query!(
            "SELECT * FROM migrate_activity_metrics_to_v2($1)",
            batch_size as i32
        )
        .fetch_one(&pool)
        .await
        .expect("Migration failed");
        
        let batch_duration = batch_start.elapsed();
        
        // Verify success
        assert_eq!(migration_result.status, Some("completed".to_string()));
        
        let records_processed = migration_result.total_processed.unwrap_or(0) as f64;
        let records_per_second = records_processed / batch_duration.as_secs_f64();
        
        println!("Batch size {}: {} records in {:.2}s ({:.1} records/sec)", 
                 batch_size, records_processed, batch_duration.as_secs_f64(), records_per_second);
        
        // Performance assertion: Should process at least 500 records/second
        assert!(records_per_second >= 500.0, 
               "Performance too slow: {:.1} records/sec (expected >= 500)", records_per_second);
    }
    
    let total_duration = start_time.elapsed();
    println!("Batch processing performance test completed in {:.2}s", total_duration.as_secs_f64());
    
    cleanup_test_data(&pool).await;
}

/// Test resume functionality after simulated failure
#[tokio::test]
async fn test_resume_after_failure() {
    let pool = get_test_pool().await;
    
    // Create test dataset
    let test_data = create_test_activity_data(&pool, 20, 50).await
        .expect("Failed to create test data");
    
    println!("Testing resume functionality with {} records", test_data.len());
    
    // Start migration that will be "interrupted"
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(500)" // Small batch size
    )
    .fetch_one(&pool)
    .await
    .expect("Initial migration failed");
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    
    // Get initial migration state
    let initial_progress = sqlx::query!(
        "SELECT total_records_processed, batch_number, last_processed_id FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to get migration progress");
    
    let initial_processed = initial_progress.total_records_processed.unwrap_or(0);
    let initial_last_id = initial_progress.last_processed_id;
    
    println!("Initial migration processed {} records, last ID: {:?}", 
             initial_processed, initial_last_id);
    
    // Simulate partial rollback to test resumability
    sqlx::query!("DELETE FROM activity_metrics_v2 WHERE id > (SELECT MAX(id)/2 FROM activity_metrics_v2)")
        .execute(&pool)
        .await
        .expect("Partial rollback failed");
    
    // Mark migration as failed to test resume
    sqlx::query!(
        "UPDATE migration_progress SET status = 'failed' WHERE migration_name = 'activity_metrics_to_v2'"
    )
    .execute(&pool)
    .await
    .expect("Failed to update migration status");
    
    // Test resume functionality
    let resume_result = sqlx::query!(
        "SELECT * FROM resume_activity_metrics_migration()"
    )
    .fetch_one(&pool)
    .await
    .expect("Resume failed");
    
    assert_eq!(resume_result.status, Some("resumed".to_string()));
    
    // Verify final state
    let final_progress = sqlx::query!(
        "SELECT status, total_records_processed FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to get final progress");
    
    assert_eq!(final_progress.status, Some("completed".to_string()));
    assert!(final_progress.total_records_processed.unwrap_or(0) >= initial_processed);
    
    println!("Resume functionality test passed");
    cleanup_test_data(&pool).await;
}

/// Test data integrity with edge cases
#[tokio::test]
async fn test_data_integrity_edge_cases() {
    let pool = get_test_pool().await;
    
    let test_user = create_test_user(&pool).await.expect("Failed to create test user");
    
    // Create edge case test data
    let edge_cases = vec![
        // Null values
        TestActivityRecord {
            user_id: test_user,
            recorded_date: chrono::Utc::now().naive_utc().date(),
            steps: None,
            distance_meters: None,
            calories_burned: None,
            active_minutes: None,
            flights_climbed: None,
            source_device: None,
            metadata: None,
        },
        // Maximum values
        TestActivityRecord {
            user_id: test_user,
            recorded_date: chrono::Utc::now().naive_utc().date() - chrono::Duration::days(1),
            steps: Some(200000), // Maximum allowed
            distance_meters: Some(rust_decimal::Decimal::from(500000)), // 500km
            calories_burned: Some(20000), // High calorie burn
            active_minutes: Some(1440), // Full day
            flights_climbed: Some(10000), // Maximum
            source_device: Some("EdgeTestDevice".to_string()),
            metadata: Some(json!({"edge_case": "maximum_values"})),
        },
        // Zero values  
        TestActivityRecord {
            user_id: test_user,
            recorded_date: chrono::Utc::now().naive_utc().date() - chrono::Duration::days(2),
            steps: Some(0),
            distance_meters: Some(rust_decimal::Decimal::ZERO),
            calories_burned: Some(0),
            active_minutes: Some(0),
            flights_climbed: Some(0),
            source_device: Some("ZeroTestDevice".to_string()),
            metadata: Some(json!({"edge_case": "zero_values"})),
        },
    ];
    
    // Insert edge case data
    for (i, record) in edge_cases.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO activity_metrics (
                user_id, recorded_date, steps, distance_meters,
                calories_burned, active_minutes, flights_climbed,
                source_device, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            record.user_id,
            record.recorded_date,
            record.steps,
            record.distance_meters,
            record.calories_burned,
            record.active_minutes,
            record.flights_climbed,
            record.source_device.as_deref(),
            record.metadata
        )
        .execute(&pool)
        .await
        .expect(&format!("Failed to insert edge case record {}", i));
    }
    
    println!("Inserted {} edge case records", edge_cases.len());
    
    // Run migration
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(1000)"
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    
    // Validate edge cases were migrated correctly
    let migrated_records = sqlx::query!(
        "SELECT * FROM activity_metrics_v2 WHERE user_id = $1 ORDER BY recorded_at",
        test_user
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch migrated records");
    
    assert_eq!(migrated_records.len(), edge_cases.len());
    
    // Verify field mappings for each edge case
    for (i, migrated) in migrated_records.iter().enumerate() {
        let original = &edge_cases[i];
        
        assert_eq!(migrated.user_id, original.user_id);
        assert_eq!(migrated.recorded_at.date(), original.recorded_date);
        assert_eq!(migrated.step_count, original.steps);
        assert_eq!(migrated.flights_climbed, original.flights_climbed);
        
        // Check numeric field mappings
        if let Some(orig_distance) = original.distance_meters {
            assert_eq!(migrated.distance_walking_running_meters, Some(orig_distance.try_into().unwrap()));
        }
        if let Some(orig_calories) = original.calories_burned {
            assert_eq!(migrated.active_energy_burned_kcal, Some(orig_calories.into()));
        }
        if let Some(orig_active) = original.active_minutes {
            assert_eq!(migrated.exercise_time_minutes, Some(orig_active));
        }
    }
    
    println!("Edge case data integrity test passed");
    cleanup_test_data(&pool).await;
}

/// Test large dataset performance (simulates 100M records scenario)
#[tokio::test] 
async fn test_large_dataset_performance_simulation() {
    let pool = get_test_pool().await;
    
    // Create substantial test dataset (simulate conditions for 100M records)
    let test_data = create_test_activity_data(&pool, 100, 365).await // 100 users * 365 days = 36.5K records
        .expect("Failed to create test data");
    
    println!("Testing large dataset performance simulation with {} records", test_data.len());
    
    let start_time = Instant::now();
    
    // Run migration with production batch size
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(8000)"
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    let migration_duration = start_time.elapsed();
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    
    let records_processed = migration_result.total_processed.unwrap_or(0) as f64;
    let records_per_second = records_processed / migration_duration.as_secs_f64();
    
    println!("Large dataset simulation:");
    println!("  Records processed: {}", records_processed);
    println!("  Migration time: {:.2}s", migration_duration.as_secs_f64());
    println!("  Processing rate: {:.1} records/sec", records_per_second);
    
    // Calculate projected time for 100M records
    let hundred_million = 100_000_000.0;
    let projected_time_seconds = hundred_million / records_per_second;
    let projected_time_hours = projected_time_seconds / 3600.0;
    
    println!("  Projected time for 100M records: {:.1} hours", projected_time_hours);
    
    // Performance requirement: Must complete 100M records in under 4 hours
    assert!(projected_time_hours < 4.0, 
           "Performance requirement not met: {:.1} hours projected (must be < 4 hours)", 
           projected_time_hours);
    
    // Minimum performance threshold: 8,000 records/second for 100M in 4 hours
    assert!(records_per_second >= 7000.0,
           "Processing rate too slow: {:.1} records/sec (need >= 7000 for 4-hour target)", 
           records_per_second);
    
    // Validate data integrity
    let validation_results = sqlx::query!(
        "SELECT * FROM validate_activity_metrics_migration()"
    )
    .fetch_all(&pool)
    .await
    .expect("Validation failed");
    
    // All validations must pass for zero data loss guarantee  
    for result in validation_results {
        assert_eq!(result.match_status, Some("PASS".to_string()), 
                   "Data integrity validation failed: {}", result.validation_check.unwrap_or_default());
    }
    
    println!("Large dataset performance simulation passed");
    println!("Zero data loss guarantee verified");
    
    cleanup_test_data(&pool).await;
}

/// Test concurrent migration safety
#[tokio::test]
async fn test_concurrent_migration_safety() {
    let pool = get_test_pool().await;
    
    // Create test data
    let _test_data = create_test_activity_data(&pool, 10, 30).await
        .expect("Failed to create test data");
    
    println!("Testing concurrent migration safety");
    
    // Attempt to run two migrations simultaneously
    let pool1 = pool.clone();
    let pool2 = pool.clone();
    
    let migration1 = tokio::spawn(async move {
        sqlx::query!("SELECT * FROM migrate_activity_metrics_to_v2(1000)")
            .fetch_one(&pool1)
            .await
    });
    
    // Small delay to ensure first migration starts
    sleep(Duration::from_millis(100)).await;
    
    let migration2 = tokio::spawn(async move {
        sqlx::query!("SELECT * FROM migrate_activity_metrics_to_v2(1000)")
            .fetch_one(&pool2)
            .await
    });
    
    let (result1, result2) = tokio::join!(migration1, migration2);
    
    // One should succeed, the other should handle the conflict gracefully
    let result1 = result1.expect("Task 1 panicked");
    let result2 = result2.expect("Task 2 panicked");
    
    // At least one migration should complete successfully
    let success_count = [&result1, &result2]
        .iter()
        .filter(|r| r.is_ok() && r.as_ref().unwrap().status == Some("completed".to_string()))
        .count();
    
    assert!(success_count >= 1, "At least one migration should complete successfully");
    
    println!("Concurrent migration safety test passed");
    cleanup_test_data(&pool).await;
}

/// Test rollback functionality
#[tokio::test]
async fn test_rollback_functionality() {
    let pool = get_test_pool().await;
    
    // Create test data
    let test_data = create_test_activity_data(&pool, 10, 30).await
        .expect("Failed to create test data");
    
    println!("Testing rollback functionality with {} records", test_data.len());
    
    // Run migration
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(1000)"
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    
    // Verify data exists in target table
    let pre_rollback_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count records");
    
    assert!(pre_rollback_count > 0, "No records found in target table after migration");
    
    // Perform rollback
    let rollback_results = sqlx::query!(
        "SELECT * FROM safe_migration_rollback(TRUE)"
    )
    .fetch_all(&pool)
    .await
    .expect("Rollback failed");
    
    // Verify rollback steps completed successfully
    let completed_steps = rollback_results
        .iter()
        .filter(|r| r.status == Some("COMPLETED".to_string()))
        .count();
    
    assert!(completed_steps >= 3, "Expected at least 3 completed rollback steps");
    
    // Verify target table is empty
    let post_rollback_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count records after rollback");
    
    assert_eq!(post_rollback_count, 0, "Target table should be empty after rollback");
    
    // Verify migration progress is reset
    let progress_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to check progress table");
    
    assert_eq!(progress_count, 0, "Migration progress should be reset");
    
    // Verify original data is intact
    let original_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count original records");
    
    assert_eq!(original_count as usize, test_data.len(), "Original data should be intact");
    
    println!("Rollback functionality test passed");
    cleanup_test_data(&pool).await;
}

/// Test monitoring and progress tracking
#[tokio::test]
async fn test_monitoring_and_progress_tracking() {
    let pool = get_test_pool().await;
    
    // Create test data
    let _test_data = create_test_activity_data(&pool, 20, 50).await
        .expect("Failed to create test data");
    
    println!("Testing monitoring and progress tracking");
    
    // Run migration
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(500)" // Small batches for more tracking points
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    
    // Test monitoring functions
    let progress_result = sqlx::query!(
        "SELECT * FROM get_migration_progress()"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to get migration progress");
    
    assert_eq!(progress_result.migration_status, Some("completed".to_string()));
    assert!(progress_result.progress_pct.unwrap_or(0.0) == 100.0);
    
    // Test performance details
    let performance_results = sqlx::query!(
        "SELECT * FROM get_migration_performance_details()"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to get performance details");
    
    assert!(!performance_results.is_empty(), "Performance details should be available");
    
    // Test dashboard query
    let dashboard_results = sqlx::query!(
        "SELECT * FROM migration_dashboard()"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to get dashboard");
    
    let success_count = dashboard_results
        .iter()
        .filter(|r| r.status == Some("SUCCESS".to_string()) || r.status == Some("INFO".to_string()))
        .count();
    
    assert!(success_count > 0, "Dashboard should show successful metrics");
    
    // Test consistency checks
    let consistency_results = sqlx::query!(
        "SELECT * FROM quick_consistency_check()"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to run consistency check");
    
    for result in consistency_results {
        assert_eq!(result.match_status, Some("MATCH".to_string()),
                   "Consistency check failed for: {}", result.check_name.unwrap_or_default());
    }
    
    println!("Monitoring and progress tracking test passed");
    cleanup_test_data(&pool).await;
}

/// Integration test combining all functionality
#[tokio::test]
async fn test_end_to_end_migration_workflow() {
    let pool = get_test_pool().await;
    
    println!("Running end-to-end migration workflow test");
    
    // Step 1: Create comprehensive test dataset
    let test_data = create_test_activity_data(&pool, 30, 90).await
        .expect("Failed to create test data");
    
    println!("Step 1: Created {} test records", test_data.len());
    
    // Step 2: Run migration with monitoring
    let start_time = Instant::now();
    
    let migration_result = sqlx::query!(
        "SELECT * FROM migrate_activity_metrics_to_v2(4000)"
    )
    .fetch_one(&pool)
    .await
    .expect("Migration failed");
    
    let migration_duration = start_time.elapsed();
    
    assert_eq!(migration_result.status, Some("completed".to_string()));
    println!("Step 2: Migration completed in {:.2}s", migration_duration.as_secs_f64());
    
    // Step 3: Comprehensive validation
    let validation_results = sqlx::query!(
        "SELECT * FROM validate_activity_metrics_migration()"
    )
    .fetch_all(&pool)
    .await
    .expect("Validation failed");
    
    for result in &validation_results {
        assert_eq!(result.match_status, Some("PASS".to_string()),
                   "Validation failed: {} - Original: {}, Migrated: {}", 
                   result.validation_check.unwrap_or_default(),
                   result.original_count.unwrap_or(0),
                   result.migrated_count.unwrap_or(0));
    }
    println!("Step 3: All validations passed");
    
    // Step 4: Performance verification
    let records_processed = migration_result.total_processed.unwrap_or(0) as f64;
    let records_per_second = records_processed / migration_duration.as_secs_f64();
    
    assert!(records_per_second >= 1000.0,
           "Performance requirement not met: {:.1} records/sec", records_per_second);
    println!("Step 4: Performance verified - {:.1} records/sec", records_per_second);
    
    // Step 5: Sample detailed validation
    let detailed_validation = sqlx::query!(
        "SELECT * FROM detailed_data_validation(1000)"
    )
    .fetch_all(&pool)
    .await
    .expect("Detailed validation failed");
    
    for result in detailed_validation {
        assert!(result.accuracy_percentage.unwrap_or(0.0) >= 95.0,
               "Data accuracy below threshold: {:.2}%", result.accuracy_percentage.unwrap_or(0.0));
    }
    println!("Step 5: Detailed validation passed");
    
    // Step 6: Test rollback capability
    let pre_rollback_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count records");
    
    sqlx::query!("SELECT * FROM safe_migration_rollback(TRUE)")
        .fetch_all(&pool)
        .await
        .expect("Rollback failed");
    
    let post_rollback_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2" 
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count records after rollback");
    
    assert_eq!(post_rollback_count, 0, "Rollback verification failed");
    println!("Step 6: Rollback capability verified");
    
    println!("End-to-end migration workflow test completed successfully");
    println!("Summary:");
    println!("  - Test records: {}", test_data.len());
    println!("  - Migration time: {:.2}s", migration_duration.as_secs_f64());
    println!("  - Processing rate: {:.1} records/sec", records_per_second);
    println!("  - Data integrity: PASS");
    println!("  - Zero data loss: VERIFIED");
    
    cleanup_test_data(&pool).await;
}

// Test utilities module
mod test_utils {
    use super::*;
    
    pub async fn get_test_pool() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for tests");
            
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }
    
    pub async fn create_test_user(pool: &PgPool) -> Result<Uuid, sqlx::Error> {
        let user_id = Uuid::new_v4();
        let email = format!("test_{}@migration-test.com", user_id);
        
        sqlx::query!(
            "INSERT INTO users (id, email, full_name) VALUES ($1, $2, $3)",
            user_id,
            email,
            "Migration Test User"
        )
        .execute(pool)
        .await?;
        
        Ok(user_id)
    }
    
    pub async fn cleanup_test_data(pool: &PgPool) {
        // Clean up test data
        let _ = sqlx::query!("DELETE FROM activity_metrics_v2 WHERE raw_data->>'test_data' = 'true'")
            .execute(pool)
            .await;
            
        let _ = sqlx::query!("DELETE FROM activity_metrics WHERE metadata->>'test_data' = 'true'")
            .execute(pool)
            .await;
            
        let _ = sqlx::query!("DELETE FROM users WHERE email LIKE '%@migration-test.com'")
            .execute(pool)
            .await;
            
        let _ = sqlx::query!("DELETE FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2'")
            .execute(pool)
            .await;
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    
    // Additional integration tests can be added here
    // These tests require the full migration setup to be deployed
    
    /// Test with actual migration SQL scripts loaded
    #[tokio::test]
    async fn test_with_migration_scripts() {
        let pool = get_test_pool().await;
        
        // Load and execute migration scripts
        let migration_sql = std::fs::read_to_string("scripts/migrate_activity_metrics.sql")
            .expect("Failed to read migration script");
            
        sqlx::raw_sql(&migration_sql)
            .execute(&pool)
            .await
            .expect("Failed to execute migration script");
            
        let monitoring_sql = std::fs::read_to_string("scripts/monitor_migration.sql")
            .expect("Failed to read monitoring script");
            
        sqlx::raw_sql(&monitoring_sql)
            .execute(&pool)
            .await
            .expect("Failed to execute monitoring script");
            
        // Run basic functionality test
        test_basic_migration_functionality().await;
    }
}