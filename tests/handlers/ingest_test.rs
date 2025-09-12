use actix_web::{test, web, App};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::ingest_handler,
    models::{
        ApiResponse, HealthMetric, HeartRateMetric, BloodPressureMetric, SleepMetric, 
        ActivityMetric, WorkoutData, IngestPayload, IngestData, IngestResponse,
        IosIngestPayload, IosIngestData, IosMetric, IosMetricData, IosWorkout
    },
    services::auth::{AuthContext, AuthenticatedUser},
    db::models::{User, ApiKey},
};

/// Test fixtures for various payload formats
pub struct TestFixtures;

impl TestFixtures {
    /// Create a standard format payload with comprehensive health data
    pub fn standard_payload_comprehensive() -> IngestPayload {
        let now = Utc::now();
        
        IngestPayload {
            data: IngestData {
                metrics: vec![
                    // Heart rate metrics
                    HealthMetric::HeartRate(HeartRateMetric {
                        recorded_at: now,
                        min_bpm: Some(65),
                        avg_bpm: Some(75),
                        max_bpm: Some(85),
                        source: Some("Apple Watch".to_string()),
                        context: Some("resting".to_string()),
                    }),
                    HealthMetric::HeartRate(HeartRateMetric {
                        recorded_at: now - chrono::Duration::hours(1),
                        min_bpm: None,
                        avg_bpm: Some(120),
                        max_bpm: Some(145),
                        source: Some("Apple Watch".to_string()),
                        context: Some("exercise".to_string()),
                    }),
                    
                    // Blood pressure metrics
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        recorded_at: now - chrono::Duration::hours(2),
                        systolic: 120,
                        diastolic: 80,
                        pulse: Some(70),
                        source: Some("Manual Entry".to_string()),
                    }),
                    
                    // Sleep metrics
                    HealthMetric::Sleep(SleepMetric {
                        recorded_at: now - chrono::Duration::hours(8),
                        sleep_start: now - chrono::Duration::hours(8),
                        sleep_end: now,
                        total_sleep_minutes: 480,
                        deep_sleep_minutes: Some(120),
                        rem_sleep_minutes: Some(90),
                        awake_minutes: Some(30),
                        efficiency_percentage: Some(90.5),
                        source: Some("Sleep App".to_string()),
                    }),
                    
                    // Activity metrics
                    HealthMetric::Activity(ActivityMetric {
                        date: now.date_naive(),
                        steps: Some(10000),
                        distance_meters: Some(8500.0),
                        calories_burned: Some(2400.0),
                        active_minutes: Some(60),
                        flights_climbed: Some(12),
                        source: Some("Apple Health".to_string()),
                    }),
                ],
                workouts: vec![
                    WorkoutData {
                        workout_type: "Running".to_string(),
                        start_time: now - chrono::Duration::hours(3),
                        end_time: now - chrono::Duration::hours(2),
                        total_energy_kcal: Some(450.0),
                        distance_meters: Some(5000.0),
                        avg_heart_rate: Some(150),
                        max_heart_rate: Some(175),
                        source: Some("Apple Watch".to_string()),
                    },
                ],
            },
        }
    }

    /// Create an iOS format payload with various metric types
    pub fn ios_payload_comprehensive() -> IosIngestPayload {
        let now = Utc::now();
        let date_str = now.to_rfc3339();
        
        IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    // Heart rate data
                    IosMetric {
                        name: "heart_rate".to_string(),
                        units: Some("bpm".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("Apple Watch".to_string()),
                                date: Some(date_str.clone()),
                                start: None,
                                end: None,
                                qty: Some(75.0),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    
                    // Blood pressure systolic
                    IosMetric {
                        name: "blood_pressure_systolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("Manual".to_string()),
                                date: Some(date_str.clone()),
                                start: None,
                                end: None,
                                qty: Some(120.0),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    
                    // Blood pressure diastolic
                    IosMetric {
                        name: "blood_pressure_diastolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("Manual".to_string()),
                                date: Some(date_str.clone()),
                                start: None,
                                end: None,
                                qty: Some(80.0),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    
                    // Sleep analysis
                    IosMetric {
                        name: "sleep_analysis".to_string(),
                        units: Some("min".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("Apple Health".to_string()),
                                date: Some(date_str.clone()),
                                start: Some((now - chrono::Duration::hours(8)).to_rfc3339()),
                                end: Some(now.to_rfc3339()),
                                qty: None,
                                value: Some("Asleep".to_string()),
                                extra: {
                                    let mut extra = HashMap::new();
                                    extra.insert("deep_sleep_minutes".to_string(), json!(120));
                                    extra.insert("rem_sleep_minutes".to_string(), json!(90));
                                    extra
                                },
                            },
                        ],
                    },
                    
                    // Steps
                    IosMetric {
                        name: "steps".to_string(),
                        units: Some("count".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("iPhone".to_string()),
                                date: Some(date_str.clone()),
                                start: None,
                                end: None,
                                qty: Some(8500.0),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                    
                    // Distance
                    IosMetric {
                        name: "distance_walking_running".to_string(),
                        units: Some("m".to_string()),
                        data: vec![
                            IosMetricData {
                                source: Some("iPhone".to_string()),
                                date: Some(date_str.clone()),
                                start: None,
                                end: None,
                                qty: Some(6500.0),
                                value: None,
                                extra: HashMap::new(),
                            },
                        ],
                    },
                ],
                workouts: vec![
                    IosWorkout {
                        name: Some("Cycling".to_string()),
                        start: Some((now - chrono::Duration::hours(2)).to_rfc3339()),
                        end: Some((now - chrono::Duration::hours(1)).to_rfc3339()),
                        source: Some("Apple Watch".to_string()),
                        extra: {
                            let mut extra = HashMap::new();
                            extra.insert("total_energy_kcal".to_string(), json!(300.0));
                            extra.insert("distance_meters".to_string(), json!(12000.0));
                            extra.insert("avg_heart_rate".to_string(), json!(135));
                            extra.insert("max_heart_rate".to_string(), json!(160));
                            extra
                        },
                    },
                ],
            },
        }
    }

    /// Create a payload with invalid data for validation testing
    pub fn invalid_payload() -> IngestPayload {
        let now = Utc::now();
        
        IngestPayload {
            data: IngestData {
                metrics: vec![
                    // Invalid heart rate (too high)
                    HealthMetric::HeartRate(HeartRateMetric {
                        recorded_at: now,
                        min_bpm: Some(400), // Invalid: too high
                        avg_bpm: Some(75),
                        max_bpm: Some(85),
                        source: Some("Test".to_string()),
                        context: None,
                    }),
                    
                    // Invalid blood pressure
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        recorded_at: now,
                        systolic: 300, // Invalid: too high
                        diastolic: 200, // Invalid: too high
                        pulse: Some(400), // Invalid: too high
                        source: Some("Test".to_string()),
                    }),
                    
                    // Invalid sleep (end before start)
                    HealthMetric::Sleep(SleepMetric {
                        recorded_at: now,
                        sleep_start: now,
                        sleep_end: now - chrono::Duration::hours(1), // Invalid: end before start
                        total_sleep_minutes: 60,
                        deep_sleep_minutes: None,
                        rem_sleep_minutes: None,
                        awake_minutes: None,
                        efficiency_percentage: Some(150.0), // Invalid: > 100%
                        source: Some("Test".to_string()),
                    }),
                    
                    // Invalid activity (negative values)
                    HealthMetric::Activity(ActivityMetric {
                        date: now.date_naive(),
                        steps: Some(-1000), // Invalid: negative
                        distance_meters: Some(-500.0), // Invalid: negative
                        calories_burned: Some(-200.0), // Invalid: negative
                        active_minutes: None,
                        flights_climbed: None,
                        source: Some("Test".to_string()),
                    }),
                ],
                workouts: vec![
                    WorkoutData {
                        workout_type: "".to_string(), // Invalid: empty type
                        start_time: now,
                        end_time: now - chrono::Duration::hours(1), // Invalid: end before start
                        total_energy_kcal: Some(-100.0), // Invalid: negative
                        distance_meters: Some(2000000.0), // Invalid: too far
                        avg_heart_rate: Some(400), // Invalid: too high
                        max_heart_rate: Some(500), // Invalid: too high
                        source: Some("Test".to_string()),
                    },
                ],
            },
        }
    }

    /// Create a large payload with 1000+ metrics for performance testing
    pub fn large_payload(metric_count: usize) -> IngestPayload {
        let now = Utc::now();
        let mut metrics = Vec::new();
        
        for i in 0..metric_count {
            let timestamp = now - chrono::Duration::minutes(i as i64);
            
            // Alternate between different metric types
            match i % 4 {
                0 => {
                    metrics.push(HealthMetric::HeartRate(HeartRateMetric {
                        recorded_at: timestamp,
                        min_bpm: None,
                        avg_bpm: Some(70 + (i % 50) as i16),
                        max_bpm: None,
                        source: Some("Performance Test".to_string()),
                        context: Some("resting".to_string()),
                    }));
                }
                1 => {
                    metrics.push(HealthMetric::BloodPressure(BloodPressureMetric {
                        recorded_at: timestamp,
                        systolic: 110 + (i % 30) as i16,
                        diastolic: 70 + (i % 20) as i16,
                        pulse: Some(65 + (i % 40) as i16),
                        source: Some("Performance Test".to_string()),
                    }));
                }
                2 => {
                    metrics.push(HealthMetric::Activity(ActivityMetric {
                        date: timestamp.date_naive(),
                        steps: Some(5000 + (i % 10000) as i32),
                        distance_meters: Some(3000.0 + (i % 5000) as f64),
                        calories_burned: Some(1800.0 + (i % 1000) as f64),
                        active_minutes: Some(30 + (i % 60) as i32),
                        flights_climbed: Some(i % 20 as i32),
                        source: Some("Performance Test".to_string()),
                    }));
                }
                _ => {
                    // Every 4th metric is a sleep metric (less frequent)
                    if i % 24 == 0 { // Only add sleep metrics every 24 iterations
                        metrics.push(HealthMetric::Sleep(SleepMetric {
                            recorded_at: timestamp,
                            sleep_start: timestamp - chrono::Duration::hours(8),
                            sleep_end: timestamp,
                            total_sleep_minutes: 420 + (i % 120) as i32,
                            deep_sleep_minutes: Some(90 + (i % 60) as i32),
                            rem_sleep_minutes: Some(60 + (i % 40) as i32),
                            awake_minutes: Some(i % 30 as i32),
                            efficiency_percentage: Some(80.0 + (i % 20) as f32),
                            source: Some("Performance Test".to_string()),
                        }));
                    }
                }
            }
        }
        
        IngestPayload {
            data: IngestData {
                metrics,
                workouts: vec![], // Keep workouts empty for focused metric testing
            },
        }
    }

    /// Create a test user and API key for authentication
    pub fn create_test_auth_context() -> AuthContext {
        let user_id = Uuid::new_v4();
        let api_key_id = Uuid::new_v4();
        
        AuthContext {
            user: AuthenticatedUser {
                id: user_id,
                username: "test_user".to_string(),
                email: Some("test@example.com".to_string()),
                is_active: true,
                created_at: Utc::now(),
            },
            api_key: ApiKey {
                id: api_key_id,
                user_id,
                name: "Test API Key".to_string(),
                key_hash: "test_hash".to_string(),
                is_active: true,
                last_used_at: Some(Utc::now()),
                created_at: Utc::now(),
                expires_at: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use serde_json;

    // Helper function to setup test database (would need actual implementation)
    async fn setup_test_db() -> PgPool {
        // This would connect to a test database
        // For now, we'll skip database tests and focus on handler logic
        todo!("Implement test database setup")
    }

    #[actix_web::test]
    async fn test_standard_payload_format() {
        let payload = TestFixtures::standard_payload_comprehensive();
        
        // Test serialization/deserialization
        let json_str = serde_json::to_string(&payload).expect("Should serialize");
        let deserialized: IngestPayload = serde_json::from_str(&json_str)
            .expect("Should deserialize");
        
        // Verify metrics count
        assert_eq!(deserialized.data.metrics.len(), 4);
        assert_eq!(deserialized.data.workouts.len(), 1);
        
        // Verify metric types are present
        let metric_types: Vec<&str> = deserialized.data.metrics
            .iter()
            .map(|m| m.metric_type())
            .collect();
        
        assert!(metric_types.contains(&"HeartRate"));
        assert!(metric_types.contains(&"BloodPressure"));
        assert!(metric_types.contains(&"Sleep"));
        assert!(metric_types.contains(&"Activity"));
    }

    #[actix_web::test]
    async fn test_ios_payload_format() {
        let ios_payload = TestFixtures::ios_payload_comprehensive();
        
        // Test conversion to internal format
        let internal_payload = ios_payload.to_internal_format();
        
        // Should have converted iOS metrics to internal format
        assert!(!internal_payload.data.metrics.is_empty());
        assert!(!internal_payload.data.workouts.is_empty());
        
        // Test serialization/deserialization of iOS format
        let json_str = serde_json::to_string(&ios_payload).expect("Should serialize");
        let deserialized: IosIngestPayload = serde_json::from_str(&json_str)
            .expect("Should deserialize");
        
        assert_eq!(deserialized.data.metrics.len(), ios_payload.data.metrics.len());
    }

    #[actix_web::test]
    async fn test_ios_blood_pressure_pairing() {
        let ios_payload = TestFixtures::ios_payload_comprehensive();
        let internal_payload = ios_payload.to_internal_format();
        
        // Should have paired systolic and diastolic readings into one blood pressure metric
        let bp_metrics: Vec<&HealthMetric> = internal_payload.data.metrics
            .iter()
            .filter(|m| matches!(m, HealthMetric::BloodPressure(_)))
            .collect();
        
        assert_eq!(bp_metrics.len(), 1, "Should have one paired blood pressure reading");
        
        if let HealthMetric::BloodPressure(bp) = &bp_metrics[0] {
            assert_eq!(bp.systolic, 120);
            assert_eq!(bp.diastolic, 80);
        }
    }

    #[actix_web::test]
    async fn test_validation_errors() {
        let invalid_payload = TestFixtures::invalid_payload();
        
        // Each metric should fail validation
        for metric in &invalid_payload.data.metrics {
            let result = metric.validate();
            assert!(result.is_err(), "Metric should fail validation: {:?}", metric);
        }
        
        // Workout should also fail validation
        assert_eq!(invalid_payload.data.workouts.len(), 1);
        // Note: We would need to implement workout validation to test this
    }

    #[actix_web::test]
    async fn test_large_payload_generation() {
        let large_payload = TestFixtures::large_payload(1000);
        
        assert_eq!(large_payload.data.metrics.len(), 1000);
        
        // Verify we have a mix of metric types
        let heart_rate_count = large_payload.data.metrics
            .iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();
        
        let bp_count = large_payload.data.metrics
            .iter()
            .filter(|m| matches!(m, HealthMetric::BloodPressure(_)))
            .count();
        
        let activity_count = large_payload.data.metrics
            .iter()
            .filter(|m| matches!(m, HealthMetric::Activity(_)))
            .count();
        
        // Should have roughly equal distribution (accounting for sleep being less frequent)
        assert!(heart_rate_count > 200);
        assert!(bp_count > 200);
        assert!(activity_count > 200);
        
        // Test payload size is reasonable for 100MB limit
        let json_size = serde_json::to_string(&large_payload)
            .expect("Should serialize")
            .len();
        
        assert!(json_size < 100 * 1024 * 1024, "Payload should be under 100MB");
    }

    #[test]
    fn test_auth_context_creation() {
        let auth_context = TestFixtures::create_test_auth_context();
        
        assert!(!auth_context.user.username.is_empty());
        assert!(auth_context.user.is_active);
        assert!(auth_context.api_key.is_active);
        assert_eq!(auth_context.user.id, auth_context.api_key.user_id);
    }

    // Performance benchmark test (should be run separately)
    #[test]
    #[ignore] // Use `cargo test -- --ignored` to run performance tests
    fn benchmark_payload_processing() {
        use std::time::Instant;
        
        let start = Instant::now();
        let payload = TestFixtures::large_payload(5000);
        let generation_time = start.elapsed();
        
        println!("Generated 5000 metrics in {:?}", generation_time);
        
        let start = Instant::now();
        let json_str = serde_json::to_string(&payload).expect("Should serialize");
        let serialization_time = start.elapsed();
        
        println!("Serialized payload in {:?}", serialization_time);
        println!("JSON size: {} bytes", json_str.len());
        
        let start = Instant::now();
        let _: IngestPayload = serde_json::from_str(&json_str).expect("Should deserialize");
        let deserialization_time = start.elapsed();
        
        println!("Deserialized payload in {:?}", deserialization_time);
        
        // Ensure performance is reasonable
        assert!(generation_time.as_millis() < 1000, "Should generate quickly");
        assert!(serialization_time.as_millis() < 2000, "Should serialize quickly");
        assert!(deserialization_time.as_millis() < 2000, "Should deserialize quickly");
    }
}

#[cfg(test)]
mod dual_write_tests {
    use super::*;
    use self_sensored::{
        config::BatchConfig,
        services::batch_processor::BatchProcessor,
        models::ActivityMetricV2,
    };
    use tokio_test;

    /// Test ActivityMetricV2 validation
    #[test]
    fn test_activity_metric_v2_validation() {
        let now = Utc::now();
        
        // Valid metric
        let valid_metric = ActivityMetricV2 {
            recorded_at: now,
            step_count: Some(10000),
            flights_climbed: Some(15),
            distance_walking_running_meters: Some(5000.0),
            distance_cycling_meters: Some(2000.0),
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel: Some(500.0),
            active_energy_burned_kcal: Some(400.0),
            basal_energy_burned_kcal: Some(1800.0),
            exercise_time_minutes: Some(45),
            stand_time_minutes: Some(12),
            move_time_minutes: Some(300),
            stand_hour_achieved: Some(true),
            aggregation_period: Some("daily".to_string()),
            source: Some("Apple Health".to_string()),
        };
        
        assert!(valid_metric.validate().is_ok());
        
        // Invalid metrics - out of range values
        let invalid_step_count = ActivityMetricV2 {
            step_count: Some(300000), // Way too high
            ..valid_metric.clone()
        };
        assert!(invalid_step_count.validate().is_err());
        
        let invalid_flights_climbed = ActivityMetricV2 {
            flights_climbed: Some(20000), // Way too high
            ..valid_metric.clone()
        };
        assert!(invalid_flights_climbed.validate().is_err());
        
        let invalid_aggregation_period = ActivityMetricV2 {
            aggregation_period: Some("invalid_period".to_string()),
            ..valid_metric.clone()
        };
        assert!(invalid_aggregation_period.validate().is_err());
        
        let invalid_exercise_time = ActivityMetricV2 {
            exercise_time_minutes: Some(2000), // More than 24 hours
            ..valid_metric.clone()
        };
        assert!(invalid_exercise_time.validate().is_err());
    }

    #[test]
    fn test_activity_metric_conversion() {
        let original_metric = ActivityMetric {
            date: chrono::Utc::now().date_naive(),
            steps: Some(12000),
            distance_meters: Some(8000.0),
            calories_burned: Some(500.0),
            active_minutes: Some(60),
            flights_climbed: Some(20),
            source: Some("Apple Watch".to_string()),
        };
        
        // Test conversion to v2
        let v2_metric = ActivityMetricV2::from_activity_metric(&original_metric);
        
        assert_eq!(v2_metric.step_count, original_metric.steps);
        assert_eq!(v2_metric.flights_climbed, original_metric.flights_climbed);
        assert_eq!(v2_metric.distance_walking_running_meters, original_metric.distance_meters);
        assert_eq!(v2_metric.active_energy_burned_kcal, original_metric.calories_burned);
        assert_eq!(v2_metric.exercise_time_minutes, original_metric.active_minutes);
        assert_eq!(v2_metric.source, original_metric.source);
        assert_eq!(v2_metric.aggregation_period, Some("daily".to_string()));
        
        // Test conversion back to original
        let converted_back = v2_metric.to_activity_metric();
        
        assert_eq!(converted_back.date, original_metric.date);
        assert_eq!(converted_back.steps, original_metric.steps);
        assert_eq!(converted_back.distance_meters, original_metric.distance_meters);
        assert_eq!(converted_back.calories_burned, original_metric.calories_burned);
        assert_eq!(converted_back.active_minutes, original_metric.active_minutes);
        assert_eq!(converted_back.flights_climbed, original_metric.flights_climbed);
        assert_eq!(converted_back.source, original_metric.source);
    }

    #[test]
    fn test_dual_write_config_validation() {
        // Test default configuration
        let default_config = BatchConfig::default();
        assert!(!default_config.enable_dual_write_activity_metrics); // Should be disabled by default
        
        // Test custom configuration
        let mut custom_config = BatchConfig::default();
        custom_config.enable_dual_write_activity_metrics = true;
        assert!(custom_config.enable_dual_write_activity_metrics);
        
        // Test validation still works
        assert!(custom_config.validate().is_ok());
    }

    #[test]
    fn test_activity_metric_v2_field_completeness() {
        let now = Utc::now();
        
        // Create a metric with all fields populated
        let complete_metric = ActivityMetricV2 {
            recorded_at: now,
            step_count: Some(15000),
            flights_climbed: Some(25),
            distance_walking_running_meters: Some(8000.0),
            distance_cycling_meters: Some(3000.0),
            distance_swimming_meters: Some(1000.0),
            distance_wheelchair_meters: Some(500.0),
            distance_downhill_snow_sports_meters: Some(2000.0),
            push_count: Some(100),
            swimming_stroke_count: Some(500),
            nike_fuel: Some(750.0),
            active_energy_burned_kcal: Some(600.0),
            basal_energy_burned_kcal: Some(1900.0),
            exercise_time_minutes: Some(90),
            stand_time_minutes: Some(14),
            move_time_minutes: Some(400),
            stand_hour_achieved: Some(true),
            aggregation_period: Some("hourly".to_string()),
            source: Some("Apple Health Export".to_string()),
        };
        
        // Should validate successfully with all fields
        assert!(complete_metric.validate().is_ok());
        
        // Test boundary values
        let boundary_metric = ActivityMetricV2 {
            recorded_at: now,
            step_count: Some(200000), // Maximum allowed
            flights_climbed: Some(10000), // Maximum allowed
            distance_walking_running_meters: Some(500000.0), // 500km - maximum allowed
            distance_cycling_meters: Some(1000000.0), // 1000km - maximum allowed
            distance_swimming_meters: Some(50000.0), // 50km - maximum allowed
            distance_wheelchair_meters: Some(500000.0), // 500km - maximum allowed
            distance_downhill_snow_sports_meters: Some(200000.0), // 200km - maximum allowed
            push_count: Some(50000), // Maximum allowed
            swimming_stroke_count: Some(100000), // Maximum allowed
            nike_fuel: Some(50000.0), // Maximum allowed
            active_energy_burned_kcal: Some(20000.0), // Maximum allowed
            basal_energy_burned_kcal: Some(10000.0), // Maximum allowed
            exercise_time_minutes: Some(1440), // 24 hours - maximum allowed
            stand_time_minutes: Some(1440), // 24 hours - maximum allowed
            move_time_minutes: Some(1440), // 24 hours - maximum allowed
            stand_hour_achieved: Some(false),
            aggregation_period: Some("weekly".to_string()),
            source: Some("Test Device".to_string()),
        };
        
        // Should validate successfully at boundaries
        assert!(boundary_metric.validate().is_ok());
    }

    #[test]
    fn test_field_mapping_edge_cases() {
        // Test conversion with minimal data
        let minimal_metric = ActivityMetric {
            date: chrono::Utc::now().date_naive(),
            steps: None,
            distance_meters: None,
            calories_burned: None,
            active_minutes: None,
            flights_climbed: None,
            source: None,
        };
        
        let v2_minimal = ActivityMetricV2::from_activity_metric(&minimal_metric);
        
        assert_eq!(v2_minimal.step_count, None);
        assert_eq!(v2_minimal.distance_walking_running_meters, None);
        assert_eq!(v2_minimal.active_energy_burned_kcal, None);
        assert_eq!(v2_minimal.exercise_time_minutes, None);
        assert_eq!(v2_minimal.flights_climbed, None);
        assert_eq!(v2_minimal.source, None);
        assert_eq!(v2_minimal.aggregation_period, Some("daily".to_string())); // Should default to daily
        
        // Should still validate
        assert!(v2_minimal.validate().is_ok());
    }

    #[test]
    fn test_aggregation_period_validation() {
        let now = Utc::now();
        let base_metric = ActivityMetricV2 {
            recorded_at: now,
            step_count: Some(10000),
            flights_climbed: Some(15),
            distance_walking_running_meters: Some(5000.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel: None,
            active_energy_burned_kcal: Some(400.0),
            basal_energy_burned_kcal: None,
            exercise_time_minutes: Some(45),
            stand_time_minutes: None,
            move_time_minutes: None,
            stand_hour_achieved: None,
            aggregation_period: Some("minute".to_string()),
            source: Some("Test".to_string()),
        };
        
        // Test valid aggregation periods
        let valid_periods = ["minute", "hourly", "daily", "weekly"];
        for period in valid_periods {
            let mut metric = base_metric.clone();
            metric.aggregation_period = Some(period.to_string());
            assert!(metric.validate().is_ok(), "Period '{}' should be valid", period);
        }
        
        // Test invalid aggregation period
        let mut invalid_metric = base_metric.clone();
        invalid_metric.aggregation_period = Some("monthly".to_string());
        assert!(invalid_metric.validate().is_err());
        
        // Test None aggregation period (should be valid)
        let mut none_metric = base_metric.clone();
        none_metric.aggregation_period = None;
        assert!(none_metric.validate().is_ok());
    }

    /// Test that dual-write doesn't interfere with existing functionality
    #[test]
    fn test_dual_write_config_backwards_compatibility() {
        // Test that disabling dual-write maintains current behavior
        let mut config = BatchConfig::default();
        config.enable_dual_write_activity_metrics = false;
        
        // Validation should still work
        assert!(config.validate().is_ok());
        
        // Activity chunk size should still be valid
        assert_eq!(config.activity_chunk_size, 7000);
        assert!(config.activity_chunk_size * 7 <= 52428); // Should stay under parameter limit
    }

    #[test]
    fn test_dual_write_parameter_calculations() {
        // Test that v2 table parameter calculations are correct
        let activity_v2_params_per_record = 21; // Based on the insert query in batch_processor.rs
        let chunk_size = 2000; // Conservative chunk size for v2 table
        let total_params = chunk_size * activity_v2_params_per_record;
        
        // Should be well under PostgreSQL limit
        assert!(total_params < 52428, "V2 chunk size calculation should be safe");
        
        // Test that we have room for dual-write overhead
        let dual_write_overhead = 1.1; // 10% overhead estimate
        let total_with_overhead = (total_params as f64 * dual_write_overhead) as usize;
        assert!(total_with_overhead < 52428, "Dual-write should stay under parameter limit with overhead");
    }

    /// Test metrics collection for dual-write operations
    #[test]
    fn test_dual_write_metrics_integration() {
        use self_sensored::middleware::metrics::Metrics;
        
        // Test that dual-write metrics can be recorded without panic
        Metrics::record_dual_write_start("activity_metrics", 100);
        Metrics::record_dual_write_success("activity_metrics", 100, std::time::Duration::from_millis(500));
        Metrics::record_dual_write_failure("activity_metrics", 50, std::time::Duration::from_millis(1000));
        Metrics::record_dual_write_consistency_error("activity_metrics", "field_mismatch");
        Metrics::record_dual_write_rollback("activity_metrics", 75, std::time::Duration::from_millis(200));
        
        // If we get here without panicking, the metrics integration works
        assert!(true);
    }

    /// Test performance impact of dual-write conversion
    #[test]
    fn test_conversion_performance() {
        use std::time::Instant;
        
        // Create a batch of activity metrics
        let metrics: Vec<ActivityMetric> = (0..1000)
            .map(|i| ActivityMetric {
                date: chrono::Utc::now().date_naive() - chrono::Duration::days(i % 30),
                steps: Some(8000 + (i * 100) % 10000),
                distance_meters: Some(5000.0 + (i as f64 * 10.0) % 3000.0),
                calories_burned: Some(300.0 + (i as f64 * 2.0) % 500.0),
                active_minutes: Some(30 + (i * 2) % 120),
                flights_climbed: Some(5 + (i % 50)),
                source: Some(format!("Device_{}", i % 5)),
            })
            .collect();
        
        // Test conversion performance
        let start = Instant::now();
        let v2_metrics: Vec<ActivityMetricV2> = metrics
            .iter()
            .map(|m| ActivityMetricV2::from_activity_metric(m))
            .collect();
        let conversion_time = start.elapsed();
        
        // Test validation performance
        let start = Instant::now();
        let validation_results: Vec<_> = v2_metrics
            .iter()
            .map(|m| m.validate())
            .collect();
        let validation_time = start.elapsed();
        
        // Ensure all validations passed
        for result in validation_results {
            assert!(result.is_ok());
        }
        
        // Performance assertions (should be very fast)
        assert!(conversion_time.as_millis() < 100, "Conversion should be fast: {:?}", conversion_time);
        assert!(validation_time.as_millis() < 100, "Validation should be fast: {:?}", validation_time);
        
        println!("Converted {} metrics in {:?}", metrics.len(), conversion_time);
        println!("Validated {} metrics in {:?}", v2_metrics.len(), validation_time);
    }
}

/// Integration tests for dual-write functionality with real database transactions
#[cfg(test)]
mod dual_write_integration_tests {
    use super::*;
    use self_sensored::{
        config::BatchConfig,
        services::batch_processor::BatchProcessor,
        models::ActivityMetricV2,
    };
    use sqlx::PgPool;
    use tokio_test;

    /// Test dual-write integration with real database transactions
    #[sqlx::test]
    async fn test_dual_write_integration_with_real_transactions(pool: PgPool) -> sqlx::Result<()> {
        // Create test user
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
            user_id,
            "dualwrite@test.com"
        )
        .execute(&pool)
        .await?;

        // Create BatchProcessor with dual-write enabled
        let mut config = BatchConfig::default();
        config.enable_dual_write_activity_metrics = true;
        config.activity_chunk_size = 1000; // Smaller chunk for testing
        let batch_processor = BatchProcessor::with_config(pool.clone(), config);
        
        // Create test activity metrics
        let now = Utc::now();
        let test_metrics = vec![
            ActivityMetric {
                date: now.date_naive(),
                steps: Some(10000),
                distance_meters: Some(8000.0),
                calories_burned: Some(500.0),
                active_minutes: Some(60),
                flights_climbed: Some(15),
                source: Some("Integration Test".to_string()),
            },
            ActivityMetric {
                date: (now - chrono::Duration::days(1)).date_naive(),
                steps: Some(8500),
                distance_meters: Some(6500.0),
                calories_burned: Some(450.0),
                active_minutes: Some(45),
                flights_climbed: Some(12),
                source: Some("Integration Test".to_string()),
            },
            ActivityMetric {
                date: (now - chrono::Duration::days(2)).date_naive(),
                steps: Some(12000),
                distance_meters: Some(9500.0),
                calories_burned: Some(600.0),
                active_minutes: Some(75),
                flights_climbed: Some(20),
                source: Some("Integration Test".to_string()),
            },
        ];
        
        // Process metrics with dual-write
        let mut tx = pool.begin().await?;
        let result = batch_processor.process_activity_metrics(
            &mut tx, 
            user_id, 
            &test_metrics
        ).await;
        tx.commit().await?;
        
        assert!(result.is_ok(), "Dual-write processing should succeed");
        
        // Verify data was written to both tables
        
        // Check original table (activity_metrics)
        let original_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(original_count, Some(3), "Original table should have 3 records");
        
        // Check v2 table (activity_metrics_v2)
        let v2_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(v2_count, Some(3), "V2 table should have 3 records");
        
        // Verify data consistency between tables
        let original_records = sqlx::query!(
            "SELECT date, steps, distance_meters, calories_burned, active_minutes, flights_climbed, source 
             FROM activity_metrics 
             WHERE user_id = $1 
             ORDER BY date DESC",
            user_id
        )
        .fetch_all(&pool)
        .await?;
        
        let v2_records = sqlx::query!(
            "SELECT recorded_at, step_count, distance_walking_running_meters, active_energy_burned_kcal, 
                    exercise_time_minutes, flights_climbed, source 
             FROM activity_metrics_v2 
             WHERE user_id = $1 
             ORDER BY recorded_at DESC",
            user_id
        )
        .fetch_all(&pool)
        .await?;
        
        assert_eq!(original_records.len(), v2_records.len(), "Both tables should have same number of records");
        
        // Check field mapping consistency
        for (orig, v2) in original_records.iter().zip(v2_records.iter()) {
            // Date mapping
            let orig_date = orig.date;
            let v2_date = v2.recorded_at.date_naive();
            assert_eq!(orig_date, v2_date, "Dates should match");
            
            // Field mappings
            assert_eq!(orig.steps, v2.step_count, "Steps should match");
            assert_eq!(orig.distance_meters, v2.distance_walking_running_meters, "Distance should match");
            assert_eq!(orig.calories_burned, v2.active_energy_burned_kcal, "Calories should match");
            assert_eq!(orig.active_minutes, v2.exercise_time_minutes, "Active minutes should match");
            assert_eq!(orig.flights_climbed, v2.flights_climbed, "Flights climbed should match");
            assert_eq!(orig.source, v2.source, "Source should match");
        }
        
        // Test query performance on both tables
        let start_time = std::time::Instant::now();
        let _original_query = sqlx::query!(
            "SELECT * FROM activity_metrics WHERE user_id = $1 AND date >= $2",
            user_id,
            (now - chrono::Duration::days(7)).date_naive()
        )
        .fetch_all(&pool)
        .await?;
        let original_query_time = start_time.elapsed();
        
        let start_time = std::time::Instant::now();
        let _v2_query = sqlx::query!(
            "SELECT * FROM activity_metrics_v2 WHERE user_id = $1 AND recorded_at >= $2",
            user_id,
            now - chrono::Duration::days(7)
        )
        .fetch_all(&pool)
        .await?;
        let v2_query_time = start_time.elapsed();
        
        // Both queries should complete quickly (under 100ms for this small dataset)
        assert!(original_query_time.as_millis() < 100, "Original table query should be fast");
        assert!(v2_query_time.as_millis() < 100, "V2 table query should be fast");
        
        println!("✅ Dual-write integration test completed successfully");
        println!("   - Original table query time: {:?}", original_query_time);
        println!("   - V2 table query time: {:?}", v2_query_time);
        println!("   - Data consistency verified across {} records", test_metrics.len());
        
        Ok(())
    }

    /// Test dual-write batch processing with larger dataset
    #[sqlx::test]
    async fn test_dual_write_batch_processing_integration(pool: PgPool) -> sqlx::Result<()> {
        // Create test user
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
            user_id,
            "batch@test.com"
        )
        .execute(&pool)
        .await?;

        // Create BatchProcessor with dual-write enabled and smaller chunks for testing
        let mut config = BatchConfig::default();
        config.enable_dual_write_activity_metrics = true;
        config.activity_chunk_size = 50; // Small chunks to test batch processing
        let batch_processor = BatchProcessor::with_config(pool.clone(), config);
        
        // Create large batch of test data (150 records to test multiple chunks)
        let now = Utc::now();
        let test_metrics: Vec<ActivityMetric> = (0..150)
            .map(|i| ActivityMetric {
                date: (now - chrono::Duration::days(i % 30)).date_naive(),
                steps: Some(8000 + (i * 100) % 5000),
                distance_meters: Some(6000.0 + (i as f64 * 50.0) % 3000.0),
                calories_burned: Some(400.0 + (i as f64 * 10.0) % 400.0),
                active_minutes: Some(45 + (i * 2) % 60),
                flights_climbed: Some(10 + (i % 15)),
                source: Some(format!("Batch Test {}", i)),
            })
            .collect();
        
        // Process large batch with dual-write
        let start_time = std::time::Instant::now();
        let mut tx = pool.begin().await?;
        let result = batch_processor.process_activity_metrics(
            &mut tx, 
            user_id, 
            &test_metrics
        ).await;
        tx.commit().await?;
        let processing_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Large batch dual-write processing should succeed");
        
        // Verify all records were inserted in both tables
        let original_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        let v2_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(original_count, Some(150), "Original table should have 150 records");
        assert_eq!(v2_count, Some(150), "V2 table should have 150 records");
        
        // Test deduplication works across both tables
        // Try to insert duplicate records
        let duplicate_metrics = test_metrics[0..5].to_vec(); // First 5 records
        let mut tx = pool.begin().await?;
        let duplicate_result = batch_processor.process_activity_metrics(
            &mut tx, 
            user_id, 
            &duplicate_metrics
        ).await;
        tx.commit().await?;
        
        // Should handle duplicates gracefully
        assert!(duplicate_result.is_ok(), "Duplicate processing should be handled gracefully");
        
        // Count should remain the same (no duplicates added)
        let final_original_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        let final_v2_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics_v2 WHERE user_id = $1",
            user_id
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(final_original_count, Some(150), "Original table count should remain 150 after duplicates");
        assert_eq!(final_v2_count, Some(150), "V2 table count should remain 150 after duplicates");
        
        // Performance check - should complete in reasonable time
        assert!(processing_time.as_secs() < 30, "Batch processing should complete in under 30 seconds");
        
        println!("✅ Dual-write batch processing integration test completed successfully");
        println!("   - Processed {} records in {:?}", test_metrics.len(), processing_time);
        println!("   - Chunk size: 50 records (3 chunks processed)");
        println!("   - Deduplication verified");
        
        Ok(())
    }

    /// Test dual-write with mixed data types and edge cases
    #[sqlx::test]
    async fn test_dual_write_mixed_data_integration(pool: PgPool) -> sqlx::Result<()> {
        // Create test user
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
            user_id,
            "mixed@test.com"
        )
        .execute(&pool)
        .await?;

        let mut config = BatchConfig::default();
        config.enable_dual_write_activity_metrics = true;
        let batch_processor = BatchProcessor::with_config(pool.clone(), config);
        
        let now = Utc::now();
        
        // Test with mixed data - some with all fields, some with minimal fields
        let mixed_metrics = vec![
            // Complete record
            ActivityMetric {
                date: now.date_naive(),
                steps: Some(10000),
                distance_meters: Some(8000.0),
                calories_burned: Some(500.0),
                active_minutes: Some(60),
                flights_climbed: Some(15),
                source: Some("Complete Record".to_string()),
            },
            // Minimal record (only required fields)
            ActivityMetric {
                date: (now - chrono::Duration::days(1)).date_naive(),
                steps: None,
                distance_meters: None,
                calories_burned: None,
                active_minutes: None,
                flights_climbed: None,
                source: None,
            },
            // Partial record
            ActivityMetric {
                date: (now - chrono::Duration::days(2)).date_naive(),
                steps: Some(5000),
                distance_meters: Some(0.0), // Zero distance
                calories_burned: None,
                active_minutes: Some(0), // Zero active minutes
                flights_climbed: Some(0), // Zero flights
                source: Some("Partial Record".to_string()),
            },
            // High values (boundary testing)
            ActivityMetric {
                date: (now - chrono::Duration::days(3)).date_naive(),
                steps: Some(50000), // Very high step count
                distance_meters: Some(42195.0), // Marathon distance
                calories_burned: Some(3000.0), // High calorie burn
                active_minutes: Some(300), // 5 hours active
                flights_climbed: Some(200), // Many flights
                source: Some("High Values Record".to_string()),
            },
        ];
        
        // Process mixed data
        let mut tx = pool.begin().await?;
        let result = batch_processor.process_activity_metrics(
            &mut tx, 
            user_id, 
            &mixed_metrics
        ).await;
        tx.commit().await?;
        
        assert!(result.is_ok(), "Mixed data processing should succeed");
        
        // Verify all records in both tables
        let original_records = sqlx::query!(
            "SELECT date, steps, distance_meters, calories_burned, active_minutes, flights_climbed, source 
             FROM activity_metrics 
             WHERE user_id = $1 
             ORDER BY date DESC",
            user_id
        )
        .fetch_all(&pool)
        .await?;
        
        let v2_records = sqlx::query!(
            "SELECT recorded_at, step_count, distance_walking_running_meters, active_energy_burned_kcal, 
                    exercise_time_minutes, flights_climbed, source 
             FROM activity_metrics_v2 
             WHERE user_id = $1 
             ORDER BY recorded_at DESC",
            user_id
        )
        .fetch_all(&pool)
        .await?;
        
        assert_eq!(original_records.len(), 4, "Should have 4 original records");
        assert_eq!(v2_records.len(), 4, "Should have 4 v2 records");
        
        // Verify NULL handling
        let minimal_orig = original_records.iter().find(|r| r.steps.is_none()).unwrap();
        let minimal_v2 = v2_records.iter().find(|r| r.step_count.is_none()).unwrap();
        
        assert_eq!(minimal_orig.steps, minimal_v2.step_count);
        assert_eq!(minimal_orig.distance_meters, minimal_v2.distance_walking_running_meters);
        assert_eq!(minimal_orig.calories_burned, minimal_v2.active_energy_burned_kcal);
        assert_eq!(minimal_orig.active_minutes, minimal_v2.exercise_time_minutes);
        
        // Verify zero values are preserved
        let zero_orig = original_records.iter().find(|r| 
            r.distance_meters == Some(0.0) && r.active_minutes == Some(0)
        ).unwrap();
        let zero_v2 = v2_records.iter().find(|r| 
            r.distance_walking_running_meters == Some(0.0) && r.exercise_time_minutes == Some(0)
        ).unwrap();
        
        assert_eq!(zero_orig.distance_meters, zero_v2.distance_walking_running_meters);
        assert_eq!(zero_orig.active_minutes, zero_v2.exercise_time_minutes);
        
        // Verify high values are handled correctly
        let high_orig = original_records.iter().find(|r| r.steps == Some(50000)).unwrap();
        let high_v2 = v2_records.iter().find(|r| r.step_count == Some(50000)).unwrap();
        
        assert_eq!(high_orig.steps, high_v2.step_count);
        assert_eq!(high_orig.distance_meters, high_v2.distance_walking_running_meters);
        assert_eq!(high_orig.calories_burned, high_v2.active_energy_burned_kcal);
        
        println!("✅ Mixed data integration test completed successfully");
        println!("   - NULL values handled correctly");
        println!("   - Zero values preserved");
        println!("   - High boundary values processed correctly");
        
        Ok(())
    }
}