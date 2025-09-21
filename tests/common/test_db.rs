use self_sensored::db::database::create_connection_pool;
use sqlx::PgPool;

/// Helper function to create test database pool with .env loading
pub async fn create_test_pool() -> PgPool {
    // Load .env file if it exists (for local development)
    dotenvy::dotenv().ok();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool")
}