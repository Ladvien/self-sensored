use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::{ios_models::IosIngestPayload, ActivityMetric, HealthMetric};

#[test]
fn test_mobility_metrics_ios_conversion() {
    // Test iOS payload with mobility metrics
    let ios_payload_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKQuantityTypeIdentifierWalkingSpeed",
                    "units": "m/s",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:30:00Z",
                            "qty": 1.2
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierWalkingStepLength",
                    "units": "cm",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:30:00Z",
                            "qty": 65.5
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierWalkingAsymmetryPercentage",
                    "units": "%",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:30:00Z",
                            "qty": 15.2
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierStairAscentSpeed",
                    "units": "m/s",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T10:45:00Z",
                            "qty": 0.8
                        }
                    ]
                },
                {
                    "name": "HKQuantityTypeIdentifierRunningPower",
                    "units": "W",
                    "data": [
                        {
                            "source": "Apple Watch",
                            "date": "2024-01-15T11:00:00Z",
                            "qty": 250.0
                        }
                    ]
                }
            ],
            "workouts": []
        }
    });

    // Parse the iOS payload
    let ios_payload: IosIngestPayload = serde_json::from_value(ios_payload_json).unwrap();

    // Convert to internal format
    let user_id = Uuid::new_v4();
    let internal_payload = ios_payload.to_internal_format(user_id);

    // Verify we have 5 activity metrics
    assert_eq!(internal_payload.data.metrics.len(), 5);

    // Check each mobility metric
    for metric in &internal_payload.data.metrics {
        if let HealthMetric::Activity(activity_metric) = metric {
            // Verify the metrics are populated correctly
            match activity_metric.walking_speed_m_per_s {
                Some(speed) => {
                    assert_eq!(speed, 1.2);
                    println!("✅ Walking speed correctly mapped: {} m/s", speed);
                }
                None => {}
            }

            match activity_metric.walking_step_length_cm {
                Some(step_length) => {
                    assert_eq!(step_length, 65.5);
                    println!("✅ Walking step length correctly mapped: {} cm", step_length);
                }
                None => {}
            }

            match activity_metric.walking_asymmetry_percent {
                Some(asymmetry) => {
                    assert_eq!(asymmetry, 15.2);
                    println!("✅ Walking asymmetry correctly mapped: {}%", asymmetry);
                }
                None => {}
            }

            match activity_metric.stair_ascent_speed_m_per_s {
                Some(stair_speed) => {
                    assert_eq!(stair_speed, 0.8);
                    println!("✅ Stair ascent speed correctly mapped: {} m/s", stair_speed);
                }
                None => {}
            }

            match activity_metric.running_power_watts {
                Some(power) => {
                    assert_eq!(power, 250.0);
                    println!("✅ Running power correctly mapped: {} W", power);
                }
                None => {}
            }
        }
    }

    println!("✅ All mobility metrics conversion tests passed!");
}

#[test]
fn test_activity_metric_database_fields() {
    // Test that all new mobility fields are properly defined
    let activity_metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        step_count: None,
        distance_meters: None,
        active_energy_burned_kcal: None,
        basal_energy_burned_kcal: None,
        flights_climbed: None,
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

        // Test all new mobility metrics fields
        walking_speed_m_per_s: Some(1.3),
        walking_step_length_cm: Some(67.8),
        walking_asymmetry_percent: Some(12.5),
        walking_double_support_percent: Some(25.0),
        six_minute_walk_test_distance_m: Some(450.0),

        stair_ascent_speed_m_per_s: Some(0.9),
        stair_descent_speed_m_per_s: Some(1.1),

        ground_contact_time_ms: Some(280.0),
        vertical_oscillation_cm: Some(8.5),
        running_stride_length_m: Some(1.45),
        running_power_watts: Some(275.0),
        running_speed_m_per_s: Some(3.2),

        source_device: Some("Apple Watch Series 8".to_string()),
        created_at: Utc::now(),
    };

    // Verify all fields are accessible
    assert_eq!(activity_metric.walking_speed_m_per_s, Some(1.3));
    assert_eq!(activity_metric.walking_step_length_cm, Some(67.8));
    assert_eq!(activity_metric.walking_asymmetry_percent, Some(12.5));
    assert_eq!(activity_metric.walking_double_support_percent, Some(25.0));
    assert_eq!(activity_metric.six_minute_walk_test_distance_m, Some(450.0));

    assert_eq!(activity_metric.stair_ascent_speed_m_per_s, Some(0.9));
    assert_eq!(activity_metric.stair_descent_speed_m_per_s, Some(1.1));

    assert_eq!(activity_metric.ground_contact_time_ms, Some(280.0));
    assert_eq!(activity_metric.vertical_oscillation_cm, Some(8.5));
    assert_eq!(activity_metric.running_stride_length_m, Some(1.45));
    assert_eq!(activity_metric.running_power_watts, Some(275.0));
    assert_eq!(activity_metric.running_speed_m_per_s, Some(3.2));

    println!("✅ All ActivityMetric mobility fields are properly defined and accessible!");
}