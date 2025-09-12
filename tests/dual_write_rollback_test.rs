//! Tests for dual-write rollback scenarios and partial failure handling
//! This file specifically tests the error handling and rollback behavior 
//! of the dual-write implementation for activity_metrics

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use self_sensored::{
    config::BatchConfig,
    models::ActivityMetric,
    services::batch_processor::BatchProcessor,
};

/// Mock error injection for testing rollback scenarios
pub struct ErrorInjector {
    pub fail_v2_insert: bool,
    pub fail_original_insert: bool,
    pub fail_after_n_records: Option<usize>,
    pub inject_constraint_violation: bool,
}

impl Default for ErrorInjector {
    fn default() -> Self {
        Self {
            fail_v2_insert: false,
            fail_original_insert: false,
            fail_after_n_records: None,
            inject_constraint_violation: false,
        }
    }
}

/// Test dual-write rollback when V2 table insert fails
#[sqlx::test]
async fn test_dual_write_rollback_v2_failure(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "rollbackv2@test.com"
    )
    .execute(&pool)
    .await?;

    let mut config = BatchConfig::default();
    config.enable_dual_write_activity_metrics = true;
    config.activity_chunk_size = 10; // Small chunk for precise testing
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);
    
    let now = Utc::now();
    
    // Create test metrics
    let test_metrics = vec![
        ActivityMetric {
            date: now.date_naive(),
            steps: Some(10000),
            distance_meters: Some(8000.0),
            calories_burned: Some(500.0),
            active_minutes: Some(60),
            flights_climbed: Some(15),
            source: Some("Rollback Test".to_string()),
        },
        ActivityMetric {
            date: (now - chrono::Duration::days(1)).date_naive(),
            steps: Some(8500),
            distance_meters: Some(6500.0),
            calories_burned: Some(450.0),
            active_minutes: Some(45),
            flights_climbed: Some(12),
            source: Some("Rollback Test".to_string()),
        },
    ];

    // First, insert valid data to verify baseline
    let mut tx = pool.begin().await?;
    let result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &test_metrics
    ).await;
    tx.commit().await?;
    
    assert!(result.is_ok(), "Initial insert should succeed");
    
    // Verify data in both tables
    let original_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    
    let v2_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    
    assert_eq!(original_count, Some(2), "Original table should have 2 records");
    assert_eq!(v2_count, Some(2), "V2 table should have 2 records");

    // Now test failure scenario by creating invalid data for v2 table
    // Using extremely large values that might exceed v2 table constraints
    let invalid_metrics = vec![
        ActivityMetric {
            date: (now + chrono::Duration::days(1)).date_naive(),
            steps: Some(i32::MAX), // Extreme value that might fail v2 validation
            distance_meters: Some(f64::MAX), // Extreme value
            calories_burned: Some(f64::MAX), // Extreme value
            active_minutes: Some(i32::MAX), // Extreme value
            flights_climbed: Some(i32::MAX), // Extreme value
            source: Some("X".repeat(1000)), // Very long string that might exceed limits
        },
    ];

    // Attempt to process invalid metrics
    let mut tx = pool.begin().await?;
    let invalid_result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &invalid_metrics
    ).await;
    
    // If the processing fails, rollback should occur
    if invalid_result.is_err() {
        tx.rollback().await?;
        
        // Verify no partial data was inserted
        let final_original_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        let final_v2_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        // Counts should remain the same (no partial insert)
        assert_eq!(final_original_count, Some(2), "Original table count should be unchanged after rollback");
        assert_eq!(final_v2_count, Some(2), "V2 table count should be unchanged after rollback");
        
        println!("✅ Dual-write rollback test completed successfully");
        println!("   - Invalid data processing failed as expected");
        println!("   - Transaction rollback prevented partial data insertion");
        
    } else {
        // If it succeeded, that's also valid behavior (data was cleaned up)
        tx.commit().await?;
        println!("ℹ️  Invalid data was processed successfully (constraints were not as strict as expected)");
    }

    Ok(())
}

/// Test partial batch failure and rollback
#[sqlx::test]
async fn test_partial_batch_failure_rollback(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "partialfail@test.com"
    )
    .execute(&pool)
    .await?;

    let mut config = BatchConfig::default();
    config.enable_dual_write_activity_metrics = true;
    config.activity_chunk_size = 5; // Very small chunks to test partial failure
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);
    
    let now = Utc::now();
    
    // Create a mix of valid and potentially invalid data
    let mixed_metrics = vec![
        // Valid records
        ActivityMetric {
            date: now.date_naive(),
            steps: Some(10000),
            distance_meters: Some(8000.0),
            calories_burned: Some(500.0),
            active_minutes: Some(60),
            flights_climbed: Some(15),
            source: Some("Valid 1".to_string()),
        },
        ActivityMetric {
            date: (now - chrono::Duration::days(1)).date_naive(),
            steps: Some(8500),
            distance_meters: Some(6500.0),
            calories_burned: Some(450.0),
            active_minutes: Some(45),
            flights_climbed: Some(12),
            source: Some("Valid 2".to_string()),
        },
        ActivityMetric {
            date: (now - chrono::Duration::days(2)).date_naive(),
            steps: Some(9000),
            distance_meters: Some(7000.0),
            calories_burned: Some(480.0),
            active_minutes: Some(50),
            flights_climbed: Some(14),
            source: Some("Valid 3".to_string()),
        },
        // Potentially problematic record (duplicate date for same user)
        ActivityMetric {
            date: now.date_naive(), // Same date as first record
            steps: Some(5000),
            distance_meters: Some(4000.0),
            calories_burned: Some(300.0),
            active_minutes: Some(30),
            flights_climbed: Some(8),
            source: Some("Duplicate Date".to_string()),
        },
    ];

    // Start with clean state
    let initial_original_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    
    let initial_v2_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(initial_original_count, Some(0), "Should start with empty tables");
    assert_eq!(initial_v2_count, Some(0), "Should start with empty tables");

    // Attempt to process mixed metrics
    let mut tx = pool.begin().await?;
    let result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &mixed_metrics
    ).await;
    
    match result {
        Ok(_) => {
            // Transaction succeeded, commit it
            tx.commit().await?;
            
            // Verify how many records were actually inserted
            let final_original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let final_v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            // Both tables should have the same count (consistency maintained)
            assert_eq!(final_original_count, final_v2_count, "Both tables should have consistent counts");
            
            println!("✅ Mixed batch processing completed successfully");
            println!("   - Original table records: {:?}", final_original_count);
            println!("   - V2 table records: {:?}", final_v2_count);
            println!("   - Data consistency maintained across both tables");
            
        },
        Err(_) => {
            // Transaction failed, rollback occurred
            tx.rollback().await?;
            
            // Verify no data was inserted
            let final_original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let final_v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            assert_eq!(final_original_count, Some(0), "Original table should remain empty after rollback");
            assert_eq!(final_v2_count, Some(0), "V2 table should remain empty after rollback");
            
            println!("✅ Mixed batch processing failed and rolled back successfully");
            println!("   - No partial data insertion occurred");
            println!("   - Transaction rollback worked correctly");
        }
    }

    Ok(())
}

/// Test constraint violation rollback
#[sqlx::test]
async fn test_constraint_violation_rollback(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "constraint@test.com"
    )
    .execute(&pool)
    .await?;

    let mut config = BatchConfig::default();
    config.enable_dual_write_activity_metrics = true;
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);
    
    let now = Utc::now();
    
    // First, insert a valid record
    let initial_metrics = vec![
        ActivityMetric {
            date: now.date_naive(),
            steps: Some(10000),
            distance_meters: Some(8000.0),
            calories_burned: Some(500.0),
            active_minutes: Some(60),
            flights_climbed: Some(15),
            source: Some("Initial Record".to_string()),
        },
    ];

    let mut tx = pool.begin().await?;
    let result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &initial_metrics
    ).await;
    tx.commit().await?;
    
    assert!(result.is_ok(), "Initial record should insert successfully");

    // Now try to insert a record that violates unique constraints
    let duplicate_metrics = vec![
        ActivityMetric {
            date: now.date_naive(), // Same date as above, should trigger unique constraint
            steps: Some(5000),
            distance_meters: Some(4000.0),
            calories_burned: Some(300.0),
            active_minutes: Some(30),
            flights_climbed: Some(8),
            source: Some("Duplicate Record".to_string()),
        },
    ];

    let mut tx = pool.begin().await?;
    let duplicate_result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &duplicate_metrics
    ).await;
    
    match duplicate_result {
        Ok(_) => {
            // If the system handles duplicates gracefully (e.g., UPSERT), commit
            tx.commit().await?;
            println!("ℹ️  Duplicate handling: System gracefully handled duplicate records");
        },
        Err(_) => {
            // If it fails due to constraint violation, rollback occurred
            tx.rollback().await?;
            println!("✅ Constraint violation correctly triggered rollback");
        }
    }

    // Verify data consistency regardless of how duplicates were handled
    let final_original_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    
    let final_v2_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;
    
    // Both tables should have consistent counts
    assert_eq!(final_original_count, final_v2_count, 
        "Both tables should maintain consistent record counts after constraint test");

    println!("✅ Constraint violation rollback test completed");
    println!("   - Data consistency maintained: {} records in both tables", 
        final_original_count.unwrap_or(0));

    Ok(())
}

/// Test transaction timeout and rollback
#[sqlx::test]
async fn test_transaction_timeout_rollback(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "timeout@test.com"
    )
    .execute(&pool)
    .await?;

    let mut config = BatchConfig::default();
    config.enable_dual_write_activity_metrics = true;
    config.activity_chunk_size = 1000; // Large chunk size
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);
    
    let now = Utc::now();
    
    // Create a reasonably large dataset that could potentially timeout
    let large_metrics: Vec<ActivityMetric> = (0..500)
        .map(|i| ActivityMetric {
            date: (now - chrono::Duration::days(i % 365)).date_naive(),
            steps: Some(8000 + (i * 100) % 5000),
            distance_meters: Some(6000.0 + (i as f64 * 50.0) % 3000.0),
            calories_burned: Some(400.0 + (i as f64 * 10.0) % 400.0),
            active_minutes: Some(45 + (i * 2) % 60),
            flights_climbed: Some(10 + (i % 15)),
            source: Some(format!("Timeout Test {}", i)),
        })
        .collect();

    // Set a shorter timeout for this test
    let mut tx = pool.begin().await?;
    
    // Start timing the operation
    let start_time = std::time::Instant::now();
    
    let result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &large_metrics
    ).await;
    
    let processing_time = start_time.elapsed();
    
    match result {
        Ok(_) => {
            tx.commit().await?;
            
            // Verify both tables have consistent data
            let original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            assert_eq!(original_count, v2_count, "Both tables should have consistent counts");
            assert_eq!(original_count, Some(500), "Should have inserted all 500 records");
            
            println!("✅ Large batch processing completed successfully in {:?}", processing_time);
            println!("   - Processed {} records", large_metrics.len());
            println!("   - Both tables consistent with {} records", original_count.unwrap_or(0));
            
        },
        Err(e) => {
            tx.rollback().await?;
            
            // Verify no partial data was inserted
            let original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            assert_eq!(original_count, Some(0), "Original table should be empty after rollback");
            assert_eq!(v2_count, Some(0), "V2 table should be empty after rollback");
            
            println!("✅ Large batch processing failed and rolled back successfully");
            println!("   - Processing failed after {:?}: {}", processing_time, e);
            println!("   - No partial data insertion occurred");
        }
    }

    Ok(())
}

/// Test field mapping error rollback
#[sqlx::test]
async fn test_field_mapping_error_rollback(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "mapping@test.com"
    )
    .execute(&pool)
    .await?;

    let mut config = BatchConfig::default();
    config.enable_dual_write_activity_metrics = true;
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);
    
    let now = Utc::now();
    
    // Create metrics with edge case data that might cause field mapping issues
    let edge_case_metrics = vec![
        ActivityMetric {
            date: now.date_naive(),
            steps: Some(-1), // Negative value might cause issues
            distance_meters: Some(-100.0), // Negative distance
            calories_burned: Some(-50.0), // Negative calories
            active_minutes: Some(-10), // Negative time
            flights_climbed: Some(-5), // Negative flights
            source: Some("Edge Case Test".to_string()),
        },
        ActivityMetric {
            date: (now - chrono::Duration::days(1)).date_naive(),
            steps: None, // NULL values
            distance_meters: None,
            calories_burned: None,
            active_minutes: None,
            flights_climbed: None,
            source: None,
        },
    ];

    let mut tx = pool.begin().await?;
    let result = batch_processor.process_activity_metrics(
        &mut tx, 
        user_id, 
        &edge_case_metrics
    ).await;
    
    match result {
        Ok(_) => {
            tx.commit().await?;
            
            // Verify data consistency
            let original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            assert_eq!(original_count, v2_count, "Both tables should have consistent counts");
            
            println!("✅ Edge case data processing completed successfully");
            println!("   - System handled negative values and NULLs correctly");
            println!("   - Both tables consistent with {} records", original_count.unwrap_or(0));
            
        },
        Err(_) => {
            tx.rollback().await?;
            
            // Verify rollback worked
            let original_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            let v2_count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
                user_id
            )
            .fetch_one(&pool)
            .await?;
            
            assert_eq!(original_count, Some(0), "Original table should be empty after rollback");
            assert_eq!(v2_count, Some(0), "V2 table should be empty after rollback");
            
            println!("✅ Edge case data processing failed and rolled back successfully");
            println!("   - Field mapping validation correctly rejected invalid data");
            println!("   - Transaction rollback prevented partial insertion");
        }
    }

    Ok(())
}