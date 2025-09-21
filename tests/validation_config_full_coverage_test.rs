use std::env;

use self_sensored::config::ValidationConfig;

#[test]
fn test_validation_config_default_all_fields() {
    let config = ValidationConfig::default();

    // Heart rate validation
    assert_eq!(config.heart_rate_min, 15);
    assert_eq!(config.heart_rate_max, 300);

    // Blood pressure validation
    assert_eq!(config.systolic_min, 50);
    assert_eq!(config.systolic_max, 250);
    assert_eq!(config.diastolic_min, 30);
    assert_eq!(config.diastolic_max, 150);

    // Sleep validation
    assert_eq!(config.sleep_efficiency_min, 0.0);
    assert_eq!(config.sleep_efficiency_max, 100.0);
    assert_eq!(config.sleep_duration_tolerance_minutes, 60);

    // Activity validation
    assert_eq!(config.step_count_min, 0);
    assert_eq!(config.step_count_max, 200000);
    assert_eq!(config.distance_max_km, 500.0);
    assert_eq!(config.calories_max, 20000.0);

    // GPS validation
    assert_eq!(config.latitude_min, -90.0);
    assert_eq!(config.latitude_max, 90.0);
    assert_eq!(config.longitude_min, -180.0);
    assert_eq!(config.longitude_max, 180.0);

    // Workout validation
    assert_eq!(config.workout_heart_rate_min, 15);
    assert_eq!(config.workout_heart_rate_max, 300);
    assert_eq!(config.workout_max_duration_hours, 24);

    // Blood glucose validation
    assert_eq!(config.blood_glucose_min, 20.0);
    assert_eq!(config.blood_glucose_max, 600.0);
    assert_eq!(config.insulin_max_units, 200.0);

    // Respiratory validation
    assert_eq!(config.respiratory_rate_min, 5);
    assert_eq!(config.respiratory_rate_max, 60);
    assert_eq!(config.oxygen_saturation_min, 60.0);
    assert_eq!(config.oxygen_saturation_max, 100.0);
    assert_eq!(config.oxygen_saturation_critical, 90.0);
    assert_eq!(config.forced_vital_capacity_min, 1.0);
    assert_eq!(config.forced_vital_capacity_max, 8.0);
    assert_eq!(config.forced_expiratory_volume_1_min, 0.5);
    assert_eq!(config.forced_expiratory_volume_1_max, 6.0);
    assert_eq!(config.peak_expiratory_flow_rate_min, 50.0);
    assert_eq!(config.peak_expiratory_flow_rate_max, 800.0);
    assert_eq!(config.inhaler_usage_max, 20);

    // Temperature validation
    assert_eq!(config.body_temperature_min, 30.0);
    assert_eq!(config.body_temperature_max, 45.0);
    assert_eq!(config.basal_body_temperature_min, 35.0);
    assert_eq!(config.basal_body_temperature_max, 38.0);
    assert_eq!(config.wrist_temperature_min, 20.0);
    assert_eq!(config.wrist_temperature_max, 45.0);
    assert_eq!(config.water_temperature_min, 0.0);
    assert_eq!(config.water_temperature_max, 100.0);
    assert_eq!(config.fever_threshold, 38.0);

    // Body measurements
    assert_eq!(config.body_weight_min_kg, 0.5);
    assert_eq!(config.body_weight_max_kg, 700.0);
    assert_eq!(config.bmi_min, 10.0);
    assert_eq!(config.bmi_max, 80.0);
    assert_eq!(config.body_fat_min_percent, 1.0);
    assert_eq!(config.body_fat_max_percent, 80.0);
    assert_eq!(config.body_temperature_min_celsius, 30.0);
    assert_eq!(config.body_temperature_max_celsius, 45.0);

    // Reproductive health
    assert_eq!(config.menstrual_cycle_day_min, 1);
    assert_eq!(config.menstrual_cycle_day_max, 45);
    assert_eq!(config.menstrual_cramps_severity_min, 0);
    assert_eq!(config.menstrual_cramps_severity_max, 10);
    assert_eq!(config.menstrual_mood_rating_min, 1);
    assert_eq!(config.menstrual_mood_rating_max, 5);
    assert_eq!(config.menstrual_energy_level_min, 1);
    assert_eq!(config.menstrual_energy_level_max, 5);

    // Fertility tracking
    assert_eq!(config.fertility_basal_temp_min, 35.0);
    assert_eq!(config.fertility_basal_temp_max, 39.0);
    assert_eq!(config.fertility_cervix_firmness_min, 1);
    assert_eq!(config.fertility_cervix_firmness_max, 3);
    assert_eq!(config.fertility_cervix_position_min, 1);
    assert_eq!(config.fertility_cervix_position_max, 3);
    assert_eq!(config.fertility_lh_level_min, 0.0);
    assert_eq!(config.fertility_lh_level_max, 100.0);
}

#[test]
fn test_validation_config_from_env_comprehensive() {
    // Set up comprehensive environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "20");
    env::set_var("VALIDATION_HEART_RATE_MAX", "250");
    env::set_var("VALIDATION_SYSTOLIC_MIN", "60");
    env::set_var("VALIDATION_SYSTOLIC_MAX", "240");
    env::set_var("VALIDATION_DIASTOLIC_MIN", "40");
    env::set_var("VALIDATION_DIASTOLIC_MAX", "140");

    env::set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "10.0");
    env::set_var("VALIDATION_SLEEP_EFFICIENCY_MAX", "95.0");
    env::set_var("VALIDATION_SLEEP_DURATION_TOLERANCE_MINUTES", "30");

    env::set_var("VALIDATION_STEP_COUNT_MIN", "100");
    env::set_var("VALIDATION_STEP_COUNT_MAX", "150000");
    env::set_var("VALIDATION_DISTANCE_MAX_KM", "400.0");
    env::set_var("VALIDATION_CALORIES_MAX", "15000.0");

    env::set_var("VALIDATION_LATITUDE_MIN", "-85.0");
    env::set_var("VALIDATION_LATITUDE_MAX", "85.0");
    env::set_var("VALIDATION_LONGITUDE_MIN", "-170.0");
    env::set_var("VALIDATION_LONGITUDE_MAX", "170.0");

    env::set_var("VALIDATION_WORKOUT_HEART_RATE_MIN", "30");
    env::set_var("VALIDATION_WORKOUT_HEART_RATE_MAX", "220");
    env::set_var("VALIDATION_WORKOUT_MAX_DURATION_HOURS", "12");

    env::set_var("VALIDATION_BLOOD_GLUCOSE_MIN", "30.0");
    env::set_var("VALIDATION_BLOOD_GLUCOSE_MAX", "500.0");
    env::set_var("VALIDATION_INSULIN_MAX_UNITS", "150.0");

    env::set_var("VALIDATION_RESPIRATORY_RATE_MIN", "8");
    env::set_var("VALIDATION_RESPIRATORY_RATE_MAX", "40");
    env::set_var("VALIDATION_OXYGEN_SATURATION_MIN", "70.0");
    env::set_var("VALIDATION_OXYGEN_SATURATION_MAX", "100.0");
    env::set_var("VALIDATION_OXYGEN_SATURATION_CRITICAL", "85.0");

    env::set_var("VALIDATION_BODY_TEMPERATURE_MIN", "32.0");
    env::set_var("VALIDATION_BODY_TEMPERATURE_MAX", "42.0");
    env::set_var("VALIDATION_FEVER_THRESHOLD", "37.5");

    env::set_var("VALIDATION_BODY_WEIGHT_MIN_KG", "1.0");
    env::set_var("VALIDATION_BODY_WEIGHT_MAX_KG", "600.0");
    env::set_var("VALIDATION_BMI_MIN", "12.0");
    env::set_var("VALIDATION_BMI_MAX", "70.0");

    let config = ValidationConfig::from_env();

    // Verify environment overrides
    assert_eq!(config.heart_rate_min, 20);
    assert_eq!(config.heart_rate_max, 250);
    assert_eq!(config.systolic_min, 60);
    assert_eq!(config.systolic_max, 240);
    assert_eq!(config.diastolic_min, 40);
    assert_eq!(config.diastolic_max, 140);

    assert_eq!(config.sleep_efficiency_min, 10.0);
    assert_eq!(config.sleep_efficiency_max, 95.0);
    assert_eq!(config.sleep_duration_tolerance_minutes, 30);

    assert_eq!(config.step_count_min, 100);
    assert_eq!(config.step_count_max, 150000);
    assert_eq!(config.distance_max_km, 400.0);
    assert_eq!(config.calories_max, 15000.0);

    assert_eq!(config.latitude_min, -85.0);
    assert_eq!(config.latitude_max, 85.0);
    assert_eq!(config.longitude_min, -170.0);
    assert_eq!(config.longitude_max, 170.0);

    assert_eq!(config.workout_heart_rate_min, 30);
    assert_eq!(config.workout_heart_rate_max, 220);
    assert_eq!(config.workout_max_duration_hours, 12);

    assert_eq!(config.blood_glucose_min, 30.0);
    assert_eq!(config.blood_glucose_max, 500.0);
    assert_eq!(config.insulin_max_units, 150.0);

    assert_eq!(config.respiratory_rate_min, 8);
    assert_eq!(config.respiratory_rate_max, 40);
    assert_eq!(config.oxygen_saturation_min, 70.0);
    assert_eq!(config.oxygen_saturation_max, 100.0);
    assert_eq!(config.oxygen_saturation_critical, 85.0);

    assert_eq!(config.body_temperature_min, 32.0);
    assert_eq!(config.body_temperature_max, 42.0);
    assert_eq!(config.fever_threshold, 37.5);

    assert_eq!(config.body_weight_min_kg, 1.0);
    assert_eq!(config.body_weight_max_kg, 600.0);
    assert_eq!(config.bmi_min, 12.0);
    assert_eq!(config.bmi_max, 70.0);

    // Clean up all environment variables
    let env_vars = [
        "VALIDATION_HEART_RATE_MIN", "VALIDATION_HEART_RATE_MAX",
        "VALIDATION_SYSTOLIC_MIN", "VALIDATION_SYSTOLIC_MAX",
        "VALIDATION_DIASTOLIC_MIN", "VALIDATION_DIASTOLIC_MAX",
        "VALIDATION_SLEEP_EFFICIENCY_MIN", "VALIDATION_SLEEP_EFFICIENCY_MAX",
        "VALIDATION_SLEEP_DURATION_TOLERANCE_MINUTES",
        "VALIDATION_STEP_COUNT_MIN", "VALIDATION_STEP_COUNT_MAX",
        "VALIDATION_DISTANCE_MAX_KM", "VALIDATION_CALORIES_MAX",
        "VALIDATION_LATITUDE_MIN", "VALIDATION_LATITUDE_MAX",
        "VALIDATION_LONGITUDE_MIN", "VALIDATION_LONGITUDE_MAX",
        "VALIDATION_WORKOUT_HEART_RATE_MIN", "VALIDATION_WORKOUT_HEART_RATE_MAX",
        "VALIDATION_WORKOUT_MAX_DURATION_HOURS",
        "VALIDATION_BLOOD_GLUCOSE_MIN", "VALIDATION_BLOOD_GLUCOSE_MAX",
        "VALIDATION_INSULIN_MAX_UNITS",
        "VALIDATION_RESPIRATORY_RATE_MIN", "VALIDATION_RESPIRATORY_RATE_MAX",
        "VALIDATION_OXYGEN_SATURATION_MIN", "VALIDATION_OXYGEN_SATURATION_MAX",
        "VALIDATION_OXYGEN_SATURATION_CRITICAL",
        "VALIDATION_BODY_TEMPERATURE_MIN", "VALIDATION_BODY_TEMPERATURE_MAX",
        "VALIDATION_FEVER_THRESHOLD",
        "VALIDATION_BODY_WEIGHT_MIN_KG", "VALIDATION_BODY_WEIGHT_MAX_KG",
        "VALIDATION_BMI_MIN", "VALIDATION_BMI_MAX",
    ];

    for var in &env_vars {
        env::remove_var(var);
    }
}

#[test]
fn test_validation_config_validate_comprehensive() {
    // Test valid configuration
    let valid_config = ValidationConfig::default();
    assert!(valid_config.validate().is_ok());

    // Test all possible validation errors

    // Invalid: heart rate min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.heart_rate_min = 200;
    invalid_config.heart_rate_max = 100;
    assert!(invalid_config.validate().is_err());

    // Invalid: blood pressure systolic min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.systolic_min = 200;
    invalid_config.systolic_max = 100;
    assert!(invalid_config.validate().is_err());

    // Invalid: blood pressure diastolic min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.diastolic_min = 120;
    invalid_config.diastolic_max = 80;
    assert!(invalid_config.validate().is_err());

    // Invalid: sleep efficiency min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.sleep_efficiency_min = 80.0;
    invalid_config.sleep_efficiency_max = 60.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative sleep efficiency
    let mut invalid_config = ValidationConfig::default();
    invalid_config.sleep_efficiency_min = -10.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: sleep efficiency > 100
    let mut invalid_config = ValidationConfig::default();
    invalid_config.sleep_efficiency_max = 150.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: step count min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.step_count_min = 100000;
    invalid_config.step_count_max = 50000;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative step count
    let mut invalid_config = ValidationConfig::default();
    invalid_config.step_count_min = -100;
    assert!(invalid_config.validate().is_err());

    // Invalid: GPS latitude out of range
    let mut invalid_config = ValidationConfig::default();
    invalid_config.latitude_min = -100.0;
    assert!(invalid_config.validate().is_err());

    let mut invalid_config = ValidationConfig::default();
    invalid_config.latitude_max = 100.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: GPS longitude out of range
    let mut invalid_config = ValidationConfig::default();
    invalid_config.longitude_min = -200.0;
    assert!(invalid_config.validate().is_err());

    let mut invalid_config = ValidationConfig::default();
    invalid_config.longitude_max = 200.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: workout heart rate min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.workout_heart_rate_min = 200;
    invalid_config.workout_heart_rate_max = 100;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative workout duration
    let mut invalid_config = ValidationConfig::default();
    invalid_config.workout_max_duration_hours = -1;
    assert!(invalid_config.validate().is_err());

    // Invalid: blood glucose min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.blood_glucose_min = 400.0;
    invalid_config.blood_glucose_max = 200.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative respiratory rate
    let mut invalid_config = ValidationConfig::default();
    invalid_config.respiratory_rate_min = -5;
    assert!(invalid_config.validate().is_err());

    // Invalid: oxygen saturation out of range
    let mut invalid_config = ValidationConfig::default();
    invalid_config.oxygen_saturation_max = 150.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: body weight min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.body_weight_min_kg = 500.0;
    invalid_config.body_weight_max_kg = 100.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative BMI
    let mut invalid_config = ValidationConfig::default();
    invalid_config.bmi_min = -5.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: menstrual cycle day out of range
    let mut invalid_config = ValidationConfig::default();
    invalid_config.menstrual_cycle_day_min = 0;
    assert!(invalid_config.validate().is_err());

    let mut invalid_config = ValidationConfig::default();
    invalid_config.menstrual_cycle_day_max = 50;
    assert!(invalid_config.validate().is_err());

    // Invalid: fertility LH level min > max
    let mut invalid_config = ValidationConfig::default();
    invalid_config.fertility_lh_level_min = 80.0;
    invalid_config.fertility_lh_level_max = 50.0;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_validation_config_debug_clone() {
    let config = ValidationConfig::default();

    // Test Debug trait
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("ValidationConfig"));
    assert!(debug_str.contains("heart_rate_min"));

    // Test Clone trait
    let cloned = config.clone();
    assert_eq!(cloned.heart_rate_min, config.heart_rate_min);
    assert_eq!(cloned.heart_rate_max, config.heart_rate_max);
    assert_eq!(cloned.systolic_min, config.systolic_min);
}

#[test]
fn test_validation_config_edge_cases() {
    // Test with extreme but valid values
    let extreme_config = ValidationConfig {
        heart_rate_min: 1,
        heart_rate_max: 500,
        systolic_min: 20,
        systolic_max: 300,
        diastolic_min: 10,
        diastolic_max: 200,
        sleep_efficiency_min: 0.0,
        sleep_efficiency_max: 100.0,
        sleep_duration_tolerance_minutes: 0,
        step_count_min: 0,
        step_count_max: 1000000,
        distance_max_km: 1000.0,
        calories_max: 50000.0,
        latitude_min: -90.0,
        latitude_max: 90.0,
        longitude_min: -180.0,
        longitude_max: 180.0,
        workout_heart_rate_min: 1,
        workout_heart_rate_max: 500,
        workout_max_duration_hours: 48,
        blood_glucose_min: 1.0,
        blood_glucose_max: 1000.0,
        insulin_max_units: 500.0,
        respiratory_rate_min: 1,
        respiratory_rate_max: 100,
        oxygen_saturation_min: 50.0,
        oxygen_saturation_max: 100.0,
        oxygen_saturation_critical: 80.0,
        forced_vital_capacity_min: 0.5,
        forced_vital_capacity_max: 10.0,
        forced_expiratory_volume_1_min: 0.1,
        forced_expiratory_volume_1_max: 8.0,
        peak_expiratory_flow_rate_min: 10.0,
        peak_expiratory_flow_rate_max: 1000.0,
        inhaler_usage_max: 50,
        body_temperature_min: 25.0,
        body_temperature_max: 50.0,
        basal_body_temperature_min: 30.0,
        basal_body_temperature_max: 42.0,
        wrist_temperature_min: 15.0,
        wrist_temperature_max: 50.0,
        water_temperature_min: -10.0,
        water_temperature_max: 120.0,
        fever_threshold: 37.0,
        body_weight_min_kg: 0.1,
        body_weight_max_kg: 1000.0,
        bmi_min: 5.0,
        bmi_max: 100.0,
        body_fat_min_percent: 0.5,
        body_fat_max_percent: 90.0,
        body_temperature_min_celsius: 25.0,
        body_temperature_max_celsius: 50.0,
        menstrual_cycle_day_min: 1,
        menstrual_cycle_day_max: 45,
        menstrual_cramps_severity_min: 0,
        menstrual_cramps_severity_max: 10,
        menstrual_mood_rating_min: 1,
        menstrual_mood_rating_max: 10,
        menstrual_energy_level_min: 1,
        menstrual_energy_level_max: 10,
        fertility_basal_temp_min: 30.0,
        fertility_basal_temp_max: 45.0,
        fertility_cervix_firmness_min: 1,
        fertility_cervix_firmness_max: 5,
        fertility_cervix_position_min: 1,
        fertility_cervix_position_max: 5,
        fertility_lh_level_min: 0.0,
        fertility_lh_level_max: 200.0,
    };

    assert!(extreme_config.validate().is_ok());
}

#[test]
fn test_partial_env_override() {
    // Set only some environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "25");
    env::set_var("VALIDATION_CALORIES_MAX", "15000.0");
    env::set_var("VALIDATION_FEVER_THRESHOLD", "37.8");

    let config = ValidationConfig::from_env();

    // Overridden values
    assert_eq!(config.heart_rate_min, 25);
    assert_eq!(config.calories_max, 15000.0);
    assert_eq!(config.fever_threshold, 37.8);

    // Default values (not overridden)
    assert_eq!(config.heart_rate_max, 300);
    assert_eq!(config.step_count_max, 200000);
    assert_eq!(config.oxygen_saturation_critical, 90.0);

    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_CALORIES_MAX");
    env::remove_var("VALIDATION_FEVER_THRESHOLD");
}

#[test]
fn test_invalid_env_values() {
    // Set invalid environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "not_a_number");
    env::set_var("VALIDATION_DISTANCE_MAX_KM", "invalid");
    env::set_var("VALIDATION_SLEEP_EFFICIENCY_MIN", "not_float");
    env::set_var("VALIDATION_WORKOUT_MAX_DURATION_HOURS", "not_int");

    // Should fall back to defaults for invalid values
    let config = ValidationConfig::from_env();
    assert_eq!(config.heart_rate_min, 15);         // Default
    assert_eq!(config.distance_max_km, 500.0);     // Default
    assert_eq!(config.sleep_efficiency_min, 0.0);  // Default
    assert_eq!(config.workout_max_duration_hours, 24); // Default

    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_DISTANCE_MAX_KM");
    env::remove_var("VALIDATION_SLEEP_EFFICIENCY_MIN");
    env::remove_var("VALIDATION_WORKOUT_MAX_DURATION_HOURS");
}

#[test]
fn test_empty_env_values() {
    // Set empty environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "");
    env::set_var("VALIDATION_SYSTOLIC_MAX", "");

    let config = ValidationConfig::from_env();

    // Should use defaults for empty values
    assert_eq!(config.heart_rate_min, 15);   // Default
    assert_eq!(config.systolic_max, 250);    // Default

    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_SYSTOLIC_MAX");
}

#[test]
fn test_config_consistency_checks() {
    let config = ValidationConfig::default();

    // Verify internal consistency of default values
    assert!(config.heart_rate_min < config.heart_rate_max);
    assert!(config.systolic_min < config.systolic_max);
    assert!(config.diastolic_min < config.diastolic_max);
    assert!(config.sleep_efficiency_min <= config.sleep_efficiency_max);
    assert!(config.step_count_min <= config.step_count_max);
    assert!(config.latitude_min <= config.latitude_max);
    assert!(config.longitude_min <= config.longitude_max);
    assert!(config.workout_heart_rate_min < config.workout_heart_rate_max);
    assert!(config.blood_glucose_min < config.blood_glucose_max);
    assert!(config.respiratory_rate_min < config.respiratory_rate_max);
    assert!(config.oxygen_saturation_min < config.oxygen_saturation_max);
    assert!(config.body_weight_min_kg < config.body_weight_max_kg);
    assert!(config.bmi_min < config.bmi_max);
    assert!(config.body_fat_min_percent < config.body_fat_max_percent);
    assert!(config.menstrual_cycle_day_min < config.menstrual_cycle_day_max);
    assert!(config.fertility_lh_level_min < config.fertility_lh_level_max);

    // Verify ranges are within expected bounds
    assert!(config.latitude_min >= -90.0 && config.latitude_max <= 90.0);
    assert!(config.longitude_min >= -180.0 && config.longitude_max <= 180.0);
    assert!(config.sleep_efficiency_min >= 0.0 && config.sleep_efficiency_max <= 100.0);
    assert!(config.oxygen_saturation_min >= 0.0 && config.oxygen_saturation_max <= 100.0);
    assert!(config.step_count_min >= 0);
    assert!(config.menstrual_cycle_day_min >= 1);
}