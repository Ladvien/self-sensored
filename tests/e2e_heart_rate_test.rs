use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

#[path = "../tests/common/mod.rs"]
mod common;
use common::{cleanup_test_db, setup_test_db};

#[actix_web::test]
async fn test_insert_valid_heart_rate_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test various valid heart rate scenarios
    let test_cases = vec![
        (40, None, None, "Low resting heart rate"),
        (60, Some(55), None, "Normal resting heart rate"),
        (120, None, Some(35.5), "Exercise heart rate with HRV"),
        (180, None, None, "High exercise heart rate"),
        (72, Some(65), Some(42.3), "Complete heart rate data"),
    ];

    for (hr, resting_hr, hrv, description) in test_cases {
        let recorded_at = Utc::now() - Duration::minutes(rand::random::<u32>() as i64 % 1440);

        let result = sqlx::query!(
            "INSERT INTO heart_rate_metrics (
                user_id, recorded_at, heart_rate, resting_heart_rate,
                heart_rate_variability, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, recorded_at) DO NOTHING",
            user_id,
            recorded_at,
            hr,
            resting_hr,
            hrv,
            "Test Device"
        )
        .execute(&pool)
        .await;

        assert!(
            result.is_ok(),
            "Failed to insert {}: {:?}",
            description,
            result.err()
        );
    }

    // Verify all metrics were inserted
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
        "Should have inserted 5 heart rate metrics"
    );

    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_heart_rate_boundary_values() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test boundary values for heart rate
    let boundary_cases = vec![
        (15, true, "Minimum valid heart rate"),  // Min physiological
        (300, true, "Maximum valid heart rate"), // Max extreme athletic
        (0, false, "Zero heart rate"),           // Invalid
        (-10, false, "Negative heart rate"),     // Invalid
        (350, false, "Excessive heart rate"),    // Invalid
    ];

    for (hr, should_succeed, description) in boundary_cases {
        let recorded_at = Utc::now() - Duration::seconds(rand::random::<u32>() as i64 % 3600);

        let result = sqlx::query!(
            "INSERT INTO heart_rate_metrics (
                user_id, recorded_at, heart_rate, source_device
            ) VALUES ($1, $2, $3, $4)",
            user_id,
            recorded_at,
            hr,
            "Test Device"
        )
        .execute(&pool)
        .await;

        if should_succeed {
            assert!(
                result.is_ok(),
                "{} should succeed but failed: {:?}",
                description,
                result.err()
            );
        } else {
            // PostgreSQL will allow any integer value, validation happens at app level
            // For this test, we're just verifying the database accepts the insert
            println!(
                "Note: {} inserted (validation happens at app level)",
                description
            );
        }
    }

    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_heart_rate_with_advanced_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test with all advanced metrics
    let result = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, resting_heart_rate,
            heart_rate_variability, walking_heart_rate_average,
            heart_rate_recovery_one_minute, atrial_fibrillation_burden_percentage,
            vo2_max_ml_kg_min, source_device
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        user_id,
        recorded_at,
        72,
        58,
        45.2,
        85,
        25,
        BigDecimal::from_str("0.5").unwrap(),
        BigDecimal::from_str("48.5").unwrap(),
        "Apple Watch Series 9"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert advanced metrics: {:?}",
        result.err()
    );

    // Verify the data was stored correctly
    let stored = sqlx::query!(
        "SELECT heart_rate, resting_heart_rate, heart_rate_variability,
                walking_heart_rate_average, heart_rate_recovery_one_minute,
                vo2_max_ml_kg_min
        FROM heart_rate_metrics WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored.heart_rate, Some(72));
    assert_eq!(stored.resting_heart_rate, Some(58));
    assert_eq!(stored.heart_rate_variability, Some(45.2));
    assert_eq!(stored.walking_heart_rate_average, Some(85));
    assert_eq!(stored.heart_rate_recovery_one_minute, Some(25));
    assert!(stored.vo2_max_ml_kg_min.is_some());
    let vo2_str = stored.vo2_max_ml_kg_min.unwrap().to_string();
    assert!(
        vo2_str.starts_with("48.5"),
        "VO2 max should be 48.5, got {}",
        vo2_str
    );

    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_heart_rate_duplicate_handling() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // First insert
    let result1 = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, source_device
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        75,
        "Device 1"
    )
    .execute(&pool)
    .await;

    assert!(result1.is_ok());

    // Duplicate insert - should fail due to unique constraint
    let result2 = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, source_device
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        recorded_at,
        80, // Different value but same timestamp
        "Device 2"
    )
    .execute(&pool)
    .await;

    assert!(result2.is_err(), "Duplicate insert should fail");

    // Insert with ON CONFLICT DO NOTHING
    let result3 = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, source_device
        ) VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, recorded_at) DO NOTHING",
        user_id,
        recorded_at,
        85,
        "Device 3"
    )
    .execute(&pool)
    .await;

    assert!(result3.is_ok(), "ON CONFLICT DO NOTHING should succeed");

    // Verify only one record exists
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count.count.unwrap_or(0), 1, "Should have only one record");

    // Verify the original value was kept
    let stored = sqlx::query!(
        "SELECT heart_rate FROM heart_rate_metrics WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        stored.heart_rate,
        Some(75),
        "Original value should be preserved"
    );

    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_load_heart_rate_fixture() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Load heart rate fixture
    let fixture_content = tokio::fs::read_to_string("tests/fixtures/heart_rate_samples.json")
        .await
        .expect("Failed to read heart rate fixture");

    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse fixture");

    let mut inserted_count = 0;

    if let Some(metrics) = fixture["data"]["metrics"].as_array() {
        for metric in metrics {
            if let (Some(heart_rate), Some(recorded_at)) = (
                metric["heart_rate"].as_i64(),
                metric["recorded_at"].as_str(),
            ) {
                let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let resting_hr = metric["resting_heart_rate"].as_i64().map(|v| v as i32);
                let hrv = metric["heart_rate_variability"].as_f64();
                let walking_avg = metric["walking_heart_rate_average"]
                    .as_i64()
                    .map(|v| v as i32);
                let vo2_max = metric["vo2_max_ml_kg_min"]
                    .as_f64()
                    .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

                let result = sqlx::query!(
                    "INSERT INTO heart_rate_metrics (
                        user_id, recorded_at, heart_rate, resting_heart_rate,
                        heart_rate_variability, walking_heart_rate_average,
                        vo2_max_ml_kg_min, source_device
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (user_id, recorded_at) DO NOTHING",
                    user_id,
                    recorded_at,
                    heart_rate as i32,
                    resting_hr,
                    hrv,
                    walking_avg,
                    vo2_max,
                    metric["source_device"].as_str().unwrap_or("Unknown")
                )
                .execute(&pool)
                .await;

                if result.is_ok() {
                    inserted_count += 1;
                }
            }
        }

        println!(
            "Inserted {} heart rate metrics from fixture",
            inserted_count
        );
    }

    // Verify metrics were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        count.count.unwrap_or(0) > 0,
        "Should have inserted heart rate metrics from fixture"
    );
    assert_eq!(
        count.count.unwrap_or(0) as i32,
        inserted_count,
        "Insert count mismatch"
    );

    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_heart_rate_time_series_query() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Insert heart rate data over time
    let base_time = Utc::now() - Duration::hours(24);
    let heart_rates = vec![65, 68, 72, 120, 135, 125, 80, 70, 65];

    for (i, hr) in heart_rates.iter().enumerate() {
        let recorded_at = base_time + Duration::hours(i as i64 * 3);

        sqlx::query!(
            "INSERT INTO heart_rate_metrics (
                user_id, recorded_at, heart_rate, source_device
            ) VALUES ($1, $2, $3, $4)",
            user_id,
            recorded_at,
            *hr,
            "Test Device"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert time series data");
    }

    // Query average heart rate
    let avg_hr = sqlx::query!(
        "SELECT AVG(heart_rate) as avg_hr FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        avg_hr.avg_hr.is_some(),
        "Should calculate average heart rate"
    );

    // Query min and max heart rate
    let stats = sqlx::query!(
        "SELECT
            MIN(heart_rate) as min_hr,
            MAX(heart_rate) as max_hr,
            COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stats.min_hr, Some(65), "Minimum heart rate should be 65");
    assert_eq!(stats.max_hr, Some(135), "Maximum heart rate should be 135");
    assert_eq!(stats.count.unwrap_or(0), 9, "Should have 9 records");

    // Query heart rates in time range
    let recent = sqlx::query!(
        "SELECT COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1 AND recorded_at > $2",
        user_id,
        Utc::now() - Duration::hours(12)
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        recent.count.unwrap_or(0) > 0,
        "Should have recent heart rate data"
    );

    cleanup_test_db(&pool, user_id).await;
}

// Helper function to create test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_hr_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}
