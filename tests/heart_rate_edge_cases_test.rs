use chrono::Utc;
use rust_decimal::Decimal;
use self_sensored::models::{
    enums::{ActivityContext, CardiacEventSeverity, HeartRateEventType, WorkoutType},
    HeartRateEvent, HeartRateMetric, WorkoutData,
};
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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

            // Advanced Cardiovascular Metrics (STORY-011)
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,

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

    /// Test Advanced Cardiovascular Metrics Validation (STORY-011)
    #[test]
    fn test_advanced_cardiovascular_metrics_validation() {
        use std::str::FromStr;

        // Test valid walking heart rate average (90-120 BPM normal range)
        let valid_walking_hr = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(105),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: Some(95), // Valid walking HR
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Walking),
            created_at: Utc::now(),
        };
        assert!(valid_walking_hr.validate().is_ok());

        // Test invalid walking heart rate (too high)
        let invalid_walking_hr = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(105),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: Some(250), // Invalid - too high
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Walking),
            created_at: Utc::now(),
        };
        assert!(invalid_walking_hr.validate().is_err());

        // Test valid heart rate recovery (18+ is considered good)
        let valid_hr_recovery = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(160),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: Some(25), // Good recovery
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Recovery),
            created_at: Utc::now(),
        };
        assert!(valid_hr_recovery.validate().is_ok());

        // Test invalid heart rate recovery (negative)
        let invalid_hr_recovery = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(160),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: Some(-5), // Invalid - negative
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Recovery),
            created_at: Utc::now(),
        };
        assert!(invalid_hr_recovery.validate().is_err());

        // Test valid AFib burden percentage (Apple Watch shows "2% or less")
        let valid_afib_burden = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(85),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: Some(Decimal::from_str("2.5").unwrap()), // Valid AFib burden
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };
        assert!(valid_afib_burden.validate().is_ok());

        // Test invalid AFib burden (over 100%)
        let invalid_afib_burden = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(85),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: Some(Decimal::from_str("105.0").unwrap()), // Invalid - over 100%
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Resting),
            created_at: Utc::now(),
        };
        assert!(invalid_afib_burden.validate().is_err());

        // Test valid VO2 max (Apple Watch supported range: 14-65 ml/kg/min)
        let valid_vo2_max = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(150),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: Some(Decimal::from_str("42.5").unwrap()), // Valid VO2 max for active person
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Exercise),
            created_at: Utc::now(),
        };
        assert!(valid_vo2_max.validate().is_ok());

        // Test invalid VO2 max (too high)
        let invalid_vo2_max = HeartRateMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(150),
            resting_heart_rate: Some(65),
            heart_rate_variability: None,
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: Some(Decimal::from_str("75.0").unwrap()), // Invalid - above Apple Watch max
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Exercise),
            created_at: Utc::now(),
        };
        assert!(invalid_vo2_max.validate().is_err());
    }

    /// Test Heart Rate Event Validation (STORY-011: Cardiac Event Detection)
    #[test]
    fn test_heart_rate_event_validation() {
        // Test valid HIGH event with appropriate heart rate
        let valid_high_event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::High,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 180, // Valid for HIGH event (>= 100 BPM)
            event_duration_minutes: Some(5),
            context: Some(ActivityContext::Exercise),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Moderate,
            is_confirmed: false,
            notes: Some("Detected during high-intensity exercise".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_high_event.validate().is_ok());

        // Test invalid HIGH event with low heart rate
        let invalid_high_event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::High,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 80, // Invalid for HIGH event (< 100 BPM)
            event_duration_minutes: Some(5),
            context: Some(ActivityContext::Exercise),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Moderate,
            is_confirmed: false,
            notes: None,
            created_at: Utc::now(),
        };
        assert!(invalid_high_event.validate().is_err());

        // Test valid LOW event (bradycardia)
        let valid_low_event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::Low,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 45, // Valid for LOW event (<= 60 BPM)
            event_duration_minutes: Some(10),
            context: Some(ActivityContext::Resting),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Moderate,
            is_confirmed: false,
            notes: Some("Detected during rest".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_low_event.validate().is_ok());

        // Test valid AFIB event
        let valid_afib_event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::Afib,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 120, // Valid for AFIB (>= 60 BPM)
            event_duration_minutes: Some(15),
            context: Some(ActivityContext::Resting),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::High,
            is_confirmed: false,
            notes: Some("Irregular rhythm detected".to_string()),
            created_at: Utc::now(),
        };
        assert!(valid_afib_event.validate().is_ok());

        // Test critical severity duration validation
        let invalid_critical_duration = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::High,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 200,
            event_duration_minutes: Some(10), // > 5 minutes for critical event
            context: Some(ActivityContext::Exercise),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Critical,
            is_confirmed: false, // Not medically confirmed
            notes: None,
            created_at: Utc::now(),
        };
        assert!(invalid_critical_duration.validate().is_err());
    }

    /// Test Heart Rate Event Risk Assessment (STORY-011)
    #[test]
    fn test_heart_rate_event_risk_assessment() {
        // Test CRITICAL AFib event
        let critical_afib = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::Afib,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 150,
            event_duration_minutes: Some(120), // 2 hours
            context: Some(ActivityContext::Resting),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Critical,
            is_confirmed: true,
            notes: Some("Sustained AFib requiring immediate attention".to_string()),
            created_at: Utc::now(),
        };

        // Test risk score calculation
        let risk_score = critical_afib.calculate_risk_score();
        assert!(
            risk_score >= 90,
            "Critical AFib should have high risk score: {risk_score}"
        );

        // Test medical urgency assessment
        let urgency = critical_afib.get_medical_urgency();
        assert!(
            urgency.contains("EMERGENCY"),
            "Critical AFib should be emergency: {urgency}"
        );

        // Test low-risk event
        let low_risk_event = HeartRateEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            event_type: HeartRateEventType::SlowRecovery,
            event_occurred_at: Utc::now(),
            heart_rate_at_event: 120,
            event_duration_minutes: Some(2),
            context: Some(ActivityContext::Recovery),
            source_device: Some("Apple Watch".to_string()),
            severity: CardiacEventSeverity::Low,
            is_confirmed: false,
            notes: None,
            created_at: Utc::now(),
        };

        let low_risk_score = low_risk_event.calculate_risk_score();
        assert!(
            low_risk_score <= 20,
            "Low-risk event should have low score: {low_risk_score}"
        );

        let low_urgency = low_risk_event.get_medical_urgency();
        assert!(
            low_urgency.contains("LOW"),
            "Low-risk event should have low urgency: {low_urgency}"
        );
    }
}
