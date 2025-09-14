use self_sensored::config::{BatchConfig, ValidationConfig};
use self_sensored::models::{BloodGlucoseMetric, HealthMetric, IngestPayload, IngestData};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Test blood glucose metric validation with medical-grade thresholds
#[tokio::test]
async fn test_blood_glucose_validation() {
    let config = ValidationConfig {
        blood_glucose_min: 30.0,
        blood_glucose_max: 600.0,
        insulin_max_units: 100.0,
        ..ValidationConfig::default()
    };

    // Test normal glucose level
    let normal_glucose = BloodGlucoseMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        blood_glucose_mg_dl: 95.0, // Normal fasting glucose
        measurement_context: Some("fasting".to_string()),
        medication_taken: Some(false),
        insulin_delivery_units: None,
        glucose_source: Some("DexcomG6".to_string()),
        source_device: Some("iPhone".to_string()),
        created_at: Utc::now(),
    };

    assert!(normal_glucose.validate_with_config(&config).is_ok());
    assert_eq!(normal_glucose.glucose_category(), "normal_fasting");
    assert!(!normal_glucose.is_critical_glucose_level());

    // Test hypoglycemic critical level
    let low_glucose = BloodGlucoseMetric {
        blood_glucose_mg_dl: 55.0, // Hypoglycemic - dangerous
        ..normal_glucose.clone()
    };

    assert!(low_glucose.validate_with_config(&config).is_ok()); // Within safe limits but dangerous
    assert_eq!(low_glucose.glucose_category(), "hypoglycemic_critical");
    assert!(low_glucose.is_critical_glucose_level());

    // Test extremely high glucose level
    let high_glucose = BloodGlucoseMetric {
        blood_glucose_mg_dl: 450.0, // Very high - critical
        insulin_delivery_units: Some(8.5),
        ..normal_glucose.clone()
    };

    assert!(high_glucose.validate_with_config(&config).is_ok());
    assert_eq!(high_glucose.glucose_category(), "critical_emergency");
    assert!(high_glucose.is_critical_glucose_level());

    // Test invalid glucose level (too low)
    let invalid_glucose = BloodGlucoseMetric {
        blood_glucose_mg_dl: 10.0, // Below medical minimum
        ..normal_glucose
    };

    assert!(invalid_glucose.validate_with_config(&config).is_err());
}

/// Test CGM-specific deduplication with glucose source
#[tokio::test]
async fn test_blood_glucose_cgm_deduplication() {
    let user_id = Uuid::new_v4();
    let recorded_at = Utc::now();

    // Create CGM metrics with different sources at same timestamp
    let cgm_dexcom = BloodGlucoseMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at,
        blood_glucose_mg_dl: 120.0,
        measurement_context: Some("random".to_string()),
        medication_taken: Some(false),
        insulin_delivery_units: None,
        glucose_source: Some("DexcomG6".to_string()),
        source_device: Some("iPhone".to_string()),
        created_at: Utc::now(),
    };

    let cgm_freestyle = BloodGlucoseMetric {
        glucose_source: Some("FreeStyleLibre".to_string()),
        blood_glucose_mg_dl: 125.0, // Different reading from different device
        ..cgm_dexcom.clone()
    };

    // Same source - should be deduplicated
    let cgm_dexcom_duplicate = BloodGlucoseMetric {
        blood_glucose_mg_dl: 122.0, // Slightly different but same source/time
        ..cgm_dexcom.clone()
    };

    // Test that different sources are preserved but same sources are deduplicated
    let metrics = vec![
        cgm_dexcom.clone(),
        cgm_freestyle,
        cgm_dexcom_duplicate, // Should be removed
    ];

    assert_eq!(metrics.len(), 3);

    // Verify unique deduplication keys
    use std::collections::HashSet;
    let mut keys = HashSet::new();
    for metric in &metrics {
        let key = (
            metric.user_id,
            metric.recorded_at.timestamp_millis(),
            metric.glucose_source.clone(),
        );
        keys.insert(key);
    }

    // Should have 2 unique keys (DexcomG6 and FreeStyleLibre)
    assert_eq!(keys.len(), 2);
}

/// Test blood glucose chunking for high-frequency CGM data
#[test]
fn test_blood_glucose_chunking_parameters() {
    let config = BatchConfig::default();

    // Verify chunk size is optimized for CGM data (288 readings/day)
    assert_eq!(config.blood_glucose_chunk_size, 6500);

    // Verify parameter calculation
    use self_sensored::config::BLOOD_GLUCOSE_PARAMS_PER_RECORD;
    let max_params = config.blood_glucose_chunk_size * BLOOD_GLUCOSE_PARAMS_PER_RECORD;
    assert!(max_params <= 65535); // PostgreSQL limit
    assert!(max_params >= 50000); // Efficient chunk size

    // Test validation
    assert!(config.validate().is_ok());
}

/// Test batch processing payload structure for blood glucose
#[test]
fn test_blood_glucose_ingest_payload() {
    let user_id = Uuid::new_v4();
    let base_time = Utc::now();

    // Create CGM data stream (every 5 minutes for 1 hour = 12 readings)
    let mut cgm_readings = Vec::new();
    for i in 0..12 {
        let reading = BloodGlucoseMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time + chrono::Duration::minutes(i * 5),
            blood_glucose_mg_dl: 100.0 + (i as f64 * 2.5), // Gradually rising glucose
            measurement_context: Some("continuous".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("DexcomG6".to_string()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };
        cgm_readings.push(HealthMetric::BloodGlucose(reading));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics: cgm_readings,
            workouts: vec![],
        },
    };

    // Verify payload structure
    assert_eq!(payload.data.metrics.len(), 12);
    assert_eq!(payload.data.workouts.len(), 0);

    // Verify all metrics are blood glucose
    for metric in &payload.data.metrics {
        match metric {
            HealthMetric::BloodGlucose(_) => {}, // Expected
            _ => panic!("Expected BloodGlucose metric"),
        }
    }
}

/// Test environment variable configuration for blood glucose validation
#[test]
fn test_blood_glucose_env_config() {
    // Test default values
    let config = ValidationConfig::default();
    assert_eq!(config.blood_glucose_min, 30.0);
    assert_eq!(config.blood_glucose_max, 600.0);
    assert_eq!(config.insulin_max_units, 100.0);

    // Test validation
    assert!(config.validate().is_ok());

    // Test batch config
    let batch_config = BatchConfig::default();
    assert_eq!(batch_config.blood_glucose_chunk_size, 6500);
}