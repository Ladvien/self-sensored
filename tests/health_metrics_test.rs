mod common;

use chrono::Utc;
use rust_decimal::Decimal;
use serde_json;
use std::str::FromStr;
use uuid::Uuid;

use common::fixtures::{
    create_minimal_activity_metric, create_test_blood_pressure_metric,
    create_test_heart_rate_metric, create_test_sleep_metric, create_test_workout_metric,
};
use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{ActivityContext, WorkoutType};
use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, HeartRateMetric, SleepMetric, WorkoutData,
};

/// Test heart rate metric validation with valid values
#[test]
fn test_heart_rate_metric_valid_validation() {
    let mut metric = create_test_heart_rate_metric();
    metric.heart_rate = Some(75); // Valid heart rate
    metric.resting_heart_rate = Some(65); // Valid resting heart rate
    metric.heart_rate_variability = Some(45.0); // Valid HRV

    assert!(metric.validate().is_ok(), "Valid heart rate metric should pass validation");
}

/// Test heart rate metric validation with invalid values
#[test]
fn test_heart_rate_metric_invalid_validation() {
    let config = ValidationConfig::default();

    // Test invalid heart rate - too low
    let mut metric = create_test_heart_rate_metric();
    metric.heart_rate = Some(10); // Below minimum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Heart rate below minimum should fail validation"
    );

    // Test invalid heart rate - too high
    metric.heart_rate = Some(350); // Above maximum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Heart rate above maximum should fail validation"
    );

    // Test invalid resting heart rate
    metric.heart_rate = Some(75); // Reset to valid
    metric.resting_heart_rate = Some(10); // Below minimum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Resting heart rate below minimum should fail validation"
    );

    // Test invalid heart rate variability
    metric.resting_heart_rate = Some(65); // Reset to valid
    metric.heart_rate_variability = Some(-5.0); // Negative value
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Negative heart rate variability should fail validation"
    );
}

/// Test heart rate metric serialization and deserialization
#[test]
fn test_heart_rate_metric_serialization() {
    let mut metric = create_test_heart_rate_metric();
    metric.heart_rate = Some(72);
    metric.context = Some(ActivityContext::Exercise);
    metric.walking_heart_rate_average = Some(85);
    metric.vo2_max_ml_kg_min = Some(Decimal::from_str("45.5").unwrap());

    // Test serialization
    let json = serde_json::to_string(&metric).expect("Should serialize");
    assert!(!json.is_empty());
    assert!(json.contains("\"heart_rate\":72"));
    assert!(json.contains("exercise"));

    // Test deserialization
    let deserialized: HeartRateMetric = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.heart_rate, Some(72));
    assert_eq!(deserialized.context, Some(ActivityContext::Exercise));
    assert_eq!(deserialized.walking_heart_rate_average, Some(85));
    assert_eq!(deserialized.user_id, metric.user_id);
}

/// Test blood pressure metric validation with valid values
#[test]
fn test_blood_pressure_metric_valid_validation() {
    let mut metric = create_test_blood_pressure_metric();
    metric.systolic = 120; // Valid systolic
    metric.diastolic = 80;  // Valid diastolic
    metric.pulse = Some(70); // Valid pulse

    assert!(metric.validate().is_ok(), "Valid blood pressure metric should pass validation");
}

/// Test blood pressure metric validation with invalid values
#[test]
fn test_blood_pressure_metric_invalid_validation() {
    let config = ValidationConfig::default();

    // Test invalid systolic - too low
    let mut metric = create_test_blood_pressure_metric();
    metric.systolic = 40; // Below minimum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Systolic below minimum should fail validation"
    );

    // Test invalid systolic - too high
    metric.systolic = 300; // Above maximum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Systolic above maximum should fail validation"
    );

    // Test invalid diastolic - too low
    metric.systolic = 120; // Reset to valid
    metric.diastolic = 20; // Below minimum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Diastolic below minimum should fail validation"
    );

    // Test invalid diastolic - too high
    metric.diastolic = 200; // Above maximum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Diastolic above maximum should fail validation"
    );

    // Test systolic lower than diastolic (physiologically impossible)
    metric.systolic = 70;
    metric.diastolic = 90;
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Systolic lower than diastolic should fail validation"
    );
}

/// Test blood pressure metric serialization and deserialization
#[test]
fn test_blood_pressure_metric_serialization() {
    let mut metric = create_test_blood_pressure_metric();
    metric.systolic = 135;
    metric.diastolic = 85;
    metric.pulse = Some(68);

    // Test serialization
    let json = serde_json::to_string(&metric).expect("Should serialize");
    assert!(!json.is_empty());
    assert!(json.contains("\"systolic\":135"));
    assert!(json.contains("\"diastolic\":85"));
    assert!(json.contains("\"pulse\":68"));

    // Test deserialization
    let deserialized: BloodPressureMetric = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.systolic, 135);
    assert_eq!(deserialized.diastolic, 85);
    assert_eq!(deserialized.pulse, Some(68));
    assert_eq!(deserialized.user_id, metric.user_id);
}

/// Test sleep metric validation with valid values
#[test]
fn test_sleep_metric_valid_validation() {
    let now = Utc::now();
    let mut metric = create_test_sleep_metric();
    metric.sleep_start = now - chrono::Duration::hours(8);
    metric.sleep_end = now;
    metric.duration_minutes = Some(480); // 8 hours
    metric.efficiency = Some(85.0); // Valid efficiency

    assert!(metric.validate().is_ok(), "Valid sleep metric should pass validation");
}

/// Test sleep metric validation with invalid values
#[test]
fn test_sleep_metric_invalid_validation() {
    let config = ValidationConfig::default();
    let now = Utc::now();

    // Test invalid sleep timing - end before start
    let mut metric = create_test_sleep_metric();
    metric.sleep_start = now;
    metric.sleep_end = now - chrono::Duration::hours(1); // End before start
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Sleep end before start should fail validation"
    );

    // Test invalid efficiency - too low
    metric.sleep_start = now - chrono::Duration::hours(8);
    metric.sleep_end = now; // Reset to valid
    metric.efficiency = Some(-5.0); // Negative efficiency
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Negative sleep efficiency should fail validation"
    );

    // Test invalid efficiency - too high
    metric.efficiency = Some(150.0); // Above 100%
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Sleep efficiency above 100% should fail validation"
    );

    // Test sleep component total exceeding calculated duration
    metric.efficiency = Some(85.0); // Reset to valid
    metric.deep_sleep_minutes = Some(300); // Large value
    metric.rem_sleep_minutes = Some(300); // Large value
    metric.awake_minutes = Some(300); // Large value - total 900 minutes > 8 hours (480 min)
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Sleep components exceeding total duration should fail validation"
    );
}

/// Test sleep metric serialization and deserialization
#[test]
fn test_sleep_metric_serialization() {
    let now = Utc::now();
    let mut metric = create_test_sleep_metric();
    metric.sleep_start = now - chrono::Duration::hours(7);
    metric.sleep_end = now;
    metric.duration_minutes = Some(420);
    metric.efficiency = Some(92.5);

    // Test serialization
    let json = serde_json::to_string(&metric).expect("Should serialize");
    assert!(!json.is_empty());
    assert!(json.contains("\"duration_minutes\":420"));
    assert!(json.contains("\"efficiency\":92.5"));

    // Test deserialization
    let deserialized: SleepMetric = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.duration_minutes, Some(420));
    assert_eq!(deserialized.efficiency, Some(92.5));
    assert_eq!(deserialized.user_id, metric.user_id);
}

/// Test activity metric validation with valid values
#[test]
fn test_activity_metric_valid_validation() {
    let user_id = Uuid::new_v4();
    let mut metric = create_minimal_activity_metric(user_id);
    metric.step_count = Some(10000);
    metric.distance_meters = Some(8000.0);
    metric.active_energy_burned_kcal = Some(500.0);
    metric.basal_energy_burned_kcal = Some(1800.0);

    assert!(metric.validate().is_ok(), "Valid activity metric should pass validation");
}

/// Test activity metric validation with invalid values
#[test]
fn test_activity_metric_invalid_validation() {
    let config = ValidationConfig::default();
    let user_id = Uuid::new_v4();

    // Test invalid step count - negative
    let mut metric = create_minimal_activity_metric(user_id);
    metric.step_count = Some(-1000); // Negative steps
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Negative step count should fail validation"
    );

    // Test invalid step count - too high
    metric.step_count = Some(250000); // Above maximum
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Step count above maximum should fail validation"
    );

    // Test invalid distance - negative
    metric.step_count = Some(10000); // Reset to valid
    metric.distance_meters = Some(-500.0); // Negative distance
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Negative distance should fail validation"
    );

    // Test invalid energy - negative
    metric.distance_meters = Some(8000.0); // Reset to valid
    metric.active_energy_burned_kcal = Some(-200.0); // Negative energy
    assert!(
        metric.validate_with_config(&config).is_err(),
        "Negative active energy should fail validation"
    );
}

/// Test activity metric serialization and deserialization
#[test]
fn test_activity_metric_serialization() {
    let user_id = Uuid::new_v4();
    let mut metric = create_minimal_activity_metric(user_id);
    metric.step_count = Some(12000);
    metric.distance_meters = Some(9500.0);
    metric.flights_climbed = Some(25);
    metric.active_energy_burned_kcal = Some(650.0);
    metric.cycling_speed_kmh = Some(22.5);

    // Test serialization
    let json = serde_json::to_string(&metric).expect("Should serialize");
    assert!(!json.is_empty());
    assert!(json.contains("\"step_count\":12000"));
    assert!(json.contains("\"distance_meters\":9500.0"));
    assert!(json.contains("\"flights_climbed\":25"));

    // Test deserialization
    let deserialized: ActivityMetric = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.step_count, Some(12000));
    assert_eq!(deserialized.distance_meters, Some(9500.0));
    assert_eq!(deserialized.flights_climbed, Some(25));
    assert_eq!(deserialized.user_id, user_id);
}

/// Test workout data validation with valid values
#[test]
fn test_workout_data_valid_validation() {
    let mut workout = create_test_workout_metric();
    workout.workout_type = WorkoutType::Running;
    workout.total_energy_kcal = Some(450.0);
    workout.distance_meters = Some(5000.0);
    workout.avg_heart_rate = Some(145);
    workout.max_heart_rate = Some(175);

    assert!(workout.validate().is_ok(), "Valid workout data should pass validation");
}

/// Test workout data validation with invalid values
#[test]
fn test_workout_data_invalid_validation() {
    let config = ValidationConfig::default();
    let now = Utc::now();

    // Test invalid workout timing - end before start
    let mut workout = create_test_workout_metric();
    workout.started_at = now;
    workout.ended_at = now - chrono::Duration::hours(1); // End before start
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Workout end before start should fail validation"
    );

    // Test invalid duration - too long
    workout.started_at = now - chrono::Duration::hours(30);
    workout.ended_at = now; // 30 hours duration
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Workout duration above maximum should fail validation"
    );

    // Test invalid energy - negative
    workout.started_at = now - chrono::Duration::hours(1);
    workout.ended_at = now; // Reset to valid duration
    workout.total_energy_kcal = Some(-200.0); // Negative energy
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Negative total energy should fail validation"
    );

    // Test invalid heart rate - too low
    workout.total_energy_kcal = Some(450.0); // Reset to valid
    workout.avg_heart_rate = Some(10); // Below minimum
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Average heart rate below minimum should fail validation"
    );

    // Test invalid heart rate - too high
    workout.avg_heart_rate = Some(350); // Above maximum
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Average heart rate above maximum should fail validation"
    );
}

/// Test workout data serialization and deserialization
#[test]
fn test_workout_data_serialization() {
    let mut workout = create_test_workout_metric();
    workout.workout_type = WorkoutType::Cycling;
    workout.total_energy_kcal = Some(650.0);
    workout.distance_meters = Some(15000.0);
    workout.avg_heart_rate = Some(155);

    // Test serialization
    let json = serde_json::to_string(&workout).expect("Should serialize");
    assert!(!json.is_empty());
    assert!(json.contains("\"total_energy_kcal\":650.0"));
    assert!(json.contains("\"distance_meters\":15000.0"));

    // Test deserialization
    let deserialized: WorkoutData = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.workout_type, WorkoutType::Cycling);
    assert_eq!(deserialized.total_energy_kcal, Some(650.0));
    assert_eq!(deserialized.distance_meters, Some(15000.0));
    assert_eq!(deserialized.user_id, workout.user_id);
}

/// Test edge cases for all metric types
#[test]
fn test_metric_edge_cases() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test heart rate with None values
    let hr_metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: None, // None value should be valid
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        context: None,
        source_device: None,
        created_at: now,
    };
    assert!(hr_metric.validate().is_ok(), "Heart rate metric with None values should be valid");

    // Test activity metric with only minimal fields
    let minimal_activity = create_minimal_activity_metric(user_id);
    assert!(minimal_activity.validate().is_ok(), "Minimal activity metric should be valid");

    // Test sleep with minimal duration that matches calculated duration
    let mut minimal_sleep = create_test_sleep_metric();
    minimal_sleep.user_id = user_id;
    minimal_sleep.sleep_start = now - chrono::Duration::minutes(30);
    minimal_sleep.sleep_end = now;
    minimal_sleep.duration_minutes = Some(30); // Very short sleep
    minimal_sleep.deep_sleep_minutes = Some(10); // Ensure components don't exceed duration
    minimal_sleep.rem_sleep_minutes = Some(10);
    minimal_sleep.light_sleep_minutes = Some(5);
    minimal_sleep.awake_minutes = Some(5); // Total: 30 minutes
    assert!(minimal_sleep.validate().is_ok(), "Short sleep duration should be valid");
}

/// Test validation with custom configuration
#[test]
fn test_metric_validation_with_custom_config() {
    let config = ValidationConfig {
        heart_rate_min: 30,
        heart_rate_max: 250, // Lower maximum
        systolic_min: 60,
        systolic_max: 200, // Lower maximum
        workout_max_duration_hours: 12, // Shorter maximum
        ..ValidationConfig::default()
    };

    // Test heart rate with custom limits
    let mut hr_metric = create_test_heart_rate_metric();
    hr_metric.heart_rate = Some(270); // Would be invalid with custom config
    assert!(
        hr_metric.validate_with_config(&config).is_err(),
        "Heart rate above custom maximum should fail"
    );

    // Test blood pressure with custom limits
    let mut bp_metric = create_test_blood_pressure_metric();
    bp_metric.systolic = 220; // Would be invalid with custom config
    assert!(
        bp_metric.validate_with_config(&config).is_err(),
        "Systolic above custom maximum should fail"
    );

    // Test workout with custom duration limit
    let now = Utc::now();
    let mut workout = create_test_workout_metric();
    workout.started_at = now - chrono::Duration::hours(15);
    workout.ended_at = now; // 15 hours, above custom limit
    assert!(
        workout.validate_with_config(&config).is_err(),
        "Workout above custom duration limit should fail"
    );
}

/// Test metric cloning and equality
#[test]
fn test_metric_cloning() {
    let original_hr = create_test_heart_rate_metric();
    let cloned_hr = original_hr.clone();
    assert_eq!(original_hr.id, cloned_hr.id);
    assert_eq!(original_hr.user_id, cloned_hr.user_id);
    assert_eq!(original_hr.heart_rate, cloned_hr.heart_rate);

    let original_bp = create_test_blood_pressure_metric();
    let cloned_bp = original_bp.clone();
    assert_eq!(original_bp.systolic, cloned_bp.systolic);
    assert_eq!(original_bp.diastolic, cloned_bp.diastolic);

    let user_id = Uuid::new_v4();
    let original_activity = create_minimal_activity_metric(user_id);
    let cloned_activity = original_activity.clone();
    assert_eq!(original_activity.user_id, cloned_activity.user_id);
    assert_eq!(original_activity.recorded_at, cloned_activity.recorded_at);
}

/// Test metric debug formatting
#[test]
fn test_metric_debug_formatting() {
    let hr_metric = create_test_heart_rate_metric();
    let debug_str = format!("{:?}", hr_metric);
    assert!(debug_str.contains("HeartRateMetric"));
    assert!(debug_str.contains("user_id"));

    let bp_metric = create_test_blood_pressure_metric();
    let debug_str = format!("{:?}", bp_metric);
    assert!(debug_str.contains("BloodPressureMetric"));
    assert!(debug_str.contains("systolic"));

    let sleep_metric = create_test_sleep_metric();
    let debug_str = format!("{:?}", sleep_metric);
    assert!(debug_str.contains("SleepMetric"));
    assert!(debug_str.contains("sleep_start"));
}

/// Test complex scenario with all metric types
#[test]
fn test_complex_multi_metric_scenario() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Create a comprehensive set of metrics for a single user
    let mut hr_metric = create_test_heart_rate_metric();
    hr_metric.user_id = user_id;
    hr_metric.recorded_at = now;
    hr_metric.heart_rate = Some(85);
    hr_metric.context = Some(ActivityContext::Exercise);

    let mut bp_metric = create_test_blood_pressure_metric();
    bp_metric.user_id = user_id;
    bp_metric.recorded_at = now - chrono::Duration::minutes(30);

    let mut activity_metric = create_minimal_activity_metric(user_id);
    activity_metric.recorded_at = now - chrono::Duration::hours(1);
    activity_metric.step_count = Some(15000);
    activity_metric.active_energy_burned_kcal = Some(800.0);

    let mut sleep_metric = create_test_sleep_metric();
    sleep_metric.user_id = user_id;
    sleep_metric.sleep_start = now - chrono::Duration::hours(16);
    sleep_metric.sleep_end = now - chrono::Duration::hours(8);

    let mut workout = create_test_workout_metric();
    workout.user_id = user_id;
    workout.started_at = now - chrono::Duration::hours(2);
    workout.ended_at = now - chrono::Duration::hours(1);

    // Validate all metrics
    assert!(hr_metric.validate().is_ok());
    assert!(bp_metric.validate().is_ok());
    assert!(activity_metric.validate().is_ok());
    assert!(sleep_metric.validate().is_ok());
    assert!(workout.validate().is_ok());

    // Test serialization of all metrics
    let metrics = vec![
        serde_json::to_value(&hr_metric).unwrap(),
        serde_json::to_value(&bp_metric).unwrap(),
        serde_json::to_value(&activity_metric).unwrap(),
        serde_json::to_value(&sleep_metric).unwrap(),
        serde_json::to_value(&workout).unwrap(),
    ];

    assert_eq!(metrics.len(), 5);
    for metric_value in metrics {
        assert!(metric_value.get("user_id").is_some());
        assert!(metric_value.get("id").is_some());
    }
}