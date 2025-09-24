use actix_web::{
    http::header,
    test::{self, TestRequest},
    web, App, HttpResponse,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use self_sensored::middleware::AuthMiddleware;
use self_sensored::services::auth::{AuthContext, AuthService};

async fn simple_handler(
    _pool: web::Data<PgPool>,
    auth: AuthContext,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user_id": auth.user.id,
        "status": "success"
    })))
}

#[tokio::test]
async fn test_middleware_auth_chain() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Create test user
    let user_id = Uuid::new_v4();
    let api_key = format!("test_key_{}", Uuid::new_v4());

    sqlx::query!(
        "INSERT INTO users (id, email, is_active, created_at) VALUES ($1, $2, true, NOW()) ON CONFLICT (id) DO NOTHING",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create API key with hash
    let auth_service = AuthService::new(pool.clone());
    let key_hash = auth_service.hash_api_key(&api_key).unwrap();

    let key_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at)
        VALUES ($1, $2, $3, $4, true, NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
        key_id,
        user_id,
        &key_hash,
        "Test Key"
    )
    .execute(&pool)
    .await
    .unwrap();

    println!("Created user {} with API key {}", user_id, api_key);
    println!("Key hash: {}", key_hash);

    // Test direct auth first
    match auth_service.authenticate(&api_key, None, None).await {
        Ok(auth) => println!("✓ Direct auth works: user_id={}", auth.user.id),
        Err(e) => panic!("✗ Direct auth failed: {:?}", e),
    }

    // Now test through middleware
    println!("\n--- Testing through middleware ---");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service))
            .wrap(AuthMiddleware)
            .route("/test", web::get().to(simple_handler)),
    )
    .await;

    // Test without auth header
    println!("Test 1: No auth header");
    let req = TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    println!("Response: {}", resp.status());
    assert_eq!(resp.status(), 401);

    // Test with auth header
    println!("\nTest 2: With auth header");
    let req = TestRequest::get()
        .uri("/test")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", api_key)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    println!("Response: {}", resp.status());

    if resp.status() != 200 {
        let body = test::read_body(resp).await;
        println!(
            "Body: {}",
            std::str::from_utf8(&body).unwrap_or("(non-UTF8)")
        );
        panic!("Expected 200, got something else");
    }

    // Cleanup
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .ok();

    println!("✓ All tests passed!");
}
