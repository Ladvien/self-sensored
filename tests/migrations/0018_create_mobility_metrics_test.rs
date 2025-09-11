use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_mobility_metrics_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'mobility_metrics' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "mobility_metrics table should exist");

    // Verify all essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Primary mobility metrics (iOS 14+ Apple Health)
        "walking_speed_m_s", "walking_step_length_cm", "walking_asymmetry_percentage",
        "double_support_percentage", "six_minute_walk_distance_m",
        "stair_ascent_speed_m_s", "stair_descent_speed_m_s",
        // Walking steadiness (iOS 15+)
        "walking_steadiness_score", "walking_steadiness_classification",
        // Additional gait metrics
        "cadence_steps_per_minute", "stride_length_cm", "ground_contact_time_ms",
        "vertical_oscillation_cm",
        // Balance and fall risk
        "postural_sway_mm", "balance_confidence_score", "fall_risk_score",
        // Context
        "surface_type", "measurement_duration_seconds", "measurement_distance_m",
        // Metadata
        "aggregation_period", "measurement_count", "source", "device_model", 
        "ios_version", "raw_data", "notes", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in mobility_metrics", 
            expected_col
        );
    }

    // Verify we have the expected number of columns
    assert!(
        column_names.len() >= 26, 
        "Should have at least 26 columns, found: {}", 
        column_names.len()
    );

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'mobility_metrics'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "mobility_metrics should be partitioned");

    // Verify initial partitions were created (should have 4 months: current + 3 ahead)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'mobility_metrics_y%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 4, 
        "Should have at least 4 partitions, found: {:?}", 
        partition_count
    );

    Ok(())
}

#[sqlx::test]
async fn test_indexes_created(pool: PgPool) -> sqlx::Result<()> {
    // Verify BRIN and other indexes were created
    let indexes = sqlx::query!(
        "SELECT indexname, indexdef FROM pg_indexes 
         WHERE tablename = 'mobility_metrics'"
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        indexes.len() >= 8,
        "Should have at least 8 indexes, found: {}",
        indexes.len()
    );

    // Verify specific indexes exist
    let index_names: Vec<&str> = indexes.iter().map(|r| r.indexname.as_str()).collect();
    
    // Check for BRIN index on recorded_at
    assert!(index_names.iter().any(|&name| name.contains("recorded_at") && name.contains("brin")));
    
    // Check for user_id + recorded_at composite index
    assert!(index_names.iter().any(|&name| name.contains("user_id_recorded_at")));
    
    // Check for walking speed index
    assert!(index_names.iter().any(|&name| name.contains("walking_speed")));
    
    // Check for walking asymmetry index
    assert!(index_names.iter().any(|&name| name.contains("walking_asymmetry")));
    
    // Check for walking steadiness composite index
    assert!(index_names.iter().any(|&name| name.contains("walking_steadiness")));
    
    // Check for six-minute walk test index
    assert!(index_names.iter().any(|&name| name.contains("six_minute_walk")));
    
    // Check for stair speeds composite index
    assert!(index_names.iter().any(|&name| name.contains("stair_speeds")));
    
    // Check for GIN index on raw_data JSONB
    assert!(index_names.iter().any(|&name| name.contains("raw_data") && name.contains("gin")));

    Ok(())
}

#[sqlx::test]
async fn test_insert_basic_mobility_data(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@mobility.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 30, 0);

    // Insert basic mobility data with valid values
    sqlx::query!(
        "INSERT INTO mobility_metrics (
            user_id, recorded_at, walking_speed_m_s, walking_step_length_cm,
            walking_asymmetry_percentage, double_support_percentage, 
            six_minute_walk_distance_m, stair_ascent_speed_m_s, stair_descent_speed_m_s,
            walking_steadiness_score, walking_steadiness_classification,
            cadence_steps_per_minute, stride_length_cm, surface_type, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        user_id,
        recorded_at,
        1.35,        // 1.35 m/s walking speed (normal pace)
        68.5,        // 68.5 cm step length
        2.5,         // 2.5% asymmetry (good)
        22.0,        // 22% double support (normal)
        580.0,       // 580m six-minute walk (good fitness)
        0.8,         // 0.8 m/s stair ascent speed
        1.1,         // 1.1 m/s stair descent speed
        0.85,        // 0.85 walking steadiness (good)
        "OK",        // Good walking steadiness
        115.0,       // 115 steps per minute cadence
        137.0,       // 137 cm stride length (2x step length)
        "flat",      // Flat surface
        "iPhone 14 Pro"
    )
    .execute(&pool)
    .await?;

    // Verify insertion
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mobility_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count.unwrap_or(0), 1, "Should have inserted 1 mobility record");

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_walking_speed_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@walkingspeed.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(15, 0, 0);

    // Test valid walking speeds
    let valid_speeds = [0.5, 1.0, 1.35, 2.0, 3.0, 4.5]; // 0.1-5.0 m/s range
    for (i, speed) in valid_speeds.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid walking speeds (should fail)
    let invalid_speeds = [0.05, 5.1, -1.0]; // Outside 0.1-5.0 m/s range
    for speed in &invalid_speeds {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Walking speed {} should be rejected", speed);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_step_length_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@steplength.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(15, 30, 0);

    // Test valid step lengths
    let valid_lengths = [25.0, 45.0, 68.5, 85.0, 120.0]; // 10-150 cm range
    for (i, length) in valid_lengths.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_step_length_cm, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, length, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid step lengths (should fail)
    let invalid_lengths = [5.0, 151.0, -10.0]; // Outside 10-150 cm range
    for length in &invalid_lengths {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_step_length_cm, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, length, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Step length {} should be rejected", length);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_asymmetry_percentage_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@asymmetry.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(16, 0, 0);

    // Test valid asymmetry percentages (0-100%)
    let valid_asymmetries = [0.0, 2.5, 5.0, 15.0, 50.0, 100.0];
    for (i, asymmetry) in valid_asymmetries.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_asymmetry_percentage, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, asymmetry, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid asymmetry percentages (should fail)
    let invalid_asymmetries = [-1.0, 100.1]; // Outside 0-100% range
    for asymmetry in &invalid_asymmetries {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, walking_asymmetry_percentage, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, asymmetry, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Asymmetry {} should be rejected", asymmetry);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_double_support_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@doublesupport.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(16, 30, 0);

    // Test valid double support percentages (5-60% range for pathological cases)
    let valid_percentages = [5.0, 15.0, 22.0, 35.0, 45.0, 60.0];
    for (i, percentage) in valid_percentages.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, double_support_percentage, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, percentage, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid double support percentages (should fail)
    let invalid_percentages = [4.9, 60.1, -5.0]; // Outside 5-60% range
    for percentage in &invalid_percentages {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, double_support_percentage, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, percentage, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Double support {} should be rejected", percentage);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_six_minute_walk_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@sixminutewalk.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(17, 0, 0);

    // Test valid six-minute walk distances (50-1000m range)
    let valid_distances = [75.0, 200.0, 450.0, 580.0, 750.0, 950.0];
    for (i, distance) in valid_distances.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, six_minute_walk_distance_m, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, distance, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid six-minute walk distances (should fail)
    let invalid_distances = [49.0, 1001.0, -100.0]; // Outside 50-1000m range
    for distance in &invalid_distances {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, six_minute_walk_distance_m, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, distance, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Six-minute walk distance {} should be rejected", distance);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_stair_speed_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@stairspeeds.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(17, 30, 0);

    // Test valid stair ascent speeds (0.1-2.0 m/s range)
    let valid_ascent_speeds = [0.2, 0.5, 0.8, 1.2, 1.8];
    for (i, speed) in valid_ascent_speeds.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, stair_ascent_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test valid stair descent speeds (0.1-2.5 m/s range, typically faster)
    let valid_descent_speeds = [0.3, 0.8, 1.1, 1.5, 2.2];
    for (i, speed) in valid_descent_speeds.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(10 + i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, stair_descent_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid stair ascent speeds (should fail)
    let invalid_ascent_speeds = [0.05, 2.1, -0.5]; // Outside 0.1-2.0 m/s range
    for speed in &invalid_ascent_speeds {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, stair_ascent_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Stair ascent speed {} should be rejected", speed);
    }

    // Test invalid stair descent speeds (should fail)
    let invalid_descent_speeds = [0.08, 2.6, -1.0]; // Outside 0.1-2.5 m/s range
    for speed in &invalid_descent_speeds {
        let test_time = recorded_at + chrono::Duration::hours(2);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (user_id, recorded_at, stair_descent_speed_m_s, source) 
             VALUES ($1, $2, $3, $4)",
            user_id, test_time, speed, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Stair descent speed {} should be rejected", speed);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_walking_steadiness_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@steadiness.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(18, 0, 0);

    // Test valid walking steadiness scores (0.0-1.0 range)
    let valid_scores = [0.0, 0.3, 0.6, 0.85, 1.0];
    let classifications = ["Very Low", "Low", "Low", "OK", "OK"];
    
    for (i, (score, classification)) in valid_scores.iter().zip(classifications.iter()).enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (
                user_id, recorded_at, walking_steadiness_score, 
                walking_steadiness_classification, source
            ) VALUES ($1, $2, $3, $4, $5)",
            user_id, test_time, score, classification, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid walking steadiness scores (should fail)
    let invalid_scores = [-0.1, 1.1]; // Outside 0.0-1.0 range
    for score in &invalid_scores {
        let test_time = recorded_at + chrono::Duration::hours(1);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (
                user_id, recorded_at, walking_steadiness_score, 
                walking_steadiness_classification, source
            ) VALUES ($1, $2, $3, $4, $5)",
            user_id, test_time, score, "OK", "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Walking steadiness score {} should be rejected", score);
    }

    // Test invalid walking steadiness classifications (should fail)
    let invalid_classifications = ["Excellent", "Good", "Bad", "Invalid"];
    for classification in &invalid_classifications {
        let test_time = recorded_at + chrono::Duration::hours(2);
        
        let result = sqlx::query!(
            "INSERT INTO mobility_metrics (
                user_id, recorded_at, walking_steadiness_score, 
                walking_steadiness_classification, source
            ) VALUES ($1, $2, $3, $4, $5)",
            user_id, test_time, 0.8, classification, "Test"
        )
        .execute(&pool)
        .await;

        assert!(result.is_err(), "Walking steadiness classification '{}' should be rejected", classification);
    }

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_stride_step_consistency_constraint(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@stride.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(18, 30, 0);

    // Test valid stride length to step length ratios (1.5x to 2.5x)
    let step_lengths = [60.0, 70.0, 80.0];
    for (i, step_length) in step_lengths.iter().enumerate() {
        let stride_length = step_length * 2.0; // Perfect 2x ratio
        let test_time = recorded_at + chrono::Duration::minutes(i as i64);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (
                user_id, recorded_at, walking_step_length_cm, 
                stride_length_cm, source
            ) VALUES ($1, $2, $3, $4, $5)",
            user_id, test_time, step_length, stride_length, "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid stride length to step length ratios (should fail)
    let test_time = recorded_at + chrono::Duration::hours(1);
    
    // Stride too small (less than 1.5x step length)
    let result = sqlx::query!(
        "INSERT INTO mobility_metrics (
            user_id, recorded_at, walking_step_length_cm, 
            stride_length_cm, source
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time, 70.0, 90.0, "Test" // 1.28x ratio, too small
    )
    .execute(&pool)
    .await;
    
    assert!(result.is_err(), "Stride length too small compared to step length should be rejected");

    // Stride too large (more than 2.5x step length)
    let test_time2 = recorded_at + chrono::Duration::hours(2);
    let result2 = sqlx::query!(
        "INSERT INTO mobility_metrics (
            user_id, recorded_at, walking_step_length_cm, 
            stride_length_cm, source
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time2, 70.0, 190.0, "Test" // 2.71x ratio, too large
    )
    .execute(&pool)
    .await;
    
    assert!(result2.is_err(), "Stride length too large compared to step length should be rejected");

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_performance_views_creation(pool: PgPool) -> sqlx::Result<()> {
    // Verify all performance views were created
    let views = sqlx::query!(
        "SELECT table_name FROM information_schema.views 
         WHERE table_name LIKE '%mobility%' 
         ORDER BY table_name"
    )
    .fetch_all(&pool)
    .await?;

    let view_names: Vec<&str> = views.iter().map(|r| r.table_name.as_str()).collect();
    
    let expected_views = [
        "mobility_daily_summary",
        "mobility_gait_analysis", 
        "mobility_fall_risk_assessment"
    ];

    for expected_view in &expected_views {
        assert!(
            view_names.contains(expected_view),
            "View '{}' should exist", 
            expected_view
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_high_frequency_sampling_performance(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@performance.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(12, 0, 0);
    let start_time = Instant::now();

    // Insert 1000 high-frequency mobility samples (simulating Apple Watch data)
    for i in 0..1000 {
        let recorded_at = base_time + chrono::Duration::seconds(i);
        
        sqlx::query!(
            "INSERT INTO mobility_metrics (
                user_id, recorded_at, walking_speed_m_s, walking_step_length_cm,
                walking_asymmetry_percentage, cadence_steps_per_minute, source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            user_id,
            recorded_at,
            1.2 + (i as f64 * 0.001), // Slight variation in speed
            65.0 + (i as f64 * 0.01),  // Slight variation in step length
            3.0 + (i as f64 * 0.005),  // Slight variation in asymmetry
            110.0 + (i as f64 * 0.02), // Slight variation in cadence
            "Apple Watch Series 8"
        )
        .execute(&pool)
        .await?;
    }

    let insert_duration = start_time.elapsed();
    println!("High-frequency insert time: {:?}", insert_duration);

    // Verify all records were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mobility_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count.unwrap_or(0), 1000, "Should have inserted 1000 mobility records");

    // Test query performance on high-frequency data
    let query_start = Instant::now();
    
    let avg_speed = sqlx::query_scalar!(
        "SELECT AVG(walking_speed_m_s) FROM mobility_metrics 
         WHERE user_id = $1 AND walking_speed_m_s IS NOT NULL",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    let query_duration = query_start.elapsed();
    println!("Query time for 1000 records: {:?}", query_duration);

    assert!(avg_speed.is_some(), "Should calculate average walking speed");
    assert!(query_duration.as_millis() < 100, "Query should complete in under 100ms");

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_aggregation_views_with_sample_data(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@aggregation.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(10, 0, 0);

    // Insert sample mobility data across multiple days
    for day in 0..7 {
        for hour in 0..3 {
            let recorded_at = base_time + chrono::Duration::days(day) + chrono::Duration::hours(hour);
            
            sqlx::query!(
                "INSERT INTO mobility_metrics (
                    user_id, recorded_at, walking_speed_m_s, walking_step_length_cm,
                    walking_asymmetry_percentage, double_support_percentage,
                    walking_steadiness_score, walking_steadiness_classification,
                    balance_confidence_score, fall_risk_score, source
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                user_id,
                recorded_at,
                1.3 + (day as f64 * 0.1),   // Varying walking speed by day
                68.0 + (day as f64 * 2.0),  // Varying step length by day
                4.0 - (day as f64 * 0.3),   // Improving asymmetry over time
                25.0 - (day as f64 * 1.0),  // Improving double support time
                0.7 + (day as f64 * 0.05),  // Improving steadiness score
                if day < 2 { "Low" } else { "OK" }, // Improving classification
                85 + (day * 2) as i16,       // Improving balance confidence
                30 - (day * 3) as i16,       // Decreasing fall risk
                "Test Data"
            )
            .execute(&pool)
            .await?;
        }
    }

    // Test daily summary view
    let daily_summaries = sqlx::query!(
        "SELECT * FROM mobility_daily_summary 
         WHERE user_id = $1 
         ORDER BY summary_date",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(daily_summaries.len(), 7, "Should have 7 days of summary data");

    // Verify daily aggregation calculations
    let first_day = &daily_summaries[0];
    assert!(first_day.avg_walking_speed_m_s.is_some());
    assert!(first_day.avg_walking_step_length_cm.is_some());
    assert_eq!(first_day.measurement_count, Some(3)); // 3 measurements per day

    // Test gait analysis view
    let gait_analysis = sqlx::query!(
        "SELECT * FROM mobility_gait_analysis 
         WHERE user_id = $1 
         ORDER BY week_start",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert!(!gait_analysis.is_empty(), "Should have gait analysis data");
    
    // Verify gait quality calculation
    let analysis = &gait_analysis[0];
    assert!(analysis.gait_quality.is_some());

    // Test fall risk assessment view
    let fall_risk = sqlx::query!(
        "SELECT * FROM mobility_fall_risk_assessment 
         WHERE user_id = $1 
         ORDER BY month_start",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert!(!fall_risk.is_empty(), "Should have fall risk assessment data");
    
    // Verify composite fall risk calculation
    let risk_assessment = &fall_risk[0];
    assert!(risk_assessment.composite_fall_risk.is_some());

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_partition_management_function(pool: PgPool) -> sqlx::Result<()> {
    // Test partition creation function
    sqlx::query!(
        "SELECT create_mobility_metrics_partition($1)",
        chrono::NaiveDate::from_ymd(2026, 3, 15)
    )
    .fetch_one(&pool)
    .await?;

    // Verify the new partition was created
    let partition_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = 'mobility_metrics_y2026m03')"
    )
    .fetch_one(&pool)
    .await?;

    assert!(partition_exists.unwrap_or(false), "March 2026 partition should be created");

    // Test the stats function
    let stats = sqlx::query!(
        "SELECT * FROM mobility_metrics_stats()"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!stats.is_empty(), "Should return mobility metrics statistics");

    Ok(())
}