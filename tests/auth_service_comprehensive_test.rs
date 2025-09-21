use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use self_sensored::services::auth::{
    ApiKey, AuthContext, AuthError, AuthService, User,
};
use self_sensored::services::cache::{CacheConfig, CacheService};
use self_sensored::services::rate_limiter::{RateLimiter, RateLimiterConfig};

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn setup_test_cache() -> CacheService {
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let config = CacheConfig {
        enabled: true,
        default_ttl_seconds: 5,
        summary_ttl_seconds: 10,
        user_data_ttl_seconds: 8,
        key_prefix: "test_auth".to_string(),
    };

    CacheService::new(&redis_url, config)
        .await
        .expect("Failed to create cache service")
}

async fn setup_test_rate_limiter() -> RateLimiter {
    let config = RateLimiterConfig {
        requests_per_hour: 100,
        requests_per_minute: 10,
        burst_size: 20,
        cleanup_interval_secs: 60,
    };

    RateLimiter::new(config)
}

async fn create_test_user(pool: &PgPool) -> (User, String) {
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    sqlx::query!(
        r#"
        INSERT INTO users (id, email, is_active, created_at)
        VALUES ($1, $2, true, NOW())
        "#,
        user_id,
        &email
    )
    .execute(pool)
    .await
    .unwrap();

    let user = User {
        id: user_id,
        email: email.clone(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: None,
        is_active: Some(true),
        metadata: None,
    };

    (user, email)
}

async fn create_test_api_key(pool: &PgPool, user_id: Uuid) -> (ApiKey, String) {
    let api_key_id = Uuid::new_v4();
    let raw_key = format!("test_key_{}", Uuid::new_v4());
    let auth_service = AuthService::new(pool.clone());
    let key_hash = auth_service.hash_api_key(&raw_key).unwrap();

    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at)
        VALUES ($1, $2, $3, $4, true, NOW())
        "#,
        api_key_id,
        user_id,
        &key_hash,
        "Test Key"
    )
    .execute(pool)
    .await
    .unwrap();

    let api_key = ApiKey {
        id: api_key_id,
        user_id,
        name: Some("Test Key".to_string()),
        created_at: Some(Utc::now()),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: Some(100),
    };

    (api_key, raw_key)
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_auth_service_creation() {
    let pool = setup_test_pool().await;
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;

    let auth_service = AuthService::new_with_cache(
        pool.clone(),
        Some(rate_limiter),
        Some(cache)
    );

    assert!(auth_service.is_caching_enabled());
}

#[sqlx::test]
async fn test_hash_and_verify_api_key(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);
    let raw_key = "test_api_key_12345";

    // Hash the key
    let hashed = auth_service.hash_api_key(raw_key).unwrap();
    assert!(!hashed.is_empty());
    assert_ne!(hashed, raw_key);

    // Verify correct key
    let is_valid = auth_service.verify_api_key(raw_key, &hashed).unwrap();
    assert!(is_valid);

    // Verify incorrect key
    let is_invalid = auth_service.verify_api_key("wrong_key", &hashed).unwrap();
    assert!(!is_invalid);

    Ok(())
}

#[sqlx::test]
async fn test_create_api_key(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;

    // Create API key
    let (api_key, raw_key) = auth_service
        .create_api_key(user.id, "Production Key", None)
        .await
        .unwrap();

    assert_eq!(api_key.user_id, user.id);
    assert_eq!(api_key.name, Some("Production Key".to_string()));
    assert!(api_key.is_active.unwrap_or(false));
    assert!(!raw_key.is_empty());

    // Verify key was stored
    let stored_key = sqlx::query!(
        "SELECT id, user_id, name FROM api_keys WHERE id = $1",
        api_key.id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(stored_key.user_id, user.id);
    assert_eq!(stored_key.name, Some("Production Key".to_string()));

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_validate_api_key_success(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let (_, raw_key) = create_test_api_key(&pool, user.id).await;

    // Validate the key
    let auth_context = auth_service.validate_api_key(&raw_key).await.unwrap();

    assert_eq!(auth_context.user.id, user.id);
    assert_eq!(auth_context.api_key.user_id, user.id);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_validate_api_key_invalid(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    // Try to validate non-existent key
    let result = auth_service.validate_api_key("invalid_key_123").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::InvalidApiKey => (),
        other => panic!("Expected InvalidApiKey, got {:?}", other),
    }

    Ok(())
}

#[sqlx::test]
async fn test_validate_api_key_expired(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let api_key_id = Uuid::new_v4();
    let raw_key = format!("expired_key_{}", Uuid::new_v4());
    let key_hash = AuthService::hash_api_key(&raw_key).unwrap();

    // Create expired key
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, expires_at, is_active, created_at)
        VALUES ($1, $2, $3, $4, $5, true, NOW())
        "#,
        api_key_id,
        user.id,
        &key_hash,
        "Expired Key",
        Utc::now() - Duration::hours(1) // Expired 1 hour ago
    )
    .execute(&pool)
    .await?;

    // Try to validate expired key
    let result = auth_service.validate_api_key(&raw_key).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::ApiKeyExpired => (),
        other => panic!("Expected ApiKeyExpired, got {:?}", other),
    }

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_validate_api_key_inactive(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let api_key_id = Uuid::new_v4();
    let raw_key = format!("inactive_key_{}", Uuid::new_v4());
    let key_hash = AuthService::hash_api_key(&raw_key).unwrap();

    // Create inactive key
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at)
        VALUES ($1, $2, $3, $4, false, NOW())
        "#,
        api_key_id,
        user.id,
        &key_hash,
        "Inactive Key"
    )
    .execute(&pool)
    .await?;

    // Try to validate inactive key
    let result = auth_service.validate_api_key(&raw_key).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::ApiKeyInactive => (),
        other => panic!("Expected ApiKeyInactive, got {:?}", other),
    }

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_validate_api_key_user_inactive(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let user_id = Uuid::new_v4();

    // Create inactive user
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, is_active, created_at)
        VALUES ($1, $2, false, NOW())
        "#,
        user_id,
        format!("inactive_{}@example.com", user_id)
    )
    .execute(&pool)
    .await?;

    let (_, raw_key) = create_test_api_key(&pool, user_id).await;

    // Try to validate key for inactive user
    let result = auth_service.validate_api_key(&raw_key).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::UserInactive => (),
        other => panic!("Expected UserInactive, got {:?}", other),
    }

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_revoke_api_key(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let (api_key, _) = create_test_api_key(&pool, user.id).await;

    // Revoke the key
    auth_service.revoke_api_key(api_key.id).await.unwrap();

    // Verify key was deactivated
    let is_active = sqlx::query_scalar!(
        "SELECT is_active FROM api_keys WHERE id = $1",
        api_key.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(!is_active);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_list_user_api_keys(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;

    // Create multiple API keys
    for i in 0..3 {
        auth_service
            .create_api_key(user.id, &format!("Key {}", i), None)
            .await
            .unwrap();
    }

    // List keys
    let keys = auth_service.list_user_api_keys(user.id).await.unwrap();

    assert_eq!(keys.len(), 3);
    for key in &keys {
        assert_eq!(key.user_id, user.id);
    }

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_update_last_used(pool: PgPool) -> sqlx::Result<()> {
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let (api_key, _) = create_test_api_key(&pool, user.id).await;

    // Update last used
    auth_service.update_last_used(api_key.id).await.unwrap();

    // Verify timestamp was updated
    let last_used = sqlx::query_scalar!(
        "SELECT last_used_at FROM api_keys WHERE id = $1",
        api_key.id
    )
    .fetch_one(&pool)
    .await?;

    assert!(last_used.is_some());

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[tokio::test]
async fn test_auth_context_creation() {
    let user_id = Uuid::new_v4();
    let auth_context = AuthContext::new_for_testing(user_id);

    assert_eq!(auth_context.user.id, user_id);
    assert_eq!(auth_context.api_key.user_id, user_id);
    assert!(auth_context.user.is_active.unwrap_or(false));
    assert!(auth_context.api_key.is_active.unwrap_or(false));
}

#[tokio::test]
async fn test_cached_authentication() {
    let pool = setup_test_pool().await;
    let cache = setup_test_cache().await;
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service = AuthService::new(
        pool.clone(),
        Arc::new(cache),
        Arc::new(RwLock::new(rate_limiter)),
    );

    let (user, _) = create_test_user(&pool).await;
    let (_, raw_key) = create_test_api_key(&pool, user.id).await;

    // First validation - should hit database
    let auth1 = auth_service.validate_api_key(&raw_key).await.unwrap();

    // Second validation - should hit cache
    let auth2 = auth_service.validate_api_key(&raw_key).await.unwrap();

    assert_eq!(auth1.user.id, auth2.user.id);
    assert_eq!(auth1.api_key.id, auth2.api_key.id);

    cleanup_test_data(&pool, user.id).await;
}

#[test]
fn test_auth_error_display() {
    let errors = vec![
        AuthError::InvalidApiKey,
        AuthError::ApiKeyExpired,
        AuthError::ApiKeyInactive,
        AuthError::UserInactive,
        AuthError::HashingError("test error".to_string()),
    ];

    for error in errors {
        let display = format!("{}", error);
        assert!(!display.is_empty());

        let debug = format!("{:?}", error);
        assert!(!debug.is_empty());
    }
}

#[test]
fn test_user_serialization() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: Some("apple123".to_string()),
        created_at: Some(Utc::now()),
        updated_at: None,
        is_active: Some(true),
        metadata: Some(json!({"key": "value"})),
    };

    // Serialize
    let json = serde_json::to_string(&user).unwrap();
    assert!(json.contains("test@example.com"));

    // Deserialize
    let deserialized: User = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, user.id);
    assert_eq!(deserialized.email, user.email);
}

#[test]
fn test_api_key_serialization() {
    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: Some("Production".to_string()),
        created_at: Some(Utc::now()),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: Some(json!(["read", "write"])),
        rate_limit_per_hour: Some(1000),
    };

    // Serialize
    let json = serde_json::to_string(&api_key).unwrap();
    assert!(json.contains("Production"));

    // Deserialize
    let deserialized: ApiKey = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, api_key.id);
    assert_eq!(deserialized.name, api_key.name);
}