use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_reproductive_health_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'reproductive_health' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "reproductive_health table should exist");

    // Verify all essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Menstrual tracking
        "menstrual_flow", "spotting", "cycle_day", "cycle_length",
        // Fertility tracking
        "basal_body_temp", "cervical_mucus_quality", "ovulation_test_result", "fertile_window",
        // Pregnancy tracking
        "pregnancy_test_result", "pregnancy_status", "gestational_age_weeks",
        // Sexual health (encrypted)
        "sexual_activity_encrypted", "contraceptive_use_encrypted",
        // Symptoms & mood
        "symptoms", "cycle_related_mood",
        // Metadata
        "source", "notes", "created_at", "updated_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in reproductive_health table", 
            expected_col
        );
    }

    // Verify we have all expected columns (23+ columns)
    assert!(
        column_names.len() >= 20, 
        "Should have at least 20 columns, found: {}", 
        column_names.len()
    );

    println!("✅ reproductive_health table structure validated");
    Ok(())
}

#[sqlx::test]
async fn test_pgcrypto_extension_enabled(pool: PgPool) -> sqlx::Result<()> {
    // Verify pgcrypto extension is enabled
    let result = sqlx::query!(
        "SELECT 1 FROM pg_extension WHERE extname = 'pgcrypto'"
    )
    .fetch_optional(&pool)
    .await?;

    assert!(result.is_some(), "pgcrypto extension should be enabled for field-level encryption");
    
    println!("✅ pgcrypto extension validated");
    Ok(())
}

#[sqlx::test]
async fn test_encryption_functions_exist(pool: PgPool) -> sqlx::Result<()> {
    // Test that our custom encryption functions exist
    let encrypt_function = sqlx::query!(
        "SELECT 1 FROM information_schema.routines 
         WHERE routine_name = 'encrypt_reproductive_data'"
    )
    .fetch_optional(&pool)
    .await?;

    let decrypt_function = sqlx::query!(
        "SELECT 1 FROM information_schema.routines 
         WHERE routine_name = 'decrypt_reproductive_data'"
    )
    .fetch_optional(&pool)
    .await?;

    assert!(encrypt_function.is_some(), "encrypt_reproductive_data function should exist");
    assert!(decrypt_function.is_some(), "decrypt_reproductive_data function should exist");

    println!("✅ Encryption/decryption functions validated");
    Ok(())
}

#[sqlx::test]
async fn test_field_level_encryption_decryption(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "reproductive_health@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 0, 0);

    // Test data to encrypt
    let sexual_activity_data = r#"{"active": true, "timestamp": "2025-09-11T14:00:00Z", "notes": "regular_activity"}"#;
    let contraceptive_data = r#"{"method": "oral_contraceptive", "effectiveness": "high", "notes": "taken_daily_at_8am"}"#;

    // Encrypt the data using our encryption function
    let encrypted_sexual_activity = sqlx::query_scalar!(
        "SELECT encrypt_reproductive_data($1)",
        sexual_activity_data
    )
    .fetch_one(&pool)
    .await?;

    let encrypted_contraceptive = sqlx::query_scalar!(
        "SELECT encrypt_reproductive_data($1)",
        contraceptive_data
    )
    .fetch_one(&pool)
    .await?;

    assert!(encrypted_sexual_activity.is_some(), "Sexual activity data should encrypt successfully");
    assert!(encrypted_contraceptive.is_some(), "Contraceptive data should encrypt successfully");

    // Insert record with encrypted data
    sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, sexual_activity_encrypted, contraceptive_use_encrypted, source) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at,
        encrypted_sexual_activity,
        encrypted_contraceptive,
        "test_encryption"
    )
    .execute(&pool)
    .await?;

    // Retrieve and decrypt the data
    let result = sqlx::query!(
        "SELECT 
           decrypt_reproductive_data(sexual_activity_encrypted) as decrypted_sexual_activity,
           decrypt_reproductive_data(contraceptive_use_encrypted) as decrypted_contraceptive
         FROM reproductive_health 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    // Verify decryption worked correctly
    assert_eq!(result.decrypted_sexual_activity.unwrap(), sexual_activity_data);
    assert_eq!(result.decrypted_contraceptive.unwrap(), contraceptive_data);

    println!("✅ Field-level encryption/decryption validated");
    Ok(())
}

#[sqlx::test]
async fn test_menstrual_flow_enumeration(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "menstrual@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(12, 0, 0);

    // Test all valid menstrual flow values
    let flow_types = ["none", "light", "medium", "heavy", "very_heavy"];

    for (i, flow_type) in flow_types.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health (user_id, recorded_at, menstrual_flow, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            flow_type,
            "test_flow"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid menstrual flow type: {}", 
            flow_type
        );
    }

    // Test invalid flow type
    let invalid_result = sqlx::query!(
        "INSERT INTO reproductive_health (user_id, recorded_at, menstrual_flow, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(10),
        "invalid_flow",
        "test_flow"
    )
    .execute(&pool)
    .await;

    assert!(invalid_result.is_err(), "Should reject invalid menstrual flow type");

    println!("✅ Menstrual flow enumeration validated");
    Ok(())
}

#[sqlx::test]
async fn test_fertility_tracking_fields(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "fertility@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(8, 0, 0);

    // Test valid basal body temperature range (35.0-40.0°C)
    let valid_temps = [36.2, 36.5, 36.8, 37.1, 37.4];
    
    for (i, temp) in valid_temps.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, basal_body_temp, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            rust_decimal::Decimal::from_f64(*temp).unwrap(),
            "test_bbt"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid basal body temperature: {}", 
            temp
        );
    }

    // Test cervical mucus quality enumeration
    let mucus_qualities = ["dry", "sticky", "creamy", "watery", "egg_white", "none"];
    
    for (i, quality) in mucus_qualities.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(6 + i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, cervical_mucus_quality, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            quality,
            "test_mucus"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid cervical mucus quality: {}", 
            quality
        );
    }

    // Test ovulation test results
    let ovulation_results = ["negative", "positive", "peak", "high", "low", "not_tested"];
    
    for (i, result_type) in ovulation_results.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(12 + i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, ovulation_test_result, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            result_type,
            "test_ovulation"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid ovulation test result: {}", 
            result_type
        );
    }

    println!("✅ Fertility tracking fields validated");
    Ok(())
}

#[sqlx::test]
async fn test_pregnancy_tracking_constraints(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "pregnancy@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(10, 0, 0);

    // Test pregnancy test results enumeration
    let test_results = ["negative", "positive", "indeterminate", "not_tested"];
    
    for (i, test_result) in test_results.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, pregnancy_test_result, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            test_result,
            "test_pregnancy_test"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid pregnancy test result: {}", 
            test_result
        );
    }

    // Test pregnancy status enumeration
    let pregnancy_statuses = ["not_pregnant", "trying_to_conceive", "pregnant", "postpartum", "unknown"];
    
    for (i, status) in pregnancy_statuses.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(4 + i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, pregnancy_status, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            status,
            "test_pregnancy_status"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid pregnancy status: {}", 
            status
        );
    }

    // Test gestational age constraints (0-50 weeks)
    let valid_ages = [0, 8, 16, 24, 32, 40, 50];
    
    for (i, age) in valid_ages.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(10 + i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, gestational_age_weeks, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            age,
            "test_gestational_age"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid gestational age: {} weeks", 
            age
        );
    }

    // Test invalid gestational age (should fail)
    let invalid_age_result = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, gestational_age_weeks, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(20),
        60, // Invalid: > 50 weeks
        "test_gestational_age"
    )
    .execute(&pool)
    .await;

    assert!(invalid_age_result.is_err(), "Should reject invalid gestational age > 50 weeks");

    println!("✅ Pregnancy tracking constraints validated");
    Ok(())
}

#[sqlx::test]
async fn test_cycle_constraints(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "cycle@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(12, 0, 0);

    // Test valid cycle day range (1-60)
    let valid_cycle_days = [1, 15, 28, 35, 60];
    
    for (i, cycle_day) in valid_cycle_days.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, cycle_day, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            cycle_day,
            "test_cycle_day"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid cycle day: {}", 
            cycle_day
        );
    }

    // Test valid cycle length range (18-60)
    let valid_cycle_lengths = [18, 21, 28, 35, 45, 60];
    
    for (i, cycle_length) in valid_cycle_lengths.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(6 + i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, cycle_length, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            cycle_length,
            "test_cycle_length"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid cycle length: {}", 
            cycle_length
        );
    }

    // Test invalid cycle day (should fail)
    let invalid_cycle_day = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, cycle_day, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(12),
        0, // Invalid: < 1
        "test_cycle_day"
    )
    .execute(&pool)
    .await;

    assert!(invalid_cycle_day.is_err(), "Should reject invalid cycle day < 1");

    // Test invalid cycle length (should fail)
    let invalid_cycle_length = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, cycle_length, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(13),
        70, // Invalid: > 60
        "test_cycle_length"
    )
    .execute(&pool)
    .await;

    assert!(invalid_cycle_length.is_err(), "Should reject invalid cycle length > 60");

    println!("✅ Cycle day and length constraints validated");
    Ok(())
}

#[sqlx::test]
async fn test_symptoms_array_functionality(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "symptoms@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 30, 0);

    // Test inserting symptoms as text array
    let symptoms = vec!["cramps", "bloating", "breast_tenderness", "mood_swings", "fatigue"];
    
    let insert_result = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, symptoms, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        &symptoms,
        "test_symptoms"
    )
    .execute(&pool)
    .await;

    assert!(insert_result.is_ok(), "Should accept symptoms as text array");

    // Test querying symptoms array
    let result = sqlx::query!(
        "SELECT symptoms FROM reproductive_health 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.symptoms.unwrap(), symptoms);

    // Test GIN index functionality with array searches
    let search_result = sqlx::query!(
        "SELECT COUNT(*) as count FROM reproductive_health 
         WHERE symptoms @> ARRAY['cramps']::TEXT[]"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(search_result.count.unwrap(), 1);

    println!("✅ Symptoms array functionality validated");
    Ok(())
}

#[sqlx::test]
async fn test_mood_enumeration(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "mood@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(16, 0, 0);

    // Test all valid mood values
    let mood_values = ["very_negative", "negative", "neutral", "positive", "very_positive", "not_assessed"];
    
    for (i, mood) in mood_values.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        
        let insert_result = sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, cycle_related_mood, source) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            mood,
            "test_mood"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_result.is_ok(), 
            "Should accept valid mood value: {}", 
            mood
        );
    }

    // Test invalid mood value
    let invalid_mood = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, cycle_related_mood, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(10),
        "invalid_mood",
        "test_mood"
    )
    .execute(&pool)
    .await;

    assert!(invalid_mood.is_err(), "Should reject invalid mood value");

    println!("✅ Cycle-related mood enumeration validated");
    Ok(())
}

#[sqlx::test]
async fn test_audit_logging_trigger(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "audit@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(18, 0, 0);

    // Count existing audit log entries
    let initial_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE action LIKE 'reproductive_health_%'"
    )
    .fetch_one(&pool)
    .await?;

    // Insert a record (should trigger audit log)
    let insert_result = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, menstrual_flow, cycle_day, source) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at,
        "medium",
        14,
        "audit_test"
    )
    .execute(&pool)
    .await;

    assert!(insert_result.is_ok(), "Insert should succeed and trigger audit log");

    // Verify audit log entry was created
    let final_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE action = 'reproductive_health_insert'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        final_count.unwrap() > initial_count.unwrap_or(0),
        "Audit log should have new reproductive_health_insert entry"
    );

    // Verify audit log content
    let audit_entry = sqlx::query!(
        "SELECT metadata FROM audit_log 
         WHERE action = 'reproductive_health_insert' AND user_id = $1 
         ORDER BY created_at DESC LIMIT 1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    let metadata = audit_entry.metadata.unwrap();
    assert!(metadata.as_object().unwrap().contains_key("user_id"));
    assert!(metadata.as_object().unwrap().contains_key("recorded_at"));
    assert!(metadata.as_object().unwrap().contains_key("menstrual_data"));

    println!("✅ Audit logging trigger validated");
    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify that partition creation function exists
    let function_exists = sqlx::query!(
        "SELECT 1 FROM information_schema.routines 
         WHERE routine_name = 'create_reproductive_health_partitions'"
    )
    .fetch_optional(&pool)
    .await?;

    assert!(function_exists.is_some(), "Partition creation function should exist");

    // Test that partitions were created
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM information_schema.tables 
         WHERE table_name LIKE 'reproductive_health_%' 
         AND table_name ~ '^reproductive_health_[0-9]{4}_[0-9]{2}$'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap() >= 6, 
        "Should have at least 6 monthly partitions (3 months back + 3 months ahead)"
    );

    println!("✅ Partitioning setup validated - {} partitions created", partition_count.unwrap());
    Ok(())
}

#[sqlx::test]
async fn test_unique_constraint(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "unique@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(20, 0, 0);

    // Insert first record
    let first_insert = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, menstrual_flow, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        "light",
        "unique_test_1"
    )
    .execute(&pool)
    .await;

    assert!(first_insert.is_ok(), "First insert should succeed");

    // Attempt duplicate insert (should fail due to unique constraint)
    let duplicate_insert = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, menstrual_flow, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at, // Same user_id and recorded_at
        "heavy",
        "unique_test_2"
    )
    .execute(&pool)
    .await;

    assert!(duplicate_insert.is_err(), "Duplicate insert should fail due to unique constraint");

    println!("✅ Unique constraint (user_id, recorded_at) validated");
    Ok(())
}

#[sqlx::test]
async fn test_performance_indexes(pool: PgPool) -> sqlx::Result<()> {
    // Verify that essential indexes were created
    let indexes = sqlx::query!(
        "SELECT indexname FROM pg_indexes 
         WHERE tablename LIKE 'reproductive_health%' 
         ORDER BY indexname"
    )
    .fetch_all(&pool)
    .await?;

    let index_names: Vec<&str> = indexes.iter().map(|r| r.indexname.as_str()).collect();

    // Check for key performance indexes
    let expected_indexes = [
        "idx_reproductive_health_user_id",
        "idx_reproductive_health_user_recorded", 
        "idx_reproductive_health_menstrual_flow",
        "idx_reproductive_health_cycle_tracking",
        "idx_reproductive_health_fertility_window",
        "idx_reproductive_health_pregnancy_status",
        "idx_reproductive_health_symptoms_gin"
    ];

    for expected_index in &expected_indexes {
        let index_exists = index_names.iter().any(|&idx| idx.contains(expected_index));
        assert!(
            index_exists,
            "Performance index '{}' should exist", 
            expected_index
        );
    }

    println!("✅ Performance indexes validated - {} indexes found", index_names.len());
    Ok(())
}

#[sqlx::test]
async fn test_data_retention_function(pool: PgPool) -> sqlx::Result<()> {
    // Verify data retention function exists
    let function_exists = sqlx::query!(
        "SELECT 1 FROM information_schema.routines 
         WHERE routine_name = 'cleanup_old_reproductive_health_data'"
    )
    .fetch_optional(&pool)
    .await?;

    assert!(function_exists.is_some(), "Data retention cleanup function should exist");

    // Test the function execution (should not delete anything in test environment)
    let cleanup_result = sqlx::query_scalar!(
        "SELECT cleanup_old_reproductive_health_data()"
    )
    .fetch_one(&pool)
    .await;

    assert!(cleanup_result.is_ok(), "Data retention function should execute successfully");

    println!("✅ Data retention function validated");
    Ok(())
}

#[sqlx::test]
async fn test_analysis_views_exist(pool: PgPool) -> sqlx::Result<()> {
    // Check that analysis views were created
    let cycle_analysis_view = sqlx::query!(
        "SELECT 1 FROM information_schema.views 
         WHERE table_name = 'reproductive_health_cycle_analysis'"
    )
    .fetch_optional(&pool)
    .await?;

    let fertility_tracking_view = sqlx::query!(
        "SELECT 1 FROM information_schema.views 
         WHERE table_name = 'reproductive_health_fertility_tracking'"
    )
    .fetch_optional(&pool)
    .await?;

    assert!(cycle_analysis_view.is_some(), "Cycle analysis view should exist");
    assert!(fertility_tracking_view.is_some(), "Fertility tracking view should exist");

    println!("✅ Analysis views validated");
    Ok(())
}

#[sqlx::test]
async fn test_basal_body_temperature_constraints(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "bbt@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(6, 0, 0);

    // Test temperature below valid range (should fail)
    let too_low_temp = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, basal_body_temp, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        rust_decimal::Decimal::from_f64(34.5).unwrap(), // Below 35.0°C
        "test_temp_low"
    )
    .execute(&pool)
    .await;

    assert!(too_low_temp.is_err(), "Should reject temperature below 35.0°C");

    // Test temperature above valid range (should fail)
    let too_high_temp = sqlx::query!(
        "INSERT INTO reproductive_health 
         (user_id, recorded_at, basal_body_temp, source) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at + chrono::Duration::hours(1),
        rust_decimal::Decimal::from_f64(40.5).unwrap(), // Above 40.0°C
        "test_temp_high"
    )
    .execute(&pool)
    .await;

    assert!(too_high_temp.is_err(), "Should reject temperature above 40.0°C");

    println!("✅ Basal body temperature constraints validated");
    Ok(())
}

#[sqlx::test]
async fn test_insert_performance(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "performance@test.com"
    )
    .execute(&pool)
    .await?;

    let start_time = Instant::now();
    let base_time = Utc.ymd(2025, 9, 11).and_hms(8, 0, 0);

    // Insert multiple records to test performance
    for i in 0..100 {
        let recorded_at = base_time + chrono::Duration::hours(i);
        
        sqlx::query!(
            "INSERT INTO reproductive_health 
             (user_id, recorded_at, cycle_day, basal_body_temp, menstrual_flow, source) 
             VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            recorded_at,
            ((i % 28) + 1) as i32, // Cycle day 1-28
            rust_decimal::Decimal::from_f64(36.0 + (i as f64 * 0.01)).unwrap(),
            if i % 5 == 0 { Some("light") } else { None },
            "performance_test"
        )
        .execute(&pool)
        .await?;
    }

    let duration = start_time.elapsed();
    
    assert!(
        duration.as_millis() < 5000, // Should complete within 5 seconds
        "100 inserts took too long: {}ms", 
        duration.as_millis()
    );

    println!("✅ Performance test completed - 100 inserts in {}ms", duration.as_millis());
    Ok(())
}