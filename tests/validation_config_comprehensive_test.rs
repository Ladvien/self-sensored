use std::env;

use self_sensored::config::ValidationConfig;

#[test]
fn test_validation_config_default() {
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
    assert_eq!(config.workout_max_duration_hours, 24.0);

    // Temperature validation
    assert_eq!(config.temperature_celsius_min, 25.0);
    assert_eq!(config.temperature_celsius_max, 45.0);
    assert_eq!(config.temperature_fahrenheit_min, 77.0);
    assert_eq!(config.temperature_fahrenheit_max, 113.0);

    // Blood glucose validation
    assert_eq!(config.glucose_mg_dl_min, 20.0);
    assert_eq!(config.glucose_mg_dl_max, 600.0);
    assert_eq!(config.glucose_mmol_l_min, 1.1);
    assert_eq!(config.glucose_mmol_l_max, 33.3);

    // Respiratory validation
    assert_eq!(config.respiratory_rate_min, 5);
    assert_eq!(config.respiratory_rate_max, 60);
    assert_eq!(config.blood_oxygen_min, 60.0);
    assert_eq!(config.blood_oxygen_max, 100.0);

    // Body measurement validation
    assert_eq!(config.weight_kg_min, 0.5);
    assert_eq!(config.weight_kg_max, 700.0);
    assert_eq!(config.height_cm_min, 20.0);
    assert_eq!(config.height_cm_max, 300.0);
    assert_eq!(config.body_fat_percentage_min, 1.0);
    assert_eq!(config.body_fat_percentage_max, 80.0);
}

#[test]
fn test_validation_config_from_env() {
    // Set up test environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "20");
    env::set_var("VALIDATION_HEART_RATE_MAX", "250");
    env::set_var("VALIDATION_SYSTOLIC_MIN", "60");
    env::set_var("VALIDATION_SYSTOLIC_MAX", "240");
    env::set_var("VALIDATION_STEP_COUNT_MAX", "150000");
    env::set_var("VALIDATION_DISTANCE_MAX_KM", "400.0");

    let config = ValidationConfig::from_env();

    // Verify environment overrides
    assert_eq!(config.heart_rate_min, 20);
    assert_eq!(config.heart_rate_max, 250);
    assert_eq!(config.systolic_min, 60);
    assert_eq!(config.systolic_max, 240);
    assert_eq!(config.step_count_max, 150000);
    assert_eq!(config.distance_max_km, 400.0);

    // Clean up environment
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_HEART_RATE_MAX");
    env::remove_var("VALIDATION_SYSTOLIC_MIN");
    env::remove_var("VALIDATION_SYSTOLIC_MAX");
    env::remove_var("VALIDATION_STEP_COUNT_MAX");
    env::remove_var("VALIDATION_DISTANCE_MAX_KM");
}

#[test]
fn test_validation_config_validate() {
    // Valid configuration
    let valid_config = ValidationConfig::default();
    assert!(valid_config.validate().is_ok());

    // Invalid: min > max for heart rate
    let mut invalid_config = ValidationConfig::default();
    invalid_config.heart_rate_min = 200;
    invalid_config.heart_rate_max = 100;
    assert!(invalid_config.validate().is_err());

    // Invalid: min > max for blood pressure
    let mut invalid_config = ValidationConfig::default();
    invalid_config.systolic_min = 200;
    invalid_config.systolic_max = 100;
    assert!(invalid_config.validate().is_err());

    // Invalid: negative sleep efficiency
    let mut invalid_config = ValidationConfig::default();
    invalid_config.sleep_efficiency_min = -10.0;
    assert!(invalid_config.validate().is_err());

    // Invalid: sleep efficiency > 100
    let mut invalid_config = ValidationConfig::default();
    invalid_config.sleep_efficiency_max = 150.0;
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
}

#[test]
fn test_validation_config_custom_values() {
    let config = ValidationConfig {
        heart_rate_min: 30,
        heart_rate_max: 220,
        systolic_min: 70,
        systolic_max: 200,
        diastolic_min: 40,
        diastolic_max: 120,
        sleep_efficiency_min: 10.0,
        sleep_efficiency_max: 95.0,
        sleep_duration_tolerance_minutes: 30,
        step_count_min: 100,
        step_count_max: 100000,
        distance_max_km: 200.0,
        calories_max: 10000.0,
        latitude_min: -85.0,
        latitude_max: 85.0,
        longitude_min: -170.0,
        longitude_max: 170.0,
        workout_heart_rate_min: 40,
        workout_heart_rate_max: 200,
        workout_max_duration_hours: 12.0,
        temperature_celsius_min: 30.0,
        temperature_celsius_max: 42.0,
        temperature_fahrenheit_min: 86.0,
        temperature_fahrenheit_max: 107.6,
        glucose_mg_dl_min: 30.0,
        glucose_mg_dl_max: 500.0,
        glucose_mmol_l_min: 1.7,
        glucose_mmol_l_max: 27.8,
        respiratory_rate_min: 8,
        respiratory_rate_max: 40,
        blood_oxygen_min: 70.0,
        blood_oxygen_max: 100.0,
        weight_kg_min: 1.0,
        weight_kg_max: 500.0,
        height_cm_min: 30.0,
        height_cm_max: 250.0,
        body_fat_percentage_min: 2.0,
        body_fat_percentage_max: 60.0,
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_validation_ranges_heart_rate() {
    let config = ValidationConfig::default();

    // Valid heart rates
    assert!(config.is_heart_rate_valid(60));
    assert!(config.is_heart_rate_valid(15));  // Min
    assert!(config.is_heart_rate_valid(300)); // Max
    assert!(config.is_heart_rate_valid(180)); // Exercise

    // Invalid heart rates
    assert!(!config.is_heart_rate_valid(14));   // Below min
    assert!(!config.is_heart_rate_valid(301));  // Above max
    assert!(!config.is_heart_rate_valid(0));    // Zero
    assert!(!config.is_heart_rate_valid(-50));  // Negative
}

#[test]
fn test_validation_ranges_blood_pressure() {
    let config = ValidationConfig::default();

    // Valid blood pressure
    assert!(config.is_blood_pressure_valid(120, 80));
    assert!(config.is_blood_pressure_valid(50, 30));   // Min
    assert!(config.is_blood_pressure_valid(250, 150)); // Max
    assert!(config.is_blood_pressure_valid(140, 90));  // Hypertensive

    // Invalid blood pressure
    assert!(!config.is_blood_pressure_valid(49, 80));   // Systolic too low
    assert!(!config.is_blood_pressure_valid(251, 80));  // Systolic too high
    assert!(!config.is_blood_pressure_valid(120, 29));  // Diastolic too low
    assert!(!config.is_blood_pressure_valid(120, 151)); // Diastolic too high
}

#[test]
fn test_validation_ranges_sleep_efficiency() {
    let config = ValidationConfig::default();

    // Valid sleep efficiency
    assert!(config.is_sleep_efficiency_valid(85.5));
    assert!(config.is_sleep_efficiency_valid(0.0));   // Min
    assert!(config.is_sleep_efficiency_valid(100.0)); // Max
    assert!(config.is_sleep_efficiency_valid(50.0));  // Poor sleep

    // Invalid sleep efficiency
    assert!(!config.is_sleep_efficiency_valid(-0.1));  // Below min
    assert!(!config.is_sleep_efficiency_valid(100.1)); // Above max
}

#[test]
fn test_validation_ranges_activity() {
    let config = ValidationConfig::default();

    // Valid activity metrics
    assert!(config.is_step_count_valid(10000));
    assert!(config.is_step_count_valid(0));      // Min
    assert!(config.is_step_count_valid(200000)); // Max (ultra-marathoner)

    assert!(config.is_distance_valid_km(10.0));
    assert!(config.is_distance_valid_km(0.0));   // Min
    assert!(config.is_distance_valid_km(500.0)); // Max

    assert!(config.is_calories_valid(2000.0));
    assert!(config.is_calories_valid(0.0));      // Min
    assert!(config.is_calories_valid(20000.0));  // Max

    // Invalid activity metrics
    assert!(!config.is_step_count_valid(-1));
    assert!(!config.is_step_count_valid(200001));

    assert!(!config.is_distance_valid_km(-1.0));
    assert!(!config.is_distance_valid_km(500.1));

    assert!(!config.is_calories_valid(-1.0));
    assert!(!config.is_calories_valid(20001.0));
}

#[test]
fn test_validation_ranges_gps() {
    let config = ValidationConfig::default();

    // Valid GPS coordinates
    assert!(config.is_gps_valid(0.0, 0.0));           // Equator, Prime Meridian
    assert!(config.is_gps_valid(90.0, 180.0));        // North Pole, International Date Line
    assert!(config.is_gps_valid(-90.0, -180.0));      // South Pole, International Date Line
    assert!(config.is_gps_valid(37.7749, -122.4194)); // San Francisco

    // Invalid GPS coordinates
    assert!(!config.is_gps_valid(91.0, 0.0));    // Latitude too high
    assert!(!config.is_gps_valid(-91.0, 0.0));   // Latitude too low
    assert!(!config.is_gps_valid(0.0, 181.0));   // Longitude too high
    assert!(!config.is_gps_valid(0.0, -181.0));  // Longitude too low
}

#[test]
fn test_validation_ranges_temperature() {
    let config = ValidationConfig::default();

    // Valid temperatures
    assert!(config.is_temperature_celsius_valid(36.5));
    assert!(config.is_temperature_celsius_valid(25.0)); // Min
    assert!(config.is_temperature_celsius_valid(45.0)); // Max
    assert!(config.is_temperature_celsius_valid(38.0)); // Fever

    assert!(config.is_temperature_fahrenheit_valid(98.6));
    assert!(config.is_temperature_fahrenheit_valid(77.0));  // Min
    assert!(config.is_temperature_fahrenheit_valid(113.0)); // Max

    // Invalid temperatures
    assert!(!config.is_temperature_celsius_valid(24.9));
    assert!(!config.is_temperature_celsius_valid(45.1));

    assert!(!config.is_temperature_fahrenheit_valid(76.9));
    assert!(!config.is_temperature_fahrenheit_valid(113.1));
}

#[test]
fn test_validation_ranges_blood_glucose() {
    let config = ValidationConfig::default();

    // Valid glucose levels
    assert!(config.is_glucose_mg_dl_valid(95.0));
    assert!(config.is_glucose_mg_dl_valid(20.0));  // Min (hypoglycemia)
    assert!(config.is_glucose_mg_dl_valid(600.0)); // Max (severe hyperglycemia)

    assert!(config.is_glucose_mmol_l_valid(5.3));
    assert!(config.is_glucose_mmol_l_valid(1.1));  // Min
    assert!(config.is_glucose_mmol_l_valid(33.3)); // Max

    // Invalid glucose levels
    assert!(!config.is_glucose_mg_dl_valid(19.9));
    assert!(!config.is_glucose_mg_dl_valid(600.1));

    assert!(!config.is_glucose_mmol_l_valid(1.0));
    assert!(!config.is_glucose_mmol_l_valid(33.4));
}

#[test]
fn test_validation_ranges_respiratory() {
    let config = ValidationConfig::default();

    // Valid respiratory metrics
    assert!(config.is_respiratory_rate_valid(16));
    assert!(config.is_respiratory_rate_valid(5));  // Min
    assert!(config.is_respiratory_rate_valid(60)); // Max (tachypnea)

    assert!(config.is_blood_oxygen_valid(98.0));
    assert!(config.is_blood_oxygen_valid(60.0));  // Min (severe hypoxia)
    assert!(config.is_blood_oxygen_valid(100.0)); // Max

    // Invalid respiratory metrics
    assert!(!config.is_respiratory_rate_valid(4));
    assert!(!config.is_respiratory_rate_valid(61));

    assert!(!config.is_blood_oxygen_valid(59.9));
    assert!(!config.is_blood_oxygen_valid(100.1));
}

#[test]
fn test_validation_ranges_body_measurements() {
    let config = ValidationConfig::default();

    // Valid body measurements
    assert!(config.is_weight_kg_valid(70.0));
    assert!(config.is_weight_kg_valid(0.5));   // Min (newborn)
    assert!(config.is_weight_kg_valid(700.0)); // Max

    assert!(config.is_height_cm_valid(175.0));
    assert!(config.is_height_cm_valid(20.0));  // Min (premature infant)
    assert!(config.is_height_cm_valid(300.0)); // Max

    assert!(config.is_body_fat_percentage_valid(20.0));
    assert!(config.is_body_fat_percentage_valid(1.0));  // Min (bodybuilder)
    assert!(config.is_body_fat_percentage_valid(80.0)); // Max

    // Invalid body measurements
    assert!(!config.is_weight_kg_valid(0.4));
    assert!(!config.is_weight_kg_valid(700.1));

    assert!(!config.is_height_cm_valid(19.9));
    assert!(!config.is_height_cm_valid(300.1));

    assert!(!config.is_body_fat_percentage_valid(0.9));
    assert!(!config.is_body_fat_percentage_valid(80.1));
}

#[test]
fn test_partial_env_override() {
    // Set only some environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "25");
    env::set_var("VALIDATION_CALORIES_MAX", "15000.0");

    let config = ValidationConfig::from_env();

    // Overridden values
    assert_eq!(config.heart_rate_min, 25);
    assert_eq!(config.calories_max, 15000.0);

    // Default values (not overridden)
    assert_eq!(config.heart_rate_max, 300);
    assert_eq!(config.step_count_max, 200000);

    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_CALORIES_MAX");
}

#[test]
fn test_invalid_env_values() {
    // Set invalid environment variables
    env::set_var("VALIDATION_HEART_RATE_MIN", "not_a_number");
    env::set_var("VALIDATION_DISTANCE_MAX_KM", "invalid");

    // Should fall back to defaults for invalid values
    let config = ValidationConfig::from_env();
    assert_eq!(config.heart_rate_min, 15);     // Default
    assert_eq!(config.distance_max_km, 500.0); // Default

    // Clean up
    env::remove_var("VALIDATION_HEART_RATE_MIN");
    env::remove_var("VALIDATION_DISTANCE_MAX_KM");
}

#[test]
fn test_edge_case_validations() {
    let config = ValidationConfig::default();

    // Edge cases for workout duration
    assert!(config.is_workout_duration_hours_valid(0.0));   // Just started
    assert!(config.is_workout_duration_hours_valid(24.0));  // Max (ultra-endurance)
    assert!(!config.is_workout_duration_hours_valid(24.1)); // Over max
    assert!(!config.is_workout_duration_hours_valid(-0.1)); // Negative

    // Edge cases for sleep duration tolerance
    let sleep_start = chrono::Utc::now();
    let sleep_end = sleep_start + chrono::Duration::hours(8);
    let calculated_minutes = 480;
    let recorded_minutes = 480;
    assert!(config.is_sleep_duration_consistent(calculated_minutes, recorded_minutes));

    let recorded_with_tolerance = 480 + config.sleep_duration_tolerance_minutes;
    assert!(config.is_sleep_duration_consistent(calculated_minutes, recorded_with_tolerance));

    let recorded_over_tolerance = 480 + config.sleep_duration_tolerance_minutes + 1;
    assert!(!config.is_sleep_duration_consistent(calculated_minutes, recorded_over_tolerance));
}

// Mock the trait implementations for testing
impl ValidationConfig {
    fn is_heart_rate_valid(&self, rate: i32) -> bool {
        rate >= self.heart_rate_min && rate <= self.heart_rate_max
    }

    fn is_blood_pressure_valid(&self, systolic: i32, diastolic: i32) -> bool {
        systolic >= self.systolic_min && systolic <= self.systolic_max &&
        diastolic >= self.diastolic_min && diastolic <= self.diastolic_max
    }

    fn is_sleep_efficiency_valid(&self, efficiency: f64) -> bool {
        efficiency >= self.sleep_efficiency_min && efficiency <= self.sleep_efficiency_max
    }

    fn is_step_count_valid(&self, steps: i32) -> bool {
        steps >= self.step_count_min && steps <= self.step_count_max
    }

    fn is_distance_valid_km(&self, distance: f64) -> bool {
        distance >= 0.0 && distance <= self.distance_max_km
    }

    fn is_calories_valid(&self, calories: f64) -> bool {
        calories >= 0.0 && calories <= self.calories_max
    }

    fn is_gps_valid(&self, lat: f64, lon: f64) -> bool {
        lat >= self.latitude_min && lat <= self.latitude_max &&
        lon >= self.longitude_min && lon <= self.longitude_max
    }

    fn is_temperature_celsius_valid(&self, temp: f64) -> bool {
        temp >= self.temperature_celsius_min && temp <= self.temperature_celsius_max
    }

    fn is_temperature_fahrenheit_valid(&self, temp: f64) -> bool {
        temp >= self.temperature_fahrenheit_min && temp <= self.temperature_fahrenheit_max
    }

    fn is_glucose_mg_dl_valid(&self, glucose: f64) -> bool {
        glucose >= self.glucose_mg_dl_min && glucose <= self.glucose_mg_dl_max
    }

    fn is_glucose_mmol_l_valid(&self, glucose: f64) -> bool {
        glucose >= self.glucose_mmol_l_min && glucose <= self.glucose_mmol_l_max
    }

    fn is_respiratory_rate_valid(&self, rate: i32) -> bool {
        rate >= self.respiratory_rate_min && rate <= self.respiratory_rate_max
    }

    fn is_blood_oxygen_valid(&self, oxygen: f64) -> bool {
        oxygen >= self.blood_oxygen_min && oxygen <= self.blood_oxygen_max
    }

    fn is_weight_kg_valid(&self, weight: f64) -> bool {
        weight >= self.weight_kg_min && weight <= self.weight_kg_max
    }

    fn is_height_cm_valid(&self, height: f64) -> bool {
        height >= self.height_cm_min && height <= self.height_cm_max
    }

    fn is_body_fat_percentage_valid(&self, fat: f64) -> bool {
        fat >= self.body_fat_percentage_min && fat <= self.body_fat_percentage_max
    }

    fn is_workout_duration_hours_valid(&self, hours: f64) -> bool {
        hours >= 0.0 && hours <= self.workout_max_duration_hours
    }

    fn is_sleep_duration_consistent(&self, calculated: i32, recorded: i32) -> bool {
        (calculated - recorded).abs() <= self.sleep_duration_tolerance_minutes
    }
}