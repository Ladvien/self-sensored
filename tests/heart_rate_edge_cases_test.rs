use chrono::{DateTime, Utc};
use self_sensored::models::{HeartRateMetric, WorkoutData};

/// Test heart rate validation edge cases to ensure 15 BPM minimum is working
#[cfg(test)]
mod heart_rate_edge_cases {
    use super::*;

    #[test]
    fn test_heart_rate_validation_edge_cases() {
        // Test with exactly 15 BPM (minimum allowed) - should pass
        let valid_hr = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(15),
            avg_bpm: Some(15),
            max_bpm: Some(15),
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };
        assert!(valid_hr.validate().is_ok());

        // Test with 14 BPM (below minimum) - should fail
        let invalid_hr = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(14),
            avg_bpm: Some(14),
            max_bpm: Some(14),
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };
        assert!(invalid_hr.validate().is_err());

        // Test with 16 BPM (above minimum) - should pass
        let valid_hr_16 = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(16),
            avg_bpm: Some(16),
            max_bpm: Some(16),
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };
        assert!(valid_hr_16.validate().is_ok());

        // Test with 300 BPM (maximum allowed) - should pass
        let valid_hr_max = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(300),
            avg_bpm: Some(300),
            max_bpm: Some(300),
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };
        assert!(valid_hr_max.validate().is_ok());

        // Test with 301 BPM (above maximum) - should fail
        let invalid_hr_max = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(301),
            avg_bpm: Some(301),
            max_bpm: Some(301),
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };
        assert!(invalid_hr_max.validate().is_err());
    }

    #[test]
    fn test_workout_heart_rate_validation_edge_cases() {
        let start_time = Utc::now();
        let end_time = start_time + chrono::Duration::minutes(30);

        // Test with exactly 15 BPM in workout (minimum allowed) - should pass
        let valid_workout = WorkoutData {
            workout_type: "Test".to_string(),
            start_time,
            end_time,
            total_energy_kcal: Some(100.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(15),
            max_heart_rate: Some(15),
            source: Some("Test".to_string()),
            route_points: None,
        };
        assert!(valid_workout.validate().is_ok());

        // Test with 14 BPM in workout (below minimum) - should fail
        let invalid_workout = WorkoutData {
            workout_type: "Test".to_string(),
            start_time,
            end_time,
            total_energy_kcal: Some(100.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(14),
            max_heart_rate: Some(14),
            source: Some("Test".to_string()),
            route_points: None,
        };
        assert!(invalid_workout.validate().is_err());

        // Test with 16 BPM in workout (above minimum) - should pass
        let valid_workout_16 = WorkoutData {
            workout_type: "Test".to_string(),
            start_time,
            end_time,
            total_energy_kcal: Some(100.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(16),
            max_heart_rate: Some(16),
            source: Some("Test".to_string()),
            route_points: None,
        };
        assert!(valid_workout_16.validate().is_ok());
    }

    #[test]
    fn test_pulse_validation_in_blood_pressure() {
        use self_sensored::models::BloodPressureMetric;

        // Test pulse with exactly 15 BPM (minimum allowed) - should pass
        let valid_bp = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(15),
            source: Some("Test".to_string()),
        };
        assert!(valid_bp.validate().is_ok());

        // Test pulse with 14 BPM (below minimum) - should fail
        let invalid_bp = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(14),
            source: Some("Test".to_string()),
        };
        assert!(invalid_bp.validate().is_err());

        // Test pulse with 16 BPM (above minimum) - should pass
        let valid_bp_16 = BloodPressureMetric {
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(16),
            source: Some("Test".to_string()),
        };
        assert!(valid_bp_16.validate().is_ok());
    }

    #[test]
    fn test_heart_rate_validation_error_messages() {
        // Test that error messages include the correct range (15-300)
        let invalid_hr_low = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: Some(10),
            avg_bpm: None,
            max_bpm: None,
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };

        match invalid_hr_low.validate() {
            Err(error_msg) => {
                assert!(error_msg.contains("15-300"));
                assert!(error_msg.contains("min_bpm"));
                assert!(error_msg.contains("10"));
            }
            Ok(()) => panic!("Expected validation to fail for heart rate below 15 BPM"),
        }

        let invalid_hr_high = HeartRateMetric {
            recorded_at: Utc::now(),
            min_bpm: None,
            avg_bpm: Some(350),
            max_bpm: None,
            source: Some("Test".to_string()),
            context: Some("resting".to_string()),
        };

        match invalid_hr_high.validate() {
            Err(error_msg) => {
                assert!(error_msg.contains("15-300"));
                assert!(error_msg.contains("avg_bpm"));
                assert!(error_msg.contains("350"));
            }
            Ok(()) => panic!("Expected validation to fail for heart rate above 300 BPM"),
        }
    }
}
