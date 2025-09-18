use chrono::{DateTime, Utc};
use serde_json::json;

use self_sensored::models::{HealthMetric, Workout};

#[test]
fn test_heart_rate_validation() {
    let valid_heart_rate = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 75,
        context: Some("rest".to_string()),
        confidence: Some(0.95),
    };

    assert!(valid_heart_rate.validate().is_ok());

    // Test invalid heart rates
    let invalid_low = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 10, // Too low
        context: Some("test".to_string()),
        confidence: Some(0.95),
    };
    assert!(invalid_low.validate().is_err());

    let invalid_high = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 350, // Too high
        context: Some("test".to_string()),
        confidence: Some(0.95),
    };
    assert!(invalid_high.validate().is_err());

    // Test edge cases
    let min_valid = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 30, // Minimum valid
        context: Some("test".to_string()),
        confidence: Some(0.95),
    };
    assert!(min_valid.validate().is_ok());

    let max_valid = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 250, // Maximum valid
        context: Some("test".to_string()),
        confidence: Some(0.95),
    };
    assert!(max_valid.validate().is_ok());
}

#[test]
fn test_blood_pressure_validation() {
    let valid_bp = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
    };
    assert!(valid_bp.validate().is_ok());

    // Test invalid systolic
    let invalid_systolic = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 300, // Too high
        diastolic: 80,
        pulse: Some(72),
    };
    assert!(invalid_systolic.validate().is_err());

    // Test invalid diastolic
    let invalid_diastolic = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 200, // Too high
        pulse: Some(72),
    };
    assert!(invalid_diastolic.validate().is_err());

    // Test systolic <= diastolic (invalid)
    let invalid_ratio = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 80,
        diastolic: 120, // Higher than systolic
        pulse: Some(72),
    };
    assert!(invalid_ratio.validate().is_err());

    // Test edge cases
    let low_valid = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 70, // Low but valid
        diastolic: 50,
        pulse: Some(60),
    };
    assert!(low_valid.validate().is_ok());

    let high_valid = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 180, // High but valid
        diastolic: 120,
        pulse: Some(100),
    };
    assert!(high_valid.validate().is_ok());
}

#[test]
fn test_sleep_validation() {
    let valid_sleep = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 480, // 8 hours
        sleep_stage: Some("deep".to_string()),
        efficiency: Some(0.85),
    };
    assert!(valid_sleep.validate().is_ok());

    // Test invalid duration (too short)
    let too_short = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 5, // Too short
        sleep_stage: Some("light".to_string()),
        efficiency: Some(0.85),
    };
    assert!(too_short.validate().is_err());

    // Test invalid duration (too long)
    let too_long = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 1500, // 25 hours
        sleep_stage: Some("deep".to_string()),
        efficiency: Some(0.85),
    };
    assert!(too_long.validate().is_err());

    // Test invalid efficiency
    let invalid_efficiency = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 480,
        sleep_stage: Some("rem".to_string()),
        efficiency: Some(1.5), // > 1.0
    };
    assert!(invalid_efficiency.validate().is_err());

    // Test valid sleep stages
    let valid_stages = ["light", "deep", "rem", "awake"];
    for stage in valid_stages {
        let sleep_metric = HealthMetric::Sleep {
            recorded_at: Utc::now(),
            duration_minutes: 300,
            sleep_stage: Some(stage.to_string()),
            efficiency: Some(0.80),
        };
        assert!(
            sleep_metric.validate().is_ok(),
            "Stage '{}' should be valid",
            stage
        );
    }

    // Test invalid sleep stage
    let invalid_stage = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 300,
        sleep_stage: Some("invalid_stage".to_string()),
        efficiency: Some(0.80),
    };
    assert!(invalid_stage.validate().is_err());
}

#[test]
fn test_activity_validation() {
    let valid_activity = HealthMetric::Activity {
        recorded_at: Utc::now(),
        activity_type: "walking".to_string(),
        duration_minutes: 30,
        calories_burned: Some(150),
        distance_meters: Some(2000),
    };
    assert!(valid_activity.validate().is_ok());

    // Test invalid duration
    let invalid_duration = HealthMetric::Activity {
        recorded_at: Utc::now(),
        activity_type: "running".to_string(),
        duration_minutes: 0, // Invalid
        calories_burned: Some(300),
        distance_meters: Some(5000),
    };
    assert!(invalid_duration.validate().is_err());

    // Test negative calories
    let negative_calories = HealthMetric::Activity {
        recorded_at: Utc::now(),
        activity_type: "cycling".to_string(),
        duration_minutes: 45,
        calories_burned: Some(-100), // Invalid
        distance_meters: Some(10000),
    };
    assert!(negative_calories.validate().is_err());

    // Test negative distance
    let negative_distance = HealthMetric::Activity {
        recorded_at: Utc::now(),
        activity_type: "swimming".to_string(),
        duration_minutes: 60,
        calories_burned: Some(400),
        distance_meters: Some(-1000), // Invalid
    };
    assert!(negative_distance.validate().is_err());

    // Test valid activity types
    let valid_types = [
        "walking",
        "running",
        "cycling",
        "swimming",
        "weightlifting",
        "yoga",
        "other",
    ];
    for activity_type in valid_types {
        let activity = HealthMetric::Activity {
            recorded_at: Utc::now(),
            activity_type: activity_type.to_string(),
            duration_minutes: 30,
            calories_burned: Some(100),
            distance_meters: Some(1000),
        };
        assert!(
            activity.validate().is_ok(),
            "Activity type '{}' should be valid",
            activity_type
        );
    }
}

#[test]
fn test_metric_type_identification() {
    let heart_rate = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 75,
        context: None,
        confidence: None,
    };
    assert_eq!(heart_rate.metric_type(), "HeartRate");

    let blood_pressure = HealthMetric::BloodPressure {
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: None,
    };
    assert_eq!(blood_pressure.metric_type(), "BloodPressure");

    let sleep = HealthMetric::Sleep {
        recorded_at: Utc::now(),
        duration_minutes: 480,
        sleep_stage: None,
        efficiency: None,
    };
    assert_eq!(sleep.metric_type(), "Sleep");

    let activity = HealthMetric::Activity {
        recorded_at: Utc::now(),
        activity_type: "walking".to_string(),
        duration_minutes: 30,
        calories_burned: None,
        distance_meters: None,
    };
    assert_eq!(activity.metric_type(), "Activity");
}

#[test]
fn test_workout_validation() {
    let valid_workout = Workout {
        workout_type: "running".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 60,
        calories_burned: Some(500),
        distance_meters: Some(10000),
        route_data: Some(vec![(37.7749, -122.4194), (37.7849, -122.4094)]),
    };
    assert!(valid_workout.validate().is_ok());

    // Test invalid duration (ended before started)
    let invalid_time = Workout {
        workout_type: "cycling".to_string(),
        started_at: Utc::now(),
        ended_at: Utc::now() - chrono::Duration::hours(1), // Ended before started
        duration_minutes: 60,
        calories_burned: Some(400),
        distance_meters: Some(15000),
        route_data: None,
    };
    assert!(invalid_time.validate().is_err());

    // Test negative calories
    let negative_calories = Workout {
        workout_type: "swimming".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 60,
        calories_burned: Some(-100), // Invalid
        distance_meters: Some(2000),
        route_data: None,
    };
    assert!(negative_calories.validate().is_err());

    // Test invalid workout type
    let invalid_type = Workout {
        workout_type: "".to_string(), // Empty
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 60,
        calories_burned: Some(300),
        distance_meters: Some(5000),
        route_data: None,
    };
    assert!(invalid_type.validate().is_err());

    // Test valid workout types
    let valid_types = [
        "running",
        "cycling",
        "swimming",
        "walking",
        "weightlifting",
        "yoga",
        "other",
    ];
    for workout_type in valid_types {
        let workout = Workout {
            workout_type: workout_type.to_string(),
            started_at: Utc::now() - chrono::Duration::hours(1),
            ended_at: Utc::now(),
            duration_minutes: 60,
            calories_burned: Some(200),
            distance_meters: Some(3000),
            route_data: None,
        };
        assert!(
            workout.validate().is_ok(),
            "Workout type '{}' should be valid",
            workout_type
        );
    }
}

#[test]
fn test_route_data_validation() {
    // Valid route with multiple points
    let valid_route = Workout {
        workout_type: "running".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 60,
        calories_burned: Some(500),
        distance_meters: Some(10000),
        route_data: Some(vec![
            (37.7749, -122.4194), // San Francisco
            (37.7849, -122.4094),
            (37.7949, -122.3994),
        ]),
    };
    assert!(valid_route.validate().is_ok());

    // Test invalid coordinates (latitude out of range)
    let invalid_lat = Workout {
        workout_type: "cycling".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 45,
        calories_burned: Some(300),
        distance_meters: Some(15000),
        route_data: Some(vec![(91.0, -122.4194)]), // Invalid latitude > 90
    };
    assert!(invalid_lat.validate().is_err());

    // Test invalid coordinates (longitude out of range)
    let invalid_lon = Workout {
        workout_type: "walking".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 30,
        calories_burned: Some(100),
        distance_meters: Some(2000),
        route_data: Some(vec![(37.7749, -181.0)]), // Invalid longitude < -180
    };
    assert!(invalid_lon.validate().is_err());

    // Test single point route (should be valid)
    let single_point = Workout {
        workout_type: "yoga".to_string(),
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        duration_minutes: 60,
        calories_burned: Some(200),
        distance_meters: Some(0),
        route_data: Some(vec![(37.7749, -122.4194)]),
    };
    assert!(single_point.validate().is_ok());
}

#[test]
fn test_health_metric_serialization() {
    let heart_rate = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 75,
        context: Some("rest".to_string()),
        confidence: Some(0.95),
    };

    let serialized = serde_json::to_string(&heart_rate).unwrap();
    assert!(serialized.contains("HeartRate"));
    assert!(serialized.contains("75"));
    assert!(serialized.contains("rest"));

    let deserialized: HealthMetric = serde_json::from_str(&serialized).unwrap();
    match deserialized {
        HealthMetric::HeartRate { heart_rate: 75, .. } => (),
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
    let future_heart_rate = HealthMetric::HeartRate {
        recorded_at: Utc::now() + chrono::Duration::days(1), // Future date
        heart_rate: 75,
        context: Some("test".to_string()),
        confidence: Some(0.95),
    };
    assert!(future_heart_rate.validate().is_err());

    // Test with very old date (should be valid but flagged)
    let old_heart_rate = HealthMetric::HeartRate {
        recorded_at: Utc::now() - chrono::Duration::days(365 * 2), // 2 years ago
        heart_rate: 75,
        context: Some("historical".to_string()),
        confidence: Some(0.95),
    };
    // Should be valid (historical data is allowed)
    assert!(old_heart_rate.validate().is_ok());
}

#[test]
fn test_confidence_validation() {
    // Valid confidence values
    let valid_confidences = [0.0, 0.5, 0.95, 1.0];

    for confidence in valid_confidences {
        let heart_rate = HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 75,
            context: Some("test".to_string()),
            confidence: Some(confidence),
        };
        assert!(
            heart_rate.validate().is_ok(),
            "Confidence {} should be valid",
            confidence
        );
    }

    // Invalid confidence values
    let invalid_confidences = [-0.1, 1.1, 2.0];

    for confidence in invalid_confidences {
        let heart_rate = HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 75,
            context: Some("test".to_string()),
            confidence: Some(confidence),
        };
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
    let valid_contexts = ["rest", "exercise", "sleep", "stress", "recovery"];

    for context in valid_contexts {
        let heart_rate = HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 75,
            context: Some(context.to_string()),
            confidence: Some(0.95),
        };
        assert!(
            heart_rate.validate().is_ok(),
            "Context '{}' should be valid",
            context
        );
    }

    // Empty context should be invalid
    let empty_context = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 75,
        context: Some("".to_string()),
        confidence: Some(0.95),
    };
    assert!(empty_context.validate().is_err());

    // Very long context should be invalid
    let long_context = HealthMetric::HeartRate {
        recorded_at: Utc::now(),
        heart_rate: 75,
        context: Some("a".repeat(1000)), // 1000 characters
        confidence: Some(0.95),
    };
    assert!(long_context.validate().is_err());
}

#[test]
fn test_batch_validation() {
    let metrics = vec![
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 75,
            context: Some("rest".to_string()),
            confidence: Some(0.95),
        },
        HealthMetric::BloodPressure {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
        },
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 400, // Invalid
            context: Some("test".to_string()),
            confidence: Some(0.95),
        },
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
