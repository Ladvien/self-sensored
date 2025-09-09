use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

use self_sensored::models::{
    HealthMetric, HeartRateMetric, BloodPressureMetric, SleepMetric, 
    ActivityMetric, WorkoutData, IngestPayload, IngestData,
    IosIngestPayload, IosIngestData, IosMetric, IosMetricData, IosWorkout
};

#[test]
fn test_standard_payload_serialization() {
    let now = Utc::now();
    
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate(HeartRateMetric {
                    recorded_at: now,
                    min_bpm: Some(65),
                    avg_bpm: Some(75),
                    max_bpm: Some(85),
                    source: Some("Test".to_string()),
                    context: Some("resting".to_string()),
                }),
                HealthMetric::BloodPressure(BloodPressureMetric {
                    recorded_at: now,
                    systolic: 120,
                    diastolic: 80,
                    pulse: Some(70),
                    source: Some("Test".to_string()),
                }),
            ],
            workouts: vec![
                WorkoutData {
                    workout_type: "Running".to_string(),
                    start_time: now,
                    end_time: now + chrono::Duration::hours(1),
                    total_energy_kcal: Some(300.0),
                    distance_meters: Some(5000.0),
                    avg_heart_rate: Some(150),
                    max_heart_rate: Some(175),
                    source: Some("Test".to_string()),
                },
            ],
        },
    };

    // Test serialization
    let json_str = serde_json::to_string(&payload).expect("Should serialize");
    assert!(!json_str.is_empty());

    // Test deserialization
    let deserialized: IngestPayload = serde_json::from_str(&json_str)
        .expect("Should deserialize");
    
    assert_eq!(deserialized.data.metrics.len(), 2);
    assert_eq!(deserialized.data.workouts.len(), 1);
}

#[test]
fn test_ios_payload_conversion() {
    let now = Utc::now();
    let date_str = now.to_rfc3339();
    
    let ios_payload = IosIngestPayload {
        data: IosIngestData {
            metrics: vec![
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
            ],
            workouts: vec![],
        },
    };

    // Test conversion to internal format
    let internal_payload = ios_payload.to_internal_format();
    
    // Should have heart rate and blood pressure metrics
    assert!(!internal_payload.data.metrics.is_empty());
    
    // Check for blood pressure pairing
    let bp_metrics: Vec<&HealthMetric> = internal_payload.data.metrics
        .iter()
        .filter(|m| matches!(m, HealthMetric::BloodPressure(_)))
        .collect();
    
    assert_eq!(bp_metrics.len(), 1, "Should have paired blood pressure readings");
    
    if let HealthMetric::BloodPressure(bp) = &bp_metrics[0] {
        assert_eq!(bp.systolic, 120);
        assert_eq!(bp.diastolic, 80);
    }
}

#[test]
fn test_metric_validation() {
    let now = Utc::now();
    
    // Valid heart rate
    let valid_hr = HealthMetric::HeartRate(HeartRateMetric {
        recorded_at: now,
        min_bpm: Some(65),
        avg_bpm: Some(75),
        max_bpm: Some(85),
        source: Some("Test".to_string()),
        context: None,
    });
    assert!(valid_hr.validate().is_ok());
    
    // Invalid heart rate (too high)
    let invalid_hr = HealthMetric::HeartRate(HeartRateMetric {
        recorded_at: now,
        min_bpm: Some(400), // Invalid
        avg_bpm: Some(75),
        max_bpm: Some(85),
        source: Some("Test".to_string()),
        context: None,
    });
    assert!(invalid_hr.validate().is_err());
    
    // Valid blood pressure
    let valid_bp = HealthMetric::BloodPressure(BloodPressureMetric {
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(70),
        source: Some("Test".to_string()),
    });
    assert!(valid_bp.validate().is_ok());
    
    // Invalid blood pressure
    let invalid_bp = HealthMetric::BloodPressure(BloodPressureMetric {
        recorded_at: now,
        systolic: 300, // Invalid
        diastolic: 80,
        pulse: Some(70),
        source: Some("Test".to_string()),
    });
    assert!(invalid_bp.validate().is_err());
    
    // Valid sleep
    let valid_sleep = HealthMetric::Sleep(SleepMetric {
        recorded_at: now,
        sleep_start: now - chrono::Duration::hours(8),
        sleep_end: now,
        total_sleep_minutes: 480,
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(90),
        awake_minutes: Some(30),
        efficiency_percentage: Some(90.0),
        source: Some("Test".to_string()),
    });
    assert!(valid_sleep.validate().is_ok());
    
    // Invalid sleep (end before start)
    let invalid_sleep = HealthMetric::Sleep(SleepMetric {
        recorded_at: now,
        sleep_start: now,
        sleep_end: now - chrono::Duration::hours(1), // Invalid
        total_sleep_minutes: 60,
        deep_sleep_minutes: None,
        rem_sleep_minutes: None,
        awake_minutes: None,
        efficiency_percentage: Some(150.0), // Also invalid
        source: Some("Test".to_string()),
    });
    assert!(invalid_sleep.validate().is_err());
    
    // Valid activity
    let valid_activity = HealthMetric::Activity(ActivityMetric {
        date: now.date_naive(),
        steps: Some(10000),
        distance_meters: Some(8000.0),
        calories_burned: Some(2000.0),
        active_minutes: Some(60),
        flights_climbed: Some(10),
        source: Some("Test".to_string()),
    });
    assert!(valid_activity.validate().is_ok());
    
    // Invalid activity (negative values)
    let invalid_activity = HealthMetric::Activity(ActivityMetric {
        date: now.date_naive(),
        steps: Some(-1000), // Invalid
        distance_meters: Some(-500.0), // Invalid
        calories_burned: Some(-200.0), // Invalid
        active_minutes: None,
        flights_climbed: None,
        source: Some("Test".to_string()),
    });
    assert!(invalid_activity.validate().is_err());
}

#[test]
fn test_large_payload_performance() {
    use std::time::Instant;
    
    let now = Utc::now();
    let start = Instant::now();
    
    // Generate 1000 metrics
    let mut metrics = Vec::new();
    for i in 0..1000 {
        let timestamp = now - chrono::Duration::minutes(i as i64);
        
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            recorded_at: timestamp,
            min_bpm: None,
            avg_bpm: Some(70 + (i % 50) as i16),
            max_bpm: None,
            source: Some("Performance Test".to_string()),
            context: Some("resting".to_string()),
        }));
    }
    
    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };
    
    let generation_time = start.elapsed();
    println!("Generated 1000 metrics in {:?}", generation_time);
    
    // Test serialization performance
    let start = Instant::now();
    let json_str = serde_json::to_string(&payload).expect("Should serialize");
    let serialization_time = start.elapsed();
    println!("Serialized in {:?}", serialization_time);
    
    // Test deserialization performance
    let start = Instant::now();
    let _: IngestPayload = serde_json::from_str(&json_str).expect("Should deserialize");
    let deserialization_time = start.elapsed();
    println!("Deserialized in {:?}", deserialization_time);
    
    // Performance assertions
    assert!(generation_time.as_millis() < 1000, "Should generate quickly");
    assert!(serialization_time.as_millis() < 2000, "Should serialize quickly");
    assert!(deserialization_time.as_millis() < 2000, "Should deserialize quickly");
    
    // Size check
    assert!(json_str.len() < 10 * 1024 * 1024, "Should be reasonable size");
    assert_eq!(payload.data.metrics.len(), 1000);
}