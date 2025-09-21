/*!
# Database Connection Pool Test Suite

Comprehensive testing for database connection management, pooling strategies,
and connection health monitoring.

## Test Coverage

1. **Connection Pool Creation**: Environment configuration, pool sizing, timeouts
2. **Connection Health Checks**: PostgreSQL version, extensions, connectivity
3. **Pool Metrics**: Connection tracking, utilization monitoring, alerts
4. **Connection Lifecycle**: Acquire, release, cleanup, reconnection
5. **Error Handling**: Connection failures, timeout scenarios, recovery
6. **Performance**: Pool efficiency, connection reuse, scaling behavior
7. **Extension Verification**: PostGIS, UUID, crypto extensions

## Key Features

- Real database connection testing with SQLx
- Environment variable configuration validation
- Pool metrics and monitoring verification
- Connection failure simulation and recovery
- Performance benchmarking for pool operations
- Extension availability checking with graceful fallbacks
*/

use dotenvy;
use self_sensored::db::database::{
    create_connection_pool, test_database_connection, update_db_pool_metrics,
};
use sqlx::PgPool;
use std::{env, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

/// Test database URL with fallback to environment variable
const TEST_DATABASE_URL: &str = "postgresql://postgres:password@localhost:5432/health_export_test";

/// Get test database URL from environment or use default
fn get_test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| TEST_DATABASE_URL.to_string())
}

// ============================================================================
// CONNECTION POOL CREATION TESTS
// ============================================================================

#[tokio::test]
async fn test_connection_pool_creation_with_defaults() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Clear environment variables to test defaults
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test that pool was created with default values
    assert!(pool.size() >= 10); // Default min_connections
    assert!(pool.size() <= 50); // Default max_connections

    // Test basic connectivity
    let result = sqlx::query("SELECT 1 as test").fetch_one(&pool).await;
    assert!(result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_connection_pool_creation_with_custom_config() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Set custom environment variables
    env::set_var("DATABASE_MAX_CONNECTIONS", "20");
    env::set_var("DATABASE_MIN_CONNECTIONS", "5");
    env::set_var("DATABASE_CONNECT_TIMEOUT", "3");
    env::set_var("DATABASE_IDLE_TIMEOUT", "300");
    env::set_var("DATABASE_MAX_LIFETIME", "900");
    env::set_var("DATABASE_TEST_TIMEOUT", "2");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Test that pool respects custom configuration
    assert!(pool.size() >= 5);  // Custom min_connections
    assert!(pool.size() <= 20); // Custom max_connections

    // Test basic connectivity
    let result = sqlx::query("SELECT 1 as test").fetch_one(&pool).await;
    assert!(result.is_ok());

    pool.close().await;

    // Clean up environment variables
    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
    env::remove_var("DATABASE_IDLE_TIMEOUT");
    env::remove_var("DATABASE_MAX_LIFETIME");
    env::remove_var("DATABASE_TEST_TIMEOUT");
}

#[tokio::test]
async fn test_connection_pool_invalid_config() {
    dotenvy::dotenv().ok();

    // Test with invalid database URL
    let invalid_url = "postgresql://invalid:invalid@nonexistent:5432/invalid";

    // Set a short timeout to fail quickly
    env::set_var("DATABASE_CONNECT_TIMEOUT", "1");

    let pool_result = create_connection_pool(invalid_url).await;
    assert!(pool_result.is_err());

    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

#[tokio::test]
async fn test_connection_pool_environment_parsing() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Test invalid numeric values (should fall back to defaults)
    env::set_var("DATABASE_MAX_CONNECTIONS", "invalid");
    env::set_var("DATABASE_MIN_CONNECTIONS", "also_invalid");

    let pool = create_connection_pool(&database_url).await;
    assert!(pool.is_ok());

    let pool = pool.unwrap();

    // Should fall back to defaults despite invalid config
    assert!(pool.size() >= 10); // Default min_connections

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

// ============================================================================
// DATABASE HEALTH CHECKS
// ============================================================================

#[tokio::test]
async fn test_database_connection_health_checks() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test comprehensive health checks
    let health_result = test_database_connection(&pool).await;
    assert!(health_result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_postgresql_version_check() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostgreSQL version query
    let version_result = sqlx::query_as::<_, (String,)>("SELECT version()")
        .fetch_one(&pool)
        .await;

    assert!(version_result.is_ok());
    let version = version_result.unwrap().0;
    assert!(version.contains("PostgreSQL"));

    pool.close().await;
}

#[tokio::test]
async fn test_uuid_extension_availability() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test UUID extension
    let uuid_result = sqlx::query("SELECT gen_random_uuid()").fetch_one(&pool).await;

    // UUID extension should be available for health export database
    assert!(uuid_result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_postgis_extension_availability() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test PostGIS extension
    let postgis_result = sqlx::query("SELECT PostGIS_version()").fetch_one(&pool).await;

    // PostGIS should be available for workout route storage
    assert!(postgis_result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_extension_graceful_degradation() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test non-existent extension (should fail gracefully)
    let fake_extension_result = sqlx::query("SELECT nonexistent_extension()")
        .fetch_one(&pool)
        .await;

    assert!(fake_extension_result.is_err());

    // Pool should still be usable for basic operations
    let basic_query = sqlx::query("SELECT 1").fetch_one(&pool).await;
    assert!(basic_query.is_ok());

    pool.close().await;
}

// ============================================================================
// POOL METRICS AND MONITORING
// ============================================================================

#[tokio::test]
async fn test_connection_pool_metrics_collection() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Create pool with known configuration
    env::set_var("DATABASE_MAX_CONNECTIONS", "10");
    env::set_var("DATABASE_MIN_CONNECTIONS", "2");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test metrics collection
    let initial_size = pool.size();
    let initial_idle = pool.num_idle();

    assert!(initial_size >= 2);
    assert!(initial_idle <= initial_size);

    // Update metrics (this function logs metrics and updates Prometheus)
    update_db_pool_metrics(&pool);

    // Acquire some connections to change pool state
    let _conn1 = pool.acquire().await.unwrap();
    let _conn2 = pool.acquire().await.unwrap();

    // Check that metrics reflect changed state
    let new_idle_count = pool.num_idle();
    assert!(new_idle_count < initial_idle || initial_idle == 0);

    // Update metrics again
    update_db_pool_metrics(&pool);

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

#[tokio::test]
async fn test_high_utilization_warning_logic() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Create small pool to easily trigger high utilization
    env::set_var("DATABASE_MAX_CONNECTIONS", "3");
    env::set_var("DATABASE_MIN_CONNECTIONS", "3");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Acquire most connections to trigger high utilization
    let _conn1 = pool.acquire().await.unwrap();
    let _conn2 = pool.acquire().await.unwrap();

    // This should trigger high utilization warning in logs
    update_db_pool_metrics(&pool);

    // Verify pool is still functional
    let test_query = sqlx::query("SELECT 1").fetch_one(&pool).await;
    assert!(test_query.is_ok());

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

// ============================================================================
// CONNECTION LIFECYCLE TESTING
// ============================================================================

#[tokio::test]
async fn test_connection_acquire_release_cycle() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    let initial_idle = pool.num_idle();

    // Acquire connection
    let conn = pool.acquire().await;
    assert!(conn.is_ok());

    let new_idle = pool.num_idle();
    assert!(new_idle <= initial_idle);

    // Connection is automatically released when dropped
    drop(conn);

    // Give pool time to reclaim connection
    tokio::time::sleep(Duration::from_millis(100)).await;

    let final_idle = pool.num_idle();
    assert!(final_idle >= new_idle);

    pool.close().await;
}

#[tokio::test]
async fn test_connection_timeout_behavior() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Set very short timeout
    env::set_var("DATABASE_CONNECT_TIMEOUT", "1");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test that operations respect timeout
    let start_time = std::time::Instant::now();

    let timeout_result = timeout(
        Duration::from_secs(5),
        sqlx::query("SELECT pg_sleep(10)").fetch_one(&pool)
    ).await;

    let elapsed = start_time.elapsed();

    // Should timeout before 10 seconds
    assert!(elapsed < Duration::from_secs(6));
    assert!(timeout_result.is_err()); // Timeout occurred

    pool.close().await;

    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

#[tokio::test]
async fn test_concurrent_connection_usage() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    env::set_var("DATABASE_MAX_CONNECTIONS", "5");
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Spawn multiple concurrent tasks
    let tasks: Vec<_> = (0..3).map(|i| {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let result = sqlx::query("SELECT $1 as task_id")
                .bind(i)
                .fetch_one(&pool_clone)
                .await;
            result.is_ok()
        })
    }).collect();

    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;

    // All tasks should succeed
    for result in results {
        assert!(result.unwrap());
    }

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
}

// ============================================================================
// ERROR HANDLING AND RECOVERY
// ============================================================================

#[tokio::test]
async fn test_connection_recovery_after_failure() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Test that pool can recover from temporary connection issues

    // First, ensure pool is working
    let initial_result = sqlx::query("SELECT 1").fetch_one(&pool).await;
    assert!(initial_result.is_ok());

    // Simulate connection issue by running invalid query
    let invalid_result = sqlx::query("SELECT FROM invalid_table").fetch_one(&pool).await;
    assert!(invalid_result.is_err());

    // Pool should still work for valid queries
    let recovery_result = sqlx::query("SELECT 2").fetch_one(&pool).await;
    assert!(recovery_result.is_ok());

    pool.close().await;
}

#[tokio::test]
async fn test_pool_exhaustion_behavior() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    // Create very small pool
    env::set_var("DATABASE_MAX_CONNECTIONS", "2");
    env::set_var("DATABASE_MIN_CONNECTIONS", "2");
    env::set_var("DATABASE_CONNECT_TIMEOUT", "2");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Acquire all connections
    let _conn1 = pool.acquire().await.unwrap();
    let _conn2 = pool.acquire().await.unwrap();

    // Try to acquire another connection (should timeout)
    let timeout_result = timeout(
        Duration::from_secs(3),
        pool.acquire()
    ).await;

    // Should timeout since pool is exhausted
    assert!(timeout_result.is_err());

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
    env::remove_var("DATABASE_CONNECT_TIMEOUT");
}

// ============================================================================
// PERFORMANCE BENCHMARKING
// ============================================================================

#[tokio::test]
async fn test_connection_pool_performance() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    env::set_var("DATABASE_MAX_CONNECTIONS", "10");
    let pool = create_connection_pool(&database_url).await.unwrap();

    let start_time = std::time::Instant::now();
    let query_count = 100;

    // Run multiple queries to test pool efficiency
    for i in 0..query_count {
        let result = sqlx::query("SELECT $1 as iteration")
            .bind(i)
            .fetch_one(&pool)
            .await;
        assert!(result.is_ok());
    }

    let duration = start_time.elapsed();
    let queries_per_second = query_count as f64 / duration.as_secs_f64();

    println!("Pool performance: {:.2} queries/second", queries_per_second);

    // Performance should be reasonable (at least 50 queries/second)
    assert!(queries_per_second > 50.0);

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
}

#[tokio::test]
async fn test_connection_reuse_efficiency() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    env::set_var("DATABASE_MAX_CONNECTIONS", "3");
    env::set_var("DATABASE_MIN_CONNECTIONS", "3");

    let pool = create_connection_pool(&database_url).await.unwrap();

    // Warm up the pool
    for _ in 0..5 {
        let _result = sqlx::query("SELECT 1").fetch_one(&pool).await.unwrap();
    }

    let initial_size = pool.size();

    // Run more queries - pool size should remain stable (connections reused)
    for _ in 0..20 {
        let _result = sqlx::query("SELECT 1").fetch_one(&pool).await.unwrap();
    }

    let final_size = pool.size();

    // Pool should not have grown significantly (efficient reuse)
    assert_eq!(initial_size, final_size);

    pool.close().await;

    env::remove_var("DATABASE_MAX_CONNECTIONS");
    env::remove_var("DATABASE_MIN_CONNECTIONS");
}

// ============================================================================
// CONFIGURATION VALIDATION TESTS
// ============================================================================

#[test]
fn test_environment_variable_parsing() {
    // Test valid numeric parsing
    env::set_var("TEST_VAR", "42");
    let parsed: u32 = env::var("TEST_VAR")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);
    assert_eq!(parsed, 42);

    // Test invalid numeric parsing (should use default)
    env::set_var("TEST_VAR", "invalid");
    let parsed: u32 = env::var("TEST_VAR")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);
    assert_eq!(parsed, 10);

    // Test missing variable (should use default)
    env::remove_var("TEST_VAR");
    let parsed: u32 = env::var("TEST_VAR")
        .unwrap_or_else(|_| "20".to_string())
        .parse()
        .unwrap_or(20);
    assert_eq!(parsed, 20);

    env::remove_var("TEST_VAR");
}

#[test]
fn test_database_url_validation() {
    // Test valid PostgreSQL URL
    let valid_url = "postgresql://user:pass@localhost:5432/dbname";
    assert!(valid_url.starts_with("postgresql://"));

    // Test URL components
    assert!(valid_url.contains("@localhost"));
    assert!(valid_url.contains(":5432"));
    assert!(valid_url.ends_with("/dbname"));
}

// ============================================================================
// CLEANUP AND RESOURCE MANAGEMENT
// ============================================================================

#[tokio::test]
async fn test_pool_cleanup_on_drop() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();

    {
        let pool = create_connection_pool(&database_url).await.unwrap();

        // Use the pool
        let _result = sqlx::query("SELECT 1").fetch_one(&pool).await.unwrap();

        // Pool will be dropped here
    }

    // Test that we can create a new pool after the previous one was dropped
    let new_pool = create_connection_pool(&database_url).await.unwrap();
    let result = sqlx::query("SELECT 1").fetch_one(&new_pool).await;
    assert!(result.is_ok());

    new_pool.close().await;
}

#[tokio::test]
async fn test_graceful_pool_shutdown() {
    dotenvy::dotenv().ok();

    let database_url = get_test_database_url();
    let pool = create_connection_pool(&database_url).await.unwrap();

    // Use the pool
    let _result = sqlx::query("SELECT 1").fetch_one(&pool).await.unwrap();

    // Test graceful shutdown
    pool.close().await;

    // Pool should be closed - trying to use it should fail
    let closed_result = sqlx::query("SELECT 1").fetch_one(&pool).await;
    assert!(closed_result.is_err());
}