// Integration tests for user characteristics functionality
use actix_web::{http::StatusCode, test, web, App};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// Import the application modules
use self_sensored::db::database::create_connection_pool;
use self_sensored::handlers::user_characteristics_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::enums::{
    ActivityMoveMode, BiologicalSex, BloodType, FitzpatrickSkinType,
};
use self_sensored::models::user_characteristics::{UserCharacteristics, UserCharacteristicsInput};
use self_sensored::services::auth::AuthContext;
use self_sensored::services::auth::AuthService;
use self_sensored::services::user_characteristics::UserCharacteristicsService;

/// Helper function to create test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool")
}

/// Helper to create test user characteristics input
fn create_test_characteristics_input() -> UserCharacteristicsInput {
    UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Female),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 5, 15).unwrap()),
        blood_type: Some(BloodType::APositive),
        fitzpatrick_skin_type: Some(FitzpatrickSkinType::Type3),
        wheelchair_use: Some(false),
        activity_move_mode: Some(ActivityMoveMode::ActiveEnergy),
        emergency_contact_info: Some(json!({
            "name": "Jane Doe",
            "phone": "+1-555-0123",
            "relationship": "spouse"
        })),
        medical_conditions: Some(vec!["Asthma".to_string()]),
        medications: Some(vec!["Albuterol".to_string()]),
        data_sharing_preferences: Some(json!({
            "research_participation": true,
            "anonymized_analytics": true,
            "emergency_sharing": true
        })),
    }
}

/// Helper to create test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at, is_active, metadata) VALUES ($1, $2, NOW(), NOW(), true, '{}'::jsonb)",
        user_id,
        format!("test-{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Helper to create test API key
async fn create_test_api_key(pool: &PgPool, user_id: Uuid) -> String {
    let api_key_id = Uuid::new_v4();
    let api_key = format!("test_key_{}", api_key_id);
    let key_hash = format!("hash_{}", api_key); // In real app, this would be properly hashed

    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, key_hash, name, created_at, is_active, permissions, rate_limit_per_hour) VALUES ($1, $2, $3, $4, NOW(), true, '[\"read\", \"write\"]'::jsonb, 1000)",
        api_key_id,
        user_id,
        key_hash,
        "Test API Key"
    )
    .execute(pool)
    .await
    .expect("Failed to create test API key");

    api_key
}

/// Cleanup test data
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Delete user characteristics (will cascade to related data)
    sqlx::query!(
        "DELETE FROM user_characteristics WHERE user_id = $1",
        user_id
    )
    .execute(pool)
    .await
    .ok();

    // Delete API keys
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    // Delete user
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

#[tokio::test]
async fn test_create_user_characteristics() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let _api_key = create_test_api_key(&pool, user_id).await;

    // Create the user characteristics service
    let characteristics_service = UserCharacteristicsService::new(pool.clone());
    let input = create_test_characteristics_input();

    // Test creating user characteristics
    let result = characteristics_service.create(user_id, input).await;
    assert!(
        result.is_ok(),
        "Failed to create user characteristics: {:?}",
        result
    );

    let characteristics = result.unwrap();
    assert_eq!(characteristics.user_id, user_id);
    assert_eq!(characteristics.biological_sex, BiologicalSex::Female);
    assert_eq!(characteristics.blood_type, BloodType::APositive);
    assert_eq!(
        characteristics.fitzpatrick_skin_type,
        FitzpatrickSkinType::Type3
    );
    assert!(!characteristics.wheelchair_use);
    assert_eq!(
        characteristics.activity_move_mode,
        ActivityMoveMode::ActiveEnergy
    );
    assert_eq!(
        characteristics.medical_conditions,
        vec!["Asthma".to_string()]
    );
    assert_eq!(characteristics.medications, vec!["Albuterol".to_string()]);

    // Test completeness score
    assert!(
        characteristics.completeness_score() > 90.0,
        "Completeness score should be high"
    );
    assert!(
        characteristics.is_complete_for_personalization(),
        "Should be complete for personalization"
    );

    // Test age calculation
    let age = characteristics.age().expect("Should have age");
    assert!(
        age >= 33 && age <= 35,
        "Age should be around 34 (born in 1990)"
    );

    // Test UV recommendations
    let uv_recommendations = characteristics.get_uv_recommendations();
    assert!(uv_recommendations["recommended_spf"].as_u64().unwrap() >= 15);

    // Test activity personalization
    let activity_personalization = characteristics.get_activity_personalization();
    assert!(!activity_personalization["wheelchair_use"]
        .as_bool()
        .unwrap());
    assert_eq!(
        activity_personalization["goal_unit"].as_str().unwrap(),
        "calories"
    );

    // Test heart rate zones
    let hr_zones = characteristics.get_heart_rate_zones(Some(65));
    assert!(hr_zones["max_heart_rate"].as_u64().unwrap() > 180);

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_get_user_characteristics() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Test getting non-existent characteristics
    let result = characteristics_service
        .get_by_user_id(user_id)
        .await
        .unwrap();
    assert!(
        result.is_none(),
        "Should not find characteristics for new user"
    );

    // Create characteristics
    let input = create_test_characteristics_input();
    characteristics_service
        .create(user_id, input)
        .await
        .unwrap();

    // Test getting existing characteristics
    let result = characteristics_service
        .get_by_user_id(user_id)
        .await
        .unwrap();
    assert!(
        result.is_some(),
        "Should find characteristics after creation"
    );

    let characteristics = result.unwrap();
    assert_eq!(characteristics.user_id, user_id);
    assert_eq!(characteristics.biological_sex, BiologicalSex::Female);

    // Test getting with personalization info
    let response = characteristics_service
        .get_with_personalization(user_id)
        .await
        .unwrap();
    assert!(response.is_some(), "Should get personalization response");

    let response = response.unwrap();
    assert_eq!(response.characteristics.user_id, user_id);
    assert!(response.personalization.is_complete);
    assert!(response.personalization.completeness_score > 90.0);
    assert!(response.personalization.personalization_features.len() > 3);

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_update_user_characteristics() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Create initial characteristics
    let input = create_test_characteristics_input();
    characteristics_service
        .create(user_id, input)
        .await
        .unwrap();

    // Test updating characteristics
    let update_input = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Male),
        wheelchair_use: Some(true),
        activity_move_mode: Some(ActivityMoveMode::MoveTime),
        medical_conditions: Some(vec!["Asthma".to_string(), "Diabetes".to_string()]),
        ..Default::default()
    };

    let result = characteristics_service
        .update(user_id, update_input)
        .await
        .unwrap();
    assert!(
        result.is_some(),
        "Should successfully update characteristics"
    );

    let updated = result.unwrap();
    assert_eq!(updated.biological_sex, BiologicalSex::Male);
    assert!(updated.wheelchair_use);
    assert_eq!(updated.activity_move_mode, ActivityMoveMode::MoveTime);
    assert_eq!(updated.medical_conditions.len(), 2);
    assert!(updated.medical_conditions.contains(&"Diabetes".to_string()));

    // Test activity personalization after update
    let activity_personalization = updated.get_activity_personalization();
    assert!(activity_personalization["wheelchair_use"]
        .as_bool()
        .unwrap());
    assert_eq!(
        activity_personalization["goal_unit"].as_str().unwrap(),
        "minutes"
    );
    assert!(activity_personalization["is_accessibility_mode"]
        .as_bool()
        .unwrap());

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_validation_ranges() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Test without characteristics
    let ranges = characteristics_service
        .get_validation_ranges(user_id, "heart_rate")
        .await
        .unwrap();
    assert!(!ranges["personalized"].as_bool().unwrap());

    // Create characteristics with specific profile
    let input = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Female),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1980, 1, 1).unwrap()), // 44 years old
        wheelchair_use: Some(true),
        ..Default::default()
    };
    characteristics_service
        .create(user_id, input)
        .await
        .unwrap();

    // Test heart rate ranges (should be personalized for 44-year-old female)
    let hr_ranges = characteristics_service
        .get_validation_ranges(user_id, "heart_rate")
        .await
        .unwrap();
    assert!(hr_ranges["personalized"].as_bool().unwrap());
    let max_exercise = hr_ranges["max_exercise"].as_u64().unwrap();
    assert!(
        max_exercise > 170 && max_exercise < 190,
        "Max exercise HR should be around 176-180 for 44-year-old"
    );

    // Test activity ranges (should be adapted for wheelchair use)
    let activity_ranges = characteristics_service
        .get_validation_ranges(user_id, "activity")
        .await
        .unwrap();
    assert!(activity_ranges["personalized"].as_bool().unwrap());
    assert!(activity_ranges["wheelchair_adapted"].as_bool().unwrap());
    assert_eq!(activity_ranges["step_count_max"].as_u64().unwrap(), 10000);
    assert_eq!(activity_ranges["distance_max_km"].as_f64().unwrap(), 100.0);

    // Test blood pressure ranges (should be age-adjusted)
    let bp_ranges = characteristics_service
        .get_validation_ranges(user_id, "blood_pressure")
        .await
        .unwrap();
    assert!(bp_ranges["personalized"].as_bool().unwrap());
    assert_eq!(bp_ranges["systolic_max"].as_u64().unwrap(), 140); // Under 65, so 140 not 150

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_ios_data_processing() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Test processing iOS data format
    let ios_data = json!({
        "characteristics": {
            "biological_sex": "male",
            "date_of_birth": "1985-12-25",
            "blood_type": "O+",
            "fitzpatrick_skin_type": "4",
            "wheelchair_use": false,
            "activity_move_mode": "active_energy",
            "medical_conditions": ["High Blood Pressure", "Seasonal Allergies"],
            "medications": ["Lisinopril", "Claritin"]
        }
    });

    let result = characteristics_service
        .process_ios_data(user_id, &ios_data)
        .await
        .unwrap();
    assert!(result.is_some(), "Should successfully process iOS data");

    let characteristics = result.unwrap();
    assert_eq!(characteristics.biological_sex, BiologicalSex::Male);
    assert_eq!(characteristics.blood_type, BloodType::OPositive);
    assert_eq!(
        characteristics.fitzpatrick_skin_type,
        FitzpatrickSkinType::Type4
    );
    assert!(!characteristics.wheelchair_use);
    assert_eq!(
        characteristics.activity_move_mode,
        ActivityMoveMode::ActiveEnergy
    );
    assert_eq!(characteristics.medical_conditions.len(), 2);
    assert!(characteristics
        .medical_conditions
        .contains(&"High Blood Pressure".to_string()));

    // Test age calculation for 1985 birth year
    let age = characteristics.age().unwrap();
    assert!(age >= 38 && age <= 40, "Age should be around 39");

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_aggregate_stats() {
    let pool = create_test_pool().await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Create multiple test users with different characteristics
    let user1_id = create_test_user(&pool).await;
    let user2_id = create_test_user(&pool).await;
    let user3_id = create_test_user(&pool).await;

    // User 1: Complete profile, wheelchair user
    let input1 = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Female),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        blood_type: Some(BloodType::APositive),
        fitzpatrick_skin_type: Some(FitzpatrickSkinType::Type2),
        wheelchair_use: Some(true),
        activity_move_mode: Some(ActivityMoveMode::MoveTime),
        ..Default::default()
    };
    characteristics_service
        .create(user1_id, input1)
        .await
        .unwrap();

    // User 2: Partial profile
    let input2 = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Male),
        wheelchair_use: Some(false),
        ..Default::default()
    };
    characteristics_service
        .create(user2_id, input2)
        .await
        .unwrap();

    // User 3: Complete profile
    let input3 = create_test_characteristics_input();
    characteristics_service
        .create(user3_id, input3)
        .await
        .unwrap();

    // Get aggregate statistics
    let stats = characteristics_service.get_aggregate_stats().await.unwrap();

    assert!(stats["total_profiles"].as_u64().unwrap() >= 3);
    assert!(
        stats["completion_rates"]["biological_sex"]
            .as_u64()
            .unwrap()
            >= 3
    );
    assert!(stats["accessibility"]["wheelchair_users"].as_u64().unwrap() >= 1);
    assert!(stats["average_completeness_score"].as_f64().unwrap() > 0.0);

    // Cleanup
    cleanup_test_data(&pool, user1_id).await;
    cleanup_test_data(&pool, user2_id).await;
    cleanup_test_data(&pool, user3_id).await;
}

#[tokio::test]
async fn test_wheelchair_user_validation_integration() {
    let pool = create_test_pool().await;
    use chrono::Utc;
    use self_sensored::config::ValidationConfig;
    use self_sensored::models::health_metrics::{ActivityMetric, HeartRateMetric};

    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Create wheelchair user characteristics
    let input = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Male),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1985, 6, 15).unwrap()),
        wheelchair_use: Some(true),
        activity_move_mode: Some(ActivityMoveMode::MoveTime),
        ..Default::default()
    };
    let characteristics = characteristics_service
        .create(user_id, input)
        .await
        .unwrap();

    let config = ValidationConfig::default();

    // Test heart rate validation with personalization
    let heart_rate_metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: Utc::now(),
        heart_rate: Some(85),
        resting_heart_rate: Some(65),
        heart_rate_variability: Some(25.0),
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Apple Watch".to_string()),
        context: None,
        created_at: Utc::now(),
    };

    // Should pass personalized validation
    let result = heart_rate_metric.validate_with_characteristics(&config, Some(&characteristics));
    assert!(
        result.is_ok(),
        "Heart rate should pass personalized validation: {:?}",
        result
    );

    // Test activity metric validation with wheelchair adaptations
    let activity_metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: Utc::now(),
        step_count: Some(500),         // Low step count for wheelchair user
        distance_meters: Some(2000.0), // 2km
        flights_climbed: Some(0),      // No flights for wheelchair user
        active_energy_burned_kcal: Some(250.0),
        basal_energy_burned_kcal: Some(1800.0),
        distance_cycling_meters: None,
        distance_swimming_meters: None,
        distance_wheelchair_meters: Some(2000.0), // Wheelchair-specific distance
        distance_downhill_snow_sports_meters: None,
        push_count: Some(200), // Wheelchair pushes
        swimming_stroke_count: None,
        nike_fuel_points: None,
        apple_exercise_time_minutes: None,
        apple_stand_time_minutes: None,
        apple_move_time_minutes: None,
        apple_stand_hour_achieved: None,
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    };

    // Should pass wheelchair-adapted validation
    let result = activity_metric.validate_with_characteristics(&config, Some(&characteristics));
    assert!(
        result.is_ok(),
        "Activity should pass wheelchair-adapted validation: {:?}",
        result
    );

    // Test activity metric that would fail for wheelchair user
    let high_step_activity = ActivityMetric {
        step_count: Some(50000), // Extremely high step count
        ..activity_metric.clone()
    };

    let result = high_step_activity.validate_with_characteristics(&config, Some(&characteristics));
    assert!(
        result.is_err(),
        "High step count should fail wheelchair validation"
    );
    assert!(result.unwrap_err().contains("wheelchair user"));

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_profile_completion_tracking() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;
    let characteristics_service = UserCharacteristicsService::new(pool.clone());

    // Create minimal profile
    let minimal_input = UserCharacteristicsInput {
        biological_sex: Some(BiologicalSex::Female),
        ..Default::default()
    };
    characteristics_service
        .create(user_id, minimal_input)
        .await
        .unwrap();

    // Check if user appears in incomplete profiles
    let incomplete_users = characteristics_service
        .get_incomplete_profiles(Some(10))
        .await
        .unwrap();
    assert!(
        incomplete_users.contains(&user_id),
        "User should be in incomplete profiles list"
    );

    // Complete the profile
    let complete_input = UserCharacteristicsInput {
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        blood_type: Some(BloodType::OPositive),
        fitzpatrick_skin_type: Some(FitzpatrickSkinType::Type3),
        activity_move_mode: Some(ActivityMoveMode::ActiveEnergy),
        ..Default::default()
    };
    characteristics_service
        .update(user_id, complete_input)
        .await
        .unwrap();

    // Check personalization capabilities
    let has_personalization = characteristics_service
        .has_personalization_data(user_id)
        .await
        .unwrap();
    assert!(
        has_personalization,
        "Should have personalization data after completion"
    );

    // Update verification timestamp
    characteristics_service
        .update_last_verified(user_id)
        .await
        .unwrap();

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

// Helper trait for default UserCharacteristicsInput
// COMMENTED OUT: Orphan rule violation
/*
impl Default for UserCharacteristicsInput {
    fn default() -> Self {
        Self {
            biological_sex: None,
            date_of_birth: None,
            blood_type: None,
            fitzpatrick_skin_type: None,
            wheelchair_use: None,
            activity_move_mode: None,
            emergency_contact_info: None,
            medical_conditions: None,
            medications: None,
            data_sharing_preferences: None,
        }
    }
}
*/
