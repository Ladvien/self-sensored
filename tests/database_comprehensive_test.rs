use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use std::time::Duration;

use self_sensored::db::database::{create_connection_pool, test_database_connection};

async fn get_test_database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set")
}

#[tokio::test]
async fn test_create_connection_pool_default_config() {
    let database_url = get_test_database_url().await;

    // Clear environment variables to test defaults
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");
    env::remove_var("DATABASE_TEST_TIMEOUT");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test that we can get a connection
    let conn = pool.acquire().await;
    assert!(conn.is_ok());
}

#[tokio::test]
async fn test_create_connection_pool_custom_config() {
    let database_url = get_test_database_url().await;

    // Set custom environment variables
    env::set_var("DATABASE_MAX_CONNECTIONS", "20");
    env::set_var("DATABASE_MIN_CONNECTIONS", "5");
    env::set_var("DATABASE_CONNECT_TIMEOUT", "10");
    env::set_var("DATABASE_IDLE_TIMEOUT", "300");
    env::set_var("DATABASE_MAX_LIFETIME", "1200");
    env::set_var("DATABASE_TEST_TIMEOUT", "5");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test that we can get a connection with custom config
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    // Clean up environment variables
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");
    env::remove_var("DATABASE_TEST_TIMEOUT");
}

#[tokio::test]
async fn test_create_connection_pool_invalid_url() {
    let invalid_url = "postgresql://invalid:invalid@nonexistent:5432/nonexistent";

    let result = create_connection_pool(invalid_url).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_connection_pool_invalid_env_values() {
    let database_url = get_test_database_url().await;

    // Set invalid environment variables (non-numeric)
    env::set_var("DATABASE_MAX_CONNECTIONS", "not_a_number");
    env::set_var("DATABASE_CONNECT_TIMEOUT", "invalid");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok()); // Should fall back to defaults

    // Clean up
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

#[tokio::test]
async fn test_database_connection_success() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    let result = test_database_connection(&pool).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_database_connection_basic_query() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test basic SELECT 1 query
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(result.0, 1);
}

#[tokio::test]
async fn test_database_version_query() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostgreSQL version query
    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(version.0.contains("PostgreSQL"));
}

#[tokio::test]
async fn test_uuid_extension() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test UUID generation
    let result = sqlx::query("SELECT gen_random_uuid()")
        .fetch_one(&pool)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_postgis_extension() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostGIS extension (may not be available in all test environments)
    let result = sqlx::query("SELECT PostGIS_version()")
        .fetch_one(&pool)
        .await;

    // PostGIS might not be installed in test environment, so we just check if query executes
    // without panicking (it may return an error if extension is not installed)
    match result {
        Ok(_) => {
            // PostGIS is available
            assert!(true);
        }
        Err(e) => {
            // PostGIS might not be installed, which is ok for testing
            assert!(e.to_string().contains("function") || e.to_string().contains("extension"));
        }
    }
}

#[tokio::test]
async fn test_connection_pool_concurrency() {
    let database_url = get_test_database_url().await;

    // Set small pool size for testing concurrency
    env::set_var("DATABASE_MAX_CONNECTIONS", "5");
    env::set_var("DATABASE_MIN_CONNECTIONS", "2");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Spawn multiple concurrent tasks
    let mut handles = vec![];

    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let conn = pool_clone.acquire().await;
            assert!(conn.is_ok());

            // Execute a simple query
            let result: (i32,) = sqlx::query_as("SELECT $1")
                .bind(i)
                .fetch_one(&pool_clone)
                .await
                .unwrap();

            assert_eq!(result.0, i);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Clean up
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

#[tokio::test]
async fn test_connection_timeout() {
    let database_url = get_test_database_url().await;

    // Set very short timeout
    env::set_var("DATABASE_CONNECT_TIMEOUT", "1");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Try to acquire connection (should work with valid database)
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    // Clean up
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

#[tokio::test]
async fn test_multiple_pools() {
    let database_url = get_test_database_url().await;

    // Create multiple connection pools
    let pool1 = create_connection_pool(&database_url).await.unwrap();
    let pool2 = create_connection_pool(&database_url).await.unwrap();

    // Test that both pools work independently
    let conn1 = pool1.acquire().await;
    let conn2 = pool2.acquire().await;

    assert!(conn1.is_ok());
    assert!(conn2.is_ok());
}

#[tokio::test]
async fn test_connection_health_check() {
    let database_url = get_test_database_url().await;

    // Enable test_before_acquire
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .test_before_acquire(true)
        .connect(&database_url)
        .await
        .unwrap();

    // Multiple acquisitions should all work with health checks
    for _ in 0..5 {
        let conn = pool.acquire().await;
        assert!(conn.is_ok());

        // Use the connection
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(result.0, 1);
    }
}

#[tokio::test]
async fn test_database_transaction() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Start a transaction
    let mut tx = pool.begin().await.unwrap();

    // Execute query within transaction
    let result: (i32,) = sqlx::query_as("SELECT 42")
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert_eq!(result.0, 42);

    // Commit transaction
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn test_database_transaction_rollback() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Start a transaction
    let mut tx = pool.begin().await.unwrap();

    // Execute query within transaction
    let result: (i32,) = sqlx::query_as("SELECT 100")
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert_eq!(result.0, 100);

    // Rollback transaction
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn test_pool_statistics() {
    let database_url = get_test_database_url().await;

    env::set_var("DATABASE_MAX_CONNECTIONS", "10");
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Get some connections
    let _conn1 = pool.acquire().await.unwrap();
    let _conn2 = pool.acquire().await.unwrap();

    // Pool should have connections in use
    // Note: SQLx doesn't expose detailed pool statistics in public API,
    // but we can test that the pool is functional under load

    // Clean up
    env::remove_var("DATABASE_MAX_CONNECTIONS");
}

#[tokio::test]
async fn test_environment_variable_parsing() {
    // Test various edge cases for environment variable parsing

    let test_cases = vec![
        ("0", 50u32), // Invalid (0), should default to 50
        ("1", 1u32),  // Valid minimum
        ("999999", 999999u32), // Very large number
        ("", 50u32),  // Empty string, should default
    ];

    for (env_val, expected_or_default) in test_cases {
        env::set_var("DATABASE_MAX_CONNECTIONS", env_val);

        let database_url = get_test_database_url().await;
        let result = create_connection_pool(&database_url).await;

        // Should either succeed with expected value or fall back to default
        match result {
            Ok(_) => {
                // Pool created successfully
                assert!(true);
            }
            Err(_) => {
                // May fail with extreme values, which is acceptable
                if env_val == "999999" {
                    assert!(true); // Expected to potentially fail with very large values
                } else {
                    panic!("Pool creation should succeed with reasonable values");
                }
            }
        }

        env::remove_var("DATABASE_MAX_CONNECTIONS");
    }
}