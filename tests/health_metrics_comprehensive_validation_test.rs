// Comprehensive validation tests for all health metric models
// Tests all validation methods with 100% coverage

mod common;
mod models;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json;
use uuid::Uuid;

use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{
    ActivityContext, CardiacEventSeverity, CervicalMucusQuality, HeartRateEventType,
    HygieneEventType, MeditationType, MenstrualFlow, OvulationTestResult, PregnancyTestResult,
    StateOfMind, SymptomSeverity, SymptomType, TemperatureContext, WorkoutType,
};
use self_sensored::models::health_metrics::*;

/// Helper to create a test user ID
fn test_user_id() -> Uuid {
    Uuid::new_v4()
}

/// Helper to create a test timestamp
fn test_timestamp() -> DateTime<Utc> {
    Utc::now() - chrono::Duration::hours(1) // 1 hour ago to avoid future date issues
}

/// Helper to create custom validation config for testing
fn custom_validation_config() -> ValidationConfig {
    let mut config = ValidationConfig::default();
    config.heart_rate_min = 20;
    config.heart_rate_max = 250;
    config.systolic_min = 60;
    config.systolic_max = 200;
    config.diastolic_min = 40;
    config.diastolic_max = 120;
    config
}

fn create_valid_heart_rate_metric() -> HeartRateMetric {
    HeartRateMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(35.5),
        walking_heart_rate_average: Some(90),
        heart_rate_recovery_one_minute: Some(25),
        atrial_fibrillation_burden_percentage: Some(Decimal::new(150, 2)), // 1.50%
        vo2_max_ml_kg_min: Some(Decimal::new(4500, 2)),                    // 45.00
        source_device: Some("Apple Watch".to_string()),
        context: Some(ActivityContext::Resting),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_heart_rate_metric_validation_success() {
    let metric = create_valid_heart_rate_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_heart_rate_validation_boundary_conditions() {
    let config = ValidationConfig::default();

    // Test minimum valid heart rate
    let mut metric = create_valid_heart_rate_metric();
    metric.heart_rate = Some(config.heart_rate_min);
    assert!(metric.validate_with_config(&config).is_ok());

    // Test maximum valid heart rate
    metric.heart_rate = Some(config.heart_rate_max);
    assert!(metric.validate_with_config(&config).is_ok());

    // Test below minimum
    metric.heart_rate = Some(config.heart_rate_min - 1);
    assert!(metric.validate_with_config(&config).is_err());

    // Test above maximum
    metric.heart_rate = Some(config.heart_rate_max + 1);
    assert!(metric.validate_with_config(&config).is_err());
}

#[test]
fn test_heart_rate_variability_validation() {
    let mut metric = create_valid_heart_rate_metric();

    // Test valid HRV
    metric.heart_rate_variability = Some(50.0);
    assert!(metric.validate().is_ok());

    // Test HRV at minimum boundary
    metric.heart_rate_variability = Some(0.0);
    assert!(metric.validate().is_ok());

    // Test HRV at maximum boundary
    metric.heart_rate_variability = Some(500.0);
    assert!(metric.validate().is_ok());

    // Test invalid HRV (below minimum)
    metric.heart_rate_variability = Some(-0.1);
    assert!(metric.validate().is_err());

    // Test invalid HRV (above maximum)
    metric.heart_rate_variability = Some(500.1);
    assert!(metric.validate().is_err());
}

#[test]
fn test_walking_heart_rate_validation() {
    let mut metric = create_valid_heart_rate_metric();

    // Test valid walking HR
    metric.walking_heart_rate_average = Some(100);
    assert!(metric.validate().is_ok());

    // Test walking HR at boundaries
    metric.walking_heart_rate_average = Some(60);
    assert!(metric.validate().is_ok());

    metric.walking_heart_rate_average = Some(200);
    assert!(metric.validate().is_ok());

    // Test invalid walking HR (below minimum)
    metric.walking_heart_rate_average = Some(59);
    assert!(metric.validate().is_err());

    // Test invalid walking HR (above maximum)
    metric.walking_heart_rate_average = Some(201);
    assert!(metric.validate().is_err());
}

#[test]
fn test_heart_rate_recovery_validation() {
    let mut metric = create_valid_heart_rate_metric();

    // Test valid recovery
    metric.heart_rate_recovery_one_minute = Some(30);
    assert!(metric.validate().is_ok());

    // Test recovery at boundaries
    metric.heart_rate_recovery_one_minute = Some(0);
    assert!(metric.validate().is_ok());

    metric.heart_rate_recovery_one_minute = Some(100);
    assert!(metric.validate().is_ok());

    // Test invalid recovery (below minimum)
    metric.heart_rate_recovery_one_minute = Some(-1);
    assert!(metric.validate().is_err());

    // Test invalid recovery (above maximum)
    metric.heart_rate_recovery_one_minute = Some(101);
    assert!(metric.validate().is_err());
}

#[test]
fn test_atrial_fibrillation_burden_validation() {
    let mut metric = create_valid_heart_rate_metric();

    // Test valid AFib burden
    metric.atrial_fibrillation_burden_percentage = Some(Decimal::new(500, 2)); // 5.00%
    assert!(metric.validate().is_ok());

    // Test AFib burden at boundaries
    metric.atrial_fibrillation_burden_percentage = Some(Decimal::new(0, 2)); // 0.00%
    assert!(metric.validate().is_ok());

    metric.atrial_fibrillation_burden_percentage = Some(Decimal::new(10000, 2)); // 100.00%
    assert!(metric.validate().is_ok());

    // Test invalid AFib burden (above maximum)
    metric.atrial_fibrillation_burden_percentage = Some(Decimal::new(10001, 2)); // 100.01%
    assert!(metric.validate().is_err());
}

#[test]
fn test_vo2_max_validation() {
    let mut metric = create_valid_heart_rate_metric();

    // Test valid VO2 max
    metric.vo2_max_ml_kg_min = Some(Decimal::new(3500, 2)); // 35.00
    assert!(metric.validate().is_ok());

    // Test VO2 max at boundaries
    metric.vo2_max_ml_kg_min = Some(Decimal::new(1400, 2)); // 14.00
    assert!(metric.validate().is_ok());

    metric.vo2_max_ml_kg_min = Some(Decimal::new(6500, 2)); // 65.00
    assert!(metric.validate().is_ok());

    // Test invalid VO2 max (below minimum)
    metric.vo2_max_ml_kg_min = Some(Decimal::new(1399, 2)); // 13.99
    assert!(metric.validate().is_err());

    // Test invalid VO2 max (above maximum)
    metric.vo2_max_ml_kg_min = Some(Decimal::new(6501, 2)); // 65.01
    assert!(metric.validate().is_err());
}

fn create_valid_blood_pressure_metric() -> BloodPressureMetric {
    BloodPressureMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
        source_device: Some("Omron BP Monitor".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_blood_pressure_metric_validation_success() {
    let metric = create_valid_blood_pressure_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_systolic_pressure_validation() {
    let config = ValidationConfig::default();
    let mut metric = create_valid_blood_pressure_metric();

    // Test valid systolic at boundaries
    metric.systolic = config.systolic_min;
    metric.diastolic = config.systolic_min - 10; // Ensure systolic > diastolic
    assert!(metric.validate_with_config(&config).is_ok());

    metric.systolic = config.systolic_max;
    metric.diastolic = config.systolic_max - 20; // Ensure systolic > diastolic
    assert!(metric.validate_with_config(&config).is_ok());

    // Test invalid systolic (below minimum)
    metric.systolic = config.systolic_min - 1;
    metric.diastolic = config.systolic_min - 10;
    assert!(metric.validate_with_config(&config).is_err());

    // Test invalid systolic (above maximum)
    metric.systolic = config.systolic_max + 1;
    metric.diastolic = 80;
    assert!(metric.validate_with_config(&config).is_err());
}

#[test]
fn test_systolic_diastolic_relationship() {
    let mut metric = create_valid_blood_pressure_metric();

    // Test invalid: systolic <= diastolic
    metric.systolic = 80;
    metric.diastolic = 80;
    assert!(metric.validate().is_err());

    metric.systolic = 80;
    metric.diastolic = 90;
    assert!(metric.validate().is_err());

    // Test valid: systolic > diastolic
    metric.systolic = 120;
    metric.diastolic = 80;
    assert!(metric.validate().is_ok());
}

fn create_valid_sleep_metric() -> SleepMetric {
    let sleep_start = test_timestamp() - chrono::Duration::hours(8);
    let sleep_end = test_timestamp();

    SleepMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        sleep_start,
        sleep_end,
        duration_minutes: Some(480), // 8 hours
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(200),
        awake_minutes: Some(60),
        efficiency: Some(85.0),
        source_device: Some("Apple Watch".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_sleep_metric_validation_success() {
    let metric = create_valid_sleep_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_sleep_end_after_start_validation() {
    let mut metric = create_valid_sleep_metric();

    // Test invalid: sleep_end <= sleep_start
    metric.sleep_end = metric.sleep_start;
    assert!(metric.validate().is_err());

    metric.sleep_end = metric.sleep_start - chrono::Duration::minutes(1);
    assert!(metric.validate().is_err());

    // Test valid: sleep_end > sleep_start
    metric.sleep_end = metric.sleep_start + chrono::Duration::hours(8);
    assert!(metric.validate().is_ok());
}

#[test]
fn test_sleep_efficiency_validation() {
    let config = ValidationConfig::default();
    let mut metric = create_valid_sleep_metric();

    // Test valid efficiency at boundaries
    metric.efficiency = Some(config.sleep_efficiency_min as f64);
    assert!(metric.validate_with_config(&config).is_ok());

    metric.efficiency = Some(config.sleep_efficiency_max as f64);
    assert!(metric.validate_with_config(&config).is_ok());

    // Test invalid efficiency (below minimum)
    metric.efficiency = Some((config.sleep_efficiency_min - 0.1) as f64);
    assert!(metric.validate_with_config(&config).is_err());

    // Test invalid efficiency (above maximum)
    metric.efficiency = Some((config.sleep_efficiency_max + 0.1) as f64);
    assert!(metric.validate_with_config(&config).is_err());

    // Test None efficiency (should be valid)
    metric.efficiency = None;
    assert!(metric.validate_with_config(&config).is_ok());
}

#[test]
fn test_calculate_efficiency() {
    let sleep_start = test_timestamp() - chrono::Duration::hours(8);
    let sleep_end = test_timestamp();

    let mut metric = create_valid_sleep_metric();
    metric.sleep_start = sleep_start;
    metric.sleep_end = sleep_end;
    metric.duration_minutes = Some(400); // 400 out of 480 minutes = ~83.33%

    let efficiency = metric.calculate_efficiency();
    assert!((efficiency - 83.33).abs() < 0.1);

    // Test with zero duration
    metric.sleep_start = test_timestamp();
    metric.sleep_end = test_timestamp();
    let efficiency = metric.calculate_efficiency();
    assert_eq!(efficiency, 0.0);
}

fn create_valid_activity_metric() -> ActivityMetric {
    ActivityMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(5),
        active_energy_burned_kcal: Some(500.0),
        basal_energy_burned_kcal: Some(1800.0),
        distance_cycling_meters: Some(20000.0),
        distance_swimming_meters: Some(1000.0),
        distance_wheelchair_meters: Some(0.0),
        distance_downhill_snow_sports_meters: Some(0.0),
        push_count: Some(0),
        swimming_stroke_count: Some(400),
        nike_fuel_points: Some(2500),
        apple_exercise_time_minutes: Some(60),
        apple_stand_time_minutes: Some(720),
        apple_move_time_minutes: Some(1440),
        apple_stand_hour_achieved: Some(true),
        walking_speed_m_per_s: Some(1.5),
        walking_step_length_cm: Some(75.0),
        walking_asymmetry_percent: Some(2.5),
        walking_double_support_percent: Some(25.0),
        six_minute_walk_test_distance_m: Some(500.0),
        stair_ascent_speed_m_per_s: Some(0.8),
        stair_descent_speed_m_per_s: Some(1.2),
        ground_contact_time_ms: Some(250.0),
        vertical_oscillation_cm: Some(8.5),
        running_stride_length_m: Some(1.8),
        running_power_watts: Some(280.0),
        running_speed_m_per_s: Some(4.2),
        cycling_speed_kmh: Some(25.0),
        cycling_power_watts: Some(200.0),
        cycling_cadence_rpm: Some(80.0),
        functional_threshold_power_watts: Some(250.0),
        underwater_depth_meters: Some(0.0),
        diving_duration_seconds: Some(0),
        source_device: Some("Apple Watch".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_activity_metric_validation_success() {
    let metric = create_valid_activity_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_step_count_validation() {
    let config = ValidationConfig::default();
    let mut metric = create_valid_activity_metric();

    // Test valid step count at boundaries
    metric.step_count = Some(config.step_count_min);
    assert!(metric.validate_with_config(&config).is_ok());

    metric.step_count = Some(config.step_count_max);
    assert!(metric.validate_with_config(&config).is_ok());

    // Test invalid step count (below minimum)
    metric.step_count = Some(config.step_count_min - 1);
    assert!(metric.validate_with_config(&config).is_err());

    // Test invalid step count (above maximum)
    metric.step_count = Some(config.step_count_max + 1);
    assert!(metric.validate_with_config(&config).is_err());

    // Test None step count (should be valid)
    metric.step_count = None;
    assert!(metric.validate_with_config(&config).is_ok());
}

fn create_valid_respiratory_metric() -> RespiratoryMetric {
    RespiratoryMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        respiratory_rate: Some(16),
        oxygen_saturation: Some(98.0),
        forced_vital_capacity: Some(4.5),
        forced_expiratory_volume_1: Some(3.8),
        peak_expiratory_flow_rate: Some(450.0),
        inhaler_usage: Some(2),
        source_device: Some("Pulse Oximeter".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_respiratory_metric_validation_success() {
    let metric = create_valid_respiratory_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_is_critical_method() {
    let config = ValidationConfig::default();
    let mut metric = create_valid_respiratory_metric();

    // Test critical oxygen saturation
    metric.oxygen_saturation = Some(config.oxygen_saturation_critical - 1.0);
    assert!(metric.is_critical(&config));

    // Test non-critical oxygen saturation
    metric.oxygen_saturation = Some(config.oxygen_saturation_critical + 1.0);
    assert!(!metric.is_critical(&config));

    // Test critical respiratory rate (too low)
    metric.respiratory_rate = Some(7);
    assert!(metric.is_critical(&config));

    // Test critical respiratory rate (too high)
    metric.respiratory_rate = Some(31);
    assert!(metric.is_critical(&config));

    // Test normal respiratory rate
    metric.respiratory_rate = Some(16);
    assert!(!metric.is_critical(&config));
}

#[test]
fn test_is_critical_condition_method() {
    let mut metric = create_valid_respiratory_metric();

    // Test critical SpO2 condition
    metric.oxygen_saturation = Some(89.0);
    assert!(metric.is_critical_condition());

    // Test non-critical SpO2 condition
    metric.oxygen_saturation = Some(95.0);
    assert!(!metric.is_critical_condition());

    // Test critical respiratory rate conditions
    metric.respiratory_rate = Some(7);
    assert!(metric.is_critical_condition());

    metric.respiratory_rate = Some(31);
    assert!(metric.is_critical_condition());

    // Test normal respiratory rate
    metric.respiratory_rate = Some(16);
    assert!(!metric.is_critical_condition());
}

fn create_valid_blood_glucose_metric() -> BloodGlucoseMetric {
    BloodGlucoseMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        blood_glucose_mg_dl: 100.0,
        measurement_context: Some("fasting".to_string()),
        medication_taken: Some(false),
        insulin_delivery_units: Some(0.0),
        glucose_source: Some("CGM".to_string()),
        source_device: Some("FreeStyle Libre".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_blood_glucose_metric_validation_success() {
    let metric = create_valid_blood_glucose_metric();
    assert!(metric.validate().is_ok());
    assert!(metric
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_is_critical_glucose_level() {
    let mut metric = create_valid_blood_glucose_metric();

    // Test critical low glucose
    metric.blood_glucose_mg_dl = 65.0;
    assert!(metric.is_critical_glucose_level());

    // Test critical high glucose
    metric.blood_glucose_mg_dl = 450.0;
    assert!(metric.is_critical_glucose_level());

    // Test normal glucose
    metric.blood_glucose_mg_dl = 100.0;
    assert!(!metric.is_critical_glucose_level());

    // Test boundary conditions
    metric.blood_glucose_mg_dl = 70.0;
    assert!(!metric.is_critical_glucose_level());

    metric.blood_glucose_mg_dl = 69.9;
    assert!(metric.is_critical_glucose_level());

    metric.blood_glucose_mg_dl = 400.0;
    assert!(!metric.is_critical_glucose_level());

    metric.blood_glucose_mg_dl = 400.1;
    assert!(metric.is_critical_glucose_level());
}

#[test]
fn test_glucose_category() {
    let mut metric = create_valid_blood_glucose_metric();

    // Test hypoglycemic critical
    metric.blood_glucose_mg_dl = 65.0;
    assert_eq!(metric.glucose_category(), "hypoglycemic_critical");

    // Test normal fasting
    metric.blood_glucose_mg_dl = 85.0;
    assert_eq!(metric.glucose_category(), "normal_fasting");

    // Test pre-diabetic
    metric.blood_glucose_mg_dl = 110.0;
    assert_eq!(metric.glucose_category(), "pre_diabetic");

    // Test diabetic controlled
    metric.blood_glucose_mg_dl = 150.0;
    assert_eq!(metric.glucose_category(), "diabetic_controlled");

    // Test diabetic uncontrolled
    metric.blood_glucose_mg_dl = 250.0;
    assert_eq!(metric.glucose_category(), "diabetic_uncontrolled");

    // Test medical emergency
    metric.blood_glucose_mg_dl = 450.0;
    assert_eq!(metric.glucose_category(), "medical_emergency");
}

fn create_valid_workout_data() -> WorkoutData {
    let start_time = test_timestamp() - chrono::Duration::hours(1);
    let end_time = test_timestamp();

    WorkoutData {
        id: test_user_id(),
        user_id: test_user_id(),
        workout_type: WorkoutType::Running,
        started_at: start_time,
        ended_at: end_time,
        total_energy_kcal: Some(500.0),
        active_energy_kcal: Some(450.0),
        distance_meters: Some(8000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source_device: Some("Apple Watch".to_string()),
        created_at: test_timestamp(),
    }
}

#[test]
fn test_workout_data_validation_success() {
    let workout = create_valid_workout_data();
    assert!(workout.validate().is_ok());
    assert!(workout
        .validate_with_config(&ValidationConfig::default())
        .is_ok());
}

#[test]
fn test_workout_time_validation() {
    let mut workout = create_valid_workout_data();

    // Test invalid: ended_at <= started_at
    workout.ended_at = workout.started_at;
    assert!(workout.validate().is_err());

    workout.ended_at = workout.started_at - chrono::Duration::minutes(1);
    assert!(workout.validate().is_err());

    // Test valid: ended_at > started_at
    workout.ended_at = workout.started_at + chrono::Duration::hours(1);
    assert!(workout.validate().is_ok());
}

#[test]
fn test_workout_logical_constraints() {
    let mut workout = create_valid_workout_data();

    // Test max_heart_rate < avg_heart_rate (invalid)
    workout.avg_heart_rate = Some(170);
    workout.max_heart_rate = Some(160);
    assert!(workout.validate().is_err());

    // Test active_energy > total_energy (invalid)
    workout.avg_heart_rate = Some(150);
    workout.max_heart_rate = Some(180);
    workout.total_energy_kcal = Some(400.0);
    workout.active_energy_kcal = Some(500.0);
    assert!(workout.validate().is_err());

    // Test valid relationship
    workout.total_energy_kcal = Some(500.0);
    workout.active_energy_kcal = Some(400.0);
    assert!(workout.validate().is_ok());
}

#[test]
fn test_validation_config_validation() {
    let mut config = ValidationConfig::default();

    // Test valid config
    assert!(config.validate().is_ok());

    // Test invalid config (heart rate min >= max)
    config.heart_rate_min = 100;
    config.heart_rate_max = 100;
    assert!(config.validate().is_err());

    config.heart_rate_max = 99;
    assert!(config.validate().is_err());

    // Reset and test blood pressure validation
    config = ValidationConfig::default();
    config.systolic_min = 150;
    config.systolic_max = 150;
    assert!(config.validate().is_err());

    // Test sleep efficiency validation
    config = ValidationConfig::default();
    config.sleep_efficiency_min = 100.0;
    config.sleep_efficiency_max = 50.0;
    assert!(config.validate().is_err());
}

#[test]
fn test_serialization_and_deserialization() {
    let heart_rate = create_valid_heart_rate_metric();
    let blood_pressure = create_valid_blood_pressure_metric();
    let sleep = create_valid_sleep_metric();
    let activity = create_valid_activity_metric();
    let respiratory = create_valid_respiratory_metric();
    let glucose = create_valid_blood_glucose_metric();
    let workout = create_valid_workout_data();

    // Test serialization
    let hr_serialized = serde_json::to_string(&heart_rate).unwrap();
    let bp_serialized = serde_json::to_string(&blood_pressure).unwrap();
    let sleep_serialized = serde_json::to_string(&sleep).unwrap();
    let activity_serialized = serde_json::to_string(&activity).unwrap();
    let respiratory_serialized = serde_json::to_string(&respiratory).unwrap();
    let glucose_serialized = serde_json::to_string(&glucose).unwrap();
    let workout_serialized = serde_json::to_string(&workout).unwrap();

    // Test deserialization
    let hr_deserialized: HeartRateMetric = serde_json::from_str(&hr_serialized).unwrap();
    let bp_deserialized: BloodPressureMetric = serde_json::from_str(&bp_serialized).unwrap();
    let sleep_deserialized: SleepMetric = serde_json::from_str(&sleep_serialized).unwrap();
    let activity_deserialized: ActivityMetric = serde_json::from_str(&activity_serialized).unwrap();
    let respiratory_deserialized: RespiratoryMetric =
        serde_json::from_str(&respiratory_serialized).unwrap();
    let glucose_deserialized: BloodGlucoseMetric =
        serde_json::from_str(&glucose_serialized).unwrap();
    let workout_deserialized: WorkoutData = serde_json::from_str(&workout_serialized).unwrap();

    // Verify deserialized data matches original
    assert_eq!(hr_deserialized.heart_rate, heart_rate.heart_rate);
    assert_eq!(bp_deserialized.systolic, blood_pressure.systolic);
    assert_eq!(sleep_deserialized.efficiency, sleep.efficiency);
    assert_eq!(activity_deserialized.step_count, activity.step_count);
    assert_eq!(
        respiratory_deserialized.respiratory_rate,
        respiratory.respiratory_rate
    );
    assert_eq!(
        glucose_deserialized.blood_glucose_mg_dl,
        glucose.blood_glucose_mg_dl
    );
    assert_eq!(workout_deserialized.workout_type, workout.workout_type);
}

#[test]
fn test_extreme_boundary_conditions() {
    let config = ValidationConfig::default();

    // Test heart rate at exact boundaries
    let mut heart_rate = HeartRateMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        heart_rate: Some(config.heart_rate_min),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: test_timestamp(),
    };

    assert!(heart_rate.validate_with_config(&config).is_ok());

    heart_rate.heart_rate = Some(config.heart_rate_max);
    assert!(heart_rate.validate_with_config(&config).is_ok());

    heart_rate.heart_rate = Some(config.heart_rate_min - 1);
    assert!(heart_rate.validate_with_config(&config).is_err());

    heart_rate.heart_rate = Some(config.heart_rate_max + 1);
    assert!(heart_rate.validate_with_config(&config).is_err());
}

#[test]
fn test_custom_validation_config() {
    let custom_config = custom_validation_config();

    let mut heart_rate = HeartRateMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        heart_rate: Some(25), // Valid with custom config, invalid with default
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: test_timestamp(),
    };

    // Should be valid with custom config
    assert!(heart_rate.validate_with_config(&custom_config).is_ok());

    // Should be invalid with default config
    assert!(heart_rate.validate().is_err());
}

#[test]
fn test_none_values_validation() {
    // Test that metrics with all None optional values validate successfully
    let heart_rate = HeartRateMetric {
        id: test_user_id(),
        user_id: test_user_id(),
        recorded_at: test_timestamp(),
        heart_rate: None,
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: test_timestamp(),
    };

    assert!(heart_rate.validate().is_ok());

    let mut activity = create_valid_activity_metric();
    activity.step_count = None;
    activity.distance_meters = None;
    activity.flights_climbed = None;
    activity.active_energy_burned_kcal = None;
    activity.basal_energy_burned_kcal = None;

    assert!(activity.validate().is_ok());
}
