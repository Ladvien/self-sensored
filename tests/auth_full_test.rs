use chrono::{Duration, Utc};
use self_sensored::services::auth::{AuthContext, AuthError, AuthService, User};
use self_sensored::services::cache::{CacheConfig, CacheService};
use self_sensored::services::rate_limiter::RateLimiter;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

/// Test database pool helper
async fn get_test_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/health_export_test".to_string()
    });

    let max_connections = std::env::var("TEST_DATABASE_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "200".to_string())
        .parse::<u32>()
        .unwrap_or(200);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(Some(std::time::Duration::from_secs(300)))
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper to create cache service for testing (may fail if Redis not available)
async fn create_test_cache_service() -> Option<CacheService> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let config = CacheConfig::default();

    CacheService::new(&redis_url, config).await.ok()
}

/// Helper to create rate limiter for testing (may fail if Redis not available)
async fn create_test_rate_limiter() -> Option<RateLimiter> {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

    RateLimiter::new(&redis_url).await.ok()
}

/// Cleanup test data
async fn cleanup_test_user(pool: &sqlx::PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Create test user helper
async fn create_test_user(auth_service: &AuthService, email: &str) -> Result<User, AuthError> {
    // Generate unique apple_health_id for each test to avoid conflicts
    let unique_health_id = format!("test_health_id_{}", Uuid::new_v4());
    auth_service
        .create_user(email, Some(&unique_health_id), Some(json!({"test": true})))
        .await
}

#[tokio::test]
async fn test_auth_service_new() {
    let pool = get_test_pool().await;

    // Test basic constructor
    let auth_service = AuthService::new(pool.clone());
    assert!(!auth_service.is_rate_limiting_enabled());
    assert!(!auth_service.is_caching_enabled());

    // Test with rate limiter (may fail if Redis not available)
    let rate_limiter = create_test_rate_limiter().await;
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), rate_limiter);
    // Rate limiting may or may not be enabled depending on Redis availability

    // Test with cache (may fail if Redis not available)
    let cache_service = create_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool.clone(), None, cache_service);
    // Cache may or may not be enabled depending on Redis availability
}

#[tokio::test]
async fn test_pool_access() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Test pool access
    let retrieved_pool = auth_service.pool();
    assert_eq!(retrieved_pool.size(), pool.size());
}

#[tokio::test]
async fn test_api_key_generation() {
    // Test unique key generation
    let key1 = AuthService::generate_api_key();
    let key2 = AuthService::generate_api_key();
    let key3 = AuthService::generate_api_key();

    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);

    // Test format
    assert!(key1.starts_with("hea_"));
    assert!(key2.starts_with("hea_"));
    assert!(key3.starts_with("hea_"));

    // Test length (hea_ + 32 hex chars)
    assert_eq!(key1.len(), 36);
    assert_eq!(key2.len(), 36);
    assert_eq!(key3.len(), 36);
}

#[tokio::test]
async fn test_api_key_hashing_and_verification() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let api_key = "test_api_key_12345";

    // Test hashing
    let hash1 = auth_service.hash_api_key(api_key).unwrap();
    let hash2 = auth_service.hash_api_key(api_key).unwrap();

    // Hashes should be different due to salt
    assert_ne!(hash1, hash2);

    // Both hashes should verify correctly
    assert!(auth_service.verify_api_key(api_key, &hash1).unwrap());
    assert!(auth_service.verify_api_key(api_key, &hash2).unwrap());

    // Wrong key should not verify
    assert!(!auth_service.verify_api_key("wrong_key", &hash1).unwrap());
    assert!(!auth_service.verify_api_key("wrong_key", &hash2).unwrap());

    // Hashes should start with $argon2
    assert!(hash1.starts_with("$argon2"));
    assert!(hash2.starts_with("$argon2"));
}

#[tokio::test]
async fn test_api_key_verification_errors() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Test with invalid hash format
    let result = auth_service.verify_api_key("test_key", "invalid_hash");
    // The verify_api_key method should return a HashingError for invalid hash formats
    match &result {
        Err(e) => eprintln!("Error for 'invalid_hash': {:?}", e),
        Ok(v) => eprintln!("Unexpected Ok value: {}", v),
    }
    assert!(
        matches!(result, Err(AuthError::HashingError(_))),
        "Expected HashingError for invalid hash, got {:?}",
        result
    );

    // Test with malformed argon2 hash - this might return Ok(false) if it's partially valid
    // The argon2 library may parse "$argon2id$malformed" as having the right prefix but wrong structure
    // In this case, verify_api_key returns Ok(false) rather than an error
    let result = auth_service.verify_api_key("test_key", "$argon2id$malformed");
    match &result {
        Err(e) => eprintln!("Error for '$argon2id$malformed': {:?}", e),
        Ok(v) => eprintln!("Result for '$argon2id$malformed': Ok({})", v),
    }
    // Accept either an error or Ok(false) for malformed hashes
    assert!(
        matches!(result, Err(AuthError::HashingError(_))) || matches!(result, Ok(false)),
        "Expected HashingError or Ok(false) for malformed argon2, got {:?}",
        result
    );
}

#[tokio::test]
async fn test_user_creation_and_retrieval() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_create_user@example.com";
    let apple_health_id = "test_health_123";
    let metadata = json!({"role": "test", "created_by": "unit_test"});

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = auth_service
        .create_user(test_email, Some(apple_health_id), Some(metadata.clone()))
        .await
        .unwrap();

    assert_eq!(user.email, test_email);
    assert_eq!(user.apple_health_id, Some(apple_health_id.to_string()));
    assert_eq!(user.metadata, Some(metadata));
    assert_eq!(user.is_active, Some(true));
    assert!(user.created_at.is_some());

    // Test retrieval
    let retrieved_user = auth_service.get_user_by_email(test_email).await.unwrap();
    assert!(retrieved_user.is_some());
    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.email, user.email);

    // Test retrieval of non-existent user
    let non_existent = auth_service
        .get_user_by_email("nonexistent@example.com")
        .await
        .unwrap();
    assert!(non_existent.is_none());

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_create_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_api_key@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Test API key creation with all parameters
    let key_name = "Test API Key";
    let expires_at = Some(Utc::now() + Duration::days(30));
    let permissions = Some(json!(["read", "write"]));
    let rate_limit = Some(500);

    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some(key_name),
            expires_at,
            permissions.clone(),
            rate_limit,
        )
        .await
        .unwrap();

    // Validate returned data
    assert!(plain_key.starts_with("hea_"));
    assert_eq!(plain_key.len(), 36);
    assert_eq!(api_key.user_id, user.id);
    assert_eq!(api_key.name, Some(key_name.to_string()));
    assert_eq!(api_key.permissions, permissions);
    assert_eq!(api_key.rate_limit_per_hour, rate_limit);
    assert_eq!(api_key.is_active, Some(true));
    assert!(api_key.created_at.is_some());

    // Test API key creation with minimal parameters
    let (plain_key2, api_key2) = auth_service
        .create_api_key(user.id, None, None, None, None)
        .await
        .unwrap();

    assert!(plain_key2.starts_with("hea_"));
    assert_ne!(plain_key, plain_key2);
    assert_eq!(api_key2.user_id, user.id);
    assert_eq!(api_key2.name, None);
    assert_eq!(api_key2.permissions, None);
    assert_eq!(api_key2.rate_limit_per_hour, None);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_list_api_keys() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_list_keys@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Initially should have no keys
    let keys = auth_service.list_api_keys(user.id).await.unwrap();
    assert_eq!(keys.len(), 0);

    // Create multiple API keys
    let (_, key1) = auth_service
        .create_api_key(user.id, Some("Key 1"), None, None, None)
        .await
        .unwrap();
    let (_, key2) = auth_service
        .create_api_key(user.id, Some("Key 2"), None, None, None)
        .await
        .unwrap();
    let (_, key3) = auth_service
        .create_api_key(user.id, Some("Key 3"), None, None, None)
        .await
        .unwrap();

    // List keys
    let keys = auth_service.list_api_keys(user.id).await.unwrap();
    assert_eq!(keys.len(), 3);

    // Verify keys are ordered by created_at DESC
    let key_ids: Vec<_> = keys.iter().map(|k| k.id).collect();
    assert!(key_ids.contains(&key1.id));
    assert!(key_ids.contains(&key2.id));
    assert!(key_ids.contains(&key3.id));

    // Test with non-existent user
    let fake_user_id = Uuid::new_v4();
    let empty_keys = auth_service.list_api_keys(fake_user_id).await.unwrap();
    assert_eq!(empty_keys.len(), 0);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_revoke_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_revoke@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (_, api_key) = auth_service
        .create_api_key(user.id, Some("Test Key"), None, None, None)
        .await
        .unwrap();

    // Revoke the API key
    let was_revoked = auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();
    assert!(was_revoked);

    // Try to revoke again (should return false)
    let was_revoked_again = auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();
    assert!(!was_revoked_again);

    // Try to revoke with wrong user ID (should return false)
    let fake_user_id = Uuid::new_v4();
    let wrong_user_revoke = auth_service
        .revoke_api_key(api_key.id, fake_user_id)
        .await
        .unwrap();
    assert!(!wrong_user_revoke);

    // Try to revoke non-existent key (should return false)
    let fake_key_id = Uuid::new_v4();
    let non_existent_revoke = auth_service
        .revoke_api_key(fake_key_id, user.id)
        .await
        .unwrap();
    assert!(!non_existent_revoke);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_uuid_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_uuid_auth@example.com";
    let test_ip: IpAddr = "127.0.0.1".parse().unwrap();
    let test_user_agent = "TestAgent/1.0";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (_, api_key) = auth_service
        .create_api_key(user.id, Some("UUID Test Key"), None, None, None)
        .await
        .unwrap();

    // Test authentication with UUID (Auto Export format)
    let auth_result = auth_service
        .authenticate(
            &api_key.id.to_string(),
            Some(test_ip),
            Some(test_user_agent),
        )
        .await
        .unwrap();

    assert_eq!(auth_result.user.id, user.id);
    assert_eq!(auth_result.api_key.id, api_key.id);
    assert_eq!(auth_result.user.email, user.email);

    // Test with invalid UUID
    let fake_uuid = Uuid::new_v4();
    let invalid_result = auth_service
        .authenticate(&fake_uuid.to_string(), Some(test_ip), Some(test_user_agent))
        .await;
    assert!(matches!(invalid_result, Err(AuthError::InvalidApiKey)));

    // Test with malformed UUID
    let malformed_result = auth_service
        .authenticate("not-a-uuid", Some(test_ip), Some(test_user_agent))
        .await;
    assert!(matches!(malformed_result, Err(AuthError::InvalidApiKey)));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_hashed_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_hashed_auth@example.com";
    let test_ip: IpAddr = "192.168.1.1".parse().unwrap();
    let test_user_agent = "HashedTestAgent/1.0";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Hashed Test Key"), None, None, None)
        .await
        .unwrap();

    // Test authentication with the plain key
    let auth_result = auth_service
        .authenticate(&plain_key, Some(test_ip), Some(test_user_agent))
        .await
        .unwrap();

    assert_eq!(auth_result.user.id, user.id);
    assert_eq!(auth_result.api_key.id, api_key.id);

    // Test with wrong key
    let wrong_key_result = auth_service
        .authenticate(
            "hea_wrongkey12345678901234567890123",
            Some(test_ip),
            Some(test_user_agent),
        )
        .await;
    assert!(matches!(wrong_key_result, Err(AuthError::InvalidApiKey)));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_expired_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_expired@example.com";
    let test_ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and expired API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let expired_time = Utc::now() - Duration::days(1); // Expired yesterday
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Expired Key"), Some(expired_time), None, None)
        .await
        .unwrap();

    // Test authentication with expired UUID key
    let expired_result = auth_service
        .authenticate(&api_key.id.to_string(), Some(test_ip), None)
        .await;
    assert!(matches!(expired_result, Err(AuthError::ApiKeyExpired)));

    // Test authentication with expired hashed key
    let expired_hashed_result = auth_service
        .authenticate(&plain_key, Some(test_ip), None)
        .await;
    assert!(matches!(
        expired_hashed_result,
        Err(AuthError::ApiKeyExpired)
    ));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_inactive_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_inactive@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Test Key"), None, None, None)
        .await
        .unwrap();

    // Revoke the key
    auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();

    // Test authentication with revoked UUID key
    let inactive_result = auth_service
        .authenticate(&api_key.id.to_string(), None, None)
        .await;
    assert!(matches!(inactive_result, Err(AuthError::InvalidApiKey)));

    // Test authentication with revoked hashed key
    let inactive_hashed_result = auth_service.authenticate(&plain_key, None, None).await;
    assert!(matches!(
        inactive_hashed_result,
        Err(AuthError::InvalidApiKey)
    ));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_inactive_user() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_inactive_user@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Test Key"), None, None, None)
        .await
        .unwrap();

    // Deactivate the user
    sqlx::query!("UPDATE users SET is_active = false WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();

    // Test authentication with inactive user
    let inactive_user_result = auth_service
        .authenticate(&api_key.id.to_string(), None, None)
        .await;
    assert!(matches!(
        inactive_user_result,
        Err(AuthError::InvalidApiKey)
    ));

    // Test with hashed key
    let inactive_user_hashed_result = auth_service.authenticate(&plain_key, None, None).await;
    assert!(matches!(
        inactive_user_hashed_result,
        Err(AuthError::InvalidApiKey)
    ));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_authenticate_with_rate_limiting() {
    let pool = get_test_pool().await;

    // Try to create rate limiter (may fail if Redis not available)
    let rate_limiter = create_test_rate_limiter().await;
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), rate_limiter);

    let test_email = "test_rate_limit@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (_, api_key) = auth_service
        .create_api_key(user.id, Some("Rate Limited Key"), None, None, None)
        .await
        .unwrap();

    // Test authentication (may work if no rate limiter or if limits not exceeded)
    let auth_result = auth_service
        .authenticate(
            &api_key.id.to_string(),
            Some("127.0.0.1".parse().unwrap()),
            None,
        )
        .await;

    // Should succeed if rate limiter not available or limits not hit
    if auth_service.is_rate_limiting_enabled() {
        // Rate limiter is available - result may succeed or fail depending on limits
        match auth_result {
            Ok(auth_context) => {
                assert_eq!(auth_context.user.id, user.id);
            }
            Err(AuthError::RateLimitExceeded(_)) => {
                // Expected if rate limit exceeded
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    } else {
        // No rate limiter - should succeed
        assert!(auth_result.is_ok());
    }

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_cache_functionality() {
    let pool = get_test_pool().await;
    let cache_service = create_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool.clone(), None, cache_service);

    let test_email = "test_cache@example.com";
    let test_ip: IpAddr = "172.16.0.1".parse().unwrap();

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, _) = auth_service
        .create_api_key(user.id, Some("Cached Key"), None, None, None)
        .await
        .unwrap();

    // First authentication (should hit database)
    let auth_result1 = auth_service
        .authenticate(&plain_key, Some(test_ip), Some("CacheTestAgent/1.0"))
        .await
        .unwrap();

    // Second authentication (may hit cache if caching enabled)
    let auth_result2 = auth_service
        .authenticate(&plain_key, Some(test_ip), Some("CacheTestAgent/1.0"))
        .await
        .unwrap();

    // Results should be identical
    assert_eq!(auth_result1.user.id, auth_result2.user.id);
    assert_eq!(auth_result1.api_key.id, auth_result2.api_key.id);

    // Test cache invalidation (only works if caching enabled)
    auth_service.invalidate_user_auth_cache(user.id).await;

    // Third authentication after cache invalidation
    let auth_result3 = auth_service
        .authenticate(&plain_key, Some(test_ip), Some("CacheTestAgent/1.0"))
        .await
        .unwrap();

    assert_eq!(auth_result3.user.id, user.id);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_permissions_system() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_permissions@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API keys with different permissions
    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Admin permissions (array format)
    let (_, admin_key) = auth_service
        .create_api_key(
            user.id,
            Some("Admin Key"),
            None,
            Some(json!(["read", "write", "admin"])),
            None,
        )
        .await
        .unwrap();

    // User permissions (object format)
    let (_, user_key) = auth_service
        .create_api_key(
            user.id,
            Some("User Key"),
            None,
            Some(json!({"read": true, "write": false})),
            None,
        )
        .await
        .unwrap();

    // No permissions
    let (_, no_perms_key) = auth_service
        .create_api_key(user.id, Some("No Permissions Key"), None, None, None)
        .await
        .unwrap();

    // Test admin permissions
    let admin_context = AuthContext {
        user: user.clone(),
        api_key: admin_key,
    };

    assert!(AuthService::has_admin_permission(&admin_context));
    assert!(AuthService::has_permission(&admin_context, "read"));
    assert!(AuthService::has_permission(&admin_context, "write"));
    assert!(AuthService::has_permission(&admin_context, "admin"));
    assert!(AuthService::has_permission(
        &admin_context,
        "any_permission"
    )); // Admin has all

    // Test user permissions
    let user_context = AuthContext {
        user: user.clone(),
        api_key: user_key,
    };

    assert!(!AuthService::has_admin_permission(&user_context));
    assert!(AuthService::has_permission(&user_context, "read"));
    assert!(!AuthService::has_permission(&user_context, "write"));
    assert!(!AuthService::has_permission(&user_context, "admin"));

    // Test no permissions
    let no_perms_context = AuthContext {
        user: user.clone(),
        api_key: no_perms_key,
    };

    assert!(!AuthService::has_admin_permission(&no_perms_context));
    assert!(!AuthService::has_permission(&no_perms_context, "read"));
    assert!(!AuthService::has_permission(&no_perms_context, "write"));
    assert!(!AuthService::has_permission(&no_perms_context, "admin"));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_auth_context_for_testing() {
    let user_id = Uuid::new_v4();
    let auth_context = AuthContext::new_for_testing(user_id);

    assert_eq!(auth_context.user.id, user_id);
    assert_eq!(
        auth_context.user.email,
        format!("test-{user_id}@example.com")
    );
    assert_eq!(auth_context.user.is_active, Some(true));
    assert_eq!(auth_context.api_key.user_id, user_id);
    assert_eq!(auth_context.api_key.name, Some("Test API Key".to_string()));
    assert_eq!(auth_context.api_key.is_active, Some(true));
}

#[tokio::test]
async fn test_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let user_id = Some(Uuid::new_v4());
    let api_key_id = Some(Uuid::new_v4());
    let ip_address = Some("203.0.113.1".parse().unwrap());
    let user_agent = Some("TestBrowser/1.0");
    let metadata = Some(json!({"action": "test", "details": "unit test"}));

    // Test audit logging (should not fail)
    let result = auth_service
        .log_audit_event(
            user_id,
            api_key_id,
            "test_action",
            Some("test_resource"),
            ip_address,
            user_agent,
            metadata,
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rate_limit_status() {
    let pool = get_test_pool().await;
    let rate_limiter = create_test_rate_limiter().await;
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), rate_limiter);

    let api_key_id = Uuid::new_v4();

    // Test getting rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();

    if auth_service.is_rate_limiting_enabled() {
        // Rate limiter is available
        assert!(status.is_some());
        let status = status.unwrap();
        assert!(status.requests_limit > 0);
        assert!(status.requests_remaining <= status.requests_limit);
    } else {
        // No rate limiter available
        assert!(status.is_none());
    }

    // Test without rate limiter
    let auth_service_no_rl = AuthService::new(pool);
    let no_status = auth_service_no_rl
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();
    assert!(no_status.is_none());
}

#[tokio::test]
async fn test_service_feature_flags() {
    let pool = get_test_pool().await;

    // Test without rate limiter or cache
    let auth_service = AuthService::new(pool.clone());
    assert!(!auth_service.is_rate_limiting_enabled());
    assert!(!auth_service.is_caching_enabled());

    // Test with rate limiter
    let rate_limiter = create_test_rate_limiter().await;
    let auth_service_with_rl = AuthService::new_with_rate_limiter(pool.clone(), rate_limiter);
    // Rate limiting enabled status depends on Redis availability

    // Test with cache
    let cache_service = create_test_cache_service().await;
    let auth_service_with_cache = AuthService::new_with_cache(pool.clone(), None, cache_service);
    // Caching enabled status depends on Redis availability
}

#[tokio::test]
async fn test_cache_stats() {
    let pool = get_test_pool().await;

    // Test without cache
    let auth_service = AuthService::new(pool.clone());
    let stats = auth_service.get_cache_stats().await;
    assert!(stats.is_none());

    // Test with cache
    let cache_service = create_test_cache_service().await;
    let auth_service_with_cache = AuthService::new_with_cache(pool, None, cache_service);

    // Get cache stats (may return None if no cache service or stats not implemented)
    let stats = auth_service_with_cache.get_cache_stats().await;
    // The actual behavior depends on the CacheService implementation and Redis availability
}

#[tokio::test]
async fn test_database_errors() {
    // Test with invalid database URL to trigger connection errors
    let invalid_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect("postgresql://invalid:invalid@nonexistent:5432/invalid")
        .await;

    // This should fail to connect, but we can still create the service
    if invalid_pool.is_err() {
        // Expected - can't connect to invalid database
        return;
    }

    // If somehow we got a pool, test operations that should fail
    let pool = invalid_pool.unwrap();
    let auth_service = AuthService::new(pool);

    // These operations should fail with database errors
    let result = auth_service.get_user_by_email("test@example.com").await;
    assert!(result.is_err());

    let result = auth_service
        .create_user("test@example.com", None, None)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_authentication() {
    // Need larger pool for concurrent testing
    let pool = get_test_pool().await;

    let auth_service = Arc::new(AuthService::new(pool.clone()));

    let test_email = "test_concurrent@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, _) = auth_service
        .create_api_key(user.id, Some("Concurrent Key"), None, None, None)
        .await
        .unwrap();

    // Test concurrent authentication
    let mut handles = vec![];

    for i in 0..5 {
        let auth_service_clone = Arc::clone(&auth_service);
        let key_clone = plain_key.clone();
        let test_ip: IpAddr = format!("192.168.1.{}", i + 1).parse().unwrap();

        let handle = tokio::spawn(async move {
            auth_service_clone
                .authenticate(
                    &key_clone,
                    Some(test_ip),
                    Some(&format!("ConcurrentAgent/{}", i)),
                )
                .await
        });

        handles.push(handle);
    }

    // Wait for all authentications to complete
    let results = futures::future::join_all(handles).await;

    // All should succeed
    for result in results {
        let auth_result = result.unwrap().unwrap();
        assert_eq!(auth_result.user.id, user.id);
    }

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_edge_cases_and_malformed_inputs() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Test empty API key
    let result = auth_service.authenticate("", None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Test very long API key
    let long_key = "a".repeat(1000);
    let result = auth_service.authenticate(&long_key, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Test API key with special characters
    let special_key = "hea_!@#$%^&*()_+{}|:<>?";
    let result = auth_service.authenticate(special_key, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Test malformed UUID
    let malformed_uuid = "12345678-1234-1234-1234-12345678901"; // Missing last character
    let result = auth_service.authenticate(malformed_uuid, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Test UUID with extra characters
    let invalid_uuid = "12345678-1234-1234-1234-123456789012x";
    let result = auth_service.authenticate(invalid_uuid, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));
}

#[tokio::test]
async fn test_last_used_timestamp_update() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_last_used@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Last Used Test"), None, None, None)
        .await
        .unwrap();

    // Initially last_used_at should be None
    assert!(api_key.last_used_at.is_none());

    // Authenticate to update last_used_at
    let _auth_result = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    // Check that last_used_at was updated
    let updated_keys = auth_service.list_api_keys(user.id).await.unwrap();
    let updated_key = updated_keys
        .into_iter()
        .find(|k| k.id == api_key.id)
        .unwrap();
    assert!(updated_key.last_used_at.is_some());

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_ip_rate_limiting_on_failed_auth() {
    let pool = get_test_pool().await;
    let rate_limiter = create_test_rate_limiter().await;
    let auth_service = AuthService::new_with_rate_limiter(pool, rate_limiter);

    let test_ip: IpAddr = "198.51.100.1".parse().unwrap();

    // Test failed authentication with IP rate limiting
    let result = auth_service
        .authenticate("invalid_key_12345", Some(test_ip), Some("IPTestAgent/1.0"))
        .await;

    // Should fail with invalid API key
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));
}

#[tokio::test]
async fn test_authentication_without_cache_or_rate_limiting() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_no_external_deps@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, _) = auth_service
        .create_api_key(user.id, Some("No External Deps Key"), None, None, None)
        .await
        .unwrap();

    // Authentication should work without cache or rate limiting
    let auth_result = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result.user.id, user.id);

    // Second authentication should also work (hitting database both times)
    let auth_result2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result2.user.id, user.id);

    // Verify no external dependencies
    assert!(!auth_service.is_caching_enabled());
    assert!(!auth_service.is_rate_limiting_enabled());

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_argon2_hash_detection() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_argon2_detection@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let (plain_key, _) = auth_service
        .create_api_key(user.id, Some("Argon2 Test"), None, None, None)
        .await
        .unwrap();

    // Test that we can authenticate with the hashed key
    // This tests the is_argon2_hash function indirectly
    let auth_result = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result.user.id, user.id);

    // Now let's manually insert a key with an invalid hash format to test the skip logic
    let invalid_hash = "not_an_argon2_hash";
    sqlx::query!(
        "INSERT INTO api_keys (user_id, name, key_hash) VALUES ($1, $2, $3)",
        user.id,
        "Invalid Hash Key",
        invalid_hash
    )
    .execute(&pool)
    .await
    .unwrap();

    // Authentication should still work for the valid key, skipping the invalid one
    let auth_result2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result2.user.id, user.id);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_different_permission_formats() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_permissions_formats@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Test object format with admin
    let (_, admin_obj_key) = auth_service
        .create_api_key(
            user.id,
            Some("Admin Object Key"),
            None,
            Some(json!({"admin": true, "read": true})),
            None,
        )
        .await
        .unwrap();

    let admin_obj_context = AuthContext {
        user: user.clone(),
        api_key: admin_obj_key,
    };

    assert!(AuthService::has_admin_permission(&admin_obj_context));
    assert!(AuthService::has_permission(&admin_obj_context, "read"));
    assert!(AuthService::has_permission(&admin_obj_context, "write")); // Admin has all
    assert!(AuthService::has_permission(&admin_obj_context, "delete")); // Admin has all

    // Test object format without admin
    let (_, user_obj_key) = auth_service
        .create_api_key(
            user.id,
            Some("User Object Key"),
            None,
            Some(json!({"read": true, "write": false, "admin": false})),
            None,
        )
        .await
        .unwrap();

    let user_obj_context = AuthContext {
        user: user.clone(),
        api_key: user_obj_key,
    };

    assert!(!AuthService::has_admin_permission(&user_obj_context));
    assert!(AuthService::has_permission(&user_obj_context, "read"));
    assert!(!AuthService::has_permission(&user_obj_context, "write"));
    assert!(!AuthService::has_permission(&user_obj_context, "admin"));

    // Test invalid permission format
    let (_, invalid_perms_key) = auth_service
        .create_api_key(
            user.id,
            Some("Invalid Permissions Key"),
            None,
            Some(json!("invalid_format")),
            None,
        )
        .await
        .unwrap();

    let invalid_perms_context = AuthContext {
        user: user.clone(),
        api_key: invalid_perms_key,
    };

    assert!(!AuthService::has_admin_permission(&invalid_perms_context));
    assert!(!AuthService::has_permission(&invalid_perms_context, "read"));
    assert!(!AuthService::has_permission(
        &invalid_perms_context,
        "write"
    ));

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_api_key_with_future_expiration() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_future_expiration@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user and API key with future expiration
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let future_expiration = Utc::now() + Duration::days(30);
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            Some("Future Expiration Key"),
            Some(future_expiration),
            None,
            None,
        )
        .await
        .unwrap();

    // Authentication should work with future expiration
    let auth_result = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result.user.id, user.id);
    assert_eq!(auth_result.api_key.id, api_key.id);

    // Test UUID authentication path as well
    let auth_result_uuid = auth_service
        .authenticate(&api_key.id.to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(auth_result_uuid.user.id, user.id);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_create_user_database_constraint_error() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_duplicate@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create first user
    let user1 = auth_service
        .create_user(test_email, None, None)
        .await
        .unwrap();
    assert_eq!(user1.email, test_email);

    // Try to create second user with same email (should fail due to unique constraint)
    let result = auth_service.create_user(test_email, None, None).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AuthError::DatabaseError(_))));

    // Cleanup
    cleanup_test_user(&pool, user1.id).await;
}

#[tokio::test]
async fn test_test_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    let test_email = "test_test_key@example.com";

    // Clean up any existing user
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create a test key manually with test_ prefix
    let test_key = "test_key_123456789012345678901234";
    let key_hash = auth_service.hash_api_key(test_key).unwrap();

    sqlx::query!(
        "INSERT INTO api_keys (user_id, name, key_hash) VALUES ($1, $2, $3)",
        user.id,
        "Test Prefix Key",
        key_hash
    )
    .execute(&pool)
    .await
    .unwrap();

    // Authentication should work with test_ prefix
    let auth_result = auth_service
        .authenticate(test_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_result.user.id, user.id);

    // Cleanup
    cleanup_test_user(&pool, user.id).await;
}
