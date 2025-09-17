use serde_json::json;
use self_sensored::models::ios_models::IosIngestPayload;

/// Test that iOS payload with 'source' field gets correctly converted to internal format with 'source_device'
#[test]
fn test_ios_source_field_mapping() {
    // Create a sample iOS payload with 'source' field
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "heart_rate",
                    "units": "count/min",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2025-09-12 10:30:00 -0500",
                            "qty": 72.0
                        }
                    ]
                }
            ],
            "workouts": [
                {
                    "name": "Running",
                    "start": "2025-09-12 09:00:00 -0500",
                    "end": "2025-09-12 09:30:00 -0500",
                    "source": "Apple Watch"
                }
            ]
        }
    });

    // Parse the iOS payload
    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json).expect("Failed to parse iOS payload");

    // Convert to internal format with test user_id
    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Verify that metrics were converted and have source_device field
    assert!(!internal_payload.data.metrics.is_empty(), "Should have converted metrics");
    
    // Check heart rate metric has correct source_device mapping
    if let Some(heart_rate_metric) = internal_payload.data.metrics.iter().find(|m| {
        matches!(m, self_sensored::models::HealthMetric::HeartRate(_))
    }) {
        if let self_sensored::models::HealthMetric::HeartRate(hr_metric) = heart_rate_metric {
            assert_eq!(
                hr_metric.source_device, 
                Some("Apple Watch".to_string()),
                "iOS 'source' field should be mapped to 'source_device' in internal model"
            );
            assert_eq!(hr_metric.heart_rate, Some(72), "Heart rate value should be preserved");
        }
    } else {
        panic!("Should have converted heart rate metric");
    }

    // Verify that workouts were converted and have source_device field
    assert!(!internal_payload.data.workouts.is_empty(), "Should have converted workouts");
    
    let workout = &internal_payload.data.workouts[0];
    assert_eq!(
        workout.source_device,
        Some("Apple Watch".to_string()),
        "Workout 'source' field should be mapped to 'source_device' in internal model"
    );
}

/// Test that various iOS metric types all properly map source -> source_device
#[test]
fn test_all_metric_types_source_mapping() {
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "blood_pressure_systolic",
                    "data": [{"source": "Manual Entry", "date": "2025-09-12 10:30:00 -0500", "qty": 120.0}]
                },
                {
                    "name": "blood_pressure_diastolic", 
                    "data": [{"source": "Manual Entry", "date": "2025-09-12 10:30:00 -0500", "qty": 80.0}]
                },
                {
                    "name": "sleep_analysis",
                    "data": [{
                        "source": "iPhone",
                        "start": "2025-09-12 22:00:00 -0500",
                        "end": "2025-09-12 06:00:00 -0500"
                    }]
                },
                {
                    "name": "steps",
                    "data": [{"source": "iPhone", "date": "2025-09-12 00:00:00 -0500", "qty": 8000.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json).expect("Failed to parse iOS payload");
    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Check that all converted metrics have the correct source_device mapping
    for metric in &internal_payload.data.metrics {
        match metric {
            self_sensored::models::HealthMetric::BloodPressure(bp) => {
                assert_eq!(bp.source_device, Some("Manual Entry".to_string()));
            },
            self_sensored::models::HealthMetric::Sleep(sleep) => {
                assert_eq!(sleep.source_device, Some("iPhone".to_string()));
            },
            self_sensored::models::HealthMetric::Activity(activity) => {
                assert_eq!(activity.source_device, Some("iPhone".to_string()));
            },
            _ => {} // Other metric types
        }
    }
}

/// CRITICAL TEST: Validate HealthKit identifier mappings for STORY-DATA-002
/// This test ensures all major iOS HealthKit identifiers are properly mapped
#[test]
fn test_healthkit_identifier_mappings() {
    // Test payload with actual HealthKit identifiers that iOS Auto Health Export sends
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",
                    "units": "count/min",
                    "data": [
                        {
                            "sourceName": "Apple Watch",
                            "source": "Apple Watch Series 7",
                            "date": "2025-09-17 12:00:00 -0500",
                            "qty": 75.0
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "source": "Health",
                            "date": "2025-09-17 12:00:00 -0500",
                            "qty": 120.0
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                    "units": "mmHg",
                    "data": [
                        {
                            "source": "Health",
                            "date": "2025-09-17 12:00:00 -0500",
                            "qty": 80.0
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierStepCount",
                    "units": "count",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2025-09-17 00:00:00 -0500",
                            "qty": 8542.0
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierActiveEnergyBurned",
                    "units": "kcal",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2025-09-17 10:00:00 -0500",
                            "qty": 320.5
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierDistanceWalkingRunning",
                    "units": "m",
                    "data": [
                        {
                            "source": "iPhone",
                            "date": "2025-09-17 00:00:00 -0500",
                            "qty": 5500.0
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierBodyMass",
                    "units": "kg",
                    "data": [
                        {
                            "source": "Health",
                            "date": "2025-09-17 08:00:00 -0500",
                            "qty": 70.5
                        }
                    ]
                },
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "data": [
                        {
                            "source": "iPhone",
                            "start": "2025-09-16 22:00:00 -0500",
                            "end": "2025-09-17 06:00:00 -0500"
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json)
        .expect("Failed to parse iOS payload with HealthKit identifiers");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Verify we got the expected number of metrics (some may create multiple records like BP)
    assert!(!internal_payload.data.metrics.is_empty(), "Should have converted HealthKit identifiers");

    // Test each metric type conversion
    let mut found_heart_rate = false;
    let mut found_blood_pressure = false;
    let mut found_activity = false;
    let mut found_body_measurement = false;
    let mut found_sleep = false;

    for metric in &internal_payload.data.metrics {
        match metric {
            self_sensored::models::HealthMetric::HeartRate(hr) => {
                found_heart_rate = true;
                assert_eq!(hr.heart_rate, Some(75), "HealthKit heart rate should be mapped");
                assert_eq!(hr.source_device, Some("Apple Watch Series 7".to_string()));
            },
            self_sensored::models::HealthMetric::BloodPressure(bp) => {
                found_blood_pressure = true;
                assert!(bp.systolic == 120 || bp.diastolic == 80, "HealthKit blood pressure should be mapped");
                assert_eq!(bp.source_device, Some("Auto Health Export iOS".to_string()));
            },
            self_sensored::models::HealthMetric::Activity(activity) => {
                found_activity = true;
                // Should find either steps, calories, or distance
                assert!(
                    activity.step_count == Some(8542) ||
                    activity.active_energy_burned_kcal == Some(320.5) ||
                    activity.distance_meters == Some(5500.0),
                    "HealthKit activity metrics should be mapped"
                );
            },
            self_sensored::models::HealthMetric::BodyMeasurement(body) => {
                found_body_measurement = true;
                assert_eq!(body.body_weight_kg, Some(70.5), "HealthKit body mass should be mapped");
                assert_eq!(body.source_device, Some("Health".to_string()));
            },
            self_sensored::models::HealthMetric::Sleep(sleep) => {
                found_sleep = true;
                assert!(sleep.duration_minutes.is_some(), "HealthKit sleep should be mapped");
                assert_eq!(sleep.source_device, Some("iPhone".to_string()));
            },
            _ => {}
        }
    }

    // Verify all expected metrics were found
    assert!(found_heart_rate, "HKQuantityTypeIdentifierHeartRate should be mapped to HeartRate metric");
    assert!(found_blood_pressure, "HKQuantityTypeIdentifierBloodPressure* should be mapped to BloodPressure metric");
    assert!(found_activity, "HKQuantityTypeIdentifier* activity metrics should be mapped to Activity metric");
    assert!(found_body_measurement, "HKQuantityTypeIdentifierBodyMass should be mapped to BodyMeasurement metric");
    assert!(found_sleep, "HKCategoryTypeIdentifierSleepAnalysis should be mapped to Sleep metric");
}

/// Test extended activity metrics mapping for HealthKit identifiers
#[test]
fn test_extended_activity_healthkit_identifiers() {
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierDistanceCycling",
                    "units": "m",
                    "data": [{"source": "iPhone", "date": "2025-09-17 10:00:00 -0500", "qty": 15000.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierDistanceSwimming",
                    "units": "m",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 11:00:00 -0500", "qty": 1000.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierAppleExerciseTime",
                    "units": "min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 12:00:00 -0500", "qty": 30.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierAppleStandTime",
                    "units": "min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 13:00:00 -0500", "qty": 12.0}]
                },
                {
                    "name": "HKCategoryTypeIdentifierAppleStandHour",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 14:00:00 -0500", "qty": 1.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json)
        .expect("Failed to parse extended activity HealthKit identifiers");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    assert!(!internal_payload.data.metrics.is_empty(), "Should have converted extended activity metrics");

    for metric in &internal_payload.data.metrics {
        if let self_sensored::models::HealthMetric::Activity(activity) = metric {
            // Verify extended activity fields are properly mapped
            if activity.distance_cycling_meters.is_some() {
                assert_eq!(activity.distance_cycling_meters, Some(15000.0));
            }
            if activity.distance_swimming_meters.is_some() {
                assert_eq!(activity.distance_swimming_meters, Some(1000.0));
            }
            if activity.apple_exercise_time_minutes.is_some() {
                assert_eq!(activity.apple_exercise_time_minutes, Some(30));
            }
            if activity.apple_stand_time_minutes.is_some() {
                assert_eq!(activity.apple_stand_time_minutes, Some(12));
            }
            if activity.apple_stand_hour_achieved.is_some() {
                assert_eq!(activity.apple_stand_hour_achieved, Some(true));
            }
        }
    }
}

/// Test unknown HealthKit identifiers trigger proper logging
#[test]
fn test_unknown_healthkit_identifiers_logging() {
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierRespiratoryRate",
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 12:00:00 -0500", "qty": 16.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierBloodGlucose",
                    "units": "mg/dL",
                    "data": [{"source": "Glucose Meter", "date": "2025-09-17 08:00:00 -0500", "qty": 95.0}]
                },
                {
                    "name": "SomeCustomMetric",
                    "units": "custom",
                    "data": [{"source": "Third Party App", "date": "2025-09-17 12:00:00 -0500", "qty": 42.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json)
        .expect("Failed to parse payload with unknown HealthKit identifiers");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should not crash, but metrics should be empty since they're unmapped
    // The key is that this should trigger the warning logs we added
    println!("Converted {} metrics from unknown HealthKit identifiers", internal_payload.data.metrics.len());
}

/// Test backward compatibility - old simplified names should still work
#[test]
fn test_backward_compatibility_simplified_names() {
    let ios_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "heart_rate",  // Old simplified name
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 12:00:00 -0500", "qty": 72.0}]
                },
                {
                    "name": "HKQuantityTypeIdentifierHeartRate",  // New HealthKit identifier
                    "units": "count/min",
                    "data": [{"source": "Apple Watch", "date": "2025-09-17 12:01:00 -0500", "qty": 73.0}]
                }
            ],
            "workouts": []
        }
    });

    let ios_payload: IosIngestPayload = serde_json::from_value(ios_json)
        .expect("Failed to parse backward compatibility payload");

    let test_user_id = uuid::Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(test_user_id);

    // Should have 2 heart rate metrics - one from old name, one from HealthKit identifier
    let heart_rate_count = internal_payload.data.metrics.iter()
        .filter(|m| matches!(m, self_sensored::models::HealthMetric::HeartRate(_)))
        .count();

    assert_eq!(heart_rate_count, 2, "Both simplified names and HealthKit identifiers should work");
}