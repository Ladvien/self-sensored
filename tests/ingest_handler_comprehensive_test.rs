use actix_web::{
    dev::Service,
    http::{header, StatusCode},
    test::{self, TestRequest},
    web, App,
};
use chrono::Utc;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use self_sensored::handlers::ingest::{ingest_handler, LoggedJson};
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::{
    HealthMetric, HeartRateMetric, IngestData, IngestPayload, IngestResponse,
};
use self_sensored::services::auth::{ApiKey, AuthContext, AuthService, User};

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

async fn create_test_user_and_key(pool: &PgPool) -> (User, ApiKey, String) {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let raw_key = format!("test_key_{}", Uuid::new_v4());

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
    let key_hash = AuthService::hash_api_key(&raw_key).unwrap();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at)
        VALUES ($1, $2, $3, $4, true, NOW())
        "#,
        api_key_id,
        user_id,
        &key_hash,
        "Test Key"
    )
    .execute(pool)
    .await
    .unwrap();

    let user = User {
        id: user_id,
        email: format!("test_{}@example.com", user_id),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: None,
        is_active: Some(true),
        metadata: None,
    };

    let api_key = ApiKey {
        id: api_key_id,
        user_id,
        name: Some("Test Key".to_string()),
        created_at: Some(Utc::now()),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: Some(100),
    };

    (user, api_key, raw_key)
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
async fn test_ingest_handler_success(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Create test payload
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![
                HealthMetric::HeartRate(HeartRateMetric {
                    user_id: user.id,
                    recorded_at: Utc::now(),
                    heart_rate: Some(75),
                    resting_heart_rate: Some(60),
                    heart_rate_variability: Some(45.0),
                    context: None,
                    source_device: Some("Apple Watch".to_string()),
                })
            ],
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(response_body.processed_count, 1);
    assert_eq!(response_body.failed_count, 0);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_ingest_handler_unauthorized(pool: PgPool) -> sqlx::Result<()> {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = json!({
        "data": {
            "metrics": [],
            "workouts": []
        }
    });

    // No authorization header
    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Invalid API key
    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, "Bearer invalid_key"))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[sqlx::test]
async fn test_ingest_handler_invalid_json(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Invalid JSON
    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{ invalid json }")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_ingest_handler_empty_payload(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Empty payload
    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(response_body.processed_count, 0);
    assert_eq!(response_body.failed_count, 0);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_ingest_handler_large_batch(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Create large batch of metrics
    let mut metrics = vec![];
    for i in 0..100 {
        metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now() - chrono::Duration::seconds(i),
            heart_rate: Some(60 + (i % 40) as i32),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(40.0 + (i % 20) as f64),
            context: None,
            source_device: Some("Apple Watch".to_string()),
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(response_body.processed_count, 100);
    assert_eq!(response_body.failed_count, 0);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_ingest_handler_validation_errors(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Create metrics with invalid data
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now(),
            heart_rate: Some(500), // Invalid - too high
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            context: None,
            source_device: None,
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now() - chrono::Duration::seconds(1),
            heart_rate: Some(5), // Invalid - too low
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            context: None,
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(response_body.processed_count, 0);
    assert_eq!(response_body.failed_count, 2);
    assert!(!response_body.errors.is_empty());

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[tokio::test]
async fn test_logged_json_extractor() {
    let app = test::init_service(
        App::new()
            .route("/test", web::post().to(|payload: LoggedJson<serde_json::Value>| async move {
                web::Json(payload.0)
            }))
    ).await;

    let test_data = json!({
        "key": "value",
        "number": 42
    });

    let req = TestRequest::post()
        .uri("/test")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&test_data)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(response_body["key"], "value");
    assert_eq!(response_body["number"], 42);
}

#[tokio::test]
async fn test_logged_json_extractor_invalid() {
    let app = test::init_service(
        App::new()
            .route("/test", web::post().to(|_payload: LoggedJson<serde_json::Value>| async move {
                web::Json(json!({"status": "ok"}))
            }))
    ).await;

    let req = TestRequest::post()
        .uri("/test")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{ invalid json }")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_ingest_handler_mixed_success_failure(pool: PgPool) -> sqlx::Result<()> {
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Mix of valid and invalid metrics
    let metrics = vec![
        HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now(),
            heart_rate: Some(75), // Valid
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            context: None,
            source_device: None,
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now() - chrono::Duration::seconds(1),
            heart_rate: Some(500), // Invalid - too high
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            context: None,
            source_device: None,
        }),
        HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now() - chrono::Duration::seconds(2),
            heart_rate: Some(80), // Valid
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            context: None,
            source_device: None,
        }),
    ];

    let payload = IngestPayload {
        data: IngestData {
            metrics,
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let response_body: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(response_body.processed_count, 2);
    assert_eq!(response_body.failed_count, 1);

    cleanup_test_data(&pool, user.id).await;
    Ok(())
}

#[tokio::test]
async fn test_payload_size_logging() {
    let pool = setup_test_pool().await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    // Create a large payload to test size logging
    let mut large_metrics = vec![];
    for i in 0..1000 {
        large_metrics.push(HealthMetric::HeartRate(HeartRateMetric {
            user_id: user.id,
            recorded_at: Utc::now() - chrono::Duration::seconds(i),
            heart_rate: Some(60 + (i % 40) as i32),
            resting_heart_rate: Some(55),
            heart_rate_variability: Some(40.0),
            context: None,
            source_device: Some("Apple Watch Ultra with very long device name for testing".to_string()),
        }));
    }

    let payload = IngestPayload {
        data: IngestData {
            metrics: large_metrics,
            workouts: vec![],
        },
    };

    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    cleanup_test_data(&pool, user.id).await;
}

#[tokio::test]
async fn test_content_type_headers() {
    let pool = setup_test_pool().await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    // Test different content types
    let content_types = vec![
        "application/json",
        "application/json; charset=utf-8",
        "text/json",
    ];

    for content_type in content_types {
        let req = TestRequest::post()
            .uri("/ingest")
            .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
            .insert_header((header::CONTENT_TYPE, content_type))
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    cleanup_test_data(&pool, user.id).await;
}

#[tokio::test]
async fn test_client_ip_extraction() {
    let pool = setup_test_pool().await;
    let (user, _api_key, raw_key) = create_test_user_and_key(&pool).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/ingest", web::post().to(ingest_handler))
    ).await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    };

    // Test with X-Forwarded-For header
    let req = TestRequest::post()
        .uri("/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", raw_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header(("X-Forwarded-For", "192.168.1.100"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    cleanup_test_data(&pool, user.id).await;
}