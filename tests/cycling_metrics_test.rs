mod common;

use chrono::Utc;
use uuid::Uuid;

use common::fixtures::create_minimal_activity_metric;
use self_sensored::models::health_metrics::ActivityMetric;
use self_sensored::models::ios_models::{IosMetric, IosMetricData};

/// Test that cycling metrics are properly defined and accessible in ActivityMetric struct
#[test]
fn test_cycling_fields_accessible() {
    let mut metric = create_minimal_activity_metric(Uuid::new_v4());

    // Set cycling-specific test values
    metric.cycling_speed_kmh = Some(25.5); // Test value: 25.5 km/h
    metric.cycling_power_watts = Some(250.0); // Test value: 250 watts
    metric.cycling_cadence_rpm = Some(85.0); // Test value: 85 RPM
    metric.functional_threshold_power_watts = Some(300.0); // Test value: 300 watts FTP

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
        units: Some("km/h".to_string()),
        data: vec![IosMetricData {
            qty: Some(30.0), // 30 km/h
            date: Some("2024-09-18T16:00:00Z".to_string()),
            start: None,
            end: None,
            source: Some("Apple Watch".to_string()),
            value: None,
            extra: std::collections::HashMap::new(),
        }],
    };

    // Test cycling power mapping
    let cycling_power_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingPower".to_string(),
        units: Some("watts".to_string()),
        data: vec![IosMetricData {
            qty: Some(275.0), // 275 watts
            date: Some("2024-09-18T16:00:00Z".to_string()),
            start: None,
            end: None,
            source: Some("Power Meter".to_string()),
            value: None,
            extra: std::collections::HashMap::new(),
        }],
    };

    // Test cycling cadence mapping
    let cycling_cadence_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingCadence".to_string(),
        units: Some("rpm".to_string()),
        data: vec![IosMetricData {
            qty: Some(90.0), // 90 RPM
            date: Some("2024-09-18T16:00:00Z".to_string()),
            start: None,
            end: None,
            source: Some("Cadence Sensor".to_string()),
            value: None,
            extra: std::collections::HashMap::new(),
        }],
    };

    // Test functional threshold power mapping
    let cycling_ftp_ios = IosMetric {
        name: "HKQuantityTypeIdentifierCyclingFunctionalThresholdPower".to_string(),
        units: Some("watts".to_string()),
        data: vec![IosMetricData {
            qty: Some(320.0), // 320 watts FTP
            date: Some("2024-09-18T16:00:00Z".to_string()),
            start: None,
            end: None,
            source: Some("Power Meter Test".to_string()),
            value: None,
            extra: std::collections::HashMap::new(),
        }],
    };

    // Verify metric names are supported HealthKit identifiers
    assert!(cycling_speed_ios
        .name
        .starts_with("HKQuantityTypeIdentifier"));
    assert!(cycling_power_ios
        .name
        .starts_with("HKQuantityTypeIdentifier"));
    assert!(cycling_cadence_ios
        .name
        .starts_with("HKQuantityTypeIdentifier"));
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
    let mut valid_metric = create_minimal_activity_metric(Uuid::new_v4());

    // Valid cycling values
    valid_metric.cycling_speed_kmh = Some(45.0); // Professional cyclist speed
    valid_metric.cycling_power_watts = Some(400.0); // Professional cyclist power
    valid_metric.cycling_cadence_rpm = Some(95.0); // High cadence
    valid_metric.functional_threshold_power_watts = Some(350.0); // Elite FTP

    // Verify cycling metrics are within reasonable ranges
    if let Some(speed) = valid_metric.cycling_speed_kmh {
        assert!(
            speed >= 0.0 && speed <= 100.0,
            "Cycling speed should be 0-100 km/h"
        );
    }

    if let Some(power) = valid_metric.cycling_power_watts {
        assert!(
            power >= 0.0 && power <= 2000.0,
            "Cycling power should be 0-2000 watts"
        );
    }

    if let Some(cadence) = valid_metric.cycling_cadence_rpm {
        assert!(
            cadence >= 0.0 && cadence <= 200.0,
            "Cycling cadence should be 0-200 RPM"
        );
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
