use std::env;

/// PostgreSQL parameter limit constants for batch processing optimization
pub const POSTGRESQL_MAX_PARAMS: usize = 65535; // PostgreSQL absolute maximum
pub const SAFE_PARAM_LIMIT: usize = 52428; // 80% of max for safety margin

/// Parameter counts per metric type for chunk size calculations
pub const HEART_RATE_PARAMS_PER_RECORD: usize = 6; // user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device
pub const BLOOD_PRESSURE_PARAMS_PER_RECORD: usize = 6; // user_id, recorded_at, systolic, diastolic, pulse, source_device
pub const SLEEP_PARAMS_PER_RECORD: usize = 10; // user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, efficiency, source_device
pub const ACTIVITY_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, basal_energy_burned_kcal, flights_climbed, source_device
pub const WORKOUT_PARAMS_PER_RECORD: usize = 10; // id, user_id, workout_type, started_at, ended_at, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source_device

/// Configuration for batch processing operations
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub enable_parallel_processing: bool,
    pub chunk_size: usize,
    pub memory_limit_mb: f64,
    // Chunking configurations to stay under PostgreSQL 65,535 parameter limit
    pub heart_rate_chunk_size: usize, // 6 params per record -> max 10,922
    pub blood_pressure_chunk_size: usize, // 6 params per record -> max 10,922
    pub sleep_chunk_size: usize,      // 10 params per record -> max 6,553
    pub activity_chunk_size: usize,   // 7 params per record -> max 9,362
    pub workout_chunk_size: usize,    // 10 params per record -> max 6,553
    pub enable_progress_tracking: bool, // Track progress for large batches
    pub enable_intra_batch_deduplication: bool, // Enable deduplication within batches
    // Dual-write configuration for activity_metrics migration
    pub enable_dual_write_activity_metrics: bool, // Feature flag for dual-write pattern
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            enable_parallel_processing: true, // Keep parallel processing enabled
            chunk_size: 1000,
            memory_limit_mb: 500.0,
            // Safe chunk sizes (80% of theoretical max for reliability)
            heart_rate_chunk_size: 8000, // 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,000 (max ~48,000 params)
            blood_pressure_chunk_size: 8000, // 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,000 (max ~48,000 params)
            sleep_chunk_size: 6000, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 6,000 (max ~60,000 params)
            activity_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)
            workout_chunk_size: 5000, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,000 (max ~50,000 params)
            enable_progress_tracking: true,
            enable_intra_batch_deduplication: true, // Enable by default for performance
            enable_dual_write_activity_metrics: false, // Disabled by default for safe rollout
        }
    }
}

impl BatchConfig {
    /// Create BatchConfig from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        Self {
            max_retries: env::var("BATCH_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            initial_backoff_ms: env::var("BATCH_INITIAL_BACKOFF_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            max_backoff_ms: env::var("BATCH_MAX_BACKOFF_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            enable_parallel_processing: env::var("BATCH_ENABLE_PARALLEL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            chunk_size: env::var("BATCH_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            memory_limit_mb: env::var("BATCH_MEMORY_LIMIT_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500.0),
            // Metric-specific chunk sizes with environment variable overrides
            heart_rate_chunk_size: env::var("BATCH_HEART_RATE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            blood_pressure_chunk_size: env::var("BATCH_BLOOD_PRESSURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            sleep_chunk_size: env::var("BATCH_SLEEP_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6000),
            activity_chunk_size: env::var("BATCH_ACTIVITY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            workout_chunk_size: env::var("BATCH_WORKOUT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            enable_progress_tracking: env::var("BATCH_ENABLE_PROGRESS_TRACKING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_intra_batch_deduplication: env::var("BATCH_ENABLE_DEDUPLICATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_dual_write_activity_metrics: env::var("DUAL_WRITE_ACTIVITY_METRICS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
        }
    }

    /// Validate chunk sizes against PostgreSQL parameter limits
    pub fn validate(&self) -> Result<(), String> {
        let validations = vec![
            (
                "heart_rate",
                self.heart_rate_chunk_size,
                HEART_RATE_PARAMS_PER_RECORD,
            ),
            (
                "blood_pressure",
                self.blood_pressure_chunk_size,
                BLOOD_PRESSURE_PARAMS_PER_RECORD,
            ),
            ("sleep", self.sleep_chunk_size, SLEEP_PARAMS_PER_RECORD),
            (
                "activity",
                self.activity_chunk_size,
                ACTIVITY_PARAMS_PER_RECORD,
            ),
            (
                "workout",
                self.workout_chunk_size,
                WORKOUT_PARAMS_PER_RECORD,
            ),
        ];

        for (metric_type, chunk_size, params_per_record) in validations {
            let total_params = chunk_size * params_per_record;
            if total_params > SAFE_PARAM_LIMIT {
                return Err(format!(
                    "{} chunk size {} would result in {} parameters, exceeding safe limit of {}",
                    metric_type, chunk_size, total_params, SAFE_PARAM_LIMIT
                ));
            }
        }

        Ok(())
    }
}
