use actix_web::{test, web, App, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers::export::{
        export_health_data, export_heart_rate_data, export_activity_summary,
        ExportParams, ExportResponse
    },
    services::auth::{AuthContext, User as AuthUser, ApiKey as AuthApiKey},
    db::models::{User, ApiKey},
    models::ApiResponse,
};

mod common;
use common::{setup_test_db, cleanup_test_data};

/// Helper function to create test user and auth context
async fn create_test_user_and_auth(pool: &PgPool) -> (User, AuthContext) {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    
    // Create test user
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW()) RETURNING *",
        user_id,
        format!("test-user-{}@example.com", user_id)
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user");
    
    // Create test API key
    let api_key = sqlx::query_as!(
        ApiKey,
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW()) RETURNING *",
        api_key_id,
        user_id,
        "dummy_hash_for_testing",
        "Test API Key"
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test API key");
    
    let auth_context = AuthContext {
        user: AuthUser {
            id: user.id,
            email: user.email.clone(),
            created_at: user.created_at,
        },
        api_key: AuthApiKey {
            id: api_key.id,
            user_id: api_key.user_id,
            name: api_key.name.clone(),
            is_active: api_key.is_active,
            created_at: api_key.created_at,
            expires_at: api_key.expires_at,
            permissions: api_key.permissions.clone(),
            rate_limit_per_hour: api_key.rate_limit_per_hour,
        },
    };
    
    (user, auth_context)
}

/// Helper function to create test health data for export testing
async fn create_test_health_data(pool: &PgPool, user_id: Uuid) {
    let now = Utc::now();
    
    // Create heart rate data
    for i in 0..5 {
        sqlx::query!(
            "INSERT INTO heart_rate_metrics (id, user_id, recorded_at, heart_rate, resting_heart_rate, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, NOW())",
            Uuid::new_v4(),
            user_id,
            now - chrono::Duration::hours(i),
            70 + i as i32,
            60,
            "Apple Watch"
        )
        .execute(pool)
        .await
        .expect("Failed to create heart rate test data");
    }
    
    // Create blood pressure data
    for i in 0..3 {
        sqlx::query!(
            "INSERT INTO blood_pressure_metrics (id, user_id, recorded_at, systolic, diastolic, pulse, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
            Uuid::new_v4(),
            user_id,
            now - chrono::Duration::hours(i * 2),
            120 + i as i32,
            80 + i as i32,
            70 + i as i32,
            "Blood Pressure Monitor"
        )
        .execute(pool)
        .await
        .expect("Failed to create blood pressure test data");
    }
    
    // Create sleep data
    for i in 0..2 {
        let sleep_start = now - chrono::Duration::days(i + 1) + chrono::Duration::hours(22);
        let sleep_end = sleep_start + chrono::Duration::hours(8);
        
        sqlx::query!(
            "INSERT INTO sleep_metrics (id, user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, efficiency, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
            Uuid::new_v4(),
            user_id,
            sleep_start,
            sleep_end,
            480,
            120,
            100,
            95.0,
            "Apple Watch"
        )
        .execute(pool)
        .await
        .expect("Failed to create sleep test data");
    }
    
    // Create activity data
    for i in 0..4 {
        sqlx::query!(
            "INSERT INTO activity_metrics (id, user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
            Uuid::new_v4(),
            user_id,
            (now - chrono::Duration::days(i)).date_naive().and_hms_opt(12, 0, 0).unwrap().and_utc(),
            10000 + i as i32 * 1000,
            8000.0 + i as f64 * 500.0,
            400.0 + i as f64 * 50.0,
            "iPhone"
        )
        .execute(pool)
        .await
        .expect("Failed to create activity test data");
    }
    
    // Create workout data
    for i in 0..2 {
        let workout_start = now - chrono::Duration::hours(i * 6);
        let workout_end = workout_start + chrono::Duration::minutes(30);
        
        sqlx::query!(
            "INSERT INTO workouts (id, user_id, workout_type, started_at, ended_at, total_energy_kcal, distance_meters, avg_heart_rate, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
            Uuid::new_v4(),
            user_id,
            "Running",
            workout_start,
            workout_end,
            300.0 + i as f64 * 100.0,
            5000.0 + i as f64 * 1000.0,
            150 + i as i32 * 10,
            "Apple Watch"
        )
        .execute(pool)
        .await
        .expect("Failed to create workout test data");
    }
}

#[sqlx::test]
async fn test_export_health_data_json_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None, // Export all types
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Check content-type header
    assert_eq!(
        response.headers().get("content-type").unwrap().to_str().unwrap(),
        "application/json"
    );
    
    // Check content-disposition header for download
    assert!(response.headers().contains_key("content-disposition"));
    let disposition = response.headers().get("content-disposition").unwrap().to_str().unwrap();
    assert!(disposition.contains("attachment"));
    assert!(disposition.contains(".json"));
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_health_data_csv_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("csv".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Check content-type header for CSV
    assert_eq!(
        response.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/csv"
    );
    
    // Check content-disposition header for CSV download
    let disposition = response.headers().get("content-disposition").unwrap().to_str().unwrap();
    assert!(disposition.contains("attachment"));
    assert!(disposition.contains(".csv"));
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_health_data_invalid_format(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    
    let params = web::Query(ExportParams {
        format: Some("xml".to_string()), // Invalid format
        start_date: None,
        end_date: None,
        metric_types: None,
        include_raw: None,
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_health_data_filtered_metrics(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: Some("heart_rate,blood_pressure".to_string()), // Only specific types
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_health_data_date_range_filtering(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    // Export only last 2 days
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(2)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_heart_rate_data_json(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_heart_rate_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Check that it's specifically a heart rate export
    let disposition = response.headers().get("content-disposition").unwrap().to_str().unwrap();
    assert!(disposition.contains("heart_rate_export"));
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_heart_rate_data_csv(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("csv".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_heart_rate_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    assert_eq!(
        response.headers().get("content-type").unwrap().to_str().unwrap(),
        "text/csv"
    );
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_activity_summary(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: None,
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: None,
    });
    
    let result = export_activity_summary(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_no_data_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    // Don't create any test data
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200); // Should succeed even with no data
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_with_include_raw_flag(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(true), // Include raw data fields
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_default_parameters(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    // Test with minimal parameters (should use defaults)
    let params = web::Query(ExportParams {
        format: None, // Should default to JSON
        start_date: None, // Should default to last year
        end_date: None, // Should default to now
        metric_types: None, // Should include all types
        include_raw: None, // Should default to false
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_performance_large_dataset(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    
    // Create a larger dataset for performance testing
    let now = Utc::now();
    for i in 0..100 {
        sqlx::query!(
            "INSERT INTO heart_rate_metrics (id, user_id, recorded_at, heart_rate, source_device, created_at) VALUES ($1, $2, $3, $4, $5, NOW())",
            Uuid::new_v4(),
            user.id,
            now - chrono::Duration::minutes(i),
            70 + (i % 50) as i32,
            "Apple Watch"
        )
        .execute(&pool)
        .await
        .expect("Failed to create heart rate test data");
    }
    
    let start_time = std::time::Instant::now();
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(1)),
        end_date: Some(Utc::now()),
        metric_types: Some("heart_rate".to_string()),
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Export should complete within reasonable time (under 5 seconds for 100 records)
    assert!(
        elapsed.as_secs() < 5,
        "Export should complete in under 5 seconds, took {}s",
        elapsed.as_secs()
    );
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_user_isolation(pool: PgPool) {
    let (user1, auth_context1) = create_test_user_and_auth(&pool).await;
    let (user2, auth_context2) = create_test_user_and_auth(&pool).await;
    
    // Create data for user1
    create_test_health_data(&pool, user1.id).await;
    
    // User2 tries to export (should get empty results, not user1's data)
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        metric_types: None,
        include_raw: Some(false),
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context2,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // User2 should not see user1's data
    
    cleanup_test_data(&pool, user1.id).await;
    cleanup_test_data(&pool, user2.id).await;
}

#[sqlx::test]
async fn test_export_filename_format(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    create_test_health_data(&pool, user.id).await;
    
    let params = web::Query(ExportParams {
        format: Some("json".to_string()),
        start_date: None,
        end_date: None,
        metric_types: None,
        include_raw: None,
    });
    
    let result = export_health_data(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Check filename format in content-disposition header
    let disposition = response.headers().get("content-disposition").unwrap().to_str().unwrap();
    assert!(disposition.contains("health_data_export_"));
    assert!(disposition.contains(".json"));
    
    // Should contain timestamp in filename (format: YYYYMMDD_HHMMSS)
    let timestamp_pattern = std::regex::Regex::new(r"\d{8}_\d{6}").unwrap();
    assert!(timestamp_pattern.is_match(disposition));
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_export_activity_summary_calculations(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    
    // Create specific activity data for calculation testing
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO activity_metrics (id, user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, source_device, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        Uuid::new_v4(),
        user.id,
        now.date_naive().and_hms_opt(12, 0, 0).unwrap().and_utc(),
        8000, // 80% of 10k step goal
        6400.0, // 6.4km
        320.0, // 320 calories
        "iPhone"
    )
    .execute(&pool)
    .await
    .expect("Failed to create activity test data");
    
    let params = web::Query(ExportParams {
        format: None,
        start_date: Some(now - chrono::Duration::days(1)),
        end_date: Some(now + chrono::Duration::days(1)),
        metric_types: None,
        include_raw: None,
    });
    
    let result = export_activity_summary(
        web::Data::new(pool.clone()),
        auth_context,
        params,
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}
