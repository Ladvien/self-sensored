use std::env;

/// Mask password in database URL for logging (copied from main.rs)
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            let password_start = colon_pos + 1;
            let password_end = at_pos;
            if password_end > password_start {
                masked.replace_range(password_start..password_end, "****");
            }
            return masked;
        }
    }
    url.to_string()
}

#[cfg(test)]
mod project_setup_tests {
    use super::*;

    #[test]
    fn test_environment_variables_loading() {
        // Load .env file
        dotenv::dotenv().ok();

        // Test that critical environment variables can be loaded
        let database_url = env::var("DATABASE_URL");
        assert!(database_url.is_ok(), "DATABASE_URL should be available");

        let test_database_url = env::var("TEST_DATABASE_URL");
        assert!(
            test_database_url.is_ok(),
            "TEST_DATABASE_URL should be available"
        );

        // Verify URLs point to the correct remote server
        let db_url = database_url.unwrap();
        assert!(
            db_url.contains("192.168.1.104"),
            "DATABASE_URL should point to 192.168.1.104"
        );

        let test_db_url = test_database_url.unwrap();
        assert!(
            test_db_url.contains("192.168.1.104"),
            "TEST_DATABASE_URL should point to 192.168.1.104"
        );

        // Test optional environment variables have defaults
        let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        assert!(!server_host.is_empty(), "Server host should not be empty");

        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>();
        assert!(server_port.is_ok(), "Server port should be a valid number");

        let workers = env::var("WORKERS")
            .unwrap_or_else(|_| "4".to_string())
            .parse::<usize>();
        assert!(workers.is_ok(), "Workers should be a valid number");

        println!("✓ All environment variables loaded successfully");
        println!("  Database URL: {}", mask_password(&db_url));
        println!("  Test Database URL: {}", mask_password(&test_db_url));
        println!("  Server: {}:{}", server_host, server_port.unwrap());
        println!("  Workers: {}", workers.unwrap());
    }

    #[test]
    fn test_database_connection_pool_config() {
        dotenv::dotenv().ok();

        let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<u32>();
        assert!(
            max_connections.is_ok(),
            "DATABASE_MAX_CONNECTIONS should be valid"
        );

        let min_connections = env::var("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>();
        assert!(
            min_connections.is_ok(),
            "DATABASE_MIN_CONNECTIONS should be valid"
        );

        let connect_timeout = env::var("DATABASE_CONNECT_TIMEOUT")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u64>();
        assert!(
            connect_timeout.is_ok(),
            "DATABASE_CONNECT_TIMEOUT should be valid"
        );

        println!("✓ Database connection pool configuration is valid");
    }

    #[test]
    fn test_redis_configuration() {
        dotenv::dotenv().ok();

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://192.168.1.104:6379".to_string());
        assert!(
            redis_url.contains("redis://"),
            "REDIS_URL should be a valid Redis URL"
        );
        assert!(
            redis_url.contains("192.168.1.104"),
            "REDIS_URL should point to 192.168.1.104"
        );

        let redis_pool_size = env::var("REDIS_POOL_SIZE")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>();
        assert!(redis_pool_size.is_ok(), "REDIS_POOL_SIZE should be valid");

        println!("✓ Redis configuration is valid: {redis_url}");
    }
}
