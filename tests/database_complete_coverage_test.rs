use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use std::time::Duration;

use self_sensored::db::database::{create_connection_pool, test_database_connection, update_db_pool_metrics};

async fn get_test_database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set")
}

#[tokio::test]
async fn test_create_connection_pool_with_all_env_vars() {
    let database_url = get_test_database_url().await;

    // Set all possible environment variables
    env::set_var("DATABASE_MAX_CONNECTIONS", "15");
    env::set_var("DATABASE_MIN_CONNECTIONS", "3");
    env::set_var("DATABASE_CONNECT_TIMEOUT", "8");
    env::set_var("DATABASE_IDLE_TIMEOUT", "400");
    env::set_var("DATABASE_MAX_LIFETIME", "2000");
    env::set_var("DATABASE_TEST_TIMEOUT", "4");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test basic functionality
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    // Test query execution
    let result: (i32,) = sqlx::query_as("SELECT 42")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result.0, 42);

    // Clean up environment variables
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");
    env::remove_var("DATABASE_TEST_TIMEOUT");
}

#[tokio::test]
async fn test_create_connection_pool_default_values() {
    let database_url = get_test_database_url().await;

    // Clear all environment variables to test defaults
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");
    env::remove_var("DATABASE_TEST_TIMEOUT");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test that default configuration works
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    // Test multiple connections can be acquired
    let conn1 = pool.acquire().await;
    let conn2 = pool.acquire().await;
    assert!(conn1.is_ok());
    assert!(conn2.is_ok());
}

#[tokio::test]
async fn test_create_connection_pool_edge_case_env_values() {
    let database_url = get_test_database_url().await;

    // Test with various edge case values
    let test_cases = vec![
        // (max_conn, min_conn, connect_timeout, expected_success)
        ("1", "1", "1", true),      // Minimum values
        ("0", "0", "0", true),      // Zero values (should default)
        ("abc", "def", "xyz", true), // Non-numeric (should default)
        ("", "", "", true),         // Empty strings (should default)
        ("9999", "1", "30", true),  // Very large max connections
    ];

    for (max_conn, min_conn, timeout, expected_success) in test_cases {
        env::set_var("DATABASE_MAX_CONNECTIONS", max_conn);
        env::set_var("DATABASE_MIN_CONNECTIONS", min_conn);
        env::set_var("DATABASE_CONNECT_TIMEOUT", timeout);

        let pool_result = create_connection_pool(&database_url).await;

        if expected_success {
            assert!(pool_result.is_ok(), "Failed with values: max={}, min={}, timeout={}", max_conn, min_conn, timeout);

            if let Ok(pool) = pool_result {
                // Test that the pool actually works
                let conn = pool.acquire().await;
                assert!(conn.is_ok());
            }
        } else {
            assert!(pool_result.is_err());
        }

        // Clean up
        env::remove_var("DATABASE_MAX_CONNECTIONS");
        env::remove_var("DATABASE_MIN_CONNECTIONS");
        env::remove_var("DATABASE_CONNECT_TIMEOUT");
    }
}

#[tokio::test]
async fn test_create_connection_pool_invalid_urls() {
    let invalid_urls = vec![
        "invalid_url",
        "postgresql://",
        "postgresql://invalid:password@",
        "postgresql://user:pass@nonexistent:5432/db",
        "",
        "not_a_url_at_all",
        "postgresql://user:pass@localhost:99999/db", // Invalid port
    ];

    for invalid_url in invalid_urls {
        let result = create_connection_pool(invalid_url).await;
        assert!(result.is_err(), "Should have failed for URL: {}", invalid_url);
    }
}

#[tokio::test]
async fn test_database_connection_comprehensive() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test the comprehensive connection test function
    let result = test_database_connection(&pool).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_database_basic_queries() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test basic SELECT 1
    let result: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result.0, 1);

    // Test SELECT with parameters
    let result: (i32,) = sqlx::query_as("SELECT $1")
        .bind(42)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result.0, 42);

    // Test multiple parameter query
    let result: (i32, String) = sqlx::query_as("SELECT $1, $2")
        .bind(123)
        .bind("test_string")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(result.0, 123);
    assert_eq!(result.1, "test_string");
}

#[tokio::test]
async fn test_database_version_and_extensions() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostgreSQL version query
    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(version.0.contains("PostgreSQL"));

    // Test current database name
    let db_name: (String,) = sqlx::query_as("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!db_name.0.is_empty());

    // Test current user
    let user: (String,) = sqlx::query_as("SELECT current_user")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!user.0.is_empty());
}

#[tokio::test]
async fn test_uuid_extension_comprehensive() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test UUID generation
    let uuid_result = sqlx::query("SELECT gen_random_uuid()")
        .fetch_one(&pool)
        .await;
    assert!(uuid_result.is_ok());

    // Test UUID generation returns different values
    let uuid1: (uuid::Uuid,) = sqlx::query_as("SELECT gen_random_uuid()")
        .fetch_one(&pool)
        .await
        .unwrap();

    let uuid2: (uuid::Uuid,) = sqlx::query_as("SELECT gen_random_uuid()")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_ne!(uuid1.0, uuid2.0);

    // Test UUID validation
    let valid_uuid = uuid::Uuid::new_v4();
    let result: (bool,) = sqlx::query_as("SELECT $1::uuid IS NOT NULL")
        .bind(valid_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(result.0);
}

#[tokio::test]
async fn test_postgis_extension_graceful_handling() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostGIS extension (may or may not be available)
    let result = sqlx::query("SELECT PostGIS_version()")
        .fetch_one(&pool)
        .await;

    match result {
        Ok(_) => {
            // PostGIS is available, test more functions
            let geom_result = sqlx::query("SELECT ST_GeomFromText('POINT(1 1)')")
                .fetch_one(&pool)
                .await;
            assert!(geom_result.is_ok());
        }
        Err(e) => {
            // PostGIS not available, which is acceptable
            let error_string = e.to_string();
            assert!(
                error_string.contains("function") ||
                error_string.contains("extension") ||
                error_string.contains("does not exist")
            );
        }
    }
}

#[tokio::test]
async fn test_connection_pool_concurrency_stress() {
    let database_url = get_test_database_url().await;

    // Set small pool size for stress testing
    env::set_var("DATABASE_MAX_CONNECTIONS", "3");
    env::set_var("DATABASE_MIN_CONNECTIONS", "1");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Spawn many concurrent tasks (more than pool size)
    let handles: Vec<_> = (0..20)
        .map(|i| {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                // Hold connection for a short time
                let conn = pool_clone.acquire().await.unwrap();

                // Execute a query that takes some time
                let result: (i32,) = sqlx::query_as("SELECT $1")
                    .bind(i)
                    .fetch_one(&pool_clone)
                    .await
                    .unwrap();

                // Small delay to increase contention
                tokio::time::sleep(Duration::from_millis(10)).await;

                result.0
            })
        })
        .collect();

    // Wait for all tasks to complete
    let results = futures::future::join_all(handles).await;

    // Verify all tasks completed successfully
    for (i, result) in results.into_iter().enumerate() {
        let value = result.unwrap();
        assert_eq!(value, i as i32);
    }

    // Clean up
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

#[tokio::test]
async fn test_connection_timeout_scenarios() {
    let database_url = get_test_database_url().await;

    // Test very short timeout
    env::set_var("DATABASE_CONNECT_TIMEOUT", "1");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Try to acquire connection (should work with valid database despite short timeout)
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    // Clean up
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

#[tokio::test]
async fn test_multiple_independent_pools() {
    let database_url = get_test_database_url().await;

    // Create multiple independent connection pools
    let pool1 = create_connection_pool(&database_url).await.unwrap();
    let pool2 = create_connection_pool(&database_url).await.unwrap();
    let pool3 = create_connection_pool(&database_url).await.unwrap();

    // Test that all pools work independently
    let futures = vec![
        pool1.acquire(),
        pool2.acquire(),
        pool3.acquire(),
    ];

    let results = futures::future::join_all(futures).await;

    for result in results {
        assert!(result.is_ok());
    }

    // Test executing queries on different pools
    let query_futures = vec![
        sqlx::query_as::<_, (i32,)>("SELECT 1").fetch_one(&pool1),
        sqlx::query_as::<_, (i32,)>("SELECT 2").fetch_one(&pool2),
        sqlx::query_as::<_, (i32,)>("SELECT 3").fetch_one(&pool3),
    ];

    let query_results = futures::future::join_all(query_futures).await;

    assert_eq!(query_results[0].as_ref().unwrap().0, 1);
    assert_eq!(query_results[1].as_ref().unwrap().0, 2);
    assert_eq!(query_results[2].as_ref().unwrap().0, 3);
}

#[tokio::test]
async fn test_connection_health_checks() {
    let database_url = get_test_database_url().await;

    // Create pool with test_before_acquire enabled
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .test_before_acquire(true)
        .connect(&database_url)
        .await
        .unwrap();

    // Multiple acquisitions with health checks
    for i in 0..10 {
        let conn = pool.acquire().await;
        assert!(conn.is_ok());

        // Use the connection
        let result: (i32,) = sqlx::query_as("SELECT $1")
            .bind(i)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(result.0, i);
    }
}

#[tokio::test]
async fn test_database_transactions() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test successful transaction
    let mut tx = pool.begin().await.unwrap();

    let result: (i32,) = sqlx::query_as("SELECT 42")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(result.0, 42);

    tx.commit().await.unwrap();

    // Test transaction rollback
    let mut tx = pool.begin().await.unwrap();

    let result: (i32,) = sqlx::query_as("SELECT 100")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(result.0, 100);

    tx.rollback().await.unwrap();

    // Test nested transactions (savepoints)
    let mut tx = pool.begin().await.unwrap();

    // SQLx doesn't support nested transactions directly
    // Instead, we can use savepoint SQL commands
    sqlx::query("SAVEPOINT sp1")
        .execute(&mut *tx)
        .await
        .unwrap();

    let _result: (i32,) = sqlx::query_as("SELECT 999")
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    sqlx::query("ROLLBACK TO SAVEPOINT sp1")
        .execute(&mut *tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();
}

#[tokio::test]
async fn test_update_db_pool_metrics() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test metrics update with no connections in use
    update_db_pool_metrics(&pool);

    // Test metrics update with connections in use
    let _conn1 = pool.acquire().await.unwrap();
    let _conn2 = pool.acquire().await.unwrap();

    update_db_pool_metrics(&pool);

    // Test with maximum connections
    let mut connections = Vec::new();

    // Try to acquire connections up to pool limit
    for _ in 0..5 {
        if let Some(conn) = pool.try_acquire() {
            connections.push(conn);
        } else {
            break;
        }
    }

    update_db_pool_metrics(&pool);

    // Drop connections to release them back to pool
    drop(connections);

    // Final metrics update
    update_db_pool_metrics(&pool);
}

#[tokio::test]
async fn test_pool_statistics_and_monitoring() {
    let database_url = get_test_database_url().await;

    env::set_var("DATABASE_MAX_CONNECTIONS", "10");
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Get initial pool stats
    let initial_size = pool.size() as usize;
    let initial_idle = pool.num_idle();

    assert!(initial_size <= 10);
    assert!(initial_idle <= initial_size);

    // Acquire some connections
    let mut connections = Vec::new();
    for _ in 0..5 {
        if let Some(conn) = pool.try_acquire() {
            connections.push(conn);
        }
    }

    // Check stats after acquiring connections
    let new_idle = pool.num_idle();
    assert!(new_idle <= initial_idle);

    // Test pool utilization calculation (similar to update_db_pool_metrics)
    let size = pool.size() as usize;
    let idle = pool.num_idle();

    if size > 0 {
        let utilization = ((size.saturating_sub(idle)) as f64 / size as f64) * 100.0;
        assert!(utilization >= 0.0);
        assert!(utilization <= 100.0);
    }

    // Clean up
    drop(connections);
    env::remove_var("DATABASE_MAX_CONNECTIONS");
}

#[tokio::test]
async fn test_database_error_handling() {
    let database_url = get_test_database_url().await;
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test syntax error
    let syntax_error = sqlx::query("SELECT INVALID SYNTAX")
        .fetch_one(&pool)
        .await;
    assert!(syntax_error.is_err());

    // Test invalid table access
    let table_error = sqlx::query("SELECT * FROM nonexistent_table")
        .fetch_one(&pool)
        .await;
    assert!(table_error.is_err());

    // Test invalid parameter count
    let param_error = sqlx::query("SELECT $1, $2")
        .bind(1) // Missing second parameter
        .fetch_one(&pool)
        .await;
    assert!(param_error.is_err());

    // Test type mismatch
    let type_error = sqlx::query_as::<_, (String,)>("SELECT 42") // Trying to cast int to string
        .fetch_one(&pool)
        .await;
    assert!(type_error.is_err());
}

#[tokio::test]
async fn test_connection_pool_cleanup() {
    let database_url = get_test_database_url().await;

    {
        // Create pool in limited scope
        let pool = create_connection_pool(&database_url).await.unwrap();

        // Use the pool
        let _result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        // Pool should be dropped when it goes out of scope
    }

    // Create a new pool to verify cleanup worked
    let new_pool = create_connection_pool(&database_url).await.unwrap();
    let _result: (i32,) = sqlx::query_as("SELECT 2")
        .fetch_one(&new_pool)
        .await
        .unwrap();
}

#[test]
fn test_environment_variable_parsing_edge_cases() {
    // Test parsing behavior without actually connecting to database

    let test_cases = vec![
        ("0", 50u32),       // Zero should default to 50
        ("1", 1u32),        // Minimum valid value
        ("65535", 65535u32), // Maximum reasonable value
        ("", 50u32),        // Empty should default
        ("abc", 50u32),     // Non-numeric should default
        ("-5", 50u32),      // Negative should default
        ("999999999999999999999", 50u32), // Overflow should default
    ];

    for (input, expected_default) in test_cases {
        env::set_var("TEST_PARSE_VAR", input);

        let parsed = env::var("TEST_PARSE_VAR")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<u32>()
            .unwrap_or(50);

        // When parse fails, it should default to 50
        if input == "0" || input == "" || input == "abc" || input == "-5" || input.len() > 10 {
            assert_eq!(parsed, expected_default);
        } else {
            assert_eq!(parsed, input.parse::<u32>().unwrap_or(expected_default));
        }

        env::remove_var("TEST_PARSE_VAR");
    }
}