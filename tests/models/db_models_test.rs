use chrono::{DateTime, Utc, NaiveDate, Duration};
use serde_json::json;
use uuid::Uuid;

use self_sensored::models::{
    HeartRateMetric, BloodPressureMetric, SleepMetric, ActivityMetric, 
    WorkoutData, GpsCoordinate
};

use self_sensored::models::db::{
    HeartRateRecord, BloodPressureRecord, SleepRecord, ActivityRecord,
    WorkoutRecord, WorkoutRoutePoint
};

/// Tests for HeartRateRecord conversion and functionality
mod heart_rate_record_tests {
    use super::*;

    #[test]
    fn test_heart_rate_record_conversion() {
        let metric = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(55),
            avg_bpm: Some(72),
            max_bpm: Some(95),
            source: Some("Apple Watch".to_string()),
            context: Some("exercise".to_string()),
        };

        let record: HeartRateRecord = metric.clone().into();
        
        assert_eq!(record.heart_rate, 72); // Uses avg_bpm
        assert_eq!(record.resting_heart_rate, Some(55)); // Uses min_bpm
        assert_eq!(record.context, metric.context);
        assert_eq!(record.source_device, metric.source);
        assert!(record.raw_data.is_none()); // No raw data in basic conversion
    }

    #[test]
    fn test_heart_rate_record_with_raw_json() {
        let metric = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(55),
            avg_bpm: Some(72),
            max_bpm: Some(95),
            source: Some("Apple Watch".to_string()),
            context: Some("exercise".to_string()),
        };

        let raw_json = json!({
            "original_payload": {
                "heart_rate": 72,
                "confidence": 0.95
            }
        });

        let record = HeartRateRecord::from_metric_with_raw(metric.clone(), raw_json.clone());
        
        assert_eq!(record.heart_rate, 72);
        assert_eq!(record.resting_heart_rate, Some(55));
        assert_eq!(record.context, metric.context);
        assert_eq!(record.source_device, metric.source);
        assert_eq!(record.raw_data, Some(raw_json));
    }

    #[test]
    fn test_heart_rate_record_edge_cases() {
        // Test with only max_bpm
        let metric_only_max = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: None,
            avg_bpm: None,
            max_bpm: Some(180),
            source: None,
            context: None,
        };

        let record: HeartRateRecord = metric_only_max.into();
        assert_eq!(record.heart_rate, 180); // Should use max_bpm
        assert_eq!(record.resting_heart_rate, None);

        // Test with no BPM values
        let metric_no_bpm = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: None,
            avg_bpm: None,
            max_bpm: None,
            source: None,
            context: None,
        };

        let record: HeartRateRecord = metric_no_bpm.into();
        assert_eq!(record.heart_rate, 70); // Default value
        assert_eq!(record.resting_heart_rate, None);
    }
}

/// Tests for BloodPressureRecord conversion
mod blood_pressure_record_tests {
    use super::*;

    #[test]
    fn test_blood_pressure_record_conversion() {
        let metric = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(72),
            source: Some("Manual".to_string()),
        };

        let record: BloodPressureRecord = metric.clone().into();
        
        assert_eq!(record.systolic, 120);
        assert_eq!(record.diastolic, 80);
        assert_eq!(record.pulse, Some(72));
        assert_eq!(record.source_device, metric.source);
        assert!(record.raw_data.is_none());
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
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420, // 7 hours sleep in 8 hours in bed = 87.5% efficiency
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: None, // Should be calculated
            source: Some("Apple Health".to_string()),
        };

        let record: SleepRecord = metric.clone().into();
        
        assert_eq!(record.sleep_start, start);
        assert_eq!(record.sleep_end, end);
        assert_eq!(record.duration_minutes, 420);
        assert_eq!(record.deep_sleep_minutes, Some(120));
        assert_eq!(record.rem_sleep_minutes, Some(90));
        assert_eq!(record.awake_minutes, Some(30));
        assert_eq!(record.source_device, metric.source);
        
        // Check that efficiency was calculated
        assert!(record.sleep_efficiency.is_some());
        let efficiency = record.sleep_efficiency.unwrap().to_string().parse::<f32>().unwrap();
        assert!((efficiency - 87.5).abs() < 0.1);
    }

    #[test]
    fn test_sleep_record_with_provided_efficiency() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();
        
        let metric = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420,
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: Some(85.0), // Provided efficiency
            source: Some("Apple Health".to_string()),
        };

        let record: SleepRecord = metric.clone().into();
        
        // Should use provided efficiency
        let efficiency = record.sleep_efficiency.unwrap().to_string().parse::<f32>().unwrap();
        assert!((efficiency - 85.0).abs() < 0.1);
    }

    #[test]
    fn test_sleep_record_with_raw_json() {
        let start = Utc::now() - Duration::hours(8);
        let end = Utc::now();
        
        let metric = SleepMetric {
            recorded_at: end,
            sleep_start: start,
            sleep_end: end,
            total_sleep_minutes: 420,
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(90),
            awake_minutes: Some(30),
            efficiency_percentage: None,
            source: Some("Apple Health".to_string()),
        };

        let raw_json = json!({
            "original_sleep_data": {
                "sleep_stages": ["deep", "light", "rem"],
                "confidence": 0.92
            }
        });

        let record = SleepRecord::from_metric_with_raw(metric.clone(), raw_json.clone());
        
        assert_eq!(record.raw_data, Some(raw_json));
        assert!(record.sleep_efficiency.is_some());
    }
}

/// Tests for ActivityRecord conversion and aggregation
mod activity_record_tests {
    use super::*;

    #[test]
    fn test_activity_record_conversion() {
        let metric = ActivityMetric {
            date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            steps: Some(10000),
            distance_meters: Some(7500.0),
            calories_burned: Some(350.0),
            active_minutes: Some(45),
            flights_climbed: Some(12),
            source: Some("iPhone".to_string()),
        };

        let record: ActivityRecord = metric.clone().into();
        
        assert_eq!(record.recorded_date, metric.date);
        assert_eq!(record.steps, metric.steps);
        assert_eq!(record.distance_meters.unwrap().to_string().parse::<f64>().unwrap(), 7500.0);
        assert_eq!(record.calories_burned, Some(350));
        assert_eq!(record.active_minutes, metric.active_minutes);
        assert_eq!(record.flights_climbed, metric.flights_climbed);
        assert_eq!(record.source_device, metric.source);
        assert!(record.raw_data.is_none());
        assert_eq!(record.created_at, record.updated_at);
    }

    #[test]
    fn test_activity_record_aggregation() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        
        let mut record1 = ActivityRecord {
            user_id: Uuid::new_v4(),
            recorded_date: date,
            steps: Some(5000),
            distance_meters: Some(sqlx::types::BigDecimal::from(3000.0)),
            calories_burned: Some(150),
            active_minutes: Some(20),
            flights_climbed: Some(5),
            source_device: Some("iPhone".to_string()),
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let record2 = ActivityRecord {
            user_id: record1.user_id,
            recorded_date: date,
            steps: Some(3000),
            distance_meters: Some(sqlx::types::BigDecimal::from(2000.0)),
            calories_burned: Some(100),
            active_minutes: Some(15),
            flights_climbed: Some(3),
            source_device: Some("Apple Watch".to_string()),
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let original_updated_at = record1.updated_at;
        
        // Aggregate the records
        record1.aggregate_with(&record2);
        
        // Check aggregated values
        assert_eq!(record1.steps, Some(8000));
        assert_eq!(record1.distance_meters.unwrap().to_string().parse::<f64>().unwrap(), 5000.0);
        assert_eq!(record1.calories_burned, Some(250));
        assert_eq!(record1.active_minutes, Some(35));
        assert_eq!(record1.flights_climbed, Some(8));
        
        // Check that updated_at was modified
        assert!(record1.updated_at > original_updated_at);
    }

    #[test]
    fn test_activity_record_aggregation_with_nulls() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        
        let mut record1 = ActivityRecord {
            user_id: Uuid::new_v4(),
            recorded_date: date,
            steps: Some(5000),
            distance_meters: None, // Null value
            calories_burned: Some(150),
            active_minutes: None, // Null value
            flights_climbed: Some(5),
            source_device: Some("iPhone".to_string()),
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let record2 = ActivityRecord {
            user_id: record1.user_id,
            recorded_date: date,
            steps: None, // Null value
            distance_meters: Some(sqlx::types::BigDecimal::from(2000.0)),
            calories_burned: Some(100),
            active_minutes: Some(15),
            flights_climbed: None, // Null value
            source_device: Some("Apple Watch".to_string()),
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        record1.aggregate_with(&record2);
        
        // Check that non-null values are preserved and combined appropriately
        assert_eq!(record1.steps, Some(5000)); // Only record1 had steps
        assert_eq!(record1.distance_meters.unwrap().to_string().parse::<f64>().unwrap(), 2000.0); // Only record2 had distance
        assert_eq!(record1.calories_burned, Some(250)); // Both had calories, so summed
        assert_eq!(record1.active_minutes, Some(15)); // Only record2 had active_minutes
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

        let record: WorkoutRecord = workout.clone().into();
        
        assert_eq!(record.workout_type, workout.workout_type);
        assert_eq!(record.started_at, start);
        assert_eq!(record.ended_at, end);
        assert_eq!(record.distance_meters.unwrap().to_string().parse::<f64>().unwrap(), 5000.0);
        assert_eq!(record.average_heart_rate, Some(150));
        assert_eq!(record.max_heart_rate, Some(175));
        assert_eq!(record.total_energy_kcal.unwrap().to_string().parse::<f64>().unwrap(), 350.0);
        assert_eq!(record.source_device, workout.source);
        assert_eq!(record.duration_seconds, Some(3600)); // 1 hour
        assert!(record.route_geometry.is_none()); // No GPS route
        assert!(record.raw_data.is_none());
    }

    #[test]
    fn test_workout_record_with_gps_route() {
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

        let record: WorkoutRecord = workout.clone().into();
        
        // Should have PostGIS LINESTRING
        assert!(record.route_geometry.is_some());
        let linestring = record.route_geometry.unwrap();
        assert!(linestring.starts_with("LINESTRING("));
        assert!(linestring.contains("-122.4194"));
        assert!(linestring.contains("37.7749"));
    }

    #[test]
    fn test_workout_record_route_points_conversion() {
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
            route_points: Some(route_points.clone()),
        };

        let record: WorkoutRecord = workout.clone().into();
        let route_point_records = record.route_points(&workout);
        
        assert_eq!(route_point_records.len(), 2);
        
        // Check first point
        assert_eq!(route_point_records[0].workout_id, record.id);
        assert_eq!(route_point_records[0].point_order, 0);
        assert_eq!(route_point_records[0].latitude, 37.7749);
        assert_eq!(route_point_records[0].longitude, -122.4194);
        assert_eq!(route_point_records[0].altitude_meters.as_ref().unwrap().to_string().parse::<f64>().unwrap(), 50.0);
        assert_eq!(route_point_records[0].recorded_at, start + Duration::minutes(10));
        
        // Check second point
        assert_eq!(route_point_records[1].workout_id, record.id);
        assert_eq!(route_point_records[1].point_order, 1);
        assert_eq!(route_point_records[1].latitude, 37.7849);
        assert_eq!(route_point_records[1].longitude, -122.4094);
    }

    #[test]
    fn test_workout_record_with_raw_json() {
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

        let raw_json = json!({
            "workout_metadata": {
                "weather": "sunny",
                "temperature": 22.5,
                "indoor": false
            }
        });

        let record = WorkoutRecord::from_workout_with_raw(workout.clone(), raw_json.clone());
        
        assert_eq!(record.raw_data, Some(raw_json));
        assert_eq!(record.duration_seconds, Some(3600));
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