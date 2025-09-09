use crate::middleware::metrics::Metrics;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::{error, info};

/// Create a PostgreSQL connection pool with configuration from environment
pub async fn create_connection_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Creating optimized PostgreSQL connection pool");

    let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "50".to_string()) // Increased from 20 for better concurrency
        .parse::<u32>()
        .unwrap_or(50);

    let min_connections = std::env::var("DATABASE_MIN_CONNECTIONS")
        .unwrap_or_else(|_| "10".to_string()) // Increased from 5 to maintain ready connections
        .parse::<u32>()
        .unwrap_or(10);

    let connect_timeout = std::env::var("DATABASE_CONNECT_TIMEOUT")
        .unwrap_or_else(|_| "5".to_string()) // Reduced from 10 for faster failures
        .parse::<u64>()
        .unwrap_or(5);

    let idle_timeout = std::env::var("DATABASE_IDLE_TIMEOUT")
        .unwrap_or_else(|_| "600".to_string()) // Increased from 300 to reduce connection churn
        .parse::<u64>()
        .unwrap_or(600);

    let max_lifetime = std::env::var("DATABASE_MAX_LIFETIME")
        .unwrap_or_else(|_| "1800".to_string()) // Reduced from 3600 for better connection health
        .parse::<u64>()
        .unwrap_or(1800);

    // New: Test connection timeout for faster health checks
    let test_timeout = std::env::var("DATABASE_TEST_TIMEOUT")
        .unwrap_or_else(|_| "3".to_string())
        .parse::<u64>()
        .unwrap_or(3);

    info!(
        "Optimized database pool config: max_conn={}, min_conn={}, connect_timeout={}s, idle_timeout={}s, max_lifetime={}s, test_timeout={}s",
        max_connections, min_connections, connect_timeout, idle_timeout, max_lifetime, test_timeout
    );

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout))
        .idle_timeout(Some(Duration::from_secs(idle_timeout)))
        .max_lifetime(Some(Duration::from_secs(max_lifetime)))
        .test_before_acquire(true) // Ensure connections are healthy
        .connect(database_url)
        .await
    {
        Ok(pool) => {
            info!("Database connection pool created successfully");
            Ok(pool)
        }
        Err(e) => {
            error!("Failed to create database connection pool: {}", e);
            Err(e)
        }
    }
}

/// Test database connection and verify required extensions
pub async fn test_database_connection(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Testing database connection and extensions");

    // Test basic connection
    sqlx::query("SELECT 1").fetch_one(pool).await?;
    info!("✓ Basic database connection successful");

    // Test PostgreSQL version
    let version: (String,) = sqlx::query_as("SELECT version()").fetch_one(pool).await?;
    info!("✓ PostgreSQL version: {}", version.0);

    // Test uuid-ossp extension
    match sqlx::query("SELECT gen_random_uuid()")
        .fetch_one(pool)
        .await
    {
        Ok(_) => info!("✓ UUID extension available"),
        Err(e) => {
            error!("✗ UUID extension not available: {}", e);
            return Err(e);
        }
    }

    // Test PostGIS extension
    match sqlx::query("SELECT PostGIS_version()")
        .fetch_one(pool)
        .await
    {
        Ok(_) => info!("✓ PostGIS extension available"),
        Err(e) => {
            error!("✗ PostGIS extension not available: {}", e);
            return Err(e);
        }
    }

    info!("All database tests passed successfully");
    Ok(())
}

/// Monitor database connection pool metrics and update Prometheus metrics
pub fn update_db_pool_metrics(pool: &PgPool) {
    let size = pool.size() as usize;
    let idle = pool.num_idle();

    // Update Prometheus metrics
    Metrics::update_db_connection_metrics(size as u64, idle as u64);

    // Log if pool utilization is high
    let utilization = if size > 0 {
        ((size.saturating_sub(idle)) as f64 / size as f64) * 100.0
    } else {
        0.0
    };
    if utilization > 80.0 {
        tracing::warn!(
            "High database pool utilization: {:.1}% ({}/{} connections active)",
            utilization,
            size.saturating_sub(idle),
            size
        );
    }
}
