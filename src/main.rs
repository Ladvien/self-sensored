use actix_cors::Cors;
use actix_web::{http::header, middleware::Compress, web, App, HttpServer};
use dotenvy::dotenv;
use std::env;
use std::time::Duration;
use tracing::{info, warn};

mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;

use config::LoggingConfig;
use db::database::{create_connection_pool, update_db_pool_metrics};
use middleware::{AuthMiddleware, CompressionAndCaching, MetricsMiddleware, RateLimitMiddleware, StructuredLogger};
use services::{auth::AuthService, rate_limiter::RateLimiter};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize structured logging with JSON format
    let logging_config = LoggingConfig::from_env();
    logging_config
        .init()
        .expect("Failed to initialize structured logging");

    // Load configuration from environment variables
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment or .env file");

    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("SERVER_PORT must be a valid port number");

    let workers = env::var("WORKERS")
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .expect("WORKERS must be a valid number");

    let request_timeout_seconds = env::var("REQUEST_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "90".to_string())
        .parse::<u64>()
        .expect("REQUEST_TIMEOUT_SECONDS must be a valid number");

    info!("Starting Health Export REST API");
    info!("Database URL: {}", mask_password(&database_url));
    info!("Server binding to: {}:{}", server_host, server_port);
    info!("Worker threads: {}", workers);
    info!("Request timeout: {}s (Cloudflare safe)", request_timeout_seconds);

    // Create database connection pool
    let pool = create_connection_pool(&database_url)
        .await
        .expect("Failed to create database connection pool");

    info!("Database connection pool created successfully");

    // Test database connection
    match sqlx::query("SELECT 1").fetch_one(&pool).await {
        Ok(_) => info!("Database connection test successful"),
        Err(e) => {
            warn!("Database connection test failed: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Database connection failed: {e}"),
            ));
        }
    }

    // Create AuthService
    let auth_service = AuthService::new(pool.clone());
    info!("AuthService initialized");

    // Create RateLimiter service
    let rate_limiter = match RateLimiter::new(&redis_url).await {
        Ok(limiter) => {
            if limiter.is_using_redis() {
                info!("Rate limiter initialized with Redis backend: {}", mask_password(&redis_url));
            } else {
                info!("Rate limiter initialized with in-memory fallback");
            }
            limiter
        }
        Err(e) => {
            warn!("Failed to create rate limiter, falling back to in-memory: {}", e);
            RateLimiter::new_in_memory(100) // Default 100 requests/hour
        }
    };

    // Start database metrics monitoring task
    let pool_for_metrics = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            update_db_pool_metrics(&pool_for_metrics);
        }
    });
    info!("Database metrics monitoring started");

    // Start MQTT subscriber for Auto Health Export data - temporarily disabled
    // let mqtt_pool = pool.clone();
    // tokio::spawn(async move {
    //     info!("Starting MQTT subscriber service");
    //     let subscriber = services::mqtt_subscriber::MqttSubscriber::new(mqtt_pool);

    //     // Keep retrying connection with backoff
    //     loop {
    //         match subscriber.start().await {
    //             Ok(_) => {
    //                 info!("MQTT subscriber connected successfully");
    //             }
    //             Err(e) => {
    //                 {
    //                     warn!("MQTT subscriber error: {}, retrying in 30 seconds", e);
    //                 } // Error drops here
    //                 tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    //             }
    //         }
    //     }
    // });
    info!("MQTT subscriber temporarily disabled - fix Send trait issue");

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::Data::new(rate_limiter.clone()))
            // Increase payload size limit to 100MB for large health data uploads
            .app_data(web::PayloadConfig::new(100 * 1024 * 1024))
            .wrap(configure_cors()) // CORS must be first for preflight requests
            .wrap(Compress::default()) // Add gzip compression
            .wrap(CompressionAndCaching) // Add caching headers
            .wrap(MetricsMiddleware)
            .wrap(StructuredLogger)
            .wrap(AuthMiddleware)
            .wrap(RateLimitMiddleware)
            .route("/health", web::get().to(handlers::health::health_check))
            .route(
                "/metrics",
                web::get().to(middleware::metrics::metrics_handler),
            )
            .service(
                web::scope("/api/v1")
                    .route("/status", web::get().to(handlers::health::api_status))
                    .route("/ingest", web::post().to(handlers::ingest::ingest_handler))
                    // Health data query endpoints
                    .route(
                        "/data/heart-rate",
                        web::get().to(handlers::query::get_heart_rate_data),
                    )
                    .route(
                        "/data/blood-pressure",
                        web::get().to(handlers::query::get_blood_pressure_data),
                    )
                    .route(
                        "/data/sleep",
                        web::get().to(handlers::query::get_sleep_data),
                    )
                    .route(
                        "/data/activity",
                        web::get().to(handlers::query::get_activity_data),
                    )
                    .route(
                        "/data/workouts",
                        web::get().to(handlers::query::get_workout_data),
                    )
                    .route(
                        "/data/summary",
                        web::get().to(handlers::query::get_health_summary),
                    )
                    // Health data export endpoints
                    .route(
                        "/export/all",
                        web::get().to(handlers::export::export_health_data),
                    )
                    .route(
                        "/export/heart-rate",
                        web::get().to(handlers::export::export_heart_rate_data),
                    )
                    .route(
                        "/export/activity-analytics",
                        web::get().to(handlers::export::export_activity_summary),
                    )
                    // Admin endpoints for logging management
                    .service(
                        web::scope("/admin")
                            .route(
                                "/logging/level",
                                web::get().to(handlers::admin::get_log_level),
                            )
                            .route(
                                "/logging/level",
                                web::put().to(handlers::admin::set_log_level),
                            )
                            .route(
                                "/logging/stats",
                                web::get().to(handlers::admin::get_logging_stats),
                            )
                            .route(
                                "/logging/test",
                                web::post().to(handlers::admin::generate_test_logs),
                            ),
                    ),
            )
    })
    .workers(workers)
    .client_request_timeout(Duration::from_secs(request_timeout_seconds))
    .bind((server_host, server_port))?
    .run()
    .await
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            let password_start = colon_pos + 1;
            let password_end = at_pos;
            if password_end > password_start {
                masked.replace_range(password_start..password_end, "****");
            }
            return masked;
        }
    }
    url.to_string()
}

/// Configure CORS middleware with security-focused settings
fn configure_cors() -> Cors {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    
    // Parse allowed origins from environment variable
    let allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| {
            match environment.as_str() {
                "production" => "https://api.yourdomain.com".to_string(),
                _ => "http://localhost:3000,https://localhost:3000".to_string(),
            }
        });
    
    let max_age = env::var("CORS_MAX_AGE")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<usize>()
        .unwrap_or(3600);
    
    let allow_credentials = env::var("CORS_ALLOW_CREDENTIALS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    info!("Configuring CORS for environment: {}", environment);
    info!("CORS allowed origins: {}", allowed_origins);
    info!("CORS max age: {} seconds", max_age);
    info!("CORS credentials allowed: {}", allow_credentials);

    let mut cors = Cors::default()
        // Restrict to necessary HTTP methods only
        .allowed_methods(vec!["GET", "POST", "OPTIONS"])
        
        // Only allow necessary headers for API authentication
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-api-key"), // Custom API key header
        ])
        
        // Expose Content-Disposition for file downloads
        .expose_headers(&[header::CONTENT_DISPOSITION])
        
        // Set preflight cache duration
        .max_age(max_age);

    // Add allowed origins
    for origin in allowed_origins.split(',') {
        let trimmed_origin = origin.trim();
        if !trimmed_origin.is_empty() {
            cors = cors.allowed_origin(trimmed_origin);
            info!("Added CORS allowed origin: {}", trimmed_origin);
        }
    }

    // Configure credentials support (be very careful with this in production)
    if allow_credentials {
        cors = cors.supports_credentials();
        warn!("CORS credentials support is ENABLED - ensure origins are explicitly set and trusted");
    }

    // Production environment security validations
    if environment == "production" {
        if allowed_origins.contains("localhost") {
            warn!("SECURITY WARNING: localhost origins detected in production CORS configuration!");
        }
        if allowed_origins.contains('*') {
            panic!("SECURITY ERROR: Wildcard origins are not allowed in production!");
        }
        info!("Production CORS security validations passed");
    }

    cors
}

/// Configure CORS middleware specifically for testing
#[cfg(test)]
pub fn configure_cors_for_testing() -> Cors {
    use actix_cors::Cors;
    use actix_web::http::header;
    
    let test_origins = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,https://trusted-app.com".to_string());
    
    let max_age = env::var("CORS_MAX_AGE")
        .unwrap_or_else(|_| "1800".to_string())
        .parse::<usize>()
        .unwrap_or(1800);
    
    let allow_credentials = env::var("CORS_ALLOW_CREDENTIALS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-api-key"),
        ])
        .expose_headers(&[header::CONTENT_DISPOSITION])
        .max_age(max_age);

    // Add test origins
    for origin in test_origins.split(',') {
        let trimmed_origin = origin.trim();
        if !trimmed_origin.is_empty() {
            cors = cors.allowed_origin(trimmed_origin);
        }
    }

    if allow_credentials {
        cors = cors.supports_credentials();
    }

    cors
}
