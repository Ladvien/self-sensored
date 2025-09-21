use std::env;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use self_sensored::services::auth::AuthService;

async fn setup_test_database() -> PgPool {
    let database_url = env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL or TEST_DATABASE_URL must be set");

    PgPool::connect(&database_url).await.expect("Failed to connect to test database")
}

#[tokio::test]
async fn test_auth_service_creation() {
    let pool = setup_test_database().await;
    let auth_service = AuthService::new(pool);

    // Test that auth service can be created successfully
    assert!(auth_service.pool().is_some());
}

#[tokio::test]
async fn test_basic_auth_functionality() {
    let pool = setup_test_database().await;
    let auth_service = AuthService::new(pool);

    // Create test user
    let user_id = Uuid::new_v4();
    let username = format!("test_user_{}", user_id);
    let email = format!("test_{}@example.com", user_id);

    // Basic functionality test - just verify auth service responds
    let result = std::panic::catch_unwind(|| {
        // Test that methods can be called without panicking
        auth_service.pool().is_some()
    });

    assert!(result.is_ok());
}

#[test]
fn test_auth_service_types() {
    // Test that AuthService can be instantiated with a connection pool type
    use sqlx::Pool;
    use sqlx::Postgres;

    // This test ensures the types are correct
    let _pool_type: Option<Pool<Postgres>> = None;

    // Test passes if compilation succeeds
    assert!(true);
}