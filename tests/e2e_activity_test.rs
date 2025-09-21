use actix_web::{http::StatusCode, test, web, App};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[path = "../tests/common/mod.rs"]
mod common;
use common::{cleanup_test_data, setup_test_db};

#[actix_web::test]
async fn test_insert_basic_activity_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test basic activity scenarios
    let test_cases = vec![
        (0, 0.0, 0, "No activity day"),
        (5000, 3500.0, 5, "Light activity"),
        (10000, 7200.0, 10, "Normal active day"),
        (15000, 11000.0, 15, "Very active day"),
        (25000, 18500.0, 25, "Extremely active day"),
    ];

    for (steps, distance, flights, description) in test_cases {
        let recorded_at = Utc::now() - Duration::days(rand::random::<u32>() as i64 % 7);

        let result = sqlx::query!(
            "INSERT INTO activity_metrics (
                user_id, recorded_at, step_count, distance_meters,
                flights_climbed, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, recorded_at) DO NOTHING",
            user_id,
            recorded_at,
            steps,
            distance,
            flights,
            "iPhone"
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
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        5,
        "Should have inserted 5 activity metrics"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_activity_energy_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test energy burned metrics
    let result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at, step_count,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            source_device
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        8500,
        320.5,  // Active calories
        1650.0, // Basal calories
        "Apple Watch"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert energy metrics: {:?}",
        result.err()
    );

    // Verify the data
    let stored = sqlx::query!(
        "SELECT active_energy_burned_kcal, basal_energy_burned_kcal
        FROM activity_metrics
        WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored.active_energy_burned_kcal, Some(320.5));
    assert_eq!(stored.basal_energy_burned_kcal, Some(1650.0));

    // Calculate total energy
    let total_energy = stored.active_energy_burned_kcal.unwrap_or(0.0)
        + stored.basal_energy_burned_kcal.unwrap_or(0.0);
    assert_eq!(total_energy, 1970.5, "Total energy should be 1970.5 kcal");

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_activity_distance_types() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test various distance types
    let result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at,
            distance_meters, distance_cycling_meters,
            distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters,
            source_device
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        user_id,
        recorded_at,
        5000.0,  // Total walking distance
        10000.0, // Cycling distance
        500.0,   // Swimming distance
        0.0,     // Wheelchair distance
        3000.0,  // Skiing distance
        "Multi-sport Watch"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert distance metrics: {:?}",
        result.err()
    );

    // Query all distance types
    let distances = sqlx::query!(
        "SELECT
            distance_meters,
            distance_cycling_meters,
            distance_swimming_meters,
            distance_wheelchair_meters,
            distance_downhill_snow_sports_meters
        FROM activity_metrics
        WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(distances.distance_meters, Some(5000.0));
    assert_eq!(distances.distance_cycling_meters, Some(10000.0));
    assert_eq!(distances.distance_swimming_meters, Some(500.0));
    assert_eq!(distances.distance_wheelchair_meters, Some(0.0));
    assert_eq!(distances.distance_downhill_snow_sports_meters, Some(3000.0));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_apple_specific_activity_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test Apple-specific metrics
    let result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at,
            apple_exercise_time_minutes,
            apple_stand_time_minutes,
            apple_move_time_minutes,
            apple_stand_hour_achieved,
            source_device
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        user_id,
        recorded_at,
        30,   // Exercise minutes (green ring)
        12,   // Stand minutes
        45,   // Move minutes (red ring)
        true, // Stand hour achieved (blue ring)
        "Apple Watch"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert Apple metrics: {:?}",
        result.err()
    );

    // Verify Apple activity rings data
    let apple_data = sqlx::query!(
        "SELECT
            apple_exercise_time_minutes,
            apple_stand_time_minutes,
            apple_move_time_minutes,
            apple_stand_hour_achieved
        FROM activity_metrics
        WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(apple_data.apple_exercise_time_minutes, Some(30));
    assert_eq!(apple_data.apple_stand_time_minutes, Some(12));
    assert_eq!(apple_data.apple_move_time_minutes, Some(45));
    assert_eq!(apple_data.apple_stand_hour_achieved, Some(true));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_swimming_and_wheelchair_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test swimming metrics
    let swim_time = Utc::now() - Duration::hours(2);
    let swim_result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at,
            distance_swimming_meters,
            swimming_stroke_count,
            source_device
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        swim_time,
        1500.0, // 1.5km swim
        1200,   // Stroke count
        "Swimming Watch"
    )
    .execute(&pool)
    .await;

    assert!(swim_result.is_ok(), "Failed to insert swimming metrics");

    // Test wheelchair metrics
    let wheel_time = Utc::now() - Duration::hours(1);
    let wheel_result = sqlx::query!(
        "INSERT INTO activity_metrics (
            user_id, recorded_at,
            distance_wheelchair_meters,
            push_count,
            source_device
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        wheel_time,
        3000.0, // 3km wheelchair distance
        2500,   // Push count
        "Wheelchair Tracker"
    )
    .execute(&pool)
    .await;

    assert!(wheel_result.is_ok(), "Failed to insert wheelchair metrics");

    // Verify both were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        2,
        "Should have 2 activity records"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_load_activity_fixture() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Load activity fixture
    let fixture_content = tokio::fs::read_to_string("tests/fixtures/activity_samples.json")
        .await
        .expect("Failed to read activity fixture");

    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse fixture");

    let mut inserted_count = 0;

    if let Some(metrics) = fixture["data"]["metrics"].as_array() {
        for metric in metrics.iter().take(20) {
            // Process first 20 to avoid duplicates
            if let Some(recorded_at_str) = metric["recorded_at"].as_str() {
                let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now() - Duration::seconds(inserted_count * 60));

                let step_count = metric["step_count"].as_i64().map(|v| v as i32);
                let distance = metric["distance_meters"].as_f64();
                let flights = metric["flights_climbed"].as_i64().map(|v| v as i32);
                let active_energy = metric["active_energy_burned_kcal"].as_f64();
                let basal_energy = metric["basal_energy_burned_kcal"].as_f64();

                let result = sqlx::query!(
                    "INSERT INTO activity_metrics (
                        user_id, recorded_at, step_count, distance_meters,
                        flights_climbed, active_energy_burned_kcal,
                        basal_energy_burned_kcal, source_device
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (user_id, recorded_at) DO NOTHING",
                    user_id,
                    recorded_at,
                    step_count,
                    distance,
                    flights,
                    active_energy,
                    basal_energy,
                    metric["source_device"].as_str().unwrap_or("Unknown")
                )
                .execute(&pool)
                .await;

                if result.is_ok() && result.unwrap().rows_affected() > 0 {
                    inserted_count += 1;
                }
            }
        }

        println!("Inserted {} activity metrics from fixture", inserted_count);
    }

    // Verify metrics were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        count.count.unwrap_or(0) > 0,
        "Should have inserted activity metrics from fixture"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_activity_daily_aggregation() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Insert multiple activity records for the same day
    let base_date = Utc::now().date_naive();
    let times = vec![8, 12, 16, 20]; // Different hours of the day

    for hour in times {
        let recorded_at = base_date
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        sqlx::query!(
            "INSERT INTO activity_metrics (
                user_id, recorded_at, step_count, distance_meters,
                active_energy_burned_kcal, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            recorded_at,
            2500,
            1800.0,
            75.0,
            "iPhone"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert hourly activity");
    }

    // Query daily totals
    let daily_stats = sqlx::query!(
        "SELECT
            DATE(recorded_at) as activity_date,
            SUM(step_count) as total_steps,
            SUM(distance_meters) as total_distance,
            SUM(active_energy_burned_kcal) as total_active_energy,
            COUNT(*) as record_count
        FROM activity_metrics
        WHERE user_id = $1
        GROUP BY DATE(recorded_at)",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        daily_stats.total_steps,
        Some(10000),
        "Total steps should be 10000"
    );
    assert_eq!(
        daily_stats.total_distance,
        Some(7200.0),
        "Total distance should be 7200m"
    );
    assert_eq!(
        daily_stats.total_active_energy,
        Some(300.0),
        "Total energy should be 300 kcal"
    );
    assert_eq!(
        daily_stats.record_count.unwrap_or(0),
        4,
        "Should have 4 records"
    );

    cleanup_test_data(&pool, user_id).await;
}

// Helper function to create test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_activity_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}
