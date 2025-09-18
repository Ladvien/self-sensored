/// Test to verify the fix for STORY-CRITICAL-004: HeartRate Metrics 41% Data Loss
///
/// This test validates that all advanced cardiovascular fields are now properly
/// captured during batch processing, resolving the critical data loss issue.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::config::BatchConfig;
use self_sensored::models::{ActivityContext, HeartRateMetric};
use self_sensored::services::batch_processor::BatchProcessor;

/// Helper function to create a comprehensive HeartRate metric with ALL fields populated
fn create_comprehensive_heart_rate_metric(user_id: Uuid, timestamp_offset_minutes: i64) -> HeartRateMetric {
    let base_time = Utc::now();
    let recorded_at = base_time + chrono::Duration::minutes(timestamp_offset_minutes);

    HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at,
        // Basic heart rate data
        heart_rate: Some(75), // BPM
        resting_heart_rate: Some(65), // BPM
        heart_rate_variability: Some(45.2), // milliseconds

        // Advanced Cardiovascular Metrics (STORY-011) - These were being lost!
        walking_heart_rate_average: Some(105), // Average HR during walking (90-120 BPM normal)
        heart_rate_recovery_one_minute: Some(22), // HR recovery after 1 min (18+ BPM = good)
        atrial_fibrillation_burden_percentage: Some(Decimal::new(125, 2)), // 1.25% AFib burden
        vo2_max_ml_kg_min: Some(Decimal::new(4520, 2)), // 45.20 ml/kg/min VO2 max

        // Context and device info
        context: Some(ActivityContext::Exercise),
        source_device: Some("Apple Watch Series 9".to_string()),
    }
}

#[sqlx::test]
async fn test_heart_rate_complete_data_capture_fix(pool: PgPool) -> sqlx::Result<()> {
    // SETUP: Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash) VALUES ($1, $2, $3)",
        user_id,
        "test-hr-fix@example.com",
        "test_hash"
    )
    .execute(&pool)
    .await?;

    // SETUP: Configure batch processor with correct HeartRate parameters
    let config = BatchConfig {
        heart_rate_chunk_size: 4766, // 11 params: 52,426 total params (safe)
        ..BatchConfig::default()
    };
    let batch_processor = BatchProcessor::with_config(pool.clone(), config);

    // TEST DATA: Create multiple HeartRate metrics with ALL advanced cardiovascular fields
    let heart_rate_metrics = vec![
        create_comprehensive_heart_rate_metric(user_id, 0),   // Current time
        create_comprehensive_heart_rate_metric(user_id, 1),   // +1 minute
        create_comprehensive_heart_rate_metric(user_id, 2),   // +2 minutes
        create_comprehensive_heart_rate_metric(user_id, 3),   // +3 minutes
        create_comprehensive_heart_rate_metric(user_id, 4),   // +4 minutes
    ];

    // CRITICAL TEST: Process the HeartRate metrics through batch processor
    println!("=€ Testing HeartRate batch processing with ALL cardiovascular fields...");
    let result = batch_processor.insert_heart_rates(user_id, heart_rate_metrics.clone()).await;

    // VALIDATION: Ensure all records were processed successfully
    assert!(result.is_ok(), "HeartRate batch processing should succeed: {:?}", result.err());
    let inserted_count = result.unwrap();
    assert_eq!(inserted_count, 5, "All 5 HeartRate metrics should be inserted");

    // CRITICAL VALIDATION: Verify ALL advanced cardiovascular fields are stored
    println!("= Verifying ALL cardiovascular fields are properly stored...");

    for (idx, original_metric) in heart_rate_metrics.iter().enumerate() {
        let stored_record = sqlx::query!(
            r#"
            SELECT
                heart_rate,
                resting_heart_rate,
                heart_rate_variability,
                walking_heart_rate_average,
                heart_rate_recovery_one_minute,
                atrial_fibrillation_burden_percentage,
                vo2_max_ml_kg_min,
                context as "context: ActivityContext",
                source_device
            FROM heart_rate_metrics
            WHERE user_id = $1 AND recorded_at = $2
            "#,
            user_id,
            original_metric.recorded_at
        )
        .fetch_one(&pool)
        .await?;

        // Validate basic heart rate fields
        assert_eq!(stored_record.heart_rate, original_metric.heart_rate.map(|hr| hr as i32));
        assert_eq!(stored_record.resting_heart_rate, original_metric.resting_heart_rate.map(|hr| hr as i32));
        assert_eq!(stored_record.heart_rate_variability, original_metric.heart_rate_variability);

        // CRITICAL: Validate advanced cardiovascular fields (these were being lost!)
        assert_eq!(
            stored_record.walking_heart_rate_average,
            original_metric.walking_heart_rate_average.map(|hr| hr as i32),
            "Walking heart rate average should be stored - was previously lost!"
        );

        assert_eq!(
            stored_record.heart_rate_recovery_one_minute,
            original_metric.heart_rate_recovery_one_minute.map(|hr| hr as i32),
            "Heart rate recovery should be stored - was previously lost!"
        );

        // AFib and VO2 max validation (converted from Decimal to f64)
        if let Some(original_afib) = &original_metric.atrial_fibrillation_burden_percentage {
            let expected_afib = original_afib.to_string().parse::<f64>().unwrap();
            assert!((stored_record.atrial_fibrillation_burden_percentage.unwrap() - expected_afib).abs() < 0.01,
                "AFib burden percentage should be stored - was previously lost!");
        }

        if let Some(original_vo2) = &original_metric.vo2_max_ml_kg_min {
            let expected_vo2 = original_vo2.to_string().parse::<f64>().unwrap();
            assert!((stored_record.vo2_max_ml_kg_min.unwrap() - expected_vo2).abs() < 0.01,
                "VO2 max should be stored - was previously lost!");
        }

        // Validate context and device info
        assert_eq!(stored_record.context, original_metric.context);
        assert_eq!(stored_record.source_device, original_metric.source_device);

        println!(" Record {}: ALL cardiovascular fields validated successfully", idx + 1);
    }

    // PERFORMANCE VALIDATION: Test chunk size safety
    println!("=' Validating PostgreSQL parameter limit safety...");
    let total_params_per_chunk = config.heart_rate_chunk_size * 11; // 11 parameters per HeartRate
    assert!(
        total_params_per_chunk <= 52428, // SAFE_PARAM_LIMIT
        "HeartRate chunk size should be within PostgreSQL parameter limits: {} params",
        total_params_per_chunk
    );
    println!(" Parameter safety validated: {} params per chunk (limit: 52,428)", total_params_per_chunk);

    // DATA INTEGRITY VALIDATION: Ensure no data loss during batch processing
    let total_stored_records = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(
        total_stored_records.count.unwrap() as usize,
        heart_rate_metrics.len(),
        "No HeartRate data should be lost during batch processing"
    );

    println!("<‰ STORY-CRITICAL-004 FIX VALIDATED:");
    println!("   All 5 HeartRate metrics processed successfully (0% data loss)");
    println!("   All 11 cardiovascular fields properly stored (100% data capture)");
    println!("   Advanced metrics (AFib, VO2 max, HR recovery) preserved");
    println!("   PostgreSQL parameter limits respected ({} params/chunk)", total_params_per_chunk);
    println!("  =¨ This resolves the critical 41% HeartRate data loss issue!");

    Ok(())
}

#[sqlx::test]
async fn test_heart_rate_parameter_count_validation(pool: PgPool) -> sqlx::Result<()> {
    // SETUP: Test the parameter count validation logic
    let config = BatchConfig::default();

    // VALIDATION: Ensure HeartRate parameter count is correctly updated
    println!("= Validating HeartRate parameter count constants...");

    // The HEART_RATE_PARAMS_PER_RECORD should be 11, not the previous incorrect value of 10
    use self_sensored::config::HEART_RATE_PARAMS_PER_RECORD;
    assert_eq!(
        HEART_RATE_PARAMS_PER_RECORD,
        11,
        "HeartRate parameter count should be 11 (was incorrectly 10, causing data loss)"
    );

    // VALIDATION: Chunk size calculation should be safe
    let total_params = config.heart_rate_chunk_size * HEART_RATE_PARAMS_PER_RECORD;
    assert!(
        total_params <= 52428,
        "HeartRate total parameters {} should be within safe limit 52,428",
        total_params
    );

    println!(" Parameter count validation passed:");
    println!("   HeartRate params per record: {}", HEART_RATE_PARAMS_PER_RECORD);
    println!("   Chunk size: {}", config.heart_rate_chunk_size);
    println!("   Total params per chunk: {} (safe limit: 52,428)", total_params);

    // VALIDATION: Ensure config validation passes
    let validation_result = config.validate();
    assert!(
        validation_result.is_ok(),
        "BatchConfig validation should pass with updated HeartRate parameters: {:?}",
        validation_result.err()
    );

    println!(" All HeartRate parameter validations passed!");

    Ok(())
}