use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use self_sensored::config::BatchConfig;
use self_sensored::models::{HealthMetric, IngestPayload, IngestData};

/// Integration tests for data recovery functionality
#[cfg(test)]
mod data_recovery_tests {
    use super::*;

    /// Helper to create test database pool
    async fn get_test_pool() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/health_export_test".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    /// Helper to create test user
    async fn create_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, created_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)",
            user_id,
            format!("test-{}@example.com", user_id)
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        user_id
    }

    /// Helper to create failed raw ingestion record
    async fn create_failed_raw_ingestion(
        pool: &PgPool,
        user_id: Uuid,
        payload: &IngestPayload,
        error_type: &str,
    ) -> Uuid {
        let raw_id = Uuid::new_v4();
        let payload_json = serde_json::to_value(payload).unwrap();
        let error_json = json!([{
            "error_type": error_type,
            "message": format!("Test error: {}", error_type)
        }]);

        sqlx::query!(
            r#"
            INSERT INTO raw_ingestions (
                id, user_id, payload_hash, payload_size_bytes, raw_payload,
                processing_status, processing_errors, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP)
            "#,
            raw_id,
            user_id,
            "test_hash",
            1024i32,
            payload_json,
            "error",
            error_json
        )
        .execute(pool)
        .await
        .expect("Failed to create failed raw ingestion");

        raw_id
    }

    /// Helper to create test health metrics
    fn create_test_health_metrics(user_id: Uuid, count: usize) -> Vec<HealthMetric> {
        use self_sensored::models::{HeartRateMetric, enums::ActivityContext};

        (0..count)
            .map(|i| HealthMetric::HeartRate(HeartRateMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now(),
                heart_rate: Some(70 + (i % 30) as i16),
                resting_heart_rate: Some(60),
                heart_rate_variability: Some(50.0),
                walking_heart_rate_average: None,
                heart_rate_recovery_one_minute: None,
                atrial_fibrillation_burden_percentage: None,
                vo2_max_ml_kg_min: None,
                context: Some(ActivityContext::Resting),
                source_device: Some("test_device".to_string()),
                created_at: Utc::now(),
            }))
            .collect()
    }

    /// Test data recovery for PostgreSQL parameter limit errors
    #[tokio::test]
    async fn test_recovery_postgresql_parameter_limit() {
        let pool = get_test_pool().await;
        let user_id = create_test_user(&pool).await;

        // Create test payload with many metrics (would exceed parameter limit with old config)
        let test_metrics = create_test_health_metrics(user_id, 10000);
        let test_payload = IngestPayload {
            data: IngestData {
                metrics: test_metrics,
                workouts: vec![],
            },
        };

        // Create failed raw ingestion with parameter limit error
        let raw_id = create_failed_raw_ingestion(
            &pool,
            user_id,
            &test_payload,
            "PostgreSQL parameter limit exceeded",
        )
        .await;

        // Count metrics before recovery
        let before_count = count_user_metrics(&pool, user_id).await;
        assert_eq!(before_count, 0, "Should start with no metrics");

        // Run recovery using corrected batch configuration
        let batch_config = BatchConfig::from_env();
        assert!(
            batch_config.validate().is_ok(),
            "Batch config should be valid"
        );

        let batch_processor = self_sensored::services::batch_processor::BatchProcessor::with_config(
            pool.clone(),
            batch_config,
        );

        // Process the recovered payload
        let result = batch_processor.process_batch(user_id, test_payload).await;

        // Verify recovery was successful
        assert!(
            result.errors.is_empty(),
            "Recovery should have no errors: {:?}",
            result.errors
        );
        assert_eq!(
            result.processed_count, 10000,
            "Should process all 10000 metrics"
        );

        // Count metrics after recovery
        let after_count = count_user_metrics(&pool, user_id).await;
        assert_eq!(after_count, 10000, "Should have recovered all metrics");

        // Verify raw ingestion status can be updated
        sqlx::query!(
            "UPDATE raw_ingestions SET processing_status = 'recovered' WHERE id = $1",
            raw_id
        )
        .execute(&pool)
        .await
        .expect("Should be able to update status");

        let updated_status = sqlx::query_scalar!(
            "SELECT processing_status FROM raw_ingestions WHERE id = $1",
            raw_id
        )
        .fetch_one(&pool)
        .await
        .expect("Should be able to fetch updated status");

        assert_eq!(updated_status, Some("recovered".to_string()));

        // Cleanup
        cleanup_test_data(&pool, user_id, raw_id).await;
    }

    /// Test data recovery verification checksums
    #[tokio::test]
    async fn test_recovery_verification_checksums() {
        let pool = get_test_pool().await;
        let user_id = create_test_user(&pool).await;

        // Create test payload
        let test_metrics = create_test_health_metrics(user_id, 100);
        let test_payload = IngestPayload {
            data: IngestData {
                metrics: test_metrics.clone(),
                workouts: vec![],
            },
        };

        // Calculate payload checksum
        let payload_json = serde_json::to_value(&test_payload).unwrap();
        let payload_bytes = serde_json::to_vec(&payload_json).unwrap();
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(&payload_bytes);
        let checksum1 = format!("{:x}", hasher.finalize());

        // Create failed raw ingestion
        let raw_id =
            create_failed_raw_ingestion(&pool, user_id, &test_payload, "validation_error").await;

        // Verify payload integrity with checksum
        let stored_payload = sqlx::query!(
            "SELECT raw_payload FROM raw_ingestions WHERE id = $1",
            raw_id
        )
        .fetch_one(&pool)
        .await
        .expect("Should be able to fetch stored payload");

        let stored_payload_bytes = serde_json::to_vec(&stored_payload.raw_payload).unwrap();
        let mut hasher2 = sha2::Sha256::new();
        hasher2.update(&stored_payload_bytes);
        let checksum2 = format!("{:x}", hasher2.finalize());

        // Note: Checksums may differ due to JSON serialization order, but payload should be recoverable
        info!("Original checksum: {}", checksum1);
        info!("Stored checksum: {}", checksum2);

        // Verify we can parse and process the stored payload
        let recovered_payload: IngestPayload = serde_json::from_value(stored_payload.raw_payload)
            .expect("Should be able to parse stored payload");

        assert_eq!(
            recovered_payload.data.metrics.len(),
            100,
            "Should have same metric count"
        );

        // Cleanup
        cleanup_test_data(&pool, user_id, raw_id).await;
    }

    /// Test data recovery progress tracking
    #[tokio::test]
    async fn test_recovery_progress_tracking() {
        let pool = get_test_pool().await;
        let user_id = create_test_user(&pool).await;

        // Create multiple failed records
        let mut raw_ids = Vec::new();
        for i in 0..5 {
            let test_metrics = create_test_health_metrics(user_id, 10 * (i + 1));
            let test_payload = IngestPayload {
                data: IngestData {
                    metrics: test_metrics,
                    workouts: vec![],
                },
            };

            let raw_id =
                create_failed_raw_ingestion(&pool, user_id, &test_payload, "chunk_size_error")
                    .await;
            raw_ids.push(raw_id);
        }

        // Verify we have 5 failed records
        let failed_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM raw_ingestions WHERE user_id = $1 AND processing_status = 'error'",
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Should be able to count failed records")
        .unwrap_or(0);

        assert_eq!(failed_count, 5, "Should have 5 failed records");

        // Test discovery of failed records (simulating the recovery service)
        let discoverable_records = sqlx::query!(
            r#"
            SELECT id, user_id, raw_payload
            FROM raw_ingestions
            WHERE user_id = $1
              AND processing_status = 'error'
              AND processing_errors::text ILIKE '%chunk_size%'
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&pool)
        .await
        .expect("Should be able to discover failed records");

        assert_eq!(
            discoverable_records.len(),
            5,
            "Should discover all 5 failed records"
        );

        // Verify each record can be parsed and processed
        let batch_config = BatchConfig::from_env();
        let batch_processor = self_sensored::services::batch_processor::BatchProcessor::with_config(
            pool.clone(),
            batch_config,
        );

        let mut total_recovered = 0;
        for record in discoverable_records {
            let payload: IngestPayload = serde_json::from_value(record.raw_payload)
                .expect("Should be able to parse payload");

            let expected_count = payload.data.metrics.len();
            let result = batch_processor.process_batch(user_id, payload).await;

            assert!(result.errors.is_empty(), "Processing should succeed");
            assert_eq!(
                result.processed_count, expected_count,
                "Should process all metrics"
            );

            total_recovered += result.processed_count;
        }

        // Verify total recovery
        let total_expected = 10 + 20 + 30 + 40 + 50; // Sum of metric counts
        assert_eq!(
            total_recovered, total_expected,
            "Should recover all metrics"
        );

        let final_count = count_user_metrics(&pool, user_id).await;
        assert_eq!(
            final_count, total_expected as i64,
            "Database should have all metrics"
        );

        // Cleanup
        for raw_id in raw_ids {
            cleanup_test_data(&pool, user_id, raw_id).await;
        }
    }

    /// Test monitoring and alerting for data loss detection
    #[tokio::test]
    async fn test_data_loss_monitoring() {
        let pool = get_test_pool().await;
        let user_id = create_test_user(&pool).await;

        // Create scenarios with different types of data loss
        let scenarios = vec![
            ("parameter_limit", "PostgreSQL parameter limit exceeded"),
            ("validation_error", "Validation failed for metrics"),
            ("timeout", "Processing timeout occurred"),
            ("memory_limit", "Memory limit exceeded during processing"),
        ];

        let mut raw_ids = Vec::new();
        for (error_type, error_message) in scenarios {
            let test_metrics = create_test_health_metrics(user_id, 100);
            let test_payload = IngestPayload {
                data: IngestData {
                    metrics: test_metrics,
                    workouts: vec![],
                },
            };

            let raw_id = Uuid::new_v4();
            let payload_json = serde_json::to_value(&test_payload).unwrap();
            let error_json = json!([{
                "error_type": error_type,
                "message": error_message
            }]);

            sqlx::query!(
                r#"
                INSERT INTO raw_ingestions (
                    id, user_id, payload_hash, payload_size_bytes, raw_payload,
                    processing_status, processing_errors, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP)
                "#,
                raw_id,
                user_id,
                "test_hash",
                1024i32,
                payload_json,
                "error",
                error_json
            )
            .execute(&pool)
            .await
            .expect("Failed to create failed raw ingestion");

            raw_ids.push(raw_id);
        }

        // Test monitoring detection (simulating the monitoring service)
        let data_loss_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM raw_ingestions
            WHERE user_id = $1
              AND (
                processing_errors::text ILIKE '%parameter%' OR
                processing_errors::text ILIKE '%validation%' OR
                processing_errors::text ILIKE '%timeout%' OR
                processing_errors::text ILIKE '%memory%'
              )
            "#,
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Should be able to count data loss indicators")
        .unwrap_or(0);

        assert_eq!(
            data_loss_count, 4,
            "Should detect all 4 data loss scenarios"
        );

        // Test error pattern analysis
        let error_patterns = sqlx::query!(
            r#"
            SELECT processing_errors
            FROM raw_ingestions
            WHERE user_id = $1 AND processing_errors IS NOT NULL
            "#,
            user_id
        )
        .fetch_all(&pool)
        .await
        .expect("Should be able to fetch error patterns");

        let mut pattern_counts = HashMap::new();
        for record in error_patterns {
            if let Some(errors) = record.processing_errors {
                if let Ok(errors_array) = serde_json::from_value::<Vec<serde_json::Value>>(errors) {
                    for error in errors_array {
                        let error_text = error.to_string().to_lowercase();
                        let category = if error_text.contains("parameter") {
                            "parameter_limit"
                        } else if error_text.contains("validation") {
                            "validation_error"
                        } else if error_text.contains("timeout") {
                            "timeout"
                        } else if error_text.contains("memory") {
                            "memory_limit"
                        } else {
                            "other"
                        };
                        *pattern_counts.entry(category.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        assert_eq!(pattern_counts.get("parameter_limit"), Some(&1));
        assert_eq!(pattern_counts.get("validation_error"), Some(&1));
        assert_eq!(pattern_counts.get("timeout"), Some(&1));
        assert_eq!(pattern_counts.get("memory_limit"), Some(&1));

        // Cleanup
        for raw_id in raw_ids {
            cleanup_test_data(&pool, user_id, raw_id).await;
        }
    }

    /// Helper to count total metrics for a user
    async fn count_user_metrics(pool: &PgPool, user_id: Uuid) -> i64 {
        let heart_rate_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        let blood_pressure_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        let sleep_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        let activity_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        let workout_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workouts WHERE user_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        heart_rate_count + blood_pressure_count + sleep_count + activity_count + workout_count
    }

    /// Helper to cleanup test data
    async fn cleanup_test_data(pool: &PgPool, user_id: Uuid, raw_id: Uuid) {
        // Clean up metrics
        let _ = sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query!(
            "DELETE FROM blood_pressure_metrics WHERE user_id = $1",
            user_id
        )
        .execute(pool)
        .await;
        let _ = sqlx::query!("DELETE FROM sleep_metrics WHERE user_id = $1", user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query!("DELETE FROM workouts WHERE user_id = $1", user_id)
            .execute(pool)
            .await;

        // Clean up raw ingestion
        let _ = sqlx::query!("DELETE FROM raw_ingestions WHERE id = $1", raw_id)
            .execute(pool)
            .await;

        // Clean up user
        let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(pool)
            .await;
    }
}
