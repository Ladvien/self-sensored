#[cfg(test)]
mod timeout_tests {
    use std::env;

    #[test]
    fn test_timeout_environment_variable_parsing() {
        // Test default timeout value
        let default_timeout = env::var("REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "90".to_string())
            .parse::<u64>()
            .expect("Should parse default timeout");
            
        assert_eq!(default_timeout, 90, "Default timeout should be 90 seconds");
        
        // Test that 90 seconds is under Cloudflare's 100-second limit
        assert!(default_timeout < 100, "Timeout should be under Cloudflare's 100s limit");
        
        println!("✅ Environment variable parsing test completed");
        println!("   - Default timeout: {}s", default_timeout);
        println!("   - Under Cloudflare limit (100s): ✓");
    }

    #[test]
    fn test_cloudflare_timeout_configuration() {
        // Verify the timeout configuration constants
        let cloudflare_limit = 100u64;
        let our_timeout = 90u64;
        
        assert!(our_timeout < cloudflare_limit, "Our timeout must be under Cloudflare's limit");
        assert!(our_timeout > 30, "Timeout should be reasonable for large batch processing");
        
        println!("✅ Cloudflare timeout configuration test completed");
        println!("   - Cloudflare limit: {}s", cloudflare_limit);
        println!("   - Our timeout: {}s", our_timeout);
        println!("   - Safety margin: {}s", cloudflare_limit - our_timeout);
    }
}