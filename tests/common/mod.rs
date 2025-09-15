use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use uuid::Uuid;

pub async fn setup_test_db() -> PgPool {
    dotenv::dotenv().ok();

    let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored_test"
            .to_string()
    });

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

pub async fn cleanup_test_db(pool: &PgPool, user_id: Uuid) {
    // Clean up in reverse order of foreign key dependencies
    sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!(
        "DELETE FROM blood_pressure_metrics WHERE user_id = $1",
        user_id
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query!("DELETE FROM sleep_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM workouts WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM body_measurements WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM audit_log WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

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
