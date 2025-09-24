use actix_web::{test as actix_test, web, App};
use chrono::{DateTime, Utc};
use self_sensored::config::BatchConfig;
use self_sensored::db;
use self_sensored::models::MetabolicMetric;
use self_sensored::services::batch_processor::BatchProcessor;
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod metabolic_metrics_tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        // Get test database URL from environment or use default
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/health_export_test".to_string());

        // Create connection pool
        let pool = PgPool::connect(&database_url).await
            .expect("Failed to connect to test database");

        // Clean up any existing test data
        sqlx::query("DELETE FROM metabolic_metrics WHERE source_device LIKE 'test_%'")
            .execute(&pool)
            .await
            .expect("Failed to clean up test data");

        pool
    }

    async fn create_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        let email = format!("test_{}@example.com", user_id);

        sqlx::query!(
            "INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            user_id,
            email
        )
        .execute(pool)
        .await
        .expect("Failed to create test user");

        user_id
    }

    #[actix_web::test]
    async fn test_insert_metabolic_metrics_single() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let config = BatchConfig::from_env();
        let processor = BatchProcessor::new(pool.clone(), config);

        // Create test metabolic metric
        let metric = MetabolicMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now(),
            blood_alcohol_content: Some(0.02),
            insulin_delivery_units: Some(5.0),
            delivery_method: Some("pump".to_string()),
            source_device: Some("test_device".to_string()),
            created_at: Utc::now(),
        };

        // Process metrics
        let result = processor.process_health_metrics(
            user_id,
            vec![],  // heart_rate
            vec![],  // blood_pressure
            vec![],  // sleep
            vec![],  // activity
            vec![],  // body_measurement
            vec![],  // temperature
            vec![],  // respiratory
            vec![],  // blood_glucose
            vec![metric.clone()],  // metabolic
            vec![],  // nutrition
            vec![],  // menstrual
            vec![],  // fertility
            vec![],  // environmental
            vec![],  // audio_exposure
            vec![],  // safety_event
            vec![],  // mindfulness
            vec![],  // mental_health
            vec![],  // symptom
            vec![],  // hygiene
        ).await;

        assert!(result.is_ok(), "Failed to process metabolic metrics: {:?}", result);

        // Verify insertion
        let saved_metric = sqlx::query_as!(
            MetabolicMetric,
            r#"SELECT
                id, user_id, recorded_at, blood_alcohol_content,
                insulin_delivery_units, delivery_method, source_device, created_at
            FROM metabolic_metrics
            WHERE user_id = $1 AND source_device = 'test_device'"#,
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved metric");

        assert_eq!(saved_metric.blood_alcohol_content, Some(0.02));
        assert_eq!(saved_metric.insulin_delivery_units, Some(5.0));
        assert_eq!(saved_metric.delivery_method, Some("pump".to_string()));
    }

    #[actix_web::test]
    async fn test_insert_metabolic_metrics_batch() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let config = BatchConfig::from_env();
        let processor = BatchProcessor::new(pool.clone(), config);

        // Create multiple test metrics
        let mut metrics = Vec::new();
        for i in 0..100 {
            let metric = MetabolicMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now() - chrono::Duration::hours(i),
                blood_alcohol_content: if i % 3 == 0 { Some(0.01) } else { None },
                insulin_delivery_units: if i % 2 == 0 { Some(i as f64) } else { None },
                delivery_method: if i % 2 == 0 { Some("pen".to_string()) } else { None },
                source_device: Some(format!("test_device_{}", i)),
                created_at: Utc::now(),
            };
            metrics.push(metric);
        }

        // Process metrics
        let result = processor.process_health_metrics(
            user_id,
            vec![],  // heart_rate
            vec![],  // blood_pressure
            vec![],  // sleep
            vec![],  // activity
            vec![],  // body_measurement
            vec![],  // temperature
            vec![],  // respiratory
            vec![],  // blood_glucose
            metrics.clone(),  // metabolic
            vec![],  // nutrition
            vec![],  // menstrual
            vec![],  // fertility
            vec![],  // environmental
            vec![],  // audio_exposure
            vec![],  // safety_event
            vec![],  // mindfulness
            vec![],  // mental_health
            vec![],  // symptom
            vec![],  // hygiene
        ).await;

        assert!(result.is_ok(), "Failed to process batch metabolic metrics: {:?}", result);

        // Verify all metrics were inserted
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM metabolic_metrics WHERE user_id = $1 AND source_device LIKE 'test_device_%'",
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count saved metrics");

        assert_eq!(count as usize, metrics.len(), "Not all metrics were saved");
    }

    #[actix_web::test]
    async fn test_metabolic_metrics_conflict_handling() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let config = BatchConfig::from_env();
        let processor = BatchProcessor::new(pool.clone(), config);

        let recorded_at = Utc::now();

        // Create initial metric
        let metric1 = MetabolicMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            blood_alcohol_content: Some(0.03),
            insulin_delivery_units: Some(10.0),
            delivery_method: Some("syringe".to_string()),
            source_device: Some("test_device_1".to_string()),
            created_at: Utc::now(),
        };

        // Process first metric
        let result1 = processor.process_health_metrics(
            user_id,
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![metric1.clone()],  // metabolic
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
        ).await;

        assert!(result1.is_ok());

        // Create conflicting metric with same timestamp
        let metric2 = MetabolicMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,  // Same timestamp
            blood_alcohol_content: Some(0.04),  // Different value
            insulin_delivery_units: None,  // Missing value
            delivery_method: Some("pump".to_string()),  // Different method
            source_device: Some("test_device_2".to_string()),
            created_at: Utc::now(),
        };

        // Process conflicting metric
        let result2 = processor.process_health_metrics(
            user_id,
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![metric2.clone()],  // metabolic
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
        ).await;

        assert!(result2.is_ok());

        // Verify conflict resolution (COALESCE should keep first non-null values)
        let saved_metric = sqlx::query_as!(
            MetabolicMetric,
            r#"SELECT
                id, user_id, recorded_at, blood_alcohol_content,
                insulin_delivery_units, delivery_method, source_device, created_at
            FROM metabolic_metrics
            WHERE user_id = $1 AND recorded_at = $2"#,
            user_id,
            recorded_at
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved metric");

        // COALESCE should update with new non-null values
        assert_eq!(saved_metric.blood_alcohol_content, Some(0.04));  // Updated
        assert_eq!(saved_metric.insulin_delivery_units, Some(10.0));  // Kept original
        assert_eq!(saved_metric.delivery_method, Some("pump".to_string()));  // Updated
    }

    #[actix_web::test]
    async fn test_metabolic_metrics_chunking() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let mut config = BatchConfig::from_env();
        // Set small chunk size to test chunking behavior
        config.metabolic_chunk_size = 10;

        let processor = BatchProcessor::new(pool.clone(), config);

        // Create metrics that will require multiple chunks
        let mut metrics = Vec::new();
        for i in 0..25 {
            let metric = MetabolicMetric {
                id: Uuid::new_v4(),
                user_id,
                recorded_at: Utc::now() - chrono::Duration::minutes(i),
                blood_alcohol_content: None,
                insulin_delivery_units: Some(i as f64),
                delivery_method: Some("pump".to_string()),
                source_device: Some(format!("test_chunk_{}", i)),
                created_at: Utc::now(),
            };
            metrics.push(metric);
        }

        // Process metrics (should be processed in 3 chunks: 10, 10, 5)
        let result = processor.process_health_metrics(
            user_id,
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            metrics.clone(),  // metabolic
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
        ).await;

        assert!(result.is_ok(), "Failed to process chunked metabolic metrics: {:?}", result);

        // Verify all metrics were inserted despite chunking
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM metabolic_metrics WHERE user_id = $1 AND source_device LIKE 'test_chunk_%'",
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count saved metrics");

        assert_eq!(count as usize, 25, "Not all chunked metrics were saved");
    }

    #[actix_web::test]
    async fn test_metabolic_metrics_validation() {
        let pool = setup_test_db().await;
        let user_id = create_test_user(&pool).await;

        let config = BatchConfig::from_env();
        let processor = BatchProcessor::new(pool.clone(), config);

        // Create metric with valid edge case values
        let metric = MetabolicMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now(),
            blood_alcohol_content: Some(0.0),  // Minimum valid
            insulin_delivery_units: Some(100.0),  // Maximum valid
            delivery_method: Some("patch".to_string()),
            source_device: Some("test_validation".to_string()),
            created_at: Utc::now(),
        };

        // Process metric
        let result = processor.process_health_metrics(
            user_id,
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
            vec![metric.clone()],  // metabolic
            vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![], vec![],
        ).await;

        assert!(result.is_ok(), "Failed to process edge case metabolic metrics: {:?}", result);

        // Verify metric was saved with edge case values
        let saved_metric = sqlx::query_as!(
            MetabolicMetric,
            r#"SELECT
                id, user_id, recorded_at, blood_alcohol_content,
                insulin_delivery_units, delivery_method, source_device, created_at
            FROM metabolic_metrics
            WHERE user_id = $1 AND source_device = 'test_validation'"#,
            user_id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved metric");

        assert_eq!(saved_metric.blood_alcohol_content, Some(0.0));
        assert_eq!(saved_metric.insulin_delivery_units, Some(100.0));
    }
}