use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::{
    health_metrics::{
        HealthMetric, WorkoutData, HeartRateMetric, BloodPressureMetric,
        SleepMetric, ActivityMetric,
    },
    enums::{ActivityContext, WorkoutType}
};

// Helper function to create a default ActivityMetric with all required fields
fn create_default_activity_metric() -> ActivityMetric {
    ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        step_count: None,
        distance_meters: None,
        flights_climbed: None,
        active_energy_burned_kcal: None,
        basal_energy_burned_kcal: None,
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
        walking_speed_m_per_s: None,
        walking_step_length_cm: None,
        walking_asymmetry_percent: None,
        walking_double_support_percent: None,
        six_minute_walk_test_distance_m: None,
        stair_ascent_speed_m_per_s: None,
        stair_descent_speed_m_per_s: None,
        ground_contact_time_ms: None,
        vertical_oscillation_cm: None,
        running_stride_length_m: None,
        running_power_watts: None,
        running_speed_m_per_s: None,
        cycling_speed_kmh: None,
        cycling_power_watts: None,
        cycling_cadence_rpm: None,
        functional_threshold_power_watts: None,
        underwater_depth_meters: None,
        diving_duration_seconds: None,
        source_device: None,
        created_at: Utc::now(),
    }
}

#[test]
fn test_heart_rate_validation() {
    let valid_heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Resting),
        created_at: Utc::now(),
    });

    assert!(valid_heart_rate.validate().is_ok());

    // Test invalid heart rates
    let invalid_low = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(10), // Too low
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    assert!(invalid_low.validate().is_err());

    let invalid_high = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(350), // Too high
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    assert!(invalid_high.validate().is_err());

    // Test edge cases
    let min_valid = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(30), // Minimum valid
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    assert!(min_valid.validate().is_ok());

    let max_valid = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(250), // Maximum valid
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    assert!(max_valid.validate().is_ok());
}

#[test]
fn test_blood_pressure_validation() {
    let valid_bp = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(valid_bp.validate().is_ok());

    // Test invalid systolic
    let invalid_systolic = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 300, // Too high
        diastolic: 80,
        pulse: Some(72),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(invalid_systolic.validate().is_err());

    // Test invalid diastolic
    let invalid_diastolic = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 200, // Too high
        pulse: Some(72),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(invalid_diastolic.validate().is_err());

    // Test systolic <= diastolic (invalid)
    let invalid_ratio = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 80,
        diastolic: 120, // Higher than systolic
        pulse: Some(72),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(invalid_ratio.validate().is_err());

    // Test edge cases
    let low_valid = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 70, // Low but valid
        diastolic: 50,
        pulse: Some(60),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(low_valid.validate().is_ok());

    let high_valid = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 180, // High but valid
        diastolic: 120,
        pulse: Some(100),
        source_device: None,
        created_at: Utc::now(),
    });
    assert!(high_valid.validate().is_ok());
}

#[test]
fn test_sleep_validation() {
    let now = Utc::now();
    let valid_sleep = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now - chrono::Duration::hours(8),
        sleep_end: now,
        duration_minutes: Some(480), // 8 hours
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(200),
        awake_minutes: Some(60),
        efficiency: Some(0.85),
        source_device: None,
        created_at: now,
    });
    assert!(valid_sleep.validate().is_ok());

    // Test invalid duration (too short)
    let too_short = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now - chrono::Duration::minutes(5),
        sleep_end: now,
        duration_minutes: Some(5), // Too short
        deep_sleep_minutes: Some(1),
        rem_sleep_minutes: Some(1),
        light_sleep_minutes: Some(2),
        awake_minutes: Some(1),
        efficiency: Some(0.85),
        source_device: None,
        created_at: now,
    });
    assert!(too_short.validate().is_err());

    // Test invalid duration (too long)
    let too_long = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now - chrono::Duration::minutes(1500),
        sleep_end: now,
        duration_minutes: Some(1500), // 25 hours
        deep_sleep_minutes: Some(300),
        rem_sleep_minutes: Some(200),
        light_sleep_minutes: Some(800),
        awake_minutes: Some(200),
        efficiency: Some(0.85),
        source_device: None,
        created_at: now,
    });
    assert!(too_long.validate().is_err());

    // Test invalid efficiency
    let invalid_efficiency = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now - chrono::Duration::hours(8),
        sleep_end: now,
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(200),
        awake_minutes: Some(60),
        efficiency: Some(1.5), // > 1.0
        source_device: None,
        created_at: now,
    });
    assert!(invalid_efficiency.validate().is_err());

    // Test valid sleep metrics with different stage combinations
    let valid_sleep_variants = vec![
        (Some(60), Some(40), Some(150), Some(50)), // deep, rem, light, awake
        (Some(0), Some(0), Some(280), Some(20)),   // mostly light sleep
        (Some(90), Some(60), Some(120), Some(30)), // balanced sleep
    ];

    for (deep, rem, light, awake) in valid_sleep_variants {
        let sleep_metric = HealthMetric::Sleep(SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: now - chrono::Duration::hours(5),
            sleep_end: now,
            duration_minutes: Some(300),
            deep_sleep_minutes: deep,
            rem_sleep_minutes: rem,
            light_sleep_minutes: light,
            awake_minutes: awake,
            efficiency: Some(0.80),
            source_device: None,
            created_at: now,
        });
        assert!(
            sleep_metric.validate().is_ok(),
            "Sleep stage combination should be valid"
        );
    }
}

#[test]
fn test_activity_validation() {
    let mut valid_activity = create_default_activity_metric();
    valid_activity.step_count = Some(3000);
    valid_activity.distance_meters = Some(2000.0);
    valid_activity.flights_climbed = Some(5);
    valid_activity.active_energy_burned_kcal = Some(150.0);
    valid_activity.basal_energy_burned_kcal = Some(80.0);
    valid_activity.apple_exercise_time_minutes = Some(30);
    valid_activity.apple_stand_time_minutes = Some(25);
    valid_activity.apple_move_time_minutes = Some(30);
    valid_activity.apple_stand_hour_achieved = Some(true);
    let valid_activity = HealthMetric::Activity(valid_activity);
    assert!(valid_activity.validate().is_ok());

    // Test invalid duration
    let mut invalid_duration = create_default_activity_metric();
    invalid_duration.step_count = Some(0); // Invalid - zero steps
    invalid_duration.distance_meters = Some(5000.0);
    invalid_duration.flights_climbed = Some(10);
    invalid_duration.active_energy_burned_kcal = Some(300.0);
    invalid_duration.basal_energy_burned_kcal = Some(100.0);
    invalid_duration.apple_exercise_time_minutes = Some(0); // Invalid duration
    invalid_duration.apple_stand_time_minutes = Some(0);
    invalid_duration.apple_move_time_minutes = Some(0);
    invalid_duration.apple_stand_hour_achieved = Some(false);
    let invalid_duration = HealthMetric::Activity(invalid_duration);
    assert!(invalid_duration.validate().is_err());

    // Test negative calories
    let mut negative_calories = create_default_activity_metric();
    negative_calories.step_count = Some(5000);
    negative_calories.distance_meters = Some(10000.0);
    negative_calories.flights_climbed = Some(15);
    negative_calories.active_energy_burned_kcal = Some(-100.0); // Invalid - negative calories
    negative_calories.basal_energy_burned_kcal = Some(150.0);
    negative_calories.distance_cycling_meters = Some(10000.0);
    negative_calories.apple_exercise_time_minutes = Some(45);
    negative_calories.apple_stand_time_minutes = Some(40);
    negative_calories.apple_move_time_minutes = Some(45);
    negative_calories.apple_stand_hour_achieved = Some(true);
    let negative_calories = HealthMetric::Activity(negative_calories);
    assert!(negative_calories.validate().is_err());

    // Test negative distance
    let mut negative_distance = create_default_activity_metric();
    negative_distance.step_count = Some(1000);
    negative_distance.distance_meters = Some(-1000.0); // Invalid - negative distance
    negative_distance.flights_climbed = Some(0);
    negative_distance.active_energy_burned_kcal = Some(400.0);
    negative_distance.basal_energy_burned_kcal = Some(200.0);
    negative_distance.distance_swimming_meters = Some(1000.0);
    negative_distance.swimming_stroke_count = Some(500);
    negative_distance.apple_exercise_time_minutes = Some(60);
    negative_distance.apple_stand_time_minutes = Some(50);
    negative_distance.apple_move_time_minutes = Some(60);
    negative_distance.apple_stand_hour_achieved = Some(true);
    let negative_distance = HealthMetric::Activity(negative_distance);
    assert!(negative_distance.validate().is_err());

    // Test valid activity metric combinations
    let valid_activities = vec![
        (Some(2000), Some(1500.0), Some(100.0)), // walking activity
        (Some(8000), Some(5000.0), Some(300.0)), // running activity
        (None, Some(15000.0), Some(400.0)),      // cycling activity
        (Some(500), Some(1000.0), Some(250.0)), // swimming activity
    ];

    for (steps, distance, calories) in valid_activities {
        let mut activity = create_default_activity_metric();
        activity.step_count = steps;
        activity.distance_meters = distance;
        activity.flights_climbed = Some(2);
        activity.active_energy_burned_kcal = calories;
        activity.basal_energy_burned_kcal = Some(50.0);
        activity.apple_exercise_time_minutes = Some(30);
        activity.apple_stand_time_minutes = Some(25);
        activity.apple_move_time_minutes = Some(30);
        activity.apple_stand_hour_achieved = Some(true);
        let activity = HealthMetric::Activity(activity);
        assert!(
            activity.validate().is_ok(),
            "Activity should be valid"
        );
    }
}

#[test]
fn test_metric_type_identification() {
    let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: Utc::now(),
    });
    assert_eq!(heart_rate.metric_type(), "HeartRate");

    let blood_pressure = HealthMetric::BloodPressure(BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: None,
        source_device: None,
        created_at: Utc::now(),
    });
    assert_eq!(blood_pressure.metric_type(), "BloodPressure");

    let now = Utc::now();
    let sleep = HealthMetric::Sleep(SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now - chrono::Duration::hours(8),
        sleep_end: now,
        duration_minutes: Some(480),
        deep_sleep_minutes: None,
        rem_sleep_minutes: None,
        light_sleep_minutes: None,
        awake_minutes: None,
        efficiency: None,
        source_device: None,
        created_at: now,
    });
    assert_eq!(sleep.metric_type(), "Sleep");

    let mut activity = create_default_activity_metric();
    activity.step_count = Some(2000);
    activity.apple_exercise_time_minutes = Some(30);
    let activity = HealthMetric::Activity(activity);
    assert_eq!(activity.metric_type(), "Activity");
}

#[test]
fn test_workout_validation() {
    let valid_workout = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Running,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(500.0),
        active_energy_kcal: Some(400.0),
        distance_meters: Some(10000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(valid_workout.validate().is_ok());

    // Test invalid duration (ended before started)
    let invalid_time = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Cycling,
        started_at: Utc::now(),
        ended_at: Utc::now() - chrono::Duration::hours(1), // Ended before started
        total_energy_kcal: Some(400.0),
        active_energy_kcal: Some(350.0),
        distance_meters: Some(15000.0),
        avg_heart_rate: Some(140),
        max_heart_rate: Some(170),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(invalid_time.validate().is_err());

    // Test negative calories
    let negative_calories = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Swimming,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(-100.0), // Invalid
        active_energy_kcal: Some(-80.0),
        distance_meters: Some(2000.0),
        avg_heart_rate: Some(120),
        max_heart_rate: Some(150),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(negative_calories.validate().is_err());

    // Test invalid workout type
    let invalid_type = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Other, // Use Other for now since we can't have empty
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(300.0),
        active_energy_kcal: Some(250.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: None, // Make invalid by having impossible heart rate
        max_heart_rate: Some(500), // Invalid heart rate > 300
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(invalid_type.validate().is_err());

    // Test valid workout types
    let valid_types = [
        WorkoutType::Running,
        WorkoutType::Cycling,
        WorkoutType::Swimming,
        WorkoutType::Walking,
        WorkoutType::StrengthTraining,
        WorkoutType::Yoga,
        WorkoutType::Other,
    ];
    for workout_type in valid_types {
        let workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type,
            started_at: Utc::now() - chrono::Duration::hours(1),
            ended_at: Utc::now(),
            total_energy_kcal: Some(200.0),
            active_energy_kcal: Some(160.0),
            distance_meters: Some(3000.0),
            avg_heart_rate: Some(130),
            max_heart_rate: Some(160),
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(
            workout.validate().is_ok(),
            "Workout type should be valid"
        );
    }
}

#[test]
fn test_route_data_validation() {
    // Valid route with multiple points (WorkoutData doesn't store route directly)
    let valid_route = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Running,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(500.0),
        active_energy_kcal: Some(450.0),
        distance_meters: Some(10000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(valid_route.validate().is_ok());

    // Test invalid heart rate (simulating validation error)
    let invalid_heart_rate = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Cycling,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(300.0),
        active_energy_kcal: Some(250.0),
        distance_meters: Some(15000.0),
        avg_heart_rate: Some(500), // Invalid heart rate > 300
        max_heart_rate: Some(520),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(invalid_heart_rate.validate().is_err());

    // Test invalid distance (negative distance)
    let invalid_distance = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Walking,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(100.0),
        active_energy_kcal: Some(80.0),
        distance_meters: Some(-2000.0), // Invalid negative distance
        avg_heart_rate: Some(90),
        max_heart_rate: Some(110),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(invalid_distance.validate().is_err());

    // Test stationary workout (should be valid)
    let stationary_workout = WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Yoga,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(200.0),
        active_energy_kcal: Some(150.0),
        distance_meters: Some(0.0), // Stationary workout
        avg_heart_rate: Some(80),
        max_heart_rate: Some(100),
        source_device: None,
        created_at: Utc::now(),
    };
    assert!(stationary_workout.validate().is_ok());
}

#[test]
fn test_health_metric_serialization() {
    let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Resting),
        created_at: Utc::now(),
    });

    let serialized = serde_json::to_string(&heart_rate).unwrap();
    assert!(serialized.contains("HeartRate"));
    assert!(serialized.contains("75"));

    let deserialized: HealthMetric = serde_json::from_str(&serialized).unwrap();
    match deserialized {
        HealthMetric::HeartRate(metric) if metric.heart_rate == Some(75) => (),
        _ => panic!("Deserialization failed"),
    }
}

#[test]
fn test_health_metric_deserialization_errors() {
    // Test invalid JSON structure
    let invalid_json = r#"{"invalid": "structure"}"#;
    let result: Result<HealthMetric, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());

    // Test missing required fields
    let missing_fields = r#"{"HeartRate": {"recorded_at": "2024-01-01T00:00:00Z"}}"#;
    let result: Result<HealthMetric, _> = serde_json::from_str(missing_fields);
    assert!(result.is_err());

    // Test invalid field types
    let invalid_types =
        r#"{"HeartRate": {"recorded_at": "2024-01-01T00:00:00Z", "heart_rate": "invalid"}}"#;
    let result: Result<HealthMetric, _> = serde_json::from_str(invalid_types);
    assert!(result.is_err());
}

#[test]
fn test_future_date_validation() {
    // Test with future date (should be invalid)
    let future_heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now() + chrono::Duration::days(1), // Future date
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    assert!(future_heart_rate.validate().is_err());

    // Test with very old date (should be valid but flagged)
    let old_heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now() - chrono::Duration::days(365 * 2), // 2 years ago
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Active),
        created_at: Utc::now(),
    });
    // Should be valid (historical data is allowed)
    assert!(old_heart_rate.validate().is_ok());
}

#[test]
fn test_confidence_validation() {
    // Valid confidence values
    let valid_confidences = [0.0, 0.5, 0.95, 1.0];

    for confidence in valid_confidences {
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(75),
            resting_heart_rate: None,
            heart_rate_variability: Some(confidence), // Using this field for confidence test
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: Some(ActivityContext::Active),
            created_at: Utc::now(),
        });
        assert!(
            heart_rate.validate().is_ok(),
            "Confidence {} should be valid",
            confidence
        );
    }

    // Invalid confidence values
    let invalid_confidences = [-0.1, 1.1, 2.0];

    for confidence in invalid_confidences {
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(75),
            resting_heart_rate: None,
            heart_rate_variability: Some(confidence), // Using this field for confidence test
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: Some(ActivityContext::Active),
            created_at: Utc::now(),
        });
        assert!(
            heart_rate.validate().is_err(),
            "Confidence {} should be invalid",
            confidence
        );
    }
}

#[test]
fn test_context_validation() {
    // Valid contexts for heart rate
    let valid_contexts = [
        ActivityContext::Resting,
        ActivityContext::Exercise,
        ActivityContext::Sleeping,
        ActivityContext::Stressed,
        ActivityContext::Active,
    ];

    for context in valid_contexts {
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(75),
            resting_heart_rate: None,
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: Some(context),
            created_at: Utc::now(),
        });
        assert!(
            heart_rate.validate().is_ok(),
            "Context should be valid"
        );
    }

    // None context should be valid (optional field)
    let none_context = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None, // None context should be valid
        created_at: Utc::now(),
    });
    assert!(none_context.validate().is_ok());

    // Test with extreme heart rate value (validation should catch this)
    let invalid_heart_rate = HealthMetric::HeartRate(HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(500), // Invalid heart rate
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: Some(ActivityContext::Resting),
        created_at: Utc::now(),
    });
    assert!(invalid_heart_rate.validate().is_err());
}

#[test]
fn test_batch_validation() {
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(75),
            resting_heart_rate: None,
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        }),
        HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source_device: None,
            created_at: Utc::now(),
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(400), // Invalid
            resting_heart_rate: None,
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: Some(ActivityContext::Active),
            created_at: Utc::now(),
        }),
    ];

    let mut valid_count = 0;
    let mut invalid_count = 0;

    for metric in &metrics {
        match metric.validate() {
            Ok(_) => valid_count += 1,
            Err(_) => invalid_count += 1,
        }
    }

    assert_eq!(valid_count, 2);
    assert_eq!(invalid_count, 1);
}
