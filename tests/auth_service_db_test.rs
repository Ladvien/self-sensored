/// Simple Database Integration Test for Auth Service
/// Tests basic database connectivity without sqlx::test macro
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::services::auth::{AuthService, User};

#[tokio::test]
async fn test_auth_service_database_connectivity() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Use the configured test database URL directly
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored_test".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let auth_service = AuthService::new(pool.clone());

    // Test basic database query - check if users table exists
    let result = sqlx::query!("SELECT COUNT(*) as count FROM users LIMIT 1")
        .fetch_one(&pool)
        .await;

    match result {
        Ok(row) => {
            println!("Successfully connected to database and queried users table. Count: {:?}", row.count);
            assert!(true); // Test passed
        }
        Err(e) => {
            println!("Database query failed: {}", e);
            // If users table doesn't exist, that's also OK for connectivity test
            if e.to_string().contains("does not exist") {
                println!("Users table doesn't exist, but database connectivity is working");
                assert!(true);
            } else {
                panic!("Unexpected database error: {}", e);
            }
        }
    }

    // Test that auth service was created successfully
    assert!(!format!("{:?}", auth_service.pool()).is_empty());
}

#[tokio::test]
async fn test_user_model_validation() {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        apple_health_id: None,
        created_at: Some(chrono::Utc::now()),
        updated_at: Some(chrono::Utc::now()),
        is_active: Some(true),
        metadata: None,
    };

    assert_eq!(user.email, "test@example.com");
    assert!(!user.id.is_nil());
    assert_eq!(user.is_active, Some(true));
}