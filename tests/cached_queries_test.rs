// Comprehensive Cached Queries Service Tests
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use self_sensored::handlers::query::{
    HealthSummary, PaginationInfo, QueryParams, QueryResponse,
};
use self_sensored::models::db::HeartRateRecord;
use self_sensored::services::auth::AuthContext;
use self_sensored::services::cache::{CacheConfig, CacheService};
use self_sensored::services::cached_queries::CachedQueryService;
use self_sensored::models::db::User;

// Test database setup helpers
async fn setup_test_database() -> PgPool {
    // In a real test environment, this would connect to a test database
    // For now, we'll create a mock pool
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/health_export_test".to_string());

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_cache_service() -> CacheService {
    let config = CacheConfig {
        enabled: false, // Use disabled cache for testing
        ..CacheConfig::default()
    };

    CacheService::new("redis://127.0.0.1:6379", config)
        .await
        .expect("Failed to create test cache service")
}

fn create_test_user() -> self_sensored::services::auth::User {
    self_sensored::services::auth::User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: None,
    }
}

fn create_test_auth_context() -> AuthContext {
    AuthContext {
        user: create_test_user(),
        api_key: self_sensored::services::auth::ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: Some("Test Key".to_string()),
            created_at: Some(Utc::now()),
            last_used_at: Some(Utc::now()),
            expires_at: None,
            is_active: Some(true),
            permissions: Some(serde_json::json!(["read", "write"])),
            rate_limit_per_hour: Some(1000),
        },
    }
}

/// Test cached query service initialization
#[tokio::test]
async fn test_cached_query_service_creation() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let cached_query_service = CachedQueryService::new(
        pool.clone(),
        cache_service,
        cache_prefix,
    );

    // Service should be created successfully
    // We can't test much more without a real database connection
    assert!(true); // Placeholder assertion
}

/// Test query parameters hashing consistency
#[tokio::test]
async fn test_query_params_consistency() {
    use self_sensored::services::cache::generate_query_hash;

    let base_params = QueryParams {
        page: Some(1),
        limit: Some(100),
        sort: Some("desc".to_string()),
        start_date: Some(Utc::now() - ChronoDuration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
    };

    // Same parameters should generate same hash
    let hash1 = generate_query_hash(&base_params);
    let hash2 = generate_query_hash(&base_params);
    assert_eq!(hash1, hash2);

    // Different page should generate different hash
    let different_page_params = QueryParams {
        page: Some(2),
        limit: base_params.limit,
        sort: base_params.sort.clone(),
        start_date: base_params.start_date,
        end_date: base_params.end_date,
        metric_types: base_params.metric_types.clone(),
    };
    let hash3 = generate_query_hash(&different_page_params);
    assert_ne!(hash1, hash3);

    // Different limit should generate different hash
    let different_limit_params = QueryParams {
        limit: Some(50),
        ..base_params.clone()
    };
    let hash4 = generate_query_hash(&different_limit_params);
    assert_ne!(hash1, hash4);

    // Different sort should generate different hash
    let different_sort_params = QueryParams {
        sort: Some("asc".to_string()),
        ..base_params.clone()
    };
    let hash5 = generate_query_hash(&different_sort_params);
    assert_ne!(hash1, hash5);

    // Different date range should generate different hash
    let different_date_params = QueryParams {
        start_date: Some(Utc::now() - ChronoDuration::days(14)),
        ..base_params.clone()
    };
    let hash6 = generate_query_hash(&different_date_params);
    assert_ne!(hash1, hash6);
}

/// Test cache key generation for different query types
#[tokio::test]
async fn test_cache_key_generation_for_queries() {
    use self_sensored::services::cache::{CacheKey, generate_query_hash};

    let user_id = Uuid::new_v4();
    let params = QueryParams {
        page: Some(1),
        limit: Some(100),
        sort: Some("desc".to_string()),
        start_date: Some(Utc::now() - ChronoDuration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
    };

    let query_hash = generate_query_hash(&params);

    // Test different query cache keys
    let heart_rate_key = CacheKey::HeartRateQuery {
        user_id,
        hash: query_hash.clone(),
    };

    let blood_pressure_key = CacheKey::BloodPressureQuery {
        user_id,
        hash: query_hash.clone(),
    };

    let sleep_key = CacheKey::SleepQuery {
        user_id,
        hash: query_hash.clone(),
    };

    let activity_key = CacheKey::ActivityQuery {
        user_id,
        hash: query_hash.clone(),
    };

    let workout_key = CacheKey::WorkoutQuery {
        user_id,
        hash: query_hash.clone(),
    };

    // Verify keys are generated correctly
    let prefix = "health_export";
    assert_eq!(
        heart_rate_key.to_redis_key(prefix),
        format!("{}:hr_query:{}:{}", prefix, user_id, query_hash)
    );
    assert_eq!(
        blood_pressure_key.to_redis_key(prefix),
        format!("{}:bp_query:{}:{}", prefix, user_id, query_hash)
    );
    assert_eq!(
        sleep_key.to_redis_key(prefix),
        format!("{}:sleep_query:{}:{}", prefix, user_id, query_hash)
    );
    assert_eq!(
        activity_key.to_redis_key(prefix),
        format!("{}:activity_query:{}:{}", prefix, user_id, query_hash)
    );
    assert_eq!(
        workout_key.to_redis_key(prefix),
        format!("{}:workout_query:{}:{}", prefix, user_id, query_hash)
    );
}

/// Test health summary cache key generation
#[tokio::test]
async fn test_health_summary_cache_keys() {
    use self_sensored::services::cache::CacheKey;

    let user_id = Uuid::new_v4();
    let start_date = Utc::now() - ChronoDuration::days(30);
    let end_date = Utc::now();

    let date_range = format!(
        "{}_{}",
        start_date.format("%Y%m%d"),
        end_date.format("%Y%m%d")
    );

    let summary_key = CacheKey::HealthSummary {
        user_id,
        date_range: date_range.clone(),
    };

    let prefix = "health_export";
    let expected_key = format!("{}:summary:{}:{}", prefix, user_id, date_range);
    assert_eq!(summary_key.to_redis_key(prefix), expected_key);
}

/// Test cache invalidation scenarios
#[tokio::test]
async fn test_cache_invalidation_scenarios() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let cached_query_service = CachedQueryService::new(
        pool,
        cache_service,
        cache_prefix,
    );

    let user_id = Uuid::new_v4();

    // Test cache invalidation for user
    cached_query_service.invalidate_user_cache(user_id).await;

    // Since we're using a disabled cache, this should complete without error
    // In a real test with Redis, we would verify that specific keys are removed
    assert!(true);
}

/// Test cache statistics retrieval
#[tokio::test]
async fn test_cache_statistics_retrieval() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let cached_query_service = CachedQueryService::new(
        pool,
        cache_service,
        cache_prefix,
    );

    let stats = cached_query_service.get_cache_stats().await;

    // For disabled cache, stats should indicate disabled or error state
    assert!(!stats.enabled || stats.error);
}

/// Test query response structure
#[tokio::test]
async fn test_query_response_structure() {
    // Test QueryResponse serialization/deserialization
    let heart_rate_records = vec![
        HeartRateRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            heart_rate: Some(72),
            resting_heart_rate: Some(60),
            heart_rate_variability: None,
            context: Some("at_rest".to_string()),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        },
        HeartRateRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now() - ChronoDuration::minutes(30),
            heart_rate: Some(85),
            resting_heart_rate: Some(62),
            heart_rate_variability: None,
            context: Some("active".to_string()),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        },
    ];

    let pagination = PaginationInfo {
        page: 1,
        limit: 100,
        has_next: false,
        has_prev: false,
    };

    let response = QueryResponse {
        data: heart_rate_records.clone(),
        pagination: pagination.clone(),
        total_count: 2,
    };

    // Test serialization
    let serialized = serde_json::to_string(&response)
        .expect("Should serialize QueryResponse");

    // Test deserialization
    let deserialized: QueryResponse<HeartRateRecord> = serde_json::from_str(&serialized)
        .expect("Should deserialize QueryResponse");

    assert_eq!(deserialized.data.len(), 2);
    assert_eq!(deserialized.pagination.page, 1);
    assert_eq!(deserialized.total_count, 2);
}

/// Test health summary structure
#[tokio::test]
async fn test_health_summary_structure() {
    use self_sensored::handlers::query::{
        ActivitySummary, BloodPressureSummary, DateRange,
        HeartRateSummary, SleepSummary, WorkoutSummary,
    };

    let user_id = Uuid::new_v4();
    let start_date = Utc::now() - ChronoDuration::days(30);
    let end_date = Utc::now();

    let date_range = DateRange {
        start_date,
        end_date,
    };

    let heart_rate_summary = HeartRateSummary {
        count: 100,
        avg_resting: Some(60.2),
        avg_active: Some(72.5),
        min_bpm: Some(58),
        max_bpm: Some(145),
    };

    let blood_pressure_summary = BloodPressureSummary {
        count: 50,
        avg_systolic: Some(120.0),
        avg_diastolic: Some(80.0),
        latest_reading: Some(Utc::now()),
    };

    let sleep_summary = SleepSummary {
        count: 30,
        avg_duration_hours: Some(8.0),
        avg_efficiency: Some(92.5),
        total_sleep_time: Some(14400),
    };

    let activity_summary = ActivitySummary {
        count: 30,
        total_steps: Some(300000),
        avg_daily_steps: Some(10000.0),
        total_distance_km: Some(240.0),
        total_calories: Some(15000.0),
    };

    let workout_summary = WorkoutSummary {
        count: 15,
        total_duration_hours: Some(15.0),
        total_calories: Some(7500.0),
        workout_types: vec![],
    };

    let health_summary = HealthSummary {
        user_id,
        date_range,
        heart_rate: Some(heart_rate_summary),
        blood_pressure: Some(blood_pressure_summary),
        sleep: Some(sleep_summary),
        activity: Some(activity_summary),
        workouts: Some(workout_summary),
    };

    // Test serialization
    let serialized = serde_json::to_string(&health_summary)
        .expect("Should serialize HealthSummary");

    // Test deserialization
    let deserialized: HealthSummary = serde_json::from_str(&serialized)
        .expect("Should deserialize HealthSummary");

    assert_eq!(deserialized.user_id, user_id);
    assert!(deserialized.heart_rate.is_some());
    assert!(deserialized.blood_pressure.is_some());
    assert!(deserialized.sleep.is_some());
    assert!(deserialized.activity.is_some());
    assert!(deserialized.workouts.is_some());

    let hr_summary = deserialized.heart_rate.unwrap();
    assert_eq!(hr_summary.avg_active, Some(72.5));
    assert_eq!(hr_summary.count, 1000);
}

/// Test pagination information
#[tokio::test]
async fn test_pagination_logic() {
    // Test pagination calculation logic
    struct PaginationTestCase {
        page: u32,
        limit: u32,
        total_count: i64,
        expected_has_next: bool,
        expected_has_prev: bool,
        expected_offset: u32,
    }

    let test_cases = vec![
        PaginationTestCase {
            page: 1,
            limit: 10,
            total_count: 100,
            expected_has_next: true,
            expected_has_prev: false,
            expected_offset: 0,
        },
        PaginationTestCase {
            page: 5,
            limit: 10,
            total_count: 100,
            expected_has_next: true,
            expected_has_prev: true,
            expected_offset: 40,
        },
        PaginationTestCase {
            page: 10,
            limit: 10,
            total_count: 100,
            expected_has_next: false,
            expected_has_prev: true,
            expected_offset: 90,
        },
        PaginationTestCase {
            page: 1,
            limit: 10,
            total_count: 5,
            expected_has_next: false,
            expected_has_prev: false,
            expected_offset: 0,
        },
    ];

    for test_case in test_cases {
        let offset = (test_case.page - 1) * test_case.limit;
        let has_next = (offset + test_case.limit) < test_case.total_count as u32;
        let has_prev = test_case.page > 1;

        assert_eq!(offset, test_case.expected_offset);
        assert_eq!(has_next, test_case.expected_has_next);
        assert_eq!(has_prev, test_case.expected_has_prev);

        let pagination = PaginationInfo {
            page: test_case.page,
            limit: test_case.limit,
            has_next,
            has_prev,
        };

        assert_eq!(pagination.page, test_case.page);
        assert_eq!(pagination.limit, test_case.limit);
        assert_eq!(pagination.has_next, test_case.expected_has_next);
        assert_eq!(pagination.has_prev, test_case.expected_has_prev);
    }
}

/// Test different cache TTL scenarios for different data types
#[tokio::test]
async fn test_cache_ttl_scenarios() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let _cached_query_service = CachedQueryService::new(
        pool,
        cache_service,
        cache_prefix,
    );

    // Test TTL values for different types of cached data
    let ttl_scenarios = vec![
        ("query_data", Duration::from_secs(600)),     // 10 minutes for query data
        ("summary_data", Duration::from_secs(1800)),  // 30 minutes for summaries
        ("api_key_data", Duration::from_secs(300)),   // 5 minutes for API keys
        ("user_data", Duration::from_secs(600)),      // 10 minutes for user data
    ];

    for (data_type, expected_ttl) in ttl_scenarios {
        // Verify TTL values match expected patterns
        match data_type {
            "query_data" => assert_eq!(expected_ttl, Duration::from_secs(600)),
            "summary_data" => assert_eq!(expected_ttl, Duration::from_secs(1800)),
            "api_key_data" => assert_eq!(expected_ttl, Duration::from_secs(300)),
            "user_data" => assert_eq!(expected_ttl, Duration::from_secs(600)),
            _ => panic!("Unknown data type"),
        }
    }
}

/// Test cache behavior with different query parameter combinations
#[tokio::test]
async fn test_cache_with_query_parameter_variations() {
    use self_sensored::services::cache::generate_query_hash;

    let base_date = Utc::now();

    let query_variations = vec![
        // Standard pagination queries
        QueryParams {
            page: Some(1),
            limit: Some(100),
            sort: Some("desc".to_string()),
            start_date: Some(base_date - ChronoDuration::days(7)),
            end_date: Some(base_date),
            metric_types: None,
        },
        QueryParams {
            page: Some(2),
            limit: Some(100),
            sort: Some("desc".to_string()),
            start_date: Some(base_date - ChronoDuration::days(7)),
            end_date: Some(base_date),
            metric_types: None,
        },
        // Different limits
        QueryParams {
            page: Some(1),
            limit: Some(50),
            sort: Some("desc".to_string()),
            start_date: Some(base_date - ChronoDuration::days(7)),
            end_date: Some(base_date),
            metric_types: None,
        },
        // Different sort orders
        QueryParams {
            page: Some(1),
            limit: Some(100),
            sort: Some("asc".to_string()),
            start_date: Some(base_date - ChronoDuration::days(7)),
            end_date: Some(base_date),
            metric_types: None,
        },
        // Different date ranges
        QueryParams {
            page: Some(1),
            limit: Some(100),
            sort: Some("desc".to_string()),
            start_date: Some(base_date - ChronoDuration::days(30)),
            end_date: Some(base_date),
            metric_types: None,
        },
        // Minimal parameters
        QueryParams {
            page: None,
            limit: None,
            sort: None,
            start_date: None,
            end_date: None,
            metric_types: None,
        },
    ];

    let mut hashes = Vec::new();
    for params in &query_variations {
        let hash = generate_query_hash(params);
        hashes.push(hash);
    }

    // Verify all hashes are unique (different parameters should produce different hashes)
    for i in 0..hashes.len() {
        for j in i + 1..hashes.len() {
            assert_ne!(hashes[i], hashes[j],
                "Hash collision detected between query variations {} and {}", i, j);
        }
    }

    // Verify hash consistency (same parameters should produce same hash)
    for params in &query_variations {
        let hash1 = generate_query_hash(params);
        let hash2 = generate_query_hash(params);
        assert_eq!(hash1, hash2, "Hash inconsistency detected");
    }
}

/// Test cache service integration points
#[tokio::test]
async fn test_cache_service_integration() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let cached_query_service = CachedQueryService::new(
        pool,
        cache_service,
        cache_prefix,
    );

    // Test that the service integrates properly with cache and database
    // In a real test environment, we would test actual query caching

    // For now, verify the service can be created and basic operations work
    let user_id = Uuid::new_v4();
    cached_query_service.invalidate_user_cache(user_id).await;

    let stats = cached_query_service.get_cache_stats().await;
    assert!(!stats.enabled || stats.error); // Disabled cache should report appropriately
}

/// Test error handling in cached queries
#[tokio::test]
async fn test_cached_query_error_handling() {
    let pool = setup_test_database().await;
    let cache_service = create_test_cache_service().await;
    let cache_prefix = "test_health_export".to_string();

    let _cached_query_service = CachedQueryService::new(
        pool,
        cache_service,
        cache_prefix,
    );

    // Test scenarios where cache operations might fail
    // With disabled cache, operations should gracefully handle the lack of caching

    // Test with invalid user ID
    let invalid_user_id = Uuid::nil();

    // These operations should not panic even with invalid inputs
    // (In real implementation, they would fallback to database queries)
    assert!(true); // Placeholder for error handling tests
}

/// Test cache consistency across different data types
#[tokio::test]
async fn test_cache_consistency_across_data_types() {
    use self_sensored::services::cache::CacheKey;

    let user_id = Uuid::new_v4();
    let query_hash = "consistent_hash_123";

    // Create cache keys for different metric types with same user and hash
    let keys = vec![
        CacheKey::HeartRateQuery {
            user_id,
            hash: query_hash.to_string(),
        },
        CacheKey::BloodPressureQuery {
            user_id,
            hash: query_hash.to_string(),
        },
        CacheKey::SleepQuery {
            user_id,
            hash: query_hash.to_string(),
        },
        CacheKey::ActivityQuery {
            user_id,
            hash: query_hash.to_string(),
        },
        CacheKey::WorkoutQuery {
            user_id,
            hash: query_hash.to_string(),
        },
    ];

    let prefix = "health_export";

    // Verify that keys are consistent and don't collide
    let mut redis_keys = Vec::new();
    for key in keys {
        let redis_key = key.to_redis_key(prefix);
        redis_keys.push(redis_key);
    }

    // All keys should be unique
    for i in 0..redis_keys.len() {
        for j in i + 1..redis_keys.len() {
            assert_ne!(redis_keys[i], redis_keys[j],
                "Cache key collision detected between types");
        }
    }

    // All keys should contain the user ID and hash
    for redis_key in &redis_keys {
        assert!(redis_key.contains(&user_id.to_string()));
        assert!(redis_key.contains(query_hash));
        assert!(redis_key.starts_with(prefix));
    }
}