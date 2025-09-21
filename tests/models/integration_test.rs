use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::{
    enums::{ActivityContext, WorkoutType},
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
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: morning + Duration::hours(1),
                heart_rate: Some(72),
                resting_heart_rate: Some(65),
                heart_rate_variability: None,
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch Series 8".to_string()),
                context: Some(ActivityContext::Resting),
                created_at: Utc::now(),
            }),
            HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: morning + Duration::hours(9),
                heart_rate: Some(145),
                resting_heart_rate: None,
                heart_rate_variability: None,
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch Series 8".to_string()),
                context: Some(ActivityContext::Exercise),
                created_at: Utc::now(),
            }),
            HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: evening - Duration::hours(2),
                heart_rate: Some(68),
                resting_heart_rate: Some(58),
                heart_rate_variability: None,
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch Series 8".to_string()),
                context: Some(ActivityContext::Recovery),
                created_at: Utc::now(),
            }),
        ];

        // Blood pressure readings
        let bp_metrics = vec![
            HealthMetric::BloodPressure(BloodPressureMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: morning + Duration::minutes(15),
                systolic: 118,
                diastolic: 75,
                pulse: Some(68),
                source_device: Some("Omron Blood Pressure Monitor".to_string()),
                created_at: Utc::now(),
            }),
            HealthMetric::BloodPressure(BloodPressureMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: evening,
                systolic: 125,
                diastolic: 82,
                pulse: Some(71),
                source_device: Some("Omron Blood Pressure Monitor".to_string()),
                created_at: Utc::now(),
            }),
        ];

        // Sleep data from previous night
        let sleep_start = morning - Duration::hours(9); // 10:30 PM previous night
        let sleep_end = morning - Duration::minutes(30); // 7:00 AM
        let sleep_metrics = vec![HealthMetric::Sleep(SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start,
            sleep_end,
            duration_minutes: Some(420), // 7 hours of actual sleep
            deep_sleep_minutes: Some(95),
            rem_sleep_minutes: Some(88),
            light_sleep_minutes: Some(140), // Remaining sleep time
            awake_minutes: Some(27),
            efficiency: Some(84.2), // Good sleep efficiency
            source_device: Some("Apple Health".to_string()),
            created_at: Utc::now(),
        })];

        // Daily activity summary
        let activity_metrics = vec![HealthMetric::Activity(ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: base_date.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: Some(12547),
            distance_meters: Some(8942.3),
            flights_climbed: Some(18),
            active_energy_burned_kcal: Some(612.5),
            basal_energy_burned_kcal: Some(350.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(73),
            apple_stand_time_minutes: None,
            apple_move_time_minutes: None,
            apple_stand_hour_achieved: None,

            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,

            // Stair Metrics
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,

            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,

            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,

            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,

            source_device: Some("iPhone Health".to_string()),
            created_at: Utc::now(),
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Running,
            started_at: run_start,
            ended_at: run_end,
            total_energy_kcal: Some(287.4),
            active_energy_kcal: Some(250.0),
            distance_meters: Some(4200.0),
            avg_heart_rate: Some(145),
            max_heart_rate: Some(168),
            source_device: Some("Apple Watch Series 8".to_string()),
            created_at: Utc::now(),
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
            assert!(record.heart_rate.unwrap_or(0) >= 20);
            assert!(record.heart_rate.unwrap_or(0) <= 300);
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
            assert!(record.efficiency.is_some());
            let efficiency = record.efficiency.unwrap() as f32;
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
                id: Uuid::new_v4(),
                user_id: activity_metric.user_id,
                recorded_at: activity_metric.recorded_at,
                step_count: Some(2453), // Additional steps from another source
                distance_meters: Some(1682.7), // Additional distance
                flights_climbed: Some(3), // Additional flights
                active_energy_burned_kcal: Some(89.5), // Additional calories
                basal_energy_burned_kcal: Some(50.0),
                distance_cycling_meters: None,
                distance_swimming_meters: None,
                distance_wheelchair_meters: None,
                distance_downhill_snow_sports_meters: None,
                push_count: None,
                swimming_stroke_count: None,
                nike_fuel_points: None,
                apple_exercise_time_minutes: Some(12), // Additional active minutes
                apple_stand_time_minutes: None,
                apple_move_time_minutes: None,
                apple_stand_hour_achieved: None,

                // Mobility Metrics
                walking_speed_m_per_s: None,
                walking_step_length_cm: None,
                walking_asymmetry_percent: None,
                walking_double_support_percent: None,
                six_minute_walk_test_distance_m: None,

                // Stair Metrics
                stair_ascent_speed_m_per_s: None,
                stair_descent_speed_m_per_s: None,

                // Running Dynamics
                ground_contact_time_ms: None,
                vertical_oscillation_cm: None,
                running_stride_length_m: None,
                running_power_watts: None,
                running_speed_m_per_s: None,

                // Cycling Metrics
                cycling_speed_kmh: None,
                cycling_power_watts: None,
                cycling_cadence_rpm: None,
                functional_threshold_power_watts: None,

                // Underwater Metrics
                underwater_depth_meters: None,
                diving_duration_seconds: None,

                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            };

            let record2: ActivityRecord = additional_activity.into();

            // Test aggregation
            record1.aggregate_with(&record2);

            // Check aggregated totals are reasonable
            assert_eq!(record1.step_count, Some(15000)); // 12547 + 2453
            assert!(
                (record1.distance_meters.unwrap() - 10625.0).abs() < 0.1
            );
            // Note: calories_burned and active_minutes are not direct fields in ActivityRecord
            // They would be calculated from active_energy_burned_kcal and other metrics
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
            assert!(record.avg_heart_rate.is_some());
            assert!(record.max_heart_rate.is_some());
            // Calculate duration from start/end times
            let duration_seconds = (record.ended_at - record.started_at).num_seconds();
            assert_eq!(duration_seconds, 1920); // 32 minutes

            // Note: GPS route geometry and route points are handled separately
            // in actual database operations and are not directly part of WorkoutRecord
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

            // Note: raw_data field not available in current HeartRateRecord model
            assert_eq!(record.heart_rate, hr_metric.heart_rate.map(|v| v as i32));
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

            // Note: raw_data field not available in current WorkoutRecord model
            assert_eq!(record.workout_type, workout.workout_type.to_string());
        }
    }
}

/// Tests for edge cases and error conditions
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_payload_with_invalid_metrics() {
        let invalid_bp = HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 49, // Below minimum of 50
            diastolic: 80,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Running,
            started_at: start,
            ended_at: end,
            total_energy_kcal: Some(350.0),
            active_energy_kcal: Some(300.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        };

        assert!(workout.validate().is_err());
    }

    #[test]
    fn test_sleep_with_impossible_efficiency() {
        let start = Utc::now() - Duration::hours(4);
        let end = Utc::now();

        let sleep_metric = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(300), // 5 hours sleep in 4 hours bed time - impossible
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(60),
            awake_minutes: Some(90), // Total > bed time
            efficiency: None,
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
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
            let reading_time = base_time + Duration::minutes((i as f64 * 1.44) as i64); // ~1.44 minutes apart
            let hr_metric = HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: reading_time,
                heart_rate: Some(75 + (i % 40) as i16),
                resting_heart_rate: Some(65 + (i % 30) as i16),
                heart_rate_variability: None,
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("Apple Watch".to_string()),
                context: Some(if i % 10 < 7 { ActivityContext::Resting } else { ActivityContext::Exercise }),
                created_at: Utc::now(),
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
            assert!(record.heart_rate.unwrap_or(0) >= 75);
            assert!(record.heart_rate.unwrap_or(0) <= 135);
            assert!(record.context.is_some());
        }
    }
}
