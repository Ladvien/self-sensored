use sqlx::PgPool;
use uuid::Uuid;
use self_sensored::services::auth::AuthService;

/// Setup a test user and API key, returning (user_id, api_key)
pub async fn setup_test_user_and_key(pool: &PgPool, email: &str) -> (Uuid, String) {
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user(email, Some("test_apple_health_id"), None)
        .await
        .unwrap();

    let (api_key_string, _api_key_obj) = auth_service
        .create_api_key(user.id, Some("Test API Key"), None, None, None)
        .await
        .unwrap();

    (user.id, api_key_string)
}

/// Clean up test data for a user
pub async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up in reverse order of foreign key dependencies
    sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM blood_pressure_metrics WHERE user_id = $1", user_id)
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

    sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id)
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

/// Get test database pool
pub async fn get_test_pool() -> PgPool {
    super::setup_test_db().await
}