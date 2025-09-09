use sqlx::{PgPool, Row};
use std::env;

#[cfg(test)]
mod schema_tests {
    use super::*;

    async fn get_test_pool() -> PgPool {
        dotenv::dotenv().ok();
        let database_url =
            env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set for testing");

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_database_connection() {
        let pool = get_test_pool().await;

        // Test basic connection
        let result = sqlx::query("SELECT 1 as test")
            .fetch_one(&pool)
            .await
            .expect("Failed to execute test query");

        let test_value: i32 = result.get("test");
        assert_eq!(test_value, 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_required_extensions() {
        let pool = get_test_pool().await;

        // Test UUID extension
        let uuid_result = sqlx::query("SELECT gen_random_uuid() as test_uuid")
            .fetch_one(&pool)
            .await
            .expect("UUID extension not available");

        let uuid_val: uuid::Uuid = uuid_result.get("test_uuid");
        println!("✓ UUID extension working: {uuid_val}");

        // Test PostGIS extension
        let postgis_result = sqlx::query("SELECT PostGIS_version() as version")
            .fetch_one(&pool)
            .await
            .expect("PostGIS extension not available");

        let postgis_version: String = postgis_result.get("version");
        assert!(!postgis_version.is_empty());
        println!("✓ PostGIS extension working: {postgis_version}");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_core_tables_exist() {
        let pool = get_test_pool().await;

        let core_tables = vec![
            "users",
            "api_keys",
            "audit_log",
            "raw_ingestions",
            "heart_rate_metrics",
            "blood_pressure_metrics",
            "sleep_metrics",
            "activity_metrics",
            "workouts",
        ];

        for table_name in core_tables {
            let query = "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = $1)".to_string();

            let result = sqlx::query(&query)
                .bind(table_name)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("Failed to check existence of table: {table_name}"));

            let exists: bool = result.get("exists");
            assert!(exists, "Table {table_name} does not exist");
            println!("✓ Table {table_name} exists");
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_table_constraints() {
        let pool = get_test_pool().await;

        // Test users table constraints
        let users_constraints = sqlx::query(
            "SELECT constraint_name FROM information_schema.table_constraints 
             WHERE table_name = 'users' AND constraint_type IN ('PRIMARY KEY', 'UNIQUE')",
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to get users constraints");

        assert!(
            !users_constraints.is_empty(),
            "Users table should have constraints"
        );
        println!("✓ Users table has {} constraints", users_constraints.len());

        // Test api_keys table constraints
        let api_keys_constraints = sqlx::query(
            "SELECT constraint_name FROM information_schema.table_constraints 
             WHERE table_name = 'api_keys' AND constraint_type IN ('PRIMARY KEY', 'FOREIGN KEY', 'UNIQUE')"
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to get api_keys constraints");

        assert!(
            !api_keys_constraints.is_empty(),
            "API keys table should have constraints"
        );
        println!(
            "✓ API keys table has {} constraints",
            api_keys_constraints.len()
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_indexes_exist() {
        let pool = get_test_pool().await;

        // Check that key indexes exist
        let expected_indexes = vec![
            "idx_users_email",
            "idx_api_keys_user_id",
            "idx_api_keys_key_hash",
            "idx_raw_ingestions_user_id",
            "idx_heart_rate_user_id",
            "idx_workouts_route_geometry",
        ];

        for index_name in expected_indexes {
            let query = "SELECT EXISTS (SELECT FROM pg_indexes WHERE indexname = $1)";

            let result = sqlx::query(query)
                .bind(index_name)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| panic!("Failed to check existence of index: {index_name}"));

            let exists: bool = result.get("exists");
            assert!(exists, "Index {index_name} does not exist");
            println!("✓ Index {index_name} exists");
        }

        pool.close().await;
    }

    #[tokio::test]
    async fn test_basic_data_operations() {
        let pool = get_test_pool().await;

        // Start a transaction for cleanup
        let mut tx = pool.begin().await.expect("Failed to begin transaction");

        // Test inserting a user
        let user_result =
            sqlx::query("INSERT INTO users (email, full_name) VALUES ($1, $2) RETURNING id, email")
                .bind("test@example.com")
                .bind("Test User")
                .fetch_one(&mut *tx)
                .await
                .expect("Failed to insert test user");

        let user_id: uuid::Uuid = user_result.get("id");
        let user_email: String = user_result.get("email");

        assert_eq!(user_email, "test@example.com");
        println!("✓ Successfully inserted user: {user_id}");

        // Test inserting an API key
        let api_key_result = sqlx::query(
            "INSERT INTO api_keys (user_id, name, key_hash) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(user_id)
        .bind("Test API Key")
        .bind("test_hash_12345")
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to insert test API key");

        let api_key_id: uuid::Uuid = api_key_result.get("id");
        println!("✓ Successfully inserted API key: {api_key_id}");

        // Test inserting heart rate data
        let heart_rate_result = sqlx::query(
            "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate) 
             VALUES ($1, NOW(), $2) RETURNING id",
        )
        .bind(user_id)
        .bind(72)
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to insert test heart rate");

        let heart_rate_id: uuid::Uuid = heart_rate_result.get("id");
        println!(
            "✓ Successfully inserted heart rate metric: {heart_rate_id}"
        );

        // Rollback transaction to clean up test data
        tx.rollback().await.expect("Failed to rollback transaction");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_postgis_geometry() {
        let pool = get_test_pool().await;

        let mut tx = pool.begin().await.expect("Failed to begin transaction");

        // Test PostGIS geometry operations
        let _geom_test = sqlx::query(
            "SELECT ST_GeomFromText('LINESTRING(0 0, 1 1, 2 2)', 4326) as test_geometry",
        )
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to create test geometry");

        println!("✓ PostGIS geometry creation successful");

        // Test inserting a user for workout test
        let user_result =
            sqlx::query("INSERT INTO users (email, full_name) VALUES ($1, $2) RETURNING id")
                .bind("workout_test@example.com")
                .bind("Workout Test User")
                .fetch_one(&mut *tx)
                .await
                .expect("Failed to insert workout test user");

        let user_id: uuid::Uuid = user_result.get("id");

        // Test inserting workout with geometry
        let workout_result = sqlx::query(
            "INSERT INTO workouts (user_id, workout_type, started_at, ended_at, duration_minutes, route_geometry)
             VALUES ($1, $2, NOW() - INTERVAL '1 hour', NOW(), 60, ST_GeomFromText('LINESTRING(-122.4194 37.7749, -122.4094 37.7849)', 4326))
             RETURNING id"
        )
        .bind(user_id)
        .bind("running")
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to insert workout with geometry");

        let workout_id: uuid::Uuid = workout_result.get("id");
        println!(
            "✓ Successfully inserted workout with PostGIS geometry: {workout_id}"
        );

        tx.rollback().await.expect("Failed to rollback transaction");
        pool.close().await;
    }
}
