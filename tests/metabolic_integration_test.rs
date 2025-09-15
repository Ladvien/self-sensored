use actix_web::{test, web, App};
use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use self_sensored::handlers::metabolic_handler::{
    get_blood_glucose_data_handler, get_metabolic_data_handler, ingest_blood_glucose_handler,
    ingest_metabolic_handler, BloodGlucoseIngestRequest, MetabolicIngestRequest,
};
use self_sensored::middleware::metrics::Metrics;
use self_sensored::models::health_metrics::{BloodGlucoseMetric, MetabolicMetric};
use self_sensored::services::auth::AuthContext;

#[tokio::test]
async fn test_blood_glucose_ingestion_comprehensive() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "glucose_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/ingest/blood-glucose",
                web::post().to(ingest_blood_glucose_handler),
            ),
    )
    .await;

    // Test comprehensive glucose data ingestion
    let glucose_data = vec![
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::hours(2),
            blood_glucose_mg_dl: 85.0, // Normal fasting
            measurement_context: Some("fasting".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("freestyle_libre_3".to_string()),
            notes: Some("Morning fasting reading".to_string()),
            source_device: Some("iPhone 15 Pro".to_string()),
        },
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::hours(1),
            blood_glucose_mg_dl: 145.0, // Post-meal elevated
            measurement_context: Some("post_meal".to_string()),
            medication_taken: Some(true),
            insulin_delivery_units: Some(4.5), // Insulin paired with glucose
            glucose_source: Some("freestyle_libre_3".to_string()),
            notes: Some("Post-breakfast reading".to_string()),
            source_device: Some("iPhone 15 Pro".to_string()),
        },
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(30),
            blood_glucose_mg_dl: 62.0, // Hypoglycemic - critical
            measurement_context: Some("random".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("freestyle_libre_3".to_string()),
            notes: Some("Feeling dizzy".to_string()),
            source_device: Some("iPhone 15 Pro".to_string()),
        },
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(10),
            blood_glucose_mg_dl: 275.0, // Hyperglycemic - critical
            measurement_context: Some("random".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("dexcom_g7".to_string()),
            notes: Some("Very high reading".to_string()),
            source_device: Some("iPhone 15 Pro".to_string()),
        },
    ];

    let req = test::TestRequest::post()
        .uri("/ingest/blood-glucose")
        .insert_header(("content-type", "application/json"))
        .set_json(&glucose_data)
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Validate response structure
    assert_eq!(body["success"], true);
    assert_eq!(body["processed_count"], 4);
    assert_eq!(body["failed_count"], 0);

    // Validate glucose analysis
    let analysis = &body["glucose_analysis"];
    assert!(analysis["average_glucose_mg_dl"].as_f64().unwrap() > 0.0);
    assert_eq!(analysis["critical_readings"].as_array().unwrap().len(), 2); // Hypo and hyper

    // Validate time in range calculation
    let tir = analysis["time_in_range_percentage"].as_f64().unwrap();
    assert!(tir >= 0.0 && tir <= 100.0);

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_metabolic_data_ingestion_comprehensive() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "metabolic_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/ingest/metabolic",
                web::post().to(ingest_metabolic_handler),
            ),
    )
    .await;

    // Test comprehensive metabolic data ingestion
    let metabolic_data = vec![
        MetabolicIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::hours(3),
            blood_alcohol_content: Some(0.04), // Under legal limit
            insulin_delivery_units: None,
            delivery_method: None,
            source_device: Some("BACtrack Mobile Pro".to_string()),
        },
        MetabolicIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::hours(2),
            blood_alcohol_content: None,
            insulin_delivery_units: Some(12.5), // Significant insulin delivery
            delivery_method: Some("pump".to_string()),
            source_device: Some("Omnipod 5".to_string()),
        },
        MetabolicIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::hours(1),
            blood_alcohol_content: Some(0.12), // Intoxicated level
            insulin_delivery_units: None,
            delivery_method: None,
            source_device: Some("BACtrack Mobile Pro".to_string()),
        },
        MetabolicIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(30),
            blood_alcohol_content: None,
            insulin_delivery_units: Some(8.0), // Moderate insulin
            delivery_method: Some("pen".to_string()),
            source_device: Some("InPen".to_string()),
        },
    ];

    let req = test::TestRequest::post()
        .uri("/ingest/metabolic")
        .insert_header(("content-type", "application/json"))
        .set_json(&metabolic_data)
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Validate response structure
    assert_eq!(body["success"], true);
    assert_eq!(body["processed_count"], 4);
    assert_eq!(body["failed_count"], 0);

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_blood_glucose_validation_errors() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "validation_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/ingest/blood-glucose",
                web::post().to(ingest_blood_glucose_handler),
            ),
    )
    .await;

    // Test invalid glucose data
    let invalid_data = vec![
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now(),
            blood_glucose_mg_dl: 25.0, // Too low for medical devices
            measurement_context: Some("fasting".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("test_device".to_string()),
            notes: None,
            source_device: Some("Test Device".to_string()),
        },
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(1),
            blood_glucose_mg_dl: 650.0, // Too high for medical devices
            measurement_context: Some("invalid_context".to_string()), // Invalid context
            medication_taken: Some(false),
            insulin_delivery_units: Some(150.0), // Too much insulin
            glucose_source: Some("test_device".to_string()),
            notes: None,
            source_device: Some("Test Device".to_string()),
        },
    ];

    let req = test::TestRequest::post()
        .uri("/ingest/blood-glucose")
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_data)
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should have validation errors
    assert_eq!(body["success"], false);
    assert_eq!(body["processed_count"], 0);
    assert_eq!(body["failed_count"], 2);
    assert_eq!(body["errors"].as_array().unwrap().len(), 2);

    // Check error messages
    let errors = body["errors"].as_array().unwrap();
    assert!(errors[0]["message"]
        .as_str()
        .unwrap()
        .contains("outside medical range"));
    assert!(
        errors[1]["message"]
            .as_str()
            .unwrap()
            .contains("outside medical range")
            || errors[1]["message"]
                .as_str()
                .unwrap()
                .contains("Invalid measurement context")
    );

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_metabolic_data_validation_errors() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "metabolic_validation_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/ingest/metabolic",
                web::post().to(ingest_metabolic_handler),
            ),
    )
    .await;

    // Test invalid metabolic data
    let invalid_data = vec![
        MetabolicIngestRequest {
            recorded_at: Utc::now(),
            blood_alcohol_content: Some(0.8), // Lethal level - too high
            insulin_delivery_units: None,
            delivery_method: None,
            source_device: Some("Invalid Device".to_string()),
        },
        MetabolicIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(1),
            blood_alcohol_content: Some(-0.1), // Negative BAC - invalid
            insulin_delivery_units: Some(200.0), // Too much insulin
            delivery_method: Some("invalid_method".to_string()), // Invalid delivery method
            source_device: Some("Invalid Device".to_string()),
        },
    ];

    let req = test::TestRequest::post()
        .uri("/ingest/metabolic")
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_data)
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Should have validation errors
    assert_eq!(body["success"], false);
    assert_eq!(body["processed_count"], 0);
    assert_eq!(body["failed_count"], 2);

    // Check for validation error messages
    let errors = body["errors"].as_array().unwrap();
    assert!(errors.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .contains("outside valid range")));
    assert!(errors.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .contains("outside safe range")
        || e["message"].as_str().unwrap().contains("invalid")));

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_blood_glucose_data_retrieval() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "glucose_query_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert test glucose data directly
    let glucose_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO blood_glucose_metrics (
            id, user_id, recorded_at, blood_glucose_mg_dl, measurement_context,
            medication_taken, insulin_delivery_units, glucose_source, source_device
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        glucose_id,
        user_id,
        Utc::now() - chrono::Duration::hours(1),
        120.5,
        Some("post_meal".to_string()),
        Some(true),
        Some(6.0),
        Some("dexcom_g7".to_string()),
        Some("iPhone 15 Pro".to_string())
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/data/blood-glucose",
                web::get().to(get_blood_glucose_data_handler),
            ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/data/blood-glucose?limit=10")
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Validate response
    assert_eq!(body["total_count"], 1);
    let glucose_data = body["blood_glucose_data"].as_array().unwrap();
    assert_eq!(glucose_data.len(), 1);
    assert_eq!(glucose_data[0]["blood_glucose_mg_dl"], 120.5);
    assert_eq!(glucose_data[0]["measurement_context"], "post_meal");

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_metabolic_data_retrieval() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "metabolic_query_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert test metabolic data directly
    let metabolic_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO metabolic_metrics (
            id, user_id, recorded_at, blood_alcohol_content,
            insulin_delivery_units, delivery_method, source_device
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        metabolic_id,
        user_id,
        Utc::now() - chrono::Duration::hours(1),
        Some(0.05),
        Some(8.5),
        Some("pump".to_string()),
        Some("Omnipod 5".to_string())
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route("/data/metabolic", web::get().to(get_metabolic_data_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/data/metabolic?limit=10")
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Validate response
    assert_eq!(body["total_count"], 1);
    let metabolic_data = body["metabolic_data"].as_array().unwrap();
    assert_eq!(metabolic_data.len(), 1);
    assert_eq!(metabolic_data[0]["blood_alcohol_content"], 0.05);
    assert_eq!(metabolic_data[0]["insulin_delivery_units"], 8.5);
    assert_eq!(metabolic_data[0]["delivery_method"], "pump");

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

#[tokio::test]
async fn test_glucose_critical_level_detection() {
    let pool = crate::setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email) VALUES ($1, $2)",
        user_id,
        "critical_glucose_test@example.com"
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics::new()))
            .route(
                "/ingest/blood-glucose",
                web::post().to(ingest_blood_glucose_handler),
            ),
    )
    .await;

    // Test critical glucose levels
    let critical_data = vec![
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(5),
            blood_glucose_mg_dl: 45.0, // Severe hypoglycemia
            measurement_context: Some("random".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("manual_meter".to_string()),
            notes: Some("Emergency glucose reading".to_string()),
            source_device: Some("OneTouch Verio".to_string()),
        },
        BloodGlucoseIngestRequest {
            recorded_at: Utc::now() - chrono::Duration::minutes(2),
            blood_glucose_mg_dl: 450.0, // Severe hyperglycemia
            measurement_context: Some("random".to_string()),
            medication_taken: Some(false),
            insulin_delivery_units: None,
            glucose_source: Some("manual_meter".to_string()),
            notes: Some("Very high emergency reading".to_string()),
            source_device: Some("OneTouch Verio".to_string()),
        },
    ];

    let req = test::TestRequest::post()
        .uri("/ingest/blood-glucose")
        .insert_header(("content-type", "application/json"))
        .set_json(&critical_data)
        .to_request();

    // Mock authentication context
    let auth_context = AuthContext { user_id };
    let req = req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;

    // Validate critical reading detection
    assert_eq!(body["success"], true);
    assert_eq!(body["processed_count"], 2);

    let analysis = &body["glucose_analysis"];
    let critical_readings = analysis["critical_readings"].as_array().unwrap();
    assert_eq!(critical_readings.len(), 2);

    // Verify critical reading recommendations
    let severe_hypo = &critical_readings[0];
    assert_eq!(severe_hypo["severity"], "SevereHypoglycemic");
    assert!(severe_hypo["recommendation"]
        .as_str()
        .unwrap()
        .contains("Immediate treatment"));

    let severe_hyper = &critical_readings[1];
    assert_eq!(severe_hyper["severity"], "SevereHyperglycemic");
    assert!(severe_hyper["recommendation"]
        .as_str()
        .unwrap()
        .contains("immediate medical attention"));

    // Cleanup
    cleanup_test_user(&pool, user_id).await;
}

// Test utilities
async fn cleanup_test_user(pool: &sqlx::PgPool, user_id: Uuid) {
    // Cleanup in reverse dependency order
    let _ = sqlx::query!("DELETE FROM metabolic_metrics WHERE user_id = $1", user_id)
        .execute(pool)
        .await;

    let _ = sqlx::query!(
        "DELETE FROM blood_glucose_metrics WHERE user_id = $1",
        user_id
    )
    .execute(pool)
    .await;

    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await;
}
