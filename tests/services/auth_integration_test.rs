use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use self_sensored::services::auth::{AuthService, AuthError};

async fn get_test_pool() -> PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");
    sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn cleanup_test_user(pool: &PgPool, email: &str) {
    sqlx::query!("DELETE FROM users WHERE email = $1", email)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_complete_auth_flow() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "auth_flow_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Step 1: Create user
    let user = auth_service
        .create_user(test_email, Some("Auth Flow Test"))
        .await
        .unwrap();

    assert_eq!(user.email, test_email);
    assert_eq!(user.full_name, Some("Auth Flow Test".to_string()));
    assert!(user.is_active.unwrap_or(false));

    // Step 2: Create API key
    let (plain_key, api_key) = auth_service
        .create_api_key(
            user.id,
            "Test API Key",
            Some(chrono::Utc::now() + chrono::Duration::days(30)),
            vec!["read".to_string(), "write".to_string()],
        )
        .await
        .unwrap();

    assert!(plain_key.starts_with("hea_"));
    assert_eq!(api_key.name, "Test API Key");
    assert_eq!(api_key.user_id, user.id);
    assert!(api_key.is_active.unwrap_or(false));
    assert_eq!(api_key.scopes, Some(vec!["read".to_string(), "write".to_string()]));

    // Step 3: Authenticate with API key
    let auth_context = auth_service.authenticate(&plain_key).await.unwrap();

    assert_eq!(auth_context.user.id, user.id);
    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.id, api_key.id);
    assert_eq!(auth_context.api_key.name, "Test API Key");

    // Step 4: Verify last_used is updated
    let updated_key = sqlx::query!(
        "SELECT last_used FROM api_keys WHERE id = $1",
        api_key.id
    )
    .fetch_one(auth_service.pool())
    .await
    .unwrap();

    assert!(updated_key.last_used.is_some());

    // Step 5: Deactivate API key
    auth_service.deactivate_api_key(api_key.id).await.unwrap();

    // Step 6: Attempt to authenticate with deactivated key should fail
    let result = auth_service.authenticate(&plain_key).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_uuid_api_key_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "uuid_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user
    let user = auth_service
        .create_user(test_email, Some("UUID Test"))
        .await
        .unwrap();

    // Create UUID-based API key (Auto Export format)
    let uuid_key = Uuid::new_v4();
    let uuid_key_str = uuid_key.to_string();

    // Insert UUID key directly into database
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, name, key_hash, key_type, is_active, scopes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        Uuid::new_v4(),
        user.id,
        "Auto Export UUID Key",
        uuid_key_str, // For UUID keys, we store the UUID directly
        "uuid",
        true,
        Some(vec!["write".to_string()])
    )
    .execute(auth_service.pool())
    .await
    .unwrap();

    // Authenticate with UUID key
    let auth_context = auth_service.authenticate(&uuid_key_str).await.unwrap();

    assert_eq!(auth_context.user.id, user.id);
    assert_eq!(auth_context.user.email, test_email);
    assert_eq!(auth_context.api_key.name, "Auto Export UUID Key");
    assert_eq!(auth_context.api_key.key_type, Some("uuid".to_string()));

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_api_key_expiration() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "expiry_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user
    let user = auth_service
        .create_user(test_email, Some("Expiry Test"))
        .await
        .unwrap();

    // Create API key with expiration in the past
    let (plain_key, _api_key) = auth_service
        .create_api_key(
            user.id,
            "Expired Key",
            Some(chrono::Utc::now() - chrono::Duration::days(1)), // Expired yesterday
            vec!["read".to_string()],
        )
        .await
        .unwrap();

    // Authentication should fail due to expiration
    let result = auth_service.authenticate(&plain_key).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_concurrent_authentication() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "concurrent_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user and API key
    let user = auth_service
        .create_user(test_email, Some("Concurrent Test"))
        .await
        .unwrap();

    let (plain_key, _api_key) = auth_service
        .create_api_key(user.id, "Concurrent Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Run multiple concurrent authentications
    let mut handles = vec![];
    
    for i in 0..10 {
        let auth_service_clone = AuthService::new(auth_service.pool().clone());
        let key_clone = plain_key.clone();
        
        let handle = tokio::spawn(async move {
            let result = auth_service_clone.authenticate(&key_clone).await;
            (i, result)
        });
        
        handles.push(handle);
    }

    // Collect results
    let mut success_count = 0;
    for handle in handles {
        let (i, result) = handle.await.unwrap();
        match result {
            Ok(auth_context) => {
                assert_eq!(auth_context.user.email, test_email);
                success_count += 1;
            }
            Err(e) => {
                panic!("Authentication {} failed: {:?}", i, e);
            }
        }
    }

    assert_eq!(success_count, 10, "All concurrent authentications should succeed");

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_rate_limiting_per_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "rate_limit_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user and API key
    let user = auth_service
        .create_user(test_email, Some("Rate Limit Test"))
        .await
        .unwrap();

    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Rate Limited Key", None, vec!["write".to_string()])
        .await
        .unwrap();

    // This test would require implementing rate limiting in the auth service
    // For now, we'll just verify the API key works normally
    let auth_context = auth_service.authenticate(&plain_key).await.unwrap();
    assert_eq!(auth_context.api_key.id, api_key.id);

    // TODO: Implement actual rate limiting tests when rate limiter is integrated

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_audit_logging() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "audit_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user and API key
    let user = auth_service
        .create_user(test_email, Some("Audit Test"))
        .await
        .unwrap();

    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Audit Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Perform authentication (should create audit log entry)
    let _auth_context = auth_service.authenticate(&plain_key).await.unwrap();

    // Check audit log entry was created
    let audit_entries = sqlx::query!(
        "SELECT * FROM audit_log WHERE user_id = $1 AND action = 'api_key_used'",
        user.id
    )
    .fetch_all(auth_service.pool())
    .await
    .unwrap();

    assert!(!audit_entries.is_empty(), "Audit log should contain authentication entry");
    
    let audit_entry = &audit_entries[0];
    assert_eq!(audit_entry.user_id, Some(user.id));
    assert_eq!(audit_entry.action, "api_key_used");

    // Verify audit metadata contains API key ID
    if let Some(metadata) = &audit_entry.metadata {
        let meta_value: serde_json::Value = metadata.clone();
        assert_eq!(meta_value["api_key_id"], api_key.id.to_string());
    }

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_key_revocation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "revocation_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user and API key
    let user = auth_service
        .create_user(test_email, Some("Revocation Test"))
        .await
        .unwrap();

    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Revocable Key", None, vec!["write".to_string()])
        .await
        .unwrap();

    // Verify key works initially
    let _auth_context = auth_service.authenticate(&plain_key).await.unwrap();

    // Revoke the key
    auth_service.deactivate_api_key(api_key.id).await.unwrap();

    // Verify key no longer works
    let result = auth_service.authenticate(&plain_key).await;
    assert!(matches!(result, Err(AuthError::InvalidApiKey)));

    // Verify database state
    let key_state = sqlx::query!(
        "SELECT is_active FROM api_keys WHERE id = $1",
        api_key.id
    )
    .fetch_one(auth_service.pool())
    .await
    .unwrap();

    assert_eq!(key_state.is_active, Some(false));

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_scope_validation() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "scope_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user
    let user = auth_service
        .create_user(test_email, Some("Scope Test"))
        .await
        .unwrap();

    // Create read-only API key
    let (read_key, _api_key) = auth_service
        .create_api_key(user.id, "Read Only Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    // Create write-enabled API key
    let (write_key, _api_key2) = auth_service
        .create_api_key(user.id, "Write Key", None, vec!["read".to_string(), "write".to_string()])
        .await
        .unwrap();

    // Authenticate with read key
    let read_context = auth_service.authenticate(&read_key).await.unwrap();
    assert_eq!(read_context.api_key.scopes, Some(vec!["read".to_string()]));

    // Authenticate with write key
    let write_context = auth_service.authenticate(&write_key).await.unwrap();
    assert_eq!(write_context.api_key.scopes, Some(vec!["read".to_string(), "write".to_string()]));

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}

#[tokio::test]
async fn test_bulk_key_operations() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);
    
    let test_email = "bulk_test@example.com";
    cleanup_test_user(auth_service.pool(), test_email).await;

    // Create user
    let user = auth_service
        .create_user(test_email, Some("Bulk Test"))
        .await
        .unwrap();

    // Create multiple API keys
    let mut keys = Vec::new();
    for i in 0..10 {
        let (plain_key, api_key) = auth_service
            .create_api_key(
                user.id,
                &format!("Bulk Key {}", i),
                None,
                vec!["read".to_string()],
            )
            .await
            .unwrap();
        keys.push((plain_key, api_key));
    }

    // Authenticate with all keys
    for (plain_key, api_key) in &keys {
        let auth_context = auth_service.authenticate(plain_key).await.unwrap();
        assert_eq!(auth_context.api_key.id, api_key.id);
        assert_eq!(auth_context.user.id, user.id);
    }

    // Deactivate all keys
    for (_, api_key) in &keys {
        auth_service.deactivate_api_key(api_key.id).await.unwrap();
    }

    // Verify all keys are deactivated
    for (plain_key, _) in &keys {
        let result = auth_service.authenticate(plain_key).await;
        assert!(matches!(result, Err(AuthError::InvalidApiKey)));
    }

    // Cleanup
    cleanup_test_user(auth_service.pool(), test_email).await;
}