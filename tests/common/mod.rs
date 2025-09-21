pub mod fixtures;
pub mod user;

use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use uuid::Uuid;

pub use fixtures::*;
pub use user::*;

pub async fn setup_test_db() -> PgPool {
    dotenvy::dotenv().ok();

    let database_url = env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set in .env file");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

// Use cleanup_test_data from user module instead

pub async fn load_test_fixture(size: &str) -> Value {
    let filename = format!("tests/fixtures/test_fixture_{}.json", size);
    let content = tokio::fs::read_to_string(&filename)
        .await
        .expect(&format!("Failed to read fixture file: {}", filename));

    serde_json::from_str(&content).expect(&format!("Failed to parse fixture file: {}", filename))
}

pub async fn create_test_redis_connection() -> redis::aio::ConnectionManager {
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    redis::aio::ConnectionManager::new(client)
        .await
        .expect("Failed to connect to Redis")
}
