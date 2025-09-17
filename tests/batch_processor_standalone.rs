// Simple tests for batch processor without depending on other modules
#[tokio::test]
async fn test_postgresql_parameter_limits_validation() {
    use self_sensored::config::BatchConfig;

    // Test that default configuration is safe
    let safe_config = BatchConfig::default();
    assert!(
        safe_config.validate().is_ok(),
        "Default configuration should pass validation"
    );

    // Test critical cases that would cause data loss
    let dangerous_config = BatchConfig {
        activity_chunk_size: 7000, // 19 params * 7000 = 133,000 params (way over limit!)
        temperature_chunk_size: 8000, // 8 params * 8000 = 64,000 params (over limit)
        sleep_chunk_size: 6000,    // 10 params * 6000 = 60,000 params (over safe limit)
        ..BatchConfig::default()
    };

    let validation_result = dangerous_config.validate();
    assert!(
        validation_result.is_err(),
        "Dangerous configuration should fail validation"
    );

    let error_message = validation_result.unwrap_err();
    assert!(
        error_message.contains("activity chunk size 7000"),
        "Should flag activity chunk size"
    );
    assert!(
        error_message.contains("temperature chunk size 8000"),
        "Should flag temperature chunk size"
    );
    assert!(
        error_message.contains("SILENT DATA LOSS"),
        "Should warn about data loss"
    );

    // Test that fixed configuration passes
    let fixed_config = BatchConfig {
        activity_chunk_size: 2700,    // 19 params * 2700 = 51,300 params (safe)
        temperature_chunk_size: 6500, // 8 params * 6500 = 52,000 params (safe)
        sleep_chunk_size: 5200,       // 10 params * 5200 = 52,000 params (safe)
        ..BatchConfig::default()
    };

    assert!(
        fixed_config.validate().is_ok(),
        "Fixed configuration should pass validation"
    );
}

#[tokio::test]
async fn test_batch_processor_compilation() {
    // This test just verifies the batch processor compiles correctly
    // It doesn't run actual database operations

    // Check that we can create batch processor types
    use self_sensored::config::BatchConfig;
    use self_sensored::services::batch_processor::ProcessingStatus;

    let config = BatchConfig::default();
    assert_eq!(config.max_retries, 3);
    assert!(config.enable_parallel_processing);
    assert_eq!(config.chunk_size, 1000);
    assert_eq!(config.memory_limit_mb, 500.0);

    // Test ProcessingStatus enum
    let status = ProcessingStatus::Pending;
    assert_eq!(status, ProcessingStatus::Pending);

    let status = ProcessingStatus::InProgress;
    assert_eq!(status, ProcessingStatus::InProgress);

    let status = ProcessingStatus::Completed;
    assert_eq!(status, ProcessingStatus::Completed);

    let status = ProcessingStatus::Failed;
    assert_eq!(status, ProcessingStatus::Failed);

    let status = ProcessingStatus::Retrying;
    assert_eq!(status, ProcessingStatus::Retrying);
}

#[tokio::test]
async fn test_batch_config_custom() {
    use self_sensored::config::BatchConfig;

    let config = BatchConfig {
        max_retries: 5,
        initial_backoff_ms: 200,
        max_backoff_ms: 10000,
        enable_parallel_processing: false,
        chunk_size: 2000,
        memory_limit_mb: 1000.0,
        // Use SAFE chunk sizes to prevent PostgreSQL parameter limit violations
        heart_rate_chunk_size: 4200, // 10 params: 42,000 total params (safe)
        blood_pressure_chunk_size: 8000, // 6 params: 48,000 total params (safe)
        sleep_chunk_size: 5200,      // 10 params: 52,000 total params (safe)
        activity_chunk_size: 2700,   // 19 params: 51,300 total params (safe)
        body_measurement_chunk_size: 3000,
        temperature_chunk_size: 6500, // 8 params: 52,000 total params (safe)
        respiratory_chunk_size: 7000,
        workout_chunk_size: 5000,
        blood_glucose_chunk_size: 6500,
        nutrition_chunk_size: 1600,
        menstrual_chunk_size: 6500,
        fertility_chunk_size: 4300,
        enable_progress_tracking: true,
        enable_intra_batch_deduplication: false,
        enable_dual_write_activity_metrics: false,
        enable_reproductive_health_encryption: true,
        reproductive_health_audit_logging: true,
    };

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_backoff_ms, 200);
    assert_eq!(config.max_backoff_ms, 10000);
    assert!(!config.enable_parallel_processing);
    assert_eq!(config.chunk_size, 2000);
    assert_eq!(config.memory_limit_mb, 1000.0);
}

#[test]
fn test_processing_status_debug() {
    use self_sensored::services::batch_processor::ProcessingStatus;

    // Test Debug trait
    let status = ProcessingStatus::Pending;
    let debug_output = format!("{status:?}");
    assert!(debug_output.contains("Pending"));

    let status = ProcessingStatus::InProgress;
    let debug_output = format!("{status:?}");
    assert!(debug_output.contains("InProgress"));
}
