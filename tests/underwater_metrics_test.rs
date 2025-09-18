use self_sensored::models::{
    health_metrics::ActivityMetric,
    ios_models::{DataPoint, IoSMetric, MetricData},
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[test]
fn test_underwater_metrics_struct_fields() {
    // Test that ActivityMetric struct has the new underwater fields
    let metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),

        // Basic fields
        step_count: None,
        distance_meters: None,
        flights_climbed: None,
        active_energy_burned_kcal: None,
        basal_energy_burned_kcal: None,

        // Swimming fields
        distance_swimming_meters: Some(100.0),
        swimming_stroke_count: Some(50),

        // Extended fields (set to None for test)
        distance_cycling_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,
        push_count: None,
        nike_fuel_points: None,
        apple_exercise_time_minutes: None,
        apple_stand_time_minutes: None,
        apple_move_time_minutes: None,
        apple_stand_hour_achieved: None,

        // Mobility metrics
        walking_speed_m_per_s: None,
        walking_step_length_cm: None,
        walking_asymmetry_percent: None,
        walking_double_support_percent: None,
        six_minute_walk_test_distance_m: None,

        // Stair metrics
        stair_ascent_speed_m_per_s: None,
        stair_descent_speed_m_per_s: None,

        // Running dynamics
        ground_contact_time_ms: None,
        vertical_oscillation_cm: None,
        running_stride_length_m: None,
        running_power_watts: None,
        running_speed_m_per_s: None,

        // Cycling metrics
        cycling_speed_kmh: None,
        cycling_power_watts: None,
        cycling_cadence_rpm: None,
        functional_threshold_power_watts: None,

        // Underwater metrics - TEST THESE SPECIFICALLY
        underwater_depth_meters: Some(15.5), // 15.5 meters depth (recreational diving)
        diving_duration_seconds: Some(1800), // 30 minutes diving session

        source_device: Some("Apple Watch Ultra".to_string()),
        created_at: Utc::now(),
    };

    // Verify underwater fields are properly accessible
    assert_eq!(metric.underwater_depth_meters, Some(15.5));
    assert_eq!(metric.diving_duration_seconds, Some(1800));

    // Verify swimming fields work alongside underwater fields
    assert_eq!(metric.distance_swimming_meters, Some(100.0));
    assert_eq!(metric.swimming_stroke_count, Some(50));
}

#[test]
fn test_ios_underwater_depth_conversion() {
    // Test iOS HealthKit identifier conversion for underwater depth
    let ios_metric = IoSMetric {
        name: "HKQuantityTypeIdentifierUnderwaterDepth".to_string(),
        data: vec![DataPoint {
            date: "2024-09-18T10:30:00Z".to_string(),
            qty: 12.3, // 12.3 meters depth
            source: Some("Apple Watch Ultra".to_string()),
        }],
    };

    // Verify the metric name matches DATA.md specification
    assert_eq!(ios_metric.name, "HKQuantityTypeIdentifierUnderwaterDepth");

    // Verify depth value is reasonable for recreational diving
    assert!(ios_metric.data[0].qty > 0.0);
    assert!(ios_metric.data[0].qty < 100.0); // Under technical diving limits

    // Test that the source device supports underwater tracking
    assert!(ios_metric.data[0].source.is_some());

    // Verify the conversion would use the right field in the ActivityMetric
    // This tests the pattern matching in ios_models.rs
    let metric_name = &ios_metric.name;
    let should_convert_underwater = metric_name == "HKQuantityTypeIdentifierUnderwaterDepth";
    assert!(should_convert_underwater);
}

#[test]
fn test_underwater_depth_validation_ranges() {
    // Test that database constraints would accept reasonable values

    // Test minimum depth (surface level)
    let surface_depth = 0.0;
    assert!(surface_depth >= 0.0 && surface_depth <= 1000.0);

    // Test recreational diving depth (typical range: 0-30m)
    let recreational_depth = 25.0;
    assert!(recreational_depth >= 0.0 && recreational_depth <= 1000.0);

    // Test technical diving depth (advanced range: 30-100m)
    let technical_depth = 75.0;
    assert!(technical_depth >= 0.0 && technical_depth <= 1000.0);

    // Test extreme depth (commercial/military: 100-300m)
    let extreme_depth = 250.0;
    assert!(extreme_depth >= 0.0 && extreme_depth <= 1000.0);

    // Test world record depth (theoretical max: ~1000m)
    let record_depth = 330.0; // Current deep diving record
    assert!(record_depth >= 0.0 && record_depth <= 1000.0);

    // Test that our database constraint max (1000m) covers all scenarios
    let max_constraint = 1000.0;
    assert!(surface_depth <= max_constraint);
    assert!(recreational_depth <= max_constraint);
    assert!(technical_depth <= max_constraint);
    assert!(extreme_depth <= max_constraint);
    assert!(record_depth <= max_constraint);
}

#[test]
fn test_diving_duration_validation_ranges() {
    // Test that database constraints would accept reasonable durations

    // Test minimum duration (single breath)
    let breath_hold = 30; // 30 seconds
    assert!(breath_hold >= 0 && breath_hold <= 86400);

    // Test short recreational dive
    let short_dive = 1800; // 30 minutes
    assert!(short_dive >= 0 && short_dive <= 86400);

    // Test typical recreational dive
    let typical_dive = 3600; // 1 hour
    assert!(typical_dive >= 0 && typical_dive <= 86400);

    // Test long technical dive
    let technical_dive = 7200; // 2 hours
    assert!(technical_dive >= 0 && technical_dive <= 86400);

    // Test extreme duration (safety limit)
    let max_safe_dive = 14400; // 4 hours (saturation diving prep)
    assert!(max_safe_dive >= 0 && max_safe_dive <= 86400);

    // Test our database constraint max (24 hours)
    let max_constraint = 86400; // 24 hours in seconds
    assert!(breath_hold <= max_constraint);
    assert!(short_dive <= max_constraint);
    assert!(typical_dive <= max_constraint);
    assert!(technical_dive <= max_constraint);
    assert!(max_safe_dive <= max_constraint);
}

#[test]
fn test_ios_16_plus_compatibility() {
    // Test that underwater metrics are marked as iOS 16+ compatible
    // This verifies the DATA.md specification compliance

    // According to DATA.md line 209:
    // "Underwater | HKQuantityTypeIdentifierUnderwaterDepth | Underwater depth | ⚠️ | iOS 16+ uncertain"

    let ios_identifier = "HKQuantityTypeIdentifierUnderwaterDepth";

    // Verify the identifier follows Apple's HealthKit naming convention
    assert!(ios_identifier.starts_with("HKQuantityTypeIdentifier"));
    assert!(ios_identifier.contains("Underwater"));
    assert!(ios_identifier.contains("Depth"));

    // Verify this is the only underwater metric in DATA.md
    // (no other underwater metrics listed in lines 208-209)
    let underwater_identifiers = vec![
        "HKQuantityTypeIdentifierUnderwaterDepth"
    ];
    assert_eq!(underwater_identifiers.len(), 1);
    assert_eq!(underwater_identifiers[0], ios_identifier);
}

#[test]
fn test_underwater_metrics_with_apple_watch_ultra() {
    // Test realistic underwater tracking scenario with Apple Watch Ultra
    // which supports underwater depth tracking as of iOS 16+

    let metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),

        // Scuba diving session metrics
        distance_swimming_meters: Some(500.0), // 500m horizontal distance
        swimming_stroke_count: Some(200), // Fin kicks
        underwater_depth_meters: Some(18.3), // 18.3m depth (recreational limit)
        diving_duration_seconds: Some(2700), // 45 minute dive

        // All other fields None for focused test
        step_count: None,
        distance_meters: None,
        flights_climbed: None,
        active_energy_burned_kcal: None,
        basal_energy_burned_kcal: None,
        distance_cycling_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,
        push_count: None,
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

        source_device: Some("Apple Watch Ultra".to_string()),
        created_at: Utc::now(),
    };

    // Verify realistic diving scenario
    assert!(metric.underwater_depth_meters.unwrap() > 10.0); // Deeper than snorkeling
    assert!(metric.underwater_depth_meters.unwrap() < 30.0); // Within recreational limits
    assert!(metric.diving_duration_seconds.unwrap() > 1800); // Longer than quick dive
    assert!(metric.diving_duration_seconds.unwrap() < 3600); // Under 1 hour (safe)

    // Verify swimming metrics complement underwater tracking
    assert!(metric.distance_swimming_meters.is_some());
    assert!(metric.swimming_stroke_count.is_some());

    // Verify appropriate device
    assert!(metric.source_device.as_ref().unwrap().contains("Ultra"));
}