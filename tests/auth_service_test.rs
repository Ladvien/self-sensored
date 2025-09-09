use self_sensored::services::auth::{AuthError, AuthService};
use sqlx::postgres::PgPoolOptions;

async fn get_test_pool() -> sqlx::PgPool {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[tokio::test]
async fn test_api_key_generation() {
    let key1 = AuthService::generate_api_key();
    let key2 = AuthService::generate_api_key();

    // Keys should be different
    assert_ne!(key1, key2);

    // Keys should have correct prefix
    assert!(key1.starts_with("hea_"));
    assert!(key2.starts_with("hea_"));

    // Keys should be proper length (hea_ + 32 hex chars)
    assert_eq!(key1.len(), 36);
    assert_eq!(key2.len(), 36);
}

#[tokio::test]
async fn test_api_key_hashing_and_verification() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    let api_key = "test_api_key_12345";

    // Hash the key
    let hash = auth_service.hash_api_key(api_key).unwrap();

    // Verify correct key
    assert!(auth_service.verify_api_key(api_key, &hash).unwrap());

    // Verify incorrect key
    assert!(!auth_service.verify_api_key("wrong_key", &hash).unwrap());
}

#[tokio::test]
async fn test_create_and_authenticate_user() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Clean up any existing test user
    sqlx::query!("DELETE FROM users WHERE email = $1", "test@example.com")
        .execute(auth_service.pool())
        .await
        .unwrap();

    // Create user
    let user = auth_service
        .create_user("test@example.com", Some("Test User"))
        .await
        .unwrap();

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.full_name, Some("Test User".to_string()));
    assert_eq!(user.is_active, Some(true));

    // Create API key for user
    let (plain_key, api_key) = auth_service
        .create_api_key(user.id, "Test Key", None, vec!["read".to_string()])
        .await
        .unwrap();

    assert_eq!(api_key.name, "Test Key");
    assert_eq!(api_key.user_id, user.id);
    assert_eq!(api_key.is_active, Some(true));

    // Authenticate with the API key (with None for IP and user agent in test)
    let auth_context = auth_service.authenticate(&plain_key, None, None).await.unwrap();

    assert_eq!(auth_context.user.email, "test@example.com");
    assert_eq!(auth_context.api_key.name, "Test Key");
    assert_eq!(auth_context.api_key.scopes, Some(vec!["read".to_string()]));

    // Clean up
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(auth_service.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_invalid_api_key() {
    let pool = get_test_pool().await;
    let auth_service = AuthService::new(pool);

    // Try to authenticate with invalid key
    let result = auth_service.authenticate("invalid_key", None, None).await;

    assert!(matches!(result, Err(AuthError::InvalidApiKey)));
}
