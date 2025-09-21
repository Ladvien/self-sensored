use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;
use futures::future::join_all;

/// API Throughput Benchmarks
/// Tests API response times against <100ms p95 latency target

struct ApiTestServer {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiTestServer {
    fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:8080".to_string(),
            api_key: "test-api-key".to_string(),
        }
    }

    async fn health_check(&self) -> Result<Response, reqwest::Error> {
        self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await
    }

    async fn ready_check(&self) -> Result<Response, reqwest::Error> {
        self.client
            .get(&format!("{}/ready", self.base_url))
            .send()
            .await
    }

    async fn ingest_data(&self, payload: &Value) -> Result<Response, reqwest::Error> {
        self.client
            .post(&format!("{}/api/v1/ingest", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
    }

    async fn query_heart_rate(&self, limit: usize) -> Result<Response, reqwest::Error> {
        self.client
            .get(&format!("{}/api/v1/data/heart-rate?limit={}", self.base_url, limit))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
    }

    async fn export_data(&self, metric_type: &str, limit: usize) -> Result<Response, reqwest::Error> {
        self.client
            .get(&format!("{}/api/v1/export/{}?limit={}", self.base_url, metric_type, limit))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept-Encoding", "gzip")
            .send()
            .await
    }
}

fn create_heart_rate_payload(count: usize) -> Value {
    let base_time = chrono::Utc::now() - chrono::Duration::hours(count as i64);
    let mut metrics = Vec::new();

    for i in 0..count {
        let recorded_at = base_time + chrono::Duration::minutes(i as i64);
        metrics.push(json!({
            "type": "heart_rate",
            "recorded_at": recorded_at.to_rfc3339(),
            "heart_rate": 70 + (i % 30) as i32,
            "resting_heart_rate": 55 + (i % 15) as i32,
            "context": if i % 3 == 0 { "resting" } else { "active" },
            "source_device": "Benchmark Device"
        }));
    }

    json!({
        "data": {
            "metrics": metrics
        }
    })
}

fn create_mixed_metrics_payload(count: usize) -> Value {
    let base_time = chrono::Utc::now() - chrono::Duration::hours(count as i64);
    let mut metrics = Vec::new();

    for i in 0..count {
        let recorded_at = base_time + chrono::Duration::minutes(i as i64 * 10);

        // Rotate through different metric types
        match i % 5 {
            0 => metrics.push(json!({
                "type": "heart_rate",
                "recorded_at": recorded_at.to_rfc3339(),
                "heart_rate": 70 + (i % 30) as i32,
                "source_device": "Apple Watch"
            })),
            1 => metrics.push(json!({
                "type": "blood_pressure",
                "recorded_at": recorded_at.to_rfc3339(),
                "systolic": 120 + (i % 40) as i32,
                "diastolic": 80 + (i % 20) as i32,
                "source_device": "Blood Pressure Monitor"
            })),
            2 => metrics.push(json!({
                "type": "activity",
                "recorded_at": recorded_at.to_rfc3339(),
                "step_count": 8000 + (i % 5000) as i32,
                "distance_meters": 5000 + (i % 3000) as i32,
                "source_device": "iPhone"
            })),
            3 => metrics.push(json!({
                "type": "sleep",
                "sleep_start": recorded_at.to_rfc3339(),
                "sleep_end": (recorded_at + chrono::Duration::hours(8)).to_rfc3339(),
                "duration_minutes": 480 + (i % 120) as i32,
                "deep_sleep_minutes": 90 + (i % 60) as i32,
                "source_device": "Sleep Tracker"
            })),
            4 => metrics.push(json!({
                "type": "workout",
                "workout_type": "running",
                "started_at": recorded_at.to_rfc3339(),
                "ended_at": (recorded_at + chrono::Duration::minutes(30)).to_rfc3339(),
                "total_energy_kcal": 300 + (i % 200) as i32,
                "source_device": "Fitness Tracker"
            })),
            _ => unreachable!(),
        }
    }

    json!({
        "data": {
            "metrics": metrics
        }
    })
}

fn bench_health_endpoint_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("health_endpoint");
    group.sample_size(200);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("health_check_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let response = server.health_check().await;
            black_box(response)
        })
    });

    group.bench_function("ready_check_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let response = server.ready_check().await;
            black_box(response)
        })
    });

    group.finish();
}

fn bench_ingest_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("ingest_throughput");
    group.sample_size(100);

    // Test different payload sizes
    for metrics_count in [1, 10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*metrics_count as u64));

        group.bench_with_input(
            BenchmarkId::new("heart_rate_metrics", metrics_count),
            metrics_count,
            |b, &metrics_count| {
                let payload = create_heart_rate_payload(metrics_count);
                b.to_async(&rt).iter(|| async {
                    let response = server.ingest_data(&payload).await;
                    black_box(response)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("mixed_metrics", metrics_count),
            metrics_count,
            |b, &metrics_count| {
                let payload = create_mixed_metrics_payload(metrics_count);
                b.to_async(&rt).iter(|| async {
                    let response = server.ingest_data(&payload).await;
                    black_box(response)
                })
            },
        );
    }

    group.finish();
}

fn bench_query_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("query_performance");
    group.sample_size(100);

    // Test different query result sizes
    for limit in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*limit as u64));

        group.bench_with_input(
            BenchmarkId::new("heart_rate_query", limit),
            limit,
            |b, &limit| {
                b.to_async(&rt).iter(|| async {
                    let response = server.query_heart_rate(limit).await;
                    black_box(response)
                })
            },
        );
    }

    group.finish();
}

fn bench_export_compression(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("export_compression");
    group.sample_size(50);

    // Test export with different data sizes
    for limit in [100, 500, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*limit as u64));

        group.bench_with_input(
            BenchmarkId::new("compressed_export", limit),
            limit,
            |b, &limit| {
                b.to_async(&rt).iter(|| async {
                    let response = server.export_data("heart-rate", limit).await;
                    black_box(response)
                })
            },
        );
    }

    group.finish();
}

fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_requests");
    group.sample_size(50);

    // Test different concurrency levels
    for concurrency in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_health_checks", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let servers: Vec<_> = (0..concurrency).map(|_| ApiTestServer::new()).collect();

                    let tasks: Vec<_> = servers
                        .iter()
                        .map(|server| async move {
                            server.health_check().await
                        })
                        .collect();

                    let results = join_all(tasks).await;
                    black_box(results)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("concurrent_ingests", concurrency),
            concurrency,
            |b, &concurrency| {
                let payload = create_heart_rate_payload(10);
                b.to_async(&rt).iter(|| async {
                    let servers: Vec<_> = (0..concurrency).map(|_| ApiTestServer::new()).collect();

                    let tasks: Vec<_> = servers
                        .iter()
                        .map(|server| async move {
                            server.ingest_data(&payload).await
                        })
                        .collect();

                    let results = join_all(tasks).await;
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

fn bench_response_time_percentiles(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("response_time_percentiles");
    group.sample_size(500); // Large sample for accurate percentiles
    group.measurement_time(Duration::from_secs(60));

    // Measure response time distribution for critical endpoints
    group.bench_function("health_check_distribution", |b| {
        b.to_async(&rt).iter(|| async {
            let response = server.health_check().await;
            black_box(response)
        })
    });

    group.bench_function("small_ingest_distribution", |b| {
        let payload = create_heart_rate_payload(10);
        b.to_async(&rt).iter(|| async {
            let response = server.ingest_data(&payload).await;
            black_box(response)
        })
    });

    group.bench_function("medium_ingest_distribution", |b| {
        let payload = create_mixed_metrics_payload(100);
        b.to_async(&rt).iter(|| async {
            let response = server.ingest_data(&payload).await;
            black_box(response)
        })
    });

    group.bench_function("query_distribution", |b| {
        b.to_async(&rt).iter(|| async {
            let response = server.query_heart_rate(100).await;
            black_box(response)
        })
    });

    group.finish();
}

fn bench_memory_usage_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(20);

    // Test memory usage with large payloads
    for payload_size in [1000, 5000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("large_payload_processing", payload_size),
            payload_size,
            |b, &payload_size| {
                let payload = create_mixed_metrics_payload(payload_size);
                b.to_async(&rt).iter(|| async {
                    // Memory usage is implicitly tested through processing time
                    let response = server.ingest_data(&payload).await;
                    black_box(response)
                })
            },
        );
    }

    group.finish();
}

fn bench_sustained_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let server = ApiTestServer::new();

    let mut group = c.benchmark_group("sustained_load");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(120)); // 2 minute sustained test

    // Simulate sustained production load
    group.bench_function("production_load_simulation", |b| {
        let payloads = vec![
            create_heart_rate_payload(20),
            create_mixed_metrics_payload(50),
            create_heart_rate_payload(10),
        ];

        b.to_async(&rt).iter(|| async {
            // Simulate burst of requests
            let mut tasks = Vec::new();

            for payload in &payloads {
                tasks.push(async move {
                    server.ingest_data(payload).await
                });

                tasks.push(async move {
                    server.health_check().await
                });

                tasks.push(async move {
                    server.query_heart_rate(50).await
                });
            }

            let results = join_all(tasks).await;
            black_box(results)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_health_endpoint_latency,
    bench_ingest_throughput,
    bench_query_performance,
    bench_export_compression,
    bench_concurrent_requests,
    bench_response_time_percentiles,
    bench_memory_usage_patterns,
    bench_sustained_load
);
criterion_main!(benches);