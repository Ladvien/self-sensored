use actix_web::{http::StatusCode, test, web, App};
use uuid::Uuid;

#[path = "../tests/common/mod.rs"]
mod common;
use common::{cleanup_test_db, setup_test_db};

#[actix_web::test]
async fn test_health_endpoint() {
    // Simple health check test
    let pool = setup_test_db().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(|| async { "OK" })),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_database_connection() {
    let pool = setup_test_db().await;

    // Test we can query the database
    let result = sqlx::query!("SELECT 1 as one").fetch_one(&pool).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().one, Some(1));
}

#[actix_web::test]
async fn test_create_and_delete_user() {
    let pool = setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create user
    let create_result = sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(&pool)
    .await;

    assert!(create_result.is_ok());

    // Verify user exists
    let user = sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
        .fetch_optional(&pool)
        .await
        .unwrap();

    assert!(user.is_some());

    // Cleanup
    cleanup_test_db(&pool, user_id).await;

    // Verify user is deleted
    let deleted_user = sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
        .fetch_optional(&pool)
        .await
        .unwrap();

    assert!(deleted_user.is_none());
}

#[actix_web::test]
async fn test_insert_heart_rate_metric() {
    let pool = setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create user first
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(&pool)
    .await
    .expect("Failed to create test user");

    // Insert heart rate metric
    let result = sqlx::query!(
        "INSERT INTO heart_rate_metrics (
            user_id, recorded_at, heart_rate, source_device
        ) VALUES ($1, $2, $3, $4)",
        user_id,
        chrono::Utc::now(),
        70i32,
        "Test Device"
    )
    .execute(&pool)
    .await;

    assert!(result.is_ok());

    // Verify metric was inserted
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count.count.unwrap_or(0), 1);

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}

#[actix_web::test]
async fn test_load_and_process_fixture() {
    let pool = setup_test_db().await;
    let user_id = Uuid::new_v4();

    // Create user
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(&pool)
    .await
    .expect("Failed to create test user");

    // Load test fixture
    let fixture_path = "tests/fixtures/test_fixture_small.json";
    let fixture_content = tokio::fs::read_to_string(fixture_path)
        .await
        .expect("Failed to read fixture");

    let fixture: serde_json::Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse fixture");

    // Process metrics from fixture
    if let Some(metrics) = fixture["data"]["metrics"].as_array() {
        println!("Processing {} metrics from fixture", metrics.len());

        for metric in metrics.iter().take(5) {
            // Process first 5 metrics only
            if let Some(metric_type) = metric["type"].as_str() {
                match metric_type {
                    "HeartRate" => {
                        if let (Some(heart_rate), Some(recorded_at)) = (
                            metric["heart_rate"].as_i64(),
                            metric["recorded_at"].as_str(),
                        ) {
                            let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now());

                            let result = sqlx::query!(
                                "INSERT INTO heart_rate_metrics (
                                    user_id, recorded_at, heart_rate, source_device
                                ) VALUES ($1, $2, $3, $4)
                                ON CONFLICT (user_id, recorded_at) DO NOTHING",
                                user_id,
                                recorded_at,
                                heart_rate as i32,
                                "Test Device"
                            )
                            .execute(&pool)
                            .await;

                            if result.is_err() {
                                println!("Failed to insert heart rate metric: {:?}", result.err());
                            }
                        }
                    }
                    "Activity" => {
                        if let Some(recorded_at) = metric["recorded_at"].as_str() {
                            let recorded_at = chrono::DateTime::parse_from_rfc3339(recorded_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now());

                            let step_count = metric["step_count"].as_i64().unwrap_or(0) as i32;
                            let distance_meters = metric["distance_meters"].as_f64();

                            let result = sqlx::query!(
                                "INSERT INTO activity_metrics (
                                    user_id, recorded_at, step_count, distance_meters, source_device
                                ) VALUES ($1, $2, $3, $4, $5)
                                ON CONFLICT (user_id, recorded_at) DO NOTHING",
                                user_id,
                                recorded_at,
                                step_count,
                                distance_meters,
                                "Test Device"
                            )
                            .execute(&pool)
                            .await;

                            if result.is_err() {
                                println!("Failed to insert activity metric: {:?}", result.err());
                            }
                        }
                    }
                    _ => {
                        println!("Skipping metric type: {}", metric_type);
                    }
                }
            }
        }
    }

    // Verify some metrics were inserted
    let heart_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM heart_rate_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let activity_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM activity_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let total_count = heart_count.count.unwrap_or(0) + activity_count.count.unwrap_or(0);

    println!(
        "Inserted {} heart rate metrics",
        heart_count.count.unwrap_or(0)
    );
    println!(
        "Inserted {} activity metrics",
        activity_count.count.unwrap_or(0)
    );

    assert!(total_count > 0, "Should have inserted at least one metric");

    // Cleanup
    cleanup_test_db(&pool, user_id).await;
}
