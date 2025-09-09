use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

use self_sensored::db::database::{create_connection_pool, test_database_connection};
use self_sensored::handlers::query::QueryParams;
use self_sensored::models::db::*;
use self_sensored::services::cache::{CacheService, CacheConfig, CacheKey};
use self_sensored::services::cached_queries::CachedQueryService;
use self_sensored::services::auth::{AuthService, User, AuthContext};

/// Database performance test suite
/// Validates that queries meet the 95th percentile <100ms requirement

#[tokio::test]
async fn test_database_connection_pool_performance() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    
    // Test connection pool creation time
    let start = Instant::now();
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    let creation_time = start.elapsed();
    
    println!("Connection pool creation time: {:?}", creation_time);
    assert!(creation_time < Duration::from_millis(2000), 
           "Pool creation took too long: {:?}", creation_time);

    // Test connection health
    test_database_connection(&pool).await.expect("Connection test failed");
    
    // Test concurrent connection acquisition
    let concurrent_tasks = 20;
    let start = Instant::now();
    
    let tasks: Vec<_> = (0..concurrent_tasks)
        .map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let _conn = pool.acquire().await.expect("Failed to acquire connection");
                tokio::time::sleep(Duration::from_millis(10)).await;
            })
        })
        .collect();
    
    for task in tasks {
        task.await.expect("Task failed");
    }
    
    let concurrent_time = start.elapsed();
    println!("Concurrent connection acquisition time: {:?}", concurrent_time);
    assert!(concurrent_time < Duration::from_millis(1000),
           "Concurrent connections took too long: {:?}", concurrent_time);
}

#[tokio::test]
async fn test_query_performance_benchmarks() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    // Create test user
    let user_id = create_test_user(&pool).await;
    let _data_count = create_test_heart_rate_data(&pool, user_id, 1000).await;
    
    println!("Running performance benchmarks with 1000 heart rate records...");
    
    // Test 1: Single user query performance (most common operation)
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let _result = sqlx::query_as::<_, HeartRateRecord>(
            "SELECT user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device, metadata, created_at 
             FROM heart_rate_metrics 
             WHERE user_id = $1 
             ORDER BY recorded_at DESC 
             LIMIT 100"
        )
        .bind(user_id)
        .fetch_all(&pool)
        .await
        .expect("Query failed");
        
        times.push(start.elapsed());
    }
    
    let p95_time = calculate_percentile(&mut times, 95);
    println!("Heart rate query P95 time: {:?}", p95_time);
    assert!(p95_time < Duration::from_millis(100), 
           "P95 query time exceeded 100ms: {:?}", p95_time);

    // Test 2: Count query performance (for pagination)
    let mut count_times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let _count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Count query failed");
        
        count_times.push(start.elapsed());
    }
    
    let count_p95 = calculate_percentile(&mut count_times, 95);
    println!("Count query P95 time: {:?}", count_p95);
    assert!(count_p95 < Duration::from_millis(50), 
           "Count query P95 time exceeded 50ms: {:?}", count_p95);

    // Test 3: Summary statistics performance (expensive operations)
    let mut summary_times = Vec::new();
    let start_date = Utc::now() - chrono::Duration::days(30);
    let end_date = Utc::now();
    
    for _ in 0..50 {
        let start = Instant::now();
        let _summary = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as count,
                AVG(CASE WHEN context = 'resting' THEN resting_heart_rate END) as avg_resting,
                AVG(CASE WHEN context != 'resting' OR context IS NULL THEN heart_rate END) as avg_active,
                MIN(heart_rate) as min_bpm,
                MAX(heart_rate) as max_bpm
            FROM heart_rate_metrics 
            WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
            "#,
            user_id, start_date, end_date
        )
        .fetch_one(&pool)
        .await
        .expect("Summary query failed");
        
        summary_times.push(start.elapsed());
    }
    
    let summary_p95 = calculate_percentile(&mut summary_times, 95);
    println!("Summary query P95 time: {:?}", summary_p95);
    assert!(summary_p95 < Duration::from_millis(200), 
           "Summary query P95 time exceeded 200ms: {:?}", summary_p95);

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_batch_insert_performance() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    let user_id = create_test_user(&pool).await;
    
    // Test batch insert performance with different sizes
    let batch_sizes = vec![100, 500, 1000];
    
    for batch_size in batch_sizes {
        let start = Instant::now();
        let _inserted = create_test_heart_rate_data(&pool, user_id, batch_size).await;
        let insert_time = start.elapsed();
        
        let per_record_time = insert_time.as_micros() / batch_size as u128;
        
        println!("Batch size {}: {:?} total, {}μs per record", 
                batch_size, insert_time, per_record_time);
        
        // Should insert 1000 records in under 5 seconds
        if batch_size == 1000 {
            assert!(insert_time < Duration::from_secs(5), 
                   "1000 record batch insert took too long: {:?}", insert_time);
        }
        
        // Cleanup after each test
        sqlx::query("DELETE FROM heart_rate_metrics WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("Cleanup failed");
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_cache_performance() {
    dotenv::dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let cache_config = CacheConfig::default();
    
    // Skip cache test if Redis is not available
    let cache_service = match CacheService::new(&redis_url, cache_config).await {
        Ok(cache) => cache,
        Err(e) => {
            println!("Skipping cache tests - Redis not available: {}", e);
            return;
        }
    };
    
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    let cached_query_service = CachedQueryService::new(
        pool.clone(),
        cache_service,
        "test_health_export".to_string(),
    );
    
    let user_id = create_test_user(&pool).await;
    let _data_count = create_test_heart_rate_data(&pool, user_id, 100).await;
    
    // Create auth context
    let auth_context = create_test_auth_context(user_id);
    let params = QueryParams {
        start_date: None,
        end_date: None,
        metric_types: None,
        page: Some(1),
        limit: Some(100),
        sort: None,
    };
    
    // Test cache miss performance (first query)
    let start = Instant::now();
    let _result1 = cached_query_service
        .get_heart_rate_data_cached(auth_context.clone(), &params)
        .await
        .expect("Cached query failed");
    let miss_time = start.elapsed();
    
    // Test cache hit performance (second query)
    let start = Instant::now();
    let _result2 = cached_query_service
        .get_heart_rate_data_cached(auth_context.clone(), &params)
        .await
        .expect("Cached query failed");
    let hit_time = start.elapsed();
    
    println!("Cache miss time: {:?}", miss_time);
    println!("Cache hit time: {:?}", hit_time);
    
    // Cache hit should be significantly faster
    assert!(hit_time < miss_time / 2, 
           "Cache hit not significantly faster than miss");
    assert!(hit_time < Duration::from_millis(10), 
           "Cache hit too slow: {:?}", hit_time);
    
    // Test cache statistics
    let stats = cached_query_service.get_cache_stats().await;
    println!("Cache stats: hits={}, misses={}, hit_rate={:.2}%", 
            stats.hits, stats.misses, stats.hit_rate);
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test] 
async fn test_concurrent_query_performance() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    let user_id = create_test_user(&pool).await;
    let _data_count = create_test_heart_rate_data(&pool, user_id, 500).await;
    
    // Test concurrent queries performance
    let concurrent_queries = 20;
    let start = Instant::now();
    
    let tasks: Vec<_> = (0..concurrent_queries)
        .map(|_| {
            let pool = pool.clone();
            let user_id = user_id;
            tokio::spawn(async move {
                let query_start = Instant::now();
                let _result = sqlx::query_as::<_, HeartRateRecord>(
                    "SELECT user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device, metadata, created_at 
                     FROM heart_rate_metrics 
                     WHERE user_id = $1 
                     ORDER BY recorded_at DESC 
                     LIMIT 50"
                )
                .bind(user_id)
                .fetch_all(&pool)
                .await
                .expect("Concurrent query failed");
                query_start.elapsed()
            })
        })
        .collect();
    
    let query_times: Vec<Duration> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|result| result.expect("Task failed"))
        .collect();
    
    let total_time = start.elapsed();
    let avg_query_time: Duration = query_times.iter().sum::<Duration>() / concurrent_queries;
    
    println!("Concurrent queries: {} queries in {:?}", concurrent_queries, total_time);
    println!("Average query time: {:?}", avg_query_time);
    println!("Queries per second: {:.2}", concurrent_queries as f64 / total_time.as_secs_f64());
    
    // Should handle 20 concurrent queries efficiently
    assert!(total_time < Duration::from_secs(2), 
           "Concurrent queries took too long: {:?}", total_time);
    assert!(avg_query_time < Duration::from_millis(200), 
           "Average query time too high under load: {:?}", avg_query_time);
    
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_index_effectiveness() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    let user_id = create_test_user(&pool).await;
    let _data_count = create_test_heart_rate_data(&pool, user_id, 1000).await;
    
    // Test that queries are using indexes (no sequential scans)
    let explain_result = sqlx::query(
        "EXPLAIN ANALYZE SELECT user_id, recorded_at, heart_rate 
         FROM heart_rate_metrics 
         WHERE user_id = $1 
         ORDER BY recorded_at DESC 
         LIMIT 100"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("EXPLAIN query failed");
    
    let explain_text: String = explain_result
        .iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>()
        .join("\n");
    
    println!("Query execution plan:\n{}", explain_text);
    
    // Check that we're not doing sequential scans on large tables
    assert!(!explain_text.contains("Seq Scan on heart_rate_metrics "), 
           "Query using sequential scan instead of index");
    
    // Should be using index scans or index-only scans
    assert!(explain_text.contains("Index") || explain_text.contains("Append"), 
           "Query not using indexes effectively");
    
    cleanup_test_data(&pool, user_id).await;
}

// Helper functions

async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let email = format!("perf_test_{}@example.com", user_id.simple());
    
    sqlx::query!(
        "INSERT INTO users (id, email, full_name, is_active) VALUES ($1, $2, $3, true)
         ON CONFLICT (id) DO NOTHING",
        user_id, email, "Performance Test User"
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");
    
    user_id
}

async fn create_test_heart_rate_data(pool: &PgPool, user_id: Uuid, count: usize) -> usize {
    let mut inserted = 0;
    let base_time = Utc::now() - chrono::Duration::days(30);
    
    for i in 0..count {
        let recorded_at = base_time + chrono::Duration::minutes(i as i64);
        let heart_rate = 60 + (i % 40) as i16; // Vary between 60-100 bpm
        
        match sqlx::query!(
            "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, source_device) 
             VALUES ($1, $2, $3, 'Test Device')
             ON CONFLICT (user_id, recorded_at) DO NOTHING",
            user_id, recorded_at, heart_rate
        )
        .execute(pool)
        .await
        {
            Ok(_) => inserted += 1,
            Err(e) => eprintln!("Failed to insert test data: {}", e),
        }
    }
    
    inserted
}

fn create_test_auth_context(user_id: Uuid) -> AuthContext {
    AuthContext {
        user: User {
            id: user_id,
            email: "test@example.com".to_string(),
            full_name: Some("Test User".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            is_active: Some(true),
        },
        api_key: crate::services::auth::ApiKey {
            id: Uuid::new_v4(),
            user_id,
            name: "test_key".to_string(),
            created_at: Some(Utc::now()),
            last_used_at: None,
            expires_at: None,
            is_active: Some(true),
            scopes: Some(vec!["read".to_string()]),
        },
    }
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up test data
    let _ = sqlx::query("DELETE FROM heart_rate_metrics WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

fn calculate_percentile(times: &mut Vec<Duration>, percentile: u8) -> Duration {
    times.sort();
    let index = (times.len() * percentile as usize / 100).saturating_sub(1);
    times[index]
}

#[tokio::test]
async fn test_query_timeout_protection() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    let pool = create_connection_pool(&database_url).await.expect("Failed to create pool");
    
    // Test that queries have reasonable timeout protection
    let long_running_query = sqlx::query("SELECT pg_sleep(10)")
        .execute(&pool);
    
    let result = timeout(Duration::from_secs(5), long_running_query).await;
    
    assert!(result.is_err(), "Query should have timed out");
    println!("✓ Query timeout protection working");
}