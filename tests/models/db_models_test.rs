use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::health_metrics::{
    ActivityMetric, BloodPressureMetric, GpsCoordinate, HeartRateMetric, SleepMetric, WorkoutData,
};

use self_sensored::models::db::{
    ActivityRecord, BloodPressureRecord, HeartRateRecord, SleepRecord, WorkoutRecord,
};

/// Tests for HeartRateRecord conversion and functionality
mod heart_rate_record_tests {
    use super::*;

    #[test]
    fn test_heart_rate_record_conversion() {
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
            context: Some(self_sensored::models::ActivityContext::Exercise),
            created_at: Utc::now(),
        };

        let record: HeartRateRecord = metric.clone().into();

        assert_eq!(record.heart_rate, Some(72)); // Uses heart_rate
        assert_eq!(record.resting_heart_rate, Some(55)); // Uses resting_heart_rate
        assert_eq!(record.context, metric.context.map(|c| c.to_string()));
        assert_eq!(record.source_device, metric.source_device);
    }

    #[test]
    fn test_heart_rate_record_with_raw_json() {
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
            context: Some(self_sensored::models::ActivityContext::Exercise),
            created_at: Utc::now(),
        };

        let raw_json = json!({
            "original_payload": {
                "heart_rate": 72,
                "confidence": 0.95
            }
        });

        let record = HeartRateRecord::from_metric_with_raw(metric.clone(), raw_json.clone());

        assert_eq!(record.heart_rate, Some(72));
        assert_eq!(record.resting_heart_rate, Some(55));
        assert_eq!(record.context, metric.context.map(|c| c.to_string()));
        assert_eq!(record.source_device, metric.source_device);
    }

    #[test]
    fn test_heart_rate_record_edge_cases() {
        // Test with only heart_rate
        let metric_only_max = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(180),
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

        let record: HeartRateRecord = metric_only_max.into();
        assert_eq!(record.heart_rate, Some(180)); // Should use max_bpm
        assert_eq!(record.resting_heart_rate, None);

        // Test with no heart rate values
        let metric_no_bpm = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: None,
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

        let record: HeartRateRecord = metric_no_bpm.into();
        assert_eq!(record.heart_rate, None); // No default value - should be None
        assert_eq!(record.resting_heart_rate, None);
    }
}

/// Tests for BloodPressureRecord conversion
mod blood_pressure_record_tests {
    use super::*;

    #[test]
    fn test_blood_pressure_record_conversion() {
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

        let record: BloodPressureRecord = metric.clone().into();

        assert_eq!(record.systolic, 120);
        assert_eq!(record.diastolic, 80);
        assert_eq!(record.pulse, Some(72));
        assert_eq!(record.source_device, metric.source_device);
    }
}

/// Tests for SleepRecord conversion and efficiency calculations
mod sleep_record_tests {
    use super::*;

    #[test]
    fn test_sleep_record_conversion_with_efficiency() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        let metric = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420), // 7 hours sleep in 8 hours in bed = 87.5% efficiency
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: None,
            awake_minutes: Some(30),
            efficiency: Some(87.5), // Calculated efficiency
            source_device: Some("Apple Health".to_string()),
            created_at: end,
        };

        let record: SleepRecord = metric.clone().into();

        assert_eq!(record.sleep_start, start);
        assert_eq!(record.sleep_end, end);
        assert_eq!(record.duration_minutes, Some(420));
        assert_eq!(record.deep_sleep_minutes, Some(120));
        assert_eq!(record.rem_sleep_minutes, Some(90));
        assert_eq!(record.awake_minutes, Some(30));
        assert_eq!(record.source_device, metric.source_device);

        // Check that efficiency was calculated
        assert!(record.efficiency.is_some());
        let efficiency = record.efficiency.unwrap() as f32;
        assert!((efficiency - 87.5).abs() < 0.1);
    }

    #[test]
    fn test_sleep_record_with_provided_efficiency() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        let metric = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420),
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: None,
            awake_minutes: Some(30),
            efficiency: Some(85.0), // Provided efficiency
            source_device: Some("Apple Health".to_string()),
            created_at: end,
        };

        let record: SleepRecord = metric.clone().into();

        // Should use provided efficiency
        let efficiency = record.efficiency.unwrap() as f32;
        assert!((efficiency - 85.0).abs() < 0.1);
    }

    #[test]
    fn test_sleep_record_with_raw_json() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();

        let metric = SleepMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            sleep_start: start,
            sleep_end: end,
            duration_minutes: Some(420),
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            light_sleep_minutes: None,
            awake_minutes: Some(30),
            efficiency: None,
            source_device: Some("Apple Health".to_string()),
            created_at: end,
        };

        let raw_json = json!({
            "original_sleep_data": {
                "sleep_stages": ["deep", "light", "rem"],
                "confidence": 0.92
            }
        });

        let record = SleepRecord::from_metric_with_raw(metric.clone(), raw_json.clone());

        assert!(record.efficiency.is_some());
    }
}

/// Tests for ActivityRecord conversion and aggregation
mod activity_record_tests {
    use super::*;

    #[test]
    fn test_activity_record_conversion() {
        let metric = ActivityMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            step_count: Some(10000),
            distance_meters: Some(7500.0),
            active_energy_burned_kcal: Some(350.0),
            basal_energy_burned_kcal: None,
            flights_climbed: Some(12),
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
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let record: ActivityRecord = metric.clone().into();

        assert_eq!(record.recorded_at, metric.recorded_at);
        assert_eq!(record.step_count, metric.step_count);
        assert_eq!(record.distance_meters, metric.distance_meters);
        assert_eq!(record.active_energy_burned_kcal, metric.active_energy_burned_kcal);
        assert_eq!(record.flights_climbed, metric.flights_climbed);
        assert_eq!(record.source_device, metric.source_device);
    }

    #[test]
    fn test_activity_record_aggregation() {
        let now = Utc::now();

        let mut record1 = ActivityRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: now,
            step_count: Some(5000),
            distance_meters: Some(3000.0),
            active_energy_burned_kcal: Some(150.0),
            basal_energy_burned_kcal: Some(100.0),
            flights_climbed: Some(5),
            source_device: Some("iPhone".to_string()),
            created_at: now,
        };

        let record2 = ActivityRecord {
            id: Uuid::new_v4(),
            user_id: record1.user_id,
            recorded_at: now,
            step_count: Some(3000),
            distance_meters: Some(2000.0),
            active_energy_burned_kcal: Some(100.0),
            basal_energy_burned_kcal: Some(80.0),
            flights_climbed: Some(3),
            source_device: Some("Apple Watch".to_string()),
            created_at: now,
        };

        // Aggregate the records
        record1.aggregate_with(&record2);

        // Check aggregated values
        assert_eq!(record1.step_count, Some(8000));
        assert_eq!(record1.distance_meters, Some(5000.0));
        assert_eq!(record1.active_energy_burned_kcal, Some(250.0));
        assert_eq!(record1.basal_energy_burned_kcal, Some(180.0));
        assert_eq!(record1.flights_climbed, Some(8));
    }

    #[test]
    fn test_activity_record_aggregation_with_nulls() {
        let now = Utc::now();

        let mut record1 = ActivityRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: now,
            step_count: Some(5000),
            distance_meters: None, // Null value
            active_energy_burned_kcal: Some(150.0),
            basal_energy_burned_kcal: None, // Null value
            flights_climbed: Some(5),
            source_device: Some("iPhone".to_string()),
            created_at: now,
        };

        let record2 = ActivityRecord {
            id: Uuid::new_v4(),
            user_id: record1.user_id,
            recorded_at: now,
            step_count: None, // Null value
            distance_meters: Some(2000.0),
            active_energy_burned_kcal: Some(100.0),
            basal_energy_burned_kcal: Some(80.0),
            flights_climbed: None, // Null value
            source_device: Some("Apple Watch".to_string()),
            created_at: now,
        };

        record1.aggregate_with(&record2);

        // Check that non-null values are preserved and combined appropriately
        assert_eq!(record1.step_count, Some(5000)); // Only record1 had steps
        assert_eq!(record1.distance_meters, Some(2000.0)); // Only record2 had distance
        assert_eq!(record1.active_energy_burned_kcal, Some(250.0)); // Both had calories, so summed
        assert_eq!(record1.basal_energy_burned_kcal, Some(80.0)); // Only record2 had basal energy
        assert_eq!(record1.flights_climbed, Some(5)); // Only record1 had flights
    }
}

/// Tests for WorkoutRecord conversion and GPS handling
mod workout_record_tests {
    use super::*;

    #[test]
    fn test_workout_record_conversion() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        let workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: self_sensored::models::WorkoutType::Running,
            started_at: start,
            ended_at: end,
            total_energy_kcal: Some(350.0),
            active_energy_kcal: None,
            distance_meters: Some(5000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(175),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        };

        let record: WorkoutRecord = workout.clone().into();

        assert_eq!(record.workout_type, workout.workout_type.to_string());
        assert_eq!(record.started_at, start);
        assert_eq!(record.ended_at, end);
        assert_eq!(record.distance_meters, Some(5000.0));
        assert_eq!(record.avg_heart_rate, Some(150));
        assert_eq!(record.max_heart_rate, Some(175));
        assert_eq!(record.total_energy_kcal, Some(350.0));
        assert_eq!(record.source_device, workout.source_device);
    }

    #[test]
    fn test_workout_record_basic_conversion() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        let workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: self_sensored::models::WorkoutType::Running,
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

        let record: WorkoutRecord = workout.clone().into();

        assert_eq!(record.workout_type, "running");
        assert_eq!(record.started_at, start);
        assert_eq!(record.ended_at, end);
        assert_eq!(record.total_energy_kcal, Some(350.0));
        assert_eq!(record.active_energy_kcal, Some(300.0));
        assert_eq!(record.distance_meters, Some(5000.0));
        assert_eq!(record.avg_heart_rate, Some(150));
        assert_eq!(record.max_heart_rate, Some(175));
        assert_eq!(record.source_device, Some("Apple Watch".to_string()));
    }


    #[test]
    fn test_workout_record_with_raw_json() {
        let start = Utc::now() - Duration::hours(1);
        let end = Utc::now();

        let workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: self_sensored::models::WorkoutType::Running,
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

        let raw_json = json!({
            "workout_metadata": {
                "weather": "sunny",
                "temperature": 22.5,
                "indoor": false
            }
        });

        let record = WorkoutRecord::from_workout_with_raw(workout.clone(), raw_json.clone());

        assert_eq!(record.workout_type, "running");
        assert_eq!(record.started_at, start);
        assert_eq!(record.ended_at, end);
    }
}

/// Tests for GPS coordinate functionality
mod gps_coordinate_tests {
    use super::*;

    #[test]
    fn test_gps_coordinate_to_postgis_point() {
        let coord = GpsCoordinate {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_meters: Some(50.0),
            recorded_at: Utc::now(),
        };

        let point_str = coord.to_postgis_point();
        assert_eq!(point_str, "POINT(-122.4194 37.7749)");
    }

    #[test]
    fn test_gps_coordinate_validation_edge_cases() {
        // Test exact boundaries
        let north_pole = GpsCoordinate {
            latitude: 90.0,
            longitude: 0.0,
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(north_pole.validate().is_ok());

        let south_pole = GpsCoordinate {
            latitude: -90.0,
            longitude: 0.0,
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(south_pole.validate().is_ok());

        let date_line_west = GpsCoordinate {
            latitude: 0.0,
            longitude: -180.0,
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(date_line_west.validate().is_ok());

        let date_line_east = GpsCoordinate {
            latitude: 0.0,
            longitude: 180.0,
            altitude_meters: None,
            recorded_at: Utc::now(),
        };
        assert!(date_line_east.validate().is_ok());
    }
}
