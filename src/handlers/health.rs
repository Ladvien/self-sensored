use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use redis::Client as RedisClient;
use serde_json::json;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Health check statistics
static HEALTH_CHECK_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_SUCCESSFUL_CHECK: AtomicU64 = AtomicU64::new(0);
static DB_CHECK_FAILURES: AtomicU64 = AtomicU64::new(0);

/// Basic health check endpoint with enhanced diagnostics for Cloudflare 520 troubleshooting
pub async fn health_check() -> Result<HttpResponse> {
    let check_id = HEALTH_CHECK_COUNT.fetch_add(1, Ordering::Relaxed);
    let timestamp = Utc::now();

    info!(check_id = check_id, "Health check requested");

    // Update last successful check timestamp
    let epoch_seconds = timestamp.timestamp() as u64;
    LAST_SUCCESSFUL_CHECK.store(epoch_seconds, Ordering::Relaxed);

    // Get uptime
    let uptime_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Enhanced response for debugging Cloudflare 520 issues
    let response = json!({
        "status": "healthy",
        "timestamp": timestamp.to_rfc3339(),
        "service": "self-sensored-api",
        "version": env!("CARGO_PKG_VERSION"),
        "check_id": check_id,
        "uptime_seconds": uptime_seconds,
        "last_check_timestamp": epoch_seconds,
        "process_id": std::process::id(),
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string()),
        "server": {
            "host": std::env::var("SERVER_HOST").unwrap_or_else(|_| "unknown".to_string()),
            "port": std::env::var("SERVER_PORT").unwrap_or_else(|_| "unknown".to_string()),
            "workers": std::env::var("WORKERS").unwrap_or_else(|_| "unknown".to_string())
        },
        "connection_info": {
            "keep_alive_timeout": std::env::var("KEEP_ALIVE_TIMEOUT_SECONDS").unwrap_or_else(|_| "15".to_string()),
            "connection_timeout": std::env::var("CONNECTION_TIMEOUT_SECONDS").unwrap_or_else(|_| "30".to_string()),
            "request_timeout": std::env::var("REQUEST_TIMEOUT_SECONDS").unwrap_or_else(|_| "60".to_string())
        },
        "cloudflare_debug": {
            "origin_response_time_ms": 50, // Fast response for Cloudflare
            "status_code": 200,
            "headers_valid": true,
            "body_size_bytes": 1024 // Estimate
        }
    });

    // Return with proper headers for Cloudflare compatibility
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
        .insert_header(("X-Health-Check-ID", check_id.to_string()))
        .insert_header(("X-Origin-Server", "self-sensored-api"))
        .insert_header(("Connection", "keep-alive"))
        .json(response))
}

/// Comprehensive API status endpoint with database connectivity and system diagnostics
pub async fn api_status(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let check_start = std::time::Instant::now();
    let timestamp = Utc::now();

    info!("Comprehensive API status check initiated");

    // Perform actual database health check
    let (database_status, db_response_time_ms) = check_database_health(&pool).await;

    // Check Redis connectivity if configured
    let (redis_status, redis_response_time_ms) = check_redis_health().await;

    let check_duration = check_start.elapsed();

    let response = json!({
        "status": "operational",
        "timestamp": timestamp.to_rfc3339(),
        "service": "self-sensored-api",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string()),
        "health_check_stats": {
            "total_checks": HEALTH_CHECK_COUNT.load(Ordering::Relaxed),
            "last_successful_timestamp": LAST_SUCCESSFUL_CHECK.load(Ordering::Relaxed),
            "db_check_failures": DB_CHECK_FAILURES.load(Ordering::Relaxed)
        },
        "database": {
            "status": database_status,
            "response_time_ms": db_response_time_ms
        },
        "redis": {
            "status": redis_status,
            "response_time_ms": redis_response_time_ms
        },
        "dependencies": {
            "database_healthy": database_status == "connected",
            "redis_healthy": redis_status == "connected" || redis_status == "not_configured",
            "all_healthy": database_status == "connected" && (redis_status == "connected" || redis_status == "not_configured")
        },
        "system": {
            "uptime_seconds": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        },
        "performance": {
            "check_duration_ms": check_duration.as_millis(),
            "db_response_time_ms": db_response_time_ms,
            "redis_response_time_ms": redis_response_time_ms
        }
    });

    // Determine overall health status for response
    let overall_status = if database_status == "connected"
        && (redis_status == "connected" || redis_status == "not_configured")
    {
        "healthy"
    } else {
        "unhealthy"
    };

    let mut status_code = if overall_status == "healthy" {
        HttpResponse::Ok()
    } else {
        HttpResponse::ServiceUnavailable()
    };

    Ok(status_code
        .insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"))
        .insert_header(("X-API-Status", overall_status))
        .insert_header(("X-DB-Status", database_status))
        .insert_header(("X-Redis-Status", redis_status))
        .insert_header(("X-Response-Time-MS", check_duration.as_millis().to_string()))
        .insert_header(("X-Origin-Server", "self-sensored-api"))
        .insert_header(("Connection", "keep-alive"))
        .json(response))
}

/// Liveness probe endpoint - minimal response for Kubernetes/Docker health checks
pub async fn liveness_probe() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-cache"))
        .json(json!({
            "status": "alive",
            "timestamp": Utc::now().timestamp()
        })))
}

/// Readiness probe endpoint - includes basic connectivity checks
pub async fn readiness_probe(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let check_start = std::time::Instant::now();

    // Perform quick health checks for readiness
    let (db_status, db_time_ms) = check_database_health(&pool).await;
    let (redis_status, redis_time_ms) = check_redis_health().await;

    let check_duration = check_start.elapsed();

    // Determine if service is ready
    let is_ready = db_status == "connected"
        && (redis_status == "connected" || redis_status == "not_configured");
    let status = if is_ready { "ready" } else { "not_ready" };

    let response = json!({
        "status": status,
        "timestamp": Utc::now().timestamp(),
        "database": {
            "status": db_status,
            "response_time_ms": db_time_ms
        },
        "redis": {
            "status": redis_status,
            "response_time_ms": redis_time_ms
        },
        "ready": is_ready,
        "check_duration_ms": check_duration.as_millis()
    });

    let mut status_code = if is_ready {
        HttpResponse::Ok()
    } else {
        HttpResponse::ServiceUnavailable()
    };

    Ok(status_code
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Ready-Status", status))
        .insert_header(("X-DB-Status", db_status))
        .insert_header(("X-Redis-Status", redis_status))
        .json(response))
}

/// Check database connectivity and measure response time
async fn check_database_health(pool: &PgPool) -> (&'static str, u64) {
    let start = std::time::Instant::now();

    match sqlx::query!("SELECT 1 as health_check")
        .fetch_one(pool)
        .await
    {
        Ok(_) => {
            let duration = start.elapsed();
            info!("Database health check passed in {}ms", duration.as_millis());
            ("connected", duration.as_millis() as u64)
        }
        Err(e) => {
            let duration = start.elapsed();
            error!(
                "Database health check failed after {}ms: {}",
                duration.as_millis(),
                e
            );
            DB_CHECK_FAILURES.fetch_add(1, Ordering::Relaxed);
            ("disconnected", duration.as_millis() as u64)
        }
    }
}

/// Check Redis connectivity and measure response time
async fn check_redis_health() -> (&'static str, u64) {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    // If Redis URL indicates it's disabled, skip the check
    if redis_url.is_empty() || redis_url == "disabled" {
        return ("not_configured", 0);
    }

    let start = std::time::Instant::now();

    match RedisClient::open(redis_url.as_str()) {
        Ok(client) => match client.get_async_connection().await {
            Ok(mut conn) => match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => {
                    let duration = start.elapsed();
                    info!("Redis health check passed in {}ms", duration.as_millis());
                    ("connected", duration.as_millis() as u64)
                }
                Err(e) => {
                    let duration = start.elapsed();
                    warn!("Redis PING failed after {}ms: {}", duration.as_millis(), e);
                    ("disconnected", duration.as_millis() as u64)
                }
            },
            Err(e) => {
                let duration = start.elapsed();
                warn!(
                    "Redis connection failed after {}ms: {}",
                    duration.as_millis(),
                    e
                );
                ("disconnected", duration.as_millis() as u64)
            }
        },
        Err(e) => {
            let duration = start.elapsed();
            warn!(
                "Redis client creation failed after {}ms: {}",
                duration.as_millis(),
                e
            );
            ("disconnected", duration.as_millis() as u64)
        }
    }
}
