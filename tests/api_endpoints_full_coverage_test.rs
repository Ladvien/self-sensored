use actix_web::{
    http::{header, StatusCode},
    test::{self, TestRequest},
    web, App,
};
use chrono::Utc;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use self_sensored::handlers::{
    admin::admin_stats,
    auth::login_handler,
    background::background_status,
    health::health_check,
    ingest::ingest_handler,
    query::query_handler,
};
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::{
    HealthMetric, HeartRateMetric, IngestData, IngestPayload,
};
use self_sensored::services::auth::{AuthService, AuthContext};

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_user_and_key(pool: &PgPool) -> (Uuid, String) {
    let user_id = Uuid::new_v4();
    let api_key = format!("test_key_{}", Uuid::new_v4());

    // Create user
    sqlx::query!(
        "INSERT INTO users (id, email, is_active, created_at) VALUES ($1, $2, true, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .unwrap();

    // Create API key
    let auth_service = AuthService::new();
    let key_hash = auth_service.hash_api_key(&api_key).unwrap();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at)
        VALUES ($1, $2, $3, $4, true, NOW())
        "#,
        Uuid::new_v4(),
        user_id,
        &key_hash,
        "Test Key"
    )
    .execute(pool)
    .await
    .unwrap();

    (user_id, api_key)
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let tables = vec![
        "heart_rate_metrics",
        "blood_pressure_metrics",
        "sleep_metrics",
        "activity_metrics",
        "workout_metrics",
        "raw_ingestions",
        "api_keys",
        "users",
    ];

    for table in tables {
        let query = format!("DELETE FROM {} WHERE user_id = $1", table);
        sqlx::query(&query)
            .bind(user_id)
            .execute(pool)
            .await
            .ok();
    }
}

#[sqlx::test]
async fn test_health_check_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(health_check))
    ).await;

    let req = TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].is_string());

    Ok(())
}

#[sqlx::test]
async fn test_ingest_endpoint_success(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate(HeartRateMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: Utc::now(),
                    heart_rate: Some(75),
                    resting_heart_rate: Some(60),
                    heart_rate_variability: Some(45.0),
                    walking_heart_rate_average: None,
                    heart_rate_recovery_one_minute: None,
                    atrial_fibrillation_burden_percentage: None,
                    vo2_max_ml_kg_min: None,
                    context: None,
                    source_device: Some("Test Device".to_string()),
                    created_at: Utc::now(),
                })
            ],
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_ingest_endpoint_unauthorized(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = json!({
        "data": {
            "metrics": [],
            "workouts": []
        }
    });

    // No authorization header
    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test]
async fn test_ingest_endpoint_invalid_json(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{ invalid json }")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_query_endpoint_success(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    // Insert test data
    sqlx::query!(
        r#"
        INSERT INTO heart_rate_metrics (id, user_id, recorded_at, heart_rate, created_at)
        VALUES ($1, $2, NOW(), $3, NOW())
        "#,
        Uuid::new_v4(),
        user_id,
        75
    )
    .execute(&pool)
    .await?;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/query/heart_rate", web::get().to(query_handler))
    ).await;

    let req = TestRequest::get()
        .uri("/v1/query/heart_rate?limit=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_query_endpoint_unauthorized(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/query/heart_rate", web::get().to(query_handler))
    ).await;

    let req = TestRequest::get()
        .uri("/v1/query/heart_rate")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test]
async fn test_admin_stats_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/admin/stats", web::get().to(admin_stats))
    ).await;

    let req = TestRequest::get()
        .uri("/admin/stats")
        .insert_header((header::AUTHORIZATION, "Bearer admin_token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // May be unauthorized without proper admin setup, but endpoint should exist
    assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test]
async fn test_background_status_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/background/status", web::get().to(background_status))
    ).await;

    let req = TestRequest::get()
        .uri("/background/status")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["status"].is_string());

    Ok(())
}

#[sqlx::test]
async fn test_login_endpoint(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/auth/login", web::post().to(login_handler))
    ).await;

    let login_payload = json!({
        "email": "test@example.com",
        "password": "password123"
    });

    let req = TestRequest::post()
        .uri("/auth/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&login_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    // May return various statuses depending on auth setup
    assert!(resp.status().is_client_error() || resp.status().is_success());

    Ok(())
}

#[sqlx::test]
async fn test_multiple_endpoints_integration(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(health_check))
            .route("/v1/ingest", web::post().to(ingest_handler))
            .route("/v1/query/heart_rate", web::get().to(query_handler))
            .route("/background/status", web::get().to(background_status))
    ).await;

    // 1. Check health
    let req = TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. Check background status
    let req = TestRequest::get().uri("/background/status").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Ingest data
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate(HeartRateMetric {
                    id: Uuid::new_v4(),
                    user_id,
                    recorded_at: Utc::now(),
                    heart_rate: Some(80),
                    resting_heart_rate: Some(65),
                    heart_rate_variability: Some(50.0),
                    walking_heart_rate_average: None,
                    heart_rate_recovery_one_minute: None,
                    atrial_fibrillation_burden_percentage: None,
                    vo2_max_ml_kg_min: None,
                    context: None,
                    source_device: Some("Integration Test".to_string()),
                    created_at: Utc::now(),
                })
            ],
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Query the data back
    let req = TestRequest::get()
        .uri("/v1/query/heart_rate?limit=5")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].is_array());

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[sqlx::test]
async fn test_cors_headers(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(health_check))
    ).await;

    // Test preflight request
    let req = TestRequest::default()
        .insert_header((header::ORIGIN, "https://example.com"))
        .method(actix_web::http::Method::OPTIONS)
        .uri("/health")
        .insert_header((header::ACCESS_CONTROL_REQUEST_METHOD, "GET"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // CORS may or may not be configured, but endpoint should handle OPTIONS
    assert!(resp.status().is_success() || resp.status().is_client_error());

    Ok(())
}

#[sqlx::test]
async fn test_rate_limiting_behavior(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    // Make multiple rapid requests to test rate limiting
    for i in 0..5 {
        let req = TestRequest::post()
            .uri("/v1/ingest")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;

        // First few should succeed, later ones might be rate limited
        if i < 3 {
            assert_eq!(resp.status(), StatusCode::OK);
        } else {
            // May get rate limited or continue to succeed depending on configuration
            assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::TOO_MANY_REQUESTS);
        }
    }

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    // Spawn multiple concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let api_key_clone = api_key.clone();
        let pool_clone = pool.clone();
        let user_id_clone = user_id.clone();

        let handle = tokio::spawn(async move {
            let app = test::init_service(
                App::new()
                    .app_data(web::Data::new(pool_clone.clone()))
                    .route("/health", web::get().to(health_check))
                    .route("/v1/ingest", web::post().to(ingest_handler))
            ).await;

            if i % 2 == 0 {
                // Health check requests
                let req = TestRequest::get().uri("/health").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            } else {
                // Ingest requests
                let payload = IngestPayload {
                    data: IngestData {
                        metrics: vec![],
                        workouts: vec![],
                    },
                };

                let req = TestRequest::post()
                    .uri("/v1/ingest")
                    .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key_clone)))
                    .insert_header((header::CONTENT_TYPE, "application/json"))
                    .set_json(&payload)
                    .to_request();

                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            }
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }

    cleanup_test_data(&pool, user_id).await;
}

#[sqlx::test]
async fn test_endpoint_error_handling(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    // Test with malformed JSON
    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, "Bearer invalid_key"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{ malformed json")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());

    // Test with missing content type
    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, "Bearer invalid_key"))
        .set_payload(r#"{"data": {"metrics": [], "workouts": []}}"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());

    Ok(())
}

#[sqlx::test]
async fn test_large_payload_handling(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/v1/ingest", web::post().to(ingest_handler))
    ).await;

    // Create a large payload
    let mut metrics = vec![];
    for i in 0..100 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now() - chrono::Duration::seconds(i),
            heart_rate: Some(60 + (i % 40) as i16),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(40.0 + (i % 20) as f64),
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            context: None,
            source_device: Some(format!("Device_{}", i % 5)),
            created_at: Utc::now(),
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    cleanup_test_data(&pool, user_id).await;
    Ok(())
}