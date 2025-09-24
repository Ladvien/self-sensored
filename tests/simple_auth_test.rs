use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use self_sensored::services::auth::{ApiKey, AuthService, User};

async fn get_test_pool() -> PgPool {
    // Load .env file
    dotenv::dotenv().ok();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[sqlx::test]
async fn test_auth_service_creation(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    // Test that auth service was created
    assert!(!auth_service.is_caching_enabled());
    assert!(!auth_service.is_rate_limiting_enabled());

    Ok(())
}

#[sqlx::test]
async fn test_hash_api_key(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let api_key = "test_key_12345";

    let hashed = auth_service.hash_api_key(api_key).unwrap();
    assert!(!hashed.is_empty());
    assert_ne!(hashed, api_key);
    assert!(hashed.starts_with("$argon2"));

    Ok(())
}

#[sqlx::test]
async fn test_verify_api_key(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let api_key = "test_key_12345";

    let hashed = auth_service.hash_api_key(api_key).unwrap();

    // Test correct verification
    let is_valid = auth_service.verify_api_key(api_key, &hashed).unwrap();
    assert!(is_valid);

    // Test incorrect verification
    let is_invalid = auth_service.verify_api_key("wrong_key", &hashed).unwrap();
    assert!(!is_invalid);

    Ok(())
}

#[sqlx::test]
async fn test_generate_api_key(pool: PgPool) -> sqlx::Result<()> {
    let key1 = AuthService::generate_api_key();
    let key2 = AuthService::generate_api_key();

    // Test keys are different
    assert_ne!(key1, key2);

    // Test keys have expected format
    assert!(key1.starts_with("hea_"));
    assert!(key2.starts_with("hea_"));

    // Test keys have reasonable length
    assert!(key1.len() > 30);
    assert!(key2.len() > 30);

    Ok(())
}

#[sqlx::test]
async fn test_user_creation(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let email = "test@example.com";

    let user = auth_service
        .create_user(email, None, None)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    assert_eq!(user.email, email);
    assert!(user.is_active.unwrap_or(false));
    assert!(user.created_at.is_some());

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_get_user_by_email(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let email = "test_lookup@example.com";

    // Create user
    let created_user = auth_service
        .create_user(email, None, None)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    // Find user by email
    let found_user = auth_service
        .get_user_by_email(email)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    assert!(found_user.is_some());
    let found_user = found_user.unwrap();
    assert_eq!(found_user.id, created_user.id);
    assert_eq!(found_user.email, email);

    // Test non-existent email
    let not_found = auth_service
        .get_user_by_email("nonexistent@example.com")
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    assert!(not_found.is_none());

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", created_user.id)
        .execute(auth_service.pool())
        .await?;

    Ok(())
}

#[sqlx::test]
async fn test_create_and_list_api_keys(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let email = "test_keys@example.com";

    // Create user
    let user = auth_service
        .create_user(email, None, None)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    // Create API key
    let (raw_key, api_key) = auth_service
        .create_api_key(user.id, Some("Test Key"), None, None, Some(100))
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    assert!(!raw_key.is_empty());
    assert_eq!(api_key.user_id, user.id);
    assert_eq!(api_key.name, Some("Test Key".to_string()));
    assert_eq!(api_key.rate_limit_per_hour, Some(100));

    // List keys
    let keys = auth_service
        .list_api_keys(user.id)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].id, api_key.id);

    // Clean up
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user.id)
        .execute(auth_service.pool())
        .await?;
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await?;

    Ok(())
}

#[test]
fn test_auth_context_for_testing() {
    let user_id = Uuid::new_v4();
    let auth_context = self_sensored::services::auth::AuthContext::new_for_testing(user_id);

    assert_eq!(auth_context.user.id, user_id);
    assert_eq!(auth_context.api_key.user_id, user_id);
    assert!(auth_context.user.is_active.unwrap_or(false));
    assert!(auth_context.api_key.is_active.unwrap_or(false));
    assert!(auth_context.user.email.contains(&user_id.to_string()));
}

#[test]
fn test_has_admin_permission() {
    let user_id = Uuid::new_v4();
    let auth_context = self_sensored::services::auth::AuthContext::new_for_testing(user_id);

    // Test without admin permission (default)
    assert!(!self_sensored::services::auth::AuthService::has_admin_permission(&auth_context));

    // Test has_permission with non-admin permission
    assert!(!self_sensored::services::auth::AuthService::has_permission(
        &auth_context,
        "admin"
    ));
    assert!(!self_sensored::services::auth::AuthService::has_permission(
        &auth_context,
        "read"
    ));
}
