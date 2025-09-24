use self_sensored::models::ios_models::{
    IosIngestData, IosIngestPayload, IosMetricData, IosMetricDataPoint, IosWorkout,
};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
mod ios_models_correct_format_tests {
    use super::*;

    #[test]
    fn test_correct_format_deserialization() {
        // Test payload with new HashMap format from Auto Health Export
        let json = r#"{
            "data": {
                "metrics": {
                    "HKQuantityTypeIdentifierHeartRate": [
                        {
                            "date": "2024-01-01 12:00:00",
                            "qty": 65.0,
                            "units": "count/min",
                            "source": "Apple Watch"
                        },
                        {
                            "date": "2024-01-01 12:05:00",
                            "qty": 72.0,
                            "units": "count/min",
                            "source": "Apple Watch"
                        }
                    ],
                    "HKQuantityTypeIdentifierStepCount": [
                        {
                            "date": "2024-01-01 08:00:00",
                            "qty": 1500.0,
                            "units": "count",
                            "source": "iPhone"
                        }
                    ]
                },
                "workouts": []
            }
        }"#;

        let payload: Result<IosIngestPayload, _> = serde_json::from_str(json);
        assert!(
            payload.is_ok(),
            "Failed to parse new format: {:?}",
            payload.err()
        );

        let payload = payload.unwrap();

        // Verify the correct variant is used
        match &payload.data {
            IosIngestData::Correct { metrics, workouts } => {
                assert_eq!(metrics.len(), 2, "Should have 2 metric types");
                assert!(metrics.contains_key("HKQuantityTypeIdentifierHeartRate"));
                assert!(metrics.contains_key("HKQuantityTypeIdentifierStepCount"));

                let heart_rate_data = metrics.get("HKQuantityTypeIdentifierHeartRate").unwrap();
                assert_eq!(
                    heart_rate_data.len(),
                    2,
                    "Should have 2 heart rate readings"
                );

                assert_eq!(workouts.len(), 0, "Should have no workouts");
            }
            IosIngestData::Legacy { .. } => {
                panic!("Should deserialize to Correct variant, not Legacy");
            }
        }
    }

    #[test]
    fn test_legacy_format_backward_compatibility() {
        // Test payload with old Vec format for backward compatibility
        let json = r#"{
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "units": "count/min",
                        "data": [
                            {
                                "date": "2024-01-01 12:00:00",
                                "qty": 65.0,
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    {
                        "name": "HKQuantityTypeIdentifierStepCount",
                        "units": "count",
                        "data": [
                            {
                                "date": "2024-01-01 08:00:00",
                                "qty": 1500.0,
                                "source": "iPhone"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        }"#;

        let payload: Result<IosIngestPayload, _> = serde_json::from_str(json);
        assert!(
            payload.is_ok(),
            "Failed to parse legacy format: {:?}",
            payload.err()
        );

        let payload = payload.unwrap();

        // Verify the legacy variant is used
        match &payload.data {
            IosIngestData::Legacy { metrics, workouts } => {
                assert_eq!(metrics.len(), 2, "Should have 2 metrics");
                assert_eq!(metrics[0].name, "HKQuantityTypeIdentifierHeartRate");
                assert_eq!(metrics[1].name, "HKQuantityTypeIdentifierStepCount");
                assert_eq!(workouts.len(), 0, "Should have no workouts");
            }
            IosIngestData::Correct { .. } => {
                panic!("Should deserialize to Legacy variant, not Correct");
            }
        }
    }

    #[test]
    fn test_normalize_correct_format() {
        let mut metrics = HashMap::new();
        metrics.insert(
            "HKQuantityTypeIdentifierHeartRate".to_string(),
            vec![IosMetricDataPoint {
                date: Some("2024-01-01 12:00:00".to_string()),
                qty: Some(65.0),
                value: None,
                units: Some("count/min".to_string()),
                source: Some("Apple Watch".to_string()),
                start: None,
                end: None,
                extra: HashMap::new(),
            }],
        );

        let data = IosIngestData::Correct {
            metrics,
            workouts: vec![],
        };

        let (normalized_metrics, normalized_workouts) = data.normalize();

        assert_eq!(normalized_metrics.len(), 1);
        assert!(normalized_metrics.contains_key("HKQuantityTypeIdentifierHeartRate"));
        assert_eq!(normalized_workouts.len(), 0);
    }

    #[test]
    fn test_normalize_legacy_format() {
        use self_sensored::models::ios_models::IosMetric;

        let data = IosIngestData::Legacy {
            metrics: vec![IosMetric {
                name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                units: Some("count/min".to_string()),
                data: vec![IosMetricData {
                    date: Some("2024-01-01 12:00:00".to_string()),
                    qty: Some(65.0),
                    value: None,
                    source: Some("Apple Watch".to_string()),
                    start: None,
                    end: None,
                    extra: HashMap::new(),
                }],
            }],
            workouts: vec![],
        };

        let (normalized_metrics, normalized_workouts) = data.normalize();

        assert_eq!(normalized_metrics.len(), 1);
        assert!(normalized_metrics.contains_key("HKQuantityTypeIdentifierHeartRate"));
        let heart_rate_data = normalized_metrics
            .get("HKQuantityTypeIdentifierHeartRate")
            .unwrap();
        assert_eq!(heart_rate_data.len(), 1);
        assert_eq!(heart_rate_data[0].qty, Some(65.0));
        assert_eq!(normalized_workouts.len(), 0);
    }

    #[test]
    fn test_to_internal_format_with_correct_format() {
        let mut metrics = HashMap::new();

        // Add heart rate data
        metrics.insert(
            "HKQuantityTypeIdentifierHeartRate".to_string(),
            vec![IosMetricDataPoint {
                date: Some("2024-01-01 12:00:00".to_string()),
                qty: Some(65.0),
                value: None,
                units: Some("count/min".to_string()),
                source: Some("Apple Watch".to_string()),
                start: None,
                end: None,
                extra: HashMap::new(),
            }],
        );

        // Add step count data
        metrics.insert(
            "HKQuantityTypeIdentifierStepCount".to_string(),
            vec![IosMetricDataPoint {
                date: Some("2024-01-01 08:00:00".to_string()),
                qty: Some(1500.0),
                value: None,
                units: Some("count".to_string()),
                source: Some("iPhone".to_string()),
                start: None,
                end: None,
                extra: HashMap::new(),
            }],
        );

        let payload = IosIngestPayload {
            data: IosIngestData::Correct {
                metrics,
                workouts: vec![],
            },
        };

        let user_id = Uuid::new_v4();
        let internal = payload.to_internal_format(user_id);

        // Should have 1 heart rate and 1 activity metric
        assert_eq!(internal.data.metrics.len(), 2);

        // Verify at least one is a heart rate metric and one is an activity metric
        use self_sensored::models::HealthMetric;
        let mut has_heart_rate = false;
        let mut has_activity = false;

        for metric in &internal.data.metrics {
            match metric {
                HealthMetric::HeartRate(_) => has_heart_rate = true,
                HealthMetric::Activity(_) => has_activity = true,
                _ => {}
            }
        }

        assert!(has_heart_rate, "Should have converted heart rate metric");
        assert!(has_activity, "Should have converted activity metric");
    }

    #[test]
    fn test_mixed_healthkit_and_simplified_names() {
        let mut metrics = HashMap::new();

        // HealthKit identifier format
        metrics.insert(
            "HKQuantityTypeIdentifierHeartRate".to_string(),
            vec![IosMetricDataPoint {
                date: Some("2024-01-01 12:00:00".to_string()),
                qty: Some(65.0),
                value: None,
                units: Some("count/min".to_string()),
                source: Some("Apple Watch".to_string()),
                start: None,
                end: None,
                extra: HashMap::new(),
            }],
        );

        // Simplified name format
        metrics.insert(
            "heart_rate".to_string(),
            vec![IosMetricDataPoint {
                date: Some("2024-01-01 12:05:00".to_string()),
                qty: Some(72.0),
                value: None,
                units: Some("bpm".to_string()),
                source: Some("Apple Watch".to_string()),
                start: None,
                end: None,
                extra: HashMap::new(),
            }],
        );

        let payload = IosIngestPayload {
            data: IosIngestData::Correct {
                metrics,
                workouts: vec![],
            },
        };

        let user_id = Uuid::new_v4();
        let internal = payload.to_internal_format(user_id);

        // Should have 2 heart rate metrics
        use self_sensored::models::HealthMetric;
        let heart_rate_count = internal
            .data
            .metrics
            .iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();

        assert_eq!(
            heart_rate_count, 2,
            "Should have converted both heart rate formats"
        );
    }

    #[test]
    fn test_workout_conversion_with_correct_format() {
        let workout = IosWorkout {
            name: Some("HKWorkoutActivityTypeRunning".to_string()),
            start: Some("2024-01-01 07:00:00".to_string()),
            end: Some("2024-01-01 07:30:00".to_string()),
            source: Some("Apple Watch".to_string()),
            extra: {
                let mut extra = HashMap::new();
                extra.insert("total_energy_kcal".to_string(), serde_json::json!(250.0));
                extra.insert("distance_meters".to_string(), serde_json::json!(5000.0));
                extra
            },
        };

        let payload = IosIngestPayload {
            data: IosIngestData::Correct {
                metrics: HashMap::new(),
                workouts: vec![workout],
            },
        };

        let user_id = Uuid::new_v4();
        let internal = payload.to_internal_format(user_id);

        assert_eq!(internal.data.workouts.len(), 1);
        let workout_data = &internal.data.workouts[0];
        use self_sensored::models::WorkoutType;
        assert_eq!(workout_data.workout_type, WorkoutType::Running);
        assert_eq!(workout_data.total_energy_kcal, Some(250.0));
        assert_eq!(workout_data.distance_meters, Some(5000.0));
    }
}
