use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use self_sensored::services::auth::{
    ApiKey, AuthContext, AuthError, AuthService, User,
};
use self_sensored::services::cache::{CacheConfig, CacheService};
use self_sensored::services::rate_limiter::{RateLimiter, RateLimitInfo};

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
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    RateLimiter::new(&redis_url)
        .await
        .unwrap_or_else(|_| RateLimiter::new_in_memory(100))
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
async fn test_auth_service_creation_variants() {
    let pool = setup_test_pool().await;

    // Test basic creation
    let auth_service = AuthService::new(pool.clone());
    assert!(!auth_service.is_caching_enabled());
    assert!(!auth_service.is_rate_limiting_enabled());

    // Test with rate limiter
    let rate_limiter = setup_test_rate_limiter().await;
    let auth_service_with_rl = AuthService::new_with_rate_limiter(pool.clone(), Some(rate_limiter));
    assert!(auth_service_with_rl.is_rate_limiting_enabled());
    assert!(!auth_service_with_rl.is_caching_enabled());

    // Test with cache
    let cache = setup_test_cache().await;
    let rate_limiter2 = setup_test_rate_limiter().await;
    let auth_service_full = AuthService::new_with_cache(pool, Some(rate_limiter2), Some(cache));
    assert!(auth_service_full.is_caching_enabled());
    assert!(auth_service_full.is_rate_limiting_enabled());
}

#[sqlx::test]
async fn test_hash_and_verify_api_key_comprehensive(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    // Test multiple key formats
    let mut test_keys_owned = Vec::new();
    test_keys_owned.push("test_api_key_12345".to_string());
    test_keys_owned.push("hea_1234567890abcdef".to_string());
    test_keys_owned.push("very_long_key_".repeat(10));
    test_keys_owned.push("short".to_string());
    test_keys_owned.push("special!@#$%^&*()chars".to_string());
    let test_keys = test_keys_owned;

    for api_key in test_keys {
        // Hash the key
        let hashed = auth_service.hash_api_key(&api_key).unwrap();
        assert!(!hashed.is_empty());
        assert_ne!(hashed, api_key);
        assert!(hashed.starts_with("$argon2"));

        // Verify correct key
        let is_valid = auth_service.verify_api_key(&api_key, &hashed).unwrap();
        assert!(is_valid);

        // Verify incorrect key
        let is_invalid = auth_service.verify_api_key("wrong_key", &hashed).unwrap();
        assert!(!is_invalid);

        // Test with empty key
        let is_empty_invalid = auth_service.verify_api_key("", &hashed).unwrap();
        assert!(!is_empty_invalid);
    }

    Ok(())
}

#[sqlx::test]
async fn test_generate_api_key_uniqueness(pool: PgPool) -> sqlx::Result<()> {
    let mut generated_keys = std::collections::HashSet::new();

    // Generate 100 keys and ensure they're all unique
    for _ in 0..100 {
        let key = AuthService::generate_api_key();

        // Test format
        assert!(key.starts_with("hea_"));
        assert!(key.len() > 30);

        // Test uniqueness
        assert!(!generated_keys.contains(&key), "Duplicate key generated: {}", key);
        generated_keys.insert(key);
    }

    Ok(())
}

#[sqlx::test]
async fn test_user_management_comprehensive(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    // Test user creation with various inputs
    let test_cases = vec![
        ("simple@example.com", None, None),
        ("with_apple@example.com", Some("apple_health_123"), None),
        ("with_metadata@example.com", None, Some(json!({"role": "admin", "preferences": {"theme": "dark"}}))),
        ("full_data@example.com", Some("apple_456"), Some(json!({"location": "US"}))),
    ];

    let mut created_users = Vec::new();

    for (email, apple_health_id, metadata) in test_cases {
        // Create user
        let user = auth_service.create_user(email, apple_health_id, metadata.clone()).await
            .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

        assert_eq!(user.email, email);
        assert_eq!(user.apple_health_id, apple_health_id.map(String::from));
        assert_eq!(user.metadata, metadata);
        assert!(user.is_active.unwrap_or(false));
        assert!(user.created_at.is_some());

        created_users.push(user);

        // Test finding user by email
        let found_user = auth_service.get_user_by_email(email).await
            .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.email, email);
        assert_eq!(found_user.apple_health_id, apple_health_id.map(String::from));
    }

    // Test duplicate email handling
    let duplicate_result = auth_service.create_user("simple@example.com", None, None).await;
    assert!(duplicate_result.is_err());

    // Test non-existent email
    let not_found = auth_service.get_user_by_email("nonexistent@example.com").await
        .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;
    assert!(not_found.is_none());

    // Clean up
    for user in created_users {
        cleanup_test_data(auth_service.pool(), user.id).await;
    }

    Ok(())
}

#[sqlx::test]
async fn test_api_key_management_lifecycle(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    let (user, _) = create_test_user(auth_service.pool()).await;

    // Create multiple API keys with different configurations
    let key_configs = vec![
        (Some("Production Key"), None, None, Some(1000)),
        (Some("Development Key"), Some(Utc::now() + Duration::days(30)), Some(json!(["read", "write"])), Some(500)),
        (None, Some(Utc::now() + Duration::hours(1)), None, None),
        (Some("Admin Key"), None, Some(json!({"admin": true})), Some(2000)),
    ];

    let mut created_keys = Vec::new();

    for (name, expires_at, permissions, rate_limit) in key_configs {
        // Create API key
        let (raw_key, api_key) = auth_service.create_api_key(
            user.id,
            name,
            expires_at,
            permissions.clone(),
            rate_limit
        ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

        assert!(!raw_key.is_empty());
        assert_eq!(api_key.user_id, user.id);
        assert_eq!(api_key.name, name.map(String::from));
        assert_eq!(api_key.expires_at, expires_at);
        assert_eq!(api_key.permissions, permissions);
        assert_eq!(api_key.rate_limit_per_hour, rate_limit);

        created_keys.push((raw_key, api_key));
    }

    // List keys
    let keys = auth_service.list_api_keys(user.id).await
        .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;
    assert_eq!(keys.len(), 4);

    // Verify all keys are present
    for (_, api_key) in &created_keys {
        assert!(keys.iter().any(|k| k.id == api_key.id));
    }

    // Test key revocation
    let (_, first_key) = &created_keys[0];
    let was_revoked = auth_service.revoke_api_key(first_key.id, user.id).await
        .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;
    assert!(was_revoked);

    // Verify key was deactivated
    let is_active = sqlx::query_scalar!(
        "SELECT is_active FROM api_keys WHERE id = $1",
        first_key.id
    )
    .fetch_one(auth_service.pool())
    .await?;
    assert!(!is_active.unwrap_or(true));

    // Test revoking non-existent key
    let fake_id = Uuid::new_v4();
    let was_not_revoked = auth_service.revoke_api_key(fake_id, user.id).await
        .map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;
    assert!(!was_not_revoked);

    // Clean up
    cleanup_test_data(auth_service.pool(), user.id).await;

    Ok(())
}

#[sqlx::test]
async fn test_authentication_comprehensive(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    let (user, _) = create_test_user(auth_service.pool()).await;

    // Create test API key
    let (raw_key, api_key) = auth_service.create_api_key(
        user.id,
        Some("Test Key"),
        None,
        Some(json!(["read", "write"])),
        Some(100)
    ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

    // Test successful authentication
    let auth_context = auth_service.authenticate(
        &raw_key,
        Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        Some("test-agent/1.0")
    ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

    assert_eq!(auth_context.user.id, user.id);
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Test UUID-based authentication (Auto Export format)
    let uuid_key = api_key.id.to_string();
    let auth_context_uuid = auth_service.authenticate(
        &uuid_key,
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
        Some("auto-export/2.0")
    ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

    assert_eq!(auth_context_uuid.user.id, user.id);
    assert_eq!(auth_context_uuid.api_key.id, api_key.id);

    // Test authentication failures
    let uuid_key = Uuid::new_v4().to_string();
    let test_failures = vec![
        ("invalid_key_123", AuthError::InvalidApiKey),
        ("", AuthError::InvalidApiKey),
        ("hea_nonexistent", AuthError::InvalidApiKey),
        (uuid_key.as_str(), AuthError::InvalidApiKey),
    ];

    for (invalid_key, expected_error) in test_failures {
        let result = auth_service.authenticate(invalid_key, None, None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            err if std::mem::discriminant(&err) == std::mem::discriminant(&expected_error) => (),
            other => panic!("Expected {:?}, got {:?}", expected_error, other),
        }
    }

    // Clean up
    cleanup_test_data(auth_service.pool(), user.id).await;

    Ok(())
}

#[sqlx::test]
async fn test_expired_and_inactive_keys(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool);

    let (user, _) = create_test_user(auth_service.pool()).await;

    // Create expired key
    let (expired_key, expired_api_key) = auth_service.create_api_key(
        user.id,
        Some("Expired Key"),
        Some(Utc::now() - Duration::hours(1)), // Expired 1 hour ago
        None,
        None
    ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

    // Test expired key authentication
    let expired_result = auth_service.authenticate(&expired_key, None, None).await;
    assert!(expired_result.is_err());
    match expired_result.unwrap_err() {
        AuthError::ApiKeyExpired => (),
        other => panic!("Expected ApiKeyExpired, got {:?}", other),
    }

    // Create and then deactivate a key
    let (active_key, active_api_key) = auth_service.create_api_key(
        user.id,
        Some("Soon Inactive"),
        None,
        None,
        None
    ).await.map_err(|e| sqlx::Error::Protocol(format!("Auth error: {:?}", e)))?;

    // Manually deactivate the key
    sqlx::query!(
        "UPDATE api_keys SET is_active = false WHERE id = $1",
        active_api_key.id
    )
    .execute(auth_service.pool())
    .await?;

    // Test inactive key authentication
    let inactive_result = auth_service.authenticate(&active_key, None, None).await;
    assert!(inactive_result.is_err());

    // Deactivate user
    sqlx::query!(
        "UPDATE users SET is_active = false WHERE id = $1",
        user.id
    )
    .execute(auth_service.pool())
    .await?;

    // Test authentication with inactive user
    let user_inactive_result = auth_service.authenticate(&expired_key, None, None).await;
    assert!(user_inactive_result.is_err());

    // Clean up
    cleanup_test_data(auth_service.pool(), user.id).await;

    Ok(())
}

#[test]
fn test_permission_checking() {
    let user_id = Uuid::new_v4();

    // Test with no permissions
    let auth_context_empty = AuthContext::new_for_testing(user_id);
    assert!(!AuthService::has_admin_permission(&auth_context_empty));
    assert!(!AuthService::has_permission(&auth_context_empty, "read"));
    assert!(!AuthService::has_permission(&auth_context_empty, "write"));

    // Create auth context with array permissions
    let mut auth_context_array = AuthContext::new_for_testing(user_id);
    auth_context_array.api_key.permissions = Some(json!(["read", "write"]));

    assert!(!AuthService::has_admin_permission(&auth_context_array));
    assert!(AuthService::has_permission(&auth_context_array, "read"));
    assert!(AuthService::has_permission(&auth_context_array, "write"));
    assert!(!AuthService::has_permission(&auth_context_array, "admin"));

    // Create auth context with admin permissions (array format)
    let mut auth_context_admin_array = AuthContext::new_for_testing(user_id);
    auth_context_admin_array.api_key.permissions = Some(json!(["read", "write", "admin"]));

    assert!(AuthService::has_admin_permission(&auth_context_admin_array));
    assert!(AuthService::has_permission(&auth_context_admin_array, "read"));
    assert!(AuthService::has_permission(&auth_context_admin_array, "admin"));
    assert!(AuthService::has_permission(&auth_context_admin_array, "anything")); // Admin has all permissions

    // Create auth context with object permissions
    let mut auth_context_object = AuthContext::new_for_testing(user_id);
    auth_context_object.api_key.permissions = Some(json!({"read": true, "write": false}));

    assert!(!AuthService::has_admin_permission(&auth_context_object));
    assert!(AuthService::has_permission(&auth_context_object, "read"));
    assert!(!AuthService::has_permission(&auth_context_object, "write"));

    // Create auth context with admin object permissions
    let mut auth_context_admin_object = AuthContext::new_for_testing(user_id);
    auth_context_admin_object.api_key.permissions = Some(json!({"admin": true, "read": true}));

    assert!(AuthService::has_admin_permission(&auth_context_admin_object));
    assert!(AuthService::has_permission(&auth_context_admin_object, "anything")); // Admin has all permissions
}

#[test]
fn test_error_handling_and_display() {
    let errors = vec![
        AuthError::InvalidApiKey,
        AuthError::ApiKeyExpired,
        AuthError::ApiKeyInactive,
        AuthError::UserInactive,
        AuthError::HashingError("test error".to_string()),
        // Create a UUID error by parsing invalid string
        AuthError::UuidError(Uuid::parse_str("invalid-uuid").unwrap_err()),
    ];

    for error in errors {
        // Test Display trait
        let display = format!("{}", error);
        assert!(!display.is_empty());

        // Test Debug trait
        let debug = format!("{:?}", error);
        assert!(!debug.is_empty());

        // Test error categorization
        match error {
            AuthError::InvalidApiKey | AuthError::ApiKeyExpired | AuthError::ApiKeyInactive | AuthError::UserInactive => {
                assert!(display.contains("key") || display.contains("User"));
            }
            AuthError::HashingError(_) => {
                assert!(display.contains("error"));
            }
            AuthError::UuidError(_) => {
                assert!(display.contains("UUID"));
            }
            _ => {}
        }
    }
}

#[test]
fn test_user_and_api_key_serialization() {
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Test User serialization
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        apple_health_id: Some("apple123".to_string()),
        created_at: Some(now),
        updated_at: None,
        is_active: Some(true),
        metadata: Some(json!({"role": "user", "preferences": {"theme": "dark"}})),
    };

    let user_json = serde_json::to_string(&user).unwrap();
    assert!(user_json.contains("test@example.com"));
    assert!(user_json.contains("apple123"));

    let deserialized_user: User = serde_json::from_str(&user_json).unwrap();
    assert_eq!(deserialized_user.id, user.id);
    assert_eq!(deserialized_user.email, user.email);

    // Test ApiKey serialization
    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id,
        name: Some("Production Key".to_string()),
        created_at: Some(now),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: Some(json!(["read", "write", "admin"])),
        rate_limit_per_hour: Some(1000),
    };

    let api_key_json = serde_json::to_string(&api_key).unwrap();
    assert!(api_key_json.contains("Production Key"));
    assert!(api_key_json.contains("1000"));

    let deserialized_api_key: ApiKey = serde_json::from_str(&api_key_json).unwrap();
    assert_eq!(deserialized_api_key.id, api_key.id);
    assert_eq!(deserialized_api_key.rate_limit_per_hour, api_key.rate_limit_per_hour);
}

#[test]
fn test_auth_context_for_testing() {
    let user_id = Uuid::new_v4();
    let auth_context = AuthContext::new_for_testing(user_id);

    assert_eq!(auth_context.user.id, user_id);
    assert_eq!(auth_context.api_key.user_id, user_id);
    assert!(auth_context.user.is_active.unwrap_or(false));
    assert!(auth_context.api_key.is_active.unwrap_or(false));
    assert!(auth_context.user.email.contains(&user_id.to_string()));
    assert_eq!(auth_context.api_key.name, Some("Test API Key".to_string()));
}

#[test]
fn test_is_argon2_hash() {
    let valid_hashes = vec![
        "$argon2i$v=19$m=4096,t=3,p=1$salt$hash",
        "$argon2d$v=19$m=4096,t=3,p=1$salt$hash",
        "$argon2id$v=19$m=4096,t=3,p=1$salt$hash",
    ];

    let invalid_hashes = vec![
        "plain_text",
        "$bcrypt$salt$hash",
        "$argon$incomplete",
        "$argon2$missing_variant",
        "argon2i$no_dollar_prefix",
        "",
    ];

    // Note: We can't directly test the private is_argon2_hash method,
    // but we can test the behavior through hash verification
    let pool = futures::executor::block_on(setup_test_pool());
    let auth_service = AuthService::new(pool);

    // Test with a real argon2 hash
    let test_key = "test_key";
    let hash = auth_service.hash_api_key(test_key).unwrap();
    let is_valid = auth_service.verify_api_key(test_key, &hash).unwrap();
    assert!(is_valid);

    // Test with invalid hash format
    let invalid_result = auth_service.verify_api_key(test_key, "invalid_hash");
    assert!(invalid_result.is_err());
}

#[tokio::test]
async fn test_concurrent_authentication() {
    let pool = setup_test_pool().await;
    let auth_service = AuthService::new(pool);

    let (user, _) = create_test_user(auth_service.pool()).await;
    let (raw_key, _) = auth_service.create_api_key(user.id, Some("Concurrent Test"), None, None, None)
        .await.unwrap();

    // Spawn multiple concurrent authentication requests
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let auth_service = AuthService::new(auth_service.pool().clone());
            let key = raw_key.clone();
            tokio::spawn(async move {
                let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, (i % 255) as u8));
                auth_service.authenticate(&key, Some(ip), Some("concurrent-test")).await
            })
        })
        .collect();

    // Wait for all authentication attempts
    let results = futures::future::join_all(handles).await;

    // Verify all authentications succeeded
    for result in results {
        let auth_result = result.unwrap();
        assert!(auth_result.is_ok());
        let auth_context = auth_result.unwrap();
        assert_eq!(auth_context.user.id, user.id);
    }

    // Clean up
    cleanup_test_data(auth_service.pool(), user.id).await;
}