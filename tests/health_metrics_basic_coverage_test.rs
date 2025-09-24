use chrono::Utc;
use self_sensored::models::enums::ActivityContext;
use self_sensored::models::health_metrics::*;
use serde_json;
use uuid::Uuid;

#[test]
fn test_heart_rate_metric_creation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

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
    assert_eq!(metric.resting_heart_rate, Some(65));
}

#[test]
fn test_blood_pressure_metric_creation() {
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

    assert_eq!(metric.user_id, user_id);
    assert_eq!(metric.systolic, 120);
    assert_eq!(metric.diastolic, 80);
    assert_eq!(metric.pulse, Some(75));
}

#[test]
fn test_sleep_metric_creation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let sleep_end = now + chrono::Duration::hours(8);

    let metric = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now,
        sleep_end,
        duration_minutes: Some(480), // 8 hours
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(200),
        awake_minutes: Some(60),
        efficiency: Some(87.5),
        source_device: Some("Sleep Tracker".to_string()),
        created_at: now,
    };

    assert_eq!(metric.user_id, user_id);
    assert_eq!(metric.duration_minutes, Some(480));
    assert_eq!(metric.efficiency, Some(87.5));
}

#[test]
fn test_activity_metric_creation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(10),
        active_energy_burned_kcal: Some(500.0),
        basal_energy_burned_kcal: Some(1800.0),
        distance_cycling_meters: Some(0.0),
        distance_swimming_meters: Some(0.0),
        distance_wheelchair_meters: Some(0.0),
        distance_downhill_snow_sports_meters: Some(0.0),
        push_count: Some(0),
        swimming_stroke_count: Some(0),
        nike_fuel_points: Some(2500),
        apple_exercise_time_minutes: Some(30),
        apple_stand_time_minutes: Some(600),
        apple_move_time_minutes: Some(480),
        apple_stand_hour_achieved: Some(true),
        // Mobility Metrics
        walking_speed_m_per_s: None,
        walking_step_length_cm: None,
        walking_asymmetry_percent: None,
        walking_double_support_percent: None,
        six_minute_walk_test_distance_m: None,
        // Stair Metrics
        stair_ascent_speed_m_per_s: None,
        stair_descent_speed_m_per_s: None,
        // Running Dynamics
        ground_contact_time_ms: None,
        vertical_oscillation_cm: None,
        running_stride_length_m: None,
        running_power_watts: None,
        running_speed_m_per_s: None,
        // Cycling Metrics
        cycling_speed_kmh: None,
        cycling_power_watts: None,
        cycling_cadence_rpm: None,
        functional_threshold_power_watts: None,
        // Underwater Metrics
        underwater_depth_meters: None,
        diving_duration_seconds: None,
        source_device: Some("iPhone".to_string()),
        created_at: now,
    };

    assert_eq!(metric.user_id, user_id);
    assert_eq!(metric.step_count, Some(10000));
    assert_eq!(metric.distance_meters, Some(8000.0));
}

#[test]
fn test_heart_rate_event_creation() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let event = HeartRateEvent {
        id: Uuid::new_v4(),
        user_id,
        event_type: self_sensored::models::enums::HeartRateEventType::High,
        event_occurred_at: now,
        heart_rate_at_event: 180,
        event_duration_minutes: Some(5),
        context: Some(ActivityContext::Exercise),
        source_device: Some("Apple Watch".to_string()),
        severity: self_sensored::models::enums::CardiacEventSeverity::Moderate,
        is_confirmed: false,
        notes: Some("During intense workout".to_string()),
        created_at: now,
    };

    assert_eq!(event.user_id, user_id);
    assert_eq!(event.heart_rate_at_event, 180);
    assert_eq!(event.event_duration_minutes, Some(5));
}

#[test]
fn test_health_metrics_serialization() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let heart_rate = HeartRateMetric {
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

    // Test serialization
    let json = serde_json::to_string(&heart_rate).unwrap();
    assert!(json.contains("Apple Watch"));
    assert!(json.contains("75"));

    // Test deserialization
    let deserialized: HeartRateMetric = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.user_id, heart_rate.user_id);
    assert_eq!(deserialized.heart_rate, heart_rate.heart_rate);
}
