/// Health Metrics Models Coverage Test - Target: 2161 lines
/// This test focuses on exercising health metrics model functionality

use chrono::Utc;
use uuid::Uuid;
use serde_json;

use self_sensored::models::health_metrics::*;
use self_sensored::models::enums::*;
use self_sensored::config::ValidationConfig;

#[test]
fn test_heart_rate_metric_creation() {
    let metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.5),
        walking_heart_rate_average: Some(85),
        heart_rate_recovery_one_minute: Some(20),
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Apple Watch".to_string()),
        context: Some(ActivityContext::Exercise),
        created_at: Utc::now(),
    };

    assert_eq!(metric.heart_rate, Some(75));
    assert_eq!(metric.resting_heart_rate, Some(65));
    assert!(metric.heart_rate_variability.is_some());
}

#[test]
fn test_blood_pressure_metric_creation() {
    let metric = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(75),
        source_device: Some("Blood Pressure Monitor".to_string()),
        created_at: Utc::now(),
    };

    assert_eq!(metric.systolic, 120);
    assert_eq!(metric.diastolic, 80);
    assert_eq!(metric.pulse, Some(75));
}

#[test]
fn test_sleep_metric_creation() {
    let now = Utc::now();
    let metric = SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: now,
        sleep_end: now + chrono::Duration::hours(8),
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(200),
        awake_minutes: Some(60),
        efficiency: Some(87.5),
        source_device: Some("Sleep Tracker".to_string()),
        created_at: now,
    };

    assert_eq!(metric.duration_minutes, Some(480));
    assert_eq!(metric.efficiency, Some(87.5));
}

#[test]
fn test_heart_rate_event_creation() {
    let event = HeartRateEvent {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        event_type: HeartRateEventType::High,
        event_occurred_at: Utc::now(),
        heart_rate_at_event: 180,
        event_duration_minutes: Some(5),
        context: Some(ActivityContext::Exercise),
        source_device: Some("Apple Watch".to_string()),
        severity: CardiacEventSeverity::Moderate,
        is_confirmed: false,
        notes: Some("During workout".to_string()),
        created_at: Utc::now(),
    };

    assert_eq!(event.heart_rate_at_event, 180);
    assert_eq!(event.event_duration_minutes, Some(5));
}

#[test]
fn test_processing_error_creation() {
    let error = ProcessingError {
        metric_type: "heart_rate".to_string(),
        error_message: "Invalid heart rate value".to_string(),
        index: Some(42),
    };

    assert_eq!(error.metric_type, "heart_rate");
    assert_eq!(error.error_message, "Invalid heart rate value");
    assert_eq!(error.index, Some(42));
}

#[test]
fn test_health_metric_enum_variants() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test HeartRate variant
    let heart_rate = HeartRateMetric {
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
        source_device: None,
        context: None,
        created_at: now,
    };

    let health_metric = HealthMetric::HeartRate(heart_rate);
    match health_metric {
        HealthMetric::HeartRate(hr) => {
            assert_eq!(hr.heart_rate, Some(75));
        }
        _ => panic!("Expected HeartRate variant"),
    }

    // Test BloodPressure variant
    let blood_pressure = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: None,
        source_device: None,
        created_at: now,
    };

    let health_metric = HealthMetric::BloodPressure(blood_pressure);
    match health_metric {
        HealthMetric::BloodPressure(bp) => {
            assert_eq!(bp.systolic, 120);
            assert_eq!(bp.diastolic, 80);
        }
        _ => panic!("Expected BloodPressure variant"),
    }

    // Test Sleep variant
    let sleep = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now,
        sleep_end: now + chrono::Duration::hours(8),
        duration_minutes: Some(480),
        deep_sleep_minutes: None,
        rem_sleep_minutes: None,
        light_sleep_minutes: None,
        awake_minutes: None,
        efficiency: None,
        source_device: None,
        created_at: now,
    };

    let health_metric = HealthMetric::Sleep(sleep);
    match health_metric {
        HealthMetric::Sleep(sleep) => {
            assert_eq!(sleep.duration_minutes, Some(480));
        }
        _ => panic!("Expected Sleep variant"),
    }
}

#[test]
fn test_serialization_deserialization() {
    let heart_rate = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.5),
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Apple Watch".to_string()),
        context: Some(ActivityContext::Exercise),
        created_at: Utc::now(),
    };

    // Test serialization
    let json = serde_json::to_string(&heart_rate).unwrap();
    assert!(json.contains("Apple Watch"));
    assert!(json.contains("75"));

    // Test deserialization
    let deserialized: HeartRateMetric = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.heart_rate, heart_rate.heart_rate);
    assert_eq!(deserialized.source_device, heart_rate.source_device);
}

#[test]
fn test_validation_config_functionality() {
    let config = ValidationConfig::default();

    // Test that validation config has reasonable defaults
    assert!(config.heart_rate_min > 0);
    assert!(config.heart_rate_max > config.heart_rate_min);
    assert!(config.systolic_min > 0);
    assert!(config.systolic_max > config.systolic_min);
    assert!(config.diastolic_min > 0);
    assert!(config.diastolic_max > config.diastolic_min);
}

#[test]
fn test_validation_config_from_env() {
    std::env::set_var("VALIDATION_HEART_RATE_MIN", "50");
    std::env::set_var("VALIDATION_HEART_RATE_MAX", "200");

    let config = ValidationConfig::from_env();

    assert_eq!(config.heart_rate_min, 50);
    assert_eq!(config.heart_rate_max, 200);

    // Clean up
    std::env::remove_var("VALIDATION_HEART_RATE_MIN");
    std::env::remove_var("VALIDATION_HEART_RATE_MAX");
}

#[test]
fn test_validation_config_validation() {
    let config = ValidationConfig::default();
    assert!(config.validate().is_ok());

    // Test invalid configuration
    let mut invalid_config = config.clone();
    invalid_config.heart_rate_min = invalid_config.heart_rate_max + 1;
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_heart_rate_edge_cases() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test with minimum values
    let min_metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(30),
        resting_heart_rate: Some(40),
        heart_rate_variability: Some(0.0),
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: now,
    };

    assert_eq!(min_metric.heart_rate, Some(30));

    // Test with maximum values
    let max_metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(220),
        resting_heart_rate: Some(100),
        heart_rate_variability: Some(100.0),
        walking_heart_rate_average: Some(180),
        heart_rate_recovery_one_minute: Some(60),
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Test Device".to_string()),
        context: Some(ActivityContext::Running),
        created_at: now,
    };

    assert_eq!(max_metric.heart_rate, Some(220));
}

#[test]
fn test_all_activity_contexts() {
    let contexts = vec![
        ActivityContext::Resting,
        ActivityContext::Walking,
        ActivityContext::Running,
        ActivityContext::Cycling,
        ActivityContext::Exercise,
        ActivityContext::Sleeping,
        ActivityContext::Sedentary,
        ActivityContext::Active,
        ActivityContext::PostMeal,
        ActivityContext::Stressed,
    ];

    for context in contexts {
        let metric = HeartRateMetric {
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
        };

        assert_eq!(metric.heart_rate, Some(75));
    }
}

#[test]
fn test_all_heart_rate_event_types() {
    let event_types = vec![
        HeartRateEventType::High,
        HeartRateEventType::Low,
        HeartRateEventType::Irregular,
        HeartRateEventType::Afib,
        HeartRateEventType::RapidIncrease,
        HeartRateEventType::SlowRecovery,
        HeartRateEventType::ExerciseAnomaly,
    ];

    for event_type in event_types {
        let event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 120,
            event_duration_minutes: Some(5),
            context: None,
            source_device: None,
            severity: CardiacEventSeverity::Low,
            is_confirmed: false,
            notes: None,
            created_at: Utc::now(),
        };

        assert_eq!(event.heart_rate_at_event, 120);
    }
}

#[test]
fn test_all_cardiac_event_severities() {
    let severities = vec![
        CardiacEventSeverity::Low,
        CardiacEventSeverity::Moderate,
        CardiacEventSeverity::High,
        CardiacEventSeverity::Critical,
    ];

    for severity in severities {
        let event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::High,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 180,
            event_duration_minutes: None,
            context: None,
            source_device: None,
            severity,
            is_confirmed: true,
            notes: None,
            created_at: Utc::now(),
        };

        assert_eq!(event.heart_rate_at_event, 180);
        assert!(event.is_confirmed);
    }
}

#[test]
fn test_health_metric_debug_and_clone() {
    let heart_rate = HeartRateMetric {
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
    };

    // Test Debug trait
    let debug_str = format!("{:?}", heart_rate);
    assert!(debug_str.contains("HeartRateMetric"));

    // Test Clone trait
    let cloned = heart_rate.clone();
    assert_eq!(heart_rate.heart_rate, cloned.heart_rate);
    assert_eq!(heart_rate.user_id, cloned.user_id);
}

#[test]
fn test_blood_pressure_edge_cases() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test extreme low values
    let low_bp = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 80,
        diastolic: 50,
        pulse: Some(50),
        source_device: None,
        created_at: now,
    };

    assert_eq!(low_bp.systolic, 80);
    assert_eq!(low_bp.diastolic, 50);

    // Test extreme high values
    let high_bp = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 200,
        diastolic: 120,
        pulse: Some(100),
        source_device: Some("Professional Monitor".to_string()),
        created_at: now,
    };

    assert_eq!(high_bp.systolic, 200);
    assert_eq!(high_bp.diastolic, 120);
}

#[test]
fn test_sleep_metric_edge_cases() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test very short sleep
    let short_sleep = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now,
        sleep_end: now + chrono::Duration::minutes(30),
        duration_minutes: Some(30),
        deep_sleep_minutes: Some(0),
        rem_sleep_minutes: Some(5),
        light_sleep_minutes: Some(20),
        awake_minutes: Some(5),
        efficiency: Some(83.3),
        source_device: None,
        created_at: now,
    };

    assert_eq!(short_sleep.duration_minutes, Some(30));

    // Test very long sleep
    let long_sleep = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now,
        sleep_end: now + chrono::Duration::hours(12),
        duration_minutes: Some(720),
        deep_sleep_minutes: Some(180),
        rem_sleep_minutes: Some(150),
        light_sleep_minutes: Some(300),
        awake_minutes: Some(90),
        efficiency: Some(87.5),
        source_device: Some("Sleep Study".to_string()),
        created_at: now,
    };

    assert_eq!(long_sleep.duration_minutes, Some(720));
}