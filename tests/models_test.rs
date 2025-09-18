mod common;

use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use common::fixtures::{
    create_minimal_activity_metric, create_test_blood_pressure_metric,
    create_test_heart_rate_metric, create_test_sleep_metric, create_test_workout_metric,
};
use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, HealthMetric, HeartRateMetric, IngestData, IngestPayload,
    IosIngestData, IosIngestPayload, IosMetric, IosMetricData, SleepMetric, WorkoutData,
    WorkoutType,
};

#[test]
fn test_standard_payload_serialization() {
    let user_id = Uuid::new_v4();

    // Create test metrics using fixtures
    let mut heart_rate_metric = create_test_heart_rate_metric();
    heart_rate_metric.user_id = user_id;
    heart_rate_metric.heart_rate = 75;

    let mut blood_pressure_metric = create_test_blood_pressure_metric();
    blood_pressure_metric.user_id = user_id;

    let mut activity_metric = create_minimal_activity_metric(user_id);
    activity_metric.step_count = Some(10000);
    activity_metric.distance_meters = Some(8000.0);
    activity_metric.flights_climbed = Some(15);
    activity_metric.active_energy_burned_kcal = Some(500.0);
    activity_metric.basal_energy_burned_kcal = Some(1800.0);

    let mut sleep_metric = create_test_sleep_metric();
    sleep_metric.user_id = user_id;

    let mut workout_metric = create_test_workout_metric();
    workout_metric.user_id = user_id;
    workout_metric.workout_type = "Running".to_string();

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate(heart_rate_metric),
                HealthMetric::BloodPressure(blood_pressure_metric),
                HealthMetric::Activity(activity_metric),
                HealthMetric::Sleep(sleep_metric),
            ],
            workouts: vec![workout_metric],
        },
    };

    // Test serialization
    let json_str = serde_json::to_string(&payload).expect("Should serialize");
    assert!(!json_str.is_empty());

    // Test deserialization
    let deserialized: IngestPayload = serde_json::from_str(&json_str).expect("Should deserialize");

    assert_eq!(deserialized.data.metrics.len(), 4);
    assert_eq!(deserialized.data.workouts.len(), 1);
}

#[test]
fn test_ios_payload_conversion() {
    let now = Utc::now();
    let date_str = now.to_rfc3339();

    let ios_payload = IosIngestPayload {
        data: IosIngestData {
            metrics: vec![
                IosMetric {
                    name: "heart_rate".to_string(),
                    units: Some("bpm".to_string()),
                    data: vec![IosMetricData {
                        qty: Some(75.0),
                        date: Some(date_str.clone()),
                        start: None,
                        end: None,
                        source: Some("Apple Watch".to_string()),
                        value: None,
                        extra: HashMap::new(),
                    }],
                },
                IosMetric {
                    name: "blood_pressure_systolic".to_string(),
                    units: Some("mmHg".to_string()),
                    data: vec![IosMetricData {
                        qty: Some(120.0),
                        date: Some(date_str.clone()),
                        start: None,
                        end: None,
                        source: Some("Manual".to_string()),
                        value: None,
                        extra: HashMap::new(),
                    }],
                },
                IosMetric {
                    name: "blood_pressure_diastolic".to_string(),
                    units: Some("mmHg".to_string()),
                    data: vec![IosMetricData {
                        qty: Some(80.0),
                        date: Some(date_str.clone()),
                        start: None,
                        end: None,
                        source: Some("Manual".to_string()),
                        value: None,
                        extra: HashMap::new(),
                    }],
                },
            ],
            workouts: vec![],
        },
    };

    // Test conversion to internal format
    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should have heart rate and blood pressure metrics
    assert!(!internal_payload.data.metrics.is_empty());

    // Check that we have some heart rate metrics
    let hr_metrics: Vec<&HealthMetric> = internal_payload
        .data
        .metrics
        .iter()
        .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
        .collect();

    assert_eq!(hr_metrics.len(), 1, "Should have heart rate metric");

    if let HealthMetric::HeartRate(hr) = &hr_metrics[0] {
        assert_eq!(hr.heart_rate, 75);
    }
}

#[test]
fn test_metric_validation() {
    let user_id = Uuid::new_v4();

    // Valid heart rate
    let mut valid_hr_metric = create_test_heart_rate_metric();
    valid_hr_metric.user_id = user_id;
    valid_hr_metric.heart_rate = 75;
    let valid_hr = HealthMetric::HeartRate(valid_hr_metric);
    assert!(valid_hr.validate().is_ok());

    // Invalid heart rate (too high)
    let mut invalid_hr_metric = create_test_heart_rate_metric();
    invalid_hr_metric.user_id = user_id;
    invalid_hr_metric.heart_rate = 400; // Invalid
    let invalid_hr = HealthMetric::HeartRate(invalid_hr_metric);
    assert!(invalid_hr.validate().is_err());

    // Valid blood pressure
    let mut valid_bp_metric = create_test_blood_pressure_metric();
    valid_bp_metric.user_id = user_id;
    let valid_bp = HealthMetric::BloodPressure(valid_bp_metric);
    assert!(valid_bp.validate().is_ok());

    // Invalid blood pressure
    let mut invalid_bp_metric = create_test_blood_pressure_metric();
    invalid_bp_metric.user_id = user_id;
    invalid_bp_metric.systolic = 300; // Invalid
    let invalid_bp = HealthMetric::BloodPressure(invalid_bp_metric);
    assert!(invalid_bp.validate().is_err());

    // Valid sleep
    let mut valid_sleep_metric = create_test_sleep_metric();
    valid_sleep_metric.user_id = user_id;
    let valid_sleep = HealthMetric::Sleep(valid_sleep_metric);
    assert!(valid_sleep.validate().is_ok());

    // Invalid sleep (end before start)
    let now = Utc::now();
    let mut invalid_sleep_metric = create_test_sleep_metric();
    invalid_sleep_metric.user_id = user_id;
    invalid_sleep_metric.sleep_start = now;
    invalid_sleep_metric.sleep_end = now - chrono::Duration::hours(1); // Invalid
    invalid_sleep_metric.efficiency = Some(150.0); // Also invalid
    let invalid_sleep = HealthMetric::Sleep(invalid_sleep_metric);
    assert!(invalid_sleep.validate().is_err());

    // Valid activity
    let mut valid_activity_metric = create_minimal_activity_metric(user_id);
    valid_activity_metric.step_count = Some(10000);
    valid_activity_metric.distance_meters = Some(8000.0);
    valid_activity_metric.flights_climbed = Some(10);
    valid_activity_metric.active_energy_burned_kcal = Some(2000.0);
    valid_activity_metric.basal_energy_burned_kcal = Some(1800.0);
    let valid_activity = HealthMetric::Activity(valid_activity_metric);
    assert!(valid_activity.validate().is_ok());

    // Invalid activity (negative values)
    let mut invalid_activity_metric = create_minimal_activity_metric(user_id);
    invalid_activity_metric.step_count = Some(-1000); // Invalid
    invalid_activity_metric.distance_meters = Some(-500.0); // Invalid
    invalid_activity_metric.active_energy_burned_kcal = Some(-200.0); // Invalid
    let invalid_activity = HealthMetric::Activity(invalid_activity_metric);
    assert!(invalid_activity.validate().is_err());
}

#[test]
fn test_large_payload_performance() {
    use std::time::Instant;

    let now = Utc::now();
    let user_id = Uuid::new_v4();
    let start = Instant::now();

    // Generate 1000 metrics - mix of all core types
    let mut metrics = Vec::new();
    for i in 0..1000 {
        let timestamp = now - chrono::Duration::minutes(i as i64);

        // Cycle through core metric types
        match i % 4 {
            0 => {
                metrics.push(HealthMetric::HeartRate(HeartRateMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: timestamp,
                    heart_rate: Some((70 + (i % 50)) as i16),
                    resting_heart_rate: Some(65),
                    heart_rate_variability: None,
                    walking_heart_rate_average: None,
                    heart_rate_recovery_one_minute: None,
                    atrial_fibrillation_burden_percentage: None,
                    vo2_max_ml_kg_min: None,
                    context: Some(ActivityContext::Resting),
                    source_device: Some("Performance Test".to_string()),
                    created_at: now,
                }));
            }
            1 => {
                metrics.push(HealthMetric::BloodPressure(BloodPressureMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: timestamp,
                    systolic: (110 + (i % 30)) as i16,
                    diastolic: (70 + (i % 20)) as i16,
                    pulse: Some(65 + (i % 40) as i16),
                    source_device: Some("Performance Test".to_string()),
                    created_at: now,
                }));
            }
            2 => {
                metrics.push(HealthMetric::Activity(ActivityMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: timestamp,
                    step_count: Some(5000 + (i % 10000)),
                    distance_meters: Some(3000.0 + (i % 5000) as f64),
                    flights_climbed: Some(i % 20_i32),
                    active_energy_burned_kcal: Some(300.0 + (i % 500) as f64),
                    basal_energy_burned_kcal: Some(1800.0 + (i % 200) as f64),
                    distance_cycling_meters: None,
                    distance_swimming_meters: None,
                    distance_wheelchair_meters: None,
                    distance_downhill_snow_sports_meters: None,
                    push_count: None,
                    swimming_stroke_count: None,
                    nike_fuel_points: None,
                    apple_exercise_time_minutes: None,
                    apple_stand_time_minutes: None,
                    apple_move_time_minutes: None,
                    apple_stand_hour_achieved: None,
                    source_device: Some("Performance Test".to_string()),
                    created_at: now,
                }));
            }
            _ => {
                // Sleep metrics less frequent
                if i % 24 == 0 {
                    metrics.push(HealthMetric::Sleep(SleepMetric {
                        id: Uuid::new_v4(),
                        user_id,
                        sleep_start: timestamp - chrono::Duration::hours(8),
                        sleep_end: timestamp,
                        duration_minutes: Some(420 + (i % 120)),
                        deep_sleep_minutes: Some(90 + (i % 60)),
                        rem_sleep_minutes: Some(60 + (i % 40)),
                        light_sleep_minutes: Some(240 + (i % 60)),
                        awake_minutes: Some(i % 30_i32),
                        efficiency: Some(80.0 + (i % 20) as f64),
                        source_device: Some("Performance Test".to_string()),
                        created_at: now,
                    }));
                }
            }
        }
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![], // Keep workouts empty for focused metric testing
        },
    };

    let generation_time = start.elapsed();
    println!(
        "Generated {} metrics in {:?}",
        payload.data.metrics.len(),
        generation_time
    );

    // Test serialization performance
    let start = Instant::now();
    let json_str = serde_json::to_string(&payload).expect("Should serialize");
    let serialization_time = start.elapsed();
    println!("Serialized in {serialization_time:?}");

    // Test deserialization performance
    let start = Instant::now();
    let _: IngestPayload = serde_json::from_str(&json_str).expect("Should deserialize");
    let deserialization_time = start.elapsed();
    println!("Deserialized in {deserialization_time:?}");

    // Performance assertions
    assert!(
        generation_time.as_millis() < 1000,
        "Should generate quickly"
    );
    assert!(
        serialization_time.as_millis() < 2000,
        "Should serialize quickly"
    );
    assert!(
        deserialization_time.as_millis() < 2000,
        "Should deserialize quickly"
    );

    // Size check
    assert!(
        json_str.len() < 10 * 1024 * 1024,
        "Should be reasonable size"
    );

    // Verify mix of metric types
    let heart_rate_count = payload
        .data
        .metrics
        .iter()
        .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
        .count();

    let bp_count = payload
        .data
        .metrics
        .iter()
        .filter(|m| matches!(m, HealthMetric::BloodPressure(_)))
        .count();

    let activity_count = payload
        .data
        .metrics
        .iter()
        .filter(|m| matches!(m, HealthMetric::Activity(_)))
        .count();

    assert!(heart_rate_count > 200);
    assert!(bp_count > 200);
    assert!(activity_count > 200);
}

#[test]
fn test_metric_type_identification() {
    let now = Utc::now();
    let user_id = Uuid::new_v4();

    let heart_rate_metric = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        context: Some(ActivityContext::Resting),
        source_device: Some("Test".to_string()),
        created_at: now,
    });

    let blood_pressure_metric = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(70),
        source_device: Some("Test".to_string()),
        created_at: now,
    });

    let sleep_metric = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now - chrono::Duration::hours(8),
        sleep_end: now,
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(90),
        light_sleep_minutes: Some(240),
        awake_minutes: Some(30),
        efficiency: Some(90.0),
        source_device: Some("Test".to_string()),
        created_at: now,
    });

    let activity_metric = HealthMetric::Activity(ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(15),
        active_energy_burned_kcal: Some(500.0),
        basal_energy_burned_kcal: Some(1800.0),
        distance_cycling_meters: None,
        distance_swimming_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,
        push_count: None,
        swimming_stroke_count: None,
        nike_fuel_points: None,
        apple_exercise_time_minutes: None,
        apple_stand_time_minutes: None,
        apple_move_time_minutes: None,
        apple_stand_hour_achieved: None,
        source_device: Some("Test".to_string()),
        created_at: now,
    });

    // Test metric type identification
    assert_eq!(heart_rate_metric.metric_type(), "HeartRate");
    assert_eq!(blood_pressure_metric.metric_type(), "BloodPressure");
    assert_eq!(sleep_metric.metric_type(), "Sleep");
    assert_eq!(activity_metric.metric_type(), "Activity");

    // Test that validation works through the enum
    assert!(heart_rate_metric.validate().is_ok());
    assert!(blood_pressure_metric.validate().is_ok());
    assert!(sleep_metric.validate().is_ok());
    assert!(activity_metric.validate().is_ok());
}
