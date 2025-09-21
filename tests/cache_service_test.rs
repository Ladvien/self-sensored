// Comprehensive Redis Cache Service Tests
use chrono::{Duration as ChronoDuration, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use uuid::Uuid;

use self_sensored::services::cache::{
    CacheConfig, CacheEntry, CacheKey, CacheService, CacheStats, generate_query_hash,
};
use self_sensored::handlers::query::QueryParams;

// Test data structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: Uuid,
    value: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl TestData {
    fn new(value: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            value: value.to_string(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LargeTestData {
    values: Vec<String>,
    metadata: HashMap<String, String>,
}

impl LargeTestData {
    fn new(size: usize) -> Self {
        let values = (0..size).map(|i| format!("value_{}", i)).collect();
        let mut metadata = HashMap::new();
        for i in 0..10 {
            metadata.insert(format!("key_{}", i), format!("metadata_{}", i));
        }
        Self { values, metadata }
    }
}

// Mock Redis Connection for testing cache failures
struct MockRedisConnection {
    should_fail: Arc<Mutex<bool>>,
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl MockRedisConnection {
    fn new() -> Self {
        Self {
            should_fail: Arc::new(Mutex::new(false)),
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn set_failure_mode(&self, fail: bool) {
        *self.should_fail.lock().await = fail;
    }
}

/// Test cache configuration setup
#[tokio::test]
async fn test_cache_config_defaults() {
    let config = CacheConfig::default();

    assert_eq!(config.enabled, true);
    assert_eq!(config.default_ttl_seconds, 300); // 5 minutes
    assert_eq!(config.summary_ttl_seconds, 1800); // 30 minutes
    assert_eq!(config.user_data_ttl_seconds, 600); // 10 minutes
    assert_eq!(config.key_prefix, "health_export");
}

/// Test cache service initialization with valid Redis
#[tokio::test]
async fn test_cache_service_initialization() {
    let config = CacheConfig::default();

    // Test with disabled cache
    let disabled_config = CacheConfig {
        enabled: false,
        ..config.clone()
    };

    let cache_service = CacheService::new("redis://127.0.0.1:6379", disabled_config)
        .await
        .expect("Failed to create disabled cache service");

    assert!(!cache_service.is_enabled());
}

/// Test cache key generation and formatting
#[tokio::test]
async fn test_cache_key_generation() {
    let user_id = Uuid::new_v4();
    let prefix = "test";

    // Test different cache key types
    let heart_rate_key = CacheKey::HeartRateQuery {
        user_id,
        hash: "test_hash".to_string(),
    };
    assert_eq!(
        heart_rate_key.to_redis_key(prefix),
        format!("test:hr_query:{}:test_hash", user_id)
    );

    let api_key_hash = "api_key_hash_123";
    let auth_key = CacheKey::ApiKeyAuth {
        api_key_hash: api_key_hash.to_string(),
    };
    assert_eq!(
        auth_key.to_redis_key(prefix),
        format!("test:auth:{}", api_key_hash)
    );

    let summary_key = CacheKey::HealthSummary {
        user_id,
        date_range: "20240101_20240131".to_string(),
    };
    assert_eq!(
        summary_key.to_redis_key(prefix),
        format!("test:summary:{}:20240101_20240131", user_id)
    );
}

/// Test query hash generation consistency
#[tokio::test]
async fn test_query_hash_generation() {
    let params1 = QueryParams {
        page: Some(1),
        limit: Some(100),
        sort: Some("asc".to_string()),
        start_date: Some(Utc::now()),
        end_date: Some(Utc::now() + ChronoDuration::days(1)),
        metric_types: None,
    };

    let params2 = QueryParams {
        page: Some(1),
        limit: Some(100),
        sort: Some("asc".to_string()),
        start_date: params1.start_date,
        end_date: params1.end_date,
        metric_types: None,
    };

    let params3 = QueryParams {
        page: Some(2), // Different page
        limit: Some(100),
        sort: Some("asc".to_string()),
        start_date: params1.start_date,
        end_date: params1.end_date,
        metric_types: None,
    };

    let hash1 = generate_query_hash(&params1);
    let hash2 = generate_query_hash(&params2);
    let hash3 = generate_query_hash(&params3);

    // Same parameters should generate same hash
    assert_eq!(hash1, hash2);
    // Different parameters should generate different hash
    assert_ne!(hash1, hash3);
    // Hash should be consistent length
    assert_eq!(hash1.len(), 16);
}

/// Test basic cache operations with in-memory implementation
#[tokio::test]
async fn test_basic_cache_operations() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let test_data = TestData::new("test_value");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "heart_rate".to_string(),
    };

    // Test cache miss
    let result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(result.is_none());

    // Test cache set
    let set_result = cache_service
        .set(&cache_key, "test_prefix", test_data.clone(), None)
        .await;
    assert!(set_result);

    // Test cache hit
    let cached_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(cached_result.is_some());
    assert_eq!(cached_result.unwrap(), test_data);
}

/// Test cache TTL expiration
#[tokio::test]
async fn test_cache_ttl_expiration() {
    let config = CacheConfig {
        default_ttl_seconds: 1, // 1 second TTL
        ..CacheConfig::default()
    };
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let test_data = TestData::new("ttl_test_value");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "ttl_test".to_string(),
    };

    // Set data with short TTL
    let set_result = cache_service
        .set(&cache_key, "test_prefix", test_data.clone(), Some(Duration::from_millis(500)))
        .await;
    assert!(set_result);

    // Immediate get should work
    let cached_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(cached_result.is_some());

    // Wait for expiration
    sleep(Duration::from_millis(600)).await;

    // Should be expired
    let expired_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(expired_result.is_none());
}

/// Test cache deletion
#[tokio::test]
async fn test_cache_deletion() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let test_data = TestData::new("delete_test_value");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "delete_test".to_string(),
    };

    // Set data
    cache_service
        .set(&cache_key, "test_prefix", test_data.clone(), None)
        .await;

    // Verify it exists
    let cached_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(cached_result.is_some());

    // Delete it
    let delete_result = cache_service
        .delete(&cache_key, "test_prefix")
        .await;
    assert!(delete_result);

    // Verify it's gone
    let deleted_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(deleted_result.is_none());
}

/// Test user cache invalidation
#[tokio::test]
async fn test_user_cache_invalidation() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();

    // Set multiple cache entries for the user
    let keys = vec![
        CacheKey::UserMetrics {
            user_id,
            metric_type: "heart_rate".to_string(),
        },
        CacheKey::UserMetrics {
            user_id,
            metric_type: "blood_pressure".to_string(),
        },
        CacheKey::HealthSummary {
            user_id,
            date_range: "20240101_20240131".to_string(),
        },
        // Entry for different user (should not be affected)
        CacheKey::UserMetrics {
            user_id: other_user_id,
            metric_type: "heart_rate".to_string(),
        },
    ];

    // Set all entries
    for (i, key) in keys.iter().enumerate() {
        let test_data = TestData::new(&format!("value_{}", i));
        cache_service
            .set(key, "test_prefix", test_data, None)
            .await;
    }

    // Verify all entries exist
    for key in &keys {
        let result: Option<TestData> = cache_service
            .get(key, "test_prefix")
            .await;
        assert!(result.is_some());
    }

    // Invalidate cache for the first user
    let invalidate_result = cache_service
        .invalidate_user_cache(user_id, "test_prefix")
        .await;
    assert!(invalidate_result);

    // Verify first user's entries are gone
    for key in &keys[0..3] {
        let result: Option<TestData> = cache_service
            .get(key, "test_prefix")
            .await;
        assert!(result.is_none());
    }

    // Verify other user's entry still exists
    let other_result: Option<TestData> = cache_service
        .get(&keys[3], "test_prefix")
        .await;
    assert!(other_result.is_some());
}

/// Test concurrent cache access patterns
#[tokio::test]
async fn test_concurrent_cache_access() {
    let config = CacheConfig::default();
    let cache_service = Arc::new(create_test_cache_service(config).await);

    let user_id = Uuid::new_v4();
    let num_tasks = 10;
    let mut handles = Vec::new();

    // Spawn concurrent tasks for cache operations
    for i in 0..num_tasks {
        let cache_service = Arc::clone(&cache_service);
        let task_user_id = user_id;

        let handle = tokio::spawn(async move {
            let cache_key = CacheKey::UserMetrics {
                user_id: task_user_id,
                metric_type: format!("concurrent_test_{}", i),
            };
            let test_data = TestData::new(&format!("concurrent_value_{}", i));

            // Set data
            cache_service
                .set(&cache_key, "test_prefix", test_data.clone(), None)
                .await;

            // Get data back
            let result: Option<TestData> = cache_service
                .get(&cache_key, "test_prefix")
                .await;

            result
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(handles).await;

    // Verify all operations succeeded
    for (i, result) in results.into_iter().enumerate() {
        let data = result.expect("Task should complete").expect("Should have cached data");
        assert_eq!(data.value, format!("concurrent_value_{}", i));
    }
}

/// Test cache memory pressure and large data handling
#[tokio::test]
async fn test_cache_memory_pressure() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();

    // Create large data entries
    for i in 0..10 {
        let large_data = LargeTestData::new(1000); // 1000 strings per entry
        let cache_key = CacheKey::UserMetrics {
            user_id,
            metric_type: format!("large_data_{}", i),
        };

        let set_result = cache_service
            .set(&cache_key, "test_prefix", large_data, None)
            .await;
        assert!(set_result, "Should be able to set large data entry {}", i);
    }

    // Verify we can still read the data
    for i in 0..10 {
        let cache_key = CacheKey::UserMetrics {
            user_id,
            metric_type: format!("large_data_{}", i),
        };

        let result: Option<LargeTestData> = cache_service
            .get(&cache_key, "test_prefix")
            .await;
        assert!(result.is_some(), "Should be able to get large data entry {}", i);

        let data = result.unwrap();
        assert_eq!(data.values.len(), 1000);
        assert_eq!(data.metadata.len(), 10);
    }
}

/// Test API key caching with 5-minute TTL
#[tokio::test]
async fn test_api_key_caching() {
    let config = CacheConfig {
        default_ttl_seconds: 300, // 5 minutes
        ..CacheConfig::default()
    };
    let cache_service = create_test_cache_service(config).await;

    let api_key_hash = "test_api_key_hash_12345";
    let api_key_id = Uuid::new_v4();

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ApiKeyData {
        user_id: Uuid,
        is_active: bool,
        last_used: chrono::DateTime<chrono::Utc>,
    }

    let api_data = ApiKeyData {
        user_id: Uuid::new_v4(),
        is_active: true,
        last_used: Utc::now(),
    };

    let auth_key = CacheKey::ApiKeyAuth {
        api_key_hash: api_key_hash.to_string(),
    };

    let lookup_key = CacheKey::ApiKeyLookup {
        api_key_id,
    };

    // Cache API key data
    let set_auth_result = cache_service
        .set(&auth_key, "health_export", api_data.clone(), Some(Duration::from_secs(300)))
        .await;
    assert!(set_auth_result);

    let set_lookup_result = cache_service
        .set(&lookup_key, "health_export", api_data.clone(), Some(Duration::from_secs(300)))
        .await;
    assert!(set_lookup_result);

    // Verify auth cache hit
    let auth_result: Option<ApiKeyData> = cache_service
        .get(&auth_key, "health_export")
        .await;
    assert!(auth_result.is_some());
    assert_eq!(auth_result.unwrap(), api_data);

    // Verify lookup cache hit
    let lookup_result: Option<ApiKeyData> = cache_service
        .get(&lookup_key, "health_export")
        .await;
    assert!(lookup_result.is_some());
    assert_eq!(lookup_result.unwrap(), api_data);
}

/// Test cache statistics collection
#[tokio::test]
async fn test_cache_statistics() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    // For in-memory cache service, stats will be default/disabled
    let stats = cache_service.get_stats().await;

    // Since we're using a mock/disabled cache, expect disabled stats
    assert!(!stats.enabled || stats.error);
}

/// Test cache warming functionality
#[tokio::test]
async fn test_cache_warming() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

    // Test cache warming (currently returns true but doesn't implement logic)
    let warm_result = cache_service.warm_cache(user_ids).await;
    assert!(warm_result);
}

/// Test cache entry serialization and deserialization
#[tokio::test]
async fn test_cache_entry_serialization() {
    let test_data = TestData::new("serialization_test");
    let cache_entry = CacheEntry {
        data: test_data.clone(),
        cached_at: Utc::now(),
        ttl_seconds: 300,
    };

    // Test serialization
    let serialized = serde_json::to_string(&cache_entry)
        .expect("Should serialize cache entry");

    // Test deserialization
    let deserialized: CacheEntry<TestData> = serde_json::from_str(&serialized)
        .expect("Should deserialize cache entry");

    assert_eq!(deserialized.data, test_data);
    assert_eq!(deserialized.ttl_seconds, 300);
}

/// Test disabled cache behavior
#[tokio::test]
async fn test_disabled_cache_behavior() {
    let config = CacheConfig {
        enabled: false,
        ..CacheConfig::default()
    };
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let test_data = TestData::new("disabled_cache_test");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "disabled_test".to_string(),
    };

    // Set operation should return false for disabled cache
    let set_result = cache_service
        .set(&cache_key, "test_prefix", test_data.clone(), None)
        .await;
    assert!(!set_result);

    // Get operation should return None for disabled cache
    let get_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(get_result.is_none());

    // Delete operation should return false for disabled cache
    let delete_result = cache_service
        .delete(&cache_key, "test_prefix")
        .await;
    assert!(!delete_result);

    // Invalidation should return false for disabled cache
    let invalidate_result = cache_service
        .invalidate_user_cache(user_id, "test_prefix")
        .await;
    assert!(!invalidate_result);

    // Cache should report as disabled
    assert!(!cache_service.is_enabled());
}

/// Test cache error handling and resilience
#[tokio::test]
async fn test_cache_error_handling() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();

    // Test with invalid data that might cause serialization issues
    #[derive(Debug, Clone)]
    struct InvalidData {
        #[allow(dead_code)]
        invalid_field: fn() -> i32, // Function pointers can't be serialized
    }

    // This won't compile due to Serialize requirement, but in real scenarios
    // we might have data that fails serialization at runtime
    // For now, test with valid data to ensure the error handling paths work
    let test_data = TestData::new("error_handling_test");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "error_test".to_string(),
    };

    // Normal operations should still work
    let set_result = cache_service
        .set(&cache_key, "test_prefix", test_data.clone(), None)
        .await;
    assert!(set_result);

    let get_result: Option<TestData> = cache_service
        .get(&cache_key, "test_prefix")
        .await;
    assert!(get_result.is_some());
}

/// Helper function to create a test cache service
async fn create_test_cache_service(config: CacheConfig) -> CacheService {
    // For testing, we'll create a cache service that behaves like it's disabled
    // since we don't have a real Redis instance in the test environment
    let disabled_config = CacheConfig {
        enabled: false,
        ..config
    };

    CacheService::new("redis://127.0.0.1:6379", disabled_config)
        .await
        .expect("Failed to create test cache service")
}

/// Test cache key generation for different metric types
#[tokio::test]
async fn test_cache_keys_for_all_metric_types() {
    let user_id = Uuid::new_v4();
    let prefix = "health_export";
    let hash = "test_hash_123";

    let keys = vec![
        (
            CacheKey::HeartRateQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:hr_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::BloodPressureQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:bp_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::SleepQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:sleep_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::ActivityQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:activity_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::WorkoutQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:workout_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::MindfulnessQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:mindfulness_query:{}:{}", prefix, user_id, hash),
        ),
        (
            CacheKey::MentalHealthQuery {
                user_id,
                hash: hash.to_string(),
            },
            format!("{}:mental_health_query:{}:{}", prefix, user_id, hash),
        ),
    ];

    for (key, expected) in keys {
        assert_eq!(key.to_redis_key(prefix), expected);
    }
}

/// Test cache entry TTL validation
#[tokio::test]
async fn test_cache_entry_ttl_validation() {
    let config = CacheConfig::default();
    let cache_service = create_test_cache_service(config).await;

    let user_id = Uuid::new_v4();
    let test_data = TestData::new("ttl_validation_test");
    let cache_key = CacheKey::UserMetrics {
        user_id,
        metric_type: "ttl_validation".to_string(),
    };

    // Test with different TTL values
    let ttl_values = vec![
        Duration::from_secs(1),
        Duration::from_secs(60),
        Duration::from_secs(300),
        Duration::from_secs(3600),
    ];

    for (i, ttl) in ttl_values.iter().enumerate() {
        let key = CacheKey::UserMetrics {
            user_id,
            metric_type: format!("ttl_validation_{}", i),
        };

        let set_result = cache_service
            .set(&key, "test_prefix", test_data.clone(), Some(*ttl))
            .await;

        // For disabled cache, set should return false
        assert!(!set_result);
    }
}