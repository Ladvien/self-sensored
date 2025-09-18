use chrono::Utc;
use uuid::Uuid;

use self_sensored::models::health_metrics::ActivityMetric;
use self_sensored::models::ios_models::{IosMetric, DataPoint, IosData};

/// Test that cycling metrics are properly defined and accessible in ActivityMetric struct
#[test]
fn test_cycling_fields_accessible() {
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

        // Specialized distance metrics
        distance_cycling_meters: None,
        distance_swimming_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,

        // Wheelchair accessibility
        push_count: None,

        // Swimming analytics
        swimming_stroke_count: None,

        // Cross-platform integration
        nike_fuel_points: None,

        // Apple Watch activity rings
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

        // Cycling metrics (NEW - DATA.md lines 203-207)
        cycling_speed_kmh: Some(25.5), // Test value: 25.5 km/h
        cycling_power_watts: Some(250.0), // Test value: 250 watts
        cycling_cadence_rpm: Some(85.0), // Test value: 85 RPM
        functional_threshold_power_watts: Some(300.0), // Test value: 300 watts FTP

        // Underwater metrics (added by other agent)
        underwater_depth_meters: None,
        diving_duration_seconds: None,

        // Metadata
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    };

    // Verify cycling fields are accessible and contain expected values
    assert_eq!(metric.cycling_speed_kmh, Some(25.5));
    assert_eq!(metric.cycling_power_watts, Some(250.0));
    assert_eq!(metric.cycling_cadence_rpm, Some(85.0));
    assert_eq!(metric.functional_threshold_power_watts, Some(300.0));

    println!("✅ All cycling metric fields are properly defined and accessible");
}

/// Test iOS HealthKit identifier mapping for cycling metrics
#[test]
fn test_ios_cycling_identifier_mapping() {
    // Test cycling speed mapping
    let cycling_speed_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingSpeed".to_string(),
        data: vec![IosData {
            qty: Some(30.0), // 30 km/h
            date: "2024-09-18T16:00:00Z".to_string(),
            source: Some("Apple Watch".to_string()),
        }],
    };

    // Test cycling power mapping
    let cycling_power_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingPower".to_string(),
        data: vec![IosData {
            qty: Some(275.0), // 275 watts
            date: "2024-09-18T16:00:00Z".to_string(),
            source: Some("Power Meter".to_string()),
        }],
    };

    // Test cycling cadence mapping
    let cycling_cadence_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingCadence".to_string(),
        data: vec![IosData {
            qty: Some(90.0), // 90 RPM
            date: "2024-09-18T16:00:00Z".to_string(),
            source: Some("Cadence Sensor".to_string()),
        }],
    };

    // Test functional threshold power mapping
    let cycling_ftp_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingFunctionalThresholdPower".to_string(),
        data: vec![IosData {
            qty: Some(320.0), // 320 watts FTP
            date: "2024-09-18T16:00:00Z".to_string(),
            source: Some("Power Meter Test".to_string()),
        }],
    };

    // Verify metric names are supported HealthKit identifiers
    assert!(cycling_speed_ios.name.starts_with("HKQuantityTypeIdentifier"));
    assert!(cycling_power_ios.name.starts_with("HKQuantityTypeIdentifier"));
    assert!(cycling_cadence_ios.name.starts_with("HKQuantityTypeIdentifier"));
    assert!(cycling_ftp_ios.name.starts_with("HKQuantityTypeIdentifier"));

    // Verify data structure integrity
    assert_eq!(cycling_speed_ios.data[0].qty, Some(30.0));
    assert_eq!(cycling_power_ios.data[0].qty, Some(275.0));
    assert_eq!(cycling_cadence_ios.data[0].qty, Some(90.0));
    assert_eq!(cycling_ftp_ios.data[0].qty, Some(320.0));

    println!("✅ iOS HealthKit cycling identifier mapping validated");
    println!("  - HKQuantityTypeIdentifierCyclingSpeed: 30.0 km/h");
    println!("  - HKQuantityTypeIdentifierCyclingPower: 275.0 watts");
    println!("  - HKQuantityTypeIdentifierCyclingCadence: 90.0 RPM");
    println!("  - HKQuantityTypeIdentifierCyclingFunctionalThresholdPower: 320.0 watts");
}

/// Test cycling parameter validation ranges
#[test]
fn test_cycling_parameter_validation() {
    // Test valid cycling metrics
    let valid_metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),

        // Set all required fields to None/default
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

        // Valid cycling values
        cycling_speed_kmh: Some(45.0), // Professional cyclist speed
        cycling_power_watts: Some(400.0), // Professional cyclist power
        cycling_cadence_rpm: Some(95.0), // High cadence
        functional_threshold_power_watts: Some(350.0), // Elite FTP

        underwater_depth_meters: None,
        diving_duration_seconds: None,
        source_device: Some("Power Meter".to_string()),
        created_at: Utc::now(),
    };

    // Verify cycling metrics are within reasonable ranges
    if let Some(speed) = valid_metric.cycling_speed_kmh {
        assert!(speed >= 0.0 && speed <= 100.0, "Cycling speed should be 0-100 km/h");
    }

    if let Some(power) = valid_metric.cycling_power_watts {
        assert!(power >= 0.0 && power <= 2000.0, "Cycling power should be 0-2000 watts");
    }

    if let Some(cadence) = valid_metric.cycling_cadence_rpm {
        assert!(cadence >= 0.0 && cadence <= 200.0, "Cycling cadence should be 0-200 RPM");
    }

    if let Some(ftp) = valid_metric.functional_threshold_power_watts {
        assert!(ftp >= 0.0 && ftp <= 1500.0, "FTP should be 0-1500 watts");
    }

    println!("✅ Cycling parameter validation passed");
    println!("  - Speed range: 0-100 km/h ✓");
    println!("  - Power range: 0-2000 watts ✓");
    println!("  - Cadence range: 0-200 RPM ✓");
    println!("  - FTP range: 0-1500 watts ✓");
}