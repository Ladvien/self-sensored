use actix_cors::Cors;
use actix_web::{http::header, middleware::Compress, web, App, HttpServer};
use dotenvy::dotenv;
use std::env;
use std::time::Duration;
use tracing::{error, info, warn};

mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;

use config::LoggingConfig;
use db::database::{create_connection_pool, update_db_pool_metrics};
use middleware::{
    AdminMiddleware, AuthMiddleware, CompressionAndCaching, MetricsMiddleware, RateLimitMiddleware,
    StructuredLogger,
};
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
        .unwrap_or_else(|_| "60".to_string()) // Reduced from 300s to 60s to prevent DoS attacks
        .parse::<u64>()
        .expect("REQUEST_TIMEOUT_SECONDS must be a valid number");

    // Additional security configurations for request limiting
    let max_payload_size_mb = env::var("MAX_PAYLOAD_SIZE_MB")
        .unwrap_or_else(|_| "50".to_string()) // Default 50MB for health data uploads
        .parse::<usize>()
        .expect("MAX_PAYLOAD_SIZE_MB must be a valid number");

    let connection_timeout_seconds = env::var("CONNECTION_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "30".to_string()) // 30s connection timeout
        .parse::<u64>()
        .expect("CONNECTION_TIMEOUT_SECONDS must be a valid number");

    let keep_alive_timeout_seconds = env::var("KEEP_ALIVE_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "75".to_string()) // 75s keep-alive timeout (under Cloudflare's 100s)
        .parse::<u64>()
        .expect("KEEP_ALIVE_TIMEOUT_SECONDS must be a valid number");

    // Additional timeout configurations for Cloudflare 520 error prevention
    let client_shutdown_timeout_seconds = env::var("CLIENT_SHUTDOWN_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "30".to_string()) // 30s client shutdown timeout
        .parse::<u64>()
        .expect("CLIENT_SHUTDOWN_TIMEOUT_SECONDS must be a valid number");

    let server_shutdown_timeout_seconds = env::var("SERVER_SHUTDOWN_TIMEOUT_SECONDS")
        .unwrap_or_else(|_| "30".to_string()) // 30s server shutdown timeout
        .parse::<u64>()
        .expect("SERVER_SHUTDOWN_TIMEOUT_SECONDS must be a valid number");

    info!("Starting Health Export REST API");
    info!("Database URL: {}", mask_password(&database_url));
    info!("Server binding to: {}:{}", server_host, server_port);
    info!("Worker threads: {}", workers);
    info!(
        "Request timeout: {}s (DoS-protected)",
        request_timeout_seconds
    );
    info!(
        "Max payload size: {}MB (security-limited)",
        max_payload_size_mb
    );
    info!("Connection timeout: {}s", connection_timeout_seconds);
    info!(
        "Keep-alive timeout: {}s (Cloudflare-optimized)",
        keep_alive_timeout_seconds
    );
    info!(
        "Client shutdown timeout: {}s",
        client_shutdown_timeout_seconds
    );
    info!(
        "Server shutdown timeout: {}s",
        server_shutdown_timeout_seconds
    );

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

    // Create UserCharacteristicsService
    let user_characteristics_service = services::user_characteristics::UserCharacteristicsService::new(pool.clone());
    info!("UserCharacteristicsService initialized");

    // Create RateLimiter service
    let rate_limiter = match RateLimiter::new(&redis_url).await {
        Ok(limiter) => {
            if limiter.is_using_redis() {
                info!(
                    "Rate limiter initialized with Redis backend: {}",
                    mask_password(&redis_url)
                );
            } else {
                info!("Rate limiter initialized with in-memory fallback");
            }
            limiter
        }
        Err(e) => {
            warn!(
                "Failed to create rate limiter, falling back to in-memory: {}",
                e
            );
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
            .app_data(web::Data::new(user_characteristics_service.clone()))
            .app_data(web::Data::new(rate_limiter.clone()))
            // Security-limited payload size to prevent DoS attacks
            .app_data(web::PayloadConfig::new(max_payload_size_mb * 1024 * 1024))
            .wrap(configure_cors()) // CORS must be first for preflight requests
            .wrap(Compress::default()) // Add gzip compression
            .wrap(CompressionAndCaching) // Add caching headers
            .wrap(MetricsMiddleware)
            .wrap(StructuredLogger)
            .wrap(AuthMiddleware)
            .wrap(RateLimitMiddleware)
            .route("/health", web::get().to(handlers::health::health_check))
            .route(
                "/health/live",
                web::get().to(handlers::health::liveness_probe),
            )
            .route(
                "/health/ready",
                web::get().to(handlers::health::readiness_probe),
            )
            .route(
                "/metrics",
                web::get().to(middleware::metrics::metrics_handler),
            )
            .service(
                web::scope("/api/v1")
                    .route("/status", web::get().to(handlers::health::api_status))
                    .route("/ingest", web::post().to(handlers::ingest::ingest_handler))
                    .route(
                        "/ingest-async",
                        web::post()
                            .to(handlers::ingest_async_simple::ingest_async_optimized_handler),
                    )
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
                    // Reproductive Health Endpoints (HIPAA-Compliant)
                    .route(
                        "/ingest/reproductive-health",
                        web::post().to(handlers::reproductive_health_handler::ingest_reproductive_health),
                    )
                    .route(
                        "/data/menstrual",
                        web::get().to(handlers::reproductive_health_handler::get_menstrual_data),
                    )
                    .route(
                        "/data/fertility",
                        web::get().to(handlers::reproductive_health_handler::get_fertility_data),
                    )
                    // Environmental & Safety data endpoints
                    .route(
                        "/ingest/environmental",
                        web::post().to(handlers::environmental_handler::ingest_environmental_handler),
                    )
                    .route(
                        "/ingest/audio-exposure",
                        web::post().to(handlers::environmental_handler::ingest_audio_exposure_handler),
                    )
                    .route(
                        "/ingest/safety-events",
                        web::post().to(handlers::environmental_handler::ingest_safety_events_handler),
                    )
                    .route(
                        "/data/environmental",
                        web::get().to(handlers::environmental_handler::get_environmental_data_handler),
                    )
                    // Temperature metrics endpoints
                    .route(
                        "/ingest/temperature",
                        web::post().to(handlers::temperature_handler::ingest_temperature_handler),
                    )
                    .route(
                        "/data/temperature",
                        web::get().to(handlers::temperature_handler::get_temperature_data_handler),
                    )
                    // Hygiene events endpoints
                    .route(
                        "/ingest/hygiene",
                        web::post().to(handlers::hygiene_handler::ingest_hygiene),
                    )
                    .route(
                        "/data/hygiene",
                        web::get().to(handlers::hygiene_handler::get_hygiene_data),
                    )
                    // Body measurements endpoints (weight, BMI, body composition)
                    .route(
                        "/ingest/body-measurements",
                        web::post().to(handlers::body_measurements_handler::ingest_body_measurements),
                    )
                    .route(
                        "/data/body-measurements",
                        web::get().to(handlers::body_measurements_handler::get_body_measurements_data),
                    )
                    // Respiratory health endpoints (SpO2, breathing rate, lung function)
                    .route(
                        "/ingest/respiratory",
                        web::post().to(handlers::respiratory_handler::ingest_respiratory_handler),
                    )
                    .route(
                        "/data/respiratory",
                        web::get().to(handlers::respiratory_handler::query_respiratory_handler),
                    )
                    // Mindfulness & Mental Health endpoints (HIPAA-compliant)
                    .route(
                        "/ingest/mindfulness",
                        web::post().to(handlers::mindfulness_handler::ingest_mindfulness),
                    )
                    .route(
                        "/ingest/mental-health",
                        web::post().to(handlers::mindfulness_handler::ingest_mental_health),
                    )
                    .route(
                        "/data/mindfulness",
                        web::get().to(handlers::mindfulness_handler::get_mindfulness_data),
                    )
                    .route(
                        "/data/mental-health",
                        web::get().to(handlers::mindfulness_handler::get_mental_health_data),
                    )
                    // Nutrition & Hydration endpoints (comprehensive dietary tracking)
                    .route(
                        "/ingest/nutrition",
                        web::post().to(handlers::nutrition_handler::ingest_nutrition_data),
                    )
                    .route(
                        "/data/nutrition",
                        web::get().to(handlers::nutrition_handler::get_nutrition_data),
                    )
                    .route(
                        "/data/hydration",
                        web::get().to(handlers::nutrition_handler::get_hydration_data),
                    )
                    // Symptoms Tracking endpoints (comprehensive illness monitoring)
                    .route(
                        "/ingest/symptoms",
                        web::post().to(handlers::symptoms_handler::ingest_symptoms_handler),
                    )
                    .route(
                        "/data/symptoms",
                        web::get().to(handlers::symptoms_handler::get_symptoms_handler),
                    )
                    // Blood Glucose & Metabolic endpoints (medical-grade data handling)
                    .route(
                        "/ingest/blood-glucose",
                        web::post().to(handlers::metabolic_handler::ingest_blood_glucose_handler),
                    )
                    .route(
                        "/ingest/metabolic",
                        web::post().to(handlers::metabolic_handler::ingest_metabolic_handler),
                    )
                    .route(
                        "/data/blood-glucose",
                        web::get().to(handlers::metabolic_handler::get_blood_glucose_data_handler),
                    )
                    .route(
                        "/data/metabolic",
                        web::get().to(handlers::metabolic_handler::get_metabolic_data_handler),
                    )
                    // User Characteristics endpoints for personalized health tracking
                    .route(
                        "/user/characteristics",
                        web::get().to(handlers::user_characteristics_handler::get_user_characteristics),
                    )
                    .route(
                        "/user/characteristics",
                        web::post().to(handlers::user_characteristics_handler::create_user_characteristics),
                    )
                    .route(
                        "/user/characteristics",
                        web::put().to(handlers::user_characteristics_handler::update_user_characteristics),
                    )
                    .route(
                        "/user/characteristics",
                        web::patch().to(handlers::user_characteristics_handler::upsert_user_characteristics),
                    )
                    .route(
                        "/user/characteristics",
                        web::delete().to(handlers::user_characteristics_handler::delete_user_characteristics),
                    )
                    .route(
                        "/user/characteristics/verify",
                        web::post().to(handlers::user_characteristics_handler::verify_user_characteristics),
                    )
                    .route(
                        "/user/characteristics/validation/{metric_type}",
                        web::get().to(handlers::user_characteristics_handler::get_validation_ranges),
                    )
                    .route(
                        "/user/characteristics/uv-recommendations",
                        web::get().to(handlers::user_characteristics_handler::get_uv_recommendations),
                    )
                    .route(
                        "/user/characteristics/activity-personalization",
                        web::get().to(handlers::user_characteristics_handler::get_activity_personalization),
                    )
                    .route(
                        "/user/characteristics/heart-rate-zones",
                        web::get().to(handlers::user_characteristics_handler::get_heart_rate_zones),
                    )
                    .route(
                        "/user/characteristics/emergency-info",
                        web::get().to(handlers::user_characteristics_handler::get_emergency_info),
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
                    // Admin endpoints for logging management (admin-only)
                    .service(
                        web::scope("/admin")
                            .wrap(AdminMiddleware) // Apply admin authorization middleware
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
                            )
                            .route(
                                "/characteristics/stats",
                                web::get().to(handlers::user_characteristics_handler::get_aggregate_stats),
                            ),
                    ),
            )
    })
    .workers(workers)
    .client_request_timeout(Duration::from_secs(request_timeout_seconds))
    .client_disconnect_timeout(Duration::from_secs(client_shutdown_timeout_seconds))
    .shutdown_timeout(server_shutdown_timeout_seconds)
    .keep_alive(Duration::from_secs(keep_alive_timeout_seconds))
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
    let allowed_origins =
        env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| match environment.as_str() {
            "production" => "https://api.yourdomain.com".to_string(),
            _ => "http://localhost:3000,https://localhost:3000".to_string(),
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
        warn!(
            "CORS credentials support is ENABLED - ensure origins are explicitly set and trusted"
        );
    }

    // Production environment security validations
    if environment == "production" {
        if allowed_origins.contains("localhost") {
            warn!("SECURITY WARNING: localhost origins detected in production CORS configuration!");
        }
        if allowed_origins.contains('*') {
            error!("SECURITY ERROR: Wildcard origins are not allowed in production!");
            // Return a restrictive CORS configuration instead of panicking
            return Cors::default()
                .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                .allowed_headers(vec![
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                ])
                .max_age(max_age);
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
