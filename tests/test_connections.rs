use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn test_database_connection() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Test basic query
    let result: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await
        .expect("Failed to execute test query");

    assert!(result.0.contains("PostgreSQL"));
    println!("Database connection successful: {}", result.0);
}

#[tokio::test]
async fn test_test_database_connection() {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Test basic query
    let result: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&pool)
        .await
        .expect("Failed to execute test query");

    assert!(result.0.contains("PostgreSQL"));
    println!("Test database connection successful: {}", result.0);
}

#[tokio::test]
async fn test_redis_connection() {
    dotenv::dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    let client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let mut con = client
        .get_async_connection()
        .await
        .expect("Failed to connect to Redis");

    let result: String = redis::cmd("PING")
        .query_async(&mut con)
        .await
        .expect("Failed to ping Redis");

    assert_eq!(result, "PONG");
    println!("Redis connection successful: {result}");
}
