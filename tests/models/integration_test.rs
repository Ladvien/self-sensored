use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::{
    ActivityMetric, BloodPressureMetric, GpsCoordinate, HealthMetric, HeartRateMetric, IngestData,
    IngestPayload, SleepMetric, WorkoutData,
};

use self_sensored::models::db::{
    ActivityRecord, BloodPressureRecord, HeartRateRecord, SleepRecord, WorkoutRecord,
};

/// Integration tests with realistic Auto Health Export data samples
mod realistic_data_tests {
    use super::*;

    /// Create a realistic daily data sample from Auto Health Export
    fn create_realistic_daily_sample() -> IngestPayload {
        let base_date = Utc::now().date_naive();
        let morning = base_date.and_hms_opt(7, 30, 0).unwrap().and_utc();
        let evening = base_date.and_hms_opt(22, 15, 0).unwrap().and_utc();

        // Heart rate data throughout the day
        let heart_rate_metrics = vec![
            HealthMetric::HeartRate(HeartRateMetric {
                recorded_at: morning + Duration::hours(1),
                min_bpm: Some(65),
                avg_bpm: Some(72),
                max_bpm: Some(85),
                source: Some("Apple Watch Series 8".to_string()),
                context: Some("rest".to_string()),
            }),
            HealthMetric::HeartRate(HeartRateMetric {
                recorded_at: morning + Duration::hours(9),
                min_bpm: Some(120),
                avg_bpm: Some(145),
                max_bpm: Some(168),
                source: Some("Apple Watch Series 8".to_string()),
                context: Some("exercise".to_string()),
            }),
            HealthMetric::HeartRate(HeartRateMetric {
                recorded_at: evening - Duration::hours(2),
                min_bpm: Some(58),
                avg_bpm: Some(68),
                max_bpm: Some(78),
                source: Some("Apple Watch Series 8".to_string()),
                context: Some("recovery".to_string()),
            }),
        ];

        // Blood pressure readings
        let bp_metrics = vec![
            HealthMetric::BloodPressure(BloodPressureMetric {
                recorded_at: morning + Duration::hours(0, 15, 0),
                systolic: 118,
                diastolic: 75,
                pulse: Some(68),
                source: Some("Omron Blood Pressure Monitor".to_string()),
            }),
            HealthMetric::BloodPressure(BloodPressureMetric {
                recorded_at: evening,
                systolic: 125,
                diastolic: 82,
                pulse: Some(71),
                source: Some("Omron Blood Pressure Monitor".to_string()),
            }),
        ];

        // Sleep data from previous night
        let sleep_start = morning - Duration::hours(9); // 10:30 PM previous night
        let sleep_end = morning - Duration::minutes(30); // 7:00 AM
        let sleep_metrics = vec![HealthMetric::Sleep(SleepMetric {
            recorded_at: sleep_end,
            sleep_start,
            sleep_end,
            total_sleep_minutes: 420, // 7 hours of actual sleep
            deep_sleep_minutes: Some(95),
            rem_sleep_minutes: Some(88),
            awake_minutes: Some(27),
            efficiency_percentage: Some(84.2), // Good sleep efficiency
            source: Some("Apple Health".to_string()),
        })];

        // Daily activity summary
        let activity_metrics = vec![HealthMetric::Activity(ActivityMetric {
            date: base_date,
            steps: Some(12547),
            distance_meters: Some(8942.3),
            calories_burned: Some(612.5),
            active_minutes: Some(73),
            flights_climbed: Some(18),
            source: Some("iPhone Health".to_string()),
        })];

        let mut all_metrics = Vec::new();
        all_metrics.extend(heart_rate_metrics);
        all_metrics.extend(bp_metrics);
        all_metrics.extend(sleep_metrics);
        all_metrics.extend(activity_metrics);

        // Morning run workout with GPS
        let run_start = morning + Duration::hours(8);
        let run_end = run_start + Duration::minutes(32);

        let run_route = vec![
            GpsCoordinate {
                latitude: 40.7831,
                longitude: -73.9712,
                altitude_meters: Some(12.5),
                recorded_at: run_start,
            },
            GpsCoordinate {
                latitude: 40.7851,
                longitude: -73.9702,
                altitude_meters: Some(15.2),
                recorded_at: run_start + Duration::minutes(3),
            },
            GpsCoordinate {
                latitude: 40.7871,
                longitude: -73.9692,
                altitude_meters: Some(18.7),
                recorded_at: run_start + Duration::minutes(8),
            },
            GpsCoordinate {
                latitude: 40.7891,
                longitude: -73.9682,
                altitude_meters: Some(22.1),
                recorded_at: run_start + Duration::minutes(15),
            },
            GpsCoordinate {
                latitude: 40.7881,
                longitude: -73.9672,
                altitude_meters: Some(19.8),
                recorded_at: run_start + Duration::minutes(22),
            },
            GpsCoordinate {
                latitude: 40.7841,
                longitude: -73.9702,
                altitude_meters: Some(14.3),
                recorded_at: run_end - Duration::minutes(2),
            },
            GpsCoordinate {
                latitude: 40.7831,
                longitude: -73.9712,
                altitude_meters: Some(12.5),
                recorded_at: run_end,
            },
        ];

        let morning_run = WorkoutData {
            workout_type: "running".to_string(),
            start_time: run_start,
            end_time: run_end,
            total_energy_kcal: Some(287.4),
            distance_meters: Some(4200.0),
            avg_heart_rate: Some(145),
            max_heart_rate: Some(168),
            source: Some("Apple Watch Series 8".to_string()),
            route_points: Some(run_route),
        };

        IngestPayload {
            data: IngestData {
                metrics: all_metrics,
                workouts: vec![morning_run],
            },
        }
    }

    #[test]
    fn test_realistic_payload_validation() {
        let payload = create_realistic_daily_sample();

        // Validate all metrics
        for metric in &payload.data.metrics {
            assert!(
                metric.validate().is_ok(),
                "Metric {:?} should validate successfully",
                metric.metric_type()
            );
        }

        // Validate all workouts
        for workout in &payload.data.workouts {
            assert!(
                workout.validate().is_ok(),
                "Workout should validate successfully"
            );
        }
    }

    #[test]
    fn test_realistic_heart_rate_conversion() {
        let payload = create_realistic_daily_sample();

        // Find heart rate metrics and convert them
        let heart_rate_metrics: Vec<_> = payload
            .data
            .metrics
            .iter()
            .filter_map(|m| match m {
                HealthMetric::HeartRate(hr) => Some(hr),
                _ => None,
            })
            .collect();

        assert!(!heart_rate_metrics.is_empty());

        for hr_metric in heart_rate_metrics {
            let record: HeartRateRecord = hr_metric.clone().into();
            assert!(record.heart_rate >= 20);
            assert!(record.heart_rate <= 300);
            assert!(record.context.is_some());
            assert!(record.source_device.is_some());
        }
    }

    #[test]
    fn test_realistic_blood_pressure_conversion() {
        let payload = create_realistic_daily_sample();

        let bp_metrics: Vec<_> = payload
            .data
            .metrics
            .iter()
            .filter_map(|m| match m {
                HealthMetric::BloodPressure(bp) => Some(bp),
                _ => None,
            })
            .collect();

        assert!(!bp_metrics.is_empty());

        for bp_metric in bp_metrics {
            let record: BloodPressureRecord = bp_metric.clone().into();
            assert!(record.systolic >= 50);
            assert!(record.systolic <= 250);
            assert!(record.diastolic >= 30);
            assert!(record.diastolic <= 150);
            assert!(record.systolic > record.diastolic);
        }
    }

    #[test]
    fn test_realistic_sleep_conversion_and_efficiency() {
        let payload = create_realistic_daily_sample();

        let sleep_metrics: Vec<_> = payload
            .data
            .metrics
            .iter()
            .filter_map(|m| match m {
                HealthMetric::Sleep(sleep) => Some(sleep),
                _ => None,
            })
            .collect();

        assert!(!sleep_metrics.is_empty());

        for sleep_metric in sleep_metrics {
            let record: SleepRecord = sleep_metric.clone().into();

            // Verify sleep efficiency calculation
            assert!(record.sleep_efficiency.is_some());
            let efficiency = record
                .sleep_efficiency
                .unwrap()
                .to_string()
                .parse::<f32>()
                .unwrap();
            assert!(efficiency >= 0.0);
            assert!(efficiency <= 100.0);

            // For our test data, should be around 84.2%
            assert!((efficiency - 84.2).abs() < 1.0);

            // Check sleep components are reasonable
            assert!(record.deep_sleep_minutes.unwrap_or(0) >= 0);
            assert!(record.rem_sleep_minutes.unwrap_or(0) >= 0);
            assert!(record.awake_minutes.unwrap_or(0) >= 0);
        }
    }

    #[test]
    fn test_realistic_activity_aggregation() {
        let payload = create_realistic_daily_sample();

        let activity_metrics: Vec<_> = payload
            .data
            .metrics
            .iter()
            .filter_map(|m| match m {
                HealthMetric::Activity(activity) => Some(activity),
                _ => None,
            })
            .collect();

        assert!(!activity_metrics.is_empty());

        for activity_metric in activity_metrics {
            let mut record1: ActivityRecord = activity_metric.clone().into();

            // Simulate a second activity reading for the same day (common with multiple data sources)
            let additional_activity = ActivityMetric {
                date: activity_metric.date,
                steps: Some(2453), // Additional steps from another source
                distance_meters: Some(1682.7), // Additional distance
                calories_burned: Some(89.5), // Additional calories
                active_minutes: Some(12), // Additional active minutes
                flights_climbed: Some(3), // Additional flights
                source: Some("Apple Watch".to_string()),
            };

            let record2: ActivityRecord = additional_activity.into();

            // Test aggregation
            record1.aggregate_with(&record2);

            // Check aggregated totals are reasonable
            assert_eq!(record1.steps, Some(15000)); // 12547 + 2453
            assert!(
                (record1
                    .distance_meters
                    .unwrap()
                    .to_string()
                    .parse::<f64>()
                    .unwrap()
                    - 10625.0)
                    .abs()
                    < 0.1
            );
            assert!((record1.calories_burned.unwrap() as f64 - 702.0).abs() < 0.1); // 612.5 + 89.5
            assert_eq!(record1.active_minutes, Some(85)); // 73 + 12
            assert_eq!(record1.flights_climbed, Some(21)); // 18 + 3
        }
    }

    #[test]
    fn test_realistic_workout_with_gps() {
        let payload = create_realistic_daily_sample();

        assert!(!payload.data.workouts.is_empty());

        for workout in &payload.data.workouts {
            let record: WorkoutRecord = workout.clone().into();

            // Check basic workout data
            assert_eq!(record.workout_type, "running");
            assert!(record.total_energy_kcal.is_some());
            assert!(record.distance_meters.is_some());
            assert!(record.average_heart_rate.is_some());
            assert!(record.max_heart_rate.is_some());
            assert_eq!(record.duration_seconds, Some(1920)); // 32 minutes

            // Check GPS route conversion
            assert!(record.route_geometry.is_some());
            let linestring = record.route_geometry.unwrap();
            assert!(linestring.starts_with("LINESTRING("));
            assert!(linestring.contains("-73.9712")); // Start/end longitude
            assert!(linestring.contains("40.7831")); // Start/end latitude

            // Check route points conversion
            let route_points = record.route_points(workout);
            assert_eq!(route_points.len(), 7); // 7 GPS points in our test route

            // Verify route points are properly ordered
            for (i, point) in route_points.iter().enumerate() {
                assert_eq!(point.point_order, i as i32);
                assert_eq!(point.workout_id, record.id);
                assert!(point.latitude >= 40.78);
                assert!(point.latitude <= 40.79);
                assert!(point.longitude >= -73.98);
                assert!(point.longitude <= -73.96);
                assert!(point.altitude_meters.is_some());
            }
        }
    }

    #[test]
    fn test_realistic_raw_json_preservation() {
        let payload = create_realistic_daily_sample();

        // Test with heart rate metric
        if let Some(HealthMetric::HeartRate(hr_metric)) = payload.data.metrics.first() {
            let original_json = json!({
                "device": "Apple Watch Series 8",
                "firmware_version": "9.2.1",
                "measurement_confidence": 0.95,
                "environmental_factors": {
                    "temperature": 22.3,
                    "activity_level": "moderate"
                }
            });

            let record =
                HeartRateRecord::from_metric_with_raw(hr_metric.clone(), original_json.clone());

            assert_eq!(record.raw_data, Some(original_json));
            assert_eq!(record.heart_rate, hr_metric.avg_bpm.unwrap() as i32);
        }

        // Test with workout
        if let Some(workout) = payload.data.workouts.first() {
            let workout_json = json!({
                "weather_conditions": {
                    "temperature_celsius": 18.5,
                    "humidity_percent": 65,
                    "wind_speed_kmh": 12.3,
                    "condition": "partly_cloudy"
                },
                "device_battery_level": 78,
                "route_accuracy": "high",
                "total_route_points": 147
            });

            let record =
                WorkoutRecord::from_workout_with_raw(workout.clone(), workout_json.clone());

            assert_eq!(record.raw_data, Some(workout_json));
            assert_eq!(record.workout_type, workout.workout_type);
        }
    }
}

/// Tests for edge cases and error conditions
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_payload_with_invalid_metrics() {
        let invalid_bp = HealthMetric::BloodPressure(BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 49, // Below minimum of 50
            diastolic: 80,
            pulse: None,
            source: None,
        });

        assert!(invalid_bp.validate().is_err());
    }

    #[test]
    fn test_workout_with_invalid_gps() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        let invalid_route_points = vec![GpsCoordinate {
            latitude: 91.0, // Invalid - over 90 degrees
            longitude: -122.4194,
            altitude_meters: Some(50.0),
            recorded_at: start + Duration::minutes(10),
        }];

        let workout = WorkoutData {
            workout_type: "running".to_string(),
            start_time: start,
            end_time: end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source: Some("test".to_string()),
            route_points: Some(invalid_route_points),
        };

        assert!(workout.validate().is_err());
    }

    #[test]
    fn test_sleep_with_impossible_efficiency() {
        let start = Utc::now() - Duration::hours(4);
        let end = Utc::now();

        let sleep_metric = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 300, // 5 hours sleep in 4 hours bed time - impossible
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(90), // Total > bed time
            efficiency_percentage: None,
            source: Some("test".to_string()),
        };

        assert!(sleep_metric.validate().is_err());
    }

    #[test]
    fn test_large_batch_processing_simulation() {
        // Simulate processing 1000+ metrics in a batch
        let mut large_payload = IngestPayload {
            data: IngestData {
                metrics: Vec::new(),
                workouts: Vec::new(),
            },
        };

        let base_time = Utc::now() - Duration::days(1);

        // Generate 1000 heart rate readings over 24 hours
        for i in 0..1000 {
            let reading_time = base_time + Duration::minutes(i as i64 * 1.44); // ~1.44 minutes apart
            let hr_metric = HealthMetric::HeartRate(HeartRateMetric {
                recorded_at: reading_time,
                min_bpm: Some(65 + (i % 30) as i16),
                avg_bpm: Some(75 + (i % 40) as i16),
                max_bpm: Some(85 + (i % 50) as i16),
                source: Some("Apple Watch".to_string()),
                context: Some(if i % 10 < 7 { "rest" } else { "exercise" }.to_string()),
            });
            large_payload.data.metrics.push(hr_metric);
        }

        // Validate all metrics can be processed
        let mut valid_count = 0;
        let mut invalid_count = 0;

        for metric in &large_payload.data.metrics {
            match metric.validate() {
                Ok(_) => valid_count += 1,
                Err(_) => invalid_count += 1,
            }
        }

        assert_eq!(valid_count, 1000);
        assert_eq!(invalid_count, 0);

        // Test conversion to database records
        let heart_rate_metrics: Vec<_> = large_payload
            .data
            .metrics
            .iter()
            .filter_map(|m| match m {
                HealthMetric::HeartRate(hr) => Some(hr),
                _ => None,
            })
            .collect();

        assert_eq!(heart_rate_metrics.len(), 1000);

        // Convert a sample to verify database model conversion works
        for hr in heart_rate_metrics.iter().take(10) {
            let record: HeartRateRecord = (*hr).clone().into();
            assert!(record.heart_rate >= 75);
            assert!(record.heart_rate <= 135);
            assert!(record.context.is_some());
        }
    }
}
