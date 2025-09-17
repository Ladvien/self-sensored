use chrono::{Duration, Utc};
use self_sensored::services::auth::{AuthError, AuthService};
use self_sensored::services::cache::{CacheConfig, CacheService};
use self_sensored::services::rate_limiter::RateLimiter;
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

async fn get_test_cache_service() -> CacheService {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let cache_config = CacheConfig {
        enabled: true,
        default_ttl_seconds: 300, // 5 minutes
        summary_ttl_seconds: 1800,
        user_data_ttl_seconds: 600,
        key_prefix: "test_health_export".to_string(),
    };

    CacheService::new(&redis_url, cache_config)
        .await
        .expect("Failed to connect to Redis for testing")
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
    auth_service.create_user(email, Some("Test User"), None).await
}

#[tokio::test]
async fn test_authentication_cache_hit() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool, None, Some(cache_service));

    let test_email = "cache_hit_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Cache Test Key"), None, None, None)
        .await
        .unwrap();

    // First authentication - should cache the result
    let start_time = std::time::Instant::now();
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    let first_auth_duration = start_time.elapsed();

    assert_eq!(auth_context1.user.email, test_email);
    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second authentication - should be faster due to cache hit
    let start_time = std::time::Instant::now();
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();
    let second_auth_duration = start_time.elapsed();

    assert_eq!(auth_context2.user.email, test_email);
    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Second authentication should be significantly faster (cache hit)
    // Allow some variance but expect at least 2x improvement
    assert!(
        second_auth_duration < first_auth_duration / 2,
        "Cache hit should be faster: first={}ms, second={}ms",
        first_auth_duration.as_millis(),
        second_auth_duration.as_millis()
    );

    // Verify cache is enabled
    assert!(auth_service.is_caching_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_authentication_cache_expiration() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool, None, Some(cache_service));

    let test_email = "cache_expiration_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Cache Expiration Test Key"), None, None, None)
        .await
        .unwrap();

    // First authentication - should cache the result
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Wait for cache to expire (this test assumes a very short TTL for testing)
    // In practice, you might want to mock time or use a shorter TTL for testing
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Authentication after expiration should still work (database fallback)
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_authentication_cache_invalidation_on_revoke() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool, None, Some(cache_service));

    let test_email = "cache_invalidation_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Cache Invalidation Test Key"), None, None, None)
        .await
        .unwrap();

    // First authentication - should cache the result
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Revoke the API key - should invalidate cache
    let was_revoked = auth_service
        .revoke_api_key(api_key.id, user.id)
        .await
        .unwrap();

    assert!(was_revoked);

    // Authentication should now fail even though it was cached before
    let result = auth_service
        .authenticate(&plain_key, None, None)
        .await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_authentication_without_cache() {
    let pool = get_test_pool().await;
    // Create auth service without cache
    let auth_service = AuthService::new(pool);

    let test_email = "no_cache_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("No Cache Test Key"), None, None, None)
        .await
        .unwrap();

    // Authentication should work without cache
    let auth_context = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.id, api_key.id);

    // Verify cache is disabled
    assert!(!auth_service.is_caching_enabled());

    // Cache stats should return None
    let cache_stats = auth_service.get_cache_stats().await;
    assert!(cache_stats.is_none());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_cache_with_rate_limiting() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let rate_limiter = RateLimiter::new_in_memory(5); // 5 requests per hour
    let auth_service = AuthService::new_with_cache(pool, Some(rate_limiter), Some(cache_service));

    let test_email = "cache_rate_limit_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, Some("Cache Rate Limit Test Key"), None, None, None)
        .await
        .unwrap();

    // First authentication - should cache and count towards rate limit
    let auth_context1 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context1.api_key.id, api_key.id);

    // Second authentication - cache hit, but should still check rate limit
    let auth_context2 = auth_service
        .authenticate(&plain_key, None, None)
        .await
        .unwrap();

    assert_eq!(auth_context2.api_key.id, api_key.id);

    // Verify both rate limiting and caching are enabled
    assert!(auth_service.is_rate_limiting_enabled());
    assert!(auth_service.is_caching_enabled());

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_uuid_api_key_caching() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool, None, Some(cache_service));

    let test_email = "uuid_cache_test@example.com";
    if let Ok(Some(existing_user)) = auth_service.get_user_by_email(test_email).await {
        cleanup_test_user(auth_service.pool(), existing_user.id).await;
    }

    let user = create_test_user(&auth_service, test_email).await.unwrap();

    // Insert a UUID-based API key directly (simulating Auto Export key)
    let api_key_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, is_active)
        VALUES ($1, $2, $3, $4)
        "#,
        api_key_id,
        user.id,
        "Auto Export UUID Key",
        true
    )
    .execute(auth_service.pool())
    .await
    .unwrap();

    // First authentication using UUID - should cache the result
    let start_time = std::time::Instant::now();
    let auth_context1 = auth_service
        .authenticate(&api_key_id.to_string(), None, None)
        .await
        .unwrap();
    let first_auth_duration = start_time.elapsed();

    assert_eq!(auth_context1.user.email, test_email);
    assert_eq!(auth_context1.api_key.id, api_key_id);

    // Second authentication - should be faster due to cache hit
    let start_time = std::time::Instant::now();
    let auth_context2 = auth_service
        .authenticate(&api_key_id.to_string(), None, None)
        .await
        .unwrap();
    let second_auth_duration = start_time.elapsed();

    assert_eq!(auth_context2.user.email, test_email);
    assert_eq!(auth_context2.api_key.id, api_key_id);

    // Second authentication should be faster (cache hit)
    assert!(
        second_auth_duration < first_auth_duration,
        "Cache hit should be faster: first={}ms, second={}ms",
        first_auth_duration.as_millis(),
        second_auth_duration.as_millis()
    );

    // Clean up
    cleanup_test_user(auth_service.pool(), user.id).await;
}

#[tokio::test]
async fn test_cache_stats() {
    let pool = get_test_pool().await;
    let cache_service = get_test_cache_service().await;
    let auth_service = AuthService::new_with_cache(pool, None, Some(cache_service));

    // Should be able to get cache stats
    let cache_stats = auth_service.get_cache_stats().await;
    assert!(cache_stats.is_some());

    let stats = cache_stats.unwrap();
    assert!(stats.enabled);
    assert!(!stats.error);
}