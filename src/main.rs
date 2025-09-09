use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use std::env;
use tracing::{info, warn};

mod db;
mod handlers;
mod middleware;
mod models;
mod services;

use db::database::create_connection_pool;
use middleware::{AuthMiddleware, RequestLogger};
use services::auth::AuthService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logging with environment-configurable level
    let log_level = env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .parse()
        .unwrap_or(tracing::Level::INFO);

    tracing_subscriber::fmt().with_max_level(log_level).init();

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

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            // Increase payload size limit to 100MB for large health data uploads
            .app_data(web::PayloadConfig::new(100 * 1024 * 1024))
            .wrap(RequestLogger)
            .wrap(AuthMiddleware)
            .route("/health", web::get().to(handlers::health::health_check))
            .service(
                web::scope("/api/v1")
                    .route("/status", web::get().to(handlers::health::api_status))
                    .route("/ingest", web::post().to(handlers::ingest::ingest_handler)),
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
