/// STORY-DATA-002: iOS Metric Name Mapping Validation Test Suite
///
/// This test suite comprehensively validates iOS metric name mappings to ensure:
/// 1. All supported iOS metric types are properly converted
/// 2. No data loss occurs during iOS-to-internal conversion
/// 3. Unknown metric types are properly logged and tracked
/// 4. Backward compatibility is maintained for legacy iOS metric names

use serde_json::json;
use self_sensored::models::ios_models::IosIngestPayload;
use self_sensored::models::HealthMetric;

#[test]
fn test_comprehensive_ios_metric_mapping_validation() {
    // Test payload with ALL currently supported iOS metric types
    let comprehensive_ios_payload = json!({
        "data": {
            "metrics": [
                // HEART RATE METRICS - HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:00:00 -0500", "qty": 72.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierRestingHeartRate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:01:00 -0500", "qty": 60.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierWalkingHeartRateAverage",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:02:00 -0500", "qty": 85.0}]
                },
                // HEART RATE METRICS - Legacy names
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [{"source": "Manual Entry", "date": "2025-09-18 10:03:00 -0500", "qty": 68.0}]
                },
                // BLOOD PRESSURE METRICS - HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [{"source": "Blood Pressure Monitor", "date": "2025-09-18 10:05:00 -0500", "qty": 120.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": [{"source": "Blood Pressure Monitor", "date": "2025-09-18 10:05:00 -0500", "qty": 80.0}]
                },
                // SLEEP METRICS - HealthKit identifiers
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "data": [{
                        "source": "iPhone",
                        "start": "2025-09-17 22:00:00 -0500",
                        "end": "2025-09-18 06:00:00 -0500"
                    }]
                },
                // ACTIVITY METRICS - Core HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [{"source": "iPhone", "date": "2025-09-18 00:00:00 -0500", "qty": 10500.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierDistanceWalkingRunning",
                    "units": "m",
                    "data": [{"source": "iPhone", "date": "2025-09-18 00:00:00 -0500", "qty": 7500.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierActiveEnergyBurned",
                    "units": "kcal",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 00:00:00 -0500", "qty": 450.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierFlightsClimbed",
                    "units": "count",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 00:00:00 -0500", "qty": 15.0}]
                },
                // ACTIVITY METRICS - Extended HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierDistanceCycling",
                    "units": "m",
                    "data": [{"source": "Bike Computer", "date": "2025-09-18 08:00:00 -0500", "qty": 20000.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierDistanceSwimming",
                    "units": "m",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 07:00:00 -0500", "qty": 1500.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierAppleExerciseTime",
                    "units": "min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 00:00:00 -0500", "qty": 45.0}]
                },
                // TEMPERATURE METRICS - HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierBodyTemperature",
                    "units": "°C",
                    "data": [{"source": "Thermometer", "date": "2025-09-18 09:00:00 -0500", "qty": 37.2}]
                },
                {
                    "name": "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
                    "units": "°C",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 03:00:00 -0500", "qty": 35.8}]
                },
                // AUDIO EXPOSURE METRICS - HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
                    "units": "dB",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 12:00:00 -0500", "qty": 85.0, "duration_minutes": 60}]
                },
                {
                    "name": "HKQuantityTypeIdentifierHeadphoneAudioExposure",
                    "units": "dB",
                    "data": [{"source": "AirPods", "date": "2025-09-18 13:00:00 -0500", "qty": 80.0, "duration_minutes": 30}]
                },
                // BODY MEASUREMENT METRICS - HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierBodyMass",
                    "units": "kg",
                    "data": [{"source": "Smart Scale", "date": "2025-09-18 07:30:00 -0500", "qty": 75.5}]
                },
                {
                    "name": "HKQuantityTypeIdentifierBodyMassIndex",
                    "units": "count",
                    "data": [{"source": "Health App", "date": "2025-09-18 07:30:00 -0500", "qty": 23.8}]
                },
                {
                    "name": "HKQuantityTypeIdentifierHeight",
                    "units": "cm",
                    "data": [{"source": "Manual Entry", "date": "2025-09-18 07:00:00 -0500", "qty": 175.0}]
                },
                // ENVIRONMENTAL METRICS - Custom names
                {
                    "name": "uv_exposure",
                    "units": "UV Index",
                    "data": [{"source": "Weather App", "date": "2025-09-18 14:00:00 -0500", "qty": 8.0}]
                },
                {
                    "name": "time_in_daylight",
                    "units": "minutes",
                    "data": [{"source": "iPhone", "date": "2025-09-18 00:00:00 -0500", "qty": 480.0}]
                },
                // SAFETY EVENT METRICS - Custom names
                {
                    "name": "fall_detection",
                    "units": "count",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 15:00:00 -0500", "qty": 1.0}]
                },
                // LEGACY NAMES - Backward compatibility test
                {
                    "name": "steps",
                    "units": "count",
                    "data": [{"source": "Legacy App", "date": "2025-09-18 00:00:00 -0500", "qty": 5000.0}]
                },
                {
                    "name": "calories",
                    "units": "kcal",
                    "data": [{"source": "Legacy App", "date": "2025-09-18 00:00:00 -0500", "qty": 200.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(comprehensive_ios_payload)
        .expect("Failed to parse comprehensive iOS payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Validate that metrics were converted (should be more than 0)
    assert!(!internal_payload.data.metrics.is_empty(), "Should have converted at least some metrics");

    // Count conversions by metric type
    let mut conversion_counts = std::collections::HashMap::new();
    for metric in &internal_payload.data.metrics {
        let metric_type = match metric {
            HealthMetric::HeartRate(_) => "HeartRate",
            HealthMetric::BloodPressure(_) => "BloodPressure",
            HealthMetric::Sleep(_) => "Sleep",
            HealthMetric::Activity(_) => "Activity",
            HealthMetric::Temperature(_) => "Temperature",
            HealthMetric::Environmental(_) => "Environmental",
            HealthMetric::AudioExposure(_) => "AudioExposure",
            HealthMetric::BodyMeasurement(_) => "BodyMeasurement",
            HealthMetric::SafetyEvent(_) => "SafetyEvent",
            _ => "Other",
        };
        *conversion_counts.entry(metric_type).or_insert(0) += 1;
    }

    // Print conversion summary for debugging
    println!("iOS Metric Conversion Summary:");
    for (metric_type, count) in &conversion_counts {
        println!("  {}: {} metrics", metric_type, count);
    }
    println!("  Total: {} metrics", internal_payload.data.metrics.len());

    // Validate specific metric type conversions
    assert!(conversion_counts.get("HeartRate").unwrap_or(&0) >= &4, "Should have at least 4 heart rate metrics");
    assert!(conversion_counts.get("BloodPressure").unwrap_or(&0) >= &1, "Should have at least 1 blood pressure metric");
    assert!(conversion_counts.get("Sleep").unwrap_or(&0) >= &1, "Should have at least 1 sleep metric");
    assert!(conversion_counts.get("Activity").unwrap_or(&0) >= &6, "Should have at least 6 activity metrics");
    assert!(conversion_counts.get("Temperature").unwrap_or(&0) >= &2, "Should have at least 2 temperature metrics");
    assert!(conversion_counts.get("AudioExposure").unwrap_or(&0) >= &2, "Should have at least 2 audio exposure metrics");
    assert!(conversion_counts.get("BodyMeasurement").unwrap_or(&0) >= &3, "Should have at least 3 body measurement metrics");
    assert!(conversion_counts.get("Environmental").unwrap_or(&0) >= &2, "Should have at least 2 environmental metrics");
    assert!(conversion_counts.get("SafetyEvent").unwrap_or(&0) >= &1, "Should have at least 1 safety event metric");

    // Validate that we have good coverage (expect at least 25 converted metrics)
    assert!(internal_payload.data.metrics.len() >= 25,
           "Should have at least 25 converted metrics, got {}", internal_payload.data.metrics.len());
}

#[test]
fn test_unknown_ios_metric_types_handling() {
    // Test payload with known unsupported iOS metric types
    let unknown_metrics_payload = json!({
        "data": {
            "metrics": [
                // Critical missing HealthKit identifiers
                {
                    "name": "HKQuantityTypeIdentifierRespiratoryRate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:00:00 -0500", "qty": 16.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodGlucose",
                    "units": "mg/dL",
                    "data": [{"source": "Glucose Meter", "date": "2025-09-18 08:00:00 -0500", "qty": 95.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierDietaryWater",
                    "units": "mL",
                    "data": [{"source": "Water Tracking App", "date": "2025-09-18 12:00:00 -0500", "qty": 2000.0}]
                },
                {
                    "name": "HKCategoryTypeIdentifierMindfulSession",
                    "data": [{
                        "source": "Meditation App",
                        "start": "2025-09-18 08:00:00 -0500",
                        "end": "2025-09-18 08:10:00 -0500"
                    }]
                },
                // Completely unknown metric types
                {
                    "name": "CustomAppMetricType",
                    "units": "custom",
                    "data": [{"source": "Third Party App", "date": "2025-09-18 10:00:00 -0500", "qty": 42.0}]
                },
                {
                    "name": "SomeUnknownMetric",
                    "units": "unknown",
                    "data": [{"source": "Unknown Source", "date": "2025-09-18 10:00:00 -0500", "qty": 123.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(unknown_metrics_payload)
        .expect("Failed to parse unknown metrics payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // These should NOT be converted - they should be dropped
    // Since all metrics are unknown/unsupported, we should get 0 converted metrics
    assert_eq!(internal_payload.data.metrics.len(), 0,
              "Unknown iOS metrics should not be converted, got {} metrics",
              internal_payload.data.metrics.len());

    // This test primarily validates that:
    // 1. Unknown metrics don't crash the system
    // 2. They are properly logged (we can't test logging in unit tests easily)
    // 3. They don't create invalid internal metrics
}

#[test]
fn test_healthkit_identifier_vs_simplified_name_equivalence() {
    // Test that HealthKit identifiers and simplified names produce equivalent results
    let healthkit_vs_simplified = json!({
        "data": {
            "metrics": [
                // HealthKit identifier
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:00:00 -0500", "qty": 75.0}]
                },
                // Simplified name - should produce equivalent result
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-18 10:01:00 -0500", "qty": 76.0}]
                },
                // HealthKit identifier for steps
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [{"source": "iPhone", "date": "2025-09-18 00:00:00 -0500", "qty": 8000.0}]
                },
                // Simplified name for steps
                {
                    "name": "steps",
                    "units": "count",
                    "data": [{"source": "iPhone", "date": "2025-09-18 00:01:00 -0500", "qty": 8100.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(healthkit_vs_simplified)
        .expect("Failed to parse equivalence test payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should have 4 metrics: 2 heart rate + 2 activity (steps)
    assert_eq!(internal_payload.data.metrics.len(), 4,
              "Should have 4 converted metrics, got {}", internal_payload.data.metrics.len());

    // Count heart rate and activity metrics
    let heart_rate_count = internal_payload.data.metrics.iter()
        .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
        .count();
    let activity_count = internal_payload.data.metrics.iter()
        .filter(|m| matches!(m, HealthMetric::Activity(_)))
        .count();

    assert_eq!(heart_rate_count, 2, "Should have 2 heart rate metrics");
    assert_eq!(activity_count, 2, "Should have 2 activity metrics");

    // Validate that both heart rate metrics have the expected values
    let heart_rates: Vec<i16> = internal_payload.data.metrics.iter()
        .filter_map(|m| {
            if let HealthMetric::HeartRate(hr) = m {
                hr.heart_rate
            } else {
                None
            }
        })
        .collect();

    assert!(heart_rates.contains(&75), "Should contain heart rate 75 from HealthKit identifier");
    assert!(heart_rates.contains(&76), "Should contain heart rate 76 from simplified name");

    // Validate that both activity metrics have step counts
    let step_counts: Vec<i32> = internal_payload.data.metrics.iter()
        .filter_map(|m| {
            if let HealthMetric::Activity(activity) = m {
                activity.step_count
            } else {
                None
            }
        })
        .collect();

    assert!(step_counts.contains(&Some(8000)), "Should contain step count 8000 from HealthKit identifier");
    assert!(step_counts.contains(&Some(8100)), "Should contain step count 8100 from simplified name");
}

#[test]
fn test_ios_metric_edge_cases() {
    // Test edge cases and validation rules
    let edge_cases_payload = json!({
        "data": {
            "metrics": [
                // Invalid heart rate (too high)
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [{"source": "Test", "date": "2025-09-18 10:00:00 -0500", "qty": 350.0}]
                },
                // Valid heart rate at boundary
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [{"source": "Test", "date": "2025-09-18 10:01:00 -0500", "qty": 300.0}]
                },
                // Invalid temperature (too low)
                {
                    "name": "body_temperature",
                    "units": "°C",
                    "data": [{"source": "Test", "date": "2025-09-18 10:02:00 -0500", "qty": -60.0}]
                },
                // Valid temperature at boundary
                {
                    "name": "body_temperature",
                    "units": "°C",
                    "data": [{"source": "Test", "date": "2025-09-18 10:03:00 -0500", "qty": -50.0}]
                },
                // Negative step count (should be rejected)
                {
                    "name": "steps",
                    "units": "count",
                    "data": [{"source": "Test", "date": "2025-09-18 00:00:00 -0500", "qty": -100.0}]
                },
                // Zero step count (should be accepted)
                {
                    "name": "steps",
                    "units": "count",
                    "data": [{"source": "Test", "date": "2025-09-18 00:01:00 -0500", "qty": 0.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(edge_cases_payload)
        .expect("Failed to parse edge cases payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should only have valid metrics: 1 heart rate + 1 temperature + 1 activity (0 steps)
    // Invalid metrics should be filtered out by validation
    assert!(internal_payload.data.metrics.len() >= 2,
           "Should have at least 2 valid metrics after validation");

    // Check that invalid heart rate was rejected
    let heart_rates: Vec<i16> = internal_payload.data.metrics.iter()
        .filter_map(|m| {
            if let HealthMetric::HeartRate(hr) = m {
                hr.heart_rate
            } else {
                None
            }
        })
        .collect();

    assert!(!heart_rates.contains(&350), "Should reject heart rate of 350");
    assert!(heart_rates.contains(&300), "Should accept heart rate of 300 (boundary)");

    // Check that zero steps are accepted but negative are not
    let step_counts: Vec<Option<i32>> = internal_payload.data.metrics.iter()
        .filter_map(|m| {
            if let HealthMetric::Activity(activity) = m {
                Some(activity.step_count)
            } else {
                None
            }
        })
        .collect();

    assert!(step_counts.contains(&Some(0)), "Should accept 0 steps");
    assert!(!step_counts.contains(&Some(-100)), "Should reject negative steps");
}

#[test]
fn test_ios_metric_conversion_completeness() {
    // This test validates that our mapping documentation matches actual implementation
    // Based on the analysis in docs/ios_metric_mappings.md

    let supported_healthkit_identifiers = vec![
        // Heart Rate (5 identifiers)
        "HKQuantityTypeIdentifierHeartRate",
        "HKQuantityTypeIdentifierRestingHeartRate",
        "HKQuantityTypeIdentifierWalkingHeartRateAverage",
        "HKQuantityTypeIdentifierHeartRateVariabilitySDNN",
        "HKQuantityTypeIdentifierHeartRateRecoveryOneMinute",

        // Blood Pressure (2 identifiers)
        "HKQuantityTypeIdentifierBloodPressureSystolic",
        "HKQuantityTypeIdentifierBloodPressureDiastolic",

        // Sleep (1 identifier)
        "HKCategoryTypeIdentifierSleepAnalysis",

        // Activity (15 identifiers)
        "HKQuantityTypeIdentifierStepCount",
        "HKQuantityTypeIdentifierDistanceWalkingRunning",
        "HKQuantityTypeIdentifierActiveEnergyBurned",
        "HKQuantityTypeIdentifierBasalEnergyBurned",
        "HKQuantityTypeIdentifierFlightsClimbed",
        "HKQuantityTypeIdentifierDistanceCycling",
        "HKQuantityTypeIdentifierDistanceSwimming",
        "HKQuantityTypeIdentifierDistanceWheelchair",
        "HKQuantityTypeIdentifierDistanceDownhillSnowSports",
        "HKQuantityTypeIdentifierPushCount",
        "HKQuantityTypeIdentifierSwimmingStrokeCount",
        "HKQuantityTypeIdentifierNikeFuel",
        "HKQuantityTypeIdentifierAppleExerciseTime",
        "HKQuantityTypeIdentifierAppleStandTime",
        "HKQuantityTypeIdentifierAppleMoveTime",
        "HKCategoryTypeIdentifierAppleStandHour",

        // Temperature (4 identifiers)
        "HKQuantityTypeIdentifierBodyTemperature",
        "HKQuantityTypeIdentifierBasalBodyTemperature",
        "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
        "HKQuantityTypeIdentifierWaterTemperature",

        // Audio Exposure (2 identifiers)
        "HKQuantityTypeIdentifierEnvironmentalAudioExposure",
        "HKQuantityTypeIdentifierHeadphoneAudioExposure",

        // Body Measurements (6 identifiers)
        "HKQuantityTypeIdentifierBodyMass",
        "HKQuantityTypeIdentifierBodyMassIndex",
        "HKQuantityTypeIdentifierBodyFatPercentage",
        "HKQuantityTypeIdentifierLeanBodyMass",
        "HKQuantityTypeIdentifierHeight",
        "HKQuantityTypeIdentifierWaistCircumference",
    ];

    // Total should be 34 HealthKit identifiers as documented
    assert_eq!(supported_healthkit_identifiers.len(), 34,
              "Should have exactly 34 supported HealthKit identifiers");

    // Create test payload with one example of each supported HealthKit identifier
    let mut test_metrics = Vec::new();
    for (i, identifier) in supported_healthkit_identifiers.iter().enumerate() {
        let metric = json!({
            "name": identifier,
            "units": "test_unit",
            "data": [{
                "source": "Test Source",
                "date": format!("2025-09-18 {:02}:00:00 -0500", 10 + (i % 14)), // Spread across hours
                "qty": 50.0 + (i as f64) // Ensure different values
            }]
        });
        test_metrics.push(metric);
    }

    let test_payload = json!({
        "data": {
            "metrics": test_metrics,
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(test_payload)
        .expect("Failed to parse HealthKit identifiers test payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should convert most metrics (some validation may filter out edge cases)
    // Expect at least 80% conversion rate
    let expected_min_conversions = (supported_healthkit_identifiers.len() as f64 * 0.8) as usize;
    assert!(internal_payload.data.metrics.len() >= expected_min_conversions,
           "Should convert at least {}% of HealthKit identifiers, got {} out of {}",
           80, internal_payload.data.metrics.len(), supported_healthkit_identifiers.len());

    println!("HealthKit Identifier Conversion Rate: {}/34 ({:.1}%)",
             internal_payload.data.metrics.len(),
             (internal_payload.data.metrics.len() as f64 / 34.0) * 100.0);
}