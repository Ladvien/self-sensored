use chrono::Utc;
use self_sensored::models::{HeartRateMetric, WorkoutData, enums::{ActivityContext, WorkoutType}};
use uuid::Uuid;

/// Test heart rate validation edge cases to ensure 15 BPM minimum is working
#[cfg(test)]
mod heart_rate_edge_cases {
    use super::*;

    #[test]
    fn test_heart_rate_validation_edge_cases() {
        // Test with exactly 15 BPM (minimum allowed) - should pass
        let valid_hr = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(15),
            resting_heart_rate: Some(15),
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };
        assert!(valid_hr.validate().is_ok());

        // Test with 14 BPM (below minimum) - should fail
        let invalid_hr = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(14),
            resting_heart_rate: Some(14),
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };
        assert!(invalid_hr.validate().is_err());

        // Test with 16 BPM (above minimum) - should pass
        let valid_hr_16 = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(16),
            resting_heart_rate: Some(16),
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };
        assert!(valid_hr_16.validate().is_ok());

        // Test with 300 BPM (maximum allowed) - should pass
        let valid_hr_max = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(300),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Exercise),
            created_at: Utc::now(),
        };
        assert!(valid_hr_max.validate().is_ok());

        // Test with 301 BPM (above maximum) - should fail
        let invalid_hr_max = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(301),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Exercise),
            created_at: Utc::now(),
        };
        assert!(invalid_hr_max.validate().is_err());
    }

    #[test]
    fn test_workout_heart_rate_validation_edge_cases() {
        let start_time = Utc::now();
        let end_time = start_time + chrono::Duration::minutes(30);

        // Test with exactly 15 BPM in workout (minimum allowed) - should pass
        let valid_workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Other,
            started_at: start_time,
            ended_at: end_time,
            total_energy_kcal: Some(100.0),
            active_energy_kcal: Some(80.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(15),
            max_heart_rate: Some(15),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_workout.validate().is_ok());

        // Test with 14 BPM in workout (below minimum) - should fail
        let invalid_workout = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Other,
            started_at: start_time,
            ended_at: end_time,
            total_energy_kcal: Some(100.0),
            active_energy_kcal: Some(80.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(14),
            max_heart_rate: Some(14),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(invalid_workout.validate().is_err());

        // Test with 16 BPM in workout (above minimum) - should pass
        let valid_workout_16 = WorkoutData {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workout_type: WorkoutType::Other,
            started_at: start_time,
            ended_at: end_time,
            total_energy_kcal: Some(100.0),
            active_energy_kcal: Some(80.0),
            distance_meters: Some(1000.0),
            avg_heart_rate: Some(16),
            max_heart_rate: Some(16),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_workout_16.validate().is_ok());
    }

    #[test]
    fn test_pulse_validation_in_blood_pressure() {
        use self_sensored::models::BloodPressureMetric;

        // Test pulse with exactly 15 BPM (minimum allowed) - should pass
        let valid_bp = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(15),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_bp.validate().is_ok());

        // Test pulse with 14 BPM (below minimum) - should fail
        let invalid_bp = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(14),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(invalid_bp.validate().is_err());

        // Test pulse with 16 BPM (above minimum) - should pass
        let valid_bp_16 = BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            systolic: 120,
            diastolic: 80,
            pulse: Some(16),
            source_device: Some("Test".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_bp_16.validate().is_ok());
    }

    #[test]
    fn test_heart_rate_validation_error_messages() {
        // Test that error messages include the correct range (15-300)
        let invalid_hr_low = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(10),
            resting_heart_rate: None,
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };

        match invalid_hr_low.validate() {
            Err(error_msg) => {
                assert!(error_msg.contains("15-300"));
                assert!(error_msg.contains("heart_rate"));
                assert!(error_msg.contains("10"));
            }
            Ok(()) => panic!("Expected validation to fail for heart rate below 15 BPM"),
        }

        let invalid_hr_high = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(350),
            resting_heart_rate: None,
            heart_rate_variability: None,
            source_device: Some("Test".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };

        match invalid_hr_high.validate() {
            Err(error_msg) => {
                assert!(error_msg.contains("15-300"));
                assert!(error_msg.contains("heart_rate"));
                assert!(error_msg.contains("350"));
            }
            Ok(()) => panic!("Expected validation to fail for heart rate above 300 BPM"),
        }
    }
}
