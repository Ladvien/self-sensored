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
        IosIngestPayload, IosIngestData, IosMetric, IosMetricData, IosWorkout,
        enums::{ActivityContext, WorkoutType}
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
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now,
                        heart_rate: Some(75),
                        resting_heart_rate: Some(65),
                        heart_rate_variability: None,
                        context: Some(ActivityContext::Resting),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    HealthMetric::HeartRate(HeartRateMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now - chrono::Duration::hours(1),
                        heart_rate: Some(120),
                        resting_heart_rate: None,
                        heart_rate_variability: Some(35.2),
                        context: Some(ActivityContext::Exercise),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
                    }),
                    
                    // Blood pressure metrics
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now - chrono::Duration::hours(2),
                        systolic: 120,
                        diastolic: 80,
                        pulse: Some(70),
                        source_device: Some("Manual Entry".to_string()),
                        created_at: now,
                    }),
                    
                    // Sleep metrics
                    HealthMetric::Sleep(SleepMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now - chrono::Duration::hours(8),
                        sleep_start: now - chrono::Duration::hours(8),
                        sleep_end: now,
                        duration_minutes: 480,
                        deep_sleep_minutes: Some(120),
                        rem_sleep_minutes: Some(90),
                        light_sleep_minutes: Some(240),
                        awake_minutes: Some(30),
                        efficiency: Some(90.5),
                        source_device: Some("Sleep App".to_string()),
                        created_at: now,
                    }),
                    
                    // Activity metrics
                    HealthMetric::Activity(ActivityMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now.date_naive().and_time(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()).and_utc(),
                        step_count: Some(10000),
                        distance_meters: Some(8500.0),
                        active_energy_burned_kcal: Some(500.0),
                        basal_energy_burned_kcal: Some(1900.0),
                        flights_climbed: Some(12),
                        source_device: Some("Apple Health".to_string()),
                        created_at: now,
                    }),
                ],
                workouts: vec![
                    WorkoutData {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        workout_type: WorkoutType::Running,
                        started_at: now - chrono::Duration::hours(3),
                        ended_at: now - chrono::Duration::hours(2),
                        total_energy_kcal: Some(450.0),
                        active_energy_kcal: Some(400.0),
                        distance_meters: Some(5000.0),
                        avg_heart_rate: Some(150),
                        max_heart_rate: Some(175),
                        source_device: Some("Apple Watch".to_string()),
                        created_at: now,
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
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now,
                        heart_rate: 400, // Invalid: too high
                        resting_heart_rate: Some(75),
                        context: None,
                        source_device: Some("Test".to_string()),
                        created_at: now,
                    }),
                    
                    // Invalid blood pressure
                    HealthMetric::BloodPressure(BloodPressureMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now,
                        systolic: 300, // Invalid: too high
                        diastolic: 200, // Invalid: too high
                        pulse: Some(400), // Invalid: too high
                        source_device: Some("Test".to_string()),
                        created_at: now,
                    }),
                    
                    // Invalid sleep (end before start)
                    HealthMetric::Sleep(SleepMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now,
                        sleep_start: now,
                        sleep_end: now - chrono::Duration::hours(1), // Invalid: end before start
                        duration_minutes: 60,
                        deep_sleep_minutes: None,
                        rem_sleep_minutes: None,
                        light_sleep_minutes: None,
                        awake_minutes: None,
                        efficiency: Some(150.0), // Invalid: > 100%
                        source_device: Some("Test".to_string()),
                        created_at: now,
                    }),
                    
                    // Invalid activity (negative values)
                    HealthMetric::Activity(ActivityMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: now.date_naive().and_time(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()).and_utc(),
                        step_count: Some(-1000), // Invalid: negative
                        distance_meters: Some(-500.0), // Invalid: negative
                        active_energy_burned_kcal: Some(-200.0), // Invalid: negative
                        basal_energy_burned_kcal: None,
                        flights_climbed: None,
                        source_device: Some("Test".to_string()),
                        created_at: now,
                    }),
                ],
                workouts: vec![
                    WorkoutData {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        workout_type: "".to_string(), // Invalid: empty type
                        started_at: now,
                        ended_at: now - chrono::Duration::hours(1), // Invalid: end before start
                        total_energy_kcal: Some(-100.0), // Invalid: negative
                        active_energy_kcal: Some(-90.0), // Invalid: negative
                        distance_meters: Some(2000000.0), // Invalid: too far
                        avg_heart_rate: Some(400), // Invalid: too high
                        max_heart_rate: Some(500), // Invalid: too high
                        source_device: Some("Test".to_string()),
                        created_at: now,
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
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: timestamp,
                        heart_rate: 70 + (i % 50) as i32,
                        resting_heart_rate: None,
                        context: Some(ActivityContext::Resting),
                        source_device: Some("Performance Test".to_string()),
                        created_at: timestamp,
                    }));
                }
                1 => {
                    metrics.push(HealthMetric::BloodPressure(BloodPressureMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: timestamp,
                        systolic: 110 + (i % 30) as i32,
                        diastolic: 70 + (i % 20) as i32,
                        pulse: Some(65 + (i % 40) as i16),
                        source_device: Some("Performance Test".to_string()),
                        created_at: timestamp,
                    }));
                }
                2 => {
                    metrics.push(HealthMetric::Activity(ActivityMetric {
                        id: Uuid::new_v4(),
                        user_id: Uuid::new_v4(),
                        recorded_at: timestamp.date_naive().and_time(chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()).and_utc(),
                        step_count: Some(5000 + (i % 10000) as i32),
                        distance_meters: Some(3000.0 + (i % 5000) as f64),
                        active_energy_burned_kcal: Some(400.0 + (i % 600) as f64),
                        basal_energy_burned_kcal: Some(1800.0 + (i % 400) as f64),
                        flights_climbed: Some(i % 20 as i32),
                        source_device: Some("Performance Test".to_string()),
                        created_at: timestamp,
                    }));
                }
                _ => {
                    // Every 4th metric is a sleep metric (less frequent)
                    if i % 24 == 0 { // Only add sleep metrics every 24 iterations
                        metrics.push(HealthMetric::Sleep(SleepMetric {
                            id: Uuid::new_v4(),
                            user_id: Uuid::new_v4(),
                            recorded_at: timestamp,
                            sleep_start: timestamp - chrono::Duration::hours(8),
                            sleep_end: timestamp,
                            duration_minutes: 420 + (i % 120) as i32,
                            deep_sleep_minutes: Some(90 + (i % 60) as i32),
                            rem_sleep_minutes: Some(60 + (i % 40) as i32),
                            light_sleep_minutes: Some(240 + (i % 60) as i32),
                            awake_minutes: Some(i % 30 as i32),
                            efficiency: Some(80.0 + (i % 20) as f32),
                            source_device: Some("Performance Test".to_string()),
                            created_at: timestamp,
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
