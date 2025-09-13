use std::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

/// Configuration for timeout management
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Maximum processing time before timeout (reduced from 80s to 30s)
    pub max_processing_seconds: u64,
    /// Threshold for large batch processing (metrics count)
    pub large_batch_threshold: usize,
    /// Timeout for JSON parsing
    pub json_parse_timeout_secs: u64,
    /// Connection pool timeout
    pub connection_timeout_secs: u64,
    /// Background job threshold - payloads above this should use background processing
    pub background_job_threshold: usize,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            max_processing_seconds: 30, // Reduced from 80s to prevent Cloudflare 524 errors
            large_batch_threshold: 5_000,
            json_parse_timeout_secs: 10,
            connection_timeout_secs: 5,
            background_job_threshold: 10_000, // Above this, recommend background processing
        }
    }
}

/// Timeout manager for handling processing timeouts and connection management
pub struct TimeoutManager {
    config: TimeoutConfig,
    start_time: Instant,
}

impl TimeoutManager {
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(TimeoutConfig::default())
    }

    /// Check if we should recommend background processing for this payload
    pub fn should_use_background_processing(&self, metric_count: usize) -> bool {
        metric_count > self.config.background_job_threshold
    }

    /// Get the maximum processing timeout duration
    pub fn get_processing_timeout(&self) -> Duration {
        Duration::from_secs(self.config.max_processing_seconds)
    }

    /// Get JSON parsing timeout duration
    pub fn get_json_timeout(&self) -> Duration {
        Duration::from_secs(self.config.json_parse_timeout_secs)
    }

    /// Get connection timeout duration
    pub fn get_connection_timeout(&self) -> Duration {
        Duration::from_secs(self.config.connection_timeout_secs)
    }

    /// Check if we're approaching the processing timeout
    pub fn is_approaching_timeout(&self, threshold_percentage: f64) -> bool {
        let elapsed = self.start_time.elapsed();
        let timeout_threshold = Duration::from_secs(
            (self.config.max_processing_seconds as f64 * threshold_percentage) as u64,
        );
        elapsed >= timeout_threshold
    }

    /// Get remaining time before timeout
    pub fn remaining_time(&self) -> Duration {
        let elapsed = self.start_time.elapsed();
        let max_duration = Duration::from_secs(self.config.max_processing_seconds);

        if elapsed >= max_duration {
            Duration::from_secs(0)
        } else {
            max_duration - elapsed
        }
    }

    /// Get elapsed time since start
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset the timer (useful for multi-stage processing)
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Log timeout warning if approaching limit
    pub fn warn_if_approaching_timeout(&self, user_id: Uuid, metric_count: usize) {
        if self.is_approaching_timeout(0.8) {
            // 80% of timeout
            warn!(
                user_id = %user_id,
                elapsed_ms = self.elapsed_time().as_millis(),
                max_timeout_secs = self.config.max_processing_seconds,
                metric_count = metric_count,
                "Approaching processing timeout limit"
            );
        }
    }

    /// Create timeout error message with context
    pub fn create_timeout_error(&self, metric_count: usize) -> String {
        format!(
            "Processing timed out after {} seconds. Payload with {} metrics is too large for real-time processing. Consider using background processing for large batches.",
            self.config.max_processing_seconds,
            metric_count
        )
    }

    /// Determine if batch size qualifies as "large"
    pub fn is_large_batch(&self, metric_count: usize) -> bool {
        metric_count >= self.config.large_batch_threshold
    }

    /// Get optimal chunk size based on remaining time and metrics
    pub fn get_optimal_chunk_size(&self, _total_metrics: usize, base_chunk_size: usize) -> usize {
        let remaining = self.remaining_time();
        let elapsed = self.elapsed_time();

        // If we've used less than 20% of time, we can afford larger chunks
        if elapsed.as_secs() < self.config.max_processing_seconds / 5 {
            return base_chunk_size * 2;
        }

        // If we're running out of time, reduce chunk size
        if remaining.as_secs() < self.config.max_processing_seconds / 4 {
            return base_chunk_size / 2;
        }

        base_chunk_size
    }

    /// Log final timing statistics
    pub fn log_final_stats(&self, user_id: Uuid, processed_count: usize, failed_count: usize) {
        let elapsed = self.elapsed_time();
        let status = if self.is_approaching_timeout(1.0) {
            "timeout_reached"
        } else if self.is_approaching_timeout(0.8) {
            "near_timeout"
        } else {
            "normal"
        };

        info!(
            user_id = %user_id,
            elapsed_ms = elapsed.as_millis(),
            max_timeout_secs = self.config.max_processing_seconds,
            processed_count = processed_count,
            failed_count = failed_count,
            status = status,
            "Processing completed with timing stats"
        );
    }
}

/// Response status for timeout-aware processing
#[derive(Debug, Clone)]
pub enum ProcessingStatus {
    Success,
    PartialSuccess { reason: String },
    Timeout { processed: usize, total: usize },
    BackgroundRecommended { reason: String },
}

impl ProcessingStatus {
    pub fn should_return_accepted(&self) -> bool {
        matches!(
            self,
            ProcessingStatus::PartialSuccess { .. } | ProcessingStatus::Timeout { .. }
        )
    }

    pub fn get_message(&self) -> String {
        match self {
            ProcessingStatus::Success => "Processing completed successfully".to_string(),
            ProcessingStatus::PartialSuccess { reason } => {
                format!("Partial processing completed: {}", reason)
            }
            ProcessingStatus::Timeout { processed, total } => {
                format!(
                    "Processing timed out. Processed {}/{} metrics",
                    processed, total
                )
            }
            ProcessingStatus::BackgroundRecommended { reason } => {
                format!("Background processing recommended: {}", reason)
            }
        }
    }
}

/// Timeout-aware processing result
#[derive(Debug)]
pub struct TimeoutAwareResult<T> {
    pub result: T,
    pub status: ProcessingStatus,
    pub elapsed_time: Duration,
    pub timeout_reached: bool,
}

impl<T> TimeoutAwareResult<T> {
    pub fn new(result: T, manager: &TimeoutManager) -> Self {
        let elapsed_time = manager.elapsed_time();
        let timeout_reached = manager.is_approaching_timeout(1.0);

        let status = if timeout_reached {
            ProcessingStatus::Timeout {
                processed: 0,
                total: 0,
            } // Will be updated by caller
        } else {
            ProcessingStatus::Success
        };

        Self {
            result,
            status,
            elapsed_time,
            timeout_reached,
        }
    }

    pub fn with_status(mut self, status: ProcessingStatus) -> Self {
        self.status = status;
        self
    }
}
