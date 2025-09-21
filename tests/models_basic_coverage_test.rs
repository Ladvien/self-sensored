use chrono::Utc;
use uuid::Uuid;
use serde_json;

#[test]
fn test_heart_rate_metric_type_creation() {
    use self_sensored::models::health_metrics::HeartRateMetric;
    use self_sensored::models::enums::ActivityContext;

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Create HeartRateMetric with all required fields
    let metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(45.5),
        walking_heart_rate_average: Some(85),
        heart_rate_recovery_one_minute: Some(20),
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Apple Watch".to_string()),
        context: Some(ActivityContext::Exercise),
        created_at: now,
    };

    assert_eq!(metric.user_id, user_id);
    assert_eq!(metric.heart_rate, Some(75));
}

#[test]
fn test_blood_pressure_metric_type_creation() {
    use self_sensored::models::health_metrics::BloodPressureMetric;

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let metric = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(75),
        source_device: Some("Blood Pressure Monitor".to_string()),
        created_at: now,
    };

    assert_eq!(metric.systolic, 120);
    assert_eq!(metric.diastolic, 80);
}

#[test]
fn test_sleep_metric_type_creation() {
    use self_sensored::models::health_metrics::SleepMetric;

    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let sleep_end = now + chrono::Duration::hours(8);

    let metric = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now,
        sleep_end,
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
}

#[test]
fn test_heart_rate_event_type_creation() {
    use self_sensored::models::health_metrics::HeartRateEvent;
    use self_sensored::models::enums::{HeartRateEventType, ActivityContext, CardiacEventSeverity};

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let event = HeartRateEvent {
        id: Uuid::new_v4(),
        user_id,
        event_type: HeartRateEventType::High,
        event_occurred_at: now,
        heart_rate_at_event: 180,
        event_duration_minutes: Some(5),
        context: Some(ActivityContext::Exercise),
        source_device: Some("Apple Watch".to_string()),
        severity: CardiacEventSeverity::Moderate,
        is_confirmed: false,
        notes: Some("During intense workout".to_string()),
        created_at: now,
    };

    assert_eq!(event.heart_rate_at_event, 180);
}

#[test]
fn test_health_metrics_serialization() {
    use self_sensored::models::health_metrics::BloodPressureMetric;

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let metric = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(75),
        source_device: Some("Test Device".to_string()),
        created_at: now,
    };

    // Test serialization
    let json = serde_json::to_string(&metric).unwrap();
    assert!(json.contains("Test Device"));
    assert!(json.contains("120"));

    // Test deserialization
    let deserialized: BloodPressureMetric = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.systolic, metric.systolic);
}

#[test]
fn test_ios_models_types_exist() {
    // Test that iOS models module exists and can be imported
    use self_sensored::models::ios_models::IosIngestPayload;

    // Test passes if types can be referenced without errors
    let _type_exists = std::marker::PhantomData::<IosIngestPayload>;
    assert!(true);
}

#[test]
fn test_enums_functionality() {
    use self_sensored::models::enums::*;

    // Test enum variants exist and can be used
    let _context = ActivityContext::Exercise;
    let _severity = CardiacEventSeverity::Low;
    let _event_type = HeartRateEventType::High;

    // Test passes if compilation succeeds
    assert!(true);
}