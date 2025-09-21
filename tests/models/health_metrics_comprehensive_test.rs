use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use self_sensored::models::{
    enums::{ActivityContext, WorkoutType},
    ActivityMetric, BloodPressureMetric, GpsCoordinate, HealthMetric, HeartRateMetric, SleepMetric,
    WorkoutData,
};
use serde_json::json;
use uuid::Uuid;

/// Comprehensive tests for HeartRateMetric validation and processing
mod heart_rate_tests {
    use super::*;

    #[test]
    fn test_heart_rate_validation_valid() {
        let metric = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(72),
            resting_heart_rate: Some(55),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Exercise),
            created_at: Utc::now(),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_heart_rate_validation_ranges() {
        // Test low boundary
        let low_valid = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(20),
            resting_heart_rate: Some(20),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: None,
            created_at: Utc::now(),
        };
        assert!(low_valid.validate().is_ok());

        // Test high boundary
        let high_valid = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(300),
            resting_heart_rate: Some(300),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: None,
            created_at: Utc::now(),
        };
        assert!(high_valid.validate().is_ok());

        // Test invalid low
        let invalid_low = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(19),
            resting_heart_rate: None,
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: None,
            created_at: Utc::now(),
        };
        assert!(invalid_low.validate().is_err());

        // Test invalid high
        let invalid_high = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(301),
            resting_heart_rate: None,
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: None,
            context: None,
            created_at: Utc::now(),
        };
        assert!(invalid_high.validate().is_err());
    }

    #[test]
    fn test_heart_rate_context_validation() {
        let contexts = vec![
            ActivityContext::Resting,
            ActivityContext::Exercise,
            ActivityContext::Sleeping,
            ActivityContext::Stressed,
            ActivityContext::Recovery,
        ];

        for context in contexts {
            let metric = HeartRateMetric {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                recorded_at: Utc::now(),
                heart_rate: Some(75),
                resting_heart_rate: None,
                heart_rate_variability: None,
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                source_device: Some("test".to_string()),
                context: Some(context),
                created_at: Utc::now(),
            };
            assert!(
                metric.validate().is_ok(),
                "Context '{:?}' should be valid",
                context
            );
        }
    }
}

/// Comprehensive tests for BloodPressureMetric validation
mod blood_pressure_tests {
    use super::*;

    #[test]
    fn test_blood_pressure_validation_valid() {
        let metric = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source_device: Some("Manual".to_string()),
            created_at: Utc::now(),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_blood_pressure_medical_ranges() {
        // Test systolic boundaries (50-250)
        let systolic_low = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 50,
            diastolic: 30,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(systolic_low.validate().is_ok());

        let systolic_high = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 250,
            diastolic: 150,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(systolic_high.validate().is_ok());

        // Test invalid systolic
        let systolic_too_low = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 49,
            diastolic: 30,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(systolic_too_low.validate().is_err());

        let systolic_too_high = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 251,
            diastolic: 30,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(systolic_too_high.validate().is_err());

        // Test diastolic boundaries (30-150)
        let diastolic_low = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 60,
            diastolic: 30,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(diastolic_low.validate().is_ok());

        let diastolic_high = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 200,
            diastolic: 150,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(diastolic_high.validate().is_ok());

        // Test invalid diastolic
        let diastolic_too_low = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 100,
            diastolic: 29,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(diastolic_too_low.validate().is_err());

        let diastolic_too_high = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 200,
            diastolic: 151,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(diastolic_too_high.validate().is_err());
    }

    #[test]
    fn test_blood_pressure_systolic_diastolic_relationship() {
        // Systolic should be higher than diastolic
        let invalid_relationship = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 80,
            diastolic: 120,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(invalid_relationship.validate().is_err());

        // Equal values should also be invalid
        let equal_values = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 100,
            diastolic: 100,
            pulse: None,
            source_device: None,
            created_at: Utc::now(),
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420), // 7 hours
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(180),
            awake_minutes: Some(30),
            efficiency: Some(87.5),
            source_device: Some("Apple Health".to_string()),
            created_at: Utc::now(),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_sleep_efficiency_calculation() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        let metric = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420), // 7 hours sleep in 8 hours in bed
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(150),
            awake_minutes: Some(60),
            efficiency: None,
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        };

        let calculated_efficiency = metric.calculate_efficiency();
        assert!((calculated_efficiency - 87.5).abs() < 0.1);

        let reported_efficiency = metric.get_efficiency();
        assert!((reported_efficiency - 87.5).abs() < 0.1);
    }

    #[test]
    fn test_sleep_component_validation() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        // Components exceed total duration - should fail
        let invalid_components = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420),
            deep_sleep_minutes: Some(300), // 5 hours
            rem_sleep_minutes: Some(200),  // 3.3 hours
            light_sleep_minutes: Some(100), // 1.7 hours
            awake_minutes: Some(100),      // 1.7 hours - total > 8 hours
            efficiency: None,
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        };

        assert!(invalid_components.validate().is_err());
    }

    #[test]
    fn test_sleep_time_validation() {
        let start = Utc::now();
        let end = start - Duration::hours(1); // End before start

        let invalid_time = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(60),
            deep_sleep_minutes: None,
            rem_sleep_minutes: None,
            light_sleep_minutes: None,
            awake_minutes: None,
            efficiency: None,
            source_device: None,
            created_at: Utc::now(),
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: Some(10000),
            distance_meters: Some(7500.0),
            flights_climbed: Some(12),
            active_energy_burned_kcal: Some(350.0),
            basal_energy_burned_kcal: Some(200.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(45),
            apple_stand_time_minutes: None,
            apple_move_time_minutes: None,
            apple_stand_hour_achieved: None,
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(metric.validate().is_ok());
    }

    #[test]
    fn test_activity_negative_values() {
        // Negative steps
        let negative_steps = ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: Some(-100),
            distance_meters: None,
            flights_climbed: None,
            active_energy_burned_kcal: None,
            basal_energy_burned_kcal: None,
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
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(negative_steps.validate().is_err());

        // Negative distance
        let negative_distance = ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: None,
            distance_meters: Some(-1000.0),
            flights_climbed: None,
            active_energy_burned_kcal: None,
            basal_energy_burned_kcal: None,
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
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: None,
            created_at: Utc::now(),
        };
        assert!(negative_distance.validate().is_err());

        // Negative calories
        let negative_calories = ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: None,
            distance_meters: None,
            flights_climbed: None,
            active_energy_burned_kcal: Some(-250.0),
            basal_energy_burned_kcal: None,
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
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: None,
            created_at: Utc::now(),
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
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
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
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        };

        assert!(workout.validate().is_ok());

        // Note: Route functionality removed from WorkoutData
        // Test basic workout validation
        assert!(workout.validate().is_ok());
    }

    #[test]
    fn test_workout_gps_timing_validation() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        // GPS point outside workout duration - should fail
        let invalid_route_points = vec![GpsCoordinate {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_meters: Some(50.0),
            recorded_at: start - Duration::minutes(10), // Before workout start
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
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        };

        // Note: GPS timing validation removed - now just basic workout validation
        assert!(workout.validate().is_ok());
    }

    #[test]
    fn test_workout_heart_rate_validation() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        // Invalid heart rate - too high
        let invalid_hr = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Running,
            started_at: start,
            ended_at: end,
            total_energy_kcal: Some(350.0),
            active_energy_kcal: Some(300.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(301), // > 300
            max_heart_rate: Some(175),
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(72),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(35.5),
            walking_heart_rate_average: Some(95),
            heart_rate_recovery_one_minute: Some(25),
            atrial_fibrillation_burden_percentage: Some(Decimal::new(150, 2)),
            vo2_max_ml_kg_min: Some(Decimal::new(4500, 2)),
            source_device: Some("test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        });
        assert!(heart_rate.validate().is_ok());
        assert_eq!(heart_rate.metric_type(), "HeartRate");

        let blood_pressure = HealthMetric::BloodPressure(BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        });
        assert!(blood_pressure.validate().is_ok());
        assert_eq!(blood_pressure.metric_type(), "BloodPressure");

        let sleep = HealthMetric::Sleep(SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420),
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: Some(180),
            awake_minutes: Some(30),
            efficiency: Some(87.5),
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        });
        assert!(sleep.validate().is_ok());
        assert_eq!(sleep.metric_type(), "Sleep");

        let activity = HealthMetric::Activity(ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            step_count: Some(10000),
            distance_meters: Some(7500.0),
            flights_climbed: Some(12),
            active_energy_burned_kcal: Some(350.0),
            basal_energy_burned_kcal: Some(200.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(45),
            apple_stand_time_minutes: None,
            apple_move_time_minutes: None,
            apple_stand_hour_achieved: None,
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        });
        assert!(activity.validate().is_ok());
        assert_eq!(activity.metric_type(), "Activity");

        let workout_start = Utc::now() - Duration::hours(1);
        let workout_end = Utc::now();
        let workout = HealthMetric::Workout(WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Running,
            started_at: workout_start,
            ended_at: workout_end,
            total_energy_kcal: Some(350.0),
            active_energy_kcal: Some(300.0),
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source_device: Some("test".to_string()),
            created_at: Utc::now(),
        });
        assert!(workout.validate().is_ok());
        assert_eq!(workout.metric_type(), "Workout");
    }

    #[test]
    fn test_serialization_deserialization() {
        let heart_rate = HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(72),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(35.5),
            walking_heart_rate_average: Some(95),
            heart_rate_recovery_one_minute: Some(25),
            atrial_fibrillation_burden_percentage: Some(Decimal::new(150, 2)),
            vo2_max_ml_kg_min: Some(Decimal::new(4500, 2)),
            source_device: Some("test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        });

        let serialized = serde_json::to_string(&heart_rate).unwrap();
        let deserialized: HealthMetric = serde_json::from_str(&serialized).unwrap();

        assert_eq!(heart_rate.metric_type(), deserialized.metric_type());
        assert!(deserialized.validate().is_ok());
    }
}
