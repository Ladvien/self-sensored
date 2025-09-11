use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_activity_metrics_v2_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'activity_metrics_v2' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "activity_metrics_v2 table should exist");

    // Verify essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        "id", "user_id", "recorded_at",
        "step_count", "flights_climbed",
        "distance_walking_running_meters", "distance_cycling_meters",
        "active_energy_burned_kcal", "basal_energy_burned_kcal",
        "exercise_time_minutes", "stand_time_minutes", "move_time_minutes",
        "aggregation_period", "source", "raw_data", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in activity_metrics_v2", 
            expected_col
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'activity_metrics_v2'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "activity_metrics_v2 should be partitioned");

    // Verify partitions were created (at least 4: past month + current + 3 future)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'activity_metrics_v2_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 4, 
        "Should have at least 4 partitions (past + current + 3 future)"
    );

    Ok(())
}

#[sqlx::test]
async fn test_brin_indexes_created(pool: PgPool) -> sqlx::Result<()> {
    // Verify BRIN indexes were created
    let brin_indexes = sqlx::query!(
        "SELECT indexname FROM pg_indexes 
         WHERE tablename = 'activity_metrics_v2' 
         AND indexdef LIKE '%USING brin%'"
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        brin_indexes.len() >= 3, 
        "Should have at least 3 BRIN indexes (recorded_at, user+recorded_at, aggregation+recorded_at)"
    );

    // Verify specific BRIN indexes exist
    let index_names: Vec<&str> = brin_indexes.iter().map(|r| r.indexname.as_str()).collect();
    assert!(
        index_names.iter().any(|name| name.contains("recorded_at_brin")),
        "Should have recorded_at BRIN index"
    );
    assert!(
        index_names.iter().any(|name| name.contains("user_recorded_brin")),
        "Should have user_id+recorded_at BRIN index"
    );

    Ok(())
}

#[sqlx::test]
async fn test_validation_constraints(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user for constraint validation
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id, "test@example.com"
    )
    .execute(&pool)
    .await?;

    let now = Utc::now();

    // Test valid data insertion
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, step_count, active_energy_burned_kcal, exercise_time_minutes) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id, now, 10000, 500.0, 60
    )
    .execute(&pool)
    .await;
    assert!(result.is_ok(), "Valid data should insert successfully");

    // Test step_count constraint (max 200,000)
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, step_count) 
         VALUES ($1, $2, $3)",
        user_id, now + chrono::Duration::minutes(1), 300000
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "Step count over 200,000 should fail");

    // Test negative values
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, step_count) 
         VALUES ($1, $2, $3)",
        user_id, now + chrono::Duration::minutes(2), -100
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "Negative step count should fail");

    // Test active energy constraint (max 20,000 kcal)
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, active_energy_burned_kcal) 
         VALUES ($1, $2, $3)",
        user_id, now + chrono::Duration::minutes(3), 25000.0
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "Active energy over 20,000 should fail");

    // Test exercise time constraint (max 1440 minutes = 24 hours)
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, exercise_time_minutes) 
         VALUES ($1, $2, $3)",
        user_id, now + chrono::Duration::minutes(4), 1500
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "Exercise time over 1440 minutes should fail");

    // Test aggregation_period constraint
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, aggregation_period) 
         VALUES ($1, $2, $3)",
        user_id, now + chrono::Duration::minutes(5), "invalid"
    )
    .execute(&pool)
    .await;
    assert!(result.is_err(), "Invalid aggregation_period should fail");

    // Test valid aggregation_period values
    for period in ["minute", "hourly", "daily", "weekly"] {
        let result = sqlx::query!(
            "INSERT INTO activity_metrics_v2 
             (user_id, recorded_at, aggregation_period) 
             VALUES ($1, $2, $3)",
            user_id, now + chrono::Duration::minutes(10) * (period.len() as i64), period
        )
        .execute(&pool)
        .await;
        assert!(result.is_ok(), "Valid aggregation_period '{}' should succeed", period);
    }

    Ok(())
}

#[sqlx::test]
async fn test_apple_health_field_names(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id, "test@example.com"
    )
    .execute(&pool)
    .await?;

    let now = Utc::now();

    // Test all Apple Health specific field names
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 (
            user_id, recorded_at,
            step_count, flights_climbed,
            distance_walking_running_meters, distance_cycling_meters, 
            distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters,
            push_count, swimming_stroke_count, nike_fuel,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            exercise_time_minutes, stand_time_minutes, move_time_minutes,
            stand_hour_achieved
        ) VALUES (
            $1, $2, 
            12000, 50, 
            5000.0, 15000.0, 
            1000.0, 2000.0, 
            10000.0,
            500, 2000, 1500.0,
            400.0, 1800.0,
            45, 720, 600,
            true
        )",
        user_id, now
    )
    .execute(&pool)
    .await;

    assert!(result.is_ok(), "All Apple Health fields should insert successfully");

    // Verify data was inserted correctly
    let record = sqlx::query!(
        "SELECT step_count, active_energy_burned_kcal, basal_energy_burned_kcal,
                exercise_time_minutes, stand_time_minutes, move_time_minutes,
                stand_hour_achieved
         FROM activity_metrics_v2 
         WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(record.step_count, Some(12000));
    assert_eq!(record.active_energy_burned_kcal, Some(rust_decimal::Decimal::from(400)));
    assert_eq!(record.basal_energy_burned_kcal, Some(rust_decimal::Decimal::from(1800)));
    assert_eq!(record.exercise_time_minutes, Some(45));
    assert_eq!(record.stand_time_minutes, Some(720));
    assert_eq!(record.move_time_minutes, Some(600));
    assert_eq!(record.stand_hour_achieved, Some(true));

    Ok(())
}

#[sqlx::test]
async fn test_partition_functions(pool: PgPool) -> sqlx::Result<()> {
    // Test the activity_v2 specific partition creation function
    sqlx::query("SELECT create_activity_v2_monthly_partitions(0, 2)")
        .execute(&pool)
        .await?;

    // Verify additional partitions were created
    let partition_count_after = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'activity_metrics_v2_%'"
    )
    .fetch_one(&pool)
    .await?;

    // Should have created at least 2 additional future partitions
    assert!(
        partition_count_after.unwrap_or(0) >= 6, 
        "Should have created additional future partitions"
    );

    // Test the updated maintain_partitions function includes activity_metrics_v2
    sqlx::query("SELECT maintain_partitions()")
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_concurrent_inserts_performance(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id, "performance_test@example.com"
    )
    .execute(&pool)
    .await?;

    let start_time = Instant::now();
    let mut tasks = Vec::new();
    
    // Prepare 1000 concurrent inserts (reduced from 10K for test performance)
    for i in 0..1000 {
        let pool_clone = pool.clone();
        let user_id_clone = user_id;
        
        let task = tokio::spawn(async move {
            let recorded_at = Utc::now() + chrono::Duration::minutes(i);
            sqlx::query!(
                "INSERT INTO activity_metrics_v2 
                 (user_id, recorded_at, step_count, active_energy_burned_kcal) 
                 VALUES ($1, $2, $3, $4)",
                user_id_clone, recorded_at, i as i32 % 50000, (i as f64 * 0.5)
            )
            .execute(&pool_clone)
            .await
        });
        
        tasks.push(task);
    }

    // Wait for all inserts to complete
    for task in tasks {
        task.await.unwrap()?;
    }

    let duration = start_time.elapsed();
    
    // Should complete within reasonable time (more lenient for testing)
    assert!(
        duration.as_millis() < 5000, 
        "1000 concurrent inserts should complete within 5 seconds, took {}ms", 
        duration.as_millis()
    );

    // Verify all records were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count.unwrap_or(0), 1000, "All 1000 records should be inserted");

    Ok(())
}

#[sqlx::test]
async fn test_daily_summary_view(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id, "view_test@example.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc::now().date_naive().and_hms_opt(10, 0, 0).unwrap();
    let base_time = DateTime::from_naive_utc_and_offset(base_time, Utc);

    // Insert multiple records for the same day with different aggregation periods
    for hour in 0..5 {
        sqlx::query!(
            "INSERT INTO activity_metrics_v2 
             (user_id, recorded_at, step_count, active_energy_burned_kcal, 
              exercise_time_minutes, aggregation_period) 
             VALUES ($1, $2, $3, $4, $5, $6)",
            user_id, 
            base_time + chrono::Duration::hours(hour),
            2000, 100.0, 10, "hourly"
        )
        .execute(&pool)
        .await?;
    }

    // Test the daily summary view
    let summary = sqlx::query!(
        "SELECT total_steps, total_active_energy_kcal, total_exercise_minutes, 
                total_records, aggregation_periods_used
         FROM activity_metrics_v2_daily_summary 
         WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(summary.total_steps, Some(10000)); // 5 * 2000
    assert_eq!(summary.total_active_energy_kcal, Some(rust_decimal::Decimal::from(500))); // 5 * 100
    assert_eq!(summary.total_exercise_minutes, Some(50)); // 5 * 10
    assert_eq!(summary.total_records, Some(5));

    Ok(())
}

#[sqlx::test] 
async fn test_performance_analysis_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the performance analysis function
    let result = sqlx::query!(
        "SELECT table_name, partition_count, total_rows 
         FROM analyze_activity_v2_performance()"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.table_name, "activity_metrics_v2");
    assert!(result.partition_count.unwrap_or(0) > 0, "Should have at least 1 partition");
    
    Ok(())
}

#[sqlx::test]
async fn test_edge_case_values(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id, "edge_test@example.com"
    )
    .execute(&pool)
    .await?;

    let now = Utc::now();

    // Test maximum valid values
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, step_count, flights_climbed, 
          active_energy_burned_kcal, exercise_time_minutes) 
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, now, 200000, 10000, 20000.0, 1440
    )
    .execute(&pool)
    .await;
    assert!(result.is_ok(), "Maximum valid values should insert successfully");

    // Test minimum valid values (0 or null)
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 
         (user_id, recorded_at, step_count, flights_climbed, 
          active_energy_burned_kcal, exercise_time_minutes) 
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, now + chrono::Duration::minutes(1), 0, 0, 0.0, 0
    )
    .execute(&pool)
    .await;
    assert!(result.is_ok(), "Minimum valid values should insert successfully");

    // Test NULL values (all optional fields except required ones)
    let result = sqlx::query!(
        "INSERT INTO activity_metrics_v2 (user_id, recorded_at) VALUES ($1, $2)",
        user_id, now + chrono::Duration::minutes(2)
    )
    .execute(&pool)
    .await;
    assert!(result.is_ok(), "Record with only required fields should insert successfully");

    Ok(())
}

#[sqlx::test]
async fn test_rollback_compatibility(pool: PgPool) -> sqlx::Result<()> {
    // Verify that dropping the table and related objects would work for rollback
    
    // Check that we can identify all related objects
    let view_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM information_schema.views WHERE table_name = 'activity_metrics_v2_daily_summary'"
    )
    .fetch_one(&pool)
    .await?;
    
    assert!(view_exists.unwrap_or(0) > 0, "Daily summary view should exist");

    let function_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM information_schema.routines 
         WHERE routine_name IN ('create_activity_v2_monthly_partitions', 'analyze_activity_v2_performance')"
    )
    .fetch_one(&pool)
    .await?;
    
    assert!(function_exists.unwrap_or(0) >= 2, "Activity v2 functions should exist");

    // Note: We don't actually drop anything in the test, just verify rollback would be possible
    Ok(())
}