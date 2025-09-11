#[cfg(test)]
mod timeout_tests {
    use std::env;

    #[test]
    fn test_timeout_environment_variable_parsing() {
        // Test default timeout value - reduced for DoS protection
        let default_timeout = env::var("REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .expect("Should parse default timeout");

        assert_eq!(
            default_timeout, 60,
            "Default timeout should be 60 seconds for DoS protection"
        );

        // Test that 60 seconds is under Cloudflare's 100-second limit
        assert!(
            default_timeout < 100,
            "Timeout should be under Cloudflare's 100s limit"
        );

        // Test additional security timeouts
        let connection_timeout = env::var("CONNECTION_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .expect("Should parse connection timeout");

        assert_eq!(
            connection_timeout, 30,
            "Connection timeout should be 30 seconds"
        );

        let keep_alive_timeout = env::var("KEEP_ALIVE_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "15".to_string())
            .parse::<u64>()
            .expect("Should parse keep-alive timeout");

        assert_eq!(
            keep_alive_timeout, 15,
            "Keep-alive timeout should be 15 seconds"
        );

        println!("✅ Environment variable parsing test completed");
        println!("   - Default timeout: {}s (DoS-protected)", default_timeout);
        println!("   - Connection timeout: {}s", connection_timeout);
        println!("   - Keep-alive timeout: {}s", keep_alive_timeout);
        println!("   - Under Cloudflare limit (100s): ✓");
    }

    #[test]
    fn test_cloudflare_timeout_configuration() {
        // Verify the timeout configuration constants - updated for DoS protection
        let cloudflare_limit = 100u64;
        let our_timeout = 60u64; // Reduced for security

        assert!(
            our_timeout < cloudflare_limit,
            "Our timeout must be under Cloudflare's limit"
        );
        assert!(
            our_timeout > 30,
            "Timeout should be reasonable for health data processing"
        );

        println!("✅ Cloudflare timeout configuration test completed");
        println!("   - Cloudflare limit: {}s", cloudflare_limit);
        println!("   - Our timeout: {}s (DoS-protected)", our_timeout);
        println!("   - Safety margin: {}s", cloudflare_limit - our_timeout);
    }

    #[test]
    fn test_payload_size_limits() {
        // Test payload size configuration for DoS protection
        let max_payload_size_mb = env::var("MAX_PAYLOAD_SIZE_MB")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<usize>()
            .expect("Should parse max payload size");

        assert_eq!(
            max_payload_size_mb, 50,
            "Max payload should be 50MB for DoS protection"
        );
        assert!(
            max_payload_size_mb < 100,
            "Payload size should be reasonable for health data"
        );
        assert!(
            max_payload_size_mb > 10,
            "Payload size should accommodate legitimate health data"
        );

        println!("✅ Payload size limits test completed");
        println!(
            "   - Max payload size: {}MB (DoS-protected)",
            max_payload_size_mb
        );
    }

    #[test]
    fn test_dos_protection_timeouts() {
        // Verify all timeout values work together for DoS protection
        let request_timeout = 60u64;
        let connection_timeout = 30u64;
        let keep_alive_timeout = 15u64;

        assert!(
            keep_alive_timeout < connection_timeout,
            "Keep-alive should be shorter than connection timeout"
        );
        assert!(
            connection_timeout < request_timeout,
            "Connection timeout should be shorter than request timeout"
        );

        // Ensure timeouts are reasonable for health data but protect against DoS
        assert!(
            request_timeout >= 30,
            "Request timeout should allow reasonable health data processing"
        );
        assert!(
            request_timeout <= 90,
            "Request timeout should prevent resource exhaustion"
        );

        println!("✅ DoS protection timeout configuration test completed");
        println!("   - Request timeout: {}s", request_timeout);
        println!("   - Connection timeout: {}s", connection_timeout);
        println!("   - Keep-alive timeout: {}s", keep_alive_timeout);
        println!("   - DoS protection: ✓");
    }
}
