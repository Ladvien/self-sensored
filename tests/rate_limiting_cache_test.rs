// Comprehensive Rate Limiting Cache Tests
use chrono::{Duration as ChronoDuration, Utc};
use redis::{aio::ConnectionManager, AsyncCommands, Client as RedisClient};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use uuid::Uuid;

use self_sensored::services::rate_limiter::{RateLimitError, RateLimitInfo, RateLimiter};

/// Test rate limiter initialization with Redis backend
#[tokio::test]
async fn test_rate_limiter_redis_initialization() {
    // Test with environment variables
    std::env::set_var("RATE_LIMIT_REQUESTS_PER_HOUR", "150");
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "200");

    // Try to create with Redis (will fallback to in-memory if Redis unavailable)
    let rate_limiter = RateLimiter::new("redis://127.0.0.1:6379/1").await;
    assert!(rate_limiter.is_ok());

    let limiter = rate_limiter.unwrap();

    // Clean up environment
    std::env::remove_var("RATE_LIMIT_REQUESTS_PER_HOUR");
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");

    // Test clear all functionality
    let clear_result = limiter.clear_all().await;
    assert!(clear_result.is_ok());
}

/// Test rate limiter fallback to in-memory when Redis unavailable
#[tokio::test]
async fn test_rate_limiter_fallback_behavior() {
    // Test with invalid Redis URL to force fallback
    let rate_limiter = RateLimiter::new("redis://invalid-host:6379").await;
    assert!(rate_limiter.is_ok());

    let limiter = rate_limiter.unwrap();

    // Should fallback to in-memory store
    assert!(!limiter.is_using_redis());

    let api_key_id = Uuid::new_v4();
    let result = limiter.check_rate_limit(api_key_id).await;
    assert!(result.is_ok());
}

/// Test sliding window rate limiting behavior
#[tokio::test]
async fn test_sliding_window_rate_limiting() {
    let rate_limiter = RateLimiter::new_in_memory(5); // 5 requests per hour
    let api_key_id = Uuid::new_v4();

    // Fill up the rate limit
    for i in 0..5 {
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        assert_eq!(result.requests_remaining, 4 - i);
        assert_eq!(result.requests_limit, 5);
        assert!(result.retry_after.is_none());
    }

    // Next request should be rate limited
    let limited_result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(limited_result.requests_remaining, 0);
    assert_eq!(limited_result.requests_limit, 5);
    assert!(limited_result.retry_after.is_some());
    assert!(limited_result.retry_after.unwrap() > 0);
}

/// Test rate limit status checking without incrementing
#[tokio::test]
async fn test_rate_limit_status_checking() {
    let rate_limiter = RateLimiter::new_in_memory(10);
    let api_key_id = Uuid::new_v4();

    // Make some requests
    for _ in 0..3 {
        rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    }

    // Check status multiple times - should not increment
    for _ in 0..5 {
        let status = rate_limiter
            .get_rate_limit_status(api_key_id)
            .await
            .unwrap();
        assert_eq!(status.requests_remaining, 7); // 10 - 3 = 7
        assert_eq!(status.requests_limit, 10);
        assert!(status.retry_after.is_none());
    }

    // Make one more request - should increment
    let after_request = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(after_request.requests_remaining, 6); // Now 6 remaining

    // Status should reflect the change
    let final_status = rate_limiter
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();
    assert_eq!(final_status.requests_remaining, 6);
}

/// Test concurrent rate limiting operations
#[tokio::test]
async fn test_concurrent_rate_limiting() {
    let rate_limiter = Arc::new(RateLimiter::new_in_memory(100));
    let api_key_id = Uuid::new_v4();
    let num_concurrent_requests = 50;

    let mut handles = Vec::new();

    // Launch concurrent requests
    for i in 0..num_concurrent_requests {
        let limiter = Arc::clone(&rate_limiter);
        let key_id = api_key_id;

        let handle = tokio::spawn(async move {
            let result = limiter.check_rate_limit(key_id).await;
            (i, result)
        });

        handles.push(handle);
    }

    // Collect results
    let results = futures::future::join_all(handles).await;

    let mut successful_requests = 0;
    let mut failed_requests = 0;

    for result in results {
        let (_, rate_limit_result) = result.unwrap();
        match rate_limit_result {
            Ok(info) => {
                if info.requests_remaining >= 0 {
                    successful_requests += 1;
                }
            }
            Err(_) => failed_requests += 1,
        }
    }

    // All concurrent requests should succeed since we have a limit of 100
    assert_eq!(successful_requests, num_concurrent_requests);
    assert_eq!(failed_requests, 0);

    // Final status should show 50 remaining
    let final_status = rate_limiter
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();
    assert_eq!(final_status.requests_remaining, 50);
}

/// Test rate limiting for different entity types (API keys, IPs, users)
#[tokio::test]
async fn test_different_entity_rate_limiting() {
    let rate_limiter = RateLimiter::new_in_memory_with_ip_limit(10, 5); // 10 for users, 5 for IPs

    let api_key_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let ip_address = "192.168.1.100";

    // Test API key rate limiting
    for i in 0..10 {
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        assert_eq!(result.requests_remaining, 9 - i);
        assert_eq!(result.requests_limit, 10);
    }

    // API key should be at limit
    let api_key_limited = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(api_key_limited.requests_remaining, 0);
    assert!(api_key_limited.retry_after.is_some());

    // Test user rate limiting (should be independent)
    for i in 0..10 {
        let result = rate_limiter.check_user_rate_limit(user_id).await.unwrap();
        assert_eq!(result.requests_remaining, 9 - i);
        assert_eq!(result.requests_limit, 10);
    }

    // User should be at limit
    let user_limited = rate_limiter.check_user_rate_limit(user_id).await.unwrap();
    assert_eq!(user_limited.requests_remaining, 0);
    assert!(user_limited.retry_after.is_some());

    // Test IP rate limiting (should be independent and have different limit)
    for i in 0..5 {
        let result = rate_limiter.check_ip_rate_limit(ip_address).await.unwrap();
        assert_eq!(result.requests_remaining, 4 - i);
        assert_eq!(result.requests_limit, 5);
    }

    // IP should be at limit
    let ip_limited = rate_limiter.check_ip_rate_limit(ip_address).await.unwrap();
    assert_eq!(ip_limited.requests_remaining, 0);
    assert!(ip_limited.retry_after.is_some());
}

/// Test rate limit key isolation
#[tokio::test]
async fn test_rate_limit_key_isolation() {
    let rate_limiter = RateLimiter::new_in_memory(3);

    let api_key_1 = Uuid::new_v4();
    let api_key_2 = Uuid::new_v4();
    let user_1 = Uuid::new_v4();
    let user_2 = Uuid::new_v4();
    let ip_1 = "192.168.1.1";
    let ip_2 = "192.168.1.2";

    // Use up limits for first set of keys
    for _ in 0..3 {
        rate_limiter.check_rate_limit(api_key_1).await.unwrap();
        rate_limiter.check_user_rate_limit(user_1).await.unwrap();
        rate_limiter.check_ip_rate_limit(ip_1).await.unwrap();
    }

    // First set should be limited
    let api_key_1_limited = rate_limiter.check_rate_limit(api_key_1).await.unwrap();
    assert_eq!(api_key_1_limited.requests_remaining, 0);

    let user_1_limited = rate_limiter.check_user_rate_limit(user_1).await.unwrap();
    assert_eq!(user_1_limited.requests_remaining, 0);

    let ip_1_limited = rate_limiter.check_ip_rate_limit(ip_1).await.unwrap();
    assert_eq!(ip_1_limited.requests_remaining, 0);

    // Second set should still have full limits
    let api_key_2_status = rate_limiter.check_rate_limit(api_key_2).await.unwrap();
    assert_eq!(api_key_2_status.requests_remaining, 2); // 3 - 1 = 2

    let user_2_status = rate_limiter.check_user_rate_limit(user_2).await.unwrap();
    assert_eq!(user_2_status.requests_remaining, 2);

    let ip_2_status = rate_limiter.check_ip_rate_limit(ip_2).await.unwrap();
    assert_eq!(ip_2_status.requests_remaining, 2);
}

/// Test rate limit reset time calculation
#[tokio::test]
async fn test_rate_limit_reset_time() {
    let rate_limiter = RateLimiter::new_in_memory(1);
    let api_key_id = Uuid::new_v4();

    let start_time = Utc::now();

    // Use up the limit
    rate_limiter.check_rate_limit(api_key_id).await.unwrap();

    // Next request should be limited with reset time
    let limited_result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(limited_result.requests_remaining, 0);
    assert!(limited_result.retry_after.is_some());

    let reset_time = limited_result.reset_time;
    let retry_after_seconds = limited_result.retry_after.unwrap();

    // Reset time should be in the future
    assert!(reset_time > start_time);

    // Retry after should be positive and reasonable (less than 1 hour)
    assert!(retry_after_seconds > 0);
    assert!(retry_after_seconds <= 3600);

    // Reset time should be approximately retry_after seconds from now
    let expected_reset = Utc::now() + ChronoDuration::seconds(retry_after_seconds as i64);
    let time_diff = (reset_time - expected_reset).num_seconds().abs();
    assert!(
        time_diff <= 2,
        "Reset time calculation should be accurate within 2 seconds"
    );
}

/// Test rate limiter memory cleanup
#[tokio::test]
async fn test_rate_limiter_memory_cleanup() {
    let rate_limiter = RateLimiter::new_in_memory(100);

    // Create many different keys to test memory cleanup
    let mut api_keys = Vec::new();
    for _ in 0..50 {
        api_keys.push(Uuid::new_v4());
    }

    // Make requests with all keys
    for api_key in &api_keys {
        rate_limiter.check_rate_limit(*api_key).await.unwrap();
    }

    // All keys should have entries
    for api_key in &api_keys {
        let status = rate_limiter.get_rate_limit_status(*api_key).await.unwrap();
        assert_eq!(status.requests_remaining, 99);
    }

    // Clear all entries
    rate_limiter.clear_all().await.unwrap();

    // After clearing, all keys should have fresh limits
    for api_key in &api_keys {
        let status = rate_limiter.get_rate_limit_status(*api_key).await.unwrap();
        assert_eq!(status.requests_remaining, 100);
    }
}

/// Test rate limiting with Redis-specific features (if Redis is available)
#[tokio::test]
async fn test_redis_specific_features() {
    // This test will try to use Redis features if available, otherwise skip
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/1".to_string());

    let rate_limiter_result = RateLimiter::new(&redis_url).await;

    if let Ok(rate_limiter) = rate_limiter_result {
        if rate_limiter.is_using_redis() {
            let api_key_id = Uuid::new_v4();

            // Test Redis-specific sliding window behavior
            for i in 0..10 {
                let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
                // With Redis, we should see consistent behavior
                assert!(result.requests_remaining >= 0);
            }

            // Test that data persists across different rate limiter instances
            let rate_limiter_2 = RateLimiter::new(&redis_url).await.unwrap();

            // Should see the same rate limit state
            let status = rate_limiter_2
                .get_rate_limit_status(api_key_id)
                .await
                .unwrap();
            assert!(status.requests_remaining < 100); // Should reflect previous usage

            // Clean up
            rate_limiter.clear_all().await.unwrap();
        }
    }
    // If Redis is not available, test passes (allows CI/CD without Redis)
}

/// Test rate limiting under high concurrency with Redis
#[tokio::test]
async fn test_high_concurrency_rate_limiting() {
    let rate_limiter = Arc::new(RateLimiter::new_in_memory(1000));
    let api_key_id = Uuid::new_v4();
    let num_concurrent_tasks = 100;
    let requests_per_task = 5;

    let mut handles = Vec::new();

    for task_id in 0..num_concurrent_tasks {
        let limiter = Arc::clone(&rate_limiter);
        let key_id = api_key_id;

        let handle = tokio::spawn(async move {
            let mut task_results = Vec::new();

            for request_id in 0..requests_per_task {
                let start = Instant::now();
                let result = limiter.check_rate_limit(key_id).await;
                let duration = start.elapsed();

                task_results.push((task_id, request_id, result, duration));
            }

            task_results
        });

        handles.push(handle);
    }

    // Collect all results
    let all_results = futures::future::join_all(handles).await;

    let mut total_successful = 0;
    let mut total_failed = 0;
    let mut max_duration = Duration::from_millis(0);

    for task_results in all_results {
        let results = task_results.unwrap();
        for (_, _, result, duration) in results {
            max_duration = max_duration.max(duration);

            match result {
                Ok(_) => total_successful += 1,
                Err(_) => total_failed += 1,
            }
        }
    }

    // All requests should succeed since limit is 1000
    let total_requests = num_concurrent_tasks * requests_per_task;
    assert_eq!(total_successful, total_requests);
    assert_eq!(total_failed, 0);

    // Performance check - operations should be fast
    assert!(
        max_duration < Duration::from_millis(100),
        "Rate limiting operations should be fast even under high concurrency"
    );

    // Final status should show correct remaining count
    let final_status = rate_limiter
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();
    assert_eq!(
        final_status.requests_remaining,
        1000 - total_requests as i32
    );
}

/// Test cache stampede prevention in rate limiting
#[tokio::test]
async fn test_rate_limit_cache_stampede_prevention() {
    let rate_limiter = Arc::new(RateLimiter::new_in_memory(50));
    let api_key_id = Uuid::new_v4();

    // Simulate cache stampede scenario - many concurrent requests for same key
    let num_concurrent_requests = 100;
    let mut handles = Vec::new();

    let start_time = Instant::now();

    for i in 0..num_concurrent_requests {
        let limiter = Arc::clone(&rate_limiter);
        let key_id = api_key_id;

        let handle = tokio::spawn(async move {
            let request_start = Instant::now();
            let result = limiter.check_rate_limit(key_id).await;
            let request_duration = request_start.elapsed();
            (i, result, request_duration)
        });

        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let total_time = start_time.elapsed();

    let mut successful_count = 0;
    let mut rate_limited_count = 0;
    let mut max_request_time = Duration::from_millis(0);

    for result in results {
        let (_, rate_result, duration) = result.unwrap();
        max_request_time = max_request_time.max(duration);

        match rate_result {
            Ok(info) => {
                if info.requests_remaining >= 0 {
                    successful_count += 1;
                } else {
                    rate_limited_count += 1;
                }
            }
            Err(_) => rate_limited_count += 1,
        }
    }

    // Should have exactly 50 successful requests and 50 rate limited
    assert_eq!(successful_count, 50);
    assert_eq!(rate_limited_count, 50);

    // Operations should complete reasonably quickly even under stampede
    assert!(
        total_time < Duration::from_secs(5),
        "Cache stampede should not cause excessive delays"
    );
    assert!(
        max_request_time < Duration::from_millis(100),
        "Individual requests should remain fast during stampede"
    );
}

/// Test rate limit information accuracy
#[tokio::test]
async fn test_rate_limit_info_accuracy() {
    let rate_limiter = RateLimiter::new_in_memory(10);
    let api_key_id = Uuid::new_v4();

    // Test incremental consumption
    for expected_remaining in (0..10).rev() {
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();

        assert_eq!(result.requests_remaining, expected_remaining);
        assert_eq!(result.requests_limit, 10);
        assert!(result.retry_after.is_none());
        assert!(result.reset_time > Utc::now());
    }

    // Next request should be rate limited
    let limited_result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(limited_result.requests_remaining, 0);
    assert_eq!(limited_result.requests_limit, 10);
    assert!(limited_result.retry_after.is_some());
    assert!(limited_result.retry_after.unwrap() > 0);

    // Status check should match the limited state
    let status = rate_limiter
        .get_rate_limit_status(api_key_id)
        .await
        .unwrap();
    assert_eq!(status.requests_remaining, 0);
    assert_eq!(status.requests_limit, 10);
    assert!(status.retry_after.is_some());
}

/// Test environment variable configuration
#[tokio::test]
async fn test_environment_variable_configuration() {
    // Test custom rate limits via environment variables
    std::env::set_var("RATE_LIMIT_REQUESTS_PER_HOUR", "25");
    std::env::set_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR", "15");

    let rate_limiter = RateLimiter::new("redis://invalid-host:6379").await.unwrap();

    let api_key_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let ip_address = "10.0.0.1";

    // Test API key limit (should use RATE_LIMIT_REQUESTS_PER_HOUR)
    let api_status = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
    assert_eq!(api_status.requests_limit, 25);
    assert_eq!(api_status.requests_remaining, 24);

    // Test user limit (should use RATE_LIMIT_REQUESTS_PER_HOUR)
    let user_status = rate_limiter.check_user_rate_limit(user_id).await.unwrap();
    assert_eq!(user_status.requests_limit, 25);
    assert_eq!(user_status.requests_remaining, 24);

    // Test IP limit (should use RATE_LIMIT_IP_REQUESTS_PER_HOUR)
    let ip_status = rate_limiter.check_ip_rate_limit(ip_address).await.unwrap();
    assert_eq!(ip_status.requests_limit, 15);
    assert_eq!(ip_status.requests_remaining, 14);

    // Clean up
    std::env::remove_var("RATE_LIMIT_REQUESTS_PER_HOUR");
    std::env::remove_var("RATE_LIMIT_IP_REQUESTS_PER_HOUR");
}
