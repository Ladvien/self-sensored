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

    // Convert to internal format
    let internal_payload = ios_payload.to_internal_format();

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
    let internal_payload = ios_payload.to_internal_format();

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