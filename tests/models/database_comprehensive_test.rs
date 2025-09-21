/*!
# Comprehensive Database Models Test Suite

This test suite provides complete coverage of all database models and operations:

1. **Health Metric Models**: HeartRate, BloodPressure, Sleep, Activity, Workout
2. **iOS Payload Models**: Complete iOS data conversion and validation
3. **Database Operations**: CRUD operations with SQLx compile-time verification
4. **Data Validation**: Constraint checking and integrity validation
5. **Partitioning**: Monthly partition management and queries
6. **Transaction Management**: Isolation and rollback testing
7. **PostGIS Operations**: Workout route storage and spatial queries
8. **Raw Ingestions**: Payload backup and recovery testing
9. **Audit Logging**: Complete audit trail verification

## Key Features

- Uses `sqlx::test` for real database testing with automatic migrations
- SQLx compile-time query verification ensures query safety
- Comprehensive constraint violation testing
- Duplicate detection and deduplication logic
- Data integrity checks across all models
- PostGIS spatial operations for workout routes
- Monthly partitioning validation
- Performance benchmarking for large datasets

## Database Schema Requirements

This test suite expects the full database schema to be available,
including all health metric tables, enums, indexes, and partitions.
*/

use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::*;
use self_sensored::models::health_metrics::*;
use self_sensored::models::ios_models::*;
use self_sensored::models::user_characteristics::*;

// Re-export test fixtures
use crate::common::fixtures::*;

/// Test database URL - uses TEST_DATABASE_URL environment variable
const TEST_DATABASE_URL: &str = "postgresql://postgres:password@localhost:5432/health_export_test";

/// Get test database URL from environment or use default
fn get_test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| TEST_DATABASE_URL.to_string())
}

// ============================================================================
// HEALTH METRIC CRUD TESTS WITH SQLx COMPILE-TIME VERIFICATION
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_heart_rate_crud_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert heart rate metric
    let heart_rate_id = Uuid::new_v4();
    let recorded_at = test_time - Duration::hours(1);

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, resting_heart_rate,
            heart_rate_variability, walking_heart_rate_average,
            heart_rate_recovery_one_minute, atrial_fibrillation_burden_percentage,
            vo2_max_ml_kg_min, source_device, context, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        heart_rate_id,
        user_id,
        recorded_at,
        72i16,
        60i16,
        45.5f64,
        120i16,
        25i16,
        Decimal::from_str_exact("1.5").unwrap(),
        Decimal::from_str_exact("45.2").unwrap(),
        "Apple Watch",
        ActivityContext::Resting as ActivityContext,
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query heart rate metric
    let heart_rate = sqlx::query_as!(
        HeartRateMetric,
        r#"
        SELECT
            id, user_id, recorded_at, heart_rate, resting_heart_rate,
            heart_rate_variability, walking_heart_rate_average,
            heart_rate_recovery_one_minute, atrial_fibrillation_burden_percentage,
            vo2_max_ml_kg_min, source_device,
            context as "context: ActivityContext", created_at
        FROM heart_rate_metrics
        WHERE id = $1
        "#,
        heart_rate_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(heart_rate.id, heart_rate_id);
    assert_eq!(heart_rate.user_id, user_id);
    assert_eq!(heart_rate.heart_rate, Some(72));
    assert_eq!(heart_rate.resting_heart_rate, Some(60));
    assert_eq!(heart_rate.heart_rate_variability, Some(45.5));
    assert_eq!(heart_rate.walking_heart_rate_average, Some(120));
    assert_eq!(heart_rate.heart_rate_recovery_one_minute, Some(25));
    assert!(heart_rate.atrial_fibrillation_burden_percentage.is_some());
    assert!(heart_rate.vo2_max_ml_kg_min.is_some());
    assert_eq!(heart_rate.source_device, Some("Apple Watch".to_string()));
    assert_eq!(heart_rate.context, Some(ActivityContext::Resting));

    // Test UPDATE: Modify heart rate metric
    let update_result = sqlx::query!(
        "UPDATE heart_rate_metrics SET heart_rate = $1, resting_heart_rate = $2 WHERE id = $3",
        75i16,
        65i16,
        heart_rate_id
    )
    .execute(&pool)
    .await?;

    assert_eq!(update_result.rows_affected(), 1);

    // Verify update
    let updated_heart_rate = sqlx::query!(
        "SELECT heart_rate, resting_heart_rate FROM heart_rate_metrics WHERE id = $1",
        heart_rate_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(updated_heart_rate.heart_rate, Some(75));
    assert_eq!(updated_heart_rate.resting_heart_rate, Some(65));

    // Test DELETE: Remove heart rate metric
    let delete_result = sqlx::query!(
        "DELETE FROM heart_rate_metrics WHERE id = $1",
        heart_rate_id
    )
    .execute(&pool)
    .await?;

    assert_eq!(delete_result.rows_affected(), 1);

    // Verify deletion
    let deleted_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE id = $1",
        heart_rate_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(deleted_count.count, Some(0));

    Ok(())
}

#[sqlx::test(migrations = "database/migrations")]
async fn test_blood_pressure_crud_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert blood pressure metric
    let bp_id = Uuid::new_v4();
    let recorded_at = test_time - Duration::hours(1);

    let insert_result = sqlx::query!(
        "INSERT INTO blood_pressure_metrics (id, user_id, recorded_at, systolic, diastolic, pulse, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        bp_id,
        user_id,
        recorded_at,
        120i16,
        80i16,
        70i16,
        "Blood Pressure Monitor",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query blood pressure metric
    let blood_pressure = sqlx::query_as!(
        BloodPressureMetric,
        "SELECT id, user_id, recorded_at, systolic, diastolic, pulse, source_device, created_at FROM blood_pressure_metrics WHERE id = $1",
        bp_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(blood_pressure.id, bp_id);
    assert_eq!(blood_pressure.user_id, user_id);
    assert_eq!(blood_pressure.systolic, 120);
    assert_eq!(blood_pressure.diastolic, 80);
    assert_eq!(blood_pressure.pulse, Some(70));
    assert_eq!(blood_pressure.source_device, Some("Blood Pressure Monitor".to_string()));

    // Test constraint validation - systolic must be > diastolic
    let invalid_bp_result = sqlx::query!(
        "INSERT INTO blood_pressure_metrics (id, user_id, recorded_at, systolic, diastolic, pulse, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        Uuid::new_v4(),
        user_id,
        recorded_at,
        80i16,  // systolic lower than diastolic
        120i16, // diastolic higher than systolic
        70i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to check constraint
    assert!(invalid_bp_result.is_err());

    Ok(())
}

#[sqlx::test(migrations = "database/migrations")]
async fn test_sleep_metric_crud_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert sleep metric
    let sleep_id = Uuid::new_v4();
    let sleep_start = test_time - Duration::hours(8);
    let sleep_end = test_time;

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO sleep_metrics (
            id, user_id, sleep_start, sleep_end, duration_minutes,
            deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes,
            awake_minutes, efficiency, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        sleep_id,
        user_id,
        sleep_start,
        sleep_end,
        480i32, // 8 hours
        120i32, // 2 hours deep
        100i32, // 1.67 hours REM
        240i32, // 4 hours light
        20i32,  // 20 minutes awake
        95.8f64, // 95.8% efficiency
        "Apple Watch",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query sleep metric
    let sleep_metric = sqlx::query_as!(
        SleepMetric,
        r#"
        SELECT
            id, user_id, sleep_start, sleep_end, duration_minutes,
            deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes,
            awake_minutes, efficiency, source_device, created_at
        FROM sleep_metrics
        WHERE id = $1
        "#,
        sleep_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(sleep_metric.id, sleep_id);
    assert_eq!(sleep_metric.user_id, user_id);
    assert_eq!(sleep_metric.duration_minutes, Some(480));
    assert_eq!(sleep_metric.deep_sleep_minutes, Some(120));
    assert_eq!(sleep_metric.rem_sleep_minutes, Some(100));
    assert_eq!(sleep_metric.light_sleep_minutes, Some(240));
    assert_eq!(sleep_metric.awake_minutes, Some(20));
    assert_eq!(sleep_metric.efficiency, Some(95.8));
    assert_eq!(sleep_metric.source_device, Some("Apple Watch".to_string()));

    // Test constraint validation - efficiency must be between 0 and 100
    let invalid_efficiency_result = sqlx::query!(
        r#"
        INSERT INTO sleep_metrics (
            id, user_id, sleep_start, sleep_end, duration_minutes,
            deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes,
            awake_minutes, efficiency, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        Uuid::new_v4(),
        user_id,
        sleep_start,
        sleep_end,
        480i32,
        120i32,
        100i32,
        240i32,
        20i32,
        150.0f64, // Invalid - over 100%
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to check constraint
    assert!(invalid_efficiency_result.is_err());

    Ok(())
}

#[sqlx::test(migrations = "database/migrations")]
async fn test_activity_metric_crud_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert comprehensive activity metric
    let activity_id = Uuid::new_v4();
    let recorded_at = test_time - Duration::hours(1);

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO activity_metrics (
            id, user_id, recorded_at, step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal, distance_cycling_meters,
            distance_swimming_meters, distance_wheelchair_meters, distance_downhill_snow_sports_meters,
            push_count, swimming_stroke_count, nike_fuel_points, apple_exercise_time_minutes,
            apple_stand_time_minutes, apple_move_time_minutes, apple_stand_hour_achieved,
            walking_speed_m_per_s, walking_step_length_cm, walking_asymmetry_percent,
            walking_double_support_percent, six_minute_walk_test_distance_m, stair_ascent_speed_m_per_s,
            stair_descent_speed_m_per_s, ground_contact_time_ms, vertical_oscillation_cm,
            running_stride_length_m, running_power_watts, running_speed_m_per_s,
            cycling_speed_kmh, cycling_power_watts, cycling_cadence_rpm, functional_threshold_power_watts,
            underwater_depth_meters, diving_duration_seconds, source_device, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39
        )
        "#,
        activity_id, user_id, recorded_at,
        10000i32, // step_count
        8000.0f64, // distance_meters
        15i32, // flights_climbed
        400.0f64, // active_energy_burned_kcal
        1600.0f64, // basal_energy_burned_kcal
        5000.0f64, // distance_cycling_meters
        1000.0f64, // distance_swimming_meters
        0.0f64, // distance_wheelchair_meters
        0.0f64, // distance_downhill_snow_sports_meters
        0i32, // push_count
        500i32, // swimming_stroke_count
        2500i32, // nike_fuel_points
        30i32, // apple_exercise_time_minutes
        12i32, // apple_stand_time_minutes
        45i32, // apple_move_time_minutes
        true, // apple_stand_hour_achieved
        1.4f64, // walking_speed_m_per_s
        75.0f64, // walking_step_length_cm
        2.5f64, // walking_asymmetry_percent
        20.0f64, // walking_double_support_percent
        500.0f64, // six_minute_walk_test_distance_m
        0.5f64, // stair_ascent_speed_m_per_s
        0.6f64, // stair_descent_speed_m_per_s
        250.0f64, // ground_contact_time_ms
        8.0f64, // vertical_oscillation_cm
        1.2f64, // running_stride_length_m
        250.0f64, // running_power_watts
        3.5f64, // running_speed_m_per_s
        25.0f64, // cycling_speed_kmh
        200.0f64, // cycling_power_watts
        85.0f64, // cycling_cadence_rpm
        250.0f64, // functional_threshold_power_watts
        5.0f64, // underwater_depth_meters
        60i32, // diving_duration_seconds
        "Apple Watch", // source_device
        test_time // created_at
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query activity metric with all fields
    let activity_metric = sqlx::query_as!(
        ActivityMetric,
        r#"
        SELECT
            id, user_id, recorded_at, step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal, distance_cycling_meters,
            distance_swimming_meters, distance_wheelchair_meters, distance_downhill_snow_sports_meters,
            push_count, swimming_stroke_count, nike_fuel_points, apple_exercise_time_minutes,
            apple_stand_time_minutes, apple_move_time_minutes, apple_stand_hour_achieved,
            walking_speed_m_per_s, walking_step_length_cm, walking_asymmetry_percent,
            walking_double_support_percent, six_minute_walk_test_distance_m, stair_ascent_speed_m_per_s,
            stair_descent_speed_m_per_s, ground_contact_time_ms, vertical_oscillation_cm,
            running_stride_length_m, running_power_watts, running_speed_m_per_s,
            cycling_speed_kmh, cycling_power_watts, cycling_cadence_rpm, functional_threshold_power_watts,
            underwater_depth_meters, diving_duration_seconds, source_device, created_at
        FROM activity_metrics
        WHERE id = $1
        "#,
        activity_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(activity_metric.id, activity_id);
    assert_eq!(activity_metric.user_id, user_id);
    assert_eq!(activity_metric.step_count, Some(10000));
    assert_eq!(activity_metric.distance_meters, Some(8000.0));
    assert_eq!(activity_metric.flights_climbed, Some(15));
    assert_eq!(activity_metric.active_energy_burned_kcal, Some(400.0));
    assert_eq!(activity_metric.basal_energy_burned_kcal, Some(1600.0));
    assert_eq!(activity_metric.cycling_speed_kmh, Some(25.0));
    assert_eq!(activity_metric.underwater_depth_meters, Some(5.0));
    assert_eq!(activity_metric.source_device, Some("Apple Watch".to_string()));

    // Test constraint validation - step count must be non-negative
    let invalid_step_count_result = sqlx::query!(
        "INSERT INTO activity_metrics (id, user_id, recorded_at, step_count, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(),
        user_id,
        recorded_at,
        -1000i32, // Invalid negative step count
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to check constraint
    assert!(invalid_step_count_result.is_err());

    Ok(())
}

// ============================================================================
// WORKOUT DATA WITH POSTGIS TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_workout_crud_with_postgis(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert workout with GPS route using PostGIS
    let workout_id = Uuid::new_v4();
    let started_at = test_time - Duration::hours(1);
    let ended_at = test_time;

    // Create a sample GPS route (San Francisco area)
    let route_points = vec![
        (37.7749, -122.4194),  // San Francisco
        (37.7849, -122.4094),  // Moving northeast
        (37.7949, -122.3994),  // Continue northeast
        (37.7849, -122.4094),  // Return
        (37.7749, -122.4194),  // Back to start
    ];

    // Convert route points to PostGIS LINESTRING
    let linestring = format!(
        "LINESTRING({})",
        route_points
            .iter()
            .map(|(lat, lon)| format!("{} {}", lon, lat))
            .collect::<Vec<_>>()
            .join(",")
    );

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO workouts (
            id, user_id, workout_type, started_at, ended_at,
            total_energy_kcal, active_energy_kcal, distance_meters,
            avg_heart_rate, max_heart_rate, route, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, ST_GeomFromText($11, 4326), $12, $13)
        "#,
        workout_id,
        user_id,
        WorkoutType::Running as WorkoutType,
        started_at,
        ended_at,
        500.0f64,
        450.0f64,
        5000.0f64,
        145i16,
        175i16,
        linestring,
        "Apple Watch",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query workout with PostGIS route extraction
    let workout_row = sqlx::query!(
        r#"
        SELECT
            id, user_id, workout_type as "workout_type: WorkoutType",
            started_at, ended_at, total_energy_kcal, active_energy_kcal,
            distance_meters, avg_heart_rate, max_heart_rate,
            ST_AsText(route) as route_text, source_device, created_at
        FROM workouts
        WHERE id = $1
        "#,
        workout_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(workout_row.id, workout_id);
    assert_eq!(workout_row.user_id, user_id);
    assert_eq!(workout_row.workout_type, WorkoutType::Running);
    assert_eq!(workout_row.total_energy_kcal, Some(500.0));
    assert_eq!(workout_row.active_energy_kcal, Some(450.0));
    assert_eq!(workout_row.distance_meters, Some(5000.0));
    assert_eq!(workout_row.avg_heart_rate, Some(145));
    assert_eq!(workout_row.max_heart_rate, Some(175));
    assert!(workout_row.route_text.is_some());
    assert!(workout_row.route_text.unwrap().contains("LINESTRING"));

    // Test PostGIS spatial queries - find workouts within area
    let bounding_box_query = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM workouts
        WHERE route && ST_MakeEnvelope(-122.5, 37.7, -122.3, 37.8, 4326)
        AND user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(bounding_box_query.count, Some(1));

    // Test route distance calculation using PostGIS
    let route_distance_query = sqlx::query!(
        r#"
        SELECT ST_Length(ST_Transform(route, 3857)) as route_length_meters
        FROM workouts
        WHERE id = $1
        "#,
        workout_id
    )
    .fetch_one(&pool)
    .await?;

    // Route should have some length
    assert!(route_distance_query.route_length_meters.unwrap() > 0.0);

    Ok(())
}

// ============================================================================
// RAW INGESTIONS TABLE TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_raw_ingestions_crud_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert raw ingestion data
    let ingestion_id = Uuid::new_v4();
    let payload_hash = "abc123def456";
    let raw_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "HeartRate",
                    "units": "BPM",
                    "data": [
                        {
                            "qty": 75.0,
                            "date": "2024-01-01T12:00:00Z",
                            "source": "Apple Watch"
                        }
                    ]
                }
            ]
        }
    });

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (
            id, user_id, payload_hash, raw_payload,
            processing_status, processing_errors,
            metrics_count, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        ingestion_id,
        user_id,
        payload_hash,
        raw_payload,
        "pending",
        json!({}),
        1i32,
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query raw ingestion
    let raw_ingestion = sqlx::query!(
        r#"
        SELECT
            id, user_id, payload_hash, raw_payload,
            processing_status, processing_errors,
            metrics_count, created_at, processed_at
        FROM raw_ingestions
        WHERE id = $1
        "#,
        ingestion_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(raw_ingestion.id, ingestion_id);
    assert_eq!(raw_ingestion.user_id, user_id);
    assert_eq!(raw_ingestion.payload_hash, payload_hash);
    assert_eq!(raw_ingestion.processing_status, "pending");
    assert_eq!(raw_ingestion.metrics_count, 1);
    assert!(raw_ingestion.processed_at.is_none());

    // Test UPDATE: Mark as processed with errors
    let processing_errors = json!({
        "errors": [
            {
                "metric_type": "HeartRate",
                "error": "Validation failed: heart rate out of range",
                "value": 500
            }
        ]
    });

    let update_result = sqlx::query!(
        r#"
        UPDATE raw_ingestions
        SET processing_status = $1, processing_errors = $2, processed_at = $3
        WHERE id = $4
        "#,
        "failed",
        processing_errors,
        test_time,
        ingestion_id
    )
    .execute(&pool)
    .await?;

    assert_eq!(update_result.rows_affected(), 1);

    // Test duplicate detection by payload hash
    let duplicate_attempt = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (
            id, user_id, payload_hash, raw_payload,
            processing_status, processing_errors,
            metrics_count, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        Uuid::new_v4(),
        user_id,
        payload_hash, // Same hash - should be unique per user
        raw_payload,
        "pending",
        json!({}),
        1i32,
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to unique constraint on (user_id, payload_hash)
    assert!(duplicate_attempt.is_err());

    Ok(())
}

// ============================================================================
// AUDIT LOGGING TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_audit_logging_operations(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test CREATE: Insert audit log entry
    let audit_id = Uuid::new_v4();
    let table_name = "heart_rate_metrics";
    let action_type = "INSERT";
    let record_id = Uuid::new_v4();

    let old_values = json!({});
    let new_values = json!({
        "heart_rate": 75,
        "resting_heart_rate": 60,
        "recorded_at": test_time.to_rfc3339()
    });

    let insert_result = sqlx::query!(
        r#"
        INSERT INTO audit_log (
            id, user_id, table_name, action_type, record_id,
            old_values, new_values, ip_address, user_agent,
            api_key_id, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        audit_id,
        user_id,
        table_name,
        action_type,
        record_id,
        old_values,
        new_values,
        "192.168.1.100",
        "Health Export iOS App/1.0",
        Uuid::new_v4(),
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(insert_result.rows_affected(), 1);

    // Test READ: Query audit log with filtering
    let audit_entries = sqlx::query!(
        r#"
        SELECT
            id, user_id, table_name, action_type, record_id,
            old_values, new_values, ip_address, user_agent,
            api_key_id, created_at
        FROM audit_log
        WHERE user_id = $1 AND table_name = $2
        ORDER BY created_at DESC
        "#,
        user_id,
        table_name
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(audit_entries.len(), 1);
    let audit_entry = &audit_entries[0];
    assert_eq!(audit_entry.user_id, user_id);
    assert_eq!(audit_entry.table_name, table_name);
    assert_eq!(audit_entry.action_type, action_type);
    assert_eq!(audit_entry.record_id, record_id);
    assert_eq!(audit_entry.ip_address, Some("192.168.1.100".to_string()));

    // Test audit trail completeness for a record lifecycle
    let heart_rate_id = Uuid::new_v4();

    // INSERT audit
    sqlx::query!(
        r#"
        INSERT INTO audit_log (
            id, user_id, table_name, action_type, record_id,
            old_values, new_values, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        Uuid::new_v4(),
        user_id,
        "heart_rate_metrics",
        "INSERT",
        heart_rate_id,
        json!({}),
        json!({"heart_rate": 72}),
        test_time
    )
    .execute(&pool)
    .await?;

    // UPDATE audit
    sqlx::query!(
        r#"
        INSERT INTO audit_log (
            id, user_id, table_name, action_type, record_id,
            old_values, new_values, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        Uuid::new_v4(),
        user_id,
        "heart_rate_metrics",
        "UPDATE",
        heart_rate_id,
        json!({"heart_rate": 72}),
        json!({"heart_rate": 75}),
        test_time + Duration::minutes(5)
    )
    .execute(&pool)
    .await?;

    // DELETE audit
    sqlx::query!(
        r#"
        INSERT INTO audit_log (
            id, user_id, table_name, action_type, record_id,
            old_values, new_values, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        Uuid::new_v4(),
        user_id,
        "heart_rate_metrics",
        "DELETE",
        heart_rate_id,
        json!({"heart_rate": 75}),
        json!({}),
        test_time + Duration::minutes(10)
    )
    .execute(&pool)
    .await?;

    // Verify complete audit trail
    let complete_trail = sqlx::query!(
        r#"
        SELECT action_type, old_values, new_values
        FROM audit_log
        WHERE user_id = $1 AND record_id = $2
        ORDER BY created_at
        "#,
        user_id,
        heart_rate_id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(complete_trail.len(), 3);
    assert_eq!(complete_trail[0].action_type, "INSERT");
    assert_eq!(complete_trail[1].action_type, "UPDATE");
    assert_eq!(complete_trail[2].action_type, "DELETE");

    Ok(())
}

// ============================================================================
// TRANSACTION MANAGEMENT TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_transaction_isolation_and_rollback(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test successful transaction
    let mut tx = pool.begin().await?;

    let heart_rate_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        heart_rate_id,
        user_id,
        test_time,
        75i16,
        "Test Device",
        test_time
    )
    .execute(&mut *tx)
    .await?;

    let bp_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO blood_pressure_metrics (
            id, user_id, recorded_at, systolic, diastolic, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        bp_id,
        user_id,
        test_time,
        120i16,
        80i16,
        "Test Device",
        test_time
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Verify both records exist
    let heart_rate_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE id = $1",
        heart_rate_id
    )
    .fetch_one(&pool)
    .await?;

    let bp_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM blood_pressure_metrics WHERE id = $1",
        bp_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(heart_rate_count.count, Some(1));
    assert_eq!(bp_count.count, Some(1));

    // Test transaction rollback on failure
    let mut tx = pool.begin().await?;

    let heart_rate_id_2 = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        heart_rate_id_2,
        user_id,
        test_time,
        85i16,
        "Test Device",
        test_time
    )
    .execute(&mut *tx)
    .await?;

    // This should fail due to constraint violation
    let invalid_bp_result = sqlx::query!(
        r#"
        INSERT INTO blood_pressure_metrics (
            id, user_id, recorded_at, systolic, diastolic, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        80i16,  // systolic lower than diastolic - should fail
        120i16, // diastolic higher than systolic
        "Test Device",
        test_time
    )
    .execute(&mut *tx)
    .await;

    assert!(invalid_bp_result.is_err());

    // Rollback the transaction
    tx.rollback().await?;

    // Verify heart rate was not inserted due to rollback
    let heart_rate_count_2 = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE id = $1",
        heart_rate_id_2
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(heart_rate_count_2.count, Some(0));

    Ok(())
}

// ============================================================================
// MONTHLY PARTITIONING TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_monthly_partitioning_logic(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test current month partition
    let current_month = test_time;
    let heart_rate_id_current = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        heart_rate_id_current,
        user_id,
        current_month,
        75i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test previous month partition
    let previous_month = test_time - Duration::days(45); // Ensure it's in previous month
    let heart_rate_id_previous = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        heart_rate_id_previous,
        user_id,
        previous_month,
        80i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await?;

    // Query current month data (should use current partition)
    let current_month_data = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1
        AND recorded_at >= date_trunc('month', $2::timestamptz)
        AND recorded_at < date_trunc('month', $2::timestamptz) + interval '1 month'
        "#,
        user_id,
        current_month
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(current_month_data.count, Some(1));

    // Query previous month data (should use previous partition)
    let previous_month_data = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1
        AND recorded_at >= date_trunc('month', $2::timestamptz)
        AND recorded_at < date_trunc('month', $2::timestamptz) + interval '1 month'
        "#,
        user_id,
        previous_month
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(previous_month_data.count, Some(1));

    // Query across multiple months
    let cross_month_data = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1
        AND recorded_at >= $2
        AND recorded_at <= $3
        "#,
        user_id,
        previous_month,
        current_month
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(cross_month_data.count, Some(2));

    Ok(())
}

// ============================================================================
// DATA VALIDATION AND CONSTRAINTS TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_data_validation_constraints(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test heart rate validation constraints

    // Valid heart rate should work
    let valid_hr_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        75i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(valid_hr_result.is_ok());

    // Invalid heart rate (too high) should fail
    let invalid_hr_high = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        350i16, // Too high
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(invalid_hr_high.is_err());

    // Invalid heart rate (too low) should fail
    let invalid_hr_low = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        10i16, // Too low
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(invalid_hr_low.is_err());

    // Test blood pressure constraints

    // Valid blood pressure should work
    let valid_bp_result = sqlx::query!(
        r#"
        INSERT INTO blood_pressure_metrics (
            id, user_id, recorded_at, systolic, diastolic, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        120i16,
        80i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(valid_bp_result.is_ok());

    // Invalid blood pressure (systolic too high) should fail
    let invalid_bp_systolic = sqlx::query!(
        r#"
        INSERT INTO blood_pressure_metrics (
            id, user_id, recorded_at, systolic, diastolic, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        Uuid::new_v4(),
        user_id,
        test_time,
        300i16, // Too high
        80i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(invalid_bp_systolic.is_err());

    // Test foreign key constraints

    // Insert with non-existent user should fail
    let invalid_user_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        Uuid::new_v4(), // Non-existent user
        test_time,
        75i16,
        "Test Device",
        test_time
    )
    .execute(&pool)
    .await;

    assert!(invalid_user_result.is_err());

    Ok(())
}

// ============================================================================
// DUPLICATE DETECTION TESTING
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_duplicate_detection_logic(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    let recorded_at = test_time - Duration::hours(1);

    // Insert first heart rate metric
    let first_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        recorded_at,
        75i16,
        "Apple Watch",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(first_result.rows_affected(), 1);

    // Try to insert duplicate (same user_id + recorded_at + heart_rate)
    let duplicate_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        recorded_at,
        75i16, // Same heart rate
        "Apple Watch",
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to unique constraint
    assert!(duplicate_result.is_err());

    // But different heart rate at same time should be allowed
    let different_value_result = sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (
            id, user_id, recorded_at, heart_rate, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        recorded_at,
        80i16, // Different heart rate
        "Apple Watch",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(different_value_result.rows_affected(), 1);

    // Test duplicate detection for activity metrics
    let activity_recorded_at = test_time - Duration::hours(2);

    // Insert first activity metric
    let first_activity_result = sqlx::query!(
        r#"
        INSERT INTO activity_metrics (
            id, user_id, recorded_at, step_count, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        activity_recorded_at,
        10000i32,
        "iPhone",
        test_time
    )
    .execute(&pool)
    .await?;

    assert_eq!(first_activity_result.rows_affected(), 1);

    // Try to insert duplicate activity metric
    let duplicate_activity_result = sqlx::query!(
        r#"
        INSERT INTO activity_metrics (
            id, user_id, recorded_at, step_count, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        user_id,
        activity_recorded_at,
        10000i32, // Same step count at same time
        "iPhone",
        test_time
    )
    .execute(&pool)
    .await;

    // This should fail due to unique constraint
    assert!(duplicate_activity_result.is_err());

    Ok(())
}

// ============================================================================
// IOS MODELS VALIDATION TESTING
// ============================================================================

#[test]
fn test_ios_models_serialization_deserialization() {
    // Test IosIngestPayload deserialization
    let payload_json = json!({
        "data": {
            "metrics": [
                {
                    "name": "HeartRate",
                    "units": "BPM",
                    "data": [
                        {
                            "qty": 75.0,
                            "date": "2024-01-01T12:00:00Z",
                            "source": "Apple Watch"
                        }
                    ]
                }
            ],
            "workouts": [
                {
                    "type": "Running",
                    "startDate": "2024-01-01T10:00:00Z",
                    "endDate": "2024-01-01T11:00:00Z",
                    "totalEnergyBurned": 500.0,
                    "totalDistance": 5000.0
                }
            ]
        }
    });

    let payload: Result<IosIngestPayload, _> = serde_json::from_value(payload_json);
    assert!(payload.is_ok());

    let payload = payload.unwrap();
    assert_eq!(payload.data.metrics.len(), 1);
    assert_eq!(payload.data.workouts.len(), 1);

    let metric = &payload.data.metrics[0];
    assert_eq!(metric.name, "HeartRate");
    assert_eq!(metric.units, Some("BPM".to_string()));
    assert_eq!(metric.data.len(), 1);

    let data_point = &metric.data[0];
    assert_eq!(data_point.qty, Some(75.0));
    assert_eq!(data_point.source, Some("Apple Watch".to_string()));
}

#[test]
fn test_ios_models_validation_errors() {
    // Test invalid JSON structure
    let invalid_json = json!({
        "invalid": "structure"
    });

    let result: Result<IosIngestPayload, _> = serde_json::from_value(invalid_json);
    assert!(result.is_err());

    // Test missing required fields
    let missing_data = json!({
        "data": {}
    });

    let result: Result<IosIngestPayload, _> = serde_json::from_value(missing_data);
    assert!(result.is_err());
}

// ============================================================================
// PERFORMANCE AND SCALING TESTS
// ============================================================================

#[sqlx::test(migrations = "database/migrations")]
async fn test_large_batch_insert_performance(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Test batch insert of 1000 heart rate metrics
    let start_time = std::time::Instant::now();
    let batch_size = 1000;

    let mut tx = pool.begin().await?;

    for i in 0..batch_size {
        let metric_time = test_time - Duration::minutes(i as i64);
        sqlx::query!(
            r#"
            INSERT INTO heart_rate_metrics (
                id, user_id, recorded_at, heart_rate, source_device, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            Uuid::new_v4(),
            user_id,
            metric_time,
            (60 + (i % 40)) as i16, // Heart rate between 60-100
            "Test Device",
            test_time
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let duration = start_time.elapsed();
    println!("Inserted {} records in {:?}", batch_size, duration);

    // Verify all records were inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count.count, Some(batch_size as i64));

    // Performance should be reasonable (less than 10 seconds for 1000 records)
    assert!(duration.as_secs() < 10);

    Ok(())
}

#[sqlx::test(migrations = "database/migrations")]
async fn test_index_performance_with_brin(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    let test_time = Utc::now();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, api_key_hash, created_at) VALUES ($1, $2, $3, $4)",
        user_id,
        "test@example.com",
        "test_hash",
        test_time
    )
    .execute(&pool)
    .await?;

    // Insert test data across multiple months
    let mut tx = pool.begin().await?;

    for month_offset in 0..6 {
        for day in 0..30 {
            let metric_time = test_time - Duration::days(month_offset * 30 + day);
            sqlx::query!(
                r#"
                INSERT INTO heart_rate_metrics (
                    id, user_id, recorded_at, heart_rate, source_device, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                Uuid::new_v4(),
                user_id,
                metric_time,
                75i16,
                "Test Device",
                test_time
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // Test time-range query performance (should use BRIN index)
    let start_time = std::time::Instant::now();

    let recent_metrics = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM heart_rate_metrics
        WHERE user_id = $1
        AND recorded_at >= $2
        AND recorded_at <= $3
        "#,
        user_id,
        test_time - Duration::days(7),
        test_time
    )
    .fetch_one(&pool)
    .await?;

    let query_duration = start_time.elapsed();

    assert!(recent_metrics.count.unwrap() > 0);

    // Query should be fast (sub-100ms for time-range queries)
    assert!(query_duration.as_millis() < 100);

    Ok(())
}