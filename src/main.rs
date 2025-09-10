use actix_web::{middleware::Compress, web, App, HttpServer};
use dotenvy::dotenv;
use std::env;
use tracing::{info, warn};

mod config;
mod db;
mod handlers;
mod middleware;
mod models;
mod services;

use config::LoggingConfig;
use db::database::{create_connection_pool, update_db_pool_metrics};
use middleware::{AuthMiddleware, CompressionAndCaching, MetricsMiddleware, StructuredLogger};
use services::auth::AuthService;

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

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("SERVER_PORT must be a valid port number");

    let workers = env::var("WORKERS")
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .expect("WORKERS must be a valid number");

    info!("Starting Health Export REST API");
    info!("Database URL: {}", mask_password(&database_url));
    info!("Server binding to: {}:{}", server_host, server_port);
    info!("Worker threads: {}", workers);

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
            // Increase payload size limit to 100MB for large health data uploads
            .app_data(web::PayloadConfig::new(100 * 1024 * 1024))
            .wrap(Compress::default()) // Add gzip compression
            .wrap(CompressionAndCaching) // Add caching headers
            .wrap(MetricsMiddleware)
            .wrap(StructuredLogger)
            .wrap(AuthMiddleware)
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
