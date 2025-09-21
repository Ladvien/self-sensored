use chrono::{Duration, Utc};
use self_sensored::services::auth::{AuthError, AuthService};
use self_sensored::services::rate_limiter::{RateLimiter, RateLimitError};
use sqlx::postgres::PgPoolOptions;
use std::net::IpAddr;
use uuid::Uuid;

async fn get_test_pool() -> sqlx::PgPool {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_user(pool: &sqlx::PgPool, user_id: Uuid) {
    // Clean up API keys first (foreign key constraint)
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    
    // Clean up user
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

async fn create_test_user(auth_service: &AuthService, email: &str) -> Result<self_sensored::services::auth::User, AuthError> {
    auth_service.create_user(email, Some("Test User")).await
}

#[tokio::test]
async fn test_api_key_generation_uniqueness() {
    let key1 = AuthService::generate_api_key();
    let key2 = AuthService::generate_api_key();
    let key3 = AuthService::generate_api_key();

    // Keys should be different
    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);

    // Keys should have correct prefix
    assert!(key1.starts_with("hea_"));
    assert!(key2.starts_with("hea_"));
    assert!(key3.starts_with("hea_"));

    // Keys should be proper length (hea_ + 32 hex chars)
    assert_eq!(key1.len(), 36);
    assert_eq!(key2.len(), 36);
    assert_eq!(key3.len(), 36);
}

#[tokio::test]
async fn test_argon2_hashing_security() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let api_key = "test_api_key_12345";

    // Hash the same key multiple times
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
async fn test_uuid_authentication_with_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Clean up any existing test user
    let test_email = "uuid_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    // Create user
    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let test_ip: IpAddr = "127.0.0.1".parse().unwrap();
    let test_user_agent = "TestAgent/1.0";

    // Insert a UUID-based API key directly (simulating Auto Export key)
    let api_key_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, scopes, is_active)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        api_key_id,
        user.id,
        "Auto Export Key",
        &vec!["read".to_string(), "write".to_string()],
        true
    )
    .execute(&pool)
    .await
    .unwrap();

    // Authenticate using UUID directly (Auto Export format)
    let auth_context = auth_service
        .authenticate(&api_key_id.to_string(), Some(test_ip), Some(test_user_agent))
        .await
        .unwrap();

    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.name, "Auto Export Key");
    assert_eq!(auth_context.api_key.id, api_key_id);
    assert_eq!(
        auth_context.api_key.permissions,
        Some(serde_json::json!(["read", "write"]))
    );

    // Verify audit log was created
    let audit_logs = sqlx::query!(
        "SELECT action, resource, metadata FROM audit_log WHERE user_id = $1 AND api_key_id = $2 ORDER BY created_at DESC LIMIT 1",
        user.id,
        api_key_id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(audit_logs.is_some());
    let audit_log = audit_logs.unwrap();
    assert_eq!(audit_log.action, "authentication_success");
    assert_eq!(audit_log.resource, Some("uuid_api_key".to_string()));

    // Verify metadata contains expected fields
    if let Some(metadata) = audit_log.metadata {
        let meta_obj = metadata.as_object().unwrap();
        assert_eq!(meta_obj["key_type"], "uuid");
        assert_eq!(meta_obj["key_name"], "Auto Export Key");
    }

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_hashed_key_authentication_with_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Clean up any existing test user
    let test_email = "hashed_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let test_ip: IpAddr = "192.168.1.100".parse().unwrap();
    let test_user_agent = "TestClient/2.0";

    // Create API key using the service (generates hashed key)
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Test Hashed Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Authenticate using the plain key
    let auth_context = auth_service
        .authenticate(&plain_key, Some(test_ip), Some(test_user_agent))
        .await
        .unwrap();

    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.name, "Test Hashed Key");
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Verify audit log was created
    let audit_logs = sqlx::query!(
        "SELECT action, resource, metadata FROM audit_log WHERE user_id = $1 AND api_key_id = $2 ORDER BY created_at DESC LIMIT 1",
        user.id,
        api_key.id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(audit_logs.is_some());
    let audit_log = audit_logs.unwrap();
    assert_eq!(audit_log.action, "authentication_success");
    assert_eq!(audit_log.resource, Some("hashed_api_key".to_string()));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_expired_key_authentication_with_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "expired_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();
    let test_ip: IpAddr = "10.0.0.1".parse().unwrap();

    // Create expired API key
    let expired_time = Utc::now() - Duration::hours(1);
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            "Expired Key",
            Some(expired_time),
            vec!["read".to_string()],
        )
        .await
        .unwrap();

    // Try to authenticate with expired key
    let result = auth_service
        .authenticate(&plain_key, Some(test_ip), Some("ExpiredTest/1.0"))
        .await;

    assert!(matches!(result, Err(AuthError::ApiKeyExpired)));

    // Verify failure audit log was created
    let audit_logs = sqlx::query!(
        "SELECT action, resource, metadata FROM audit_log WHERE user_id = $1 AND api_key_id = $2 AND action = 'authentication_failed' ORDER BY created_at DESC LIMIT 1",
        user.id,
        api_key.id
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(audit_logs.is_some());
    let audit_log = audit_logs.unwrap();
    assert_eq!(audit_log.action, "authentication_failed");
    assert_eq!(audit_log.resource, Some("api_key_expired".to_string()));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_invalid_key_authentication_with_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_ip: IpAddr = "172.16.0.1".parse().unwrap();
    let test_user_agent = "InvalidTest/1.0";

    // Try to authenticate with completely invalid key
    let result = auth_service
        .authenticate("totally_invalid_key", Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Try to authenticate with valid UUID format but non-existent key
    let fake_uuid = Uuid::new_v4();
    let result = auth_service
        .authenticate(&fake_uuid.to_string(), Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Try to authenticate with valid hea_ format but non-existent key
    let fake_hea_key = "hea_nonexistent12345678901234567890";
    let result = auth_service
        .authenticate(fake_hea_key, Some(test_ip), Some(test_user_agent))
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Check that audit logs were created for failed attempts
    let audit_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE action = 'authentication_failed' AND resource = 'invalid_api_key'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Should have at least the attempts we just made
    assert!(audit_count.unwrap_or(0) >= 3);
}

#[tokio::test]
async fn test_inactive_user_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "inactive_user@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Deactivate the user
    sqlx::query!("UPDATE users SET is_active = false WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();

    // Try to authenticate - should fail because user is inactive
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_inactive_api_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "inactive_key@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First authenticate successfully
    let auth_context = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Deactivate the API key
    sqlx::query!(
        "UPDATE api_keys SET is_active = false WHERE id = $1",
        api_key.id
    )
    .execute(&pool)
    .await
    .unwrap();

    // Try to authenticate - should fail because API key is inactive
    let result = auth_service.authenticate(&plain_key, None, None).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_api_key_revocation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "revocation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (_plain_key, api_key) = auth_service
        .create_api_key(user.id, "Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Revoke the API key
    let revoked = auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();
    assert!(revoked);

    // Try to revoke non-existent key
    let fake_key_id = Uuid::new_v4();
    let not_revoked = auth_service
        .revoke_api_key(fake_key_id, user.id)
        .await
        .unwrap();
    assert!(!not_revoked);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_list_api_keys() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "list_keys@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create multiple API keys
    let (_key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (_key2, api_key2) = auth_service
        .create_api_key(
            user.id,
            "Key 2",
            Some(Utc::now() + Duration::days(30)),
            vec!["read".to_string(), "write".to_string()],
        )
        .await
        .unwrap();

    // List API keys
    let api_keys = auth_service.list_api_keys(user.id).await.unwrap();

    assert_eq!(api_keys.len(), 2);
    assert!(api_keys.iter().any(|k| k.id == api_key1.id));
    assert!(api_keys.iter().any(|k| k.id == api_key2.id));

    // Keys should be ordered by created_at DESC (newest first)
    assert!(api_keys[0].created_at >= api_keys[1].created_at);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_last_used_timestamp_update() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "last_used@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Initially, last_used_at should be None
    assert!(api_key.last_used_at.is_none());

    // Authenticate - this should update last_used_at
    let _auth_context = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    // Check that last_used_at was updated
    let updated_keys = auth_service.list_api_keys(user.id).await.unwrap();
    let updated_key = updated_keys.iter().find(|k| k.id == api_key.id).unwrap();
    
    assert!(updated_key.last_used_at.is_some());
    assert!(updated_key.last_used_at.unwrap() > api_key.created_at.unwrap());

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_comprehensive_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_ip: IpAddr = "203.0.113.1".parse().unwrap();
    let test_user_agent = "AuditTest/1.0";

    // Test direct audit log creation
    let test_user_id = Uuid::new_v4();
    let test_key_id = Uuid::new_v4();
    let test_metadata = serde_json::json!({
        "test_field": "test_value",
        "numeric_field": 42
    });

    auth_service
        .log_audit_event(
            Some(test_user_id),
            Some(test_key_id),
            "test_action",
            Some("test_resource"),
            Some(test_ip),
            Some(test_user_agent),
            Some(test_metadata.clone()),
        )
        .await
        .unwrap();

    // Verify the audit log was created
    let audit_log = sqlx::query!(
        r#"
        SELECT user_id, api_key_id, action, resource, ip_address, user_agent, metadata
        FROM audit_log 
        WHERE user_id = $1 AND api_key_id = $2 AND action = 'test_action'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        test_user_id,
        test_key_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(audit_log.user_id, Some(test_user_id));
    assert_eq!(audit_log.api_key_id, Some(test_key_id));
    assert_eq!(audit_log.action, "test_action");
    assert_eq!(audit_log.resource, Some("test_resource".to_string()));
    assert_eq!(audit_log.user_agent, Some(test_user_agent.to_string()));
    assert!(audit_log.ip_address.is_some());
    assert_eq!(audit_log.metadata, Some(test_metadata));
}

#[tokio::test] 
async fn test_performance_authentication_timing() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let test_email = "perf_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(&pool, existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Perf Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Test multiple authentication calls to ensure consistent performance
    let mut total_duration = std::time::Duration::from_nanos(0);
    let iterations = 10;

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let _auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        let duration = start.elapsed();
        total_duration += duration;
    }

    let average_duration = total_duration / iterations;
    
    // Requirement: <10ms for authentication check
    assert!(
        average_duration.as_millis() < 10,
        "Average authentication time {}ms exceeds 10ms requirement",
        average_duration.as_millis()
    );

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with very low limit for testing
    let rate_limiter = RateLimiter::new_in_memory(2); // Only 2 requests per hour
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // First request should succeed
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second request should succeed
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Third request should fail due to rate limiting
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));

    // Check rate limit status
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 2);
    assert!(status.retry_after.is_some());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_disabled() {
    let pool = get_test_pool().await;
    
    // Create auth service without rate limiting
    let auth_service = AuthService::new(pool);

    let test_email = "no_rate_limit@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Unlimited Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Should be able to authenticate many times without rate limiting
    for _ in 0..10 {
        let auth_context = auth_service
            .authenticate(&plain_key, None, None)
            .await
            .unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
    }

    // Rate limit status should be None when not enabled
    let status = auth_service
        .get_rate_limit_status(api_key.id)
        .await
        .unwrap();
    assert!(status.is_none());

    assert!(!auth_service.is_rate_limiting_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key_isolation() {
    let pool = get_test_pool().await;
    
    // Create rate limiter with limit of 1 request per hour for testing
    let rate_limiter = RateLimiter::new_in_memory(1);
    let auth_service = AuthService::new_with_rate_limiter(pool, Some(rate_limiter));

    let test_email = "isolation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create two different API keys for the same user
    let (key1, api_key1) = auth_service
        .create_api_key(user.id, "Key 1", None, vec!["read".to_string()])
        .await
        .unwrap();

    let (key2, api_key2) = auth_service
        .create_api_key(user.id, "Key 2", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Use up the rate limit for key1
    let _auth_context1 = auth_service
        .authenticate(&key1, None, None)
        .await
        .unwrap();

    // key1 should now be rate limited
    let result1 = auth_service.authenticate(&key1, None, None).await;
    assert!(matches!(result1, Err(AuthError::RateLimitExceeded(_))));

    // key2 should still work (separate rate limit)
    let auth_context2 = auth_service
        .authenticate(&key2, None, None)
        .await
        .unwrap();
    assert_eq!(auth_context2.api_key.id, api_key2.id);

    // key2 should now also be rate limited
    let result2 = auth_service.authenticate(&key2, None, None).await;
    assert!(matches!(result2, Err(AuthError::RateLimitExceeded(_))));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}