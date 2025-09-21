use actix_web::{test, web, App, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers::auth::{
        create_api_key, list_api_keys, revoke_api_key, get_rate_limit_status,
        CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysResponse, 
        RevokeApiKeyRequest, RevokeApiKeyResponse, RateLimitStatusResponse
    },
    services::auth::{AuthContext, User as AuthUser, ApiKey as AuthApiKey, AuthService},
    db::models::{User, ApiKey},
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

/// Helper function to create AuthService instance
fn create_auth_service(pool: &PgPool) -> AuthService {
    AuthService::new(
        pool.clone(),
        None, // Redis connection manager - None for tests
        false, // Rate limiting disabled for tests
    )
}

#[sqlx::test]
async fn test_create_api_key_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        expires_at: Some(Utc::now() + chrono::Duration::days(30)),
        permissions: Some(json!({"ingest": true, "export": true})),
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context.clone(),
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 201);
    
    // Verify API key was created in database
    let api_key_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM api_keys WHERE user_id = $1 AND name = $2",
        user.id,
        "Test API Key"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert_eq!(api_key_count, 2); // Original + new one
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_create_api_key_empty_name_error(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = CreateApiKeyRequest {
        name: "".to_string(), // Empty name should fail
        expires_at: None,
        permissions: None,
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_create_api_key_name_too_long_error(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = CreateApiKeyRequest {
        name: "a".repeat(101), // 101 characters - too long
        expires_at: None,
        permissions: None,
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_create_api_key_past_expiration_error(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        expires_at: Some(Utc::now() - chrono::Duration::days(1)), // Past date
        permissions: None,
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_create_api_key_invalid_permissions_error(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        expires_at: None,
        permissions: Some(json!("invalid_string")), // Should be object/array/null
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 400);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_list_api_keys_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    // Create additional API keys for the user
    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW())",
        Uuid::new_v4(),
        user.id,
        "hash2",
        "Second Key"
    )
    .execute(&pool)
    .await
    .expect("Failed to create second API key");
    
    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, false, NOW())",
        Uuid::new_v4(),
        user.id,
        "hash3",
        "Inactive Key"
    )
    .execute(&pool)
    .await
    .expect("Failed to create inactive API key");
    
    let result = list_api_keys(
        auth_context,
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_revoke_api_key_success(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    // Create an API key to revoke
    let key_to_revoke_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW())",
        key_to_revoke_id,
        user.id,
        "hash_to_revoke",
        "Key to Revoke"
    )
    .execute(&pool)
    .await
    .expect("Failed to create API key to revoke");
    
    let request = RevokeApiKeyRequest {
        api_key_id: key_to_revoke_id,
    };
    
    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = revoke_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    // Verify API key was deactivated
    let is_active = sqlx::query_scalar!(
        "SELECT is_active FROM api_keys WHERE id = $1",
        key_to_revoke_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert!(!is_active.unwrap_or(true), "API key should be deactivated");
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_revoke_api_key_not_found(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let request = RevokeApiKeyRequest {
        api_key_id: Uuid::new_v4(), // Non-existent key
    };
    
    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = revoke_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 404);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_revoke_api_key_different_user(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    // Create another user and their API key
    let other_user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        other_user_id,
        format!("other-user-{}@example.com", other_user_id)
    )
    .execute(&pool)
    .await
    .expect("Failed to create other user");
    
    let other_user_key_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO api_keys (id, user_id, key_hash, name, is_active, created_at) VALUES ($1, $2, $3, $4, true, NOW())",
        other_user_key_id,
        other_user_id,
        "other_user_hash",
        "Other User Key"
    )
    .execute(&pool)
    .await
    .expect("Failed to create other user's API key");
    
    let request = RevokeApiKeyRequest {
        api_key_id: other_user_key_id, // Try to revoke another user's key
    };
    
    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = revoke_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 404); // Should not find key belonging to different user
    
    // Verify other user's key is still active
    let is_active = sqlx::query_scalar!(
        "SELECT is_active FROM api_keys WHERE id = $1",
        other_user_key_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert!(is_active.unwrap_or(false), "Other user's API key should remain active");
    
    // Cleanup both users
    cleanup_test_data(&pool, user.id).await;
    cleanup_test_data(&pool, other_user_id).await;
}

#[sqlx::test]
async fn test_get_rate_limit_status_disabled(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool); // Rate limiting disabled by default
    
    let result = get_rate_limit_status(
        auth_context,
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_create_api_key_with_custom_permissions(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    let custom_permissions = json!({
        "ingest": true,
        "export": false,
        "admin": false,
        "read_only": true
    });
    
    let request = CreateApiKeyRequest {
        name: "Custom Permissions Key".to_string(),
        expires_at: None,
        permissions: Some(custom_permissions.clone()),
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 201);
    
    // Verify permissions were stored correctly
    let stored_permissions = sqlx::query_scalar!(
        "SELECT permissions FROM api_keys WHERE user_id = $1 AND name = $2",
        user.id,
        "Custom Permissions Key"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert!(stored_permissions.is_some());
    assert_eq!(stored_permissions.unwrap(), custom_permissions);
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_audit_logging_for_api_key_operations(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    // Create API key (should generate audit log)
    let request = CreateApiKeyRequest {
        name: "Audit Test Key".to_string(),
        expires_at: None,
        permissions: None,
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let _result = create_api_key(
        req,
        auth_context.clone(),
        web::Json(request),
        web::Data::new(auth_service.clone()),
    )
    .await;
    
    // Give a moment for audit log to be written
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Check audit log was created
    let audit_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM audit_log WHERE user_id = $1 AND action = 'api_key_created'",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    
    assert!(audit_count > 0, "Should have created audit log entry for API key creation");
    
    cleanup_test_data(&pool, user.id).await;
}

#[sqlx::test]
async fn test_api_key_expiration_handling(pool: PgPool) {
    let (user, auth_context) = create_test_user_and_auth(&pool).await;
    let auth_service = create_auth_service(&pool);
    
    // Create API key that expires in 1 minute
    let expiration = Utc::now() + chrono::Duration::minutes(1);
    let request = CreateApiKeyRequest {
        name: "Short Lived Key".to_string(),
        expires_at: Some(expiration),
        permissions: None,
    };
    
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/keys")
        .insert_header(("content-type", "application/json"))
        .to_http_request();
    
    let result = create_api_key(
        req,
        auth_context,
        web::Json(request),
        web::Data::new(auth_service),
    )
    .await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 201);
    
    // Verify expiration was stored correctly
    let stored_expiration = sqlx::query_scalar!(
        "SELECT expires_at FROM api_keys WHERE user_id = $1 AND name = $2",
        user.id,
        "Short Lived Key"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    
    assert!(stored_expiration.is_some());
    let stored_exp = stored_expiration.unwrap();
    assert!(
        (stored_exp - expiration).num_seconds().abs() < 2,
        "Stored expiration should match requested expiration"
    );
    
    cleanup_test_data(&pool, user.id).await;
}
