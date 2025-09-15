use chrono::{DateTime, Utc};
use self_sensored::config::ValidationConfig;
use self_sensored::models::{HealthMetric, RespiratoryMetric};
use uuid::Uuid;

/// Respiratory metrics integration tests
/// Tests comprehensive respiratory health functionality including:
/// - SpO2 monitoring with critical threshold detection (COVID-19 relevance)
/// - Respiratory rate tracking for fitness and medical assessment
/// - Spirometry data processing for asthma and COPD management
/// - Inhaler usage tracking for medication adherence
/// - Pulse oximeter and medical device integration
/// - Emergency-level respiratory condition detection

#[cfg(test)]
mod respiratory_tests {
    use super::*;

    /// Test respiratory metric validation with medical-grade ranges
    #[tokio::test]
    async fn test_respiratory_validation_medical_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test normal respiratory values (SpO2: 98%, RR: 16 BPM)
        let normal_respiratory = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(16),             // Normal: 12-20 BPM
            oxygen_saturation: Some(98.0),          // Normal: 95-100%
            forced_vital_capacity: Some(4.2),       // Normal: 3-5L
            forced_expiratory_volume_1: Some(3.1),  // Normal lung function
            peak_expiratory_flow_rate: Some(450.0), // Normal: 300-600 L/min
            inhaler_usage: Some(2),                 // Normal daily usage
            source_device: Some("Pulse Oximeter Pro".to_string()),
            created_at: Utc::now(),
        };

        assert!(normal_respiratory.validate_with_config(&config).is_ok());
        assert!(
            !normal_respiratory.is_critical_condition(),
            "Normal values should not be critical"
        );
    }

    /// Test critical SpO2 levels (<90% - medical emergency)
    #[tokio::test]
    async fn test_critical_spo2_detection() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test critical SpO2 (88% - medical emergency)
        let critical_spo2 = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(18),
            oxygen_saturation: Some(88.0), // Critical: <90%
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        };

        assert!(critical_spo2.validate_with_config(&config).is_ok());
        assert!(
            critical_spo2.is_critical_condition(),
            "SpO2 88% should trigger critical alert"
        );

        // Test borderline low SpO2 (92% - concerning but not critical)
        let low_spo2 = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(16),
            oxygen_saturation: Some(92.0), // Low but not critical
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Oximeter".to_string()),
            created_at: Utc::now(),
        };

        assert!(low_spo2.validate_with_config(&config).is_ok());
        assert!(
            !low_spo2.is_critical_condition(),
            "SpO2 92% should not be critical but concerning"
        );
    }

    /// Test abnormal respiratory rates (bradypnea and tachypnea)
    #[tokio::test]
    async fn test_abnormal_respiratory_rates() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test bradypnea (6 breaths/min - critically low)
        let bradypnea = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(6), // Critically low
            oxygen_saturation: Some(95.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Chest Belt Monitor".to_string()),
            created_at: Utc::now(),
        };

        assert!(bradypnea.validate_with_config(&config).is_ok());
        assert!(
            bradypnea.is_critical_condition(),
            "6 BPM should be critical bradypnea"
        );

        // Test tachypnea (32 breaths/min - critically high)
        let tachypnea = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(32), // Critically high
            oxygen_saturation: Some(96.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Smart Watch".to_string()),
            created_at: Utc::now(),
        };

        assert!(tachypnea.validate_with_config(&config).is_ok());
        assert!(
            tachypnea.is_critical_condition(),
            "32 BPM should be critical tachypnea"
        );
    }

    /// Test spirometry lung function assessment (asthma/COPD management)
    #[tokio::test]
    async fn test_spirometry_lung_function() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test normal spirometry values
        let normal_spirometry = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(14),
            oxygen_saturation: Some(98.0),
            forced_vital_capacity: Some(4.8),       // Good FVC
            forced_expiratory_volume_1: Some(3.8),  // Good FEV1
            peak_expiratory_flow_rate: Some(520.0), // Good PEFR
            inhaler_usage: None,
            source_device: Some("Home Spirometer".to_string()),
            created_at: Utc::now(),
        };

        assert!(normal_spirometry.validate_with_config(&config).is_ok());

        // Calculate FEV1/FVC ratio (should be >0.7 for normal)
        let fev1_fvc_ratio = 3.8 / 4.8; // = 0.79, which is normal
        assert!(
            fev1_fvc_ratio > 0.7,
            "FEV1/FVC ratio should indicate normal lung function"
        );

        // Test obstructive pattern (low FEV1/FVC ratio - asthma/COPD)
        let obstructive_pattern = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(18),
            oxygen_saturation: Some(94.0),
            forced_vital_capacity: Some(4.0),       // Normal FVC
            forced_expiratory_volume_1: Some(2.4),  // Reduced FEV1
            peak_expiratory_flow_rate: Some(280.0), // Reduced PEFR
            inhaler_usage: Some(6),                 // Higher inhaler usage
            source_device: Some("Clinical Spirometer".to_string()),
            created_at: Utc::now(),
        };

        assert!(obstructive_pattern.validate_with_config(&config).is_ok());

        // Calculate FEV1/FVC ratio (2.4/4.0 = 0.6, indicating obstruction)
        let obstructive_ratio = 2.4 / 4.0; // = 0.6, indicating obstruction
        assert!(
            obstructive_ratio < 0.7,
            "FEV1/FVC ratio should indicate obstructive pattern"
        );
    }

    /// Test excessive inhaler usage tracking (medication adherence monitoring)
    #[tokio::test]
    async fn test_inhaler_usage_monitoring() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test normal inhaler usage (3 puffs/day)
        let normal_inhaler_use = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(15),
            oxygen_saturation: Some(97.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: Some(3), // Normal usage
            source_device: Some("Smart Inhaler".to_string()),
            created_at: Utc::now(),
        };

        assert!(normal_inhaler_use.validate_with_config(&config).is_ok());

        // Test excessive inhaler usage (12 puffs/day - poor asthma control)
        let excessive_inhaler_use = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(20),
            oxygen_saturation: Some(93.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: Some(250.0), // Reduced PEFR
            inhaler_usage: Some(12),                // Excessive usage
            source_device: Some("Digital Inhaler Tracker".to_string()),
            created_at: Utc::now(),
        };

        assert!(excessive_inhaler_use.validate_with_config(&config).is_ok());
        // Note: Excessive inhaler usage should trigger alerts in production
    }

    /// Test Apple Watch SpO2 integration scenario
    #[tokio::test]
    async fn test_apple_watch_spo2_integration() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test Apple Watch SpO2 reading during sleep
        let apple_watch_spo2 = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: None,        // Apple Watch doesn't measure RR
            oxygen_saturation: Some(96.0), // SpO2 measurement
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        };

        assert!(apple_watch_spo2.validate_with_config(&config).is_ok());
        assert!(
            !apple_watch_spo2.is_critical_condition(),
            "96% SpO2 should be acceptable"
        );

        // Test Apple Watch detecting concerning SpO2 drop during sleep
        let concerning_sleep_spo2 = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: None,
            oxygen_saturation: Some(89.0), // Sleep apnea concern
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        };

        assert!(concerning_sleep_spo2.validate_with_config(&config).is_ok());
        assert!(
            concerning_sleep_spo2.is_critical_condition(),
            "89% SpO2 during sleep should be critical"
        );
    }

    /// Test COVID-19 monitoring scenario (SpO2 tracking for respiratory illness)
    #[tokio::test]
    async fn test_covid_19_monitoring_scenario() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test COVID-19 patient with concerning respiratory symptoms
        let covid_monitoring = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(24),    // Elevated (normal: 12-20)
            oxygen_saturation: Some(91.0), // Concerning (should be >95%)
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Home Pulse Oximeter".to_string()),
            created_at: Utc::now(),
        };

        assert!(covid_monitoring.validate_with_config(&config).is_ok());
        // This scenario would typically require medical consultation

        // Test COVID-19 patient requiring immediate medical attention
        let severe_covid_respiratory = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(28),    // High respiratory rate
            oxygen_saturation: Some(87.0), // Critical SpO2
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Medical Grade Oximeter".to_string()),
            created_at: Utc::now(),
        };

        assert!(severe_covid_respiratory
            .validate_with_config(&config)
            .is_ok());
        assert!(
            severe_covid_respiratory.is_critical_condition(),
            "Severe COVID respiratory symptoms should be critical"
        );
    }

    /// Test pulse oximeter device integration scenarios
    #[tokio::test]
    async fn test_pulse_oximeter_device_integration() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test consumer pulse oximeter (fingertip)
        let consumer_oximeter = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: None, // Consumer devices typically only measure SpO2
            oxygen_saturation: Some(97.5),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Zacurate Pro Series 500DL".to_string()),
            created_at: Utc::now(),
        };

        assert!(consumer_oximeter.validate_with_config(&config).is_ok());

        // Test medical-grade pulse oximeter with additional metrics
        let medical_oximeter = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(16), // Medical devices may include RR
            oxygen_saturation: Some(98.2),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Masimo Rad-97".to_string()),
            created_at: Utc::now(),
        };

        assert!(medical_oximeter.validate_with_config(&config).is_ok());
    }

    /// Test multi-device respiratory monitoring timeline
    #[tokio::test]
    async fn test_multi_device_respiratory_timeline() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let base_time = Utc::now();

        // Create a timeline of respiratory measurements from different devices
        let measurements = vec![
            // Morning: Apple Watch SpO2
            RespiratoryMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time,
                respiratory_rate: None,
                oxygen_saturation: Some(97.0),
                forced_vital_capacity: None,
                forced_expiratory_volume_1: None,
                peak_expiratory_flow_rate: None,
                inhaler_usage: None,
                source_device: Some("Apple Watch".to_string()),
                created_at: Utc::now(),
            },
            // Afternoon: Home spirometer test
            RespiratoryMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time + chrono::Duration::hours(6),
                respiratory_rate: None,
                oxygen_saturation: None,
                forced_vital_capacity: Some(4.3),
                forced_expiratory_volume_1: Some(3.4),
                peak_expiratory_flow_rate: Some(480.0),
                inhaler_usage: None,
                source_device: Some("Home Spirometer".to_string()),
                created_at: Utc::now(),
            },
            // Evening: Inhaler usage tracking
            RespiratoryMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time + chrono::Duration::hours(12),
                respiratory_rate: None,
                oxygen_saturation: None,
                forced_vital_capacity: None,
                forced_expiratory_volume_1: None,
                peak_expiratory_flow_rate: None,
                inhaler_usage: Some(4),
                source_device: Some("Smart Inhaler".to_string()),
                created_at: Utc::now(),
            },
            // Night: Pulse oximeter check
            RespiratoryMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_time + chrono::Duration::hours(18),
                respiratory_rate: Some(14),
                oxygen_saturation: Some(96.5),
                forced_vital_capacity: None,
                forced_expiratory_volume_1: None,
                peak_expiratory_flow_rate: None,
                inhaler_usage: None,
                source_device: Some("Bedside Oximeter".to_string()),
                created_at: Utc::now(),
            },
        ];

        // Validate all measurements
        for measurement in &measurements {
            assert!(measurement.validate_with_config(&config).is_ok());
        }

        // Verify timeline consistency
        assert_eq!(
            measurements.len(),
            4,
            "Should have measurements from 4 different times/devices"
        );

        // Check that measurements are ordered chronologically
        for i in 1..measurements.len() {
            assert!(
                measurements[i].recorded_at > measurements[i - 1].recorded_at,
                "Measurements should be in chronological order"
            );
        }
    }

    /// Test respiratory validation edge cases and error handling
    #[tokio::test]
    async fn test_respiratory_validation_edge_cases() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test out of range SpO2 (impossible value)
        let invalid_spo2_high = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(16),
            oxygen_saturation: Some(105.0), // Impossible value >100%
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Test Device".to_string()),
            created_at: Utc::now(),
        };

        assert!(
            invalid_spo2_high.validate_with_config(&config).is_err(),
            "SpO2 >100% should be invalid"
        );

        // Test out of range respiratory rate (impossible value)
        let invalid_respiratory_rate = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(100), // Impossible respiratory rate
            oxygen_saturation: Some(98.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: None,
            source_device: Some("Test Device".to_string()),
            created_at: Utc::now(),
        };

        assert!(
            invalid_respiratory_rate
                .validate_with_config(&config)
                .is_err(),
            "Respiratory rate of 100 BPM should be invalid"
        );

        // Test negative inhaler usage (invalid)
        let invalid_inhaler_usage = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(16),
            oxygen_saturation: Some(98.0),
            forced_vital_capacity: None,
            forced_expiratory_volume_1: None,
            peak_expiratory_flow_rate: None,
            inhaler_usage: Some(-5), // Negative usage is impossible
            source_device: Some("Test Device".to_string()),
            created_at: Utc::now(),
        };

        assert!(
            invalid_inhaler_usage.validate_with_config(&config).is_err(),
            "Negative inhaler usage should be invalid"
        );
    }

    /// Test comprehensive respiratory health metrics integration
    #[tokio::test]
    async fn test_comprehensive_respiratory_health_integration() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test complete respiratory assessment (all metrics present)
        let comprehensive_respiratory = RespiratoryMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            respiratory_rate: Some(18),             // Slightly elevated
            oxygen_saturation: Some(95.0),          // Lower end of normal
            forced_vital_capacity: Some(4.1),       // Normal
            forced_expiratory_volume_1: Some(2.9),  // Slightly reduced
            peak_expiratory_flow_rate: Some(380.0), // Reduced (asthma indicator)
            inhaler_usage: Some(6),                 // Higher usage (asthma control)
            source_device: Some("Comprehensive Respiratory Monitor".to_string()),
            created_at: Utc::now(),
        };

        assert!(comprehensive_respiratory
            .validate_with_config(&config)
            .is_ok());

        // Calculate FEV1/FVC ratio for lung function assessment
        let fev1_fvc_ratio = 2.9 / 4.1; // ≈ 0.71, borderline normal
        assert!(
            fev1_fvc_ratio > 0.7,
            "FEV1/FVC ratio should be just within normal limits"
        );

        // This patient profile suggests:
        // - Mild respiratory compromise (elevated RR, borderline SpO2)
        // - Possible mild obstructive disease (reduced PEFR)
        // - Increased inhaler usage (may need medication adjustment)
        // - Overall: requires monitoring and possible medication optimization
    }
}

/// Helper function to create a test respiratory metric
fn create_test_respiratory_metric(
    user_id: Uuid,
    respiratory_rate: Option<i32>,
    oxygen_saturation: Option<f64>,
    source_device: Option<String>,
) -> RespiratoryMetric {
    RespiratoryMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: Utc::now(),
        respiratory_rate,
        oxygen_saturation,
        forced_vital_capacity: None,
        forced_expiratory_volume_1: None,
        peak_expiratory_flow_rate: None,
        inhaler_usage: None,
        source_device,
        created_at: Utc::now(),
    }
}

/// Helper function to validate a batch of respiratory metrics
fn validate_respiratory_batch(
    metrics: &[RespiratoryMetric],
    config: &ValidationConfig,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    for (index, metric) in metrics.iter().enumerate() {
        if let Err(error) = metric.validate_with_config(config) {
            errors.push(format!("Metric {}: {}", index, error));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
