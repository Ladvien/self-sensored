use actix_web::{web, HttpResponse, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

/// Basic health check endpoint
pub async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
        "service": "self-sensored-api"
    })))
}

/// API status endpoint with database connectivity check
pub async fn api_status(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let database_status = match sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    Ok(HttpResponse::Ok().json(json!({
        "status": "operational",
        "timestamp": Utc::now().to_rfc3339(),
        "service": "self-sensored-api",
        "version": env!("CARGO_PKG_VERSION"),
        "database": {
            "status": database_status
        },
        "environment": std::env::var("ENVIRONMENT").unwrap_or_else(|_| "unknown".to_string())
    })))
}
