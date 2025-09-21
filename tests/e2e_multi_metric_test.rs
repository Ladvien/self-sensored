use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[path = "../tests/common/mod.rs"]
mod common;
use common::{cleanup_test_data, setup_test_db};

#[actix_web::test]
async fn test_mixed_metrics_insertion() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let base_time = Utc::now();

    // Insert different metric types
    let mut total_inserted = 0;

    // Heart rate metric
    let hr_result = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, source_device
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time - Duration::minutes(10),
        72,
        "Multi-sensor Device"
    )
    .execute(&pool)
    .await;

    if hr_result.is_ok() {
        total_inserted += 1;
    }

    // Activity metric
    let activity_result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at, step_count, distance_meters, source_device
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        base_time - Duration::minutes(20),
        5000,
        3500.0,
        "Multi-sensor Device"
    )
    .execute(&pool)
    .await;

    if activity_result.is_ok() {
        total_inserted += 1;
    }

    // Body measurement
    let body_result = sqlx::query!(
        "INSERT INTO body_measurements (
            user_id, recorded_at, body_mass_index, source_device
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        base_time - Duration::minutes(30),
        24.5,
        "Multi-sensor Device"
    )
    .execute(&pool)
    .await;

    if body_result.is_ok() {
        total_inserted += 1;
    }

    // Environmental metric
    let env_result = sqlx::query!(
        "INSERT INTO environmental_metrics (
            user_id, recorded_at, time_in_daylight_minutes,
            ambient_temperature_celsius, source_device
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        base_time - Duration::minutes(40),
        30,
        22.5,
        "Multi-sensor Device"
    )
    .execute(&pool)
    .await;

    if env_result.is_ok() {
        total_inserted += 1;
    }

    assert_eq!(
        total_inserted, 4,
        "Should have inserted 4 different metric types"
    );

    // Verify all metrics were inserted
    let hr_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let body_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM body_measurements WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    let env_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    assert_eq!(hr_count, 1, "Should have 1 heart rate metric");
    assert_eq!(activity_count, 1, "Should have 1 activity metric");
    assert_eq!(body_count, 1, "Should have 1 body measurement");
    assert_eq!(env_count, 1, "Should have 1 environmental metric");

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_concurrent_metric_insertion() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Create multiple metrics to insert concurrently
    let base_time = Utc::now();
    let mut handles = vec![];

    // Spawn concurrent insertions
    for i in 0..5 {
        let pool_clone = pool.clone();
        let time = base_time - Duration::seconds(i * 10);

        let handle = tokio::spawn(async move {
            sqlx::query!(
                "INSERT INTO heart_rate_metrics (
                    user_id, recorded_at, heart_rate, source_device
                ) VALUES ($1, $2, $3, $4)
                ON CONFLICT (user_id, recorded_at) DO NOTHING",
                user_id,
                time,
                60 + (i * 5) as i32,
                "Concurrent Device"
            )
            .execute(&pool_clone)
            .await
        });

        handles.push(handle);
    }

    // Wait for all insertions to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(result) = handle.await {
            if result.is_ok() {
                success_count += 1;
            }
        }
    }

    assert_eq!(
        success_count, 5,
        "All 5 concurrent insertions should succeed"
    );

    // Verify all were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        5,
        "Should have 5 heart rate metrics"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_load_mixed_fixture() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Load mixed metrics fixture
    let fixture_content = tokio::fs::read_to_string("tests/fixtures/mixed_metrics.json")
        .await
        .expect("Failed to read mixed fixture");

    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse fixture");

    let mut metrics_by_type = std::collections::HashMap::new();

    if let Some(metrics) = fixture["data"]["metrics"].as_array() {
        for (idx, metric) in metrics.iter().enumerate() {
            let metric_type = metric["type"].as_str().unwrap_or("Unknown");
            let recorded_at_str = metric["recorded_at"].as_str().unwrap_or("");

            let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at_str)
                .map(|dt| dt.with_timezone(&Utc) - Duration::seconds(idx as i64 * 60))
                .unwrap_or_else(|_| Utc::now() - Duration::minutes(idx as i64));

            let source = metric["source_device"].as_str().unwrap_or("Test Device");

            match metric_type {
                "HeartRate" => {
                    if let Some(hr) = metric["heart_rate"].as_i64() {
                        let result = sqlx::query!(
                            "INSERT INTO heart_rate_metrics (
                                user_id, recorded_at, heart_rate, source_device
                            ) VALUES ($1, $2, $3, $4)
                            ON CONFLICT (user_id, recorded_at) DO NOTHING",
                            user_id,
                            recorded_at,
                            hr as i32,
                            source
                        )
                        .execute(&pool)
                        .await;

                        if result.is_ok() && result.unwrap().rows_affected() > 0 {
                            *metrics_by_type.entry("HeartRate").or_insert(0) += 1;
                        }
                    }
                }
                "Activity" => {
                    let steps = metric["step_count"].as_i64().map(|v| v as i32);
                    let distance = metric["distance_meters"].as_f64();

                    let result = sqlx::query!(
                        "INSERT INTO activity_metrics (
                            user_id, recorded_at, step_count, distance_meters, source_device
                        ) VALUES ($1, $2, $3, $4, $5)
                        ON CONFLICT (user_id, recorded_at) DO NOTHING",
                        user_id,
                        recorded_at,
                        steps,
                        distance,
                        source
                    )
                    .execute(&pool)
                    .await;

                    if result.is_ok() && result.unwrap().rows_affected() > 0 {
                        *metrics_by_type.entry("Activity").or_insert(0) += 1;
                    }
                }
                "BodyMeasurement" => {
                    let bmi = metric["body_mass_index"].as_f64();

                    if bmi.is_some() {
                        let result = sqlx::query!(
                            "INSERT INTO body_measurements (
                                user_id, recorded_at, body_mass_index, source_device
                            ) VALUES ($1, $2, $3, $4)
                            ON CONFLICT (user_id, recorded_at) DO NOTHING",
                            user_id,
                            recorded_at,
                            bmi,
                            source
                        )
                        .execute(&pool)
                        .await;

                        if result.is_ok() && result.unwrap().rows_affected() > 0 {
                            *metrics_by_type.entry("BodyMeasurement").or_insert(0) += 1;
                        }
                    }
                }
                "Environmental" => {
                    let daylight = metric["time_in_daylight_minutes"]
                        .as_i64()
                        .map(|v| v as i32);

                    let result = sqlx::query!(
                        "INSERT INTO environmental_metrics (
                            user_id, recorded_at, time_in_daylight_minutes, source_device
                        ) VALUES ($1, $2, $3, $4)
                        ON CONFLICT (user_id, recorded_at) DO NOTHING",
                        user_id,
                        recorded_at,
                        daylight,
                        source
                    )
                    .execute(&pool)
                    .await;

                    if result.is_ok() && result.unwrap().rows_affected() > 0 {
                        *metrics_by_type.entry("Environmental").or_insert(0) += 1;
                    }
                }
                "AudioExposure" => {
                    let env_db = metric["environmental_audio_exposure_db"].as_f64();
                    let headphone_db = metric["headphone_audio_exposure_db"].as_f64();

                    let result = sqlx::query!(
                        "INSERT INTO environmental_metrics (
                            user_id, recorded_at,
                            environmental_audio_exposure_db,
                            headphone_audio_exposure_db,
                            source_device
                        ) VALUES ($1, $2, $3, $4, $5)
                        ON CONFLICT (user_id, recorded_at) DO NOTHING",
                        user_id,
                        recorded_at,
                        env_db,
                        headphone_db,
                        source
                    )
                    .execute(&pool)
                    .await;

                    if result.is_ok() && result.unwrap().rows_affected() > 0 {
                        *metrics_by_type.entry("AudioExposure").or_insert(0) += 1;
                    }
                }
                _ => {
                    println!("Unknown metric type: {}", metric_type);
                }
            }
        }
    }

    // Print summary
    println!("Inserted metrics by type:");
    for (metric_type, count) in &metrics_by_type {
        println!("  {}: {}", metric_type, count);
    }

    // Verify at least some metrics were inserted
    let total_inserted: i32 = metrics_by_type.values().sum();
    assert!(
        total_inserted > 0,
        "Should have inserted at least some metrics from mixed fixture"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_transaction_isolation() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Start a transaction that will fail
    let mut tx = pool.begin().await.unwrap();

    // First insert should succeed
    let result1 = sqlx::query!(
        "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, source_device)
        VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        75,
        "Device 1"
    )
    .execute(&mut *tx)
    .await;

    assert!(result1.is_ok(), "First insert should succeed");

    // Second insert will fail due to duplicate
    let result2 = sqlx::query!(
        "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, source_device)
        VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        80,
        "Device 2"
    )
    .execute(&mut *tx)
    .await;

    assert!(
        result2.is_err(),
        "Second insert should fail due to duplicate"
    );

    // Rollback transaction
    tx.rollback().await.unwrap();

    // Verify no data was inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        0,
        "No data should be inserted due to failed transaction"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_bulk_metric_performance() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let start = Instant::now();
    let base_time = Utc::now();

    // Insert 100 metrics
    for i in 0..100 {
        let recorded_at = base_time - Duration::seconds(i * 30);

        sqlx::query!(
            "INSERT INTO heart_rate_metrics (
                user_id, recorded_at, heart_rate, source_device
            ) VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, recorded_at) DO NOTHING",
            user_id,
            recorded_at,
            60 + (i % 40) as i32,
            "Performance Test Device"
        )
        .execute(&pool)
        .await
        .ok();
    }

    let duration = start.elapsed();
    println!("Inserted 100 metrics in {:?}", duration);

    // Verify insertion
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        100,
        "Should have inserted 100 metrics"
    );
    assert!(
        duration.as_secs() < 10,
        "Bulk insertion should complete within 10 seconds"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_cross_metric_queries() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Insert correlated metrics (e.g., during exercise)
    sqlx::query!(
        "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, source_device)
        VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        140,
        "Exercise Device"
    )
    .execute(&pool)
    .await
    .expect("Failed to insert heart rate");

    sqlx::query!(
        "INSERT INTO activity_metrics (user_id, recorded_at, step_count, active_energy_burned_kcal, source_device)
        VALUES ($1, $2, $3, $4, $5)",
        user_id, recorded_at, 2000, 150.0, "Exercise Device"
    )
    .execute(&pool)
    .await
    .expect("Failed to insert activity");

    // Query correlated data
    let correlation = sqlx::query!(
        "SELECT
            h.heart_rate,
            a.step_count,
            a.active_energy_burned_kcal
        FROM heart_rate_metrics h
        JOIN activity_metrics a ON h.user_id = a.user_id
            AND h.recorded_at = a.recorded_at
        WHERE h.user_id = $1 AND h.recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(correlation.is_some(), "Should find correlated metrics");
    let data = correlation.unwrap();
    assert_eq!(data.heart_rate, Some(140));
    assert_eq!(data.step_count, Some(2000));
    assert_eq!(data.active_energy_burned_kcal, Some(150.0));

    cleanup_test_data(&pool, user_id).await;
}

// Helper function to create test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_multi_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}
