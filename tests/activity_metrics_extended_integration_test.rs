//! Integration tests for extended activity metrics functionality
//!
//! Tests comprehensive activity tracking with specialized metrics for:
//! - Cycling, swimming, wheelchair, and snow sports distance tracking
//! - Wheelchair accessibility (push count and specialized distance)
//! - Swimming analytics (stroke count)
//! - Apple Watch activity ring integration
//! - Nike Fuel points cross-platform compatibility
//! - Multi-sport activity scenarios

use chrono::{Duration, Utc};
use self_sensored::{
    config::ValidationConfig,
    models::{
        enums::{ActivityMoveMode, BiologicalSex, BloodType, FitzpatrickSkinType},
        health_metrics::*,
        ios_models::*,
        user_characteristics::UserCharacteristics,
    },
    services::batch_processor::BatchProcessor,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

mod common;

/// Helper function to create test database pool
async fn create_test_pool() -> PgPool {
    common::setup_test_db().await
}

/// Helper function to create a complete ActivityMetric with all required fields
fn create_base_activity_metric(user_id: Uuid) -> ActivityMetric {
    ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: Utc::now(),
        step_count: Some(0),
        distance_meters: Some(0.0),
        active_energy_burned_kcal: Some(0.0),
        basal_energy_burned_kcal: Some(0.0),
        flights_climbed: Some(0),
        distance_cycling_meters: Some(0.0),
        distance_swimming_meters: Some(0.0),
        distance_wheelchair_meters: Some(0.0),
        distance_downhill_snow_sports_meters: Some(0.0),
        push_count: Some(0),
        swimming_stroke_count: Some(0),
        nike_fuel_points: Some(0),
        apple_exercise_time_minutes: Some(0),
        apple_stand_time_minutes: Some(0),
        apple_move_time_minutes: Some(0),
        apple_stand_hour_achieved: Some(false),
        // Mobility Metrics
        walking_speed_m_per_s: None,
        walking_step_length_cm: None,
        walking_asymmetry_percent: None,
        walking_double_support_percent: None,
        six_minute_walk_test_distance_m: None,
        stair_ascent_speed_m_per_s: None,
        stair_descent_speed_m_per_s: None,
        // Running Dynamics
        ground_contact_time_ms: None,
        vertical_oscillation_cm: None,
        running_stride_length_m: None,
        running_power_watts: None,
        running_speed_m_per_s: None,
        // Cycling Metrics
        cycling_speed_kmh: None,
        cycling_power_watts: None,
        cycling_cadence_rpm: None,
        functional_threshold_power_watts: None,
        // Underwater Metrics
        underwater_depth_meters: None,
        diving_duration_seconds: None,
        source_device: Some("Test Device".to_string()),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_extended_activity_metrics_ingestion() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_test_pool().await;
    let user = create_test_user(&pool).await?;
    let batch_processor = BatchProcessor::new(pool.clone());

    // Create comprehensive activity metrics with all new specialized fields
    let mut cycling_activity = create_base_activity_metric(user.id);
    cycling_activity.recorded_at = Utc::now() - Duration::hours(2);
    cycling_activity.step_count = Some(0);            // No steps during cycling
    cycling_activity.distance_meters = Some(15000.0); // 15km total
    cycling_activity.flights_climbed = Some(0);
    cycling_activity.active_energy_burned_kcal = Some(450.0);
    cycling_activity.basal_energy_burned_kcal = Some(120.0);
    // Specialized fields
    cycling_activity.distance_cycling_meters = Some(15000.0); // Cycling-specific distance
    cycling_activity.distance_swimming_meters = None;
    cycling_activity.distance_wheelchair_meters = None;
    cycling_activity.distance_downhill_snow_sports_meters = None;
    cycling_activity.push_count = None;
    cycling_activity.swimming_stroke_count = None;
    cycling_activity.nike_fuel_points = Some(750);           // Nike+ integration
    cycling_activity.apple_exercise_time_minutes = Some(45); // Apple Watch exercise ring
    cycling_activity.apple_stand_time_minutes = Some(8);     // Achieved stand goal 8 hours
    cycling_activity.apple_move_time_minutes = Some(45);
    cycling_activity.apple_stand_hour_achieved = Some(true);
    cycling_activity.source_device = Some("Apple Watch Series 9".to_string());
    // Extended cycling metrics
    cycling_activity.cycling_speed_kmh = Some(20.0);
    cycling_activity.cycling_cadence_rpm = Some(85.0);
    cycling_activity.cycling_power_watts = Some(200.0);

    let mut swimming_activity = create_base_activity_metric(user.id);
    swimming_activity.recorded_at = Utc::now() - Duration::hours(1);
    swimming_activity.step_count = Some(0);           // No steps during swimming
    swimming_activity.distance_meters = Some(2000.0); // 2km pool swimming
    swimming_activity.flights_climbed = Some(0);
    swimming_activity.active_energy_burned_kcal = Some(380.0);
    swimming_activity.basal_energy_burned_kcal = Some(90.0);
    // Specialized fields
    swimming_activity.distance_cycling_meters = None;
    swimming_activity.distance_swimming_meters = Some(2000.0); // Swimming-specific distance
    swimming_activity.distance_wheelchair_meters = None;
    swimming_activity.distance_downhill_snow_sports_meters = None;
    swimming_activity.push_count = None;
    swimming_activity.swimming_stroke_count = Some(1800); // Stroke count analytics
    swimming_activity.nike_fuel_points = None;
    swimming_activity.apple_exercise_time_minutes = Some(30);
    swimming_activity.apple_stand_time_minutes = None;
    swimming_activity.apple_move_time_minutes = Some(30);
    swimming_activity.apple_stand_hour_achieved = Some(false);
    swimming_activity.source_device = Some("Apple Watch Ultra 2".to_string());

    let mut wheelchair_activity = create_base_activity_metric(user.id);
    wheelchair_activity.recorded_at = Utc::now() - Duration::minutes(30);
    wheelchair_activity.step_count = Some(0);           // No steps for wheelchair user
    wheelchair_activity.distance_meters = Some(8000.0); // 8km wheelchair distance
    wheelchair_activity.flights_climbed = Some(0);      // No flights for wheelchair
    wheelchair_activity.active_energy_burned_kcal = Some(320.0);
    wheelchair_activity.basal_energy_burned_kcal = Some(110.0);
    // Specialized fields for accessibility
    wheelchair_activity.distance_cycling_meters = None;
    wheelchair_activity.distance_swimming_meters = None;
    wheelchair_activity.distance_wheelchair_meters = Some(8000.0); // Wheelchair-specific distance
    wheelchair_activity.distance_downhill_snow_sports_meters = None;
    wheelchair_activity.push_count = Some(2400); // Wheelchair push count
    wheelchair_activity.swimming_stroke_count = None;
    wheelchair_activity.nike_fuel_points = Some(420);
    wheelchair_activity.apple_exercise_time_minutes = Some(35);
    wheelchair_activity.apple_stand_time_minutes = None; // Not applicable for wheelchair users
    wheelchair_activity.apple_move_time_minutes = Some(35);
    wheelchair_activity.apple_stand_hour_achieved = Some(false);
    wheelchair_activity.source_device = Some("Apple Watch Series 9".to_string());

    let mut snow_sports_activity = create_base_activity_metric(user.id);
    snow_sports_activity.recorded_at = Utc::now() - Duration::minutes(15);
    snow_sports_activity.step_count = Some(1200);         // Some steps during downhill skiing
    snow_sports_activity.distance_meters = Some(25000.0); // 25km downhill skiing
    snow_sports_activity.flights_climbed = Some(0);       // Downhill only
    snow_sports_activity.active_energy_burned_kcal = Some(680.0);
    snow_sports_activity.basal_energy_burned_kcal = Some(150.0);
    // Specialized fields
    snow_sports_activity.distance_cycling_meters = None;
    snow_sports_activity.distance_swimming_meters = None;
    snow_sports_activity.distance_wheelchair_meters = None;
    snow_sports_activity.distance_downhill_snow_sports_meters = Some(25000.0); // Snow sports distance
    snow_sports_activity.push_count = None;
    snow_sports_activity.swimming_stroke_count = None;
    snow_sports_activity.nike_fuel_points = None;
    snow_sports_activity.apple_exercise_time_minutes = Some(90); // Long ski session
    snow_sports_activity.apple_stand_time_minutes = Some(4);
    snow_sports_activity.apple_move_time_minutes = Some(90);
    snow_sports_activity.apple_stand_hour_achieved = Some(true);
    snow_sports_activity.source_device = Some("Apple Watch Ultra 2".to_string());

    // Test batch processing with extended fields
    let activities = vec![
        cycling_activity,
        swimming_activity,
        wheelchair_activity,
        snow_sports_activity,
    ];

    println!("Processing {} activities for user {}", activities.len(), user.id);

    let batch_result = batch_processor
        .process_activity_metrics(user.id, activities)
        .await?;

    println!("Batch result: processed={}, failed={}, errors={:?}",
        batch_result.processed_count,
        batch_result.failed_count,
        batch_result.errors);

    // Validate batch processing success
    assert_eq!(
        batch_result.processed_count, 4,
        "All 4 extended activity metrics should be inserted"
    );

    // Verify data was stored correctly with specialized fields
    let stored_activities: Vec<ActivityMetric> = sqlx::query_as!(
        ActivityMetric,
        r#"SELECT
            id, user_id, recorded_at as "recorded_at!", step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters, push_count, swimming_stroke_count,
            nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes,
            apple_move_time_minutes, apple_stand_hour_achieved,
            NULL::DOUBLE PRECISION as walking_speed_m_per_s, NULL::DOUBLE PRECISION as walking_step_length_cm, NULL::DOUBLE PRECISION as walking_asymmetry_percent,
            NULL::DOUBLE PRECISION as walking_double_support_percent, NULL::DOUBLE PRECISION as six_minute_walk_test_distance_m,
            NULL::DOUBLE PRECISION as stair_ascent_speed_m_per_s, NULL::DOUBLE PRECISION as stair_descent_speed_m_per_s,
            NULL::DOUBLE PRECISION as ground_contact_time_ms, NULL::DOUBLE PRECISION as vertical_oscillation_cm, NULL::DOUBLE PRECISION as running_stride_length_m,
            NULL::DOUBLE PRECISION as running_power_watts, NULL::DOUBLE PRECISION as running_speed_m_per_s,
            NULL::DOUBLE PRECISION as cycling_speed_kmh, NULL::DOUBLE PRECISION as cycling_power_watts, NULL::DOUBLE PRECISION as cycling_cadence_rpm,
            NULL::DOUBLE PRECISION as functional_threshold_power_watts,
            NULL::DOUBLE PRECISION as underwater_depth_meters, NULL::INTEGER as diving_duration_seconds,
            source_device, created_at as "created_at!"
        FROM activity_metrics
        WHERE user_id = $1
        ORDER BY recorded_at DESC"#,
        user.id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(stored_activities.len(), 4);

    // Verify cycling activity specialized fields
    let cycling = stored_activities
        .iter()
        .find(|a| a.distance_cycling_meters.is_some())
        .unwrap();
    assert_eq!(cycling.distance_cycling_meters, Some(15000.0));
    assert_eq!(cycling.nike_fuel_points, Some(750));
    assert_eq!(cycling.apple_exercise_time_minutes, Some(45));
    assert_eq!(cycling.apple_stand_hour_achieved, Some(true));

    // Verify swimming activity specialized fields
    let swimming = stored_activities
        .iter()
        .find(|a| a.swimming_stroke_count.is_some())
        .unwrap();
    assert_eq!(swimming.distance_swimming_meters, Some(2000.0));
    assert_eq!(swimming.swimming_stroke_count, Some(1800));
    assert_eq!(swimming.apple_exercise_time_minutes, Some(30));

    // Verify wheelchair activity accessibility fields
    let wheelchair = stored_activities
        .iter()
        .find(|a| a.push_count.is_some())
        .unwrap();
    assert_eq!(wheelchair.distance_wheelchair_meters, Some(8000.0));
    assert_eq!(wheelchair.push_count, Some(2400));
    assert_eq!(wheelchair.step_count, Some(0)); // No steps for wheelchair user
    assert_eq!(wheelchair.apple_stand_time_minutes, None); // Not applicable

    // Verify snow sports activity specialized fields
    let snow_sports = stored_activities
        .iter()
        .find(|a| a.distance_downhill_snow_sports_meters.is_some())
        .unwrap();
    assert_eq!(
        snow_sports.distance_downhill_snow_sports_meters,
        Some(25000.0)
    );
    assert_eq!(snow_sports.apple_exercise_time_minutes, Some(90));

    cleanup_test_user(&pool, user.id).await?;
    Ok(())
}

#[tokio::test]
async fn test_activity_metrics_validation_extended_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let pool = create_test_pool().await;
    let user = create_test_user(&pool).await?;
    let config = ValidationConfig::default();

    // Test valid activity metrics with extended fields
    let mut valid_activity = create_base_activity_metric(user.id);
    valid_activity.step_count = Some(8500);
    valid_activity.distance_meters = Some(6000.0);
    valid_activity.flights_climbed = Some(5);
    valid_activity.active_energy_burned_kcal = Some(350.0);
    valid_activity.basal_energy_burned_kcal = Some(120.0);
    // Valid specialized fields
    valid_activity.distance_cycling_meters = Some(10000.0);
    valid_activity.distance_swimming_meters = Some(1500.0);
    valid_activity.distance_wheelchair_meters = Some(5000.0);
    valid_activity.distance_downhill_snow_sports_meters = Some(15000.0);
    valid_activity.push_count = Some(1800);
    valid_activity.swimming_stroke_count = Some(1200);
    valid_activity.nike_fuel_points = Some(500);
    valid_activity.apple_exercise_time_minutes = Some(60);
    valid_activity.apple_stand_time_minutes = Some(12);
    valid_activity.apple_move_time_minutes = Some(60);
    valid_activity.apple_stand_hour_achieved = Some(true);
    valid_activity.source_device = Some("Apple Watch".to_string());

    // Should validate successfully
    assert!(valid_activity.validate_with_config(&config).is_ok());

    // Test edge case validations for specialized fields

    // Test swimming distance limits (50km max)
    let mut excessive_swimming = create_base_activity_metric(user.id);
    excessive_swimming.step_count = Some(0);
    excessive_swimming.distance_meters = Some(60000.0); // 60km swimming - excessive
    excessive_swimming.flights_climbed = Some(0);
    excessive_swimming.active_energy_burned_kcal = Some(2000.0);
    excessive_swimming.basal_energy_burned_kcal = Some(300.0);
    excessive_swimming.distance_cycling_meters = None;
    excessive_swimming.distance_swimming_meters = Some(60000.0); // Exceeds 50km limit
    excessive_swimming.distance_wheelchair_meters = None;
    excessive_swimming.distance_downhill_snow_sports_meters = None;
    excessive_swimming.push_count = None;
    excessive_swimming.swimming_stroke_count = Some(50000);
    excessive_swimming.nike_fuel_points = None;
    excessive_swimming.apple_exercise_time_minutes = Some(180);
    excessive_swimming.apple_stand_time_minutes = None;
    excessive_swimming.apple_move_time_minutes = Some(180);
    excessive_swimming.apple_stand_hour_achieved = Some(false);
    excessive_swimming.source_device = Some("Apple Watch Ultra".to_string());

    let result = excessive_swimming.validate_with_config(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("swimming distance"));

    // Test negative values validation
    let mut negative_values = create_base_activity_metric(user.id);
    negative_values.step_count = Some(5000);
    negative_values.distance_meters = Some(5000.0);
    negative_values.flights_climbed = Some(3);
    negative_values.active_energy_burned_kcal = Some(250.0);
    negative_values.basal_energy_burned_kcal = Some(100.0);
    negative_values.distance_cycling_meters = Some(-1000.0); // Negative cycling distance
    negative_values.distance_swimming_meters = None;
    negative_values.distance_wheelchair_meters = None;
    negative_values.distance_downhill_snow_sports_meters = None;
    negative_values.push_count = Some(-500); // Negative push count
    negative_values.swimming_stroke_count = None;
    negative_values.nike_fuel_points = Some(-100);           // Negative Nike Fuel
    negative_values.apple_exercise_time_minutes = Some(-30); // Negative exercise time
    negative_values.apple_stand_time_minutes = Some(8);
    negative_values.apple_move_time_minutes = Some(30);
    negative_values.apple_stand_hour_achieved = Some(true);
    negative_values.source_device = Some("Test Device".to_string());

    let result = negative_values.validate_with_config(&config);
    assert!(result.is_err());

    // Test excessive values for specialized fields
    let mut excessive_values = create_base_activity_metric(user.id);
    excessive_values.step_count = Some(1000);
    excessive_values.distance_meters = Some(1000.0);
    excessive_values.flights_climbed = Some(0);
    excessive_values.active_energy_burned_kcal = Some(100.0);
    excessive_values.basal_energy_burned_kcal = Some(50.0);
    excessive_values.distance_cycling_meters = None;
    excessive_values.distance_swimming_meters = None;
    excessive_values.distance_wheelchair_meters = None;
    excessive_values.distance_downhill_snow_sports_meters = None;
    excessive_values.push_count = Some(60000);                 // Exceeds 50,000 limit
    excessive_values.swimming_stroke_count = Some(150000);     // Exceeds 100,000 limit
    excessive_values.nike_fuel_points = Some(15000);           // Exceeds 10,000 limit
    excessive_values.apple_exercise_time_minutes = Some(1500); // Exceeds 1440 minutes (24 hours)
    excessive_values.apple_stand_time_minutes = Some(1500);    // Exceeds 1440 minutes
    excessive_values.apple_move_time_minutes = Some(1500);     // Exceeds 1440 minutes
    excessive_values.apple_stand_hour_achieved = Some(false);
    excessive_values.source_device = Some("Test Device".to_string());

    let result = excessive_values.validate_with_config(&config);
    assert!(result.is_err());

    cleanup_test_user(&pool, user.id).await?;
    Ok(())
}

#[tokio::test]
async fn test_wheelchair_user_activity_accessibility() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_test_pool().await;
    let user = create_test_user(&pool).await?;
    let batch_processor = BatchProcessor::new(pool.clone());

    // Create wheelchair user characteristics
    let wheelchair_characteristics = UserCharacteristics {
        id: Uuid::new_v4(),
        user_id: user.id,
        biological_sex: BiologicalSex::NotSet,
        date_of_birth: None,
        blood_type: BloodType::NotSet,
        fitzpatrick_skin_type: FitzpatrickSkinType::NotSet,
        wheelchair_use: true, // Wheelchair accessibility enabled
        activity_move_mode: ActivityMoveMode::ActiveEnergy,
        emergency_contact_info: serde_json::json!({}),
        medical_conditions: vec![],
        medications: vec![],
        data_sharing_preferences: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_verified_at: Utc::now(),
    };

    // Insert wheelchair user characteristics
    sqlx::query!(
        r#"INSERT INTO user_characteristics (
            id, user_id, biological_sex, date_of_birth, blood_type,
            fitzpatrick_skin_type, wheelchair_use, activity_move_mode,
            emergency_contact_info, medical_conditions, medications,
            data_sharing_preferences, created_at, updated_at, last_verified_at
        ) VALUES ($1, $2, $3, $4, $5,
                  $6, $7, $8,
                  $9, $10, $11, $12, $13, $14, $15)"#,
        wheelchair_characteristics.id,
        wheelchair_characteristics.user_id,
        wheelchair_characteristics.biological_sex as BiologicalSex,
        wheelchair_characteristics.date_of_birth,
        wheelchair_characteristics.blood_type as BloodType,
        wheelchair_characteristics.fitzpatrick_skin_type as FitzpatrickSkinType,
        wheelchair_characteristics.wheelchair_use,
        wheelchair_characteristics.activity_move_mode as ActivityMoveMode,
        wheelchair_characteristics.emergency_contact_info,
        &wheelchair_characteristics.medical_conditions as &[String],
        &wheelchair_characteristics.medications as &[String],
        wheelchair_characteristics.data_sharing_preferences,
        wheelchair_characteristics.created_at,
        wheelchair_characteristics.updated_at,
        wheelchair_characteristics.last_verified_at
    )
    .execute(&pool)
    .await?;

    // Create realistic wheelchair activity scenarios
    let wheelchair_activities = vec![
        // Morning wheelchair commute
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(8),
            step_count: Some(0), // No steps for wheelchair user
            distance_meters: Some(5000.0),
            flights_climbed: Some(0), // No flights for wheelchair
            active_energy_burned_kcal: Some(180.0),
            basal_energy_burned_kcal: Some(60.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: Some(5000.0),
            distance_downhill_snow_sports_meters: None,
            push_count: Some(1500), // Push count for wheelchair
            swimming_stroke_count: None,
            nike_fuel_points: Some(200),
            apple_exercise_time_minutes: Some(25),
            apple_stand_time_minutes: None, // Not applicable for wheelchair users
            apple_move_time_minutes: Some(25),
            apple_stand_hour_achieved: Some(false), // Not applicable
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
        // Afternoon wheelchair workout
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(3),
            step_count: Some(0),
            distance_meters: Some(8000.0),
            flights_climbed: Some(0),
            active_energy_burned_kcal: Some(400.0),
            basal_energy_burned_kcal: Some(120.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: Some(8000.0),
            distance_downhill_snow_sports_meters: None,
            push_count: Some(2400),
            swimming_stroke_count: None,
            nike_fuel_points: Some(450),
            apple_exercise_time_minutes: Some(45),
            apple_stand_time_minutes: None,
            apple_move_time_minutes: Some(45),
            apple_stand_hour_achieved: Some(false),
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
    ];

    // Process wheelchair accessibility metrics
    let batch_result = batch_processor
        .process_activity_metrics(user.id, wheelchair_activities.clone())
        .await?;
    assert_eq!(
        batch_result.processed_count, 2,
        "Both wheelchair activity metrics should be processed"
    );

    // Verify wheelchair-specific metrics were stored
    let stored_activities: Vec<ActivityMetric> = sqlx::query_as!(
        ActivityMetric,
        r#"SELECT
            id, user_id, recorded_at as "recorded_at!", step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters, push_count, swimming_stroke_count,
            nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes,
            apple_move_time_minutes, apple_stand_hour_achieved,
            NULL::DOUBLE PRECISION as walking_speed_m_per_s, NULL::DOUBLE PRECISION as walking_step_length_cm, NULL::DOUBLE PRECISION as walking_asymmetry_percent,
            NULL::DOUBLE PRECISION as walking_double_support_percent, NULL::DOUBLE PRECISION as six_minute_walk_test_distance_m,
            NULL::DOUBLE PRECISION as stair_ascent_speed_m_per_s, NULL::DOUBLE PRECISION as stair_descent_speed_m_per_s,
            NULL::DOUBLE PRECISION as ground_contact_time_ms, NULL::DOUBLE PRECISION as vertical_oscillation_cm, NULL::DOUBLE PRECISION as running_stride_length_m,
            NULL::DOUBLE PRECISION as running_power_watts, NULL::DOUBLE PRECISION as running_speed_m_per_s,
            NULL::DOUBLE PRECISION as cycling_speed_kmh, NULL::DOUBLE PRECISION as cycling_power_watts, NULL::DOUBLE PRECISION as cycling_cadence_rpm,
            NULL::DOUBLE PRECISION as functional_threshold_power_watts,
            NULL::DOUBLE PRECISION as underwater_depth_meters, NULL::INTEGER as diving_duration_seconds,
            source_device, created_at as "created_at!"
        FROM activity_metrics
        WHERE user_id = $1 AND distance_wheelchair_meters IS NOT NULL
        ORDER BY recorded_at DESC"#,
        user.id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(stored_activities.len(), 2);

    // Verify accessibility-specific features
    for activity in &stored_activities {
        assert_eq!(activity.step_count, Some(0)); // No steps for wheelchair user
        assert_eq!(activity.flights_climbed, Some(0)); // No flights
        assert!(activity.distance_wheelchair_meters.is_some()); // Wheelchair distance tracked
        assert!(activity.push_count.is_some()); // Push count tracked
        assert_eq!(activity.apple_stand_time_minutes, None); // Stand time not applicable
        assert_eq!(activity.apple_stand_hour_achieved, Some(false)); // Stand goals not applicable
    }

    // Test accessibility-adapted validation
    let config = ValidationConfig::default();
    let wheelchair_activity = &wheelchair_activities[0];

    // Should pass validation with wheelchair user characteristics
    assert!(wheelchair_activity
        .validate_with_characteristics(&config, Some(&wheelchair_characteristics))
        .is_ok());

    cleanup_test_user(&pool, user.id).await?;
    Ok(())
}

#[tokio::test]
async fn test_multi_sport_activity_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_test_pool().await;
    let user = create_test_user(&pool).await?;
    let batch_processor = BatchProcessor::new(pool.clone());

    // Create a comprehensive multi-sport day with different specialized metrics
    let mut morning_swim = create_base_activity_metric(user.id);
    morning_swim.recorded_at = Utc::now() - Duration::hours(10);
    morning_swim.step_count = Some(0);
    morning_swim.distance_meters = Some(1000.0); // 1km swim
    morning_swim.flights_climbed = Some(0);
    morning_swim.active_energy_burned_kcal = Some(250.0);
    morning_swim.basal_energy_burned_kcal = Some(80.0);
    morning_swim.distance_cycling_meters = None;
    morning_swim.distance_swimming_meters = Some(1000.0);
    morning_swim.distance_wheelchair_meters = None;
    morning_swim.distance_downhill_snow_sports_meters = None;
    morning_swim.push_count = None;
    morning_swim.swimming_stroke_count = Some(900); // Pool swimming strokes
    morning_swim.nike_fuel_points = None;
    morning_swim.apple_exercise_time_minutes = Some(30);
    morning_swim.apple_stand_time_minutes = Some(1);
    morning_swim.apple_move_time_minutes = Some(30);
    morning_swim.apple_stand_hour_achieved = Some(true);
    morning_swim.source_device = Some("Apple Watch Ultra 2".to_string());

    let multi_sport_activities = vec![
        morning_swim,
        // Afternoon cycling
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(6),
            step_count: Some(150),          // Minimal steps during cycling
            distance_meters: Some(25000.0), // 25km cycling
            flights_climbed: Some(200),     // Hills during cycling
            active_energy_burned_kcal: Some(650.0),
            basal_energy_burned_kcal: Some(180.0),
            distance_cycling_meters: Some(25000.0),
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: Some(850), // High Nike Fuel from intense cycling
            apple_exercise_time_minutes: Some(75),
            apple_stand_time_minutes: Some(2),
            apple_move_time_minutes: Some(75),
            apple_stand_hour_achieved: Some(true),
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
        // Evening walk (basic activity)
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(2),
            step_count: Some(6000), // Regular walking
            distance_meters: Some(4500.0),
            flights_climbed: Some(12),
            active_energy_burned_kcal: Some(180.0),
            basal_energy_burned_kcal: Some(60.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: Some(220),
            apple_exercise_time_minutes: Some(20), // Light exercise
            apple_stand_time_minutes: Some(6),     // Good stand hours
            apple_move_time_minutes: Some(45),
            apple_stand_hour_achieved: Some(true),
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
    ];

    // Process multi-sport day
    let batch_result = batch_processor
        .process_activity_metrics(user.id, multi_sport_activities)
        .await?;
    assert_eq!(
        batch_result.processed_count, 3,
        "All three multi-sport activities should be processed"
    );

    // Query and analyze the multi-sport day data
    let stored_activities: Vec<ActivityMetric> = sqlx::query_as!(
        ActivityMetric,
        r#"SELECT
            id, user_id, recorded_at as "recorded_at!", step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters, push_count, swimming_stroke_count,
            nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes,
            apple_move_time_minutes, apple_stand_hour_achieved,
            NULL::DOUBLE PRECISION as walking_speed_m_per_s, NULL::DOUBLE PRECISION as walking_step_length_cm, NULL::DOUBLE PRECISION as walking_asymmetry_percent,
            NULL::DOUBLE PRECISION as walking_double_support_percent, NULL::DOUBLE PRECISION as six_minute_walk_test_distance_m,
            NULL::DOUBLE PRECISION as stair_ascent_speed_m_per_s, NULL::DOUBLE PRECISION as stair_descent_speed_m_per_s,
            NULL::DOUBLE PRECISION as ground_contact_time_ms, NULL::DOUBLE PRECISION as vertical_oscillation_cm, NULL::DOUBLE PRECISION as running_stride_length_m,
            NULL::DOUBLE PRECISION as running_power_watts, NULL::DOUBLE PRECISION as running_speed_m_per_s,
            NULL::DOUBLE PRECISION as cycling_speed_kmh, NULL::DOUBLE PRECISION as cycling_power_watts, NULL::DOUBLE PRECISION as cycling_cadence_rpm,
            NULL::DOUBLE PRECISION as functional_threshold_power_watts,
            NULL::DOUBLE PRECISION as underwater_depth_meters, NULL::INTEGER as diving_duration_seconds,
            source_device, created_at as "created_at!"
        FROM activity_metrics
        WHERE user_id = $1
        ORDER BY recorded_at ASC"#,
        user.id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(stored_activities.len(), 3);

    // Verify multi-sport day totals and specialized tracking
    let total_distance: f64 = stored_activities
        .iter()
        .filter_map(|a| a.distance_meters)
        .sum();
    assert_eq!(total_distance, 30500.0); // 1km + 25km + 4.5km

    let total_active_energy: f64 = stored_activities
        .iter()
        .filter_map(|a| a.active_energy_burned_kcal)
        .sum();
    assert_eq!(total_active_energy, 1080.0); // Sum of all activities

    let total_exercise_time: i32 = stored_activities
        .iter()
        .filter_map(|a| a.apple_exercise_time_minutes)
        .sum();
    assert_eq!(total_exercise_time, 125); // 30 + 75 + 20

    // Verify sport-specific distances are tracked separately
    let swimming_distance: f64 = stored_activities
        .iter()
        .filter_map(|a| a.distance_swimming_meters)
        .sum();
    assert_eq!(swimming_distance, 1000.0);

    let cycling_distance: f64 = stored_activities
        .iter()
        .filter_map(|a| a.distance_cycling_meters)
        .sum();
    assert_eq!(cycling_distance, 25000.0);

    // Verify swimming stroke analytics
    let total_strokes: i32 = stored_activities
        .iter()
        .filter_map(|a| a.swimming_stroke_count)
        .sum();
    assert_eq!(total_strokes, 900);

    // Verify Nike Fuel cross-platform tracking
    let total_nike_fuel: i32 = stored_activities
        .iter()
        .filter_map(|a| a.nike_fuel_points)
        .sum();
    assert_eq!(total_nike_fuel, 1070); // 850 + 220

    cleanup_test_user(&pool, user.id).await?;
    Ok(())
}

#[tokio::test]
async fn test_apple_watch_activity_rings_integration() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_test_pool().await;
    let user = create_test_user(&pool).await?;
    let batch_processor = BatchProcessor::new(pool.clone());

    // Simulate a complete Apple Watch activity rings day
    let activity_rings_data = vec![
        // Hour 1: Morning workout - achieving exercise and move goals
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(12),
            step_count: Some(0),
            distance_meters: Some(5000.0),
            flights_climbed: Some(0),
            active_energy_burned_kcal: Some(300.0),
            basal_energy_burned_kcal: Some(120.0),
            distance_cycling_meters: Some(5000.0),
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(30), // Exercise ring progress
            apple_stand_time_minutes: Some(1),     // Stand ring progress
            apple_move_time_minutes: Some(30),     // Move ring progress
            apple_stand_hour_achieved: Some(true), // Stand goal achieved this hour
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
        // Hours 2-12: Throughout the day, accumulating stand hours
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now() - Duration::hours(6),
            step_count: Some(8000),
            distance_meters: Some(6000.0),
            flights_climbed: Some(15),
            active_energy_burned_kcal: Some(220.0),
            basal_energy_burned_kcal: Some(180.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(15), // Additional exercise
            apple_stand_time_minutes: Some(8),     // Accumulated stand hours
            apple_move_time_minutes: Some(45),     // Total move time
            apple_stand_hour_achieved: Some(true), // Good standing throughout day
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
        // Evening: Final activity ring completion
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id: user.id,
            recorded_at: Utc::now(),
            step_count: Some(12500), // Daily step goal achieved
            distance_meters: Some(9500.0),
            flights_climbed: Some(25),
            active_energy_burned_kcal: Some(150.0),
            basal_energy_burned_kcal: Some(100.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: Some(5), // Final exercise minutes
            apple_stand_time_minutes: Some(12),   // Full 12-hour stand goal
            apple_move_time_minutes: Some(50),    // Complete move goal
            apple_stand_hour_achieved: Some(true), // Perfect stand day
            // Mobility Metrics
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            // Running Dynamics
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            // Cycling Metrics
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            // Underwater Metrics
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("Apple Watch Series 9".to_string()),
            created_at: Utc::now(),
        },
    ];

    // Process Apple Watch activity rings data
    let batch_result = batch_processor
        .process_activity_metrics(user.id, activity_rings_data)
        .await?;
    assert_eq!(
        batch_result.processed_count, 3,
        "All Apple Watch activity ring metrics should be processed"
    );

    // Query Apple Watch specific metrics
    let apple_watch_activities: Vec<ActivityMetric> = sqlx::query_as!(
        ActivityMetric,
        r#"SELECT
            id, user_id, recorded_at as "recorded_at!", step_count, distance_meters, flights_climbed,
            active_energy_burned_kcal, basal_energy_burned_kcal,
            distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters,
            distance_downhill_snow_sports_meters, push_count, swimming_stroke_count,
            nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes,
            apple_move_time_minutes, apple_stand_hour_achieved,
            NULL::DOUBLE PRECISION as walking_speed_m_per_s, NULL::DOUBLE PRECISION as walking_step_length_cm, NULL::DOUBLE PRECISION as walking_asymmetry_percent,
            NULL::DOUBLE PRECISION as walking_double_support_percent, NULL::DOUBLE PRECISION as six_minute_walk_test_distance_m,
            NULL::DOUBLE PRECISION as stair_ascent_speed_m_per_s, NULL::DOUBLE PRECISION as stair_descent_speed_m_per_s,
            NULL::DOUBLE PRECISION as ground_contact_time_ms, NULL::DOUBLE PRECISION as vertical_oscillation_cm, NULL::DOUBLE PRECISION as running_stride_length_m,
            NULL::DOUBLE PRECISION as running_power_watts, NULL::DOUBLE PRECISION as running_speed_m_per_s,
            NULL::DOUBLE PRECISION as cycling_speed_kmh, NULL::DOUBLE PRECISION as cycling_power_watts, NULL::DOUBLE PRECISION as cycling_cadence_rpm,
            NULL::DOUBLE PRECISION as functional_threshold_power_watts,
            NULL::DOUBLE PRECISION as underwater_depth_meters, NULL::INTEGER as diving_duration_seconds,
            source_device, created_at as "created_at!"
        FROM activity_metrics
        WHERE user_id = $1 AND source_device LIKE '%Apple Watch%'
        ORDER BY recorded_at ASC"#,
        user.id
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(apple_watch_activities.len(), 3);

    // Verify Apple Watch activity rings completion
    let final_activity = &apple_watch_activities[2]; // Evening activity

    // Verify exercise ring (goal typically 30 minutes)
    let total_exercise_time: i32 = apple_watch_activities
        .iter()
        .filter_map(|a| a.apple_exercise_time_minutes)
        .sum();
    assert_eq!(total_exercise_time, 50); // 30 + 15 + 5 (exceeds 30-minute goal)

    // Verify stand ring (goal typically 12 hours)
    assert_eq!(final_activity.apple_stand_time_minutes, Some(12)); // Complete 12-hour goal

    // Verify move ring (varies per user, tracking move time)
    assert_eq!(final_activity.apple_move_time_minutes, Some(50));

    // Verify consistent stand hour achievement
    let stand_hours_achieved = apple_watch_activities
        .iter()
        .filter(|a| a.apple_stand_hour_achieved == Some(true))
        .count();
    assert_eq!(stand_hours_achieved, 3); // All three periods achieved stand goals

    cleanup_test_user(&pool, user.id).await?;
    Ok(())
}

// Helper functions
#[derive(Debug)]
struct TestUser {
    id: Uuid,
}

async fn create_test_user(pool: &PgPool) -> Result<TestUser, Box<dyn std::error::Error>> {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_activity_ext_{}@example.com", user_id)
    )
    .execute(pool)
    .await?;

    Ok(TestUser { id: user_id })
}

async fn cleanup_test_user(pool: &PgPool, user_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
    // Clean up in reverse order of foreign key dependencies
    sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await?;

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await?;

    Ok(())
}
