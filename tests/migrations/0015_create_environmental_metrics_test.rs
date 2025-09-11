use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_environmental_metrics_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'environmental_metrics' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "environmental_metrics table should exist");

    // Verify all essential columns exist
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Audio exposure
        "environmental_sound_level_db", "headphone_exposure_db", "noise_reduction_db", "exposure_duration_seconds",
        // UV exposure
        "uv_index", "time_in_sun_minutes", "time_in_shade_minutes", "sunscreen_applied", "uv_dose_joules_per_m2",
        // Fall detection
        "fall_detected", "fall_severity", "impact_force_g", "emergency_contacted", "fall_response_time_seconds",
        // Hygiene tracking
        "handwashing_events", "handwashing_duration_seconds", "toothbrushing_events", "toothbrushing_duration_seconds",
        // Air quality
        "pm2_5_micrograms_m3", "pm10_micrograms_m3", "air_quality_index", "ozone_ppb", "no2_ppb", "so2_ppb", "co_ppm",
        // Location context
        "altitude_meters", "barometric_pressure_hpa", "indoor_outdoor_context",
        // Aggregation support
        "aggregation_period", "measurement_count",
        // Metadata
        "source", "device_type", "raw_data", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in environmental_metrics", 
            expected_col
        );
    }

    // Verify we have the expected number of fields (33+ environmental fields plus metadata)
    assert!(
        column_names.len() >= 35, 
        "Should have at least 35 columns, found: {}", 
        column_names.len()
    );

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'environmental_metrics'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "environmental_metrics should be partitioned");

    // Verify initial partitions were created (should have 4 months: current + 3 ahead)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'environmental_metrics_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 4,
        "Should have at least 4 partitions, found: {}",
        partition_count.unwrap_or(0)
    );

    Ok(())
}

#[sqlx::test]
async fn test_audio_exposure_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid audio exposure values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db, headphone_exposure_db, noise_reduction_db, exposure_duration_seconds)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time, 65.5, 80.0, 25.5, 3600
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid audio exposure values should be accepted");

    // Test boundary values (0 dB, 140 dB)
    let boundary_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db, headphone_exposure_db, noise_reduction_db)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 0.0, 140.0, 60.0
    )
    .execute(&pool)
    .await;

    assert!(boundary_result.is_ok(), "Boundary audio values (0, 140, 60 dB) should be accepted");

    // Test invalid values (negative, over 140 dB)
    let invalid_env_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(2)).unwrap(), -5.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_env_result.is_err(), "Negative environmental sound level should be rejected");

    let invalid_headphone_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, headphone_exposure_db)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(3)).unwrap(), 150.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_headphone_result.is_err(), "Headphone exposure > 140 dB should be rejected");

    let invalid_noise_reduction_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, noise_reduction_db)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(4)).unwrap(), 70.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_noise_reduction_result.is_err(), "Noise reduction > 60 dB should be rejected");

    // Test invalid exposure duration (over 24 hours)
    let invalid_duration_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, exposure_duration_seconds)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(5)).unwrap(), 90000  // > 86400 seconds
    )
    .execute(&pool)
    .await;

    assert!(invalid_duration_result.is_err(), "Exposure duration > 24 hours should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_uv_exposure_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid UV exposure values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index, time_in_sun_minutes, time_in_shade_minutes, sunscreen_applied, uv_dose_joules_per_m2)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        user_id, test_time, 8.5, 120, 300, true, 5000.0
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid UV exposure values should be accepted");

    // Test boundary values (UV Index 0.0, 15.0)
    let boundary_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index, time_in_sun_minutes, time_in_shade_minutes)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 0.0, 0, 1440  // Max 24 hours shade
    )
    .execute(&pool)
    .await;

    assert!(boundary_result.is_ok(), "Boundary UV values should be accepted");

    let extreme_uv_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(2)).unwrap(), 15.0
    )
    .execute(&pool)
    .await;

    assert!(extreme_uv_result.is_ok(), "Extreme UV Index 15.0 should be accepted");

    // Test invalid UV values
    let invalid_uv_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(3)).unwrap(), -1.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_uv_result.is_err(), "Negative UV Index should be rejected");

    let over_limit_uv_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(4)).unwrap(), 16.0
    )
    .execute(&pool)
    .await;

    assert!(over_limit_uv_result.is_err(), "UV Index > 15.0 should be rejected");

    // Test invalid time values (over 24 hours)
    let invalid_sun_time_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, time_in_sun_minutes)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(5)).unwrap(), 1500  // > 1440 minutes
    )
    .execute(&pool)
    .await;

    assert!(invalid_sun_time_result.is_err(), "Sun time > 24 hours should be rejected");

    // Test invalid UV dose
    let invalid_dose_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_dose_joules_per_m2)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(6)).unwrap(), -100.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_dose_result.is_err(), "Negative UV dose should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_fall_detection_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid fall detection values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, fall_detected, fall_severity, impact_force_g, emergency_contacted, fall_response_time_seconds)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        user_id, test_time, true, "high", 15.5, false, 30
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid fall detection values should be accepted");

    // Test all fall severity levels
    let severities = ["low", "medium", "high", "severe"];
    for (i, severity) in severities.iter().enumerate() {
        let result = sqlx::query!(
            "INSERT INTO environmental_metrics (user_id, recorded_at, fall_detected, fall_severity)
             VALUES ($1, $2, $3, $4)",
            user_id, test_time.checked_add_signed(chrono::Duration::seconds(i as i64 + 1)).unwrap(), true, *severity
        )
        .execute(&pool)
        .await;

        assert!(result.is_ok(), "Fall severity '{}' should be accepted", severity);
    }

    // Test boundary impact force values (0G, 50G)
    let boundary_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, impact_force_g)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(10)).unwrap(), 0.0
    )
    .execute(&pool)
    .await;

    assert!(boundary_result.is_ok(), "Impact force 0G should be accepted");

    let max_force_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, impact_force_g)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(11)).unwrap(), 50.0
    )
    .execute(&pool)
    .await;

    assert!(max_force_result.is_ok(), "Impact force 50G should be accepted");

    // Test invalid fall severity
    let invalid_severity_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, fall_severity)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(12)).unwrap(), "extreme"
    )
    .execute(&pool)
    .await;

    assert!(invalid_severity_result.is_err(), "Invalid fall severity should be rejected");

    // Test invalid impact force (negative, over 50G)
    let negative_force_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, impact_force_g)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(13)).unwrap(), -5.0
    )
    .execute(&pool)
    .await;

    assert!(negative_force_result.is_err(), "Negative impact force should be rejected");

    let excessive_force_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, impact_force_g)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(14)).unwrap(), 60.0
    )
    .execute(&pool)
    .await;

    assert!(excessive_force_result.is_err(), "Impact force > 50G should be rejected");

    // Test invalid response time (over 1 hour)
    let invalid_response_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, fall_response_time_seconds)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(15)).unwrap(), 4000  // > 3600 seconds
    )
    .execute(&pool)
    .await;

    assert!(invalid_response_result.is_err(), "Response time > 1 hour should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_hygiene_tracking_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid hygiene tracking values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, handwashing_events, handwashing_duration_seconds, toothbrushing_events, toothbrushing_duration_seconds)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time, 8, 25, 2, 180
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid hygiene tracking values should be accepted");

    // Test boundary values
    let boundary_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, handwashing_events, handwashing_duration_seconds, toothbrushing_events, toothbrushing_duration_seconds)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 0, 0, 0, 0
    )
    .execute(&pool)
    .await;

    assert!(boundary_result.is_ok(), "Zero hygiene values should be accepted");

    let max_values_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, handwashing_events, handwashing_duration_seconds, toothbrushing_events, toothbrushing_duration_seconds)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(2)).unwrap(), 100, 300, 10, 600
    )
    .execute(&pool)
    .await;

    assert!(max_values_result.is_ok(), "Maximum hygiene values should be accepted");

    // Test invalid handwashing values
    let invalid_handwashing_events_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, handwashing_events)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(3)).unwrap(), 101  // > 100
    )
    .execute(&pool)
    .await;

    assert!(invalid_handwashing_events_result.is_err(), "Handwashing events > 100 should be rejected");

    let invalid_handwashing_duration_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, handwashing_duration_seconds)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(4)).unwrap(), 350  // > 300
    )
    .execute(&pool)
    .await;

    assert!(invalid_handwashing_duration_result.is_err(), "Handwashing duration > 5 minutes should be rejected");

    // Test invalid toothbrushing values
    let invalid_toothbrushing_events_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, toothbrushing_events)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(5)).unwrap(), 15  // > 10
    )
    .execute(&pool)
    .await;

    assert!(invalid_toothbrushing_events_result.is_err(), "Toothbrushing events > 10 should be rejected");

    let invalid_toothbrushing_duration_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, toothbrushing_duration_seconds)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(6)).unwrap(), 700  // > 600
    )
    .execute(&pool)
    .await;

    assert!(invalid_toothbrushing_duration_result.is_err(), "Toothbrushing duration > 10 minutes should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_air_quality_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid air quality values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, pm2_5_micrograms_m3, pm10_micrograms_m3, air_quality_index, ozone_ppb, no2_ppb, so2_ppb, co_ppm)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        user_id, test_time, 35.5, 80.0, 150, 45.2, 25.8, 15.3, 2.5
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid air quality values should be accepted");

    // Test boundary values (AQI 0-500, PM 0-1000)
    let boundary_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, pm2_5_micrograms_m3, pm10_micrograms_m3, air_quality_index)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 0.0, 0.0, 0
    )
    .execute(&pool)
    .await;

    assert!(boundary_result.is_ok(), "Zero air quality values should be accepted");

    let max_values_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, pm2_5_micrograms_m3, pm10_micrograms_m3, air_quality_index, ozone_ppb, no2_ppb, so2_ppb, co_ppm)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(2)).unwrap(), 1000.0, 1000.0, 500, 1000.0, 2000.0, 1000.0, 100.0
    )
    .execute(&pool)
    .await;

    assert!(max_values_result.is_ok(), "Maximum air quality values should be accepted");

    // Test invalid AQI values
    let invalid_aqi_negative_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, air_quality_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(3)).unwrap(), -10
    )
    .execute(&pool)
    .await;

    assert!(invalid_aqi_negative_result.is_err(), "Negative AQI should be rejected");

    let invalid_aqi_high_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, air_quality_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(4)).unwrap(), 600  // > 500
    )
    .execute(&pool)
    .await;

    assert!(invalid_aqi_high_result.is_err(), "AQI > 500 should be rejected");

    // Test invalid PM values
    let invalid_pm25_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, pm2_5_micrograms_m3)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(5)).unwrap(), 1100.0  // > 1000
    )
    .execute(&pool)
    .await;

    assert!(invalid_pm25_result.is_err(), "PM2.5 > 1000 should be rejected");

    // Test invalid gas concentrations
    let invalid_co_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, co_ppm)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(6)).unwrap(), 150.0  // > 100
    )
    .execute(&pool)
    .await;

    assert!(invalid_co_result.is_err(), "CO > 100 ppm should be rejected");

    let invalid_no2_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, no2_ppb)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(7)).unwrap(), 2500.0  // > 2000
    )
    .execute(&pool)
    .await;

    assert!(invalid_no2_result.is_err(), "NO2 > 2000 ppb should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_geographic_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test valid geographic values
    let valid_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, altitude_meters, barometric_pressure_hpa, indoor_outdoor_context)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time, 1500.5, 1013.25, "outdoor"
    )
    .execute(&pool)
    .await;

    assert!(valid_result.is_ok(), "Valid geographic values should be accepted");

    // Test all indoor_outdoor_context values
    let contexts = ["indoor", "outdoor", "mixed", "unknown"];
    for (i, context) in contexts.iter().enumerate() {
        let result = sqlx::query!(
            "INSERT INTO environmental_metrics (user_id, recorded_at, indoor_outdoor_context)
             VALUES ($1, $2, $3)",
            user_id, test_time.checked_add_signed(chrono::Duration::seconds(i as i64 + 1)).unwrap(), *context
        )
        .execute(&pool)
        .await;

        assert!(result.is_ok(), "Indoor/outdoor context '{}' should be accepted", context);
    }

    // Test boundary altitude values (Dead Sea to Everest range)
    let min_altitude_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, altitude_meters)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(10)).unwrap(), -500.0  // Dead Sea level
    )
    .execute(&pool)
    .await;

    assert!(min_altitude_result.is_ok(), "Minimum altitude (-500m) should be accepted");

    let max_altitude_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, altitude_meters)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(11)).unwrap(), 9000.0  // Everest range
    )
    .execute(&pool)
    .await;

    assert!(max_altitude_result.is_ok(), "Maximum altitude (9000m) should be accepted");

    // Test boundary pressure values
    let min_pressure_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, barometric_pressure_hpa)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(12)).unwrap(), 800.0
    )
    .execute(&pool)
    .await;

    assert!(min_pressure_result.is_ok(), "Minimum pressure (800 hPa) should be accepted");

    let max_pressure_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, barometric_pressure_hpa)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(13)).unwrap(), 1100.0
    )
    .execute(&pool)
    .await;

    assert!(max_pressure_result.is_ok(), "Maximum pressure (1100 hPa) should be accepted");

    // Test invalid values
    let invalid_altitude_low_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, altitude_meters)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(14)).unwrap(), -600.0  // < -500
    )
    .execute(&pool)
    .await;

    assert!(invalid_altitude_low_result.is_err(), "Altitude < -500m should be rejected");

    let invalid_altitude_high_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, altitude_meters)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(15)).unwrap(), 10000.0  // > 9000
    )
    .execute(&pool)
    .await;

    assert!(invalid_altitude_high_result.is_err(), "Altitude > 9000m should be rejected");

    let invalid_pressure_low_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, barometric_pressure_hpa)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(16)).unwrap(), 750.0  // < 800
    )
    .execute(&pool)
    .await;

    assert!(invalid_pressure_low_result.is_err(), "Pressure < 800 hPa should be rejected");

    let invalid_context_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, indoor_outdoor_context)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(17)).unwrap(), "invalid"
    )
    .execute(&pool)
    .await;

    assert!(invalid_context_result.is_err(), "Invalid indoor/outdoor context should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_aggregation_period_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test all valid aggregation periods
    let periods = ["event", "hourly", "daily"];
    for (i, period) in periods.iter().enumerate() {
        let result = sqlx::query!(
            "INSERT INTO environmental_metrics (user_id, recorded_at, aggregation_period, measurement_count)
             VALUES ($1, $2, $3, $4)",
            user_id, test_time.checked_add_signed(chrono::Duration::seconds(i as i64 + 1)).unwrap(), *period, 1
        )
        .execute(&pool)
        .await;

        assert!(result.is_ok(), "Aggregation period '{}' should be accepted", period);
    }

    // Test measurement count constraints
    let valid_count_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, measurement_count)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(10)).unwrap(), 1000
    )
    .execute(&pool)
    .await;

    assert!(valid_count_result.is_ok(), "Valid measurement count should be accepted");

    let max_count_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, measurement_count)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(11)).unwrap(), 10000
    )
    .execute(&pool)
    .await;

    assert!(max_count_result.is_ok(), "Maximum measurement count (10000) should be accepted");

    // Test invalid values
    let invalid_period_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, aggregation_period)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(12)).unwrap(), "invalid"
    )
    .execute(&pool)
    .await;

    assert!(invalid_period_result.is_err(), "Invalid aggregation period should be rejected");

    let invalid_count_zero_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, measurement_count)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(13)).unwrap(), 0
    )
    .execute(&pool)
    .await;

    assert!(invalid_count_zero_result.is_err(), "Measurement count 0 should be rejected");

    let invalid_count_high_result = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, measurement_count)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(14)).unwrap(), 15000  // > 10000
    )
    .execute(&pool)
    .await;

    assert!(invalid_count_high_result.is_err(), "Measurement count > 10000 should be rejected");

    Ok(())
}

#[sqlx::test]
async fn test_safety_events_table_creation(pool: PgPool) -> sqlx::Result<()> {
    // Verify safety_events table was created
    let table_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM information_schema.tables WHERE table_name = 'safety_events'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(table_exists.unwrap_or(false), "safety_events table should exist");

    // Test inserting a fall detection event
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Insert environmental metric with fall detection
    sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, fall_detected, fall_severity, impact_force_g, emergency_contacted)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time, true, "high", 20.5, true
    )
    .execute(&pool)
    .await?;

    // Check if safety event was automatically logged
    let safety_event_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND event_type = 'fall_detected'",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(safety_event_count.unwrap_or(0), 1, "Fall detection safety event should be logged");

    Ok(())
}

#[sqlx::test]
async fn test_safety_event_alerting_triggers(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test dangerous audio exposure logging (>85dB for >15 minutes)
    sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db, exposure_duration_seconds)
         VALUES ($1, $2, $3, $4)",
        user_id, test_time, 95.0, 1200  // 95dB for 20 minutes
    )
    .execute(&pool)
    .await?;

    let audio_event_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND event_type = 'dangerous_audio_exposure'",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(audio_event_count.unwrap_or(0), 1, "Dangerous audio exposure event should be logged");

    // Test extreme UV exposure logging (UV Index > 8, >30 minutes sun)
    sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index, time_in_sun_minutes, sunscreen_applied)
         VALUES ($1, $2, $3, $4, $5)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 10.5, 45, false
    )
    .execute(&pool)
    .await?;

    let uv_event_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND event_type = 'extreme_uv_exposure'",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(uv_event_count.unwrap_or(0), 1, "Extreme UV exposure event should be logged");

    // Test dangerous air quality logging (AQI > 200)
    sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, air_quality_index, pm2_5_micrograms_m3)
         VALUES ($1, $2, $3, $4)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(2)).unwrap(), 250, 150.5
    )
    .execute(&pool)
    .await?;

    let air_quality_event_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM safety_events WHERE user_id = $1 AND event_type = 'dangerous_air_quality'",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(air_quality_event_count.unwrap_or(0), 1, "Dangerous air quality event should be logged");

    Ok(())
}

#[sqlx::test]
async fn test_analytics_views_creation(pool: PgPool) -> sqlx::Result<()> {
    // Verify hourly aggregation view exists
    let hourly_view_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM information_schema.views WHERE table_name = 'environmental_metrics_hourly'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(hourly_view_exists.unwrap_or(false), "environmental_metrics_hourly view should exist");

    // Verify daily aggregation view exists
    let daily_view_exists = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM information_schema.views WHERE table_name = 'environmental_metrics_daily'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(daily_view_exists.unwrap_or(false), "environmental_metrics_daily view should exist");

    // Test querying the views with sample data
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Insert sample environmental data
    sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db, uv_index, handwashing_events, air_quality_index)
         VALUES ($1, $2, $3, $4, $5, $6)",
        user_id, test_time, 75.0, 6.5, 3, 85
    )
    .execute(&pool)
    .await?;

    // Test hourly view query
    let hourly_result = sqlx::query!(
        "SELECT * FROM environmental_metrics_hourly WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(hourly_result.len(), 1, "Should have one hourly aggregation result");

    // Test daily view query
    let daily_result = sqlx::query!(
        "SELECT * FROM environmental_metrics_daily WHERE user_id = $1",
        user_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(daily_result.len(), 1, "Should have one daily aggregation result");

    Ok(())
}

#[sqlx::test]
async fn test_performance_monitoring_function(pool: PgPool) -> sqlx::Result<()> {
    // Test performance monitoring function exists and works
    let performance_result = sqlx::query!(
        "SELECT * FROM monitor_environmental_metrics_performance()"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!performance_result.is_empty(), "Performance monitoring should return metrics");

    // Verify expected metric names
    let metric_names: Vec<String> = performance_result.iter()
        .map(|r| r.metric_name.clone().unwrap_or_default())
        .collect();

    let expected_metrics = [
        "total_records",
        "records_last_24h", 
        "fall_events_last_24h",
        "dangerous_audio_events_last_24h"
    ];

    for expected_metric in &expected_metrics {
        assert!(
            metric_names.contains(&expected_metric.to_string()),
            "Performance monitoring should include metric: {}",
            expected_metric
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_comprehensive_environmental_data_insertion(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Test inserting comprehensive environmental data (Apple Watch Series 8+ scenario)
    let result = sqlx::query!(
        "INSERT INTO environmental_metrics (
            user_id, recorded_at, 
            environmental_sound_level_db, headphone_exposure_db, noise_reduction_db, exposure_duration_seconds,
            uv_index, time_in_sun_minutes, time_in_shade_minutes, sunscreen_applied, uv_dose_joules_per_m2,
            fall_detected, fall_severity, impact_force_g, emergency_contacted, fall_response_time_seconds,
            handwashing_events, handwashing_duration_seconds, toothbrushing_events, toothbrushing_duration_seconds,
            pm2_5_micrograms_m3, pm10_micrograms_m3, air_quality_index, ozone_ppb, no2_ppb, so2_ppb, co_ppm,
            altitude_meters, barometric_pressure_hpa, indoor_outdoor_context,
            aggregation_period, measurement_count, source, device_type,
            raw_data
        ) VALUES (
            $1, $2,
            $3, $4, $5, $6,
            $7, $8, $9, $10, $11,
            $12, $13, $14, $15, $16,
            $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27,
            $28, $29, $30,
            $31, $32, $33, $34,
            $35
        )",
        user_id, test_time,
        // Audio exposure
        82.5, 78.0, 15.5, 1800,
        // UV exposure
        7.5, 60, 180, true, 3500.0,
        // Fall detection
        false, None::<String>, None::<f64>, false, None::<i32>,
        // Hygiene
        5, 30, 2, 150,
        // Air quality
        25.5, 45.0, 95, 35.2, 20.1, 10.5, 1.8,
        // Location
        350.5, 1015.25, "outdoor",
        // Aggregation
        "event", 1, "Apple Watch Series 8", "Apple Watch",
        // Raw data
        serde_json::json!({"device_model": "Watch8,4", "watchOS": "10.0"})
    )
    .execute(&pool)
    .await;

    assert!(result.is_ok(), "Comprehensive environmental data insertion should succeed");

    // Verify the data was inserted correctly
    let inserted_data = sqlx::query!(
        "SELECT * FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(inserted_data.environmental_sound_level_db, Some(rust_decimal::Decimal::from_f32_retain(82.5).unwrap()));
    assert_eq!(inserted_data.uv_index, Some(rust_decimal::Decimal::from_f32_retain(7.5).unwrap()));
    assert_eq!(inserted_data.handwashing_events, Some(5));
    assert_eq!(inserted_data.air_quality_index, Some(95));
    assert_eq!(inserted_data.indoor_outdoor_context, Some("outdoor".to_string()));

    Ok(())
}

#[sqlx::test]
async fn test_unique_constraint_and_primary_key(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Insert first record
    let first_insert = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db)
         VALUES ($1, $2, $3)",
        user_id, test_time, 70.0
    )
    .execute(&pool)
    .await;

    assert!(first_insert.is_ok(), "First insert should succeed");

    // Try to insert duplicate (same user_id, recorded_at)
    let duplicate_insert = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index)
         VALUES ($1, $2, $3)",
        user_id, test_time, 8.0
    )
    .execute(&pool)
    .await;

    assert!(duplicate_insert.is_err(), "Duplicate insert should fail due to unique constraint");

    // Insert with different timestamp should succeed
    let different_time_insert = sqlx::query!(
        "INSERT INTO environmental_metrics (user_id, recorded_at, uv_index)
         VALUES ($1, $2, $3)",
        user_id, test_time.checked_add_signed(chrono::Duration::seconds(1)).unwrap(), 8.0
    )
    .execute(&pool)
    .await;

    assert!(different_time_insert.is_ok(), "Insert with different timestamp should succeed");

    Ok(())
}

#[sqlx::test]
async fn test_insert_performance_benchmark(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let start_time = Utc::now();
    let batch_size = 100;

    let insert_start = Instant::now();

    // Insert batch of environmental metrics
    for i in 0..batch_size {
        let record_time = start_time.checked_add_signed(chrono::Duration::seconds(i)).unwrap();
        
        sqlx::query!(
            "INSERT INTO environmental_metrics (user_id, recorded_at, environmental_sound_level_db, uv_index, air_quality_index)
             VALUES ($1, $2, $3, $4, $5)",
            user_id, record_time, 75.0 + (i as f64 * 0.1), 6.0 + (i as f64 * 0.01), 85 + (i as i32 % 20)
        )
        .execute(&pool)
        .await?;
    }

    let insert_duration = insert_start.elapsed();

    // Performance target: Should be able to insert 100 records in under 5 seconds
    assert!(
        insert_duration.as_secs() < 5,
        "Batch insert of {} records took {:?}, should be under 5 seconds",
        batch_size, insert_duration
    );

    println!("Inserted {} environmental metrics in {:?}", batch_size, insert_duration);

    // Verify all records were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM environmental_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count.unwrap_or(0), batch_size as i64, "All records should be inserted");

    Ok(())
}