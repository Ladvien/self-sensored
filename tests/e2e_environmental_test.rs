use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "../tests/common/mod.rs"]
mod common;
use common::{cleanup_test_data, setup_test_db};

#[actix_web::test]
async fn test_insert_environmental_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test various environmental scenarios
    let test_cases = vec![
        (0, None, None, "No daylight exposure"),
        (30, Some(3.0), None, "30 min daylight, UV index 3"),
        (
            120,
            Some(8.0),
            Some(25.5),
            "2 hours daylight, high UV, warm temp",
        ),
        (
            180,
            Some(11.0),
            Some(35.0),
            "3 hours daylight, extreme UV, hot temp",
        ),
    ];

    for (daylight_mins, uv_index, temp, description) in test_cases {
        let recorded_at = Utc::now() - Duration::hours(rand::random::<u32>() as i64 % 24);

        let result = sqlx::query!(
            "INSERT INTO environmental_metrics (
                user_id, recorded_at, time_in_daylight_minutes,
                uv_index, ambient_temperature_celsius, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, recorded_at) DO NOTHING",
            user_id,
            recorded_at,
            daylight_mins,
            uv_index,
            temp,
            "Environmental Sensor"
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
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count.count.unwrap_or(0),
        4,
        "Should have inserted 4 environmental metrics"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_environmental_location_data() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test with location data
    let result = sqlx::query!(
        "INSERT INTO environmental_metrics (
            user_id, recorded_at,
            location_latitude, location_longitude,
            altitude_meters, source_device
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        37.7749,   // San Francisco latitude
        -122.4194, // San Francisco longitude
        16.0,      // Altitude in meters
        "GPS Sensor"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert location data: {:?}",
        result.err()
    );

    // Verify location data
    let stored = sqlx::query!(
        "SELECT location_latitude, location_longitude, altitude_meters
        FROM environmental_metrics
        WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored.location_latitude, Some(37.7749));
    assert_eq!(stored.location_longitude, Some(-122.4194));
    assert_eq!(stored.altitude_meters, Some(16.0));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_environmental_air_quality() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    let recorded_at = Utc::now();

    // Test air quality metrics
    let result = sqlx::query!(
        "INSERT INTO environmental_metrics (
            user_id, recorded_at,
            air_pressure_hpa, humidity_percent,
            ambient_temperature_celsius, source_device
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        1013.25, // Standard atmospheric pressure
        65.0,    // 65% humidity
        22.5,    // Comfortable temperature
        "Weather Station"
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "Failed to insert air quality data: {:?}",
        result.err()
    );

    // Verify air quality data
    let stored = sqlx::query!(
        "SELECT air_pressure_hpa, humidity_percent, ambient_temperature_celsius
        FROM environmental_metrics
        WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored.air_pressure_hpa, Some(1013.25));
    assert_eq!(stored.humidity_percent, Some(65.0));
    assert_eq!(stored.ambient_temperature_celsius, Some(22.5));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_environmental_audio_exposure() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Test different audio exposure levels
    let exposures = vec![
        (Some(50.0), None, "Quiet environment"),
        (Some(70.0), None, "Normal conversation level"),
        (Some(85.0), None, "City traffic"),
        (None, Some(95.0), "Loud headphones"),
        (Some(75.0), Some(80.0), "Both environmental and headphone"),
    ];

    for (env_db, headphone_db, description) in exposures {
        let recorded_at = Utc::now() - Duration::minutes(rand::random::<u32>() as i64 % 60);

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
            "Audio Sensor"
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

    // Query audio exposure stats
    let stats = sqlx::query!(
        "SELECT
            AVG(environmental_audio_exposure_db) as avg_env_db,
            MAX(headphone_audio_exposure_db) as max_headphone_db,
            COUNT(*) as count
        FROM environmental_metrics
        WHERE user_id = $1
        AND (environmental_audio_exposure_db IS NOT NULL
             OR headphone_audio_exposure_db IS NOT NULL)",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        stats.count.unwrap_or(0),
        5,
        "Should have 5 audio exposure records"
    );
    assert!(
        stats.avg_env_db.is_some(),
        "Should have average environmental dB"
    );
    assert_eq!(
        stats.max_headphone_db,
        Some(95.0),
        "Max headphone dB should be 95"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_load_environmental_fixture() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Load environmental fixture
    let fixture_content = tokio::fs::read_to_string("tests/fixtures/environmental_samples.json")
        .await
        .expect("Failed to read environmental fixture");

    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse fixture");

    let mut inserted_count = 0;

    if let Some(metrics) = fixture["data"]["metrics"].as_array() {
        for (idx, metric) in metrics.iter().take(20).enumerate() {
            if let Some(recorded_at_str) = metric["recorded_at"].as_str() {
                let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at_str)
                    .map(|dt| dt.with_timezone(&Utc) - Duration::seconds(idx as i64 * 30))
                    .unwrap_or_else(|_| Utc::now() - Duration::minutes(idx as i64));

                let daylight_mins = metric["time_in_daylight_minutes"]
                    .as_i64()
                    .map(|v| v as i32);
                let uv_index = metric["uv_index"].as_f64();
                let temp = metric["ambient_temperature_celsius"].as_f64();
                let humidity = metric["humidity_percent"].as_f64();

                let result = sqlx::query!(
                    "INSERT INTO environmental_metrics (
                        user_id, recorded_at,
                        time_in_daylight_minutes, uv_index,
                        ambient_temperature_celsius, humidity_percent,
                        source_device
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (user_id, recorded_at) DO NOTHING",
                    user_id,
                    recorded_at,
                    daylight_mins,
                    uv_index,
                    temp,
                    humidity,
                    metric["source_device"].as_str().unwrap_or("Unknown")
                )
                .execute(&pool)
                .await;

                if result.is_ok() && result.unwrap().rows_affected() > 0 {
                    inserted_count += 1;
                }
            }
        }

        println!(
            "Inserted {} environmental metrics from fixture",
            inserted_count
        );
    }

    // Verify metrics were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        count.count.unwrap_or(0) > 0,
        "Should have inserted environmental metrics from fixture"
    );

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_environmental_daily_summary() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;

    // Insert environmental data throughout a day
    let base_date = Utc::now().date_naive();
    let hours = vec![6, 9, 12, 15, 18]; // Morning to evening

    for hour in hours {
        let recorded_at = base_date
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let daylight = if hour >= 7 && hour <= 19 { 60 } else { 0 };
        let uv = if hour >= 10 && hour <= 16 { 5.0 } else { 1.0 };
        let temp = 15.0 + (hour as f64 - 6.0) * 1.5; // Temperature rises during day

        sqlx::query!(
            "INSERT INTO environmental_metrics (
                user_id, recorded_at,
                time_in_daylight_minutes, uv_index,
                ambient_temperature_celsius, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6)",
            user_id,
            recorded_at,
            daylight,
            uv,
            temp,
            "Weather Station"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert hourly environmental data");
    }

    // Query daily summary
    let summary = sqlx::query!(
        "SELECT
            DATE(recorded_at) as date,
            SUM(time_in_daylight_minutes) as total_daylight,
            MAX(uv_index) as max_uv,
            MIN(ambient_temperature_celsius) as min_temp,
            MAX(ambient_temperature_celsius) as max_temp,
            AVG(ambient_temperature_celsius) as avg_temp,
            COUNT(*) as readings
        FROM environmental_metrics
        WHERE user_id = $1
        GROUP BY DATE(recorded_at)",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        summary.total_daylight,
        Some(240),
        "Total daylight should be 240 minutes"
    );
    assert_eq!(summary.max_uv, Some(5.0), "Max UV should be 5");
    assert_eq!(summary.readings.unwrap_or(0), 5, "Should have 5 readings");
    assert!(
        summary.min_temp.unwrap() < summary.max_temp.unwrap(),
        "Temperature should increase during day"
    );

    cleanup_test_data(&pool, user_id).await;
}

// Helper function to create test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_env_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}
