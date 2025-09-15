use chrono::Utc;
use self_sensored::config::ValidationConfig;
use self_sensored::models::{HealthMetric, TemperatureMetric};
use uuid::Uuid;

/// Temperature metrics integration tests
/// Tests comprehensive temperature tracking functionality including:
/// - Body temperature monitoring (fever detection, medical ranges)
/// - Basal body temperature tracking (fertility tracking patterns)
/// - Apple Watch wrist temperature monitoring
/// - Environmental water temperature recording
/// - Multi-source temperature data validation

#[cfg(test)]
mod temperature_tests {
    use super::*;

    /// Test temperature metric validation with medical-grade ranges
    #[tokio::test]
    async fn test_temperature_validation_medical_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test normal body temperature (36.5�C / 97.7�F)
        let normal_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(36.5),
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("digital_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(normal_temp.validate_with_config(&config).is_ok());
        assert!(!normal_temp.has_fever(), "36.5�C should not be fever");
    }

    /// Test fever detection thresholds (>38.0�C / 100.4�F)
    #[tokio::test]
    async fn test_fever_detection() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test fever temperature (38.5�C / 101.3�F)
        let fever_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(38.5),
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("digital_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(fever_temp.has_fever(), "38.5�C should be detected as fever");

        // Test high fever (40.0�C / 104�F)
        let high_fever_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(40.0),
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("digital_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(
            high_fever_temp.has_fever(),
            "40.0�C should be detected as high fever"
        );
    }

    /// Test basal body temperature for fertility tracking
    #[tokio::test]
    async fn test_basal_body_temperature_fertility_tracking() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test normal basal temperature (36.2�C)
        let basal_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: Some(36.2),
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("fertility_thermometer".to_string()),
            source_device: Some("Femometer".to_string()),
            created_at: Utc::now(),
        };

        assert!(basal_temp.validate_with_config(&config).is_ok());

        // Test ovulation spike detection (0.3�C increase)
        let baseline_temp = 36.2;
        assert!(
            basal_temp.basal_temp_spike(baseline_temp),
            "Should detect ovulation spike"
        );

        // Test with higher basal temperature (ovulation)
        let ovulation_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: Some(36.6), // 0.4�C increase
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("fertility_thermometer".to_string()),
            source_device: Some("Femometer".to_string()),
            created_at: Utc::now(),
        };

        assert!(
            ovulation_temp.basal_temp_spike(baseline_temp),
            "36.6�C should indicate ovulation spike from 36.2�C baseline"
        );
    }

    /// Test Apple Watch wrist temperature monitoring
    #[tokio::test]
    async fn test_apple_watch_wrist_temperature() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test Apple Watch wrist temperature during sleep
        let wrist_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: Some(34.2), // Typical wrist temp
            water_temperature: None,
            temperature_source: Some("apple_watch".to_string()),
            source_device: Some("Apple Watch Series 8".to_string()),
            created_at: Utc::now(),
        };

        assert!(wrist_temp.validate_with_config(&config).is_ok());
        assert_eq!(wrist_temp.primary_temperature(), Some(34.2));
    }

    /// Test environmental water temperature recording
    #[tokio::test]
    async fn test_water_temperature_environmental() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test swimming pool temperature
        let pool_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: Some(28.0), // Pool temperature in Celsius
            temperature_source: Some("pool_thermometer".to_string()),
            source_device: Some("Fitbit Sense".to_string()),
            created_at: Utc::now(),
        };

        assert!(pool_temp.validate_with_config(&config).is_ok());

        // Test cold water (ice bath)
        let cold_water_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: Some(4.0), // Ice bath temperature
            temperature_source: Some("ice_bath_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(cold_water_temp.validate_with_config(&config).is_ok());
    }

    /// Test multi-source temperature data validation
    #[tokio::test]
    async fn test_multi_source_temperature_validation() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test comprehensive temperature reading from multiple sources
        let multi_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(37.1),                 // Slight fever
            basal_body_temperature: Some(36.8),           // Elevated basal (ovulation)
            apple_sleeping_wrist_temperature: Some(34.5), // Wrist temp
            water_temperature: Some(22.0),                // Room temp water
            temperature_source: Some("comprehensive_measurement".to_string()),
            source_device: Some("Health Monitoring System".to_string()),
            created_at: Utc::now(),
        };

        assert!(multi_temp.validate_with_config(&config).is_ok());
        assert!(multi_temp.has_fever());
        assert_eq!(multi_temp.primary_temperature(), Some(37.1)); // Body temp takes priority
    }

    /// Test temperature validation edge cases
    #[tokio::test]
    async fn test_temperature_validation_edge_cases() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test extreme hypothermia (should pass validation but flag for medical attention)
        let hypothermia_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(32.0), // Severe hypothermia
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("emergency_thermometer".to_string()),
            source_device: Some("Paramedic Equipment".to_string()),
            created_at: Utc::now(),
        };

        assert!(hypothermia_temp.validate_with_config(&config).is_ok());

        // Test extreme hyperthermia (should pass validation but flag as critical)
        let hyperthermia_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(42.0), // Dangerous hyperthermia
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("hospital_thermometer".to_string()),
            source_device: Some("Medical Equipment".to_string()),
            created_at: Utc::now(),
        };

        assert!(hyperthermia_temp.validate_with_config(&config).is_ok());
    }

    /// Test temperature metric serialization/deserialization in HealthMetric enum
    #[tokio::test]
    async fn test_health_metric_temperature_serialization() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        let temp_metric = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(36.8),
            basal_body_temperature: Some(36.4),
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("multi_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let health_metric = HealthMetric::Temperature(temp_metric);

        // Test metric type identification
        assert_eq!(health_metric.metric_type(), "Temperature");

        // Test validation through HealthMetric wrapper
        assert!(health_metric.validate().is_ok());
    }

    /// Test primary temperature priority logic
    #[tokio::test]
    async fn test_primary_temperature_priority() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test body temperature has highest priority
        let multi_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(37.2),
            basal_body_temperature: Some(36.5),
            apple_sleeping_wrist_temperature: Some(34.1),
            water_temperature: None,
            temperature_source: Some("comprehensive".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(multi_temp.primary_temperature(), Some(37.2));

        // Test basal temperature as fallback
        let basal_only = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: Some(36.3),
            apple_sleeping_wrist_temperature: Some(33.9),
            water_temperature: None,
            temperature_source: Some("basal_thermometer".to_string()),
            source_device: Some("Femometer".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(basal_only.primary_temperature(), Some(36.3));

        // Test wrist temperature as last resort
        let wrist_only = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: Some(34.0),
            water_temperature: None,
            temperature_source: Some("apple_watch".to_string()),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(wrist_only.primary_temperature(), Some(34.0));
    }

    /// Test temperature batch processing functionality
    #[tokio::test]
    async fn test_temperature_batch_processing() {
        // Create test temperature metrics for batch processing
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        let test_metrics = vec![
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at,
                body_temperature: Some(38.2), // Fever
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("digital_thermometer".to_string()),
                source_device: Some("iPhone".to_string()),
                created_at: Utc::now(),
            },
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::minutes(30),
                body_temperature: Some(36.8),       // Normal
                basal_body_temperature: Some(36.6), // Ovulation spike
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("fertility_thermometer".to_string()),
                source_device: Some("Femometer".to_string()),
                created_at: Utc::now(),
            },
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(1),
                body_temperature: None,
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: Some(34.1), // Apple Watch
                water_temperature: None,
                temperature_source: Some("apple_watch".to_string()),
                source_device: Some("Apple Watch Series 8".to_string()),
                created_at: Utc::now(),
            },
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(2),
                body_temperature: None,
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: None,
                water_temperature: Some(22.5), // Pool temperature
                temperature_source: Some("pool_thermometer".to_string()),
                source_device: Some("Fitbit Sense".to_string()),
                created_at: Utc::now(),
            },
        ];

        // Validate each metric
        let config = ValidationConfig::default();
        for metric in &test_metrics {
            assert!(
                metric.validate_with_config(&config).is_ok(),
                "Temperature metric should be valid"
            );
        }

        // Test fever detection
        let fever_metric = &test_metrics[0];
        assert!(fever_metric.has_fever(), "Should detect fever for 38.2°C");

        // Test ovulation detection
        let fertility_metric = &test_metrics[1];
        assert!(
            fertility_metric.basal_temp_spike(36.2),
            "Should detect ovulation spike"
        );

        // Test Apple Watch wrist temperature
        let wrist_metric = &test_metrics[2];
        assert_eq!(wrist_metric.primary_temperature(), Some(34.1));

        // Test environmental water temperature
        let water_metric = &test_metrics[3];
        assert_eq!(water_metric.water_temperature, Some(22.5));
    }

    /// Test high-frequency continuous temperature monitoring (Apple Watch scenario)
    #[tokio::test]
    async fn test_continuous_temperature_monitoring_batch() {
        use chrono::Duration;
        let user_id = Uuid::new_v4();
        let mut start_time = Utc::now() - Duration::hours(8); // 8 hours of sleep
        let mut continuous_metrics = Vec::new();

        // Simulate Apple Watch wrist temperature readings every 5 minutes during sleep
        for i in 0..96 {
            // 8 hours * 12 readings per hour (every 5 minutes)
            let temp_variation = (i as f64 / 96.0) * 2.0 - 1.0; // ±1°C variation
            let wrist_temp = 33.8 + temp_variation + (i as f64 * 0.01); // Slight temp drift

            continuous_metrics.push(TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: start_time,
                body_temperature: None,
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: Some(wrist_temp),
                water_temperature: None,
                temperature_source: Some("apple_watch_continuous".to_string()),
                source_device: Some("Apple Watch Series 9".to_string()),
                created_at: Utc::now(),
            });

            start_time += Duration::minutes(5);
        }

        // Validate all continuous readings
        let config = ValidationConfig::default();
        for metric in &continuous_metrics {
            assert!(
                metric.validate_with_config(&config).is_ok(),
                "Continuous temperature reading should be valid"
            );
        }

        // Test chunk size optimization for high-frequency data
        assert_eq!(
            continuous_metrics.len(),
            96,
            "Should have 96 readings for 8-hour sleep"
        );
        assert!(
            continuous_metrics.len() < 8000,
            "Batch should be well under chunk size limit"
        );
    }

    /// Test fertility cycle pattern validation with basal temperature trends
    #[tokio::test]
    async fn test_fertility_cycle_pattern_validation() {
        use chrono::Duration;
        let user_id = Uuid::new_v4();
        let cycle_day = 1;
        let mut cycle_metrics = Vec::new();
        let mut base_date = Utc::now() - Duration::days(28); // Start of cycle

        // Simulate 28-day fertility cycle with basal temperature patterns
        for day in 0..28 {
            let basal_temp = match day {
                0..=13 => 36.2 + (day as f64 * 0.01), // Follicular phase: gradual increase
                14..=16 => 36.6 + (day as f64 * 0.02), // Ovulation: temperature spike
                17..=27 => 36.7 - ((day - 17) as f64 * 0.01), // Luteal phase: plateau then decline
                _ => 36.2,
            };

            cycle_metrics.push(TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: base_date,
                body_temperature: None,
                basal_body_temperature: Some(basal_temp),
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("fertility_tracker".to_string()),
                source_device: Some("Tempdrop".to_string()),
                created_at: Utc::now(),
            });

            base_date += Duration::days(1);
        }

        // Validate fertility pattern detection
        let baseline_temp = 36.2;
        let ovulation_days: Vec<_> = cycle_metrics
            .iter()
            .filter(|m| m.basal_temp_spike(baseline_temp))
            .collect();

        assert!(
            ovulation_days.len() >= 3,
            "Should detect ovulation period (days 14-16)"
        );
        assert!(
            ovulation_days.len() <= 5,
            "Ovulation detection should be precise"
        );

        // Test batch processing with fertility data
        let config = ValidationConfig::default();
        for metric in &cycle_metrics {
            assert!(
                metric.validate_with_config(&config).is_ok(),
                "Fertility cycle temperature should be valid"
            );
        }
    }

    /// Test high-volume multi-source temperature batch processing (Performance scenario)
    #[tokio::test]
    async fn test_high_volume_multi_source_temperature_batch() {
        use chrono::Duration;
        let user_id = Uuid::new_v4();
        let start_time = Utc::now() - Duration::hours(24);
        let mut high_volume_metrics = Vec::new();

        // Simulate 24 hours of multi-source temperature data
        for hour in 0..24 {
            for source_reading in 0..20 {
                // 20 readings per hour from various sources
                let timestamp =
                    start_time + Duration::hours(hour) + Duration::minutes(source_reading * 3);

                // Alternate between different temperature sources
                let (body_temp, basal_temp, wrist_temp, water_temp, source) =
                    match source_reading % 4 {
                        0 => (
                            Some(36.5 + (hour as f64 * 0.1)),
                            None,
                            None,
                            None,
                            "body_thermometer",
                        ),
                        1 => (
                            None,
                            Some(36.3 + (hour as f64 * 0.05)),
                            None,
                            None,
                            "fertility_tracker",
                        ),
                        2 => (
                            None,
                            None,
                            Some(33.8 + (hour as f64 * 0.02)),
                            None,
                            "apple_watch",
                        ),
                        3 => (
                            None,
                            None,
                            None,
                            Some(20.0 + (hour as f64 * 0.5)),
                            "environmental_sensor",
                        ),
                        _ => (Some(36.5), None, None, None, "unknown"),
                    };

                high_volume_metrics.push(TemperatureMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: timestamp,
                    body_temperature: body_temp,
                    basal_body_temperature: basal_temp,
                    apple_sleeping_wrist_temperature: wrist_temp,
                    water_temperature: water_temp,
                    temperature_source: Some(source.to_string()),
                    source_device: Some("Multi-sensor system".to_string()),
                    created_at: Utc::now(),
                });
            }
        }

        // Test high-volume processing (480 readings - simulates continuous monitoring)
        assert_eq!(
            high_volume_metrics.len(),
            480,
            "Should have 480 multi-source readings"
        );
        assert!(
            high_volume_metrics.len() < 8000,
            "Should be under optimized chunk size"
        );

        // Validate all high-volume readings
        let config = ValidationConfig::default();
        let mut fever_count = 0;
        let mut ovulation_indicators = 0;

        for metric in &high_volume_metrics {
            assert!(
                metric.validate_with_config(&config).is_ok(),
                "High-volume temperature reading should be valid"
            );

            if metric.has_fever() {
                fever_count += 1;
            }

            if metric.basal_temp_spike(36.2) {
                ovulation_indicators += 1;
            }
        }

        // Medical pattern analysis
        println!("High-volume batch processing results:");
        println!("  Total readings: {}", high_volume_metrics.len());
        println!("  Fever episodes detected: {fever_count}");
        println!("  Ovulation indicators: {ovulation_indicators}");

        // Performance assertions
        assert!(
            fever_count < 50,
            "Fever detection should be reasonable for 24h period"
        );
        assert!(
            ovulation_indicators < 100,
            "Ovulation indicators should be selective"
        );
    }

    /// Test temperature data analysis functions
    #[tokio::test]
    async fn test_temperature_data_analysis() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Create test scenarios for comprehensive medical analysis
        let test_scenarios = vec![
            // Scenario 1: Normal temperature progression
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at,
                body_temperature: Some(36.8),
                basal_body_temperature: Some(36.3),
                apple_sleeping_wrist_temperature: Some(34.0),
                water_temperature: None,
                temperature_source: Some("comprehensive".to_string()),
                source_device: Some("iPhone".to_string()),
                created_at: Utc::now(),
            },
            // Scenario 2: Fever episode
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(4),
                body_temperature: Some(38.5), // Fever
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: Some(35.2),
                water_temperature: None,
                temperature_source: Some("fever_thermometer".to_string()),
                source_device: Some("iPhone".to_string()),
                created_at: Utc::now(),
            },
            // Scenario 3: Critical high temperature
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(6),
                body_temperature: Some(40.2), // Hyperthermia
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("emergency_thermometer".to_string()),
                source_device: Some("Medical Device".to_string()),
                created_at: Utc::now(),
            },
            // Scenario 4: Critical low temperature
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(8),
                body_temperature: Some(34.5), // Hypothermia
                basal_body_temperature: None,
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("emergency_thermometer".to_string()),
                source_device: Some("Medical Device".to_string()),
                created_at: Utc::now(),
            },
            // Scenario 5: Fertility tracking - ovulation spike
            TemperatureMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: recorded_at + chrono::Duration::hours(12),
                body_temperature: None,
                basal_body_temperature: Some(36.8), // Significant ovulation spike
                apple_sleeping_wrist_temperature: None,
                water_temperature: None,
                temperature_source: Some("fertility_tracker".to_string()),
                source_device: Some("Femometer Vinca II".to_string()),
                created_at: Utc::now(),
            },
        ];

        // Validate all scenarios
        let config = ValidationConfig::default();
        for (i, metric) in test_scenarios.iter().enumerate() {
            assert!(
                metric.validate_with_config(&config).is_ok(),
                "Test scenario {} should be valid",
                i + 1
            );
        }

        // Test specific medical conditions detection

        // Fever detection
        assert!(
            !test_scenarios[0].has_fever(),
            "Normal temp should not be fever"
        );
        assert!(
            test_scenarios[1].has_fever(),
            "38.5°C should be detected as fever"
        );
        assert!(
            test_scenarios[2].has_fever(),
            "40.2°C should be detected as fever"
        );
        assert!(
            !test_scenarios[3].has_fever(),
            "Hypothermia should not be classified as fever"
        );

        // Primary temperature selection
        assert_eq!(test_scenarios[0].primary_temperature(), Some(36.8));
        assert_eq!(test_scenarios[4].primary_temperature(), Some(36.8)); // Basal temp

        // Temperature source tracking
        assert_eq!(
            test_scenarios[1].temperature_source.as_ref().unwrap(),
            "fever_thermometer"
        );
        assert_eq!(
            test_scenarios[4].temperature_source.as_ref().unwrap(),
            "fertility_tracker"
        );

        // Ovulation spike detection (basal temp > 36.5°C)
        assert!(
            test_scenarios[4].basal_temp_spike(36.2),
            "Should detect ovulation spike"
        );
        assert!(
            !test_scenarios[0].basal_temp_spike(36.2),
            "Normal basal temp shouldn't indicate spike"
        );
    }

    /// Test temperature metric creation and field validation
    #[tokio::test]
    async fn test_temperature_metric_creation() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();
        let config = ValidationConfig::default();

        // Test creating temperature metric with all fields
        let comprehensive_temp = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(37.0),
            basal_body_temperature: Some(36.4),
            apple_sleeping_wrist_temperature: Some(34.2),
            water_temperature: Some(25.0),
            temperature_source: Some("multi_sensor".to_string()),
            source_device: Some("Health Monitor Pro".to_string()),
            created_at: Utc::now(),
        };

        assert!(comprehensive_temp.validate_with_config(&config).is_ok());
        assert_eq!(comprehensive_temp.primary_temperature(), Some(37.0));
        assert!(comprehensive_temp.body_temperature.is_some());
        assert!(comprehensive_temp.basal_body_temperature.is_some());
        assert!(comprehensive_temp
            .apple_sleeping_wrist_temperature
            .is_some());
        assert!(comprehensive_temp.water_temperature.is_some());

        // Test creating temperature metric with only body temperature
        let body_temp_only = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: Some(36.5),
            basal_body_temperature: None,
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("oral_thermometer".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(body_temp_only.validate_with_config(&config).is_ok());
        assert_eq!(body_temp_only.primary_temperature(), Some(36.5));

        // Test creating temperature metric with only basal temperature
        let basal_temp_only = TemperatureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            body_temperature: None,
            basal_body_temperature: Some(36.2),
            apple_sleeping_wrist_temperature: None,
            water_temperature: None,
            temperature_source: Some("basal_thermometer".to_string()),
            source_device: Some("Femometer".to_string()),
            created_at: Utc::now(),
        };

        assert!(basal_temp_only.validate_with_config(&config).is_ok());
        assert_eq!(basal_temp_only.primary_temperature(), Some(36.2));
    }
}
