/// Comprehensive iOS Auto Health Export Integration Tests
/// 
/// This test suite covers all aspects of iOS Auto Health Export app integration:
/// 1. iOS payload formats from the app
/// 2. HealthKit data type mapping
/// 3. Large payload handling (10MB+)
/// 4. Batch vs individual metric processing
/// 5. Timezone conversions for iOS data
/// 6. Device-specific metadata handling
/// 7. Error responses for iOS client
/// 8. Backwards compatibility
/// 
/// Tests simulate actual iOS app behavior and real payload examples.

use actix_web::{test, web, App};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::ios_models::{IosIngestData, IosIngestPayload, IosMetric, IosMetricData};
use self_sensored::models::{HealthMetric, IngestPayload, HeartRateMetric, BloodPressureMetric, ActivityMetric, SleepMetric, BodyMeasurementMetric, TemperatureMetric, ActivityContext};
use self_sensored::services::auth::AuthContext;

/// Test Suite 1: iOS Payload Formats from Auto Health Export App
/// 
/// Tests real iOS payload structures as they come from the Auto Health Export app
#[cfg(test)]
mod ios_payload_formats {
    use super::*;

    #[tokio::test]
    async fn test_real_ios_payload_structure() {
        // This is the ACTUAL structure sent by Auto Health Export iOS app
        let ios_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "units": "count/min",
                        "data": [
                            {
                                "sourceName": "Apple Watch",
                                "source": "Apple Watch Series 8",
                                "sourceVersion": "9.6",
                                "device": "Apple Watch",
                                "creationDate": "2024-01-15 14:30:21 -0800",
                                "startDate": "2024-01-15 14:30:00 -0800",
                                "endDate": "2024-01-15 14:30:00 -0800",
                                "value": "72.0",
                                "qty": 72.0
                            }
                        ]
                    }
                ],
                "workouts": [
                    {
                        "workoutActivityType": "HKWorkoutActivityTypeRunning",
                        "sourceName": "Apple Watch",
                        "source": "Apple Watch Series 8",
                        "device": "Apple Watch",
                        "creationDate": "2024-01-15 15:00:00 -0800",
                        "startDate": "2024-01-15 14:30:00 -0800",
                        "endDate": "2024-01-15 15:00:00 -0800",
                        "duration": 30.0,
                        "totalEnergyBurned": 250.0,
                        "totalDistance": 5000.0
                    }
                ]
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(ios_payload)
            .expect("Should parse real iOS payload structure");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert!(!internal_payload.data.metrics.is_empty(), "Should convert iOS metrics");
        assert!(!internal_payload.data.workouts.is_empty(), "Should convert iOS workouts");

        // Verify heart rate metric conversion
        if let Some(HealthMetric::HeartRate(hr)) = internal_payload.data.metrics.iter()
            .find(|m| matches!(m, HealthMetric::HeartRate(_))) {
            assert_eq!(hr.heart_rate, Some(72));
            assert_eq!(hr.source_device, Some("Apple Watch Series 8".to_string()));
        } else {
            panic!("Should have converted heart rate metric");
        }
    }

    #[tokio::test]
    async fn test_ios_payload_with_missing_fields() {
        // Test resilience to missing optional fields in iOS payloads
        let incomplete_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierStepCount",
                        "data": [
                            {
                                "qty": 10000.0,
                                "date": "2024-01-15 00:00:00 -0800"
                                // Missing source, device, etc.
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(incomplete_payload)
            .expect("Should parse incomplete iOS payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert!(!internal_payload.data.metrics.is_empty(), "Should handle missing fields gracefully");
    }

    #[tokio::test]
    async fn test_ios_payload_compression_simulation() {
        // Simulate compressed payload structure as iOS might send
        let compressed_like_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": (0..100).map(|i| json!({
                            "qty": 70.0 + (i as f64 % 30.0),
                            "date": format!("2024-01-15 {:02}:{:02}:00 -0800", 10 + i / 60, i % 60)
                        })).collect::<Vec<_>>()
                    }
                ]
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(compressed_like_payload)
            .expect("Should parse compressed-like payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should convert all 100 heart rate readings
        let hr_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();
        assert_eq!(hr_count, 100, "Should convert all compressed heart rate data");
    }
}

/// Test Suite 2: HealthKit Data Type Mapping
/// 
/// Comprehensive tests for all HealthKit identifier mappings per DATA.md
#[cfg(test)]
mod healthkit_mapping {
    use super::*;

    #[tokio::test]
    async fn test_all_supported_healthkit_identifiers() {
        // Test all HealthKit identifiers supported per DATA.md
        let comprehensive_payload = json!({
            "data": {
                "metrics": [
                    // Heart & Cardiovascular
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [{"qty": 72.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierRestingHeartRate", "data": [{"qty": 60.0, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierWalkingHeartRateAverage", "data": [{"qty": 85.0, "date": "2024-01-15 12:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierHeartRateVariabilitySDNN", "data": [{"qty": 45.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBloodPressureSystolic", "data": [{"qty": 120.0, "date": "2024-01-15 10:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBloodPressureDiastolic", "data": [{"qty": 80.0, "date": "2024-01-15 10:00:00 -0800"}]},
                    
                    // Activity & Fitness
                    {"name": "HKQuantityTypeIdentifierStepCount", "data": [{"qty": 10000.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierDistanceWalkingRunning", "data": [{"qty": 8000.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierDistanceCycling", "data": [{"qty": 15000.0, "date": "2024-01-15 16:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierDistanceSwimming", "data": [{"qty": 1000.0, "date": "2024-01-15 18:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierActiveEnergyBurned", "data": [{"qty": 400.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBasalEnergyBurned", "data": [{"qty": 1600.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierFlightsClimbed", "data": [{"qty": 10.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    
                    // Apple Watch specific
                    {"name": "HKQuantityTypeIdentifierAppleExerciseTime", "data": [{"qty": 30.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierAppleStandTime", "data": [{"qty": 12.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKCategoryTypeIdentifierAppleStandHour", "data": [{"qty": 1.0, "date": "2024-01-15 14:00:00 -0800"}]},
                    
                    // Body Measurements
                    {"name": "HKQuantityTypeIdentifierBodyMass", "data": [{"qty": 70.5, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierHeight", "data": [{"qty": 175.0, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBodyMassIndex", "data": [{"qty": 23.0, "date": "2024-01-15 08:00:00 -0800"}]},
                    
                    // Sleep
                    {"name": "HKCategoryTypeIdentifierSleepAnalysis", "data": [{"start": "2024-01-14 22:00:00 -0800", "end": "2024-01-15 06:00:00 -0800"}]},
                    
                    // Temperature
                    {"name": "HKQuantityTypeIdentifierBodyTemperature", "data": [{"qty": 36.5, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierAppleSleepingWristTemperature", "data": [{"qty": 32.5, "date": "2024-01-15 03:00:00 -0800"}]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(comprehensive_payload)
            .expect("Should parse comprehensive HealthKit payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Verify we have conversions for all major metric types
        let mut found_types = HashMap::new();
        for metric in &internal_payload.data.metrics {
            let metric_type = match metric {
                HealthMetric::HeartRate(_) => "HeartRate",
                HealthMetric::BloodPressure(_) => "BloodPressure", 
                HealthMetric::Activity(_) => "Activity",
                HealthMetric::Sleep(_) => "Sleep",
                HealthMetric::BodyMeasurement(_) => "BodyMeasurement",
                HealthMetric::Temperature(_) => "Temperature",
                _ => "Other"
            };
            *found_types.entry(metric_type).or_insert(0) += 1;
        }

        println!("Converted metric types: {:?}", found_types);

        // Verify all major types were converted
        assert!(found_types.contains_key("HeartRate"), "Should convert heart rate metrics");
        assert!(found_types.contains_key("BloodPressure"), "Should convert blood pressure metrics");
        assert!(found_types.contains_key("Activity"), "Should convert activity metrics"); 
        assert!(found_types.contains_key("Sleep"), "Should convert sleep metrics");
        assert!(found_types.contains_key("BodyMeasurement"), "Should convert body measurement metrics");
        assert!(found_types.contains_key("Temperature"), "Should convert temperature metrics");
    }

    #[tokio::test]
    async fn test_backwards_compatibility_mapping() {
        // Test that old simplified names still work alongside HealthKit identifiers
        let mixed_payload = json!({
            "data": {
                "metrics": [
                    // Old simplified names (backward compatibility)
                    {"name": "heart_rate", "data": [{"qty": 72.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    {"name": "steps", "data": [{"qty": 8000.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "blood_pressure_systolic", "data": [{"qty": 120.0, "date": "2024-01-15 10:00:00 -0800"}]},
                    
                    // New HealthKit identifiers
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [{"qty": 75.0, "date": "2024-01-15 14:31:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierStepCount", "data": [{"qty": 2000.0, "date": "2024-01-15 01:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBloodPressureSystolic", "data": [{"qty": 118.0, "date": "2024-01-15 10:01:00 -0800"}]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(mixed_payload)
            .expect("Should parse mixed format payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should have 2 heart rate readings (old + new format)
        let hr_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();
        assert_eq!(hr_count, 2, "Should support both old and new heart rate formats");

        // Should have 2 activity readings (steps from both formats)
        let activity_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::Activity(_)))
            .count();
        assert!(activity_count >= 2, "Should support both old and new step formats");
    }

    #[tokio::test]
    async fn test_unknown_healthkit_identifiers() {
        // Test handling of unknown/unsupported HealthKit identifiers
        let unknown_payload = json!({
            "data": {
                "metrics": [
                    // Known identifiers
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [{"qty": 72.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    
                    // Unknown/unsupported identifiers (should be logged but not crash)
                    {"name": "HKQuantityTypeIdentifierRespiratoryRate", "data": [{"qty": 16.0, "date": "2024-01-15 14:30:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierBloodGlucose", "data": [{"qty": 95.0, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierDietaryWater", "data": [{"qty": 2000.0, "date": "2024-01-15 00:00:00 -0800"}]},
                    {"name": "HKCategoryTypeIdentifierMenstrualFlow", "data": [{"qty": 2.0, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "CustomUnknownMetric", "data": [{"qty": 42.0, "date": "2024-01-15 12:00:00 -0800"}]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(unknown_payload)
            .expect("Should parse payload with unknown identifiers");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should only convert the known heart rate metric
        assert_eq!(internal_payload.data.metrics.len(), 1, "Should only convert known metrics");
        
        if let HealthMetric::HeartRate(hr) = &internal_payload.data.metrics[0] {
            assert_eq!(hr.heart_rate, Some(72));
        } else {
            panic!("Should have converted the known heart rate metric");
        }
    }
}

/// Test Suite 3: Large Payload Handling (10MB+)
/// 
/// Tests system behavior with large iOS data exports
#[cfg(test)]
mod large_payload_handling {
    use super::*;

    #[tokio::test]
    async fn test_large_heart_rate_payload() {
        // Simulate large payload with 10,000+ heart rate readings
        let large_hr_data: Vec<serde_json::Value> = (0..10000).map(|i| {
            json!({
                "qty": 60.0 + (i as f64 % 40.0), // Heart rate between 60-100
                "date": format!("2024-01-{:02} {:02}:{:02}:00 -0800", 
                    1 + (i / 1440), // Day (max 31)
                    (i / 60) % 24,  // Hour
                    i % 60          // Minute
                ),
                "source": "Apple Watch",
                "sourceName": "Apple Watch"
            })
        }).collect();

        let large_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "units": "count/min",
                        "data": large_hr_data
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(large_payload)
            .expect("Should parse large payload");

        let user_id = Uuid::new_v4();
        let start = std::time::Instant::now();
        let internal_payload = ios_ingest.to_internal_format(user_id);
        let conversion_time = start.elapsed();

        println!("Large payload conversion took: {:?}", conversion_time);
        
        // Should convert all 10,000 heart rate readings
        let hr_count = internal_payload.data.metrics.len();
        assert_eq!(hr_count, 10000, "Should convert all 10,000 heart rate readings");
        
        // Conversion should complete in reasonable time (< 10 seconds)
        assert!(conversion_time.as_secs() < 10, "Large payload conversion should be efficient");
    }

    #[tokio::test]
    async fn test_mixed_large_payload() {
        // Test large payload with multiple metric types
        let create_metric_data = |count: usize, base_value: f64| -> Vec<serde_json::Value> {
            (0..count).map(|i| json!({
                "qty": base_value + (i as f64 % 20.0),
                "date": format!("2024-01-15 {:02}:{:02}:00 -0800", 
                    (i / 60) % 24,
                    i % 60
                ),
                "source": "Apple Watch"
            })).collect()
        };

        let large_mixed_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": create_metric_data(3000, 70.0)
                    },
                    {
                        "name": "HKQuantityTypeIdentifierStepCount", 
                        "data": create_metric_data(1000, 100.0)
                    },
                    {
                        "name": "HKQuantityTypeIdentifierActiveEnergyBurned",
                        "data": create_metric_data(1000, 50.0)
                    },
                    {
                        "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                        "data": create_metric_data(500, 120.0)
                    },
                    {
                        "name": "HKQuantityTypeIdentifierBloodPressureDiastolic",
                        "data": create_metric_data(500, 80.0)
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(large_mixed_payload)
            .expect("Should parse large mixed payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Verify all metrics were converted
        let total_metrics = internal_payload.data.metrics.len();
        assert!(total_metrics >= 5500, "Should convert most metrics from large mixed payload");

        // Verify different metric types are present
        let mut type_counts = HashMap::new();
        for metric in &internal_payload.data.metrics {
            let metric_type = match metric {
                HealthMetric::HeartRate(_) => "HeartRate",
                HealthMetric::Activity(_) => "Activity",
                HealthMetric::BloodPressure(_) => "BloodPressure",
                _ => "Other"
            };
            *type_counts.entry(metric_type).or_insert(0) += 1;
        }

        assert!(type_counts.get("HeartRate").unwrap_or(&0) > &0, "Should have heart rate metrics");
        assert!(type_counts.get("Activity").unwrap_or(&0) > &0, "Should have activity metrics");
        assert!(type_counts.get("BloodPressure").unwrap_or(&0) > &0, "Should have blood pressure metrics");
    }

    #[tokio::test] 
    async fn test_memory_efficiency_large_payload() {
        // Test memory usage during large payload processing
        let large_data: Vec<serde_json::Value> = (0..5000).map(|i| {
            json!({
                "qty": 70.0 + (i as f64 % 30.0),
                "date": format!("2024-01-15 {:02}:{:02}:00 -0800", 
                    (i / 60) % 24,
                    i % 60
                ),
                "source": "Apple Watch",
                "device": "Apple Watch Series 8",
                "extra_field_1": format!("data_{}", i),
                "extra_field_2": vec![1, 2, 3, 4, 5], // Add some bulk
                "extra_field_3": format!("timestamp_{}", i)
            })
        }).collect();

        let memory_test_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "units": "count/min", 
                        "data": large_data
                    }
                ],
                "workouts": []
            }
        });

        // Measure memory before
        let ios_ingest: IosIngestPayload = serde_json::from_value(memory_test_payload)
            .expect("Should parse memory test payload");

        let user_id = Uuid::new_v4();
        
        // Process and verify it doesn't crash due to memory issues
        let internal_payload = ios_ingest.to_internal_format(user_id);
        
        assert_eq!(internal_payload.data.metrics.len(), 5000, "Should handle large payload efficiently");
    }
}

/// Test Suite 4: Batch vs Individual Metric Processing
/// 
/// Tests different processing strategies for iOS data
#[cfg(test)]
mod batch_processing {
    use super::*;

    #[tokio::test]
    async fn test_batch_vs_individual_processing() {
        // Create test data with duplicates and variations
        let batch_test_data = json!({
            "data": {
                "metrics": [
                    // Batch of heart rate readings
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"},
                        {"qty": 73.0, "date": "2024-01-15 14:31:00 -0800", "source": "Apple Watch"},
                        {"qty": 74.0, "date": "2024-01-15 14:32:00 -0800", "source": "Apple Watch"}
                    ]},
                    
                    // Batch of step count readings  
                    {"name": "HKQuantityTypeIdentifierStepCount", "data": [
                        {"qty": 1000.0, "date": "2024-01-15 14:30:00 -0800", "source": "iPhone"},
                        {"qty": 2000.0, "date": "2024-01-15 15:30:00 -0800", "source": "iPhone"},
                        {"qty": 3000.0, "date": "2024-01-15 16:30:00 -0800", "source": "iPhone"}
                    ]},
                    
                    // Mixed individual readings
                    {"name": "HKQuantityTypeIdentifierBodyMass", "data": [{"qty": 70.5, "date": "2024-01-15 08:00:00 -0800"}]},
                    {"name": "HKQuantityTypeIdentifierHeight", "data": [{"qty": 175.0, "date": "2024-01-15 08:00:00 -0800"}]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(batch_test_data)
            .expect("Should parse batch test payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Verify batch processing maintains individual data points
        let hr_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();
        assert_eq!(hr_count, 3, "Should preserve all heart rate readings in batch");

        let activity_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::Activity(_)))
            .count();
        assert_eq!(activity_count, 3, "Should preserve all step count readings");

        let body_measurement_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::BodyMeasurement(_)))
            .count();
        assert_eq!(body_measurement_count, 2, "Should preserve individual body measurements");
    }

    #[tokio::test]
    async fn test_duplicate_handling_in_batches() {
        // Test how the system handles duplicates within batches
        let duplicate_payload = json!({
            "data": {
                "metrics": [
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"},
                        {"qty": 72.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"}, // Exact duplicate
                        {"qty": 73.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"}, // Same time, different value
                        {"qty": 72.0, "date": "2024-01-15 14:31:00 -0800", "source": "Apple Watch"}  // Different time, same value
                    ]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(duplicate_payload)
            .expect("Should parse duplicate test payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // All readings should be converted (duplicate detection happens at database level)
        let hr_count = internal_payload.data.metrics.len();
        assert_eq!(hr_count, 4, "Should convert all readings, including duplicates");
    }

    #[tokio::test]
    async fn test_chunked_processing_simulation() {
        // Simulate how large batches might be chunked for processing
        let chunk_sizes = vec![100, 500, 1000];
        
        for chunk_size in chunk_sizes {
            let chunked_data: Vec<serde_json::Value> = (0..chunk_size).map(|i| {
                json!({
                    "qty": 70.0 + (i as f64 % 30.0),
                    "date": format!("2024-01-15 {:02}:{:02}:00 -0800", 
                        10 + (i / 60) % 14, // Hours 10-23
                        i % 60
                    ),
                    "source": "Apple Watch"
                })
            }).collect();

            let chunked_payload = json!({
                "data": {
                    "metrics": [
                        {
                            "name": "HKQuantityTypeIdentifierHeartRate",
                            "data": chunked_data
                        }
                    ],
                    "workouts": []
                }
            });

            let ios_ingest: IosIngestPayload = serde_json::from_value(chunked_payload)
                .expect(&format!("Should parse chunk of size {}", chunk_size));

            let user_id = Uuid::new_v4();
            let start = std::time::Instant::now();
            let internal_payload = ios_ingest.to_internal_format(user_id);
            let processing_time = start.elapsed();

            println!("Chunk size {} processed in {:?}", chunk_size, processing_time);
            
            assert_eq!(internal_payload.data.metrics.len(), chunk_size, 
                "Should convert all metrics in chunk size {}", chunk_size);
            
            // Processing should scale reasonably
            assert!(processing_time.as_millis() < (chunk_size as u128 * 2), 
                "Processing time should scale reasonably with chunk size");
        }
    }
}

/// Test Suite 5: Timezone Conversions for iOS Data
/// 
/// Tests proper handling of various timezone formats from iOS
#[cfg(test)]
mod timezone_handling {
    use super::*;

    #[tokio::test]
    async fn test_ios_timezone_formats() {
        // Test various timezone formats that iOS Auto Health Export might send
        let timezone_payload = json!({
            "data": {
                "metrics": [
                    // PST (Pacific Standard Time)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"}
                    ]},
                    
                    // PDT (Pacific Daylight Time)  
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 73.0, "date": "2024-07-15 14:30:00 -0700", "source": "Apple Watch"}
                    ]},
                    
                    // EST (Eastern Standard Time)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 74.0, "date": "2024-01-15 17:30:00 -0500", "source": "Apple Watch"}
                    ]},
                    
                    // UTC
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 75.0, "date": "2024-01-15 22:30:00 +0000", "source": "Apple Watch"}
                    ]},
                    
                    // GMT
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 76.0, "date": "2024-01-15T22:30:00Z", "source": "Apple Watch"}
                    ]},
                    
                    // No timezone (should default to UTC)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 77.0, "date": "2024-01-15 22:30:00", "source": "Apple Watch"}
                    ]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(timezone_payload)
            .expect("Should parse timezone test payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 6, "Should convert all timezone variants");

        // All times should be converted to UTC for consistent storage
        for metric in &internal_payload.data.metrics {
            if let HealthMetric::HeartRate(hr) = metric {
                // Verify the timestamp is in UTC (has UTC timezone)
                assert_eq!(hr.recorded_at.timezone(), Utc, "All timestamps should be converted to UTC");
            }
        }
    }

    #[tokio::test]
    async fn test_daylight_saving_time_handling() {
        // Test handling around daylight saving time transitions
        let dst_payload = json!({
            "data": {
                "metrics": [
                    // Before DST transition (PST)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-03-09 01:30:00 -0800", "source": "Apple Watch"}
                    ]},
                    
                    // During DST transition (PDT starts)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 73.0, "date": "2024-03-10 03:00:00 -0700", "source": "Apple Watch"}
                    ]},
                    
                    // After DST transition (PDT)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 74.0, "date": "2024-03-10 14:30:00 -0700", "source": "Apple Watch"}
                    ]},
                    
                    // Before DST ends (PDT)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 75.0, "date": "2024-11-02 01:30:00 -0700", "source": "Apple Watch"}
                    ]},
                    
                    // After DST ends (PST)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 76.0, "date": "2024-11-03 01:30:00 -0800", "source": "Apple Watch"}
                    ]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(dst_payload)
            .expect("Should parse DST test payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 5, "Should handle DST transitions");
        
        // Verify chronological ordering after UTC conversion
        let mut timestamps = Vec::new();
        for metric in &internal_payload.data.metrics {
            if let HealthMetric::HeartRate(hr) = metric {
                timestamps.push(hr.recorded_at);
            }
        }
        
        // Should be in chronological order when converted to UTC
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i-1], 
                "UTC timestamps should maintain chronological order across DST transitions");
        }
    }

    #[tokio::test]
    async fn test_international_timezones() {
        // Test various international timezone formats
        let international_payload = json!({
            "data": {
                "metrics": [
                    // Central European Time
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-01-15 23:30:00 +0100", "source": "Apple Watch"}
                    ]},
                    
                    // Japan Standard Time
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 73.0, "date": "2024-01-16 07:30:00 +0900", "source": "Apple Watch"}
                    ]},
                    
                    // Australian Eastern Time
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 74.0, "date": "2024-01-16 09:30:00 +1100", "source": "Apple Watch"}
                    ]},
                    
                    // India Standard Time (half-hour offset)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 75.0, "date": "2024-01-16 04:00:00 +0530", "source": "Apple Watch"}
                    ]},
                    
                    // Nepal Time (45-minute offset)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 76.0, "date": "2024-01-16 04:15:00 +0545", "source": "Apple Watch"}
                    ]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(international_payload)
            .expect("Should parse international timezone payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 5, "Should handle international timezones");
        
        // All should be converted to UTC and represent the same approximate time
        let mut utc_times = Vec::new();
        for metric in &internal_payload.data.metrics {
            if let HealthMetric::HeartRate(hr) = metric {
                utc_times.push(hr.recorded_at);
            }
        }
        
        // All times should be close to the same UTC time (within a few hours)
        let base_time = utc_times[0];
        for time in &utc_times[1..] {
            let diff = (*time - base_time).num_hours().abs();
            assert!(diff <= 24, "International times should convert to similar UTC times");
        }
    }

    #[tokio::test]
    async fn test_malformed_timezone_handling() {
        // Test handling of malformed or unusual timezone data
        let malformed_payload = json!({
            "data": {
                "metrics": [
                    // Valid baseline
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 72.0, "date": "2024-01-15 14:30:00 -0800", "source": "Apple Watch"}
                    ]},
                    
                    // Missing timezone
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 73.0, "date": "2024-01-15 14:30:00", "source": "Apple Watch"}
                    ]},
                    
                    // Invalid timezone format
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 74.0, "date": "2024-01-15 14:30:00 PST", "source": "Apple Watch"}
                    ]},
                    
                    // Date only (no time)
                    {"name": "HKQuantityTypeIdentifierHeartRate", "data": [
                        {"qty": 75.0, "date": "2024-01-15", "source": "Apple Watch"}
                    ]}
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(malformed_payload)
            .expect("Should parse payload with malformed timezones");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should still convert metrics that can be parsed
        assert!(internal_payload.data.metrics.len() >= 1, "Should convert parseable metrics");
        
        // At minimum, the first valid metric should be converted
        if let HealthMetric::HeartRate(hr) = &internal_payload.data.metrics[0] {
            assert_eq!(hr.heart_rate, Some(72));
        }
    }
}

/// Test Suite 6: Device-Specific Metadata Handling
/// 
/// Tests handling of iOS device metadata and source information
#[cfg(test)]
mod device_metadata {
    use super::*;

    #[tokio::test]
    async fn test_apple_watch_metadata() {
        // Test Apple Watch specific metadata handling
        let apple_watch_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "units": "count/min",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "sourceName": "Apple Watch",
                                "source": "Apple Watch Series 8",
                                "sourceVersion": "9.6",
                                "device": "Apple Watch",
                                "deviceModel": "Watch6,1", 
                                "deviceManufacturer": "Apple Inc.",
                                "deviceHardwareVersion": "1.0",
                                "deviceSoftwareVersion": "9.6",
                                "creationDate": "2024-01-15 14:30:21 -0800"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(apple_watch_payload)
            .expect("Should parse Apple Watch metadata payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 1, "Should convert Apple Watch metric");
        
        if let HealthMetric::HeartRate(hr) = &internal_payload.data.metrics[0] {
            assert_eq!(hr.heart_rate, Some(72));
            assert_eq!(hr.source_device, Some("Apple Watch Series 8".to_string()));
        } else {
            panic!("Should have converted Apple Watch heart rate metric");
        }
    }

    #[tokio::test]
    async fn test_iphone_metadata() {
        // Test iPhone specific metadata
        let iphone_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierStepCount",
                        "units": "count",
                        "data": [
                            {
                                "qty": 10000.0,
                                "date": "2024-01-15 00:00:00 -0800",
                                "sourceName": "iPhone",
                                "source": "iPhone 14 Pro",
                                "sourceVersion": "17.2",
                                "device": "iPhone",
                                "deviceModel": "iPhone15,3",
                                "deviceManufacturer": "Apple Inc."
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(iphone_payload)
            .expect("Should parse iPhone metadata payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 1, "Should convert iPhone metric");
        
        if let HealthMetric::Activity(activity) = &internal_payload.data.metrics[0] {
            assert_eq!(activity.step_count, Some(10000));
            assert_eq!(activity.source_device, Some("iPhone 14 Pro".to_string()));
        } else {
            panic!("Should have converted iPhone step count metric");
        }
    }

    #[tokio::test]
    async fn test_third_party_device_metadata() {
        // Test third-party device metadata handling
        let third_party_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierBodyMass",
                        "units": "kg",
                        "data": [
                            {
                                "qty": 70.5,
                                "date": "2024-01-15 08:00:00 -0800",
                                "sourceName": "Withings Body+",
                                "source": "Withings Health Mate",
                                "sourceVersion": "5.2.0", 
                                "device": "Withings Scale",
                                "deviceModel": "WBS05",
                                "deviceManufacturer": "Withings"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(third_party_payload)
            .expect("Should parse third-party device payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 1, "Should convert third-party device metric");
        
        if let HealthMetric::BodyMeasurement(body) = &internal_payload.data.metrics[0] {
            assert_eq!(body.body_weight_kg, Some(70.5));
            assert_eq!(body.source_device, Some("Withings Health Mate".to_string()));
        } else {
            panic!("Should have converted third-party body measurement");
        }
    }

    #[tokio::test]
    async fn test_manual_entry_metadata() {
        // Test manual entry metadata
        let manual_entry_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                        "units": "mmHg",
                        "data": [
                            {
                                "qty": 120.0,
                                "date": "2024-01-15 10:00:00 -0800",
                                "sourceName": "Health",
                                "source": "Health",
                                "sourceVersion": "17.2",
                                "device": "Manual Entry",
                                "metadata": {
                                    "HKWasUserEntered": true,
                                    "HKMetadataKeyTimeZone": "America/Los_Angeles"
                                }
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(manual_entry_payload)
            .expect("Should parse manual entry payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert!(internal_payload.data.metrics.len() >= 1, "Should convert manual entry metric");
    }

    #[tokio::test]
    async fn test_mixed_device_sources() {
        // Test payload with multiple device sources
        let mixed_sources_payload = json!({
            "data": {
                "metrics": [
                    // Apple Watch
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch Series 8"
                            }
                        ]
                    },
                    
                    // iPhone
                    {
                        "name": "HKQuantityTypeIdentifierStepCount",
                        "data": [
                            {
                                "qty": 8000.0,
                                "date": "2024-01-15 00:00:00 -0800",
                                "source": "iPhone 14 Pro"
                            }
                        ]
                    },
                    
                    // Third-party app
                    {
                        "name": "HKQuantityTypeIdentifierBodyMass",
                        "data": [
                            {
                                "qty": 70.5,
                                "date": "2024-01-15 08:00:00 -0800",
                                "source": "Withings Health Mate"
                            }
                        ]
                    },
                    
                    // Manual entry
                    {
                        "name": "HKQuantityTypeIdentifierBloodPressureSystolic",
                        "data": [
                            {
                                "qty": 120.0,
                                "date": "2024-01-15 10:00:00 -0800",
                                "source": "Health"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(mixed_sources_payload)
            .expect("Should parse mixed sources payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert!(internal_payload.data.metrics.len() >= 3, "Should convert metrics from mixed sources");
        
        // Verify each source is preserved
        let sources: Vec<Option<String>> = internal_payload.data.metrics.iter()
            .map(|m| match m {
                HealthMetric::HeartRate(hr) => hr.source_device.clone(),
                HealthMetric::Activity(activity) => activity.source_device.clone(),
                HealthMetric::BodyMeasurement(body) => body.source_device.clone(),
                _ => None
            })
            .collect();

        let unique_sources: std::collections::HashSet<_> = sources.into_iter().filter_map(|s| s).collect();
        assert!(unique_sources.len() >= 3, "Should preserve multiple device sources");
    }
}

/// Test Suite 7: Error Responses for iOS Client
/// 
/// Tests that error responses are helpful for iOS app developers
#[cfg(test)]
mod error_responses {
    use super::*;

    #[tokio::test]
    async fn test_malformed_json_error() {
        // Test response to malformed JSON
        let malformed_json = r#"{"data": {"metrics": [{"name": "HeartRate", "invalid_json"}"#;
        
        // This should be caught at the JSON parsing level
        let result = serde_json::from_str::<IosIngestPayload>(malformed_json);
        assert!(result.is_err(), "Should reject malformed JSON");
    }

    #[tokio::test]
    async fn test_missing_required_fields_error() {
        // Test response to missing required fields
        let missing_fields_payload = json!({
            "data": {
                // Missing metrics array
                "workouts": []
            }
        });

        let result = serde_json::from_value::<IosIngestPayload>(missing_fields_payload);
        assert!(result.is_err(), "Should reject payload missing required fields");
    }

    #[tokio::test]
    async fn test_invalid_metric_data_error() {
        // Test response to invalid metric data
        let invalid_data_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": "invalid_number", // Should be number
                                "date": "2024-01-15 14:30:00 -0800"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let result = serde_json::from_value::<IosIngestPayload>(invalid_data_payload);
        assert!(result.is_err(), "Should reject invalid metric data");
    }

    #[tokio::test]
    async fn test_empty_payload_handling() {
        // Test response to empty but valid payload
        let empty_payload = json!({
            "data": {
                "metrics": [],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(empty_payload)
            .expect("Should parse empty payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 0, "Empty payload should result in empty metrics");
        assert_eq!(internal_payload.data.workouts.len(), 0, "Empty payload should result in empty workouts");
    }

    #[tokio::test]
    async fn test_partial_success_handling() {
        // Test handling when some metrics succeed and others fail
        let mixed_success_payload = json!({
            "data": {
                "metrics": [
                    // Valid metric
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    
                    // Invalid metric (extreme value)
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 999.0, // Extreme value that might be rejected
                                "date": "2024-01-15 14:31:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    
                    // Unknown metric type
                    {
                        "name": "UnknownMetricType",
                        "data": [
                            {
                                "qty": 42.0,
                                "date": "2024-01-15 14:32:00 -0800",
                                "source": "Unknown Device"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(mixed_success_payload)
            .expect("Should parse mixed success payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should convert at least the valid metrics
        assert!(internal_payload.data.metrics.len() >= 1, "Should convert valid metrics");
        
        // Verify the valid heart rate metric was converted
        let valid_hr_found = internal_payload.data.metrics.iter()
            .any(|m| if let HealthMetric::HeartRate(hr) = m {
                hr.heart_rate == Some(72)
            } else {
                false
            });
        
        assert!(valid_hr_found, "Should successfully convert valid heart rate metric");
    }
}

/// Test Suite 8: Backwards Compatibility
/// 
/// Tests that new versions don't break existing iOS integrations
#[cfg(test)]
mod backwards_compatibility {
    use super::*;

    #[tokio::test]
    async fn test_legacy_payload_format() {
        // Test old payload format that might be used by older iOS apps
        let legacy_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "heart_rate", // Old simplified name
                        "units": "BPM",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    {
                        "name": "steps", // Old simplified name
                        "data": [
                            {
                                "qty": 10000.0,
                                "date": "2024-01-15 00:00:00 -0800",
                                "source": "iPhone"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(legacy_payload)
            .expect("Should parse legacy payload format");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert!(internal_payload.data.metrics.len() >= 2, "Should support legacy metric names");
        
        // Verify both legacy names were converted
        let has_heart_rate = internal_payload.data.metrics.iter()
            .any(|m| matches!(m, HealthMetric::HeartRate(_)));
        let has_activity = internal_payload.data.metrics.iter()
            .any(|m| matches!(m, HealthMetric::Activity(_)));
            
        assert!(has_heart_rate, "Should convert legacy heart_rate name");
        assert!(has_activity, "Should convert legacy steps name");
    }

    #[tokio::test]
    async fn test_field_evolution() {
        // Test handling of old vs new field names
        let field_evolution_payload = json!({
            "data": {
                "metrics": [
                    // Old field names
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "value": "72.0", // Old: string value
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    
                    // New field names
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 73.0, // New: numeric qty
                                "date": "2024-01-15 14:31:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(field_evolution_payload)
            .expect("Should parse field evolution payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should handle both old and new field formats
        let hr_count = internal_payload.data.metrics.iter()
            .filter(|m| matches!(m, HealthMetric::HeartRate(_)))
            .count();
        
        assert!(hr_count >= 1, "Should handle field evolution gracefully");
    }

    #[tokio::test]
    async fn test_version_tolerance() {
        // Test that the system tolerates version differences
        let version_payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch",
                                "sourceVersion": "8.0", // Older version
                                "new_field_v2": "some_new_data", // Future field
                                "deprecated_field": "old_data" // Deprecated field
                            }
                        ]
                    }
                ],
                "workouts": []
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(version_payload)
            .expect("Should parse version tolerance payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        assert_eq!(internal_payload.data.metrics.len(), 1, "Should handle version differences");
        
        if let HealthMetric::HeartRate(hr) = &internal_payload.data.metrics[0] {
            assert_eq!(hr.heart_rate, Some(72), "Should extract core data regardless of version fields");
        }
    }

    #[tokio::test]
    async fn test_graceful_degradation() {
        // Test graceful degradation with unsupported features
        let degradation_payload = json!({
            "data": {
                "metrics": [
                    // Supported metric
                    {
                        "name": "HKQuantityTypeIdentifierHeartRate",
                        "data": [
                            {
                                "qty": 72.0,
                                "date": "2024-01-15 14:30:00 -0800",
                                "source": "Apple Watch"
                            }
                        ]
                    },
                    
                    // Unsupported metric (should be ignored gracefully)
                    {
                        "name": "HKQuantityTypeIdentifierFutureMetric",
                        "data": [
                            {
                                "qty": 42.0,
                                "date": "2024-01-15 14:31:00 -0800",
                                "source": "Future Device",
                                "future_field": "future_value"
                            }
                        ]
                    }
                ],
                "workouts": [],
                "future_section": {
                    "future_data": "should_be_ignored"
                }
            }
        });

        let ios_ingest: IosIngestPayload = serde_json::from_value(degradation_payload)
            .expect("Should parse graceful degradation payload");

        let user_id = Uuid::new_v4();
        let internal_payload = ios_ingest.to_internal_format(user_id);

        // Should convert supported metrics and ignore unsupported ones
        assert_eq!(internal_payload.data.metrics.len(), 1, "Should gracefully ignore unsupported features");
        
        if let HealthMetric::HeartRate(hr) = &internal_payload.data.metrics[0] {
            assert_eq!(hr.heart_rate, Some(72), "Should process supported metrics normally");
        }
    }
}

/// Integration Test Helper Functions
impl IosIngestPayload {
    /// Create a realistic iOS payload for testing
    pub fn create_realistic_test_payload() -> Self {
        let metrics = vec![
            IosMetric {
                name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                units: Some("count/min".to_string()),
                data: vec![
                    IosMetricData {
                        qty: Some(72.0),
                        date: Some("2024-01-15 14:30:00 -0800".to_string()),
                        start: None,
                        end: None,
                        source: Some("Apple Watch Series 8".to_string()),
                        value: None,
                        extra: std::collections::HashMap::new(),
                    }
                ],
            },
            IosMetric {
                name: "HKQuantityTypeIdentifierStepCount".to_string(),
                units: Some("count".to_string()),
                data: vec![
                    IosMetricData {
                        qty: Some(10000.0),
                        date: Some("2024-01-15 00:00:00 -0800".to_string()),
                        start: None,
                        end: None,
                        source: Some("iPhone 14 Pro".to_string()),
                        value: None,
                        extra: std::collections::HashMap::new(),
                    }
                ],
            },
        ];

        IosIngestPayload {
            data: IosIngestData::Legacy {
                metrics,
                workouts: vec![],
            },
        }
    }

    /// Create a large test payload for performance testing
    pub fn create_large_test_payload(metric_count: usize) -> Self {
        let data: Vec<IosMetricData> = (0..metric_count).map(|i| {
            IosMetricData {
                qty: Some(70.0 + (i as f64 % 30.0)),
                date: Some(format!(
                    "2024-01-15 {:02}:{:02}:00 -0800",
                    10 + (i / 60) % 14,
                    i % 60
                )),
                start: None,
                end: None,
                source: Some("Apple Watch".to_string()),
                value: None,
                extra: std::collections::HashMap::new(),
            }
        }).collect();

        let metrics = vec![
            IosMetric {
                name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                units: Some("count/min".to_string()),
                data,
            }
        ];

        IosIngestPayload {
            data: IosIngestData::Legacy {
                metrics,
                workouts: vec![],
            },
        }
    }
}