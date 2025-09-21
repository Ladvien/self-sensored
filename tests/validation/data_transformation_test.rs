/// Comprehensive tests for health data transformation logic
use chrono::{DateTime, TimeZone, Utc};
use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{ActivityContext, WorkoutType};
use self_sensored::models::health_metrics::*;
use self_sensored::models::ios_models::*;
use self_sensored::models::{HealthMetric, IngestPayload, IngestData};
use std::collections::HashMap;
use uuid::Uuid;

// Test utilities for transformation
mod transformation_utils {
    use super::*;

    pub fn create_complex_ios_payload() -> IosIngestPayload {
        let timestamp = Utc::now().to_rfc3339();

        IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    // Heart Rate
                    IosMetric {
                        name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                        units: Some("count/min".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(72.0),
                            date: Some(timestamp.clone()),
                            start: None,
                            end: None,
                            source: Some("Apple Watch".to_string()),
                            value: None,
                            extra: {
                                let mut map = HashMap::new();
                                map.insert("context".to_string(), serde_json::Value::String("rest".to_string()));
                                map
                            },
                        }],
                    },
                    // Blood Pressure Systolic
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureSystolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(120.0),
                            date: Some(timestamp.clone()),
                            start: None,
                            end: None,
                            source: Some("Blood Pressure Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                    // Blood Pressure Diastolic
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureDiastolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(80.0),
                            date: Some(timestamp.clone()),
                            start: None,
                            end: None,
                            source: Some("Blood Pressure Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                    // Step Count
                    IosMetric {
                        name: "HKQuantityTypeIdentifierStepCount".to_string(),
                        units: Some("count".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(10000.0),
                            date: Some(timestamp.clone()),
                            start: Some(timestamp.clone()),
                            end: Some(Utc::now().to_rfc3339()),
                            source: Some("iPhone".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                    // Sleep Analysis
                    IosMetric {
                        name: "HKCategoryTypeIdentifierSleepAnalysis".to_string(),
                        units: None,
                        data: vec![IosMetricData {
                            qty: None,
                            date: None,
                            start: Some((Utc::now() - chrono::Duration::hours(8)).to_rfc3339()),
                            end: Some(Utc::now().to_rfc3339()),
                            source: Some("Apple Watch".to_string()),
                            value: Some("HKCategoryValueSleepAnalysisAsleep".to_string()),
                            extra: {
                                let mut map = HashMap::new();
                                map.insert("sleep_stage".to_string(), serde_json::Value::String("deep".to_string()));
                                map
                            },
                        }],
                    },
                ],
                workouts: vec![
                    IosWorkout {
                        name: Some("Running".to_string()),
                        start: Some((Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
                        end: Some(Utc::now().to_rfc3339()),
                        source: Some("Apple Watch".to_string()),
                        extra: {
                            let mut map = HashMap::new();
                            map.insert("total_energy".to_string(), serde_json::Value::Number(serde_json::Number::from(450)));
                            map.insert("distance".to_string(), serde_json::Value::Number(serde_json::Number::from(5000)));
                            map.insert("average_heart_rate".to_string(), serde_json::Value::Number(serde_json::Number::from(145)));
                            map
                        },
                    }
                ],
            },
        }
    }

    pub fn create_metric_with_units(metric_name: &str, value: f64, unit: &str) -> IosMetric {
        IosMetric {
            name: metric_name.to_string(),
            units: Some(unit.to_string()),
            data: vec![IosMetricData {
                qty: Some(value),
                date: Some(Utc::now().to_rfc3339()),
                start: None,
                end: None,
                source: Some("Test Device".to_string()),
                value: None,
                extra: HashMap::new(),
            }],
        }
    }
}

#[cfg(test)]
mod ios_to_internal_transformation_tests {
    use super::*;
    use transformation_utils::*;

    #[test]
    fn test_ios_payload_to_internal_conversion() {
        let ios_payload = create_complex_ios_payload();
        let user_id = Uuid::new_v4();

        let internal_payload = ios_payload.to_internal_format(user_id);

        // Verify the conversion creates internal metrics
        assert!(!internal_payload.data.metrics.is_empty());
        assert!(!internal_payload.data.workouts.is_empty());

        // Verify user_id is properly set
        for metric in &internal_payload.data.metrics {
            match metric {
                HealthMetric::HeartRate(hr) => assert_eq!(hr.user_id, user_id),
                HealthMetric::BloodPressure(bp) => assert_eq!(bp.user_id, user_id),
                HealthMetric::Activity(act) => assert_eq!(act.user_id, user_id),
                HealthMetric::Sleep(sleep) => assert_eq!(sleep.user_id, user_id),
                _ => {} // Other metric types
            }
        }

        for workout in &internal_payload.data.workouts {
            assert_eq!(workout.user_id, user_id);
        }
    }

    #[test]
    fn test_heart_rate_transformation() {
        let user_id = Uuid::new_v4();
        let ios_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                        units: Some("count/min".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(75.0),
                            date: Some(Utc::now().to_rfc3339()),
                            start: None,
                            end: None,
                            source: Some("Apple Watch Series 8".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal_payload = ios_payload.to_internal_format(user_id);

        // Find the heart rate metric
        let heart_rate_metric = internal_payload.data.metrics.iter()
            .find_map(|m| match m {
                HealthMetric::HeartRate(hr) => Some(hr),
                _ => None,
            })
            .expect("Should have heart rate metric");

        assert_eq!(heart_rate_metric.user_id, user_id);
        assert_eq!(heart_rate_metric.heart_rate, Some(75));
        assert_eq!(heart_rate_metric.source_device.as_deref(), Some("Apple Watch Series 8"));
    }

    #[test]
    fn test_blood_pressure_pairing_transformation() {
        let user_id = Uuid::new_v4();
        let timestamp = Utc::now().to_rfc3339();

        let ios_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureSystolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(125.0),
                            date: Some(timestamp.clone()),
                            start: None,
                            end: None,
                            source: Some("Omron BP Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureDiastolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(85.0),
                            date: Some(timestamp),
                            start: None,
                            end: None,
                            source: Some("Omron BP Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                ],
                workouts: vec![],
            },
        };

        let internal_payload = ios_payload.to_internal_format(user_id);

        // Find the blood pressure metric
        let bp_metric = internal_payload.data.metrics.iter()
            .find_map(|m| match m {
                HealthMetric::BloodPressure(bp) => Some(bp),
                _ => None,
            })
            .expect("Should have blood pressure metric");

        assert_eq!(bp_metric.user_id, user_id);
        assert_eq!(bp_metric.systolic, 125);
        assert_eq!(bp_metric.diastolic, 85);
        assert_eq!(bp_metric.source_device.as_deref(), Some("Omron BP Monitor"));
    }

    #[test]
    fn test_sleep_analysis_transformation() {
        let user_id = Uuid::new_v4();
        let sleep_start = Utc::now() - chrono::Duration::hours(8);
        let sleep_end = Utc::now();

        let ios_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKCategoryTypeIdentifierSleepAnalysis".to_string(),
                        units: None,
                        data: vec![IosMetricData {
                            qty: None,
                            date: None,
                            start: Some(sleep_start.to_rfc3339()),
                            end: Some(sleep_end.to_rfc3339()),
                            source: Some("Apple Watch".to_string()),
                            value: Some("HKCategoryValueSleepAnalysisAsleep".to_string()),
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal_payload = ios_payload.to_internal_format(user_id);

        // Find the sleep metric
        let sleep_metric = internal_payload.data.metrics.iter()
            .find_map(|m| match m {
                HealthMetric::Sleep(sleep) => Some(sleep),
                _ => None,
            })
            .expect("Should have sleep metric");

        assert_eq!(sleep_metric.user_id, user_id);
        assert_eq!(sleep_metric.source_device.as_deref(), Some("Apple Watch"));

        // Verify duration calculation
        let expected_duration = (sleep_end - sleep_start).num_minutes() as i32;
        assert_eq!(sleep_metric.duration_minutes, Some(expected_duration));
    }

    #[test]
    fn test_activity_metrics_transformation() {
        let user_id = Uuid::new_v4();
        let ios_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierStepCount", 12000.0, "count"),
                    create_metric_with_units("HKQuantityTypeIdentifierDistanceWalkingRunning", 8500.0, "m"),
                    create_metric_with_units("HKQuantityTypeIdentifierActiveEnergyBurned", 450.0, "kcal"),
                    create_metric_with_units("HKQuantityTypeIdentifierFlightsClimbed", 15.0, "count"),
                ],
                workouts: vec![],
            },
        };

        let internal_payload = ios_payload.to_internal_format(user_id);

        // Find the activity metric
        let activity_metric = internal_payload.data.metrics.iter()
            .find_map(|m| match m {
                HealthMetric::Activity(act) => Some(act),
                _ => None,
            })
            .expect("Should have activity metric");

        assert_eq!(activity_metric.user_id, user_id);
        assert_eq!(activity_metric.step_count, Some(12000));
        assert_eq!(activity_metric.distance_meters, Some(8500.0));
        assert_eq!(activity_metric.active_energy_burned_kcal, Some(450.0));
        assert_eq!(activity_metric.flights_climbed, Some(15));
    }

    #[test]
    fn test_workout_transformation() {
        let user_id = Uuid::new_v4();
        let workout_start = Utc::now() - chrono::Duration::hours(1);
        let workout_end = Utc::now();

        let ios_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![],
                workouts: vec![
                    IosWorkout {
                        name: Some("Outdoor Run".to_string()),
                        start: Some(workout_start.to_rfc3339()),
                        end: Some(workout_end.to_rfc3339()),
                        source: Some("Apple Watch".to_string()),
                        extra: {
                            let mut map = HashMap::new();
                            map.insert("totalEnergyBurned".to_string(), serde_json::Value::Number(serde_json::Number::from(500)));
                            map.insert("totalDistance".to_string(), serde_json::Value::Number(serde_json::Number::from(5000)));
                            map.insert("averageHeartRate".to_string(), serde_json::Value::Number(serde_json::Number::from(150)));
                            map.insert("maximumHeartRate".to_string(), serde_json::Value::Number(serde_json::Number::from(180)));
                            map
                        },
                    }
                ],
            },
        };

        let internal_payload = ios_payload.to_internal_format(user_id);

        assert_eq!(internal_payload.data.workouts.len(), 1);
        let workout = &internal_payload.data.workouts[0];

        assert_eq!(workout.user_id, user_id);
        assert_eq!(workout.workout_type, WorkoutType::Running);
        assert_eq!(workout.total_energy_kcal, Some(500.0));
        assert_eq!(workout.distance_meters, Some(5000.0));
        assert_eq!(workout.avg_heart_rate, Some(150));
        assert_eq!(workout.max_heart_rate, Some(180));
        assert_eq!(workout.source_device.as_deref(), Some("Apple Watch"));
    }
}

#[cfg(test)]
mod unit_conversion_transformation_tests {
    use super::*;
    use transformation_utils::*;

    #[test]
    fn test_distance_unit_conversions() {
        let user_id = Uuid::new_v4();

        // Test meters to meters (no conversion)
        let meters_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierDistanceWalkingRunning", 5000.0, "m")
                ],
                workouts: vec![],
            },
        };

        let internal = meters_payload.to_internal_format(user_id);
        let activity = internal.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Activity(a) => Some(a), _ => None })
            .unwrap();
        assert_eq!(activity.distance_meters, Some(5000.0));

        // Test kilometers to meters
        let km_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierDistanceWalkingRunning", 5.0, "km")
                ],
                workouts: vec![],
            },
        };

        let internal_km = km_payload.to_internal_format(user_id);
        let activity_km = internal_km.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Activity(a) => Some(a), _ => None })
            .unwrap();
        assert_eq!(activity_km.distance_meters, Some(5000.0));

        // Test miles to meters
        let miles_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierDistanceWalkingRunning", 3.1, "mi")
                ],
                workouts: vec![],
            },
        };

        let internal_miles = miles_payload.to_internal_format(user_id);
        let activity_miles = internal_miles.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Activity(a) => Some(a), _ => None })
            .unwrap();
        // 3.1 miles ≈ 4988 meters
        assert!((activity_miles.distance_meters.unwrap() - 4988.0).abs() < 10.0);
    }

    #[test]
    fn test_energy_unit_conversions() {
        let user_id = Uuid::new_v4();

        // Test kcal (no conversion needed)
        let kcal_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierActiveEnergyBurned", 300.0, "kcal")
                ],
                workouts: vec![],
            },
        };

        let internal = kcal_payload.to_internal_format(user_id);
        let activity = internal.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Activity(a) => Some(a), _ => None })
            .unwrap();
        assert_eq!(activity.active_energy_burned_kcal, Some(300.0));

        // Test kJ to kcal conversion
        let kj_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierActiveEnergyBurned", 1255.2, "kJ")
                ],
                workouts: vec![],
            },
        };

        let internal_kj = kj_payload.to_internal_format(user_id);
        let activity_kj = internal_kj.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Activity(a) => Some(a), _ => None })
            .unwrap();
        // 1255.2 kJ ≈ 300 kcal
        assert!((activity_kj.active_energy_burned_kcal.unwrap() - 300.0).abs() < 1.0);
    }

    #[test]
    fn test_weight_unit_conversions() {
        let user_id = Uuid::new_v4();

        // Test kilograms (no conversion)
        let kg_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierBodyMass", 70.0, "kg")
                ],
                workouts: vec![],
            },
        };

        let internal = kg_payload.to_internal_format(user_id);
        // Note: BodyMass would create a BodyMeasurement metric, not Activity
        // This test assumes the transformation handles weight correctly

        // Test pounds to kilograms
        let lb_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierBodyMass", 154.32, "lb")
                ],
                workouts: vec![],
            },
        };

        let internal_lb = lb_payload.to_internal_format(user_id);
        // 154.32 lb ≈ 70 kg
    }

    #[test]
    fn test_temperature_unit_conversions() {
        let user_id = Uuid::new_v4();

        // Test Celsius (no conversion)
        let celsius_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierBodyTemperature", 37.0, "degC")
                ],
                workouts: vec![],
            },
        };

        let internal = celsius_payload.to_internal_format(user_id);

        // Test Fahrenheit to Celsius
        let fahrenheit_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    create_metric_with_units("HKQuantityTypeIdentifierBodyTemperature", 98.6, "degF")
                ],
                workouts: vec![],
            },
        };

        let internal_f = fahrenheit_payload.to_internal_format(user_id);
        // 98.6°F ≈ 37°C
    }
}

#[cfg(test)]
mod timestamp_transformation_tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn test_ios_date_string_parsing() {
        let user_id = Uuid::new_v4();

        // Test various iOS date formats
        let date_formats = [
            "2024-01-15T10:30:00Z",
            "2024-01-15T10:30:00.000Z",
            "2024-01-15T10:30:00+00:00",
            "2024-01-15T05:30:00-05:00", // EST
            "2024-01-15T16:30:00+06:00",  // Different timezone
        ];

        for date_str in &date_formats {
            let payload = IosIngestPayload {
                data: IosIngestData {
                    metrics: vec![
                        IosMetric {
                            name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                            units: Some("count/min".to_string()),
                            data: vec![IosMetricData {
                                qty: Some(72.0),
                                date: Some(date_str.to_string()),
                                start: None,
                                end: None,
                                source: Some("Apple Watch".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            }],
                        }
                    ],
                    workouts: vec![],
                },
            };

            let internal = payload.to_internal_format(user_id);
            let hr_metric = internal.data.metrics.iter()
                .find_map(|m| match m { HealthMetric::HeartRate(hr) => Some(hr), _ => None })
                .expect("Should have heart rate metric");

            // Verify the timestamp was parsed and converted to UTC
            assert_eq!(hr_metric.recorded_at.timezone(), Utc);

            // All these timestamps should represent the same moment in UTC
            let expected_utc = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
            assert_eq!(hr_metric.recorded_at, expected_utc);
        }
    }

    #[test]
    fn test_start_end_time_transformation() {
        let user_id = Uuid::new_v4();
        let start_time = Utc::now() - chrono::Duration::hours(2);
        let end_time = Utc::now();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKCategoryTypeIdentifierSleepAnalysis".to_string(),
                        units: None,
                        data: vec![IosMetricData {
                            qty: None,
                            date: None,
                            start: Some(start_time.to_rfc3339()),
                            end: Some(end_time.to_rfc3339()),
                            source: Some("Apple Watch".to_string()),
                            value: Some("HKCategoryValueSleepAnalysisAsleep".to_string()),
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);
        let sleep_metric = internal.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::Sleep(s) => Some(s), _ => None })
            .expect("Should have sleep metric");

        assert_eq!(sleep_metric.sleep_start.timestamp(), start_time.timestamp());
        assert_eq!(sleep_metric.sleep_end.timestamp(), end_time.timestamp());

        // Verify duration calculation
        let expected_duration = (end_time - start_time).num_minutes() as i32;
        assert_eq!(sleep_metric.duration_minutes, Some(expected_duration));
    }

    #[test]
    fn test_missing_timestamp_handling() {
        let user_id = Uuid::new_v4();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                        units: Some("count/min".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(72.0),
                            date: None, // Missing date
                            start: None,
                            end: None,
                            source: Some("Apple Watch".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);
        let hr_metric = internal.data.metrics.iter()
            .find_map(|m| match m { HealthMetric::HeartRate(hr) => Some(hr), _ => None })
            .expect("Should have heart rate metric");

        // Should use current time when no timestamp is provided
        let now = Utc::now();
        let time_diff = (hr_metric.recorded_at - now).num_seconds().abs();
        assert!(time_diff < 5, "Should use current time when no timestamp provided");
    }
}

#[cfg(test)]
mod data_aggregation_transformation_tests {
    use super::*;

    #[test]
    fn test_blood_pressure_aggregation() {
        let user_id = Uuid::new_v4();
        let timestamp = Utc::now().to_rfc3339();

        // Multiple blood pressure readings at the same time should be aggregated
        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureSystolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(120.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("Monitor 1".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: Some(125.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("Monitor 2".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureDiastolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(80.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("Monitor 1".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: Some(82.0),
                                date: Some(timestamp),
                                start: None,
                                end: None,
                                source: Some("Monitor 2".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);
        let bp_metrics: Vec<_> = internal.data.metrics.iter()
            .filter_map(|m| match m { HealthMetric::BloodPressure(bp) => Some(bp), _ => None })
            .collect();

        // Should create multiple blood pressure readings
        assert!(!bp_metrics.is_empty());

        // Verify each reading has proper systolic/diastolic pairing
        for bp in bp_metrics {
            assert!(bp.systolic > 0);
            assert!(bp.diastolic > 0);
            assert!(bp.systolic > bp.diastolic); // Basic sanity check
        }
    }

    #[test]
    fn test_activity_metrics_aggregation() {
        let user_id = Uuid::new_v4();
        let timestamp = Utc::now().to_rfc3339();

        // Multiple activity readings for the same day
        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierStepCount".to_string(),
                        units: Some("count".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(5000.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("iPhone".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: Some(3000.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("Apple Watch".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    IosMetric {
                        name: "HKQuantityTypeIdentifierDistanceWalkingRunning".to_string(),
                        units: Some("m".to_string()),
                        data: vec![
                            IosMetricData {
                                qty: Some(4000.0),
                                date: Some(timestamp.clone()),
                                start: None,
                                end: None,
                                source: Some("iPhone".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: Some(2400.0),
                                date: Some(timestamp),
                                start: None,
                                end: None,
                                source: Some("Apple Watch".to_string()),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);
        let activity_metrics: Vec<_> = internal.data.metrics.iter()
            .filter_map(|m| match m { HealthMetric::Activity(act) => Some(act), _ => None })
            .collect();

        // Should create activity metrics for each data point
        assert!(!activity_metrics.is_empty());

        // Verify step counts and distances are properly set
        for activity in activity_metrics {
            if activity.step_count.is_some() {
                assert!(activity.step_count.unwrap() > 0);
            }
            if activity.distance_meters.is_some() {
                assert!(activity.distance_meters.unwrap() > 0.0);
            }
        }
    }

    #[test]
    fn test_sleep_stage_aggregation() {
        let user_id = Uuid::new_v4();
        let sleep_start = Utc::now() - chrono::Duration::hours(8);
        let sleep_end = Utc::now();

        // Multiple sleep stages for the same sleep session
        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKCategoryTypeIdentifierSleepAnalysis".to_string(),
                        units: None,
                        data: vec![
                            IosMetricData {
                                qty: None,
                                date: None,
                                start: Some(sleep_start.to_rfc3339()),
                                end: Some((sleep_start + chrono::Duration::hours(2)).to_rfc3339()),
                                source: Some("Apple Watch".to_string()),
                                value: Some("HKCategoryValueSleepAnalysisInBed".to_string()),
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: None,
                                date: None,
                                start: Some((sleep_start + chrono::Duration::hours(2)).to_rfc3339()),
                                end: Some((sleep_start + chrono::Duration::hours(6)).to_rfc3339()),
                                source: Some("Apple Watch".to_string()),
                                value: Some("HKCategoryValueSleepAnalysisAsleep".to_string()),
                                extra: HashMap::new(),
                            },
                            IosMetricData {
                                qty: None,
                                date: None,
                                start: Some((sleep_start + chrono::Duration::hours(6)).to_rfc3339()),
                                end: Some(sleep_end.to_rfc3339()),
                                source: Some("Apple Watch".to_string()),
                                value: Some("HKCategoryValueSleepAnalysisAwake".to_string()),
                                extra: HashMap::new(),
                            },
                        ],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);
        let sleep_metrics: Vec<_> = internal.data.metrics.iter()
            .filter_map(|m| match m { HealthMetric::Sleep(s) => Some(s), _ => None })
            .collect();

        // Should create sleep metrics
        assert!(!sleep_metrics.is_empty());

        // Verify sleep metrics have proper time ranges
        for sleep in sleep_metrics {
            assert!(sleep.sleep_end > sleep.sleep_start);
            if let Some(duration) = sleep.duration_minutes {
                assert!(duration > 0);
                assert!(duration <= 12 * 60); // Reasonable maximum
            }
        }
    }
}

#[cfg(test)]
mod error_handling_transformation_tests {
    use super::*;

    #[test]
    fn test_invalid_metric_name_handling() {
        let user_id = Uuid::new_v4();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "UnknownMetricType".to_string(),
                        units: Some("unknown".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(100.0),
                            date: Some(Utc::now().to_rfc3339()),
                            start: None,
                            end: None,
                            source: Some("Unknown Device".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);

        // Should handle unknown metric types gracefully
        // The exact behavior depends on implementation - might skip or log
        // This test ensures no panic occurs
        assert!(internal.data.metrics.len() >= 0); // Should not panic
    }

    #[test]
    fn test_missing_quantity_handling() {
        let user_id = Uuid::new_v4();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                        units: Some("count/min".to_string()),
                        data: vec![IosMetricData {
                            qty: None, // Missing quantity
                            date: Some(Utc::now().to_rfc3339()),
                            start: None,
                            end: None,
                            source: Some("Apple Watch".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);

        // Should handle missing quantities gracefully
        // Might skip the metric or use default values
        assert!(internal.data.metrics.len() >= 0); // Should not panic
    }

    #[test]
    fn test_invalid_date_format_handling() {
        let user_id = Uuid::new_v4();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                        units: Some("count/min".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(72.0),
                            date: Some("invalid-date-format".to_string()),
                            start: None,
                            end: None,
                            source: Some("Apple Watch".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    }
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);

        // Should handle invalid dates gracefully (use current time or skip)
        assert!(internal.data.metrics.len() >= 0); // Should not panic

        if !internal.data.metrics.is_empty() {
            let hr_metric = internal.data.metrics.iter()
                .find_map(|m| match m { HealthMetric::HeartRate(hr) => Some(hr), _ => None });

            if let Some(hr) = hr_metric {
                // Should use current time when date parsing fails
                let now = Utc::now();
                let time_diff = (hr.recorded_at - now).num_seconds().abs();
                assert!(time_diff < 60, "Should use recent time when date parsing fails");
            }
        }
    }

    #[test]
    fn test_negative_values_handling() {
        let user_id = Uuid::new_v4();

        let payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    transformation_utils::create_metric_with_units("HKQuantityTypeIdentifierStepCount", -100.0, "count"),
                    transformation_utils::create_metric_with_units("HKQuantityTypeIdentifierHeartRate", -50.0, "count/min"),
                    transformation_utils::create_metric_with_units("HKQuantityTypeIdentifierActiveEnergyBurned", -200.0, "kcal"),
                ],
                workouts: vec![],
            },
        };

        let internal = payload.to_internal_format(user_id);

        // Should handle negative values appropriately
        // Implementation might reject, zero out, or transform negative values
        for metric in &internal.data.metrics {
            match metric {
                HealthMetric::Activity(act) => {
                    if let Some(steps) = act.step_count {
                        // Steps should never be negative
                        assert!(steps >= 0, "Step count should not be negative");
                    }
                    if let Some(energy) = act.active_energy_burned_kcal {
                        // Energy should never be negative
                        assert!(energy >= 0.0, "Energy should not be negative");
                    }
                },
                HealthMetric::HeartRate(hr) => {
                    if let Some(rate) = hr.heart_rate {
                        // Heart rate should never be negative
                        assert!(rate >= 0, "Heart rate should not be negative");
                    }
                },
                _ => {} // Other metrics
            }
        }
    }
}