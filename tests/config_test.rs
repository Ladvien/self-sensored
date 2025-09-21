use std::env;

use self_sensored::config::{BatchConfig, ValidationConfig};

#[cfg(test)]
mod batch_config_tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.heart_rate_chunk_size, 4200);
        assert_eq!(config.blood_pressure_chunk_size, 8000);
        assert_eq!(config.sleep_chunk_size, 5200);
        assert_eq!(config.activity_chunk_size, 2700);
        assert_eq!(config.workout_chunk_size, 5000);
        assert!(config.enable_parallel_processing);
        assert!(config.enable_progress_tracking);
        assert!(config.enable_intra_batch_deduplication);
    }

    #[test]
    fn test_batch_config_from_env() {
        // Set environment variables
        env::set_var("BATCH_MAX_RETRIES", "5");
        env::set_var("BATCH_HEART_RATE_CHUNK_SIZE", "9000");
        env::set_var("BATCH_ENABLE_PARALLEL", "false");
        env::set_var("BATCH_INITIAL_BACKOFF_MS", "200");
        env::set_var("BATCH_MAX_BACKOFF_MS", "10000");
        env::set_var("BATCH_MEMORY_LIMIT_MB", "1000.0");

        let config = BatchConfig::from_env();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.heart_rate_chunk_size, 9000);
        assert!(!config.enable_parallel_processing);

        // Clean up
        env::remove_var("BATCH_MAX_RETRIES");
        env::remove_var("BATCH_HEART_RATE_CHUNK_SIZE");
        env::remove_var("BATCH_ENABLE_PARALLEL");
        env::remove_var("BATCH_INITIAL_BACKOFF_MS");
        env::remove_var("BATCH_MAX_BACKOFF_MS");
        env::remove_var("BATCH_MEMORY_LIMIT_MB");
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
    fn test_batch_config_invalid_retries() {
        let mut config = BatchConfig::default();
        config.max_retries = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_batch_config_invalid_memory_limit() {
        let mut config = BatchConfig::default();
        config.memory_limit_mb = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_batch_config_all_chunk_sizes_validation() {
        let mut config = BatchConfig::default();

        // Test all chunk sizes with invalid values
        config.heart_rate_chunk_size = 0;
        assert!(config.validate().is_err());

        config = BatchConfig::default();
        config.blood_pressure_chunk_size = 100000;
        assert!(config.validate().is_err());

        config = BatchConfig::default();
        config.sleep_chunk_size = 0;
        assert!(config.validate().is_err());

        config = BatchConfig::default();
        config.activity_chunk_size = 100000;
        assert!(config.validate().is_err());

        config = BatchConfig::default();
        config.workout_chunk_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_batch_config_env_parsing_edge_cases() {
        // Test invalid boolean parsing
        env::set_var("BATCH_ENABLE_PARALLEL", "invalid_bool");
        let config = BatchConfig::from_env();
        // Should fall back to default when parsing fails
        assert!(config.enable_parallel_processing); // Default value
        env::remove_var("BATCH_ENABLE_PARALLEL");

        // Test invalid number parsing
        env::set_var("BATCH_MAX_RETRIES", "not_a_number");
        let config = BatchConfig::from_env();
        assert_eq!(config.max_retries, 3); // Default value
        env::remove_var("BATCH_MAX_RETRIES");
    }
}

#[cfg(test)]
mod validation_config_tests {
    use super::*;

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
        assert_eq!(config.latitude_min, -90.0);
        assert_eq!(config.latitude_max, 90.0);
        assert_eq!(config.longitude_min, -180.0);
        assert_eq!(config.longitude_max, 180.0);
    }

    #[test]
    fn test_validation_config_from_env() {
        // Set comprehensive environment variables
        env::set_var("VALIDATION_HEART_RATE_MIN", "20");
        env::set_var("VALIDATION_HEART_RATE_MAX", "250");
        env::set_var("VALIDATION_SYSTOLIC_MAX", "200");
        env::set_var("VALIDATION_BLOOD_GLUCOSE_MIN", "40.0");
        env::set_var("VALIDATION_BLOOD_GLUCOSE_MAX", "600.0");
        env::set_var("VALIDATION_RESPIRATORY_RATE_MIN", "8");
        env::set_var("VALIDATION_RESPIRATORY_RATE_MAX", "40");
        env::set_var("VALIDATION_OXYGEN_SATURATION_MIN", "85.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.heart_rate_min, 20);
        assert_eq!(config.heart_rate_max, 250);
        assert_eq!(config.systolic_max, 200);

        // Clean up
        env::remove_var("VALIDATION_HEART_RATE_MIN");
        env::remove_var("VALIDATION_HEART_RATE_MAX");
        env::remove_var("VALIDATION_SYSTOLIC_MAX");
        env::remove_var("VALIDATION_BLOOD_GLUCOSE_MIN");
        env::remove_var("VALIDATION_BLOOD_GLUCOSE_MAX");
        env::remove_var("VALIDATION_RESPIRATORY_RATE_MIN");
        env::remove_var("VALIDATION_RESPIRATORY_RATE_MAX");
        env::remove_var("VALIDATION_OXYGEN_SATURATION_MIN");
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

    #[test]
    fn test_validation_config_blood_pressure_validation() {
        let mut config = ValidationConfig::default();

        // Test systolic pressure validation
        config.systolic_min = 300;
        config.systolic_max = 200;
        assert!(config.validate().is_err());

        // Test diastolic pressure validation
        config = ValidationConfig::default();
        config.diastolic_min = 200;
        config.diastolic_max = 100;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_sleep_validation() {
        let mut config = ValidationConfig::default();

        // Test sleep efficiency validation
        config.sleep_efficiency_min = 150.0;
        config.sleep_efficiency_max = 100.0;
        assert!(config.validate().is_err());

        // Test negative sleep efficiency
        config = ValidationConfig::default();
        config.sleep_efficiency_min = -10.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_activity_validation() {
        let mut config = ValidationConfig::default();

        // Test step count validation
        config.step_count_min = 1000;
        config.step_count_max = 500;
        assert!(config.validate().is_err());

        // Test negative step count
        config = ValidationConfig::default();
        config.step_count_min = -100;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_gps_validation() {
        let mut config = ValidationConfig::default();

        // Test latitude bounds
        config.latitude_min = 100.0;
        assert!(config.validate().is_err());

        config = ValidationConfig::default();
        config.latitude_max = -100.0;
        assert!(config.validate().is_err());

        // Test longitude bounds
        config = ValidationConfig::default();
        config.longitude_min = 200.0;
        assert!(config.validate().is_err());

        config = ValidationConfig::default();
        config.longitude_max = -200.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_blood_glucose_validation() {
        let mut config = ValidationConfig::default();

        // Test blood glucose validation
        config.blood_glucose_min = 700.0;
        config.blood_glucose_max = 600.0;
        assert!(config.validate().is_err());

        // Test negative blood glucose
        config = ValidationConfig::default();
        config.blood_glucose_min = -10.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_respiratory_validation() {
        let mut config = ValidationConfig::default();

        // Test respiratory rate validation
        config.respiratory_rate_min = 50;
        config.respiratory_rate_max = 40;
        assert!(config.validate().is_err());

        // Test oxygen saturation validation
        config = ValidationConfig::default();
        config.oxygen_saturation_min = 110.0;
        assert!(config.validate().is_err());

        config = ValidationConfig::default();
        config.oxygen_saturation_max = 110.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_workout_validation() {
        let mut config = ValidationConfig::default();

        // Test workout heart rate validation
        config.workout_heart_rate_min = 400;
        config.workout_heart_rate_max = 300;
        assert!(config.validate().is_err());

        // Test negative workout duration
        config = ValidationConfig::default();
        config.workout_max_duration_hours = -1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_edge_cases() {
        let mut config = ValidationConfig::default();

        // Test zero values where appropriate
        config.step_count_min = 0;
        config.calories_max = 0.0;
        config.distance_max_km = 0.0;
        assert!(config.validate().is_ok());

        // Test critical oxygen saturation threshold
        config = ValidationConfig::default();
        config.oxygen_saturation_critical = 110.0; // Invalid critical threshold
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_config_env_parsing_edge_cases() {
        // Test invalid number parsing for validation config
        env::set_var("VALIDATION_HEART_RATE_MIN", "not_a_number");
        let config = ValidationConfig::from_env();
        assert_eq!(config.heart_rate_min, 15); // Default value
        env::remove_var("VALIDATION_HEART_RATE_MIN");

        // Test invalid float parsing
        env::set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "invalid_float");
        let config = ValidationConfig::from_env();
        assert_eq!(config.sleep_efficiency_min, 0.0); // Default value
        env::remove_var("VALIDATION_SLEEP_EFFICIENCY_MIN");
    }
}
