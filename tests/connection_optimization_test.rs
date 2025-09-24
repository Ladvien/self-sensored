use self_sensored::{
    config::BatchConfig, db::database::create_connection_pool,
    services::batch_processor::BatchProcessor,
};
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test]
async fn test_optimized_connection_pool_configuration() {
    // Load .env file
    dotenv::dotenv().ok();

    // Test that the optimized pool configuration is correctly applied
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://test:test@localhost:5433/health_export_test".to_string()
        })
    });

    let pool = create_connection_pool(&database_url)
        .await
        .expect("Failed to create connection pool");

    // Verify pool is configured with optimized settings
    assert!(
        pool.size() >= 15,
        "Pool should have at least 15 connections for warm pool"
    );

    // Test connection acquisition performance
    let start = Instant::now();
    let mut conn = pool
        .acquire()
        .await
        .expect("Should acquire connection quickly");
    let acquisition_time = start.elapsed();

    // With optimized pool, acquisition should be fast (under 100ms for warm pool)
    assert!(
        acquisition_time < Duration::from_millis(100),
        "Connection acquisition took too long: {:?}",
        acquisition_time
    );

    // Test basic query works
    sqlx::query("SELECT 1")
        .fetch_one(&mut *conn)
        .await
        .expect("Basic query should work");
}

#[tokio::test]
async fn test_batch_processor_semaphore_optimization() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost:5433/health_export_test".to_string());

    let pool = create_connection_pool(&database_url)
        .await
        .expect("Failed to create connection pool");

    // Create batch processor with optimized configuration
    let config = BatchConfig::default();
    let processor = BatchProcessor::with_config(pool.clone(), config);

    // Test connection acquisition diagnostics
    let diag_result = processor.diagnose_connection_acquisition().await;

    match diag_result {
        Ok((acquisition_ms, query_ms)) => {
            println!("Connection diagnostics successful:");
            println!("  Acquisition time: {}ms", acquisition_ms);
            println!("  Query time: {}ms", query_ms);

            // With optimized pool, these should be reasonable
            assert!(
                acquisition_ms < 1000,
                "Connection acquisition should be under 1s"
            );
            assert!(query_ms < 100, "Query should be under 100ms");
        }
        Err(e) => {
            panic!("Connection diagnostics failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_bounded_concurrency_configuration() {
    let config = BatchConfig::default();

    // Verify bounded concurrency is properly configured
    assert!(
        config.max_concurrent_metric_types > 0,
        "Should have bounded concurrency configured"
    );

    assert!(
        config.max_concurrent_metric_types <= 16,
        "Concurrency limit should be reasonable to prevent connection exhaustion"
    );

    println!(
        "Bounded concurrency configured: {} metric types",
        config.max_concurrent_metric_types
    );
}

#[tokio::test]
async fn test_concurrent_connection_acquisition() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost:5433/health_export_test".to_string());

    let pool = create_connection_pool(&database_url)
        .await
        .expect("Failed to create connection pool");

    // Test concurrent connection acquisition (simulating batch processing load)
    let concurrent_tasks = 10;
    let mut tasks = Vec::new();

    let start = Instant::now();

    for i in 0..concurrent_tasks {
        let pool_clone = pool.clone();
        tasks.push(tokio::spawn(async move {
            let task_start = Instant::now();
            match pool_clone.acquire().await {
                Ok(mut conn) => {
                    let acquisition_time = task_start.elapsed();

                    // Test query
                    match sqlx::query("SELECT $1::int as task_id")
                        .bind(i as i32)
                        .fetch_one(&mut *conn)
                        .await
                    {
                        Ok(_) => Ok(acquisition_time),
                        Err(e) => Err(format!("Query failed for task {}: {}", i, e)),
                    }
                }
                Err(e) => Err(format!(
                    "Connection acquisition failed for task {}: {}",
                    i, e
                )),
            }
        }));
    }

    // Wait for all tasks to complete
    let results = futures::future::join_all(tasks).await;
    let total_time = start.elapsed();

    let mut successful_acquisitions = 0;
    let mut total_acquisition_time = Duration::ZERO;
    let mut max_acquisition_time = Duration::ZERO;

    for (task_idx, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(acquisition_time)) => {
                successful_acquisitions += 1;
                total_acquisition_time += acquisition_time;
                max_acquisition_time = max_acquisition_time.max(acquisition_time);
                println!(
                    "Task {}: Acquisition time: {:?}",
                    task_idx, acquisition_time
                );
            }
            Ok(Err(e)) => {
                eprintln!("Task {}: {}", task_idx, e);
            }
            Err(e) => {
                eprintln!("Task {} panicked: {}", task_idx, e);
            }
        }
    }

    println!("Concurrent acquisition test results:");
    println!("  Total time: {:?}", total_time);
    println!(
        "  Successful acquisitions: {}/{}",
        successful_acquisitions, concurrent_tasks
    );
    println!(
        "  Average acquisition time: {:?}",
        if successful_acquisitions > 0 {
            total_acquisition_time / successful_acquisitions
        } else {
            Duration::ZERO
        }
    );
    println!("  Max acquisition time: {:?}", max_acquisition_time);

    // With optimized pool, most acquisitions should succeed
    assert!(
        successful_acquisitions >= concurrent_tasks * 8 / 10,
        "At least 80% of concurrent acquisitions should succeed"
    );

    // With optimized timeouts, max acquisition time should be reasonable
    assert!(
        max_acquisition_time < Duration::from_secs(10),
        "Max acquisition time should be under 10s with optimized timeout"
    );
}
