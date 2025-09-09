use actix_web::{test, web, App};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use self_sensored::handlers::auth::{
    configure_routes, CreateApiKeyRequest, RevokeApiKeyRequest,
    CreateApiKeyResponse, ListApiKeysResponse, RevokeApiKeyResponse, RateLimitStatusResponse
};
use self_sensored::middleware::auth::{AuthContext, AuthMiddleware};
use self_sensored::services::auth::{AuthService, User, ApiKey};

async fn get_test_pool() -> sqlx::PgPool {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_user(pool: &sqlx::PgPool, user_id: Uuid) {
    // Clean up API keys first (foreign key constraint)
    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();
    
    // Clean up user
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

async fn create_test_user_and_key(pool: &sqlx::PgPool) -> (User, ApiKey, String) {
    let auth_service = AuthService::new(pool.clone());
    
    let user = auth_service
        .create_user("test_handler@example.com", Some("Test Handler User"))
        .await
        .unwrap();
    
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            "Test Handler Key",
            None,
            vec!["read".to_string(), "write".to_string()]
        )
        .await
        .unwrap();
    
    (user, api_key, plain_key)
}

#[tokio::test]
async fn test_create_api_key_handler() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());
    
    // Create test user and key
    let (user, _existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service.clone()))
            .configure(configure_routes)
    )
    .await;

    // Create auth context for the request
    let auth_context = AuthContext {
        user: user.clone(),
        api_key: _existing_key.clone(),
    };

    // Test successful API key creation
    let create_request = CreateApiKeyRequest {
        name: "New Test Key".to_string(),
        expires_at: Some(Utc::now() + Duration::days(30)),
        scopes: vec!["read".to_string()],
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&create_request)
        .to_request();

    // Manually insert auth context (simulating successful authentication)
    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: CreateApiKeyResponse = test::read_body_json(resp).await;
    assert!(response.success);
    assert!(response.api_key.is_some());
    assert!(response.key_info.is_some());
    
    let key_info = response.key_info.unwrap();
    assert_eq!(key_info.name, "New Test Key");
    assert_eq!(key_info.scopes, Some(vec!["read".to_string()]));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_create_api_key_validation_errors() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());
    
    let (user, existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service.clone()))
            .configure(configure_routes)
    )
    .await;

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: existing_key,
    };

    // Test empty name
    let create_request = CreateApiKeyRequest {
        name: "".to_string(),
        expires_at: None,
        scopes: vec!["read".to_string()],
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&create_request)
        .to_request();

    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let response: CreateApiKeyResponse = test::read_body_json(resp).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("name cannot be empty"));

    // Test empty scopes
    let create_request = CreateApiKeyRequest {
        name: "Valid Name".to_string(),
        expires_at: None,
        scopes: vec![],
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&create_request)
        .to_request();

    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let response: CreateApiKeyResponse = test::read_body_json(resp).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("At least one scope"));

    // Test invalid scope
    let create_request = CreateApiKeyRequest {
        name: "Valid Name".to_string(),
        expires_at: None,
        scopes: vec!["invalid_scope".to_string()],
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&create_request)
        .to_request();

    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let response: CreateApiKeyResponse = test::read_body_json(resp).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("Invalid scope"));

    // Test past expiration date
    let create_request = CreateApiKeyRequest {
        name: "Valid Name".to_string(),
        expires_at: Some(Utc::now() - Duration::hours(1)), // Past date
        scopes: vec!["read".to_string()],
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&create_request)
        .to_request();

    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let response: CreateApiKeyResponse = test::read_body_json(resp).await;
    assert!(!response.success);
    assert!(response.error.unwrap().contains("must be in the future"));

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_list_api_keys_handler() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());
    
    let (user, existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    // Create a second API key
    let (_second_key, _second_api_key) = auth_service
        .create_api_key(
            user.id,
            "Second Key",
            Some(Utc::now() + Duration::days(7)),
            vec!["read".to_string()]
        )
        .await
        .unwrap();
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .configure(configure_routes)
    )
    .await;

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: existing_key,
    };

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/keys")
        .to_request();

    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: ListApiKeysResponse = test::read_body_json(resp).await;
    assert!(response.success);
    assert_eq!(response.api_keys.len(), 2);

    // Verify keys are ordered by created_at DESC
    let first_key = &response.api_keys[0];
    let second_key = &response.api_keys[1];
    assert!(first_key.created_at >= second_key.created_at);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_revoke_api_key_handler() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone());
    
    let (user, existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    // Create another key to revoke
    let (_key_to_revoke, api_key_to_revoke) = auth_service
        .create_api_key(
            user.id,
            "Key to Revoke",
            None,
            vec!["read".to_string()]
        )
        .await
        .unwrap();
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .configure(configure_routes)
    )
    .await;

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: existing_key,
    };

    // Test successful revocation
    let revoke_request = RevokeApiKeyRequest {
        api_key_id: api_key_to_revoke.id,
    };

    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&revoke_request)
        .to_request();

    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: RevokeApiKeyResponse = test::read_body_json(resp).await;
    assert!(response.success);
    assert!(response.revoked);

    // Test revoking non-existent key
    let revoke_request = RevokeApiKeyRequest {
        api_key_id: Uuid::new_v4(), // Random UUID
    };

    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .set_json(&revoke_request)
        .to_request();

    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let response: RevokeApiKeyResponse = test::read_body_json(resp).await;
    assert!(!response.success);
    assert!(!response.revoked);

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limit_status_handler_disabled() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool.clone()); // No rate limiter
    
    let (user, existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .configure(configure_routes)
    )
    .await;

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: existing_key,
    };

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/rate-limit")
        .to_request();

    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: RateLimitStatusResponse = test::read_body_json(resp).await;
    assert!(response.success);
    assert!(!response.rate_limit_enabled);
    assert!(response.status.is_none());

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}

#[tokio::test]
async fn test_rate_limit_status_handler_enabled() {
    let pool = get_test_pool().await;
    
    // Create auth service with rate limiting
    let rate_limiter = self_sensored::services::rate_limiter::RateLimiter::new_in_memory(100);
    let auth_service = AuthService::new_with_rate_limiter(pool.clone(), Some(rate_limiter));
    
    let (user, existing_key, _plain_key) = create_test_user_and_key(&pool).await;
    
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(auth_service))
            .configure(configure_routes)
    )
    .await;

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: existing_key,
    };

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/rate-limit")
        .to_request();

    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: RateLimitStatusResponse = test::read_body_json(resp).await;
    assert!(response.success);
    assert!(response.rate_limit_enabled);
    assert!(response.status.is_some());

    let status = response.status.unwrap();
    assert_eq!(status.requests_limit, 100);
    assert_eq!(status.requests_remaining, 100); // No requests made yet

    // Clean up
    cleanup_test_user(&pool, user.id).await;
}