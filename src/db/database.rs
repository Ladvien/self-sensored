use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::{error, info};

/// Create a PostgreSQL connection pool with configuration from environment
pub async fn create_connection_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    info!("Creating PostgreSQL connection pool");

    let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "20".to_string())
        .parse::<u32>()
        .unwrap_or(20);

    let min_connections = std::env::var("DATABASE_MIN_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse::<u32>()
        .unwrap_or(5);

    let connect_timeout = std::env::var("DATABASE_CONNECT_TIMEOUT")
        .unwrap_or_else(|_| "10".to_string())
        .parse::<u64>()
        .unwrap_or(10);

    let idle_timeout = std::env::var("DATABASE_IDLE_TIMEOUT")
        .unwrap_or_else(|_| "300".to_string())
        .parse::<u64>()
        .unwrap_or(300);

    let max_lifetime = std::env::var("DATABASE_MAX_LIFETIME")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<u64>()
        .unwrap_or(3600);

    info!(
        "Database pool config: max_conn={}, min_conn={}, connect_timeout={}s, idle_timeout={}s, max_lifetime={}s",
        max_connections, min_connections, connect_timeout, idle_timeout, max_lifetime
    );

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout))
        .idle_timeout(Some(Duration::from_secs(idle_timeout)))
        .max_lifetime(Some(Duration::from_secs(max_lifetime)))
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
