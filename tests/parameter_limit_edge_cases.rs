/// Integration tests for PostgreSQL parameter limit edge cases
/// STORY-CRITICAL-002: Activity Metrics PostgreSQL Parameter Limit Exceeded
///
/// These tests verify that our batch processing never exceeds PostgreSQL's
/// 65,535 parameter limit, which would cause silent data loss.

#[cfg(test)]
mod parameter_limit_tests {
    use self_sensored::config::batch_config::{BatchConfig, SAFE_PARAM_LIMIT, ACTIVITY_PARAMS_PER_RECORD};

    #[test]
    fn test_activity_chunk_size_fix_validation() {
        // Test the critical fix for STORY-CRITICAL-002
        let fixed_config = BatchConfig {
            activity_chunk_size: 2700, // Fixed chunk size
            ..BatchConfig::default()
        };

        let result = fixed_config.validate();
        assert!(result.is_ok(), "Fixed activity chunk size should pass validation");

        // Verify the calculation manually
        let total_params = 2700 * ACTIVITY_PARAMS_PER_RECORD;
        assert!(total_params <= SAFE_PARAM_LIMIT,
            "Activity: 2700 * {} = {} should be <= {}",
            ACTIVITY_PARAMS_PER_RECORD, total_params, SAFE_PARAM_LIMIT);
    }

    #[test]
    fn test_dangerous_activity_configuration_fails() {
        // Test that the old dangerous configuration correctly fails
        let dangerous_config = BatchConfig {
            activity_chunk_size: 7000, // Old dangerous chunk size
            ..BatchConfig::default()
        };

        let result = dangerous_config.validate();
        assert!(result.is_err(), "Dangerous activity chunk size should fail validation");

        let error_message = result.unwrap_err();
        assert!(error_message.contains("activity"), "Error should mention activity metric");
        assert!(error_message.contains("CRITICAL"), "Error should be marked as critical");
        assert!(error_message.contains("SILENT DATA LOSS"), "Error should warn about data loss");
    }

    #[test]
    fn test_all_default_chunk_sizes_are_safe() {
        let config = BatchConfig::default();
        let result = config.validate();

        assert!(result.is_ok(), "Default configuration should be safe: {}",
            result.unwrap_err());
    }

    #[test]
    fn test_parameter_limit_edge_cases() {
        use self_sensored::config::batch_config::*;

        // Test each metric type at the exact safe limit
        let edge_cases = vec![
            ("heart_rate", SAFE_PARAM_LIMIT / HEART_RATE_PARAMS_PER_RECORD, HEART_RATE_PARAMS_PER_RECORD),
            ("blood_pressure", SAFE_PARAM_LIMIT / BLOOD_PRESSURE_PARAMS_PER_RECORD, BLOOD_PRESSURE_PARAMS_PER_RECORD),
            ("sleep", SAFE_PARAM_LIMIT / SLEEP_PARAMS_PER_RECORD, SLEEP_PARAMS_PER_RECORD),
            ("activity", SAFE_PARAM_LIMIT / ACTIVITY_PARAMS_PER_RECORD, ACTIVITY_PARAMS_PER_RECORD),
            ("temperature", SAFE_PARAM_LIMIT / TEMPERATURE_PARAMS_PER_RECORD, TEMPERATURE_PARAMS_PER_RECORD),
        ];

        for (metric_name, max_safe_chunk, params_per_record) in edge_cases {
            let total_params = max_safe_chunk * params_per_record;

            // Should be at or just under the safe limit
            assert!(total_params <= SAFE_PARAM_LIMIT,
                "{}: max chunk {} * {} params = {} should be <= {}",
                metric_name, max_safe_chunk, params_per_record, total_params, SAFE_PARAM_LIMIT);

            // Test that one more record would exceed the limit
            let over_limit_params = (max_safe_chunk + 1) * params_per_record;
            assert!(over_limit_params > SAFE_PARAM_LIMIT,
                "{}: {} + 1 chunk should exceed safe limit",
                metric_name, max_safe_chunk);
        }
    }

    #[test]
    fn test_multiple_metric_violations() {
        // Test configuration with multiple violations
        let multi_violation_config = BatchConfig {
            activity_chunk_size: 7000,    // 133,000 params (way over)
            sleep_chunk_size: 6000,       // 60,000 params (over safe limit)
            temperature_chunk_size: 8000, // 64,000 params (over safe limit)
            ..BatchConfig::default()
        };

        let result = multi_violation_config.validate();
        assert!(result.is_err(), "Multi-violation config should fail");

        let error_message = result.unwrap_err();
        assert!(error_message.contains("activity"), "Should mention activity violation");
        assert!(error_message.contains("sleep"), "Should mention sleep violation");
        assert!(error_message.contains("temperature"), "Should mention temperature violation");
    }

    #[test]
    fn test_exact_postgresql_limit() {
        // Test the absolute PostgreSQL limit (65,535 parameters)
        const POSTGRESQL_MAX_PARAMS: usize = 65535;

        // Activity metrics have the most parameters per record (19)
        // So they hit the limit fastest
        let dangerous_activity_chunk = POSTGRESQL_MAX_PARAMS / ACTIVITY_PARAMS_PER_RECORD;
        let dangerous_config = BatchConfig {
            activity_chunk_size: dangerous_activity_chunk,
            ..BatchConfig::default()
        };

        // This should still fail because we use a safety margin
        let result = dangerous_config.validate();
        assert!(result.is_err(),
            "Configuration at PostgreSQL limit should fail due to safety margin");
    }

    #[test]
    fn test_environment_variable_safety() {
        // Test that environment variables can't override safety
        std::env::set_var("BATCH_ACTIVITY_CHUNK_SIZE", "7000"); // Dangerous value

        let config = BatchConfig::from_env();
        let result = config.validate();

        // Even if env var sets dangerous value, validation should catch it
        assert!(result.is_err(),
            "Environment variables with dangerous values should fail validation");

        std::env::remove_var("BATCH_ACTIVITY_CHUNK_SIZE");
    }

    #[test]
    fn test_activity_metric_parameter_count() {
        // Verify our parameter count is correct for activity metrics
        // Based on database schema: user_id, recorded_at, step_count, distance_meters,
        // active_energy_burned_kcal, basal_energy_burned_kcal, flights_climbed,
        // distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters,
        // distance_downhill_snow_sports_meters, push_count, swimming_stroke_count,
        // nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes,
        // apple_move_time_minutes, apple_stand_hour_achieved, source_device

        assert_eq!(ACTIVITY_PARAMS_PER_RECORD, 19,
            "Activity parameter count should match database schema");
    }
}