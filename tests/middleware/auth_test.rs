use actix_web::{test, web, App, HttpMessage, HttpResponse, Result};
use sqlx::PgPool;
use std::env;

use self_sensored::{
    middleware::auth::{AuthMiddleware, AuthenticatedUser},
    services::auth::AuthService,
};

async fn get_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn test_handler(_user: AuthenticatedUser) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "authenticated"})))
}

#[tokio::test]
async fn test_valid_api_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "middleware_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user("middleware_test@example.com", Some("Middleware Test"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Middleware Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "authenticated");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_missing_authorization_header() {
    let pool = get_test_pool().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_malformed_authorization_header() {
    let pool = get_test_pool().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", "InvalidFormat"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_invalid_api_key() {
    let pool = get_test_pool().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", "Bearer invalid_key_12345"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_expired_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "expired_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user
    let user = auth_service
        .create_user("expired_test@example.com", Some("Expired Test"))
        .await
        .unwrap();

    // Create expired API key
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id, 
            "Expired Key", 
            Some(chrono::Utc::now() - chrono::Duration::days(1)), // Expired yesterday
            vec!["read".to_string()]
        )
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_deactivated_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "deactivated_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user("deactivated_test@example.com", Some("Deactivated Test"))
        .await
        .unwrap();

    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Deactivated Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Deactivate the key
    auth_service.deactivate_api_key(api_key.id).await.unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", plain_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_uuid_api_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "uuid_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user
    let user = auth_service
        .create_user("uuid_test@example.com", Some("UUID Test"))
        .await
        .unwrap();

    // Create UUID-based API key directly in database
    let uuid_key = uuid::Uuid::new_v4().to_string();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, key_hash, key_type, is_active, scopes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        uuid::Uuid::new_v4(),
        user.id,
        "UUID Test Key",
        uuid_key,
        "uuid",
        true,
        Some(vec!["write".to_string()])
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/test")
        .insert_header(("Authorization", format!("Bearer {}", uuid_key)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_concurrent_authentication_requests() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "concurrent_auth_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user("concurrent_auth_test@example.com", Some("Concurrent Auth Test"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Concurrent Auth Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Run multiple concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let app_clone = app.clone();
        let key_clone = plain_key.clone();
        
        let handle = tokio::spawn(async move {
            let req = test::TestRequest::get()
                .uri("/test")
                .insert_header(("Authorization", format!("Bearer {}", key_clone)))
                .to_request();

            let resp = test::call_service(&app_clone, req).await;
            (i, resp.status().is_success())
        });
        
        handles.push(handle);
    }

    // Collect results
    let mut success_count = 0;
    for handle in handles {
        let (i, is_success) = handle.await.unwrap();
        if is_success {
            success_count += 1;
        } else {
            println!("Request {} failed", i);
        }
    }

    assert_eq!(success_count, 10, "All concurrent authentication requests should succeed");

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_authentication_performance() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());

    // Clean up existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "perf_test@example.com")
        .execute(&pool)
        .await
        .unwrap();

    // Create test user and API key
    let user = auth_service
        .create_user("perf_test@example.com", Some("Performance Test"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Performance Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(AuthMiddleware::new(pool.clone()))
            .route("/test", web::get().to(test_handler)),
    )
    .await;

    // Measure authentication performance
    let start = std::time::Instant::now();
    
    for _ in 0..100 {
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Authorization", format!("Bearer {}", plain_key)))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
    
    let duration = start.elapsed();
    let avg_per_request = duration.as_millis() / 100;

    // Performance requirement: authentication should take < 10ms per request
    assert!(avg_per_request < 10, 
           "Authentication took {}ms per request, expected < 10ms", avg_per_request);

    println!("✓ Authentication performance: {}ms average per request", avg_per_request);

    // Cleanup
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&pool)
        .await
        .unwrap();
}