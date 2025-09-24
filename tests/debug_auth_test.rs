use actix_web::{
    http::header,
    test::{self, TestRequest},
    web, App,
};
use chrono::Utc;
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use self_sensored::handlers::ingest::ingest_handler;
use self_sensored::middleware::auth::AuthMiddleware;
use self_sensored::models::{HealthMetric, HeartRateMetric, IngestData, IngestPayload};
use self_sensored::services::auth::AuthService;

async fn setup_test_pool() -> PgPool {
    dotenvy::dotenv().ok();
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

    println!("Creating user with ID: {}", user_id);
    println!("Creating API key: {}", api_key);

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
    let auth_service = AuthService::new(pool.clone());
    let key_hash = auth_service.hash_api_key(&api_key).unwrap();

    println!("API key hash: {}", key_hash);

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

    // Verify we can authenticate
    println!("Testing direct authentication...");
    match auth_service.authenticate(&api_key, None, None).await {
        Ok(auth) => println!("Direct auth succeeded: user_id={}", auth.user.id),
        Err(e) => println!("Direct auth failed: {:?}", e),
    }

    (user_id, api_key)
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let tables = vec!["heart_rate_metrics", "api_keys", "users"];

    for table in tables {
        let query = format!("DELETE FROM {} WHERE user_id = $1", table);
        sqlx::query(&query).bind(user_id).execute(pool).await.ok();
    }
}

#[tokio::test]
async fn debug_auth_test() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    println!("\n=== Setting up test app ===");
    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    let payload = IngestPayload {
        data: IngestData {
            metrics: vec![HealthMetric::HeartRate(HeartRateMetric {
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
            })],
            workouts: vec![],
        },
    };

    println!("\n=== Making request ===");
    println!("Authorization header: Bearer {}", api_key);

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;

    println!("Response status: {}", resp.status());

    if resp.status() != 200 {
        let body = test::read_body(resp).await;
        println!(
            "Response body: {}",
            std::str::from_utf8(&body).unwrap_or("(non-UTF8)")
        );
    } else {
        println!("Success!");
    }

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn debug_sleep_hanging_test() {
    let pool = setup_test_pool().await;
    let (user_id, api_key) = create_test_user_and_key(&pool).await;

    println!("\n=== Testing sleep metric hanging issue ===");
    let auth_service = AuthService::new(pool.clone());

    // Simplified app without middleware to isolate the hanging issue
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/v1/ingest", web::post().to(ingest_handler)),
    )
    .await;

    // Create sleep payload using the same format as the performance test
    let sleep_payload = json!({
        "data": {
            "metrics": [
                {
                    "name": "HKCategoryTypeIdentifierSleepAnalysis",
                    "data": [
                        {
                            "start": "2024-01-15 23:00:00 +0000",
                            "end": "2024-01-16 07:00:00 +0000",
                            "qty": 480.0,
                            "source": "Test"
                        }
                    ]
                }
            ]
        }
    });

    println!(
        "Sleep payload: {}",
        serde_json::to_string_pretty(&sleep_payload).unwrap()
    );

    let req = TestRequest::post()
        .uri("/v1/ingest")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&sleep_payload)
        .to_request();

    println!("Starting sleep request...");
    let start_time = std::time::Instant::now();

    let resp = test::call_service(&app, req).await;
    let duration = start_time.elapsed();

    println!("Sleep request completed in {:?}", duration);
    println!("Response status: {}", resp.status());

    if resp.status() != 200 {
        let body = test::read_body(resp).await;
        println!(
            "Response body: {}",
            std::str::from_utf8(&body).unwrap_or("(non-UTF8)")
        );
    } else {
        println!("Success!");
    }

    cleanup_test_data(&pool, user_id).await;
}
