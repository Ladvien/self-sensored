use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_mental_health_metrics_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'mental_health_metrics' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "mental_health_metrics table should exist");

    // Verify all essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Mental health metrics
        "mindful_minutes", "mood_valence", "mood_labels", "daylight_minutes",
        "stress_level", "depression_score", "anxiety_score", "sleep_quality_score",
        // Metadata
        "source", "raw_data", "notes", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in mental_health_metrics", 
            expected_col
        );
    }

    // Verify we have the expected number of columns
    assert!(
        column_names.len() >= 13, 
        "Should have at least 13 columns, found: {}", 
        column_names.len()
    );

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'mental_health_metrics'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "mental_health_metrics should be partitioned");

    // Verify initial partitions were created (should have 4 months: current + 3 ahead)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'mental_health_metrics_y%'"
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
         WHERE tablename = 'mental_health_metrics'"
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        indexes.len() >= 5,
        "Should have at least 5 indexes, found: {}",
        indexes.len()
    );

    // Verify specific indexes exist
    let index_names: Vec<&str> = indexes.iter().map(|r| r.indexname.as_str()).collect();
    
    // Check for BRIN index on recorded_at
    assert!(index_names.iter().any(|&name| name.contains("recorded_at") && name.contains("brin")));
    
    // Check for user_id + recorded_at composite index
    assert!(index_names.iter().any(|&name| name.contains("user_id_recorded_at")));
    
    // Check for mood valence index
    assert!(index_names.iter().any(|&name| name.contains("mood_valence")));
    
    // Check for GIN index on mood_labels array
    assert!(index_names.iter().any(|&name| name.contains("mood_labels") && name.contains("gin")));
    
    // Check for GIN index on raw_data JSONB
    assert!(index_names.iter().any(|&name| name.contains("raw_data") && name.contains("gin")));

    Ok(())
}

#[sqlx::test]
async fn test_insert_basic_mental_health_data(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@mentalhealth.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 30, 0);

    // Insert basic mental health data
    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mindful_minutes, mood_valence, 
            daylight_minutes, stress_level, depression_score, anxiety_score,
            sleep_quality_score, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        user_id,
        recorded_at,
        20.0,        // 20 minutes mindfulness
        0.7,         // Positive mood
        180.0,       // 3 hours daylight
        "medium",    // Medium stress level
        5i16,        // Low depression score
        3i16,        // Low anxiety score  
        8i16,        // Good sleep quality
        "Apple Health"
    )
    .execute(&pool)
    .await?;

    // Verify insertion
    let result = sqlx::query!(
        "SELECT user_id, mindful_minutes, mood_valence, daylight_minutes, 
                stress_level, depression_score, anxiety_score, sleep_quality_score
         FROM mental_health_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.user_id, user_id);
    assert_eq!(result.mindful_minutes, Some(20.0));
    assert_eq!(result.mood_valence, Some(0.7));
    assert_eq!(result.daylight_minutes, Some(180.0));
    assert_eq!(result.stress_level, Some("medium".to_string()));
    assert_eq!(result.depression_score, Some(5));
    assert_eq!(result.anxiety_score, Some(3));
    assert_eq!(result.sleep_quality_score, Some(8));

    Ok(())
}

#[sqlx::test]
async fn test_mood_valence_range_validation(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "mood@test.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(15, 0, 0);

    // Test valid mood valence values at extremes
    sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id, base_time, -1.0, "Test"
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id, base_time.checked_add_signed(chrono::Duration::minutes(1)).unwrap(), 1.0, "Test"
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id, base_time.checked_add_signed(chrono::Duration::minutes(2)).unwrap(), 0.0, "Test"
    )
    .execute(&pool)
    .await?;

    // Test invalid mood valence values (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id, base_time.checked_add_signed(chrono::Duration::minutes(3)).unwrap(), -1.1, "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with mood_valence < -1.0");

    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id, base_time.checked_add_signed(chrono::Duration::minutes(4)).unwrap(), 1.1, "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with mood_valence > 1.0");

    // Verify valid entries were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(3));

    Ok(())
}

#[sqlx::test]
async fn test_mood_labels_array_operations(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "moods@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(16, 0, 0);

    // Test inserting mood labels array
    let mood_labels = vec!["happy", "energetic", "focused"];
    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mood_labels, mood_valence, source
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at,
        &mood_labels,
        0.8,
        "iOS 17"
    )
    .execute(&pool)
    .await?;

    // Verify array insertion and retrieval
    let result = sqlx::query!(
        "SELECT mood_labels FROM mental_health_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.mood_labels, Some(mood_labels));

    // Test array query operations
    let contains_happy = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM mental_health_metrics 
         WHERE user_id = $1 AND 'happy' = ANY(mood_labels)",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert!(contains_happy.unwrap_or(false), "Should find 'happy' in mood_labels");

    // Test array overlap query
    let overlaps_with_positive = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM mental_health_metrics 
         WHERE user_id = $1 AND mood_labels && ARRAY['happy', 'joyful', 'excited']",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert!(overlaps_with_positive.unwrap_or(false), "Should overlap with positive moods");

    // Test empty array constraint (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mood_labels, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at.checked_add_signed(chrono::Duration::minutes(1)).unwrap(),
        &Vec::<String>::new(),
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with empty mood_labels array");

    Ok(())
}

#[sqlx::test]
async fn test_stress_level_validation(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "stress@test.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(17, 0, 0);

    // Test valid stress levels
    let valid_levels = ["low", "medium", "high", "critical"];
    for (i, level) in valid_levels.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO mental_health_metrics (user_id, recorded_at, stress_level, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            base_time.checked_add_signed(chrono::Duration::minutes(i as i64)).unwrap(),
            level,
            "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test invalid stress level (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, stress_level, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(5)).unwrap(),
        "extreme",
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with invalid stress_level");

    // Verify valid entries were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(4));

    Ok(())
}

#[sqlx::test]
async fn test_screening_scores_validation(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "screening@test.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(18, 0, 0);

    // Test valid screening scores at boundaries
    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, depression_score, anxiety_score, sleep_quality_score, source
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, base_time, 0i16, 0i16, 1i16, "Test" // Minimum values
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, depression_score, anxiety_score, sleep_quality_score, source
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, 
        base_time.checked_add_signed(chrono::Duration::minutes(1)).unwrap(),
        27i16, 21i16, 10i16, "Test" // Maximum values
    )
    .execute(&pool)
    .await?;

    // Test invalid depression score (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, depression_score, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(2)).unwrap(),
        28i16, // Above PHQ-9 max
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with depression_score > 27");

    // Test invalid anxiety score (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, anxiety_score, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(3)).unwrap(),
        22i16, // Above GAD-7 max
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with anxiety_score > 21");

    // Test invalid sleep quality score (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, sleep_quality_score, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(4)).unwrap(),
        11i16, // Above 10-point scale max
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with sleep_quality_score > 10");

    // Verify valid entries were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(2));

    Ok(())
}

#[sqlx::test]
async fn test_minutes_constraints_validation(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "minutes@test.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(19, 0, 0);

    // Test valid minutes at boundaries
    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mindful_minutes, daylight_minutes, source
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id, base_time, 0.0, 0.0, "Test" // Minimum values
    )
    .execute(&pool)
    .await?;

    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mindful_minutes, daylight_minutes, source
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(1)).unwrap(),
        1440.0, 1440.0, "Test" // Maximum values (24 hours)
    )
    .execute(&pool)
    .await?;

    // Test invalid mindful_minutes (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mindful_minutes, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(2)).unwrap(),
        1441.0, // More than 24 hours
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with mindful_minutes > 1440");

    // Test invalid daylight_minutes (should fail)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, daylight_minutes, source
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time.checked_add_signed(chrono::Duration::minutes(3)).unwrap(),
        1441.0, // More than 24 hours
        "Test"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail with daylight_minutes > 1440");

    // Verify valid entries were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(2));

    Ok(())
}

#[sqlx::test]
async fn test_aggregation_queries_mood_trends(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "trends@test.com"
    )
    .execute(&pool)
    .await?;

    let base_date = Utc.ymd(2025, 9, 11);

    // Insert test data over multiple days with different moods
    let test_data = vec![
        (base_date.and_hms(9, 0, 0), 0.8, vec!["happy", "energetic"]),
        (base_date.and_hms(15, 0, 0), 0.5, vec!["calm", "focused"]),
        (base_date.and_hms(21, 0, 0), -0.2, vec!["tired", "stressed"]),
        (base_date.checked_add_signed(chrono::Duration::days(1)).unwrap().and_hms(9, 0, 0), 0.9, vec!["joyful", "excited"]),
        (base_date.checked_add_signed(chrono::Duration::days(1)).unwrap().and_hms(18, 0, 0), 0.3, vec!["neutral", "calm"]),
        (base_date.checked_add_signed(chrono::Duration::days(2)).unwrap().and_hms(12, 0, 0), -0.5, vec!["sad", "anxious"]),
    ];

    for (timestamp, valence, labels) in test_data {
        sqlx::query!(
            "INSERT INTO mental_health_metrics (
                user_id, recorded_at, mood_valence, mood_labels, mindful_minutes, source
            ) VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            timestamp,
            valence,
            &labels,
            10.0,
            "Test"
        )
        .execute(&pool)
        .await?;
    }

    // Test daily summary view
    let daily_summary = sqlx::query!(
        "SELECT summary_date, avg_mood_valence, total_mindful_minutes, all_mood_labels, entry_count
         FROM mental_health_daily_summary 
         WHERE user_id = $1 
         ORDER BY summary_date",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(daily_summary.len(), 3, "Should have 3 days of summary data");
    
    // Check first day average mood
    let first_day_avg = daily_summary[0].avg_mood_valence.unwrap();
    assert!((first_day_avg - 0.366).abs() < 0.01, "First day average should be ~0.366");

    // Test mood trends view
    let mood_trends = sqlx::query!(
        "SELECT week_start, avg_weekly_mood, mood_volatility, total_weekly_mindfulness
         FROM mental_health_mood_trends 
         WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert!(mood_trends.avg_weekly_mood.is_some(), "Should have weekly mood average");
    assert!(mood_trends.mood_volatility.is_some(), "Should have mood volatility measure");
    assert_eq!(mood_trends.total_weekly_mindfulness, Some(60.0), "Should have 60 minutes total mindfulness");

    // Test mood label aggregation
    let mood_label_counts = sqlx::query!(
        "SELECT mood_label, COUNT(*) as label_count
         FROM mental_health_metrics
         CROSS JOIN LATERAL unnest(mood_labels) AS mood_label
         WHERE user_id = $1
         GROUP BY mood_label
         ORDER BY label_count DESC",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert!(!mood_label_counts.is_empty(), "Should have mood label counts");
    assert!(mood_label_counts.iter().any(|row| row.mood_label == "calm"), "Should have 'calm' in results");

    Ok(())
}

#[sqlx::test]
async fn test_ios17_data_import_compatibility(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "ios17@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(20, 0, 0);

    // Simulate iOS 17+ State of Mind data import
    let raw_ios17_data = serde_json::json!({
        "HKCategoryTypeIdentifierStateOfMind": {
            "valence": 0.75,
            "labels": ["happy", "grateful", "proud", "content"],
            "associations": ["work", "family", "exercise"],
            "timestamp": "2025-09-11T20:00:00Z",
            "source": "iPhone",
            "version": "iOS 17.1"
        },
        "HKCategoryTypeIdentifierMindfulSession": {
            "duration": 900, // 15 minutes in seconds
            "timestamp": "2025-09-11T20:00:00Z"
        },
        "additional_context": {
            "daylight_exposure_minutes": 240,
            "stress_level": "low",
            "sleep_quality": 8
        }
    });

    // Insert comprehensive iOS 17+ mental health data
    sqlx::query!(
        "INSERT INTO mental_health_metrics (
            user_id, recorded_at, mindful_minutes, mood_valence, mood_labels,
            daylight_minutes, stress_level, sleep_quality_score, raw_data, source, notes
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        user_id,
        recorded_at,
        15.0, // 15 minutes mindfulness from raw data
        0.75, // iOS 17 mood valence
        &vec!["happy", "grateful", "proud", "content"],
        240.0, // 4 hours daylight
        "low",
        8i16,
        raw_ios17_data,
        "iOS 17.1",
        "Imported from Apple Health with State of Mind feature"
    )
    .execute(&pool)
    .await?;

    // Verify iOS 17 data was stored correctly
    let result = sqlx::query!(
        "SELECT mindful_minutes, mood_valence, mood_labels, daylight_minutes, 
                stress_level, sleep_quality_score, raw_data, source
         FROM mental_health_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    // Verify all iOS 17+ specific fields
    assert_eq!(result.mindful_minutes, Some(15.0));
    assert_eq!(result.mood_valence, Some(0.75));
    assert_eq!(result.mood_labels, Some(vec!["happy".to_string(), "grateful".to_string(), "proud".to_string(), "content".to_string()]));
    assert_eq!(result.daylight_minutes, Some(240.0));
    assert_eq!(result.stress_level, Some("low".to_string()));
    assert_eq!(result.sleep_quality_score, Some(8));
    assert!(result.raw_data.is_some(), "Should have raw iOS data stored");
    assert_eq!(result.source, Some("iOS 17.1".to_string()));

    // Test JSONB queries on raw_data
    let has_state_of_mind = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM mental_health_metrics 
         WHERE user_id = $1 AND raw_data ? 'HKCategoryTypeIdentifierStateOfMind'",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert!(has_state_of_mind.unwrap_or(false), "Should find State of Mind data in raw_data");

    // Test complex JSONB path queries
    let extracted_valence = sqlx::query_scalar!(
        "SELECT (raw_data->'HKCategoryTypeIdentifierStateOfMind'->>'valence')::numeric 
         FROM mental_health_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(extracted_valence, Some(0.75));

    Ok(())
}

#[sqlx::test]
async fn test_performance_views_functionality(pool: PgPool) -> sqlx::Result<()> {
    // Test that performance views work correctly
    
    // Test daily summary view exists and is queryable
    let view_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM information_schema.views 
         WHERE table_name = 'mental_health_daily_summary'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(view_exists.unwrap_or(false), "mental_health_daily_summary view should exist");

    // Test mood trends view exists and is queryable
    let trends_view_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM information_schema.views 
         WHERE table_name = 'mental_health_mood_trends'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(trends_view_exists.unwrap_or(false), "mental_health_mood_trends view should exist");

    Ok(())
}

#[sqlx::test]
async fn test_partition_management_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the partition management function exists
    let function_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_proc 
         WHERE proname = 'create_mental_health_metrics_partition'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(function_exists.unwrap_or(false), "create_mental_health_metrics_partition function should exist");

    // Test creating a new partition
    sqlx::query!(
        "SELECT create_mental_health_metrics_partition($1)",
        chrono::NaiveDate::from_ymd(2026, 2, 1)
    )
    .execute(&pool)
    .await?;

    // Verify new partition was created
    let partition_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_class WHERE relname = 'mental_health_metrics_y2026m02'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(partition_exists.unwrap_or(false), "New partition should be created");

    Ok(())
}

#[sqlx::test]
async fn test_performance_monitoring_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the performance monitoring function exists
    let function_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_proc 
         WHERE proname = 'mental_health_metrics_stats'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(function_exists.unwrap_or(false), "mental_health_metrics_stats function should exist");

    // Test calling the function
    let stats = sqlx::query!(
        "SELECT * FROM mental_health_metrics_stats()"
    )
    .fetch_all(&pool)
    .await?;

    // Should return stats for existing partitions
    assert!(stats.len() >= 4, "Should return stats for at least 4 partitions");

    Ok(())
}

#[sqlx::test]
async fn test_unique_constraint_enforcement(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "unique@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(21, 0, 0);

    // Insert first record
    sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        0.5,
        "Test"
    )
    .execute(&pool)
    .await?;

    // Try to insert duplicate (should fail due to unique constraint)
    let result = sqlx::query!(
        "INSERT INTO mental_health_metrics (user_id, recorded_at, mood_valence, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        0.7,
        "Test Duplicate"
    )
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Should fail due to unique constraint on (user_id, recorded_at)");

    // Verify only one record exists
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mental_health_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(1));

    Ok(())
}