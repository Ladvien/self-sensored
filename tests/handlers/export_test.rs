use actix_web::{test, web, App};
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers, middleware::AuthMiddleware, models::ApiResponse, 
    services::auth::AuthService
};

async fn setup_test_data(pool: &PgPool) -> (Uuid, String) {
    // Create test user
    let user_id = Uuid::new_v4();
    let user_email = format!("test_export_{}@example.com", user_id);
    
    sqlx::query!(
        "INSERT INTO users (id, email, full_name, is_active) VALUES ($1, $2, $3, $4)",
        user_id,
        user_email,
        Some("Export Test User"),
        true
    )
    .execute(pool)
    .await
    .unwrap();

    // Create test API key
    let api_key_id = Uuid::new_v4();
    let api_key = "test_export_key_123456789";
    let key_hash = argon2::Argon2::default()
        .hash_password(api_key.as_bytes(), &argon2::password_hash::SaltString::generate(&mut rand::thread_rng()))
        .unwrap()
        .to_string();

    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, name, key_hash, is_active) VALUES ($1, $2, $3, $4, $5)",
        api_key_id,
        user_id,
        "Test Export Key",
        key_hash,
        true
    )
    .execute(pool)
    .await
    .unwrap();

    // Insert test heart rate data
    let test_times = [
        Utc.with_ymd_and_hms(2023, 12, 1, 10, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2023, 12, 2, 11, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2023, 12, 3, 12, 0, 0).unwrap(),
    ];

    for (i, time) in test_times.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO heart_rate_metrics (user_id, recorded_at, min_bpm, avg_bpm, max_bpm, context, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            user_id,
            time,
            Some(60i16 + i as i16 * 5),
            Some(70i16 + i as i16 * 5),
            Some(80i16 + i as i16 * 5),
            Some("resting"),
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test activity data for analytics
    for (i, time) in test_times.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO activity_metrics (user_id, date, steps, distance_meters, calories_burned, 
                                        active_minutes, flights_climbed, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            user_id,
            time.date_naive(),
            Some(8000i32 + i as i32 * 1000), // 8000, 9000, 10000 steps
            Some(6400.0f64 + i as f64 * 800.0), // Corresponding distances
            Some(1800.0f64 + i as f64 * 200.0), // Corresponding calories
            Some(45i32 + i as i32 * 15), // 45, 60, 75 active minutes
            Some(8i32 + i as i32 * 4), // 8, 12, 16 flights
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (user_id, api_key.to_string())
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id).execute(pool).await;
}

#[actix_web::test]
async fn test_export_health_data_json() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/all", web::get().to(handlers::export::export_health_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/all?format=json&start_date=2023-11-01T00:00:00Z&end_date=2023-12-31T23:59:59Z")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    // Verify export response structure
    assert_eq!(data["user_id"], user_id.to_string());
    assert_eq!(data["export_format"], "json");
    assert!(data["record_count"].as_u64().unwrap() > 0);
    assert!(data["data"].is_string());
    
    // Parse the exported data
    let exported_data: Value = serde_json::from_str(data["data"].as_str().unwrap()).unwrap();
    assert!(exported_data["heart_rate"].is_array());
    assert!(exported_data["activity"].is_array());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_health_data_csv() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/all", web::get().to(handlers::export::export_health_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/all?format=csv&metric_types=heart_rate,activity")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    assert_eq!(data["export_format"], "csv");
    let csv_data = data["data"].as_str().unwrap();
    
    // Verify CSV format
    assert!(csv_data.contains("metric_type,timestamp,value1"));
    assert!(csv_data.contains("heart_rate,"));
    assert!(csv_data.contains("activity,"));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_heart_rate_data_json() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/heart-rate", web::get().to(handlers::export::export_heart_rate_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/heart-rate?format=json")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    assert_eq!(data["export_format"], "json");
    assert_eq!(data["record_count"], 3);
    assert!(data["metric_types"].as_array().unwrap().contains(&json!("heart_rate")));
    
    // Verify the exported heart rate data
    let heart_rate_data: Value = serde_json::from_str(data["data"].as_str().unwrap()).unwrap();
    assert!(heart_rate_data.is_array());
    assert_eq!(heart_rate_data.as_array().unwrap().len(), 3);

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_heart_rate_data_csv() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/heart-rate", web::get().to(handlers::export::export_heart_rate_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/heart-rate?format=csv")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    assert_eq!(data["export_format"], "csv");
    let csv_data = data["data"].as_str().unwrap();
    
    // Verify CSV structure for heart rate
    assert!(csv_data.contains("recorded_at,min_bpm,avg_bpm,max_bpm,context,source"));
    assert!(csv_data.contains("resting"));
    
    // Count CSV rows (header + 3 data rows)
    let row_count = csv_data.lines().count();
    assert_eq!(row_count, 4); // 1 header + 3 data rows

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_activity_summary() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/activity-analytics", web::get().to(handlers::export::export_activity_summary))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/activity-analytics")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    // Verify activity analytics structure
    assert!(data.is_array());
    let analytics = data.as_array().unwrap();
    assert_eq!(analytics.len(), 3);
    
    // Verify first analytics record
    let first_day = &analytics[0];
    assert!(first_day["date"].is_string());
    assert!(first_day["steps"].is_number());
    assert!(first_day["distance_km"].is_number());
    assert!(first_day["calories"].is_number());
    assert!(first_day["step_goal_percentage"].is_number());
    
    // Verify goal percentage calculation (8000 steps = 80% of 10k goal)
    assert_eq!(first_day["step_goal_percentage"].as_f64().unwrap(), 80.0);

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_invalid_format() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/all", web::get().to(handlers::export::export_health_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/all?format=xml") // Invalid format
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(!response.success);
    assert!(response.error.unwrap().contains("Invalid format"));

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_export_unauthorized() {
    let pool = crate::test_helpers::setup_test_db().await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/all", web::get().to(handlers::export::export_health_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/export/all")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_export_specific_metric_types() {
    let pool = crate::test_helpers::setup_test_db().await;
    let (user_id, api_key) = setup_test_data(&pool).await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/export/all", web::get().to(handlers::export::export_health_data))
            )
    )
    .await;

    // Test exporting only heart rate data
    let req = test::TestRequest::get()
        .uri("/api/v1/export/all?format=json&metric_types=heart_rate")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    // Verify only heart_rate is in metric_types
    let metric_types = data["metric_types"].as_array().unwrap();
    assert_eq!(metric_types.len(), 1);
    assert_eq!(metric_types[0], "heart_rate");
    
    // Verify exported data contains only heart rate
    let exported_data: Value = serde_json::from_str(data["data"].as_str().unwrap()).unwrap();
    assert!(exported_data["heart_rate"].is_array());
    assert!(!exported_data.as_object().unwrap().contains_key("blood_pressure"));
    assert!(!exported_data.as_object().unwrap().contains_key("sleep"));

    cleanup_test_data(&pool, user_id).await;
}

#[cfg(test)]
mod test_helpers {
    use sqlx::PgPool;
    use std::env;

    pub async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/self_sensored_test".to_string());

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }
}