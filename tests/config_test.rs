use std::env;

use self_sensored::config::{BatchConfig, ValidationConfig};

#[test]
fn test_batch_config_default() {
    let config = BatchConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.heart_rate_chunk_size, 8000);
    assert_eq!(config.blood_pressure_chunk_size, 8000);
    assert_eq!(config.sleep_chunk_size, 5000);
    assert_eq!(config.activity_chunk_size, 7000);
    assert_eq!(config.workout_chunk_size, 5000);
    assert!(config.enable_parallel_processing);
    assert!(config.enable_progress_tracking);
    assert!(config.enable_intra_batch_deduplication);
}

#[test]
fn test_batch_config_from_env() {
    // Set some environment variables
    env::set_var("BATCH_MAX_RETRIES", "5");
    env::set_var("BATCH_HEART_RATE_CHUNK_SIZE", "9000");
    env::set_var("BATCH_ENABLE_PARALLEL", "false");
    
    let config = BatchConfig::from_env();
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.heart_rate_chunk_size, 9000);
    assert!(!config.enable_parallel_processing);
    
    // Clean up
    env::remove_var("BATCH_MAX_RETRIES");
    env::remove_var("BATCH_HEART_RATE_CHUNK_SIZE");
    env::remove_var("BATCH_ENABLE_PARALLEL");
}

#[test]
fn test_batch_config_validation() {
    let mut config = BatchConfig::default();
    // Valid configuration should pass
    assert!(config.validate().is_ok());
    
    // Invalid configuration should fail (too large chunk size)
    config.heart_rate_chunk_size = 20000; // This would exceed PostgreSQL limit
    assert!(config.validate().is_err());
    
    let error_message = config.validate().unwrap_err();
    assert!(error_message.contains("exceeding safe limit"));
}

#[test]
fn test_validation_config_default() {
    let config = ValidationConfig::default();
    assert_eq!(config.heart_rate_min, 15);
    assert_eq!(config.heart_rate_max, 300);
    assert_eq!(config.systolic_min, 50);
    assert_eq!(config.systolic_max, 250);
    assert_eq!(config.diastolic_min, 30);
    assert_eq!(config.diastolic_max, 150);
    assert_eq!(config.sleep_efficiency_min, 0.0);
    assert_eq!(config.sleep_efficiency_max, 100.0);
}

#[test]
fn test_validation_config_from_env() {
    // Set some environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "20");
    env::set_var("VALIDATION_HEART_RATE_MAX", "250");
    env::set_var("VALIDATION_SYSTOLIC_MAX", "200");
    
    let config = ValidationConfig::from_env();
    assert_eq!(config.heart_rate_min, 20);
    assert_eq!(config.heart_rate_max, 250);
    assert_eq!(config.systolic_max, 200);
    
    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_HEART_RATE_MAX");
    env::remove_var("VALIDATION_SYSTOLIC_MAX");
}

#[test]
fn test_validation_config_validation() {
    let mut config = ValidationConfig::default();
    // Valid configuration should pass
    assert!(config.validate().is_ok());
    
    // Invalid configuration should fail (min >= max)
    config.heart_rate_min = 300;
    config.heart_rate_max = 200;
    assert!(config.validate().is_err());
    
    let error_message = config.validate().unwrap_err();
    assert!(error_message.contains("heart_rate_min must be less than heart_rate_max"));
}