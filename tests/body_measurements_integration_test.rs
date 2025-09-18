use actix_web::{http::StatusCode, test, web, App, HttpMessage};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    config::ValidationConfig,
    db::database::create_connection_pool,
    handlers::body_measurements_handler::{
        get_body_measurements_data, ingest_body_measurements, BodyMeasurementsDataResponse,
        BodyMeasurementsIngestPayload, BodyMeasurementsIngestRequest,
        BodyMeasurementsIngestResponse,
    },
    middleware::metrics::Metrics,
    models::health_metrics::BodyMeasurementMetric,
    services::{auth::AuthContext, batch_processor::BatchProcessor},
};

/// Helper function to create test database pool
async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");

    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool")
}

/// Helper function to create test user and return user_id
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW()) ON CONFLICT (id) DO NOTHING",
        user_id,
        format!("test-{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");
    user_id
}

/// Helper function to clean up test data
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up in correct order due to foreign key constraints
    sqlx::query!("DELETE FROM body_measurements WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Helper function to create auth context
fn create_auth_context(user_id: Uuid) -> AuthContext {
    AuthContext::new_for_testing(user_id)
}

#[actix_web::test]
async fn test_body_measurements_ingestion_basic() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Create test body measurements payload
    let payload = BodyMeasurementsIngestPayload {
        body_measurements: vec![BodyMeasurementsIngestRequest {
            recorded_at: Utc::now(),
            body_weight_kg: Some(70.5),
            body_mass_index: Some(22.8),
            body_fat_percentage: Some(15.2),
            lean_body_mass_kg: Some(59.8),
            height_cm: Some(175.0),
            waist_circumference_cm: Some(80.0),
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: None,
            basal_body_temperature_celsius: None,
            measurement_source: Some("smart_scale".to_string()),
            bmi_calculated: Some(true),
            measurement_reliability: Some("high".to_string()),
            body_composition_method: Some("bioelectric_impedance".to_string()),
            fitness_phase: Some("maintenance".to_string()),
            measurement_conditions: Some("morning_fasted".to_string()),
            measurement_notes: Some("Post-workout measurement".to_string()),
            source_device: Some("InBody Scale".to_string()),
        }],
    };

    // Create test dependencies
    let batch_processor = web::Data::new(BatchProcessor::new(pool.clone()));
    let metrics = web::Data::new(Metrics::new().expect("Failed to create metrics"));
    let validation_config = web::Data::new(ValidationConfig::default());
    let auth = create_auth_context(user_id);

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(batch_processor.clone())
            .app_data(metrics.clone())
            .app_data(validation_config.clone())
            .route("/ingest", web::post().to(ingest_body_measurements)),
    )
    .await;

    // Create request with authentication
    let req = test::TestRequest::post()
        .uri("/ingest")
        .set_json(&payload)
        .to_request();

    // Manually set auth context in extensions
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;

    // Check response
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsIngestResponse = test::read_body_json(response).await;
    assert!(response_body.success);
    assert_eq!(response_body.processed_count, 1);
    assert_eq!(response_body.failed_count, 0);
    assert!(response_body.errors.is_empty());
    assert!(response_body.body_composition_analysis.is_some());

    // Verify data was stored in database
    let stored_measurement = sqlx::query_as!(
        BodyMeasurementMetric,
        r#"
        SELECT
            id, user_id, recorded_at,
            body_weight_kg, body_mass_index, body_fat_percentage, lean_body_mass_kg,
            height_cm, waist_circumference_cm, hip_circumference_cm, chest_circumference_cm,
            arm_circumference_cm, thigh_circumference_cm,
            body_temperature_celsius, basal_body_temperature_celsius,
            measurement_source, source_device, created_at as "created_at!"
        FROM body_measurements
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch stored body measurement");

    assert_eq!(stored_measurement.user_id, user_id);
    assert_eq!(stored_measurement.body_weight_kg, Some(70.5));
    assert_eq!(stored_measurement.body_mass_index, Some(22.8));
    assert_eq!(stored_measurement.body_fat_percentage, Some(15.2));
    assert_eq!(
        stored_measurement.measurement_source,
        Some("smart_scale".to_string())
    );

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_body_measurements_validation() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Create test payload with invalid data
    let payload = BodyMeasurementsIngestPayload {
        body_measurements: vec![
            // Invalid weight (too high)
            BodyMeasurementsIngestRequest {
                recorded_at: Utc::now(),
                body_weight_kg: Some(600.0), // Over 500kg limit
                body_mass_index: None,
                body_fat_percentage: None,
                lean_body_mass_kg: None,
                height_cm: None,
                waist_circumference_cm: None,
                hip_circumference_cm: None,
                chest_circumference_cm: None,
                arm_circumference_cm: None,
                thigh_circumference_cm: None,
                body_temperature_celsius: None,
                basal_body_temperature_celsius: None,
                measurement_source: Some("manual".to_string()),
                bmi_calculated: Some(false),
                measurement_reliability: None,
                body_composition_method: None,
                fitness_phase: None,
                measurement_conditions: None,
                measurement_notes: None,
                source_device: None,
            },
            // Invalid BMI (too high)
            BodyMeasurementsIngestRequest {
                recorded_at: Utc::now(),
                body_weight_kg: None,
                body_mass_index: Some(80.0), // Over 60 limit
                body_fat_percentage: None,
                lean_body_mass_kg: None,
                height_cm: None,
                waist_circumference_cm: None,
                hip_circumference_cm: None,
                chest_circumference_cm: None,
                arm_circumference_cm: None,
                thigh_circumference_cm: None,
                body_temperature_celsius: None,
                basal_body_temperature_celsius: None,
                measurement_source: Some("manual".to_string()),
                bmi_calculated: Some(false),
                measurement_reliability: None,
                body_composition_method: None,
                fitness_phase: None,
                measurement_conditions: None,
                measurement_notes: None,
                source_device: None,
            },
            // Valid measurement
            BodyMeasurementsIngestRequest {
                recorded_at: Utc::now(),
                body_weight_kg: Some(75.0),
                body_mass_index: Some(24.0),
                body_fat_percentage: Some(18.0),
                lean_body_mass_kg: Some(61.5),
                height_cm: Some(178.0),
                waist_circumference_cm: Some(85.0),
                hip_circumference_cm: None,
                chest_circumference_cm: None,
                arm_circumference_cm: None,
                thigh_circumference_cm: None,
                body_temperature_celsius: None,
                basal_body_temperature_celsius: None,
                measurement_source: Some("smart_scale".to_string()),
                bmi_calculated: Some(true),
                measurement_reliability: Some("standard".to_string()),
                body_composition_method: Some("bioelectric_impedance".to_string()),
                fitness_phase: Some("general_fitness".to_string()),
                measurement_conditions: Some("evening".to_string()),
                measurement_notes: None,
                source_device: Some("Withings Scale".to_string()),
            },
        ],
    };

    // Create test dependencies
    let batch_processor = web::Data::new(BatchProcessor::new(pool.clone()));
    let metrics = web::Data::new(Metrics::new().expect("Failed to create metrics"));
    let validation_config = web::Data::new(ValidationConfig::default());
    let auth = create_auth_context(user_id);

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(batch_processor.clone())
            .app_data(metrics.clone())
            .app_data(validation_config.clone())
            .route("/ingest", web::post().to(ingest_body_measurements)),
    )
    .await;

    // Create request with authentication
    let req = test::TestRequest::post()
        .uri("/ingest")
        .set_json(&payload)
        .to_request();

    // Manually set auth context in extensions
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;

    // Check response
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsIngestResponse = test::read_body_json(response).await;
    // Should have 1 successful and 2 failed
    assert_eq!(response_body.processed_count, 1);
    assert_eq!(response_body.failed_count, 2);
    assert_eq!(response_body.errors.len(), 2);

    // Check error messages
    assert!(response_body.errors[0]
        .message
        .contains("outside valid range"));
    assert!(response_body.errors[1]
        .message
        .contains("outside valid range"));

    // Verify only the valid measurement was stored
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count body measurements");

    assert_eq!(count, Some(1));

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_body_measurements_data_retrieval() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Insert test body measurements directly into database
    let measurement1_id = Uuid::new_v4();
    let measurement2_id = Uuid::new_v4();
    let now = Utc::now();
    let earlier = now - chrono::Duration::days(1);

    sqlx::query!(
        r#"
        INSERT INTO body_measurements (
            id, user_id, recorded_at, body_weight_kg, body_mass_index,
            body_fat_percentage, height_cm, measurement_source, source_device
        ) VALUES
        ($1, $2, $3, $4, $5, $6, $7, $8, $9),
        ($10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#,
        measurement1_id,
        user_id,
        now,
        72.5,
        23.2,
        16.8,
        176.0,
        "smart_scale",
        "InBody",
        measurement2_id,
        user_id,
        earlier,
        73.0,
        23.5,
        17.2,
        176.0,
        "manual",
        "Manual Entry"
    )
    .execute(&pool)
    .await
    .expect("Failed to insert test body measurements");

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/data", web::get().to(get_body_measurements_data)),
    )
    .await;

    // Test data retrieval without filters
    let auth = create_auth_context(user_id);
    let req = test::TestRequest::get().uri("/data").to_request();
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsDataResponse = test::read_body_json(response).await;
    assert_eq!(response_body.body_measurements.len(), 2);
    assert_eq!(response_body.total_count, 2);
    assert!(response_body.date_range.is_some());

    // Measurements should be ordered by recorded_at DESC (most recent first)
    assert_eq!(response_body.body_measurements[0].id, measurement1_id);
    assert_eq!(response_body.body_measurements[1].id, measurement2_id);

    // Test with measurement type filter
    let auth = create_auth_context(user_id);
    let req = test::TestRequest::get()
        .uri("/data?measurement_type=weight")
        .to_request();
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsDataResponse = test::read_body_json(response).await;
    assert_eq!(response_body.body_measurements.len(), 2); // Both have weight

    // Test with measurement source filter
    let auth = create_auth_context(user_id);
    let req = test::TestRequest::get()
        .uri("/data?measurement_source=smart_scale")
        .to_request();
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsDataResponse = test::read_body_json(response).await;
    assert_eq!(response_body.body_measurements.len(), 1); // Only one from smart_scale

    // Test with limit
    let auth = create_auth_context(user_id);
    let req = test::TestRequest::get().uri("/data?limit=1").to_request();
    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsDataResponse = test::read_body_json(response).await;
    assert_eq!(response_body.body_measurements.len(), 1); // Limited to 1
    assert_eq!(response_body.total_count, 2); // But total count is still 2

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_bmi_consistency_validation() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Create test payload with BMI inconsistency
    let payload = BodyMeasurementsIngestPayload {
        body_measurements: vec![BodyMeasurementsIngestRequest {
            recorded_at: Utc::now(),
            body_weight_kg: Some(70.0),  // 70kg
            height_cm: Some(170.0),      // 170cm
            body_mass_index: Some(30.0), // BMI 30, but calculated should be ~24.2
            body_fat_percentage: None,
            lean_body_mass_kg: None,
            waist_circumference_cm: None,
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: None,
            basal_body_temperature_celsius: None,
            measurement_source: Some("smart_scale".to_string()),
            bmi_calculated: Some(true), // This indicates BMI was calculated from weight/height
            measurement_reliability: Some("standard".to_string()),
            body_composition_method: None,
            fitness_phase: None,
            measurement_conditions: None,
            measurement_notes: Some("BMI inconsistency test".to_string()),
            source_device: Some("Test Scale".to_string()),
        }],
    };

    // Create test dependencies
    let batch_processor = web::Data::new(BatchProcessor::new(pool.clone()));
    let metrics = web::Data::new(Metrics::new().expect("Failed to create metrics"));
    let validation_config = web::Data::new(ValidationConfig::default());
    let auth = create_auth_context(user_id);

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(batch_processor.clone())
            .app_data(metrics.clone())
            .app_data(validation_config.clone())
            .route("/ingest", web::post().to(ingest_body_measurements)),
    )
    .await;

    // Create request with authentication
    let req = test::TestRequest::post()
        .uri("/ingest")
        .set_json(&payload)
        .to_request();

    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;

    // Check response - should still succeed but with warnings logged
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsIngestResponse = test::read_body_json(response).await;
    assert!(response_body.success);
    assert_eq!(response_body.processed_count, 1);
    assert_eq!(response_body.failed_count, 0);

    // Verify data was stored (inconsistency is logged but doesn't prevent storage)
    let stored_measurement = sqlx::query!(
        "SELECT body_weight_kg, height_cm, body_mass_index FROM body_measurements WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch stored body measurement");

    assert_eq!(stored_measurement.body_weight_kg, Some(70.0));
    assert_eq!(stored_measurement.height_cm, Some(170.0));
    assert_eq!(stored_measurement.body_mass_index, Some(30.0));

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_body_measurements_smart_scale_integration() {
    let pool = create_test_pool().await;
    let user_id = create_test_user(&pool).await;

    // Simulate smart scale data with comprehensive body composition
    let payload = BodyMeasurementsIngestPayload {
        body_measurements: vec![BodyMeasurementsIngestRequest {
            recorded_at: Utc::now(),
            body_weight_kg: Some(68.2),
            body_mass_index: Some(21.8),
            body_fat_percentage: Some(12.5),
            lean_body_mass_kg: Some(59.7),
            height_cm: Some(178.0), // Stored from previous measurement
            waist_circumference_cm: None,
            hip_circumference_cm: None,
            chest_circumference_cm: None,
            arm_circumference_cm: None,
            thigh_circumference_cm: None,
            body_temperature_celsius: None,
            basal_body_temperature_celsius: None,
            measurement_source: Some("smart_scale".to_string()),
            bmi_calculated: Some(true),
            measurement_reliability: Some("high".to_string()),
            body_composition_method: Some("bioelectric_impedance".to_string()),
            fitness_phase: Some("cutting".to_string()),
            measurement_conditions: Some("morning_fasted".to_string()),
            measurement_notes: Some("Pre-workout measurement from InBody scale".to_string()),
            source_device: Some("InBody H20N".to_string()),
        }],
    };

    // Create test dependencies
    let batch_processor = web::Data::new(BatchProcessor::new(pool.clone()));
    let metrics = web::Data::new(Metrics::new().expect("Failed to create metrics"));
    let validation_config = web::Data::new(ValidationConfig::default());
    let auth = create_auth_context(user_id);

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(batch_processor.clone())
            .app_data(metrics.clone())
            .app_data(validation_config.clone())
            .route("/ingest", web::post().to(ingest_body_measurements)),
    )
    .await;

    // Create request with authentication
    let req = test::TestRequest::post()
        .uri("/ingest")
        .set_json(&payload)
        .to_request();

    let req = req;
    req.extensions_mut().insert(auth);

    let response = test::call_service(&app, req).await;

    // Check response
    assert_eq!(response.status(), StatusCode::OK);

    let response_body: BodyMeasurementsIngestResponse = test::read_body_json(response).await;
    assert!(response_body.success);
    assert_eq!(response_body.processed_count, 1);
    assert_eq!(response_body.failed_count, 0);

    // Check body composition analysis
    assert!(response_body.body_composition_analysis.is_some());
    let analysis = response_body.body_composition_analysis.unwrap();
    assert!(analysis.bmi_category.is_some());
    assert!(analysis.body_fat_category.is_some());
    assert_eq!(analysis.bmi_category.unwrap(), "normal");
    assert_eq!(analysis.body_fat_category.unwrap(), "athletic"); // 12.5% is in athletic range

    // Check fitness insights
    assert!(response_body.fitness_insights.is_some());
    let insights = response_body.fitness_insights.unwrap();
    assert!(!insights.progress_indicators.is_empty());

    // Find the body fat progress indicator
    let body_fat_indicator = insights
        .progress_indicators
        .iter()
        .find(|indicator| indicator.metric == "body_fat_percentage")
        .expect("Should have body fat percentage indicator");
    assert_eq!(body_fat_indicator.value, 12.5);

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}
