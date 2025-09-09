use chrono::{DateTime, Utc, NaiveDate};
use serde_json::json;
use self_sensored::models::{
    HeartRateMetric, BloodPressureMetric, SleepMetric, ActivityMetric, 
    WorkoutData, GpsCoordinate, HealthMetric
};

/// Comprehensive tests for HeartRateMetric validation and processing
mod heart_rate_tests {
    use super::*;

    #[test]
    fn test_heart_rate_validation_valid() {
        let metric = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(55),
            avg_bpm: Some(72),
            max_bpm: Some(95),
            source: Some("Apple Watch".to_string()),
            context: Some("exercise".to_string()),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_heart_rate_validation_ranges() {
        // Test low boundary
        let low_valid = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(20),
            avg_bpm: Some(20),
            max_bpm: Some(20),
            source: None,
            context: None,
        };
        assert!(low_valid.validate().is_ok());

        // Test high boundary
        let high_valid = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(300),
            avg_bpm: Some(300),
            max_bpm: Some(300),
            source: None,
            context: None,
        };
        assert!(high_valid.validate().is_ok());

        // Test invalid low
        let invalid_low = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(19),
            avg_bpm: None,
            max_bpm: None,
            source: None,
            context: None,
        };
        assert!(invalid_low.validate().is_err());

        // Test invalid high
        let invalid_high = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: None,
            avg_bpm: Some(301),
            max_bpm: None,
            source: None,
            context: None,
        };
        assert!(invalid_high.validate().is_err());
    }

    #[test]
    fn test_heart_rate_context_validation() {
        let contexts = vec![
            "rest", "exercise", "sleep", "stress", "recovery"
        ];

        for context in contexts {
            let metric = HeartRateMetric {
                recorded_at: Utc::now(),
                min_bpm: None,
                avg_bpm: Some(75),
                max_bpm: None,
                source: Some("test".to_string()),
                context: Some(context.to_string()),
            };
            assert!(metric.validate().is_ok(), "Context '{}' should be valid", context);
        }
    }
}

/// Comprehensive tests for BloodPressureMetric validation
mod blood_pressure_tests {
    use super::*;

    #[test]
    fn test_blood_pressure_validation_valid() {
        let metric = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source: Some("Manual".to_string()),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_blood_pressure_medical_ranges() {
        // Test systolic boundaries (50-250)
        let systolic_low = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 50,
            diastolic: 30,
            pulse: None,
            source: None,
        };
        assert!(systolic_low.validate().is_ok());

        let systolic_high = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 250,
            diastolic: 150,
            pulse: None,
            source: None,
        };
        assert!(systolic_high.validate().is_ok());

        // Test invalid systolic
        let systolic_too_low = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 49,
            diastolic: 30,
            pulse: None,
            source: None,
        };
        assert!(systolic_too_low.validate().is_err());

        let systolic_too_high = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 251,
            diastolic: 30,
            pulse: None,
            source: None,
        };
        assert!(systolic_too_high.validate().is_err());

        // Test diastolic boundaries (30-150)
        let diastolic_low = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 60,
            diastolic: 30,
            pulse: None,
            source: None,
        };
        assert!(diastolic_low.validate().is_ok());

        let diastolic_high = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 200,
            diastolic: 150,
            pulse: None,
            source: None,
        };
        assert!(diastolic_high.validate().is_ok());

        // Test invalid diastolic
        let diastolic_too_low = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 100,
            diastolic: 29,
            pulse: None,
            source: None,
        };
        assert!(diastolic_too_low.validate().is_err());

        let diastolic_too_high = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 200,
            diastolic: 151,
            pulse: None,
            source: None,
        };
        assert!(diastolic_too_high.validate().is_err());
    }

    #[test]
    fn test_blood_pressure_systolic_diastolic_relationship() {
        // Systolic should be higher than diastolic
        let invalid_relationship = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 80,
            diastolic: 120,
            pulse: None,
            source: None,
        };
        assert!(invalid_relationship.validate().is_err());

        // Equal values should also be invalid
        let equal_values = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 100,
            diastolic: 100,
            pulse: None,
            source: None,
        };
        assert!(equal_values.validate().is_err());
    }
}

/// Comprehensive tests for SleepMetric validation and efficiency calculations
mod sleep_tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_sleep_metric_validation_valid() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();
        
        let metric = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420, // 7 hours
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: Some(87.5),
            source: Some("Apple Health".to_string()),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_sleep_efficiency_calculation() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();
        
        let metric = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420, // 7 hours sleep in 8 hours in bed
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(60),
            efficiency_percentage: None,
            source: Some("test".to_string()),
        };

        let calculated_efficiency = metric.calculate_efficiency();
        assert!((calculated_efficiency - 87.5).abs() < 0.1);

        let reported_efficiency = metric.get_efficiency_percentage();
        assert!((reported_efficiency - 87.5).abs() < 0.1);
    }

    #[test]
    fn test_sleep_component_validation() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();
        
        // Components exceed total duration - should fail
        let invalid_components = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420,
            deep_sleep_minutes: Some(300), // 5 hours
            rem_sleep_minutes: Some(200), // 3.3 hours  
            awake_minutes: Some(100), // 1.7 hours - total > 8 hours
            efficiency_percentage: None,
            source: Some("test".to_string()),
        };

        assert!(invalid_components.validate().is_err());
    }

    #[test]
    fn test_sleep_time_validation() {
        let start = Utc::now();
        let end = start - Duration::hours(1); // End before start

        let invalid_time = SleepMetric {
            recorded_at: start,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 60,
            deep_sleep_minutes: None,
            rem_sleep_minutes: None,
            awake_minutes: None,
            efficiency_percentage: None,
            source: None,
        };

        assert!(invalid_time.validate().is_err());
    }
}

/// Comprehensive tests for ActivityMetric validation
mod activity_tests {
    use super::*;

    #[test]
    fn test_activity_metric_validation_valid() {
        let metric = ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: Some(10000),
            distance_meters: Some(7500.0),
            calories_burned: Some(350.0),
            active_minutes: Some(45),
            flights_climbed: Some(12),
            source: Some("iPhone".to_string()),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_activity_negative_values() {
        // Negative steps
        let negative_steps = ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: Some(-100),
            distance_meters: None,
            calories_burned: None,
            active_minutes: None,
            flights_climbed: None,
            source: None,
        };
        assert!(negative_steps.validate().is_err());

        // Negative distance
        let negative_distance = ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: None,
            distance_meters: Some(-1000.0),
            calories_burned: None,
            active_minutes: None,
            flights_climbed: None,
            source: None,
        };
        assert!(negative_distance.validate().is_err());

        // Negative calories
        let negative_calories = ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: None,
            distance_meters: None,
            calories_burned: Some(-250.0),
            active_minutes: None,
            flights_climbed: None,
            source: None,
        };
        assert!(negative_calories.validate().is_err());
    }
}

/// Comprehensive tests for WorkoutData and GPS validation
mod workout_tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_workout_validation_valid() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        
        let workout = WorkoutData {
            workout_type: "running".to_string(),
            start_time: start,
            end_time: end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source: Some("Apple Watch".to_string()),
            route_points: None,
        };

        assert!(workout.validate().is_ok());
    }

    #[test]
    fn test_gps_coordinate_validation() {
        // Valid coordinates
        let valid_coord = GpsCoordinate {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_meters: Some(50.0),
            recorded_at: Utc::now(),
        };
        assert!(valid_coord.validate().is_ok());

        // Invalid latitude
        let invalid_lat = GpsCoordinate {
            latitude: 91.0, // > 90
            longitude: -122.4194,
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(invalid_lat.validate().is_err());

        // Invalid longitude
        let invalid_lon = GpsCoordinate {
            latitude: 37.7749,
            longitude: -181.0, // < -180
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(invalid_lon.validate().is_err());
    }

    #[test]
    fn test_workout_with_gps_route() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        
        let route_points = vec![
            GpsCoordinate {
                latitude: 37.7749,
                longitude: -122.4194,
                altitude_meters: Some(50.0),
                recorded_at: start + Duration::minutes(10),
            },
            GpsCoordinate {
                latitude: 37.7849,
                longitude: -122.4094,
                altitude_meters: Some(55.0),
                recorded_at: start + Duration::minutes(30),
            },
            GpsCoordinate {
                latitude: 37.7949,
                longitude: -122.3994,
                altitude_meters: Some(60.0),
                recorded_at: end - Duration::minutes(10),
            },
        ];

        let workout = WorkoutData {
            workout_type: "running".to_string(),
            start_time: start,
            end_time: end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source: Some("Apple Watch".to_string()),
            route_points: Some(route_points),
        };

        assert!(workout.validate().is_ok());
        
        // Test LINESTRING generation
        let linestring = workout.route_to_linestring();
        assert!(linestring.is_some());
        assert!(linestring.unwrap().starts_with("LINESTRING("));
    }

    #[test]
    fn test_workout_gps_timing_validation() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        
        // GPS point outside workout duration - should fail
        let invalid_route_points = vec![
            GpsCoordinate {
                latitude: 37.7749,
                longitude: -122.4194,
                altitude_meters: Some(50.0),
                recorded_at: start - Duration::minutes(10), // Before workout start
            },
        ];

        let workout = WorkoutData {
            workout_type: "running".to_string(),
            start_time: start,
            end_time: end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source: Some("Apple Watch".to_string()),
            route_points: Some(invalid_route_points),
        };

        assert!(workout.validate().is_err());
    }

    #[test]
    fn test_workout_heart_rate_validation() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();
        
        // Invalid heart rate - too high
        let invalid_hr = WorkoutData {
            workout_type: "running".to_string(),
            start_time: start,
            end_time: end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(301), // > 300
            max_heart_rate: Some(175),
            source: Some("test".to_string()),
            route_points: None,
        };

        assert!(invalid_hr.validate().is_err());
    }
}

/// Integration tests for HealthMetric enum
mod health_metric_integration_tests {
    use super::*;

    #[test]
    fn test_health_metric_validation_dispatch() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        // Test all metric types through HealthMetric enum
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(55),
            avg_bpm: Some(72),
            max_bpm: Some(95),
            source: Some("test".to_string()),
            context: Some("rest".to_string()),
        });
        assert!(heart_rate.validate().is_ok());
        assert_eq!(heart_rate.metric_type(), "HeartRate");

        let blood_pressure = HealthMetric::BloodPressure(BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source: Some("test".to_string()),
        });
        assert!(blood_pressure.validate().is_ok());
        assert_eq!(blood_pressure.metric_type(), "BloodPressure");

        let sleep = HealthMetric::Sleep(SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420,
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: Some(87.5),
            source: Some("test".to_string()),
        });
        assert!(sleep.validate().is_ok());
        assert_eq!(sleep.metric_type(), "Sleep");

        let activity = HealthMetric::Activity(ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: Some(10000),
            distance_meters: Some(7500.0),
            calories_burned: Some(350.0),
            active_minutes: Some(45),
            flights_climbed: Some(12),
            source: Some("test".to_string()),
        });
        assert!(activity.validate().is_ok());
        assert_eq!(activity.metric_type(), "Activity");

        let workout_start = Utc::now() - Duration::hours(1);
        let workout_end = Utc::now();
        let workout = HealthMetric::Workout(WorkoutData {
            workout_type: "running".to_string(),
            start_time: workout_start,
            end_time: workout_end,
            total_energy_kcal: Some(350.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source: Some("test".to_string()),
            route_points: None,
        });
        assert!(workout.validate().is_ok());
        assert_eq!(workout.metric_type(), "Workout");
    }

    #[test]
    fn test_serialization_deserialization() {
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(55),
            avg_bpm: Some(72),
            max_bpm: Some(95),
            source: Some("test".to_string()),
            context: Some("rest".to_string()),
        });

        let serialized = serde_json::to_string(&heart_rate).unwrap();
        let deserialized: HealthMetric = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(heart_rate.metric_type(), deserialized.metric_type());
        assert!(deserialized.validate().is_ok());
    }
}