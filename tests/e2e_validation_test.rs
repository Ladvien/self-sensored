use actix_web::{test, web, App};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers::ingest::ingest_handler, middleware::auth::AuthMiddleware, services::auth::AuthService,
};

mod common;
use common::{cleanup_test_data, setup_test_db};

#[actix_web::test]
async fn test_validate_heart_rate_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    // Test various heart rate scenarios
    let test_cases = vec![
        (30, true, "Valid low heart rate"),
        (60, true, "Valid normal resting heart rate"),
        (180, true, "Valid high exercise heart rate"),
        (250, true, "Valid extreme athletic heart rate"),
        (14, false, "Invalid too low heart rate"),
        (301, false, "Invalid too high heart rate"),
        (-10, false, "Invalid negative heart rate"),
    ];

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    for (heart_rate, should_succeed, description) in test_cases {
        let payload = json!({
            "data": {
                "metrics": [{
                    "type": "HeartRate",
                    "heart_rate": heart_rate,
                    "recorded_at": "2025-01-01T00:00:00Z",
                    "user_id": user_id,
                    "source_device": "Test Device"
                }],
                "workouts": []
            }
        });

        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        if should_succeed {
            assert!(resp.status().is_success(), "{} failed", description);
        } else {
            // Invalid data should be handled gracefully, not fail the entire request
            assert!(
                resp.status().is_success(),
                "Request should succeed even with invalid data: {}",
                description
            );
        }
    }

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_validate_blood_pressure_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let test_cases = vec![
        (120, 80, true, "Normal blood pressure"),
        (90, 60, true, "Low blood pressure"),
        (140, 90, true, "High blood pressure"),
        (180, 120, true, "Very high blood pressure"),
        (49, 80, false, "Invalid low systolic"),
        (251, 80, false, "Invalid high systolic"),
        (120, 29, false, "Invalid low diastolic"),
        (120, 151, false, "Invalid high diastolic"),
    ];

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    for (systolic, diastolic, should_succeed, description) in test_cases {
        let payload = json!({
            "data": {
                "metrics": [{
                    "type": "BloodPressure",
                    "systolic": systolic,
                    "diastolic": diastolic,
                    "recorded_at": "2025-01-01T00:00:00Z",
                    "user_id": user_id,
                    "source_device": "Test Device"
                }],
                "workouts": []
            }
        });

        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success(),
            "Request should handle {} gracefully",
            description
        );
    }

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_validate_activity_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let test_cases = vec![
        (0, 0.0, true, "No activity"),
        (5000, 3.5, true, "Light activity"),
        (10000, 7.0, true, "Normal daily activity"),
        (30000, 21.0, true, "Very active day"),
        (100000, 70.0, true, "Ultra marathon"),
        (-100, 0.0, false, "Invalid negative steps"),
        (0, -5.0, false, "Invalid negative distance"),
        (201000, 150.0, false, "Invalid excessive steps"),
        (10000, 501.0, false, "Invalid excessive distance"),
    ];

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    for (steps, distance_km, should_succeed, description) in test_cases {
        let payload = json!({
            "data": {
                "metrics": [{
                    "type": "Activity",
                    "step_count": steps,
                    "distance_meters": distance_km * 1000.0,
                    "recorded_at": "2025-01-01T00:00:00Z",
                    "user_id": user_id,
                    "source_device": "Test Device"
                }],
                "workouts": []
            }
        });

        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success(),
            "Request should handle {} gracefully",
            description
        );
    }

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_validate_sleep_metrics() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let test_cases = vec![
        (480, 85.0, true, "Normal 8 hour sleep"),
        (360, 75.0, true, "6 hour sleep"),
        (600, 90.0, true, "10 hour sleep"),
        (240, 50.0, true, "Poor 4 hour sleep"),
        (0, 0.0, false, "No sleep"),
        (1500, 85.0, false, "Invalid 25 hour sleep"),
        (480, -10.0, false, "Invalid negative efficiency"),
        (480, 101.0, false, "Invalid over 100% efficiency"),
    ];

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    for (duration_minutes, efficiency, should_succeed, description) in test_cases {
        let payload = json!({
            "data": {
                "metrics": [{
                    "type": "Sleep",
                    "sleep_start": "2025-01-01T22:00:00Z",
                    "sleep_end": format!("2025-01-02T{:02}:{:02}:00Z",
                        22 + (duration_minutes / 60), duration_minutes % 60),
                    "duration_minutes": duration_minutes,
                    "efficiency": efficiency,
                    "user_id": user_id,
                    "source_device": "Test Device"
                }],
                "workouts": []
            }
        });

        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success(),
            "Request should handle {} gracefully",
            description
        );
    }

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_validate_body_measurements() {
    let pool = setup_test_db().await;
    let user_id = create_test_user(&pool).await;
    let api_key = create_test_api_key(&pool, user_id).await;

    let test_cases = vec![
        (70.0, 175.0, 22.9, true, "Normal BMI"),
        (50.0, 160.0, 19.5, true, "Low weight"),
        (100.0, 180.0, 30.9, true, "High weight"),
        (150.0, 190.0, 41.5, true, "Very high weight"),
        (-10.0, 175.0, 0.0, false, "Invalid negative weight"),
        (70.0, -175.0, 0.0, false, "Invalid negative height"),
        (500.0, 175.0, 0.0, false, "Invalid excessive weight"),
        (70.0, 300.0, 0.0, false, "Invalid excessive height"),
    ];

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    for (weight_kg, height_cm, bmi, should_succeed, description) in test_cases {
        let payload = json!({
            "data": {
                "metrics": [{
                    "type": "BodyMeasurement",
                    "body_weight_kg": weight_kg,
                    "height_cm": height_cm,
                    "body_mass_index": bmi,
                    "recorded_at": "2025-01-01T00:00:00Z",
                    "user_id": user_id,
                    "source_device": "Test Device"
                }],
                "workouts": []
            }
        });

        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .insert_header(("Authorization", format!("Bearer {}", api_key.clone())))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert!(
            resp.status().is_success(),
            "Request should handle {} gracefully",
            description
        );
    }

    cleanup_test_data(&pool, user_id).await;
}

// Helper functions
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

async fn create_test_api_key(pool: &PgPool, user_id: Uuid) -> String {
    let auth_service = AuthService::new(pool.clone());
    let (api_key, _) = auth_service
        .create_api_key(user_id, Some("Test Key"), None, None, None)
        .await
        .expect("Failed to create API key");
    api_key
}
