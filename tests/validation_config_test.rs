use std::env;

use self_sensored::config::ValidationConfig;

/// Test fixture for setting up isolated environment variable tests
struct EnvTestFixture {
    vars_to_clean: Vec<String>,
}

impl EnvTestFixture {
    fn new() -> Self {
        Self {
            vars_to_clean: Vec::new(),
        }
    }

    fn set_var(&mut self, key: &str, value: &str) {
        env::set_var(key, value);
        self.vars_to_clean.push(key.to_string());
    }
}

impl Drop for EnvTestFixture {
    fn drop(&mut self) {
        for var in &self.vars_to_clean {
            env::remove_var(var);
        }
    }
}

#[cfg(test)]
mod validation_config_comprehensive_tests {
    use super::*;

    // ========================================
    // Default Configuration Tests
    // ========================================

    #[test]
    fn test_default_heart_rate_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.heart_rate_min, 15);
        assert_eq!(config.heart_rate_max, 300);
        assert_eq!(config.workout_heart_rate_min, 15);
        assert_eq!(config.workout_heart_rate_max, 300);
    }

    #[test]
    fn test_default_blood_pressure_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.systolic_min, 50);
        assert_eq!(config.systolic_max, 250);
        assert_eq!(config.diastolic_min, 30);
        assert_eq!(config.diastolic_max, 150);
    }

    #[test]
    fn test_default_sleep_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.sleep_efficiency_min, 0.0);
        assert_eq!(config.sleep_efficiency_max, 100.0);
        assert_eq!(config.sleep_duration_tolerance_minutes, 60);
    }

    #[test]
    fn test_default_activity_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.step_count_min, 0);
        assert_eq!(config.step_count_max, 200_000);
        assert_eq!(config.distance_max_km, 500.0);
        assert_eq!(config.calories_max, 20_000.0);
    }

    #[test]
    fn test_default_gps_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.latitude_min, -90.0);
        assert_eq!(config.latitude_max, 90.0);
        assert_eq!(config.longitude_min, -180.0);
        assert_eq!(config.longitude_max, 180.0);
    }

    #[test]
    fn test_default_workout_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.workout_max_duration_hours, 24);
    }

    #[test]
    fn test_default_blood_glucose_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.blood_glucose_min, 30.0);
        assert_eq!(config.blood_glucose_max, 600.0);
        assert_eq!(config.insulin_max_units, 100.0);
    }

    #[test]
    fn test_default_respiratory_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.respiratory_rate_min, 5);
        assert_eq!(config.respiratory_rate_max, 60);
        assert_eq!(config.oxygen_saturation_min, 70.0);
        assert_eq!(config.oxygen_saturation_max, 100.0);
        assert_eq!(config.oxygen_saturation_critical, 90.0);
        assert_eq!(config.forced_vital_capacity_min, 1.0);
        assert_eq!(config.forced_vital_capacity_max, 8.0);
        assert_eq!(config.forced_expiratory_volume_1_min, 0.5);
        assert_eq!(config.forced_expiratory_volume_1_max, 6.0);
        assert_eq!(config.peak_expiratory_flow_rate_min, 50.0);
        assert_eq!(config.peak_expiratory_flow_rate_max, 800.0);
        assert_eq!(config.inhaler_usage_max, 50);
    }

    #[test]
    fn test_default_temperature_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.body_temperature_min, 30.0);
        assert_eq!(config.body_temperature_max, 45.0);
        assert_eq!(config.basal_body_temperature_min, 35.0);
        assert_eq!(config.basal_body_temperature_max, 39.0);
        assert_eq!(config.wrist_temperature_min, 30.0);
        assert_eq!(config.wrist_temperature_max, 45.0);
        assert_eq!(config.water_temperature_min, 0.0);
        assert_eq!(config.water_temperature_max, 100.0);
        assert_eq!(config.fever_threshold, 38.0);
    }

    #[test]
    fn test_default_body_measurements_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.body_weight_min_kg, 20.0);
        assert_eq!(config.body_weight_max_kg, 500.0);
        assert_eq!(config.bmi_min, 15.0);
        assert_eq!(config.bmi_max, 50.0);
        assert_eq!(config.body_fat_min_percent, 3.0);
        assert_eq!(config.body_fat_max_percent, 50.0);
        assert_eq!(config.body_temperature_min_celsius, 30.0);
        assert_eq!(config.body_temperature_max_celsius, 45.0);
    }

    #[test]
    fn test_default_reproductive_health_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.menstrual_cycle_day_min, 1);
        assert_eq!(config.menstrual_cycle_day_max, 45);
        assert_eq!(config.menstrual_cramps_severity_min, 0);
        assert_eq!(config.menstrual_cramps_severity_max, 10);
        assert_eq!(config.menstrual_mood_rating_min, 1);
        assert_eq!(config.menstrual_mood_rating_max, 5);
        assert_eq!(config.menstrual_energy_level_min, 1);
        assert_eq!(config.menstrual_energy_level_max, 5);
    }

    #[test]
    fn test_default_fertility_tracking_configuration() {
        let config = ValidationConfig::default();
        assert_eq!(config.fertility_basal_temp_min, 35.0);
        assert_eq!(config.fertility_basal_temp_max, 39.0);
        assert_eq!(config.fertility_cervix_firmness_min, 1);
        assert_eq!(config.fertility_cervix_firmness_max, 3);
        assert_eq!(config.fertility_cervix_position_min, 1);
        assert_eq!(config.fertility_cervix_position_max, 3);
        assert_eq!(config.fertility_lh_level_min, 0.0);
        assert_eq!(config.fertility_lh_level_max, 100.0);
    }

    // ========================================
    // Environment Variable Loading Tests
    // ========================================

    #[test]
    fn test_heart_rate_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_HEART_RATE_MIN", "25");
        fixture.set_var("VALIDATION_HEART_RATE_MAX", "250");
        fixture.set_var("VALIDATION_WORKOUT_HEART_RATE_MIN", "30");
        fixture.set_var("VALIDATION_WORKOUT_HEART_RATE_MAX", "280");

        let config = ValidationConfig::from_env();
        assert_eq!(config.heart_rate_min, 25);
        assert_eq!(config.heart_rate_max, 250);
        assert_eq!(config.workout_heart_rate_min, 30);
        assert_eq!(config.workout_heart_rate_max, 280);
    }

    #[test]
    fn test_blood_pressure_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_SYSTOLIC_MIN", "60");
        fixture.set_var("VALIDATION_SYSTOLIC_MAX", "200");
        fixture.set_var("VALIDATION_DIASTOLIC_MIN", "40");
        fixture.set_var("VALIDATION_DIASTOLIC_MAX", "120");

        let config = ValidationConfig::from_env();
        assert_eq!(config.systolic_min, 60);
        assert_eq!(config.systolic_max, 200);
        assert_eq!(config.diastolic_min, 40);
        assert_eq!(config.diastolic_max, 120);
    }

    #[test]
    fn test_sleep_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "10.0");
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MAX", "95.0");
        fixture.set_var("VALIDATION_SLEEP_DURATION_TOLERANCE_MINUTES", "30");

        let config = ValidationConfig::from_env();
        assert_eq!(config.sleep_efficiency_min, 10.0);
        assert_eq!(config.sleep_efficiency_max, 95.0);
        assert_eq!(config.sleep_duration_tolerance_minutes, 30);
    }

    #[test]
    fn test_activity_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_STEP_COUNT_MIN", "100");
        fixture.set_var("VALIDATION_STEP_COUNT_MAX", "150000");
        fixture.set_var("VALIDATION_DISTANCE_MAX_KM", "300.0");
        fixture.set_var("VALIDATION_CALORIES_MAX", "15000.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.step_count_min, 100);
        assert_eq!(config.step_count_max, 150_000);
        assert_eq!(config.distance_max_km, 300.0);
        assert_eq!(config.calories_max, 15_000.0);
    }

    #[test]
    fn test_gps_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_LATITUDE_MIN", "-85.0");
        fixture.set_var("VALIDATION_LATITUDE_MAX", "85.0");
        fixture.set_var("VALIDATION_LONGITUDE_MIN", "-170.0");
        fixture.set_var("VALIDATION_LONGITUDE_MAX", "170.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.latitude_min, -85.0);
        assert_eq!(config.latitude_max, 85.0);
        assert_eq!(config.longitude_min, -170.0);
        assert_eq!(config.longitude_max, 170.0);
    }

    #[test]
    fn test_workout_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_WORKOUT_MAX_DURATION_HOURS", "12");

        let config = ValidationConfig::from_env();
        assert_eq!(config.workout_max_duration_hours, 12);
    }

    #[test]
    fn test_blood_glucose_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_BLOOD_GLUCOSE_MIN", "40.0");
        fixture.set_var("VALIDATION_BLOOD_GLUCOSE_MAX", "500.0");
        fixture.set_var("VALIDATION_INSULIN_MAX_UNITS", "80.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.blood_glucose_min, 40.0);
        assert_eq!(config.blood_glucose_max, 500.0);
        assert_eq!(config.insulin_max_units, 80.0);
    }

    #[test]
    fn test_respiratory_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_RESPIRATORY_RATE_MIN", "8");
        fixture.set_var("VALIDATION_RESPIRATORY_RATE_MAX", "40");
        fixture.set_var("VALIDATION_OXYGEN_SATURATION_MIN", "85.0");
        fixture.set_var("VALIDATION_OXYGEN_SATURATION_MAX", "99.0");
        fixture.set_var("VALIDATION_OXYGEN_SATURATION_CRITICAL", "88.0");
        fixture.set_var("VALIDATION_FORCED_VITAL_CAPACITY_MIN", "2.0");
        fixture.set_var("VALIDATION_FORCED_VITAL_CAPACITY_MAX", "6.0");
        fixture.set_var("VALIDATION_INHALER_USAGE_MAX", "20");

        let config = ValidationConfig::from_env();
        assert_eq!(config.respiratory_rate_min, 8);
        assert_eq!(config.respiratory_rate_max, 40);
        assert_eq!(config.oxygen_saturation_min, 85.0);
        assert_eq!(config.oxygen_saturation_max, 99.0);
        assert_eq!(config.oxygen_saturation_critical, 88.0);
        assert_eq!(config.forced_vital_capacity_min, 2.0);
        assert_eq!(config.forced_vital_capacity_max, 6.0);
        assert_eq!(config.inhaler_usage_max, 20);
    }

    #[test]
    fn test_temperature_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_BODY_TEMPERATURE_MIN", "32.0");
        fixture.set_var("VALIDATION_BODY_TEMPERATURE_MAX", "42.0");
        fixture.set_var("VALIDATION_BASAL_BODY_TEMPERATURE_MIN", "36.0");
        fixture.set_var("VALIDATION_BASAL_BODY_TEMPERATURE_MAX", "38.0");
        fixture.set_var("VALIDATION_WRIST_TEMPERATURE_MIN", "28.0");
        fixture.set_var("VALIDATION_WRIST_TEMPERATURE_MAX", "40.0");
        fixture.set_var("VALIDATION_WATER_TEMPERATURE_MIN", "5.0");
        fixture.set_var("VALIDATION_WATER_TEMPERATURE_MAX", "80.0");
        fixture.set_var("VALIDATION_FEVER_THRESHOLD", "37.5");

        let config = ValidationConfig::from_env();
        assert_eq!(config.body_temperature_min, 32.0);
        assert_eq!(config.body_temperature_max, 42.0);
        assert_eq!(config.basal_body_temperature_min, 36.0);
        assert_eq!(config.basal_body_temperature_max, 38.0);
        assert_eq!(config.wrist_temperature_min, 28.0);
        assert_eq!(config.wrist_temperature_max, 40.0);
        assert_eq!(config.water_temperature_min, 5.0);
        assert_eq!(config.water_temperature_max, 80.0);
        assert_eq!(config.fever_threshold, 37.5);
    }

    #[test]
    fn test_body_measurements_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_BODY_WEIGHT_MIN_KG", "30.0");
        fixture.set_var("VALIDATION_BODY_WEIGHT_MAX_KG", "300.0");
        fixture.set_var("VALIDATION_BMI_MIN", "12.0");
        fixture.set_var("VALIDATION_BMI_MAX", "40.0");
        fixture.set_var("VALIDATION_BODY_FAT_MIN_PERCENT", "5.0");
        fixture.set_var("VALIDATION_BODY_FAT_MAX_PERCENT", "40.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.body_weight_min_kg, 30.0);
        assert_eq!(config.body_weight_max_kg, 300.0);
        assert_eq!(config.bmi_min, 12.0);
        assert_eq!(config.bmi_max, 40.0);
        assert_eq!(config.body_fat_min_percent, 5.0);
        assert_eq!(config.body_fat_max_percent, 40.0);
    }

    #[test]
    fn test_reproductive_health_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_MENSTRUAL_CYCLE_DAY_MIN", "2");
        fixture.set_var("VALIDATION_MENSTRUAL_CYCLE_DAY_MAX", "35");
        fixture.set_var("VALIDATION_MENSTRUAL_CRAMPS_SEVERITY_MIN", "1");
        fixture.set_var("VALIDATION_MENSTRUAL_CRAMPS_SEVERITY_MAX", "8");
        fixture.set_var("VALIDATION_MENSTRUAL_MOOD_RATING_MIN", "2");
        fixture.set_var("VALIDATION_MENSTRUAL_MOOD_RATING_MAX", "4");
        fixture.set_var("VALIDATION_MENSTRUAL_ENERGY_LEVEL_MIN", "2");
        fixture.set_var("VALIDATION_MENSTRUAL_ENERGY_LEVEL_MAX", "4");

        let config = ValidationConfig::from_env();
        assert_eq!(config.menstrual_cycle_day_min, 2);
        assert_eq!(config.menstrual_cycle_day_max, 35);
        assert_eq!(config.menstrual_cramps_severity_min, 1);
        assert_eq!(config.menstrual_cramps_severity_max, 8);
        assert_eq!(config.menstrual_mood_rating_min, 2);
        assert_eq!(config.menstrual_mood_rating_max, 4);
        assert_eq!(config.menstrual_energy_level_min, 2);
        assert_eq!(config.menstrual_energy_level_max, 4);
    }

    #[test]
    fn test_fertility_tracking_env_loading() {
        let mut fixture = EnvTestFixture::new();
        fixture.set_var("VALIDATION_FERTILITY_BASAL_TEMP_MIN", "36.0");
        fixture.set_var("VALIDATION_FERTILITY_BASAL_TEMP_MAX", "38.0");
        fixture.set_var("VALIDATION_FERTILITY_CERVIX_FIRMNESS_MIN", "2");
        fixture.set_var("VALIDATION_FERTILITY_CERVIX_FIRMNESS_MAX", "2");
        fixture.set_var("VALIDATION_FERTILITY_CERVIX_POSITION_MIN", "2");
        fixture.set_var("VALIDATION_FERTILITY_CERVIX_POSITION_MAX", "2");
        fixture.set_var("VALIDATION_FERTILITY_LH_LEVEL_MIN", "5.0");
        fixture.set_var("VALIDATION_FERTILITY_LH_LEVEL_MAX", "80.0");

        let config = ValidationConfig::from_env();
        assert_eq!(config.fertility_basal_temp_min, 36.0);
        assert_eq!(config.fertility_basal_temp_max, 38.0);
        assert_eq!(config.fertility_cervix_firmness_min, 2);
        assert_eq!(config.fertility_cervix_firmness_max, 2);
        assert_eq!(config.fertility_cervix_position_min, 2);
        assert_eq!(config.fertility_cervix_position_max, 2);
        assert_eq!(config.fertility_lh_level_min, 5.0);
        assert_eq!(config.fertility_lh_level_max, 80.0);
    }

    // ========================================
    // Validation Logic Tests
    // ========================================

    #[test]
    fn test_valid_default_configuration() {
        let config = ValidationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_heart_rate_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test min >= max
        config.heart_rate_min = 300;
        config.heart_rate_max = 200;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("heart_rate_min must be less than heart_rate_max"));

        // Test workout heart rate validation
        config = ValidationConfig::default();
        config.workout_heart_rate_min = 350;
        config.workout_heart_rate_max = 300;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("workout_heart_rate_min must be less than workout_heart_rate_max"));
    }

    #[test]
    fn test_blood_pressure_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test systolic validation
        config.systolic_min = 300;
        config.systolic_max = 200;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("systolic_min must be less than systolic_max"));

        // Test diastolic validation
        config = ValidationConfig::default();
        config.diastolic_min = 200;
        config.diastolic_max = 100;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("diastolic_min must be less than diastolic_max"));
    }

    #[test]
    fn test_sleep_validation_errors() {
        let mut config = ValidationConfig::default();

        config.sleep_efficiency_min = 150.0;
        config.sleep_efficiency_max = 100.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sleep_efficiency_min must be less than sleep_efficiency_max"));
    }

    #[test]
    fn test_activity_validation_errors() {
        let mut config = ValidationConfig::default();

        config.step_count_min = 100_000;
        config.step_count_max = 50_000;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("step_count_min must be less than step_count_max"));
    }

    #[test]
    fn test_gps_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test latitude validation
        config.latitude_min = 100.0;
        config.latitude_max = 90.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("latitude_min must be less than latitude_max"));

        // Test longitude validation
        config = ValidationConfig::default();
        config.longitude_min = 200.0;
        config.longitude_max = 180.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("longitude_min must be less than longitude_max"));
    }

    #[test]
    fn test_temperature_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test body temperature validation
        config.body_temperature_min = 50.0;
        config.body_temperature_max = 40.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("body_temperature_min must be less than body_temperature_max"));

        // Test basal body temperature validation
        config = ValidationConfig::default();
        config.basal_body_temperature_min = 40.0;
        config.basal_body_temperature_max = 35.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("basal_body_temperature_min must be less than basal_body_temperature_max"));

        // Test wrist temperature validation
        config = ValidationConfig::default();
        config.wrist_temperature_min = 50.0;
        config.wrist_temperature_max = 40.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrist_temperature_min must be less than wrist_temperature_max"));

        // Test water temperature validation
        config = ValidationConfig::default();
        config.water_temperature_min = 120.0;
        config.water_temperature_max = 100.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("water_temperature_min must be less than water_temperature_max"));
    }

    #[test]
    fn test_body_measurements_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test body weight validation
        config.body_weight_min_kg = 600.0;
        config.body_weight_max_kg = 500.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("body_weight_min_kg must be less than body_weight_max_kg"));

        // Test BMI validation
        config = ValidationConfig::default();
        config.bmi_min = 60.0;
        config.bmi_max = 50.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("bmi_min must be less than bmi_max"));

        // Test body fat validation
        config = ValidationConfig::default();
        config.body_fat_min_percent = 60.0;
        config.body_fat_max_percent = 50.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("body_fat_min_percent must be less than body_fat_max_percent"));

        // Test body temperature celsius validation
        config = ValidationConfig::default();
        config.body_temperature_min_celsius = 50.0;
        config.body_temperature_max_celsius = 45.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("body_temperature_min_celsius must be less than body_temperature_max_celsius"));
    }

    #[test]
    fn test_blood_glucose_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test blood glucose validation
        config.blood_glucose_min = 700.0;
        config.blood_glucose_max = 600.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blood_glucose_min must be less than blood_glucose_max"));

        // Test negative insulin validation
        config = ValidationConfig::default();
        config.insulin_max_units = -10.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("insulin_max_units must be positive"));
    }

    #[test]
    fn test_reproductive_health_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test menstrual cycle day validation
        config.menstrual_cycle_day_min = 50;
        config.menstrual_cycle_day_max = 45;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("menstrual_cycle_day_min must be less than menstrual_cycle_day_max"));

        // Test menstrual cramps severity validation
        config = ValidationConfig::default();
        config.menstrual_cramps_severity_min = 15;
        config.menstrual_cramps_severity_max = 10;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("menstrual_cramps_severity_min must be less than or equal to menstrual_cramps_severity_max"));

        // Test menstrual mood rating validation
        config = ValidationConfig::default();
        config.menstrual_mood_rating_min = 8;
        config.menstrual_mood_rating_max = 5;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("menstrual_mood_rating_min must be less than or equal to menstrual_mood_rating_max"));

        // Test menstrual energy level validation
        config = ValidationConfig::default();
        config.menstrual_energy_level_min = 8;
        config.menstrual_energy_level_max = 5;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("menstrual_energy_level_min must be less than or equal to menstrual_energy_level_max"));
    }

    #[test]
    fn test_fertility_tracking_validation_errors() {
        let mut config = ValidationConfig::default();

        // Test fertility basal temp validation
        config.fertility_basal_temp_min = 40.0;
        config.fertility_basal_temp_max = 39.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fertility_basal_temp_min must be less than fertility_basal_temp_max"));

        // Test fertility cervix firmness validation
        config = ValidationConfig::default();
        config.fertility_cervix_firmness_min = 5;
        config.fertility_cervix_firmness_max = 3;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fertility_cervix_firmness_min must be less than or equal to fertility_cervix_firmness_max"));

        // Test fertility cervix position validation
        config = ValidationConfig::default();
        config.fertility_cervix_position_min = 5;
        config.fertility_cervix_position_max = 3;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fertility_cervix_position_min must be less than or equal to fertility_cervix_position_max"));

        // Test fertility LH level validation
        config = ValidationConfig::default();
        config.fertility_lh_level_min = 150.0;
        config.fertility_lh_level_max = 100.0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fertility_lh_level_min must be less than fertility_lh_level_max"));
    }

    // ========================================
    // Environment Variable Parsing Edge Cases
    // ========================================

    #[test]
    fn test_invalid_env_var_parsing_fallback_to_defaults() {
        let mut fixture = EnvTestFixture::new();

        // Test invalid integer parsing
        fixture.set_var("VALIDATION_HEART_RATE_MIN", "not_a_number");
        fixture.set_var("VALIDATION_HEART_RATE_MAX", "also_not_a_number");

        // Test invalid float parsing
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "invalid_float");
        fixture.set_var("VALIDATION_BLOOD_GLUCOSE_MAX", "not_a_float");

        let config = ValidationConfig::from_env();

        // Should fall back to defaults
        assert_eq!(config.heart_rate_min, 15); // Default
        assert_eq!(config.heart_rate_max, 300); // Default
        assert_eq!(config.sleep_efficiency_min, 0.0); // Default
        assert_eq!(config.blood_glucose_max, 600.0); // Default
    }

    #[test]
    fn test_empty_env_var_parsing() {
        let mut fixture = EnvTestFixture::new();

        // Test empty string values
        fixture.set_var("VALIDATION_HEART_RATE_MIN", "");
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MAX", "");
        fixture.set_var("VALIDATION_STEP_COUNT_MAX", "");

        let config = ValidationConfig::from_env();

        // Should fall back to defaults
        assert_eq!(config.heart_rate_min, 15); // Default
        assert_eq!(config.sleep_efficiency_max, 100.0); // Default
        assert_eq!(config.step_count_max, 200_000); // Default
    }

    #[test]
    fn test_boundary_values_env_parsing() {
        let mut fixture = EnvTestFixture::new();

        // Test boundary values
        fixture.set_var("VALIDATION_HEART_RATE_MIN", "0");
        fixture.set_var("VALIDATION_HEART_RATE_MAX", "1000");
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "-100.0");
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MAX", "200.0");

        let config = ValidationConfig::from_env();

        // Should parse these extreme values (validation logic handles reasonableness)
        assert_eq!(config.heart_rate_min, 0);
        assert_eq!(config.heart_rate_max, 1000);
        assert_eq!(config.sleep_efficiency_min, -100.0);
        assert_eq!(config.sleep_efficiency_max, 200.0);

        // The validation should pass because these are technically valid ranges (min < max)
        // The validation logic currently only checks that min < max, not reasonableness
        assert!(config.validate().is_ok());

        // Test actual invalid ranges that should fail validation
        let mut invalid_config = ValidationConfig::default();
        invalid_config.heart_rate_min = 1000;
        invalid_config.heart_rate_max = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_partial_env_configuration() {
        let mut fixture = EnvTestFixture::new();

        // Set only some environment variables
        fixture.set_var("VALIDATION_HEART_RATE_MIN", "20");
        fixture.set_var("VALIDATION_SYSTOLIC_MAX", "220");
        fixture.set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "5.0");

        let config = ValidationConfig::from_env();

        // Modified values
        assert_eq!(config.heart_rate_min, 20);
        assert_eq!(config.systolic_max, 220);
        assert_eq!(config.sleep_efficiency_min, 5.0);

        // Default values for unset variables
        assert_eq!(config.heart_rate_max, 300); // Default
        assert_eq!(config.systolic_min, 50); // Default
        assert_eq!(config.sleep_efficiency_max, 100.0); // Default
    }

    // ========================================
    // Configuration Consistency Tests
    // ========================================

    #[test]
    fn test_medical_grade_thresholds_are_reasonable() {
        let config = ValidationConfig::default();

        // Heart rate thresholds should cover medical emergency scenarios
        assert!(config.heart_rate_min <= 20); // Severe bradycardia
        assert!(config.heart_rate_max >= 250); // Severe tachycardia

        // Blood pressure thresholds should cover hypotensive and hypertensive crisis
        assert!(config.systolic_min <= 60); // Severe hypotension
        assert!(config.systolic_max >= 220); // Hypertensive crisis
        assert!(config.diastolic_min <= 40); // Severe hypotension
        assert!(config.diastolic_max >= 120); // Hypertensive crisis

        // Blood glucose should cover diabetic emergencies
        assert!(config.blood_glucose_min <= 40.0); // Severe hypoglycemia
        assert!(config.blood_glucose_max >= 500.0); // Diabetic ketoacidosis

        // Oxygen saturation should cover critical thresholds
        assert!(config.oxygen_saturation_min <= 80.0); // Severe hypoxemia
        assert!(config.oxygen_saturation_critical >= 88.0); // Clinical threshold
    }

    #[test]
    fn test_athletic_performance_thresholds() {
        let config = ValidationConfig::default();

        // Should accommodate elite athletic performance
        assert!(config.heart_rate_max >= 250); // Elite athlete max
        assert!(config.step_count_max >= 100_000); // Ultra-marathon distances
        assert!(config.distance_max_km >= 200.0); // Ultra-marathon distances
        assert!(config.calories_max >= 10_000.0); // Extreme endurance events
        assert!(config.workout_max_duration_hours >= 12); // Ultra events
    }

    #[test]
    fn test_reproductive_health_medical_compliance() {
        let config = ValidationConfig::default();

        // Menstrual cycle tracking should accommodate irregular cycles
        assert!(config.menstrual_cycle_day_min <= 1); // Start of cycle
        assert!(config.menstrual_cycle_day_max >= 35); // Irregular cycles

        // Pain scales should follow medical standards (0-10)
        assert_eq!(config.menstrual_cramps_severity_min, 0);
        assert_eq!(config.menstrual_cramps_severity_max, 10);

        // Fertility tracking should cover normal ranges
        assert!(config.fertility_basal_temp_min <= 36.0);
        assert!(config.fertility_basal_temp_max >= 38.0);
        assert!(config.fertility_lh_level_max >= 50.0); // LH surge levels
    }

    #[test]
    fn test_temperature_thresholds_safety() {
        let config = ValidationConfig::default();

        // Body temperature should cover hypothermia and hyperthermia
        assert!(config.body_temperature_min <= 32.0); // Severe hypothermia
        assert!(config.body_temperature_max >= 42.0); // Severe hyperthermia

        // Fever threshold should be medically accurate
        assert!(config.fever_threshold >= 37.5 && config.fever_threshold <= 38.5);

        // Environmental temperatures should be reasonable
        assert_eq!(config.water_temperature_min, 0.0); // Freezing
        assert_eq!(config.water_temperature_max, 100.0); // Boiling
    }

    // ========================================
    // Complex Validation Scenarios
    // ========================================

    #[test]
    fn test_multiple_simultaneous_validation_errors() {
        let mut config = ValidationConfig::default();

        // Set multiple invalid configurations
        config.heart_rate_min = 400;
        config.heart_rate_max = 300;
        config.systolic_min = 300;
        config.systolic_max = 200;
        config.sleep_efficiency_min = 150.0;
        config.sleep_efficiency_max = 100.0;

        let result = config.validate();
        assert!(result.is_err());

        // Should report the first error encountered
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("heart_rate_min must be less than heart_rate_max"));
    }

    #[test]
    fn test_edge_case_equal_min_max_values() {
        let mut config = ValidationConfig::default();

        // Test equal min/max values (should fail for strict less-than checks)
        config.heart_rate_min = 150;
        config.heart_rate_max = 150;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("heart_rate_min must be less than heart_rate_max"));
    }

    #[test]
    fn test_edge_case_equal_values_allowed() {
        let mut config = ValidationConfig::default();

        // Test equal values where <= is allowed (like menstrual ratings)
        config.menstrual_cramps_severity_min = 5;
        config.menstrual_cramps_severity_max = 5;

        let result = config.validate();
        // This should pass since we use <= for these fields
        assert!(result.is_ok());
    }

    #[test]
    fn test_configuration_cloning_and_debug() {
        let config = ValidationConfig::default();

        // Test that the config can be cloned
        let cloned_config = config.clone();
        assert_eq!(config.heart_rate_min, cloned_config.heart_rate_min);
        assert_eq!(config.blood_glucose_max, cloned_config.blood_glucose_max);

        // Test debug formatting (should not panic)
        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("ValidationConfig"));
        assert!(debug_output.contains("heart_rate_min"));
    }
}