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
    let user_email = format!("test_query_{}@example.com", user_id);
    
    sqlx::query!(
        "INSERT INTO users (id, email, full_name, is_active) VALUES ($1, $2, $3, $4)",
        user_id,
        user_email,
        Some("Query Test User"),
        true
    )
    .execute(pool)
    .await
    .unwrap();

    // Create test API key
    let api_key_id = Uuid::new_v4();
    let api_key = "test_query_key_123456789";
    let key_hash = argon2::Argon2::default()
        .hash_password(api_key.as_bytes(), &argon2::password_hash::SaltString::generate(&mut rand::thread_rng()))
        .unwrap()
        .to_string();

    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, name, key_hash, is_active) VALUES ($1, $2, $3, $4, $5)",
        api_key_id,
        user_id,
        "Test Query Key",
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

    // Insert test blood pressure data
    for (i, time) in test_times.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO blood_pressure_metrics (user_id, recorded_at, systolic, diastolic, pulse, source)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            time,
            120i16 + i as i16 * 5,
            80i16 + i as i16 * 2,
            Some(75i16 + i as i16 * 3),
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test sleep data
    for (i, time) in test_times.iter().enumerate() {
        let sleep_start = *time;
        let sleep_end = sleep_start + chrono::Duration::hours(8);
        
        sqlx::query!(
            r#"
            INSERT INTO sleep_metrics (user_id, recorded_at, sleep_start, sleep_end, total_sleep_minutes, 
                                     deep_sleep_minutes, rem_sleep_minutes, awake_minutes, efficiency_percentage, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            user_id,
            time,
            sleep_start,
            sleep_end,
            480i32 - i as i32 * 10, // 8 hours minus some variation
            Some(120i32 - i as i32 * 5),
            Some(90i32 - i as i32 * 3),
            Some(30i32 + i as i32 * 5),
            Some(85.0f32 + i as f32),
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test activity data
    for (i, time) in test_times.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO activity_metrics (user_id, date, steps, distance_meters, calories_burned, 
                                        active_minutes, flights_climbed, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            user_id,
            time.date_naive(),
            Some(10000i32 + i as i32 * 1000),
            Some(8000.0f64 + i as f64 * 500.0),
            Some(2200.0f64 + i as f64 * 100.0),
            Some(60i32 + i as i32 * 10),
            Some(10i32 + i as i32 * 2),
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test workout data
    for (i, time) in test_times.iter().enumerate() {
        let workout_id = Uuid::new_v4();
        let end_time = *time + chrono::Duration::minutes(45);
        
        sqlx::query!(
            r#"
            INSERT INTO workouts (id, user_id, workout_type, start_time, end_time, total_energy_kcal,
                                distance_meters, avg_heart_rate, max_heart_rate, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            workout_id,
            user_id,
            format!("TestWorkout{}", i + 1),
            time,
            end_time,
            Some(350.0f64 + i as f64 * 50.0),
            Some(5000.0f64 + i as f64 * 500.0),
            Some(140i16 + i as i16 * 5),
            Some(160i16 + i as i16 * 5),
            Some("test")
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (user_id, api_key.to_string())
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up in reverse order of foreign key dependencies
    let _ = sqlx::query!("DELETE FROM workouts WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM activity_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM sleep_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM blood_pressure_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM heart_rate_metrics WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id).execute(pool).await;
    let _ = sqlx::query!("DELETE FROM users WHERE id = $1", user_id).execute(pool).await;
}

#[actix_web::test]
async fn test_get_heart_rate_data() {
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
                    .route("/data/heart-rate", web::get().to(handlers::query::get_heart_rate_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/heart-rate?start_date=2023-11-01T00:00:00Z&end_date=2023-12-31T23:59:59Z")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    assert!(response.data.is_some());
    
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    assert_eq!(data["data"].as_array().unwrap().len(), 3);
    
    // Verify pagination info
    let pagination = &data["pagination"];
    assert_eq!(pagination["page"], 1);
    assert_eq!(pagination["limit"], 100);
    assert_eq!(pagination["has_next"], false);
    assert_eq!(pagination["has_prev"], false);

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_heart_rate_data_with_pagination() {
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
                    .route("/data/heart-rate", web::get().to(handlers::query::get_heart_rate_data))
            )
    )
    .await;

    // Test with pagination limit
    let req = test::TestRequest::get()
        .uri("/api/v1/data/heart-rate?limit=2&page=1")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    assert_eq!(data["data"].as_array().unwrap().len(), 2);
    assert_eq!(data["pagination"]["has_next"], true);
    assert_eq!(data["pagination"]["has_prev"], false);

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_blood_pressure_data() {
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
                    .route("/data/blood-pressure", web::get().to(handlers::query::get_blood_pressure_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/blood-pressure")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    
    // Verify first record structure
    let first_record = &data["data"][0];
    assert!(first_record["systolic"].is_number());
    assert!(first_record["diastolic"].is_number());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_sleep_data() {
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
                    .route("/data/sleep", web::get().to(handlers::query::get_sleep_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/sleep?sort=asc")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    
    // Verify sleep record structure
    let first_record = &data["data"][0];
    assert!(first_record["total_sleep_minutes"].is_number());
    assert!(first_record["efficiency_percentage"].is_number());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_activity_data() {
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
                    .route("/data/activity", web::get().to(handlers::query::get_activity_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/activity")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    
    // Verify activity record structure
    let first_record = &data["data"][0];
    assert!(first_record["steps"].is_number());
    assert!(first_record["distance_meters"].is_number());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_workout_data() {
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
                    .route("/data/workouts", web::get().to(handlers::query::get_workout_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/workouts")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    assert_eq!(data["total_count"], 3);
    
    // Verify workout record structure
    let first_record = &data["data"][0];
    assert!(first_record["workout_type"].is_string());
    assert!(first_record["total_energy_kcal"].is_number());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_get_health_summary() {
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
                    .route("/data/summary", web::get().to(handlers::query::get_health_summary))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/summary?start_date=2023-11-01T00:00:00Z&end_date=2023-12-31T23:59:59Z")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let response: ApiResponse<Value> = serde_json::from_value(body).unwrap();
    
    assert!(response.success);
    let data = response.data.unwrap();
    
    // Verify summary structure
    assert!(data["user_id"].is_string());
    assert!(data["date_range"].is_object());
    assert!(data["heart_rate"].is_object());
    assert!(data["blood_pressure"].is_object());
    assert!(data["sleep"].is_object());
    assert!(data["activity"].is_object());
    assert!(data["workouts"].is_object());
    
    // Verify heart rate summary
    let hr_summary = &data["heart_rate"];
    assert_eq!(hr_summary["count"], 3);
    assert!(hr_summary["avg_resting"].is_number());

    cleanup_test_data(&pool, user_id).await;
}

#[actix_web::test]
async fn test_query_unauthorized() {
    let pool = crate::test_helpers::setup_test_db().await;

    let auth_service = AuthService::new(pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .service(
                web::scope("/api/v1")
                    .route("/data/heart-rate", web::get().to(handlers::query::get_heart_rate_data))
            )
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/data/heart-rate")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn test_query_invalid_date_range() {
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
                    .route("/data/heart-rate", web::get().to(handlers::query::get_heart_rate_data))
            )
    )
    .await;

    // Test with invalid date format
    let req = test::TestRequest::get()
        .uri("/api/v1/data/heart-rate?start_date=invalid-date")
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

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