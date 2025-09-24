use self_sensored::handlers::data_loader::{DataLoaderConfig, LazyDataLoader, SupportLevel};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(test)]
mod data_loader_db_tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/health_export_test".to_string());

        let pool = PgPool::connect(&database_url).await
            .expect("Failed to connect to test database");

        // Create test table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS data_mappings (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                healthkit_identifier VARCHAR(255) UNIQUE NOT NULL,
                description TEXT NOT NULL,
                support_level VARCHAR(50) NOT NULL CHECK (support_level IN ('fully_supported', 'partial', 'planned', 'not_supported')),
                category VARCHAR(100) NOT NULL,
                notes TEXT,
                is_active BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .expect("Failed to create test table");

        // Clean up any existing test data
        sqlx::query("DELETE FROM data_mappings WHERE healthkit_identifier LIKE 'TEST_%'")
            .execute(&pool)
            .await
            .expect("Failed to clean up test data");

        pool
    }

    async fn insert_test_mapping(pool: &PgPool, identifier: &str, description: &str, support_level: &str, category: &str) {
        sqlx::query!(
            "INSERT INTO data_mappings (healthkit_identifier, description, support_level, category) VALUES ($1, $2, $3, $4)",
            identifier,
            description,
            support_level,
            category
        )
        .execute(pool)
        .await
        .expect("Failed to insert test mapping");
    }

    #[tokio::test]
    async fn test_load_from_database_success() {
        let pool = setup_test_db().await;

        // Insert test data
        insert_test_mapping(&pool, "TEST_HeartRate", "Test Heart Rate", "fully_supported", "HEART").await;
        insert_test_mapping(&pool, "TEST_StepCount", "Test Step Count", "partial", "ACTIVITY").await;
        insert_test_mapping(&pool, "TEST_BloodGlucose", "Test Blood Glucose", "planned", "METABOLIC").await;

        // Create data loader with database
        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "data_mappings".to_string(),
        };

        let loader = LazyDataLoader::new(config)
            .with_database(pool.clone());

        // Load from database
        let mappings = loader.load_data().await.expect("Failed to load mappings");

        // Verify test mappings exist
        assert!(mappings.contains_key("TEST_HeartRate"));
        assert!(mappings.contains_key("TEST_StepCount"));
        assert!(mappings.contains_key("TEST_BloodGlucose"));

        // Verify support levels
        let heart_rate = mappings.get("TEST_HeartRate").unwrap();
        assert_eq!(heart_rate.support_level, SupportLevel::FullySupported);
        assert_eq!(heart_rate.description, "Test Heart Rate");
        assert_eq!(heart_rate.category, "HEART");

        let step_count = mappings.get("TEST_StepCount").unwrap();
        assert_eq!(step_count.support_level, SupportLevel::Partial);

        let blood_glucose = mappings.get("TEST_BloodGlucose").unwrap();
        assert_eq!(blood_glucose.support_level, SupportLevel::Planned);
    }

    #[tokio::test]
    async fn test_fallback_to_static_when_no_database() {
        // Create data loader without database
        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "data_mappings".to_string(),
        };

        let loader = LazyDataLoader::new(config);

        // Should fall back to static data
        let mappings = loader.load_data().await.expect("Failed to load mappings");

        // Verify some core static mappings exist
        assert!(mappings.contains_key("HKQuantityTypeIdentifierStepCount"));
        assert!(mappings.contains_key("HKQuantityTypeIdentifierHeartRate"));
    }

    #[tokio::test]
    async fn test_database_with_redis_caching() {
        let pool = setup_test_db().await;

        // Insert test data
        insert_test_mapping(&pool, "TEST_CachedMetric", "Test Cached Metric", "fully_supported", "TEST").await;

        // Create mock cache service
        let cache_service = Arc::new(self_sensored::services::cache::CacheService::new(
            "redis://localhost:6379".to_string(),
            "test".to_string(),
        ));

        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "data_mappings".to_string(),
        };

        let loader = LazyDataLoader::new(config)
            .with_database(pool.clone())
            .with_redis(cache_service);

        // First load - from database
        let mappings1 = loader.load_data().await.expect("Failed to load mappings");
        assert!(mappings1.contains_key("TEST_CachedMetric"));

        // Second load - should use cache if Redis is available
        let mappings2 = loader.load_data().await.expect("Failed to load mappings");
        assert!(mappings2.contains_key("TEST_CachedMetric"));
    }

    #[tokio::test]
    async fn test_handle_database_error() {
        // Create pool with invalid connection
        let invalid_pool = PgPool::connect("postgres://invalid:invalid@nonexistent:5432/invalid")
            .await
            .unwrap_or_else(|_| {
                // If we can't connect to invalid DB, create a valid pool but we'll simulate error
                setup_test_db().await.unwrap()
            });

        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "nonexistent_table".to_string(),
        };

        let loader = LazyDataLoader::new(config)
            .with_database(invalid_pool);

        // Should fall back to static data on error
        let mappings = loader.load_data().await.expect("Failed to load mappings");

        // Should have static fallback data
        assert!(!mappings.is_empty());
        assert!(mappings.contains_key("HKQuantityTypeIdentifierStepCount"));
    }

    #[tokio::test]
    async fn test_inactive_mappings_filtered() {
        let pool = setup_test_db().await;

        // Insert active and inactive test data
        insert_test_mapping(&pool, "TEST_ActiveMetric", "Active Metric", "fully_supported", "TEST").await;

        // Insert inactive mapping
        sqlx::query!(
            "INSERT INTO data_mappings (healthkit_identifier, description, support_level, category, is_active) VALUES ($1, $2, $3, $4, $5)",
            "TEST_InactiveMetric",
            "Inactive Metric",
            "fully_supported",
            "TEST",
            false
        )
        .execute(&pool)
        .await
        .expect("Failed to insert inactive mapping");

        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "data_mappings".to_string(),
        };

        let loader = LazyDataLoader::new(config)
            .with_database(pool.clone());

        let mappings = loader.load_data().await.expect("Failed to load mappings");

        // Active mapping should exist
        assert!(mappings.contains_key("TEST_ActiveMetric"));

        // Inactive mapping should be filtered out
        assert!(!mappings.contains_key("TEST_InactiveMetric"));
    }

    #[tokio::test]
    async fn test_support_level_mapping() {
        let pool = setup_test_db().await;

        // Insert test data with all support levels
        insert_test_mapping(&pool, "TEST_FullySupported", "Test", "fully_supported", "TEST").await;
        insert_test_mapping(&pool, "TEST_Partial", "Test", "partial", "TEST").await;
        insert_test_mapping(&pool, "TEST_Planned", "Test", "planned", "TEST").await;
        insert_test_mapping(&pool, "TEST_NotSupported", "Test", "not_supported", "TEST").await;

        let config = DataLoaderConfig {
            preload_on_startup: false,
            cache_timeout_secs: 60,
            use_database_source: true,
            database_table: "data_mappings".to_string(),
        };

        let loader = LazyDataLoader::new(config)
            .with_database(pool.clone());

        let mappings = loader.load_data().await.expect("Failed to load mappings");

        // Verify all support levels are mapped correctly
        assert_eq!(mappings.get("TEST_FullySupported").unwrap().support_level, SupportLevel::FullySupported);
        assert_eq!(mappings.get("TEST_Partial").unwrap().support_level, SupportLevel::Partial);
        assert_eq!(mappings.get("TEST_Planned").unwrap().support_level, SupportLevel::Planned);
        assert_eq!(mappings.get("TEST_NotSupported").unwrap().support_level, SupportLevel::NotSupported);
    }
}