use redis::aio::ConnectionManager;
use redis::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use self_sensored::services::cache::{
    CacheConfig, CacheEntry, CacheKey, CacheService, CacheStats, generate_query_hash,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: String,
    value: i32,
    name: String,
}

async fn setup_test_cache() -> CacheService {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let config = CacheConfig {
        enabled: true,
        default_ttl_seconds: 5,
        summary_ttl_seconds: 10,
        user_data_ttl_seconds: 8,
        key_prefix: "test_health_export".to_string(),
    };

    CacheService::new(&redis_url, config).await.expect("Failed to create cache service")
}

async fn setup_disabled_cache() -> CacheService {
    let config = CacheConfig {
        enabled: false,
        default_ttl_seconds: 5,
        summary_ttl_seconds: 10,
        user_data_ttl_seconds: 8,
        key_prefix: "test_disabled".to_string(),
    };

    CacheService::new("redis://127.0.0.1:6379", config).await.expect("Failed to create disabled cache")
}

#[tokio::test]
async fn test_cache_service_creation_enabled() {
    let cache = setup_test_cache().await;
    assert!(cache.is_enabled());
}

#[tokio::test]
async fn test_cache_service_creation_disabled() {
    let cache = setup_disabled_cache().await;
    assert!(!cache.is_enabled());
}

#[tokio::test]
async fn test_cache_set_and_get() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();
    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "test_metric".to_string(),
    };

    let test_data = TestData {
        id: "test_123".to_string(),
        value: 42,
        name: "Test Item".to_string(),
    };

    // Set the cache
    let result = cache.set(&key, "test", test_data.clone(), None).await;
    assert!(result);

    // Get from cache
    let cached_data: Option<TestData> = cache.get(&key, "test").await;
    assert_eq!(cached_data, Some(test_data));
}

#[tokio::test]
async fn test_cache_expiration() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();
    let key = CacheKey::HealthSummary {
        user_id,
        date_range: "2024-01-01_2024-01-31".to_string(),
    };

    let test_data = TestData {
        id: "exp_test".to_string(),
        value: 100,
        name: "Expiration Test".to_string(),
    };

    // Set with 1 second TTL
    let result = cache.set(&key, "test", test_data.clone(), Some(Duration::from_secs(1))).await;
    assert!(result);

    // Immediate get should work
    let cached: Option<TestData> = cache.get(&key, "test").await;
    assert_eq!(cached, Some(test_data));

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should return None after expiration
    let expired: Option<TestData> = cache.get(&key, "test").await;
    assert_eq!(expired, None);
}

#[tokio::test]
async fn test_cache_delete() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();
    let key = CacheKey::HeartRateQuery {
        user_id,
        hash: "test_hash_123".to_string(),
    };

    let test_data = TestData {
        id: "delete_test".to_string(),
        value: 55,
        name: "Delete Test".to_string(),
    };

    // Set and verify
    cache.set(&key, "test", test_data, None).await;
    let exists: Option<TestData> = cache.get(&key, "test").await;
    assert!(exists.is_some());

    // Delete
    let deleted = cache.delete(&key, "test").await;
    assert!(deleted);

    // Verify deletion
    let after_delete: Option<TestData> = cache.get(&key, "test").await;
    assert_eq!(after_delete, None);
}

#[tokio::test]
async fn test_cache_key_generation() {
    let user_id = Uuid::new_v4();
    let prefix = "test_prefix";

    // Test all cache key variants
    let keys = vec![
        CacheKey::HeartRateQuery {
            user_id,
            hash: "hr_hash".to_string(),
        },
        CacheKey::BloodPressureQuery {
            user_id,
            hash: "bp_hash".to_string(),
        },
        CacheKey::SleepQuery {
            user_id,
            hash: "sleep_hash".to_string(),
        },
        CacheKey::ActivityQuery {
            user_id,
            hash: "activity_hash".to_string(),
        },
        CacheKey::WorkoutQuery {
            user_id,
            hash: "workout_hash".to_string(),
        },
        CacheKey::MindfulnessQuery {
            user_id,
            hash: "mindfulness_hash".to_string(),
        },
        CacheKey::MentalHealthQuery {
            user_id,
            hash: "mental_hash".to_string(),
        },
        CacheKey::MindfulnessInsights {
            user_id,
            period: "weekly".to_string(),
        },
        CacheKey::MentalHealthInsights {
            user_id,
            period: "monthly".to_string(),
        },
        CacheKey::MindfulnessTrends {
            user_id,
            days: 30,
        },
        CacheKey::HealthSummary {
            user_id,
            date_range: "2024-01".to_string(),
        },
        CacheKey::UserMetrics {
            user_id,
            metric_type: "steps".to_string(),
        },
        CacheKey::ApiKeyAuth {
            api_key_hash: "auth_hash_123".to_string(),
        },
        CacheKey::ApiKeyLookup {
            api_key_id: Uuid::new_v4(),
        },
        CacheKey::ApiKeyAuthHash {
            key_prefix: "prefix".to_string(),
            hash_suffix: "suffix".to_string(),
        },
    ];

    for key in keys {
        let redis_key = key.to_redis_key(prefix);
        assert!(redis_key.starts_with(prefix));
        assert!(redis_key.len() > prefix.len());
    }
}

#[tokio::test]
async fn test_invalidate_user_cache() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();
    let prefix = "test_invalidate";

    // Create multiple cache entries for the user
    let keys = vec![
        CacheKey::HeartRateQuery {
            user_id,
            hash: "hash1".to_string(),
        },
        CacheKey::BloodPressureQuery {
            user_id,
            hash: "hash2".to_string(),
        },
        CacheKey::UserMetrics {
            user_id,
            metric_type: "test".to_string(),
        },
    ];

    let test_data = TestData {
        id: "user_data".to_string(),
        value: 77,
        name: "User Test".to_string(),
    };

    // Set multiple cache entries
    for key in &keys {
        cache.set(key, prefix, test_data.clone(), None).await;
    }

    // Verify all exist
    for key in &keys {
        let exists: Option<TestData> = cache.get(key, prefix).await;
        assert!(exists.is_some());
    }

    // Invalidate all user cache
    let result = cache.invalidate_user_cache(user_id, prefix).await;
    assert!(result);

    // Verify all are gone
    for key in &keys {
        let exists: Option<TestData> = cache.get(key, prefix).await;
        assert!(exists.is_none());
    }
}

#[tokio::test]
async fn test_disabled_cache_operations() {
    let cache = setup_disabled_cache().await;
    let user_id = Uuid::new_v4();
    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "disabled_test".to_string(),
    };

    let test_data = TestData {
        id: "disabled".to_string(),
        value: 0,
        name: "Disabled".to_string(),
    };

    // All operations should return false/None when disabled
    let set_result = cache.set(&key, "test", test_data, None).await;
    assert!(!set_result);

    let get_result: Option<TestData> = cache.get(&key, "test").await;
    assert!(get_result.is_none());

    let delete_result = cache.delete(&key, "test").await;
    assert!(!delete_result);

    let invalidate_result = cache.invalidate_user_cache(user_id, "test").await;
    assert!(!invalidate_result);

    let warm_result = cache.warm_cache(vec![user_id]).await;
    assert!(!warm_result);
}

#[tokio::test]
async fn test_cache_stats() {
    let cache = setup_test_cache().await;

    // Get stats from enabled cache
    let stats = cache.get_stats().await;
    assert!(stats.enabled);
    assert!(!stats.error);

    // Disabled cache stats
    let disabled_cache = setup_disabled_cache().await;
    let disabled_stats = disabled_cache.get_stats().await;
    assert!(!disabled_stats.enabled);
    assert!(!disabled_stats.error);
}

#[tokio::test]
async fn test_cache_stats_hit_rate_calculation() {
    let mut stats = CacheStats {
        hits: 75,
        misses: 25,
        memory_usage: 1024,
        hit_rate: 0.0,
        enabled: true,
        error: false,
    };

    stats.calculate_hit_rate();
    assert_eq!(stats.hit_rate, 75.0);

    // Test with no hits or misses
    let mut empty_stats = CacheStats::default();
    empty_stats.calculate_hit_rate();
    assert_eq!(empty_stats.hit_rate, 0.0);

    // Test with only misses
    let mut miss_stats = CacheStats {
        hits: 0,
        misses: 100,
        memory_usage: 2048,
        hit_rate: 0.0,
        enabled: true,
        error: false,
    };
    miss_stats.calculate_hit_rate();
    assert_eq!(miss_stats.hit_rate, 0.0);
}

#[tokio::test]
async fn test_cache_entry_serialization() {
    let test_data = TestData {
        id: "serial_test".to_string(),
        value: 999,
        name: "Serialization Test".to_string(),
    };

    let entry = CacheEntry {
        data: test_data.clone(),
        cached_at: chrono::Utc::now(),
        ttl_seconds: 300,
    };

    // Serialize
    let serialized = serde_json::to_string(&entry).unwrap();
    assert!(serialized.contains("serial_test"));

    // Deserialize
    let deserialized: CacheEntry<TestData> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.data, test_data);
    assert_eq!(deserialized.ttl_seconds, 300);
}

#[tokio::test]
async fn test_generate_query_hash() {
    use self_sensored::handlers::query::QueryParams;
    use chrono::{DateTime, Utc};

    let params1 = QueryParams {
        start: Some(DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&Utc)),
        end: Some(DateTime::parse_from_rfc3339("2024-01-31T23:59:59Z").unwrap().with_timezone(&Utc)),
        limit: Some(100),
        offset: None,
    };

    let params2 = QueryParams {
        start: Some(DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&Utc)),
        end: Some(DateTime::parse_from_rfc3339("2024-01-31T23:59:59Z").unwrap().with_timezone(&Utc)),
        limit: Some(100),
        offset: None,
    };

    let params3 = QueryParams {
        start: Some(DateTime::parse_from_rfc3339("2024-02-01T00:00:00Z").unwrap().with_timezone(&Utc)),
        end: Some(DateTime::parse_from_rfc3339("2024-02-29T23:59:59Z").unwrap().with_timezone(&Utc)),
        limit: Some(200),
        offset: Some(50),
    };

    let hash1 = generate_query_hash(&params1);
    let hash2 = generate_query_hash(&params2);
    let hash3 = generate_query_hash(&params3);

    // Same parameters should produce same hash
    assert_eq!(hash1, hash2);

    // Different parameters should produce different hash
    assert_ne!(hash1, hash3);

    // Hash should be 16 characters (truncated)
    assert_eq!(hash1.len(), 16);
}

#[tokio::test]
async fn test_cache_warm_cache() {
    let cache = setup_test_cache().await;
    let user_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

    // Currently just returns true for enabled cache
    let result = cache.warm_cache(user_ids).await;
    assert!(result);
}

#[tokio::test]
async fn test_concurrent_cache_operations() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();

    // Spawn multiple concurrent operations
    let mut handles = vec![];

    for i in 0..10 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            let key = CacheKey::UserMetrics {
                user_id,
                metric_type: format!("concurrent_{}", i),
            };

            let test_data = TestData {
                id: format!("concurrent_{}", i),
                value: i as i32,
                name: format!("Concurrent Test {}", i),
            };

            // Set and get
            cache_clone.set(&key, "test", test_data.clone(), None).await;
            let result: Option<TestData> = cache_clone.get(&key, "test").await;
            assert_eq!(result, Some(test_data));
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_cache_with_invalid_data() {
    let cache = setup_test_cache().await;
    let user_id = Uuid::new_v4();
    let key = CacheKey::UserMetrics {
        user_id,
        metric_type: "invalid_test".to_string(),
    };

    // Manually set invalid JSON in Redis to test error handling
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let client = Client::open(redis_url).unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();

    let redis_key = key.to_redis_key("test");
    let invalid_json = "{ invalid json }";
    let _: () = redis::cmd("SET")
        .arg(&redis_key)
        .arg(invalid_json)
        .arg("EX")
        .arg(10)
        .query_async(&mut conn)
        .await
        .unwrap();

    // Try to get the invalid data
    let result: Option<TestData> = cache.get(&key, "test").await;
    assert!(result.is_none()); // Should handle gracefully and return None
}

#[tokio::test]
async fn test_cache_debug_implementation() {
    let cache = setup_test_cache().await;
    let debug_str = format!("{:?}", cache);
    assert!(debug_str.contains("CacheService"));
    assert!(debug_str.contains("default_ttl"));
    assert!(debug_str.contains("enabled"));
    assert!(debug_str.contains("RedisConnectionManager"));

    let config = CacheConfig::default();
    let config_debug = format!("{:?}", config);
    assert!(config_debug.contains("CacheConfig"));
    assert!(config_debug.contains("enabled"));
    assert!(config_debug.contains("default_ttl_seconds"));
}

#[tokio::test]
async fn test_cache_config_default() {
    let config = CacheConfig::default();
    assert!(config.enabled);
    assert_eq!(config.default_ttl_seconds, 300);
    assert_eq!(config.summary_ttl_seconds, 1800);
    assert_eq!(config.user_data_ttl_seconds, 600);
    assert_eq!(config.key_prefix, "health_export");
}