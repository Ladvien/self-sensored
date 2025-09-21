use self_sensored::handlers::timeout_manager::{TimeoutConfig, TimeoutManager};
use std::time::Duration;
use tokio::time::sleep;

#[test]
fn test_timeout_config_default() {
    let config = TimeoutConfig::default();
    assert_eq!(config.max_processing_seconds, 30);
    assert_eq!(config.large_batch_threshold, 5_000);
    assert_eq!(config.json_parse_timeout_secs, 10);
    assert_eq!(config.connection_timeout_secs, 5);
    assert_eq!(config.background_job_threshold, 10_000);
}

#[test]
fn test_timeout_config_custom() {
    let config = TimeoutConfig {
        max_processing_seconds: 60,
        large_batch_threshold: 1_000,
        json_parse_timeout_secs: 5,
        connection_timeout_secs: 10,
        background_job_threshold: 5_000,
    };

    assert_eq!(config.max_processing_seconds, 60);
    assert_eq!(config.large_batch_threshold, 1_000);
}

#[test]
fn test_timeout_manager_creation() {
    let config = TimeoutConfig::default();
    let manager = TimeoutManager::new(config.clone());

    // Verify durations match config
    assert_eq!(
        manager.get_processing_timeout(),
        Duration::from_secs(config.max_processing_seconds)
    );
    assert_eq!(
        manager.get_json_timeout(),
        Duration::from_secs(config.json_parse_timeout_secs)
    );
    assert_eq!(
        manager.get_connection_timeout(),
        Duration::from_secs(config.connection_timeout_secs)
    );
}

#[test]
fn test_timeout_manager_with_default_config() {
    let manager = TimeoutManager::with_default_config();
    assert_eq!(manager.get_processing_timeout(), Duration::from_secs(30));
}

#[test]
fn test_should_use_background_processing() {
    let manager = TimeoutManager::with_default_config();

    // Below threshold
    assert!(!manager.should_use_background_processing(5_000));
    assert!(!manager.should_use_background_processing(9_999));
    assert!(!manager.should_use_background_processing(10_000));

    // Above threshold
    assert!(manager.should_use_background_processing(10_001));
    assert!(manager.should_use_background_processing(20_000));
    assert!(manager.should_use_background_processing(100_000));
}

#[tokio::test]
async fn test_elapsed_time() {
    let manager = TimeoutManager::with_default_config();

    // Initial elapsed should be very small
    let initial_elapsed = manager.elapsed_time();
    assert!(initial_elapsed < Duration::from_millis(100));

    // After sleeping, elapsed should increase
    sleep(Duration::from_millis(200)).await;
    let after_sleep = manager.elapsed_time();
    assert!(after_sleep >= Duration::from_millis(200));
    assert!(after_sleep < Duration::from_millis(400));
}

#[tokio::test]
async fn test_remaining_time() {
    let config = TimeoutConfig {
        max_processing_seconds: 1,
        ..Default::default()
    };
    let manager = TimeoutManager::new(config);

    // Initially should have full time remaining
    let initial_remaining = manager.remaining_time();
    assert!(initial_remaining <= Duration::from_secs(1));
    assert!(initial_remaining > Duration::from_millis(900));

    // After sleeping past timeout, should be zero
    sleep(Duration::from_secs(2)).await;
    let after_timeout = manager.remaining_time();
    assert_eq!(after_timeout, Duration::from_secs(0));
}

#[tokio::test]
async fn test_is_approaching_timeout() {
    let config = TimeoutConfig {
        max_processing_seconds: 1,
        ..Default::default()
    };
    let manager = TimeoutManager::new(config);

    // Initially should not be approaching timeout
    assert!(!manager.is_approaching_timeout(0.5)); // 50% threshold
    assert!(!manager.is_approaching_timeout(0.1)); // 10% threshold

    // After 600ms, should be past 50% but not 70%
    sleep(Duration::from_millis(600)).await;
    assert!(manager.is_approaching_timeout(0.5)); // 50% threshold
    assert!(!manager.is_approaching_timeout(0.7)); // 70% threshold

    // After total 900ms, should be past 70%
    sleep(Duration::from_millis(300)).await;
    assert!(manager.is_approaching_timeout(0.7)); // 70% threshold
    assert!(manager.is_approaching_timeout(0.9)); // 90% threshold
}

#[tokio::test]
async fn test_reset_timer() {
    let config = TimeoutConfig {
        max_processing_seconds: 2,
        ..Default::default()
    };
    let mut manager = TimeoutManager::new(config);

    // Sleep for 1 second
    sleep(Duration::from_secs(1)).await;
    let elapsed_before = manager.elapsed_time();
    assert!(elapsed_before >= Duration::from_secs(1));

    // Reset the timer
    manager.reset();

    // Elapsed should be near zero again
    let elapsed_after = manager.elapsed_time();
    assert!(elapsed_after < Duration::from_millis(100));

    // Remaining time should be full again
    let remaining = manager.remaining_time();
    assert!(remaining > Duration::from_millis(1900));
}

#[test]
fn test_timeout_config_boundaries() {
    // Test with zero values
    let zero_config = TimeoutConfig {
        max_processing_seconds: 0,
        large_batch_threshold: 0,
        json_parse_timeout_secs: 0,
        connection_timeout_secs: 0,
        background_job_threshold: 0,
    };

    let manager = TimeoutManager::new(zero_config);
    assert_eq!(manager.get_processing_timeout(), Duration::from_secs(0));
    assert!(manager.should_use_background_processing(1));

    // Test with very large values
    let large_config = TimeoutConfig {
        max_processing_seconds: u64::MAX / 2,
        large_batch_threshold: usize::MAX,
        json_parse_timeout_secs: u64::MAX / 2,
        connection_timeout_secs: u64::MAX / 2,
        background_job_threshold: usize::MAX,
    };

    let manager = TimeoutManager::new(large_config);
    assert!(!manager.should_use_background_processing(usize::MAX - 1));
}

#[tokio::test]
async fn test_concurrent_timeout_managers() {
    // Test that multiple timeout managers don't interfere with each other
    let manager1 = TimeoutManager::with_default_config();
    sleep(Duration::from_millis(100)).await;
    let manager2 = TimeoutManager::with_default_config();

    let elapsed1 = manager1.elapsed_time();
    let elapsed2 = manager2.elapsed_time();

    // Manager1 should have more elapsed time
    assert!(elapsed1 > elapsed2);
    assert!(elapsed1 >= Duration::from_millis(100));
    assert!(elapsed2 < Duration::from_millis(50));
}

#[test]
fn test_debug_implementation() {
    let config = TimeoutConfig::default();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("TimeoutConfig"));
    assert!(debug_str.contains("max_processing_seconds"));
    assert!(debug_str.contains("30"));
}

#[test]
fn test_clone_implementation() {
    let config = TimeoutConfig::default();
    let cloned = config.clone();

    assert_eq!(config.max_processing_seconds, cloned.max_processing_seconds);
    assert_eq!(config.large_batch_threshold, cloned.large_batch_threshold);
    assert_eq!(config.json_parse_timeout_secs, cloned.json_parse_timeout_secs);
    assert_eq!(config.connection_timeout_secs, cloned.connection_timeout_secs);
    assert_eq!(config.background_job_threshold, cloned.background_job_threshold);
}

#[tokio::test]
async fn test_percentage_calculations() {
    let config = TimeoutConfig {
        max_processing_seconds: 10,
        ..Default::default()
    };
    let manager = TimeoutManager::new(config);

    // Test various percentage thresholds
    assert!(!manager.is_approaching_timeout(0.0)); // 0% always false initially
    assert!(manager.is_approaching_timeout(-0.1)); // Negative should work (always true)
    assert!(!manager.is_approaching_timeout(1.0)); // 100% threshold

    sleep(Duration::from_secs(5)).await;
    assert!(manager.is_approaching_timeout(0.5)); // 50% after 5 seconds
    assert!(manager.is_approaching_timeout(0.49)); // Just under 50%
    assert!(!manager.is_approaching_timeout(0.51)); // Just over 50%

    sleep(Duration::from_secs(5)).await;
    assert!(manager.is_approaching_timeout(1.0)); // 100% after timeout
    assert!(manager.is_approaching_timeout(1.1)); // Over 100%
}