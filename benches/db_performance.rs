use chrono::{DateTime, Utc};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

use self_sensored::db::database::create_connection_pool;
use self_sensored::models::db::*;

/// Database Performance Benchmarks
/// Tests query performance against 95th percentile <100ms target

async fn setup_benchmark_data(pool: &PgPool) -> (Uuid, Vec<Uuid>) {
    let user_id = Uuid::new_v4();
    let email = format!("benchmark_{}@example.com", user_id.simple());

    // Create benchmark user
    sqlx::query!(
        "INSERT INTO users (id, email, full_name, is_active) VALUES ($1, $2, $3, true)
         ON CONFLICT (id) DO NOTHING",
        user_id,
        email,
        "Benchmark User"
    )
    .execute(pool)
    .await
    .expect("Failed to create benchmark user");

    // Create 10,000 heart rate records
    let mut record_ids = Vec::new();
    let base_time = Utc::now() - chrono::Duration::days(365);

    for i in 0..10_000 {
        let recorded_at = base_time + chrono::Duration::minutes(i as i64 * 5);
        let heart_rate = 60 + (i % 40) as i16;
        let resting_hr = 50 + (i % 20) as i16;

        match sqlx::query!(
            r#"
            INSERT INTO heart_rate_metrics
            (user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device)
            VALUES ($1, $2, $3, $4, 'resting', 'Benchmark Device')
            ON CONFLICT (user_id, recorded_at) DO NOTHING
            "#,
            user_id,
            recorded_at,
            heart_rate,
            Some(resting_hr)
        )
        .execute(pool)
        .await
        {
            Ok(_) => record_ids.push(user_id),
            Err(e) => eprintln!("Failed to insert benchmark data: {}", e),
        }
    }

    // Create large activity dataset
    for i in 0..5_000 {
        let recorded_at = base_time + chrono::Duration::hours(i as i64);
        let steps = 8000 + (i % 5000) as i32;

        let _ = sqlx::query!(
            r#"
            INSERT INTO activity_metrics
            (user_id, recorded_at, step_count, distance_meters, source_device)
            VALUES ($1, $2, $3, $4, 'Benchmark Device')
            ON CONFLICT (user_id, recorded_at) DO NOTHING
            "#,
            user_id,
            recorded_at,
            steps,
            Some((steps as f64 * 0.7) as i32)
        )
        .execute(pool)
        .await;
    }

    (user_id, record_ids)
}

async fn cleanup_benchmark_data(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM heart_rate_metrics WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM activity_metrics WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

fn bench_heart_rate_queries(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(async {
        dotenv::dotenv().ok();
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set for benchmarks");
        create_connection_pool(&database_url)
            .await
            .expect("Failed to create pool")
    });

    let (user_id, _) = rt.block_on(setup_benchmark_data(&pool));

    let mut group = c.benchmark_group("heart_rate_queries");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(30));

    // Benchmark: Recent data query (most common operation)
    group.bench_function("recent_100_records", |b| {
        b.to_async(&rt).iter(|| async {
            let result = sqlx::query_as::<_, HeartRateRecord>(
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

            black_box(result)
        })
    });

    // Benchmark: Pagination count query
    group.bench_function("count_query", |b| {
        b.to_async(&rt).iter(|| async {
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_one(&pool)
                    .await
                    .expect("Count query failed");

            black_box(count)
        })
    });

    // Benchmark: Date range query with aggregation
    group.bench_function("monthly_summary", |b| {
        b.to_async(&rt).iter(|| async {
            let start_date = Utc::now() - chrono::Duration::days(30);
            let end_date = Utc::now();

            let summary = sqlx::query!(
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

            black_box(summary)
        })
    });

    group.finish();
    rt.block_on(cleanup_benchmark_data(&pool, user_id));
}

fn bench_connection_pool_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("connection_pool");
    group.sample_size(50);

    // Benchmark pool creation time
    group.bench_function("pool_creation", |b| {
        b.to_async(&rt).iter(|| async {
            dotenv::dotenv().ok();
            let database_url =
                std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

            let pool = create_connection_pool(&database_url)
                .await
                .expect("Failed to create pool");

            black_box(pool)
        })
    });

    // Benchmark concurrent connection acquisition
    for concurrency in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_connections", concurrency),
            concurrency,
            |b, &concurrency| {
                let pool = rt.block_on(async {
                    dotenv::dotenv().ok();
                    let database_url =
                        std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
                    create_connection_pool(&database_url)
                        .await
                        .expect("Failed to create pool")
                });

                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..concurrency)
                        .map(|_| {
                            let pool = pool.clone();
                            tokio::spawn(async move {
                                let _conn =
                                    pool.acquire().await.expect("Failed to acquire connection");
                                tokio::time::sleep(Duration::from_millis(1)).await;
                            })
                        })
                        .collect();

                    for task in tasks {
                        task.await.expect("Task failed");
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_index_effectiveness(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(async {
        dotenv::dotenv().ok();
        let database_url =
            std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
        create_connection_pool(&database_url)
            .await
            .expect("Failed to create pool")
    });

    let (user_id, _) = rt.block_on(setup_benchmark_data(&pool));

    let mut group = c.benchmark_group("index_effectiveness");
    group.sample_size(50);

    // Test query with user_id index
    group.bench_function("user_id_index_scan", |b| {
        b.to_async(&rt).iter(|| async {
            let result = sqlx::query("SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .expect("Query failed");

            black_box(result)
        })
    });

    // Test query with recorded_at BRIN index
    group.bench_function("time_range_brin_scan", |b| {
        b.to_async(&rt).iter(|| async {
            let start_date = Utc::now() - chrono::Duration::days(7);
            let end_date = Utc::now();

            let result = sqlx::query(
                "SELECT COUNT(*) FROM heart_rate_metrics
                 WHERE recorded_at BETWEEN $1 AND $2",
            )
            .bind(start_date)
            .bind(end_date)
            .fetch_one(&pool)
            .await
            .expect("Query failed");

            black_box(result)
        })
    });

    // Test composite index performance
    group.bench_function("composite_index_scan", |b| {
        b.to_async(&rt).iter(|| async {
            let start_date = Utc::now() - chrono::Duration::days(7);
            let end_date = Utc::now();

            let result = sqlx::query(
                "SELECT * FROM heart_rate_metrics
                 WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
                 ORDER BY recorded_at DESC
                 LIMIT 50",
            )
            .bind(user_id)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&pool)
            .await
            .expect("Query failed");

            black_box(result)
        })
    });

    group.finish();
    rt.block_on(cleanup_benchmark_data(&pool, user_id));
}

fn bench_batch_insert_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(async {
        dotenv::dotenv().ok();
        let database_url =
            std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
        create_connection_pool(&database_url)
            .await
            .expect("Failed to create pool")
    });

    let mut group = c.benchmark_group("batch_inserts");
    group.sample_size(20);

    for batch_size in [100, 500, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("heart_rate_batch", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let user_id = Uuid::new_v4();
                    let email = format!("batch_{}@example.com", user_id.simple());

                    // Create user
                    sqlx::query!(
                        "INSERT INTO users (id, email, full_name, is_active) VALUES ($1, $2, $3, true)",
                        user_id, email, "Batch User"
                    )
                    .execute(&pool)
                    .await
                    .expect("Failed to create user");

                    let base_time = Utc::now() - chrono::Duration::minutes(batch_size as i64);

                    // Batch insert heart rate data
                    for i in 0..batch_size {
                        let recorded_at = base_time + chrono::Duration::minutes(i as i64);
                        let heart_rate = 60 + (i % 40) as i16;

                        sqlx::query!(
                            "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, source_device)
                             VALUES ($1, $2, $3, 'Batch Device')
                             ON CONFLICT (user_id, recorded_at) DO NOTHING",
                            user_id, recorded_at, heart_rate
                        )
                        .execute(&pool)
                        .await
                        .expect("Failed to insert");
                    }

                    // Cleanup
                    let _ = sqlx::query("DELETE FROM heart_rate_metrics WHERE user_id = $1")
                        .bind(user_id)
                        .execute(&pool)
                        .await;
                    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
                        .bind(user_id)
                        .execute(&pool)
                        .await;

                    black_box(batch_size)
                })
            },
        );
    }

    group.finish();
}

fn bench_query_complexity(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(async {
        dotenv::dotenv().ok();
        let database_url =
            std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
        create_connection_pool(&database_url)
            .await
            .expect("Failed to create pool")
    });

    let (user_id, _) = rt.block_on(setup_benchmark_data(&pool));

    let mut group = c.benchmark_group("query_complexity");
    group.sample_size(50);

    // Simple query
    group.bench_function("simple_select", |b| {
        b.to_async(&rt).iter(|| async {
            let result = sqlx::query(
                "SELECT heart_rate FROM heart_rate_metrics WHERE user_id = $1 LIMIT 100",
            )
            .bind(user_id)
            .fetch_all(&pool)
            .await
            .expect("Query failed");

            black_box(result)
        })
    });

    // Complex aggregation query
    group.bench_function("complex_aggregation", |b| {
        b.to_async(&rt).iter(|| async {
            let result = sqlx::query!(
                r#"
                SELECT
                    DATE_TRUNC('day', recorded_at) as day,
                    COUNT(*) as readings,
                    AVG(heart_rate) as avg_hr,
                    MIN(heart_rate) as min_hr,
                    MAX(heart_rate) as max_hr,
                    STDDEV(heart_rate) as stddev_hr,
                    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY heart_rate) as median_hr,
                    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY heart_rate) as p95_hr
                FROM heart_rate_metrics
                WHERE user_id = $1
                  AND recorded_at >= NOW() - INTERVAL '30 days'
                GROUP BY DATE_TRUNC('day', recorded_at)
                ORDER BY day DESC
                "#,
                user_id
            )
            .fetch_all(&pool)
            .await
            .expect("Complex query failed");

            black_box(result)
        })
    });

    // Multi-table join query
    group.bench_function("multi_table_join", |b| {
        b.to_async(&rt).iter(|| async {
            let result = sqlx::query!(
                r#"
                SELECT
                    u.id,
                    u.email,
                    COUNT(hr.id) as heart_rate_count,
                    COUNT(am.id) as activity_count,
                    AVG(hr.heart_rate) as avg_heart_rate,
                    SUM(am.step_count) as total_steps
                FROM users u
                LEFT JOIN heart_rate_metrics hr ON u.id = hr.user_id
                  AND hr.recorded_at >= NOW() - INTERVAL '7 days'
                LEFT JOIN activity_metrics am ON u.id = am.user_id
                  AND am.recorded_at >= NOW() - INTERVAL '7 days'
                WHERE u.id = $1
                GROUP BY u.id, u.email
                "#,
                user_id
            )
            .fetch_one(&pool)
            .await
            .expect("Join query failed");

            black_box(result)
        })
    });

    group.finish();
    rt.block_on(cleanup_benchmark_data(&pool, user_id));
}

criterion_group!(
    benches,
    bench_heart_rate_queries,
    bench_connection_pool_performance,
    bench_index_effectiveness,
    bench_batch_insert_performance,
    bench_query_complexity
);
criterion_main!(benches);
