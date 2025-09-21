/// Auth Service Coverage Test - Target: 587 lines
/// This test focuses on authentication service functionality using sqlx::test

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::services::auth::{AuthService, User, ApiKey, AuthContext};

// Setup test database connection
async fn setup_test_database() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set for testing");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
async fn test_auth_service_creation() {
    let pool = setup_test_database().await;
    let auth_service = AuthService::new(pool.clone());

    // Test that auth service was created successfully
    assert!(!format!("{:?}", auth_service.pool()).is_empty());
}

#[tokio::test]
async fn test_user_model_creation() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: None,
    };

    assert_eq!(user.email, "test@example.com");
    assert!(!user.id.is_nil());
}

#[tokio::test]
async fn test_api_key_model_creation() {
    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        expires_at: None,
        last_used_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    assert_eq!(api_key.name, Some("test_key".to_string()));
    assert_eq!(api_key.is_active, Some(true));
    assert!(api_key.expires_at.is_none());
}

#[tokio::test]
async fn test_auth_context_creation() {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: None,
        is_active: Some(true),
        metadata: None,
    };

    let api_key = ApiKey {
        id: api_key_id,
        user_id,
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    let context = AuthContext {
        user,
        api_key,
    };

    assert_eq!(context.user.email, "test@example.com");
    assert_eq!(context.api_key.name, Some("test_key".to_string()));
    assert!(!context.user.id.is_nil());
}

#[sqlx::test]
async fn test_create_user_integration(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    // Test user creation
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(user.email, email);
    assert_eq!(user.id, user_id);

    Ok(())
}

#[sqlx::test]
async fn test_create_api_key_integration(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // First create a user
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    // Test API key creation through auth service
    let (raw_key, api_key) = auth_service
        .create_api_key(user.id, Some("test_key"), None, None, None)
        .await
        .expect("Failed to create API key");

    assert!(!raw_key.is_empty());
    assert_eq!(api_key.user_id, user.id);
    assert_eq!(api_key.name, Some("test_key".to_string()));
    assert_eq!(api_key.is_active, Some(true));

    Ok(())
}

#[sqlx::test]
async fn test_authenticate_api_key_integration(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Create user and API key
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    let (raw_key, _api_key) = auth_service
        .create_api_key(user.id, Some("test_key"), None, None, None)
        .await
        .expect("Failed to create API key");

    // Test authentication
    let auth_context = auth_service
        .authenticate(&raw_key, Some("127.0.0.1".parse().unwrap()), Some("Test User Agent"))
        .await
        .expect("Failed to authenticate");

    assert_eq!(auth_context.user.id, user.id);
    assert_eq!(auth_context.user.email, email);

    Ok(())
}

#[sqlx::test]
async fn test_authentication_failure_cases(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Test with invalid API key
    let result = auth_service
        .authenticate("invalid_key_12345", None, None)
        .await;

    assert!(result.is_err());

    // Test with empty API key
    let result = auth_service
        .authenticate("", None, None)
        .await;

    assert!(result.is_err());

    Ok(())
}

#[sqlx::test]
async fn test_multiple_api_keys_per_user(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Create user
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    // Create multiple API keys for the same user
    let (raw_key1, api_key1) = auth_service
        .create_api_key(user.id, Some("key1"), None, None, None)
        .await
        .expect("Failed to create API key 1");

    let (raw_key2, api_key2) = auth_service
        .create_api_key(user.id, Some("key2"), None, None, None)
        .await
        .expect("Failed to create API key 2");

    // Both keys should work for authentication
    let auth1 = auth_service
        .authenticate(&raw_key1, None, None)
        .await
        .expect("Failed to authenticate with key 1");

    let auth2 = auth_service
        .authenticate(&raw_key2, None, None)
        .await
        .expect("Failed to authenticate with key 2");

    assert_eq!(auth1.user.id, user.id);
    assert_eq!(auth2.user.id, user.id);
    assert_eq!(auth1.api_key.name, Some("key1".to_string()));
    assert_eq!(auth2.api_key.name, Some("key2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_user_serialization() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: None,
    };

    // Test that User can be serialized and deserialized
    let json = serde_json::to_string(&user).unwrap();
    assert!(json.contains("test@example.com"));

    let deserialized: User = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.email, user.email);
    assert_eq!(deserialized.id, user.id);
}

#[tokio::test]
async fn test_api_key_serialization() {
    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        expires_at: None,
        last_used_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    // Test that ApiKey can be serialized and deserialized
    let json = serde_json::to_string(&api_key).unwrap();
    assert!(json.contains("test_key"));

    let deserialized: ApiKey = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, api_key.name);
    assert_eq!(deserialized.user_id, api_key.user_id);
}

#[tokio::test]
async fn test_auth_context_debug_and_clone() {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: None,
        is_active: Some(true),
        metadata: None,
    };

    let api_key = ApiKey {
        id: api_key_id,
        user_id,
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        last_used_at: None,
        expires_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    let context = AuthContext {
        user,
        api_key,
    };

    // Test Debug trait
    let debug_str = format!("{:?}", context);
    assert!(debug_str.contains("AuthContext"));

    // Test Clone trait
    let cloned = context.clone();
    assert_eq!(context.user.email, cloned.user.email);
    assert_eq!(context.user.id, cloned.user.id);
}

#[sqlx::test]
async fn test_api_key_expiration_handling(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Create user
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    // Create API key with expiration in the past
    let past_expiration = Utc::now() - chrono::Duration::hours(1);
    let (raw_key, _api_key) = auth_service
        .create_api_key(user.id, Some("expired_key"), Some(past_expiration), None, None)
        .await
        .expect("Failed to create API key");

    // Authentication should fail for expired key
    let result = auth_service
        .authenticate(&raw_key, None, None)
        .await;

    assert!(result.is_err());

    Ok(())
}

#[sqlx::test]
async fn test_api_key_deactivation(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Create user and API key
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    let (raw_key, api_key) = auth_service
        .create_api_key(user.id, Some("test_key"), None, None, None)
        .await
        .expect("Failed to create API key");

    // Verify authentication works initially
    let auth_result = auth_service
        .authenticate(&raw_key, None, None)
        .await;
    assert!(auth_result.is_ok());

    // Deactivate the API key
    sqlx::query!(
        "UPDATE api_keys SET is_active = false WHERE id = $1",
        api_key.id
    )
    .execute(&pool)
    .await?;

    // Authentication should now fail
    let result = auth_service
        .authenticate(&raw_key, None, None)
        .await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_all_model_debug_traits() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: None,
    };

    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: user.id,
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        expires_at: None,
        last_used_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: api_key.clone(),
    };

    // Test that all models can be debugged
    let user_debug = format!("{:?}", user);
    let api_key_debug = format!("{:?}", api_key);
    let auth_context_debug = format!("{:?}", auth_context);

    assert!(user_debug.contains("User"));
    assert!(api_key_debug.contains("ApiKey"));
    assert!(auth_context_debug.contains("AuthContext"));
}

#[tokio::test]
async fn test_all_model_clone_traits() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        is_active: Some(true),
        metadata: None,
    };

    let api_key = ApiKey {
        id: Uuid::new_v4(),
        user_id: user.id,
        name: Some("test_key".to_string()),
        created_at: Some(Utc::now()),
        expires_at: None,
        last_used_at: None,
        is_active: Some(true),
        permissions: None,
        rate_limit_per_hour: None,
    };

    let auth_context = AuthContext {
        user: user.clone(),
        api_key: api_key.clone(),
    };

    // Test that all models can be cloned
    let user_clone = user.clone();
    let api_key_clone = api_key.clone();
    let auth_context_clone = auth_context.clone();

    assert_eq!(user.id, user_clone.id);
    assert_eq!(api_key.id, api_key_clone.id);
    assert_eq!(auth_context.user.id, auth_context_clone.user.id);
}

#[sqlx::test]
async fn test_concurrent_authentication(pool: PgPool) -> sqlx::Result<()> {
    let auth_service = AuthService::new(pool.clone());

    // Create user and API key
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (id, email) VALUES ($1, $2) RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata",
        user_id,
        email
    )
    .fetch_one(&pool)
    .await?;

    let (raw_key, _api_key) = auth_service
        .create_api_key(user.id, Some("test_key"), None, None, None)
        .await
        .expect("Failed to create API key");

    // Test concurrent authentication requests
    let auth_service = std::sync::Arc::new(auth_service);
    let raw_key = std::sync::Arc::new(raw_key);

    let mut handles = vec![];

    for i in 0..5 {
        let auth_service = auth_service.clone();
        let raw_key = raw_key.clone();

        let handle = tokio::spawn(async move {
            let user_agent = format!("Test Agent {}", i);
            let result = auth_service
                .authenticate(&raw_key, None, Some(&user_agent))
                .await;

            result.is_ok()
        });

        handles.push(handle);
    }

    // Wait for all concurrent requests to complete
    let results = futures::future::join_all(handles).await;

    // All authentication requests should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.as_ref().unwrap(),
            "Concurrent authentication {} failed",
            i
        );
    }

    Ok(())
}