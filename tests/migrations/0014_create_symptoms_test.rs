use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_symptoms_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'symptoms' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "symptoms table should exist");

    // Verify all essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Symptom tracking fields
        "symptom_type", "severity", "duration_minutes", "onset_at",
        // Context fields
        "notes", "triggers", "treatments",
        // Metadata
        "source", "raw_data", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in symptoms table", 
            expected_col
        );
    }

    // Verify we have all expected columns
    assert!(
        column_names.len() >= 12, 
        "Should have at least 12 columns, found: {}", 
        column_names.len()
    );

    Ok(())
}

#[sqlx::test]
async fn test_symptom_type_enumeration(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "symptoms@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(12, 0, 0);

    // Test all major symptom categories - sample from each category
    let symptom_types = [
        // General/Constitutional
        "fever", "fatigue", "weakness", "night_sweats", "chills",
        // Head & Neurological  
        "headache", "dizziness", "confusion", "mood_changes", "anxiety",
        // Respiratory
        "cough", "shortness_of_breath", "chest_tightness_or_pain", "wheezing", "runny_nose",
        // Gastrointestinal
        "nausea", "vomiting", "abdominal_cramps", "bloating", "diarrhea",
        // Musculoskeletal & Pain
        "body_and_muscle_aches", "joint_pain", "back_pain", "muscle_cramps",
        // Skin & Dermatological
        "dry_skin", "rash", "itching", "acne",
        // Genitourinary & Reproductive
        "pelvic_pain", "vaginal_dryness", "bladder_incontinence",
        // Sleep & Rest
        "sleep_changes", "insomnia", "excessive_sleepiness",
        // Sensory & Perception
        "vision_changes", "hearing_changes", "taste_changes",
        // Other
        "hot_flashes", "tremor", "irregular_heartbeat"
    ];

    // Test inserting each symptom type
    for (i, symptom_type) in symptom_types.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64 * 5);
        
        let insert_result = sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            symptom_type,
            "mild"
        )
        .execute(&pool)
        .await;

        assert!(insert_result.is_ok(), "Symptom type '{}' should be valid", symptom_type);
    }

    // Test invalid symptom type
    let invalid_insert = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(1),
        "invalid_symptom_type",
        "mild"
    )
    .execute(&pool)
    .await;

    assert!(invalid_insert.is_err(), "Invalid symptom type should fail");

    // Verify all symptom types were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM symptoms WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(symptom_types.len() as i64));

    Ok(())
}

#[sqlx::test]
async fn test_severity_validation(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "severity@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(13, 0, 0);

    // Test all valid severity levels
    let valid_severities = ["not_present", "mild", "moderate", "severe"];
    for (i, severity) in valid_severities.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64 * 10);
        
        let insert_result = sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            "headache",
            severity
        )
        .execute(&pool)
        .await;

        assert!(insert_result.is_ok(), "Severity '{}' should be valid", severity);
    }

    // Test invalid severity - comprehensive edge cases
    let invalid_severities = [
        "extreme",     // Not in valid enum
        "invalid",    // Random string
        "severe_plus", // Close to valid but invalid
        "MILD",       // Wrong case
        "Moderate",   // Mixed case
        "not present", // Space instead of underscore
        "",           // Empty string
        "none",       // Different valid-sounding option
        "low",        // Alternative valid-sounding option
        "high",       // Alternative valid-sounding option
        "critical",   // Medical-sounding but invalid
    ];

    for (i, invalid_severity) in invalid_severities.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64 + 1);
        let invalid_insert = sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            "headache",
            invalid_severity
        )
        .execute(&pool)
        .await;

        assert!(
            invalid_insert.is_err(), 
            "Invalid severity '{}' should fail constraint validation", 
            invalid_severity
        );
    }

    // Test NULL severity (should fail since severity is required)
    let null_severity_insert = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
         VALUES ($1, $2, $3, NULL)",
        user_id,
        recorded_at + chrono::Duration::hours(20)
    )
    .execute(&pool)
    .await;

    assert!(null_severity_insert.is_err(), "NULL severity should fail NOT NULL constraint");

    Ok(())
}

#[sqlx::test]
async fn test_duration_validation(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "duration@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 0, 0);

    // Test valid duration values
    let valid_durations = [0, 30, 120, 480, 1440, 10080]; // 0 min to 1 week
    for (i, duration) in valid_durations.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64 * 15);
        
        let insert_result = sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes) 
             VALUES ($1, $2, $3, $4, $5)",
            user_id,
            test_time,
            "fatigue",
            "moderate",
            duration
        )
        .execute(&pool)
        .await;

        assert!(insert_result.is_ok(), "Duration {} minutes should be valid", duration);
    }

    // Test invalid duration (too high)
    let invalid_insert = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at + chrono::Duration::hours(2),
        "fatigue",
        "moderate",
        20160 // 2 weeks - too long
    )
    .execute(&pool)
    .await;

    assert!(invalid_insert.is_err(), "Duration >1 week should fail");

    // Test negative duration
    let negative_insert = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at + chrono::Duration::hours(3),
        "fatigue",
        "moderate",
        -30 // Negative duration
    )
    .execute(&pool)
    .await;

    assert!(negative_insert.is_err(), "Negative duration should fail");

    Ok(())
}

#[sqlx::test]
async fn test_complex_symptom_data(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "complex@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(15, 0, 0);
    let onset_at = recorded_at - chrono::Duration::hours(2);

    // Insert complex symptom with all fields
    sqlx::query!(
        "INSERT INTO symptoms (
            user_id, recorded_at, symptom_type, severity, duration_minutes, onset_at,
            notes, triggers, treatments, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        user_id,
        recorded_at,
        "nausea",
        "severe",
        180, // 3 hours
        onset_at,
        "Severe morning nausea after eating breakfast",
        serde_json::json!(["greasy_food", "lack_of_sleep", "stress"]),
        serde_json::json!(["ginger_tea", "rest", "deep_breathing"]),
        "Apple Health"
    )
    .execute(&pool)
    .await?;

    // Verify insertion with all fields
    let result = sqlx::query!(
        "SELECT symptom_type, severity, duration_minutes, onset_at, notes, 
                triggers, treatments, source
         FROM symptoms 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.symptom_type, "nausea");
    assert_eq!(result.severity, "severe");
    assert_eq!(result.duration_minutes, Some(180));
    assert_eq!(result.onset_at, Some(onset_at));
    assert_eq!(result.notes, Some("Severe morning nausea after eating breakfast".to_string()));
    assert_eq!(result.source, Some("Apple Health".to_string()));
    
    // Verify JSON fields
    assert!(result.triggers.is_some());
    assert!(result.treatments.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'symptoms'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "symptoms table should be partitioned");

    // Verify initial partitions were created (should have 3+ months)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'symptoms_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 3, 
        "Should have at least 3 partitions, found: {:?}", 
        partition_count
    );

    Ok(())
}

#[sqlx::test]
async fn test_brin_indexes_created(pool: PgPool) -> sqlx::Result<()> {
    // Verify BRIN indexes were created
    let brin_indexes = sqlx::query!(
        "SELECT indexname FROM pg_indexes 
         WHERE tablename = 'symptoms' 
         AND indexname LIKE '%_brin'"
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        brin_indexes.len() >= 3,
        "Should have at least 3 BRIN indexes, found: {}",
        brin_indexes.len()
    );

    // Verify specific BRIN indexes exist
    let index_names: Vec<&str> = brin_indexes.iter().map(|r| r.indexname.as_str()).collect();
    assert!(index_names.iter().any(|&name| name.contains("recorded_at")));
    assert!(index_names.iter().any(|&name| name.contains("user_recorded")));
    assert!(index_names.iter().any(|&name| name.contains("type_recorded")));

    Ok(())
}

#[sqlx::test]
async fn test_composite_indexes(pool: PgPool) -> sqlx::Result<()> {
    // Verify composite indexes were created for symptom analysis
    let indexes = sqlx::query!(
        "SELECT indexname FROM pg_indexes 
         WHERE tablename = 'symptoms' 
         ORDER BY indexname"
    )
    .fetch_all(&pool)
    .await?;

    let index_names: Vec<&str> = indexes.iter().map(|r| r.indexname.as_str()).collect();
    
    // Verify key composite indexes exist
    assert!(index_names.iter().any(|&name| name.contains("user_type_recorded")));
    assert!(index_names.iter().any(|&name| name.contains("user_severity_recorded")));
    assert!(index_names.iter().any(|&name| name.contains("user_onset")));

    Ok(())
}

#[sqlx::test]
async fn test_json_indexes(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "json@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(16, 0, 0);

    // Insert symptom with triggers and treatments
    sqlx::query!(
        "INSERT INTO symptoms (
            user_id, recorded_at, symptom_type, severity, 
            triggers, treatments
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        "headache",
        "moderate",
        serde_json::json!(["stress", "bright_lights", "loud_noise"]),
        serde_json::json!(["ibuprofen", "dark_room", "cold_compress"])
    )
    .execute(&pool)
    .await?;

    // Test JSON query using GIN index (should use index efficiently)
    let stress_result = sqlx::query!(
        "SELECT COUNT(*) as count FROM symptoms 
         WHERE triggers ? 'stress'"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(stress_result.count, Some(1));

    // Test treatment query
    let ibuprofen_result = sqlx::query!(
        "SELECT COUNT(*) as count FROM symptoms 
         WHERE treatments ? 'ibuprofen'"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(ibuprofen_result.count, Some(1));

    Ok(())
}

#[sqlx::test]
async fn test_concurrent_symptom_logging(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "concurrent@test.com"
    )
    .execute(&pool)
    .await?;

    let base_time = Utc.ymd(2025, 9, 11).and_hms(17, 0, 0);

    // Simulate concurrent logging of multiple symptoms at the same time
    let symptoms = [
        ("headache", "moderate"),
        ("nausea", "mild"),
        ("fatigue", "severe"),
        ("dizziness", "mild"),
        ("body_and_muscle_aches", "moderate")
    ];

    // Insert all symptoms at the same time (different symptom types)
    for (symptom_type, severity) in &symptoms {
        sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            base_time,
            symptom_type,
            severity
        )
        .execute(&pool)
        .await?;
    }

    // Verify all symptoms were recorded
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM symptoms 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        base_time
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(symptoms.len() as i64));

    // Test duplicate prevention (same user, time, and symptom type)
    let duplicate_insert = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        base_time,
        "headache", // Same as first symptom
        "severe"
    )
    .execute(&pool)
    .await;

    assert!(duplicate_insert.is_err(), "Duplicate symptom entry should fail due to unique constraint");

    Ok(())
}

#[sqlx::test]
async fn test_symptom_history_query_performance(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "performance@test.com"
    )
    .execute(&pool)
    .await?;

    // Insert 3 months of symptom data (simulate daily tracking)
    let start_date = Utc.ymd(2025, 6, 11);
    let symptom_types = ["headache", "fatigue", "nausea", "back_pain", "anxiety"];
    let severities = ["mild", "moderate", "severe"];

    for day in 0..90 { // 3 months
        let record_date = start_date + chrono::Duration::days(day);
        
        // Random 1-3 symptoms per day
        for i in 0..(day % 3 + 1) {
            let symptom_type = symptom_types[(day + i) as usize % symptom_types.len()];
            let severity = severities[(day * 2 + i) as usize % severities.len()];
            let record_time = record_date.and_hms(9 + (i * 4) as u32, 0, 0);
            
            sqlx::query!(
                "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes) 
                 VALUES ($1, $2, $3, $4, $5)",
                user_id,
                record_time,
                symptom_type,
                severity,
                30 + (i * 30) as i32 // 30-120 minutes
            )
            .execute(&pool)
            .await?;
        }
    }

    // Performance test: Query 3-month symptom history
    let start_time = Instant::now();
    
    let history = sqlx::query!(
        "SELECT symptom_type, severity, recorded_at, duration_minutes 
         FROM symptoms 
         WHERE user_id = $1 
           AND recorded_at >= $2 
           AND recorded_at <= $3
         ORDER BY recorded_at DESC",
        user_id,
        start_date.naive_utc(),
        (start_date + chrono::Duration::days(90)).naive_utc()
    )
    .fetch_all(&pool)
    .await?;

    let query_duration = start_time.elapsed();
    
    // Should complete in under 50ms as per requirements
    assert!(
        query_duration.as_millis() < 50,
        "3-month symptom history query took too long: {:?}ms (should be <50ms)",
        query_duration.as_millis()
    );

    // Verify we got the expected data
    assert!(!history.is_empty(), "Should have symptom history");
    println!("✅ 3-month symptom history query completed in {:?}", query_duration);

    Ok(())
}

#[sqlx::test]
async fn test_symptoms_severity_summary_view(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "summary@test.com"
    )
    .execute(&pool)
    .await?;

    let test_date = Utc.ymd(2025, 9, 11);
    
    // Insert multiple episodes of the same symptom on the same day
    let episodes = [
        (test_date.and_hms(8, 0, 0), "mild", 60),
        (test_date.and_hms(14, 0, 0), "moderate", 90), 
        (test_date.and_hms(20, 0, 0), "severe", 120),
    ];

    for (record_time, severity, duration) in &episodes {
        sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes, source) 
             VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            record_time,
            "headache",
            severity,
            duration,
            "manual"
        )
        .execute(&pool)
        .await?;
    }

    // Query the severity summary view
    let summary = sqlx::query!(
        "SELECT max_severity_score, symptom_episodes, total_duration_minutes, 
                avg_duration_minutes, data_sources
         FROM symptoms_severity_summary 
         WHERE user_id = $1 AND symptom_date = $2 AND symptom_type = $3",
        user_id,
        test_date.naive_utc().date(),
        "headache"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(summary.max_severity_score, Some(3)); // Severe = 3
    assert_eq!(summary.symptom_episodes, Some(3)); // 3 episodes
    assert_eq!(summary.total_duration_minutes, Some(270)); // 60+90+120
    assert_eq!(summary.avg_duration_minutes, Some(90.0)); // 270/3
    assert!(summary.data_sources.as_ref().unwrap().contains(&Some("manual".to_string())));

    Ok(())
}

#[sqlx::test]
async fn test_symptoms_daily_summary_view(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "daily@test.com"
    )
    .execute(&pool)
    .await?;

    let test_date = Utc.ymd(2025, 9, 11);
    
    // Insert various symptoms with different severities
    let symptoms = [
        (test_date.and_hms(9, 0, 0), "headache", "severe", 120),
        (test_date.and_hms(10, 0, 0), "nausea", "moderate", 60),
        (test_date.and_hms(11, 0, 0), "fatigue", "mild", 240),
        (test_date.and_hms(12, 0, 0), "dizziness", "moderate", 30),
    ];

    for (record_time, symptom_type, severity, duration) in &symptoms {
        sqlx::query!(
            "INSERT INTO symptoms (
                user_id, recorded_at, symptom_type, severity, duration_minutes,
                triggers, treatments
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            user_id,
            record_time,
            symptom_type,
            severity,
            duration,
            serde_json::json!(["stress"]),
            serde_json::json!(["rest"])
        )
        .execute(&pool)
        .await?;
    }

    // Query the daily summary view
    let summary = sqlx::query!(
        "SELECT unique_symptoms_count, severe_symptoms_count, moderate_symptoms_count, 
                mild_symptoms_count, total_symptom_duration_minutes, 
                daily_symptom_burden_score, symptoms_with_triggers, symptoms_with_treatments
         FROM symptoms_daily_summary 
         WHERE user_id = $1 AND symptom_date = $2",
        user_id,
        test_date.naive_utc().date()
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(summary.unique_symptoms_count, Some(4)); // 4 different symptoms
    assert_eq!(summary.severe_symptoms_count, Some(1)); // 1 severe (headache)
    assert_eq!(summary.moderate_symptoms_count, Some(2)); // 2 moderate (nausea, dizziness)
    assert_eq!(summary.mild_symptoms_count, Some(1)); // 1 mild (fatigue)
    assert_eq!(summary.total_symptom_duration_minutes, Some(450)); // 120+60+240+30
    assert_eq!(summary.daily_symptom_burden_score, Some(8)); // 3+2+1+2 = 8
    assert_eq!(summary.symptoms_with_triggers, Some(4)); // All have triggers
    assert_eq!(summary.symptoms_with_treatments, Some(4)); // All have treatments

    Ok(())
}

#[sqlx::test]
async fn test_performance_monitoring_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the performance analysis function
    let result = sqlx::query!(
        "SELECT * FROM analyze_symptoms_performance()"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.table_name, Some("symptoms".to_string()));
    assert!(result.partition_count.unwrap_or(0) > 0, "Should have partitions");

    Ok(())
}

#[sqlx::test]
async fn test_partition_management_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the partition creation function
    sqlx::query!("SELECT create_symptoms_monthly_partitions(2, 6)")
        .execute(&pool)
        .await?;

    // Verify additional partitions were created
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'symptoms_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 8, // Should have at least 8 partitions now (2 back + 6 ahead)
        "Should have created additional partitions"
    );

    Ok(())
}

#[sqlx::test]
async fn test_comprehensive_symptom_categories(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "comprehensive@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(18, 0, 0);

    // Test comprehensive symptom coverage - all major categories
    let comprehensive_symptoms = [
        // General/Constitutional (9 symptoms)
        "fever", "fatigue", "weakness", "night_sweats", "chills", 
        "malaise", "appetite_loss", "weight_loss", "weight_gain",
        
        // Head & Neurological (10 symptoms)  
        "headache", "dizziness", "lightheadedness", "confusion", "memory_issues",
        "concentration_difficulty", "mood_changes", "anxiety", "depression",
        
        // Respiratory (8 symptoms)
        "cough", "shortness_of_breath", "chest_tightness_or_pain", "wheezing", 
        "runny_nose", "sinus_congestion", "sneezing", "sore_throat",
        
        // Gastrointestinal (10 symptoms)
        "nausea", "vomiting", "abdominal_cramps", "bloating", "diarrhea",
        "constipation", "heartburn", "acid_reflux", "stomach_pain", "gas", "indigestion",
        
        // Musculoskeletal & Pain (7 symptoms)
        "body_and_muscle_aches", "joint_pain", "back_pain", "neck_pain", 
        "muscle_cramps", "stiffness", "swelling",
        
        // Skin & Dermatological (5 symptoms)
        "dry_skin", "rash", "itching", "acne", "skin_irritation",
        
        // Genitourinary & Reproductive (5 symptoms)
        "pelvic_pain", "vaginal_dryness", "bladder_incontinence", 
        "frequent_urination", "painful_urination",
        
        // Sleep & Rest (4 symptoms)
        "sleep_changes", "insomnia", "excessive_sleepiness", "sleep_disturbances",
        
        // Sensory & Perception (4 symptoms)
        "vision_changes", "hearing_changes", "taste_changes", "smell_changes",
        
        // Other Symptoms (6 symptoms)
        "hot_flashes", "cold_intolerance", "heat_intolerance", 
        "hair_loss", "tremor", "irregular_heartbeat"
    ];

    println!("Testing {} comprehensive symptom types", comprehensive_symptoms.len());

    // Insert all comprehensive symptom types
    for (i, symptom_type) in comprehensive_symptoms.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::minutes(i as i64 * 2);
        let severity = match i % 4 {
            0 => "mild",
            1 => "moderate", 
            2 => "severe",
            _ => "not_present"
        };
        
        let insert_result = sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            symptom_type,
            severity
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Comprehensive symptom type '{}' should be valid", 
            symptom_type
        );
    }

    // Verify all symptoms were inserted
    let total_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM symptoms WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(total_count, Some(comprehensive_symptoms.len() as i64));

    // Verify symptom category distribution
    let category_stats = sqlx::query!(
        "SELECT 
            COUNT(DISTINCT symptom_type) as unique_symptoms,
            COUNT(*) FILTER (WHERE severity != 'not_present') as present_symptoms,
            COUNT(*) FILTER (WHERE severity = 'severe') as severe_symptoms
         FROM symptoms 
         WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(category_stats.unique_symptoms, Some(comprehensive_symptoms.len() as i64));
    println!("✅ Successfully tested {} comprehensive Apple Health symptom types", comprehensive_symptoms.len());

    Ok(())
}

#[sqlx::test]
async fn test_symptom_correlation_analysis(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "correlation@test.com"
    )
    .execute(&pool)
    .await?;

    let base_date = Utc.ymd(2025, 9, 11);

    // Insert related symptoms that might correlate (headache + nausea + light sensitivity pattern)
    let symptom_clusters = [
        // Cluster 1: Migraine-like pattern
        (base_date.and_hms(9, 0, 0), "headache", "severe", 240),
        (base_date.and_hms(9, 15, 0), "nausea", "moderate", 180),
        (base_date.and_hms(9, 30, 0), "vision_changes", "mild", 120),
        
        // Cluster 2: Stress/anxiety pattern
        (base_date.and_hms(14, 0, 0), "anxiety", "moderate", 90),
        (base_date.and_hms(14, 30, 0), "body_and_muscle_aches", "mild", 60),
        (base_date.and_hms(15, 0, 0), "concentration_difficulty", "moderate", 120),
    ];

    for (record_time, symptom_type, severity, duration) in &symptom_clusters {
        sqlx::query!(
            "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, duration_minutes) 
             VALUES ($1, $2, $3, $4, $5)",
            user_id,
            record_time,
            symptom_type,
            severity,
            duration
        )
        .execute(&pool)
        .await?;
    }

    // Query for symptom correlations within time windows
    let correlation_query = sqlx::query!(
        "SELECT 
            s1.symptom_type as primary_symptom,
            s2.symptom_type as related_symptom,
            ABS(EXTRACT(epoch FROM (s2.recorded_at - s1.recorded_at))) / 60 as minutes_apart
         FROM symptoms s1 
         JOIN symptoms s2 ON s1.user_id = s2.user_id
         WHERE s1.user_id = $1 
           AND s1.symptom_type != s2.symptom_type
           AND s1.recorded_at::date = s2.recorded_at::date
           AND ABS(EXTRACT(epoch FROM (s2.recorded_at - s1.recorded_at))) <= 3600 -- Within 1 hour
         ORDER BY s1.recorded_at, minutes_apart",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert!(!correlation_query.is_empty(), "Should find symptom correlations");
    
    // Verify specific correlations were found
    let has_headache_nausea = correlation_query.iter().any(|r| 
        (r.primary_symptom == "headache" && r.related_symptom == "nausea") ||
        (r.primary_symptom == "nausea" && r.related_symptom == "headache")
    );
    
    assert!(has_headache_nausea, "Should detect headache-nausea correlation");

    Ok(())
}

#[sqlx::test]
async fn test_time_constraints_validation(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "timetest@test.com"
    )
    .execute(&pool)
    .await?;

    let now = Utc::now();

    // Test future onset constraint (should fail)
    let future_onset = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, onset_at) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        now,
        "headache",
        "mild",
        now + chrono::Duration::hours(2) // Future onset
    )
    .execute(&pool)
    .await;

    assert!(future_onset.is_err(), "Future onset should fail constraint");

    // Test far future recorded_at (should fail)
    let far_future = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        now + chrono::Duration::days(30), // Too far in future
        "headache",
        "mild"
    )
    .execute(&pool)
    .await;

    assert!(far_future.is_err(), "Far future recorded_at should fail constraint");

    // Test valid past onset (should succeed)
    let valid_past = sqlx::query!(
        "INSERT INTO symptoms (user_id, recorded_at, symptom_type, severity, onset_at) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        now,
        "headache",
        "mild",
        now - chrono::Duration::hours(2) // Past onset
    )
    .execute(&pool)
    .await;

    assert!(valid_past.is_ok(), "Valid past onset should succeed");

    Ok(())
}