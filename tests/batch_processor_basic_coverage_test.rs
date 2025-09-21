use self_sensored::config::BatchConfig;
use self_sensored::services::batch_processor::*;

#[test]
fn test_batch_config_creation() {
    let config = BatchConfig::default();

    // Test that default configuration is valid
    assert!(config.heart_rate_chunk_size > 0);
    assert!(config.blood_pressure_chunk_size > 0);
    assert!(config.sleep_chunk_size > 0);
    assert!(config.activity_chunk_size > 0);
    assert!(config.workout_chunk_size > 0);
}

#[test]
fn test_batch_processing_result() {
    let mut result = BatchProcessingResult::default();

    result.processed_count = 100;
    result.failed_count = 5;
    result.skipped_count = 2;

    assert_eq!(result.processed_count, 100);
    assert_eq!(result.failed_count, 5);
    assert_eq!(result.skipped_count, 2);

    let total_attempted = result.processed_count + result.failed_count + result.skipped_count;
    assert_eq!(total_attempted, 107);
}

#[test]
fn test_deduplication_stats() {
    let mut stats = DeduplicationStats::default();

    stats.total_records = 1000;
    stats.duplicates_found = 50;
    stats.duplicates_removed = 45;

    assert_eq!(stats.total_records, 1000);
    assert_eq!(stats.duplicates_found, 50);
    assert_eq!(stats.duplicates_removed, 45);

    let unique_records = stats.total_records - stats.duplicates_removed;
    assert_eq!(unique_records, 955);
}

#[test]
fn test_chunk_size_calculations() {
    // Test that chunk sizes are within PostgreSQL parameter limits
    let config = BatchConfig::default();

    // PostgreSQL has a limit of 65,535 parameters per query
    let max_params = 65_535;

    // Heart rate has 7 parameters per record
    let heart_rate_max_safe = max_params / 7;
    assert!(config.heart_rate_chunk_size <= heart_rate_max_safe);

    // Blood pressure has 6 parameters per record
    let blood_pressure_max_safe = max_params / 6;
    assert!(config.blood_pressure_chunk_size <= blood_pressure_max_safe);

    // Sleep has 9 parameters per record
    let sleep_max_safe = max_params / 9;
    assert!(config.sleep_chunk_size <= sleep_max_safe);

    // Activity has 19 parameters per record
    let activity_max_safe = max_params / 19;
    assert!(config.activity_chunk_size <= activity_max_safe);

    // Workout has 10 parameters per record
    let workout_max_safe = max_params / 10;
    assert!(config.workout_chunk_size <= workout_max_safe);
}

#[test]
fn test_batch_processing_error_handling() {
    let mut result = BatchProcessingResult::default();

    // Simulate processing with errors
    result.processed_count = 95;
    result.failed_count = 5;
    result.errors.push("Database connection failed".to_string());
    result.errors.push("Invalid heart rate value".to_string());

    assert_eq!(result.errors.len(), 2);
    assert!(result.errors.contains(&"Database connection failed".to_string()));
    assert!(result.errors.contains(&"Invalid heart rate value".to_string()));

    // Calculate success rate
    let total_attempted = result.processed_count + result.failed_count;
    let success_rate = if total_attempted > 0 {
        (result.processed_count as f64 / total_attempted as f64) * 100.0
    } else {
        0.0
    };

    assert_eq!(success_rate, 95.0);
}

#[test]
fn test_performance_tracking() {
    let start_time = std::time::Instant::now();

    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(1));

    let elapsed = start_time.elapsed();
    assert!(elapsed.as_millis() >= 1);

    // Test timing tracking for batch processing
    let mut result = BatchProcessingResult::default();
    result.processing_time_ms = elapsed.as_millis() as u64;

    assert!(result.processing_time_ms > 0);
}

#[test]
fn test_memory_usage_tracking() {
    let config = BatchConfig::default();

    // Test memory limit configuration
    assert!(config.memory_limit_mb > 0.0);

    // Simulate memory usage calculation
    let estimated_memory_per_record = 1024; // bytes
    let chunk_size = 1000;
    let estimated_memory_mb = (estimated_memory_per_record * chunk_size) as f64 / (1024.0 * 1024.0);

    assert!(estimated_memory_mb < config.memory_limit_mb);
}