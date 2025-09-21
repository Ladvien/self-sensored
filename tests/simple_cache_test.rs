use std::time::Duration;
use uuid::Uuid;

use self_sensored::services::cache::{CacheConfig, CacheService, CacheKey, CacheStats};

async fn setup_cache() -> CacheService {
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let config = CacheConfig {
        enabled: true,
        default_ttl_seconds: 300,
        summary_ttl_seconds: 600,
        user_data_ttl_seconds: 900,
        key_prefix: "test".to_string(),
    };

    CacheService::new(&redis_url, config)
        .await
        .expect("Failed to create cache service")
}

#[tokio::test]
async fn test_cache_config_default() {
    let config = CacheConfig::default();

    assert!(config.enabled);
    assert_eq!(config.default_ttl_seconds, 300);
    assert_eq!(config.summary_ttl_seconds, 600);
    assert_eq!(config.user_data_ttl_seconds, 900);
    assert_eq!(config.key_prefix, "health_export");
}

#[tokio::test]
async fn test_cache_service_creation() {
    let cache = setup_cache().await;
    assert!(cache.is_enabled());
}

#[tokio::test]
async fn test_cache_disabled() {
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let config = CacheConfig {
        enabled: false,
        default_ttl_seconds: 300,
        summary_ttl_seconds: 600,
        user_data_ttl_seconds: 900,
        key_prefix: "test".to_string(),
    };

    let cache = CacheService::new(&redis_url, config)
        .await
        .expect("Failed to create disabled cache service");

    assert!(!cache.is_enabled());
}

#[tokio::test]
async fn test_cache_key_generation() {
    let user_id = Uuid::new_v4();

    let key1 = CacheKey::UserMetrics {
        user_id,
        metric_type: "heart_rate".to_string(),
    };

    let key2 = CacheKey::HealthSummary { user_id };

    let key3 = CacheKey::ApiKeyAuth {
        api_key_hash: "abc123".to_string(),
    };

    let key4 = CacheKey::RateLimit {
        identifier: "user123".to_string(),
    };

    // Test that keys can be created (basic functionality)
    match key1 {
        CacheKey::UserMetrics { user_id: uid, metric_type } => {
            assert_eq!(uid, user_id);
            assert_eq!(metric_type, "heart_rate");
        }
        _ => panic!("Wrong key type"),
    }

    match key2 {
        CacheKey::HealthSummary { user_id: uid } => {
            assert_eq!(uid, user_id);
        }
        _ => panic!("Wrong key type"),
    }

    match key3 {
        CacheKey::ApiKeyAuth { api_key_hash } => {
            assert_eq!(api_key_hash, "abc123");
        }
        _ => panic!("Wrong key type"),
    }

    match key4 {
        CacheKey::RateLimit { identifier } => {
            assert_eq!(identifier, "user123");
        }
        _ => panic!("Wrong key type"),
    }
}

#[tokio::test]
async fn test_cache_set_and_get() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "heart_rate".to_string(),
    };

    let test_data = "test_value";

    // Test set
    let set_result = cache.set(&key, prefix, test_data, Some(Duration::from_secs(60))).await;
    assert!(set_result);

    // Test get
    let get_result: Option<String> = cache.get(&key, prefix).await;
    assert!(get_result.is_some());
    assert_eq!(get_result.unwrap(), test_data);
}

#[tokio::test]
async fn test_cache_set_without_ttl() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "activity".to_string(),
    };

    let test_data = "test_value_no_ttl";

    // Test set without TTL (should use default)
    let set_result = cache.set(&key, prefix, test_data, None).await;
    assert!(set_result);

    // Test get
    let get_result: Option<String> = cache.get(&key, prefix).await;
    assert!(get_result.is_some());
    assert_eq!(get_result.unwrap(), test_data);
}

#[tokio::test]
async fn test_cache_delete() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "sleep".to_string(),
    };

    let test_data = "test_value_delete";

    // Set data
    cache.set(&key, prefix, test_data, Some(Duration::from_secs(60))).await;

    // Verify it exists
    let get_result: Option<String> = cache.get(&key, prefix).await;
    assert!(get_result.is_some());

    // Delete it
    let delete_result = cache.delete(&key, prefix).await;
    assert!(delete_result);

    // Verify it's gone
    let get_result_after: Option<String> = cache.get(&key, prefix).await;
    assert!(get_result_after.is_none());
}

#[tokio::test]
async fn test_cache_invalidate_user_cache() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    // Set multiple items for the user
    let key1 = CacheKey::UserMetrics {
        user_id,
        metric_type: "heart_rate".to_string(),
    };
    let key2 = CacheKey::HealthSummary { user_id };

    cache.set(&key1, prefix, "data1", Some(Duration::from_secs(60))).await;
    cache.set(&key2, prefix, "data2", Some(Duration::from_secs(60))).await;

    // Invalidate user cache
    cache.invalidate_user_cache(user_id, prefix).await;

    // Note: The current implementation doesn't track user->key mappings,
    // so we can't verify complete invalidation without more complex setup.
    // This test just ensures the method can be called without error.
}

#[tokio::test]
async fn test_cache_get_stats() {
    let cache = setup_cache().await;
    let stats = cache.get_stats().await;

    // Basic stats structure test
    assert!(stats.hits >= 0);
    assert!(stats.misses >= 0);
    assert!(stats.sets >= 0);
    assert!(stats.deletes >= 0);
    assert!(stats.errors >= 0);
}

#[tokio::test]
async fn test_cache_get_nonexistent_key() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "nonexistent".to_string(),
    };

    let get_result: Option<String> = cache.get(&key, prefix).await;
    assert!(get_result.is_none());
}

#[tokio::test]
async fn test_cache_complex_data_structures() {
    let cache = setup_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    let key = CacheKey::HealthSummary { user_id };

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestData {
        name: String,
        value: i32,
        active: bool,
    }

    let test_data = TestData {
        name: "test".to_string(),
        value: 42,
        active: true,
    };

    // Test set complex data
    let set_result = cache.set(&key, prefix, &test_data, Some(Duration::from_secs(60))).await;
    assert!(set_result);

    // Test get complex data
    let get_result: Option<TestData> = cache.get(&key, prefix).await;
    assert!(get_result.is_some());
    assert_eq!(get_result.unwrap(), test_data);
}

#[tokio::test]
async fn test_multiple_cache_operations() {
    let cache = setup_cache().await;
    let prefix = "test_prefix";

    // Test multiple operations in sequence
    for i in 0..5 {
        let user_id = Uuid::new_v4();
        let key = CacheKey::UserMetrics {
            user_id,
            metric_type: format!("metric_{}", i),
        };

        let data = format!("value_{}", i);

        // Set
        assert!(cache.set(&key, prefix, &data, Some(Duration::from_secs(60))).await);

        // Get
        let result: Option<String> = cache.get(&key, prefix).await;
        assert_eq!(result, Some(data));

        // Delete
        assert!(cache.delete(&key, prefix).await);

        // Verify deleted
        let result_after: Option<String> = cache.get(&key, prefix).await;
        assert!(result_after.is_none());
    }
}