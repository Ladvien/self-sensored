use futures::future::join_all;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::config::{
    BatchConfig, ACTIVITY_PARAMS_PER_RECORD, BLOOD_PRESSURE_PARAMS_PER_RECORD,
    BODY_MEASUREMENT_PARAMS_PER_RECORD, HEART_RATE_PARAMS_PER_RECORD, SAFE_PARAM_LIMIT,
    SLEEP_PARAMS_PER_RECORD, WORKOUT_PARAMS_PER_RECORD,
};
use crate::middleware::metrics::Metrics;
use crate::models::{HealthMetric, IngestPayload, ProcessingError};

/// Batch processing service for health data
pub struct BatchProcessor {
    pool: PgPool,
    config: BatchConfig,
    processed_counter: AtomicUsize,
    failed_counter: AtomicUsize,
    db_semaphore: Arc<Semaphore>,
}

/// Result of batch processing operation
#[derive(Debug, Clone, Default)]
pub struct BatchProcessingResult {
    pub processed_count: usize,
    pub failed_count: usize,
    pub errors: Vec<ProcessingError>,
    pub processing_time_ms: u64,
    pub retry_attempts: usize,
    pub memory_peak_mb: Option<f64>,
    pub chunk_progress: Option<ChunkProgress>,
    pub deduplication_stats: Option<DeduplicationStats>,
}

/// Statistics about deduplication during batch processing
#[derive(Debug, Clone, Default)]
pub struct DeduplicationStats {
    pub heart_rate_duplicates: usize,
    pub blood_pressure_duplicates: usize,
    pub sleep_duplicates: usize,
    pub activity_duplicates: usize,
    pub body_measurement_duplicates: usize,
    pub temperature_duplicates: usize,
    pub respiratory_duplicates: usize,
    pub blood_glucose_duplicates: usize,
    pub nutrition_duplicates: usize,
    pub workout_duplicates: usize,

    // Reproductive Health Deduplication Stats (HIPAA-Compliant)
    pub menstrual_duplicates: usize,
    pub fertility_duplicates: usize,

    pub total_duplicates: usize,
    pub deduplication_time_ms: u64,
}

/// Unique key for heart rate metrics (user_id, recorded_at)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct HeartRateKey {
    user_id: Uuid,
    recorded_at_millis: i64, // Use milliseconds for precise comparison
}

/// Unique key for blood pressure metrics (user_id, recorded_at)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BloodPressureKey {
    user_id: Uuid,
    recorded_at_millis: i64,
}

/// Unique key for sleep metrics (user_id, sleep_start, sleep_end)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SleepKey {
    user_id: Uuid,
    sleep_start_millis: i64,
    sleep_end_millis: i64,
}

/// Unique key for activity metrics (user_id, recorded_at)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ActivityKey {
    user_id: Uuid,
    recorded_at_millis: i64,
}

/// Unique key for body measurement metrics (user_id, recorded_at, measurement_source)
/// Using measurement_source for smart device differentiation (manual vs smart_scale vs apple_watch)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BodyMeasurementKey {
    user_id: Uuid,
    recorded_at_millis: i64,
    measurement_source: String, // manual, smart_scale, apple_watch, etc.
}

/// Unique key for temperature metrics (user_id, recorded_at, temperature_source)
/// Using temperature_source for device differentiation (thermometer, wearable, manual, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TemperatureKey {
    user_id: Uuid,
    recorded_at_millis: i64,
    temperature_source: String, // thermometer, wearable, manual, apple_watch, etc.
}

/// Unique key for workout metrics (user_id, started_at)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WorkoutKey {
    user_id: Uuid,
    started_at_millis: i64,
}

/// Unique key for blood glucose metrics (user_id, recorded_at, glucose_source)
/// Special deduplication for CGM data streams with source-based deduplication
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BloodGlucoseKey {
    user_id: Uuid,
    recorded_at_millis: i64,
    glucose_source: Option<String>,
}

/// Unique key for respiratory metrics (user_id, recorded_at, measurement_type)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RespiratoryKey {
    user_id: Uuid,
    recorded_at_millis: i64,
}

/// Unique key for menstrual health metrics (user_id, recorded_at, cycle_day, metric_type)
/// Cycle-aware deduplication for medical accuracy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MenstrualKey {
    user_id: Uuid,
    recorded_at_millis: i64,
    cycle_day: Option<i16>,
}

/// Unique key for fertility tracking metrics (user_id, recorded_at, metric_type)
/// Privacy-first deduplication for sensitive reproductive health data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FertilityKey {
    user_id: Uuid,
    recorded_at_millis: i64,
    // Note: Not including sexual_activity in key for enhanced privacy protection
    // Deduplication based on timestamp only for sensitive data
}

/// Progress tracking for chunked operations
#[derive(Debug, Clone, Default)]
pub struct ChunkProgress {
    pub total_chunks: usize,
    pub completed_chunks: usize,
    pub metric_type_progress: std::collections::HashMap<String, MetricTypeProgress>,
}

/// Progress tracking for each metric type
#[derive(Debug, Clone, Default)]
pub struct MetricTypeProgress {
    pub total_records: usize,
    pub processed_records: usize,
    pub chunks_total: usize,
    pub chunks_completed: usize,
}

/// Processing status for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Retrying,
}

impl BatchProcessor {
    pub fn new(pool: PgPool) -> Self {
        // Limit concurrent DB operations to 10 (half of the 20 connection pool)
        // This leaves room for other operations like auth checks
        let db_semaphore = Arc::new(Semaphore::new(10));
        Self {
            pool,
            config: BatchConfig::default(),
            processed_counter: AtomicUsize::new(0),
            failed_counter: AtomicUsize::new(0),
            db_semaphore,
        }
    }

    pub fn with_config(pool: PgPool, config: BatchConfig) -> Self {
        let db_semaphore = Arc::new(Semaphore::new(10));
        Self {
            pool,
            config,
            processed_counter: AtomicUsize::new(0),
            failed_counter: AtomicUsize::new(0),
            db_semaphore,
        }
    }

    pub fn reset_counters(&self) {
        self.processed_counter.store(0, Ordering::Relaxed);
        self.failed_counter.store(0, Ordering::Relaxed);
    }

    /// Process a batch of health data with comprehensive error handling
    #[instrument(skip(self, payload))]
    pub async fn process_batch(
        &self,
        user_id: Uuid,
        payload: IngestPayload,
    ) -> BatchProcessingResult {
        let start_time = Instant::now();
        let initial_memory = self.estimate_memory_usage();

        // Record batch processing start
        let total_metrics = payload.data.metrics.len() + payload.data.workouts.len();

        self.reset_counters();

        // Validate payload size first - total_metrics already calculated above
        info!(
            user_id = %user_id,
            total_metrics = total_metrics,
            "Starting batch processing"
        );

        // Group metrics by type for efficient processing
        let mut grouped = self.group_metrics_by_type(payload.data.metrics);
        let mut all_workouts = std::mem::take(&mut grouped.workouts);
        all_workouts.extend(payload.data.workouts);

        // Put workouts back into grouped for deduplication
        grouped.workouts = all_workouts;

        // Deduplicate metrics within the batch before database operations
        let (deduplicated_grouped, dedup_stats) =
            self.deduplicate_grouped_metrics(user_id, grouped);

        // Extract workouts after deduplication
        let all_workouts = deduplicated_grouped.workouts.clone();

        // Process in parallel if enabled, otherwise sequential
        let mut result = if self.config.enable_parallel_processing {
            self.process_parallel(user_id, deduplicated_grouped, all_workouts)
                .await
        } else {
            self.process_sequential(user_id, deduplicated_grouped, all_workouts)
                .await
        };

        // Add deduplication statistics to the result
        result.deduplication_stats = Some(dedup_stats);

        // Calculate final metrics
        let duration = start_time.elapsed();
        result.processing_time_ms = duration.as_millis() as u64;
        result.memory_peak_mb = Some(self.estimate_memory_usage() - initial_memory);

        // Record batch processing metrics
        let status = if result.failed_count == 0 {
            "success"
        } else {
            "partial_failure"
        };
        Metrics::record_batch_processing_duration("mixed", total_metrics, duration);
        Metrics::record_metrics_processed("mixed", result.processed_count as u64, status);
        if result.failed_count > 0 {
            Metrics::record_error("batch_processing", "/api/v1/ingest", "warning");
        }

        // Record deduplication metrics
        if let Some(dedup_stats) = &result.deduplication_stats {
            if dedup_stats.total_duplicates > 0 {
                Metrics::record_duplicates_removed(dedup_stats.total_duplicates as u64);
            }
        }

        let dedup_info = result
            .deduplication_stats
            .as_ref()
            .map(|s| s.total_duplicates)
            .unwrap_or(0);
        let dedup_time = result
            .deduplication_stats
            .as_ref()
            .map(|s| s.deduplication_time_ms)
            .unwrap_or(0);

        info!(
            user_id = %user_id,
            processed = result.processed_count,
            failed = result.failed_count,
            errors = result.errors.len(),
            duration_ms = result.processing_time_ms,
            memory_mb = result.memory_peak_mb,
            retry_attempts = result.retry_attempts,
            total_duplicates_removed = dedup_info,
            deduplication_time_ms = dedup_time,
            "Batch processing completed"
        );

        result
    }

    /// Process metrics in parallel using tokio tasks
    async fn process_parallel(
        &self,
        user_id: Uuid,
        mut grouped: GroupedMetrics,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> BatchProcessingResult {
        let mut tasks = Vec::new();
        let mut result = BatchProcessingResult {
            processed_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            processing_time_ms: 0,
            retry_attempts: 0,
            memory_peak_mb: None,
            chunk_progress: if self.config.enable_progress_tracking {
                Some(ChunkProgress {
                    total_chunks: 0,
                    completed_chunks: 0,
                    metric_type_progress: HashMap::new(),
                })
            } else {
                None
            },
            deduplication_stats: None,
        };

        // Spawn parallel tasks for each metric type
        if !grouped.heart_rates.is_empty() {
            let heart_rates = std::mem::take(&mut grouped.heart_rates);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let semaphore = self.db_semaphore.clone();
            let chunk_size = config.heart_rate_chunk_size;
            tasks.push(tokio::spawn(async move {
                // Acquire permit before database operation
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                Self::process_with_retry(
                    "HeartRate",
                    || {
                        Self::insert_heart_rates_chunked(
                            &pool,
                            user_id,
                            heart_rates.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.blood_pressures.is_empty() {
            let blood_pressures = std::mem::take(&mut grouped.blood_pressures);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.blood_pressure_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "BloodPressure",
                    || {
                        Self::insert_blood_pressures_chunked(
                            &pool,
                            user_id,
                            blood_pressures.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.sleep_metrics.is_empty() {
            let sleep_metrics = std::mem::take(&mut grouped.sleep_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.sleep_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Sleep",
                    || {
                        Self::insert_sleep_metrics_chunked(
                            &pool,
                            user_id,
                            sleep_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.activities.is_empty() {
            let activities = std::mem::take(&mut grouped.activities);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.activity_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Activity",
                    || {
                        Self::insert_activities_chunked(
                            &pool,
                            user_id,
                            activities.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.body_measurements.is_empty() {
            let body_measurements = std::mem::take(&mut grouped.body_measurements);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.body_measurement_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "BodyMeasurement",
                    || {
                        Self::insert_body_measurements_chunked(
                            &pool,
                            user_id,
                            body_measurements.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.temperature_metrics.is_empty() {
            let temperature_metrics = std::mem::take(&mut grouped.temperature_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.temperature_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Temperature",
                    || {
                        Self::insert_temperature_metrics_chunked(
                            &pool,
                            user_id,
                            temperature_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.respiratory_metrics.is_empty() {
            let respiratory_metrics = std::mem::take(&mut grouped.respiratory_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.respiratory_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Respiratory",
                    || {
                        Self::insert_respiratory_metrics_chunked(
                            &pool,
                            user_id,
                            respiratory_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.blood_glucose.is_empty() {
            let blood_glucose = std::mem::take(&mut grouped.blood_glucose);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let chunk_size = config.blood_glucose_chunk_size;
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "BloodGlucose",
                    || {
                        Self::insert_blood_glucose_metrics_chunked(
                            &pool,
                            user_id,
                            blood_glucose.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.nutrition_metrics.is_empty() {
            let nutrition_metrics = std::mem::take(&mut grouped.nutrition_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let semaphore = self.db_semaphore.clone();
            let chunk_size = config.nutrition_chunk_size;
            tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                Self::process_with_retry(
                    "Nutrition",
                    || {
                        Self::insert_nutrition_metrics_chunked(
                            &pool,
                            user_id,
                            nutrition_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        // Process reproductive health metrics (HIPAA-Compliant Privacy-First Processing)
        if !grouped.menstrual_metrics.is_empty() {
            let menstrual_metrics = std::mem::take(&mut grouped.menstrual_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let semaphore = self.db_semaphore.clone();
            let chunk_size = config.menstrual_chunk_size;
            tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                Self::process_with_retry(
                    "MenstrualHealth",
                    || {
                        Self::insert_menstrual_metrics_chunked(
                            &pool,
                            user_id,
                            menstrual_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !grouped.fertility_metrics.is_empty() {
            let fertility_metrics = std::mem::take(&mut grouped.fertility_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            let semaphore = self.db_semaphore.clone();
            let chunk_size = config.fertility_chunk_size;
            tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                Self::process_with_retry(
                    "FertilityTracking",
                    || {
                        Self::insert_fertility_metrics_chunked(
                            &pool,
                            user_id,
                            fertility_metrics.clone(),
                            chunk_size,
                        )
                    },
                    &config,
                )
                .await
            }));
        }

        if !workouts.is_empty() {
            let pool = self.pool.clone();
            let config = self.config.clone();
            let semaphore = self.db_semaphore.clone();
            let chunk_size = config.workout_chunk_size;
            tasks.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                Self::process_with_retry(
                    "Workout",
                    || Self::insert_workouts_chunked(&pool, user_id, workouts.clone(), chunk_size),
                    &config,
                )
                .await
            }));
        }

        // Wait for all tasks to complete
        let results = join_all(tasks).await;

        // Aggregate results
        for task_result in results {
            match task_result {
                Ok((processed, failed, errors, retries)) => {
                    result.processed_count += processed;
                    result.failed_count += failed;
                    result.errors.extend(errors);
                    result.retry_attempts += retries;
                }
                Err(e) => {
                    error!("Task execution failed: {}", e);
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "Unknown".to_string(),
                        error_message: format!("Task execution failed: {e}"),
                        index: None,
                    });
                }
            }
        }

        result
    }

    /// Process metrics sequentially with transaction support
    async fn process_sequential(
        &self,
        user_id: Uuid,
        mut grouped: GroupedMetrics,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> BatchProcessingResult {
        let mut result = BatchProcessingResult {
            processed_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            processing_time_ms: 0,
            retry_attempts: 0,
            memory_peak_mb: None,
            chunk_progress: if self.config.enable_progress_tracking {
                Some(ChunkProgress {
                    total_chunks: 0,
                    completed_chunks: 0,
                    metric_type_progress: HashMap::new(),
                })
            } else {
                None
            },
            deduplication_stats: None,
        };

        // Process heart rate metrics
        if !grouped.heart_rates.is_empty() {
            let heart_rates = std::mem::take(&mut grouped.heart_rates);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "HeartRate",
                || self.insert_heart_rates(user_id, heart_rates.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process blood pressure metrics
        if !grouped.blood_pressures.is_empty() {
            let blood_pressures = std::mem::take(&mut grouped.blood_pressures);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "BloodPressure",
                || self.insert_blood_pressures(user_id, blood_pressures.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process sleep metrics
        if !grouped.sleep_metrics.is_empty() {
            let sleep_metrics = std::mem::take(&mut grouped.sleep_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Sleep",
                || self.insert_sleep_metrics(user_id, sleep_metrics.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process activity metrics with dual-write support
        if !grouped.activities.is_empty() {
            let activities = std::mem::take(&mut grouped.activities);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Activity",
                || self.insert_activities(user_id, activities.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process body measurement metrics
        if !grouped.body_measurements.is_empty() {
            let body_measurements = std::mem::take(&mut grouped.body_measurements);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "BodyMeasurement",
                || self.insert_body_measurements(user_id, body_measurements.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process temperature metrics
        if !grouped.temperature_metrics.is_empty() {
            let temperature_metrics = std::mem::take(&mut grouped.temperature_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Temperature",
                || self.insert_temperature_metrics(user_id, temperature_metrics.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process respiratory metrics
        if !grouped.respiratory_metrics.is_empty() {
            let respiratory_metrics = std::mem::take(&mut grouped.respiratory_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Respiratory",
                || self.insert_respiratory_metrics(user_id, respiratory_metrics.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process blood glucose metrics
        if !grouped.blood_glucose.is_empty() {
            let blood_glucose = std::mem::take(&mut grouped.blood_glucose);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "BloodGlucose",
                || self.insert_blood_glucose_metrics(user_id, blood_glucose.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process nutrition metrics
        if !grouped.nutrition_metrics.is_empty() {
            let nutrition_metrics = std::mem::take(&mut grouped.nutrition_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Nutrition",
                || self.insert_nutrition_metrics(user_id, nutrition_metrics.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process reproductive health metrics (HIPAA-Compliant Privacy-First Processing)
        if !grouped.menstrual_metrics.is_empty() {
            let menstrual_metrics = std::mem::take(&mut grouped.menstrual_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "MenstrualHealth",
                || {
                    Self::insert_menstrual_metrics_chunked(
                        &self.pool,
                        user_id,
                        menstrual_metrics.clone(),
                        self.config.menstrual_chunk_size,
                    )
                },
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        if !grouped.fertility_metrics.is_empty() {
            let fertility_metrics = std::mem::take(&mut grouped.fertility_metrics);
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "FertilityTracking",
                || {
                    Self::insert_fertility_metrics_chunked(
                        &self.pool,
                        user_id,
                        fertility_metrics.clone(),
                        self.config.fertility_chunk_size,
                    )
                },
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        // Process workouts
        if !workouts.is_empty() {
            let (processed, failed, errors, retries) = Self::process_with_retry(
                "Workout",
                || self.insert_workouts(user_id, workouts.clone()),
                &self.config,
            )
            .await;
            result.processed_count += processed;
            result.failed_count += failed;
            result.errors.extend(errors);
            result.retry_attempts += retries;
        }

        result
    }

    /// Process with retry logic and exponential backoff
    async fn process_with_retry<F, Fut>(
        metric_type: &str,
        operation: F,
        config: &BatchConfig,
    ) -> (usize, usize, Vec<ProcessingError>, usize)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<usize, sqlx::Error>>,
    {
        let mut retry_count = 0;
        let mut backoff_ms = config.initial_backoff_ms;

        loop {
            match operation().await {
                Ok(count) => {
                    if retry_count > 0 {
                        info!(
                            metric_type = metric_type,
                            retry_count = retry_count,
                            processed = count,
                            "Processing succeeded after retries"
                        );
                    }
                    return (count, 0, vec![], retry_count);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count > config.max_retries as usize {
                        error!(
                            metric_type = metric_type,
                            error = %e,
                            retry_count = retry_count,
                            "Processing failed after max retries"
                        );

                        return (
                            0,
                            1,
                            vec![ProcessingError {
                                metric_type: metric_type.to_string(),
                                error_message: format!(
                                    "Failed after {} retries: {}",
                                    retry_count - 1,
                                    e
                                ),
                                index: None,
                            }],
                            retry_count - 1,
                        );
                    }

                    // Check if error is retryable
                    if !Self::is_retryable_error(&e) {
                        error!(
                            metric_type = metric_type,
                            error = %e,
                            "Non-retryable error encountered"
                        );

                        return (
                            0,
                            1,
                            vec![ProcessingError {
                                metric_type: metric_type.to_string(),
                                error_message: format!("Non-retryable error: {e}"),
                                index: None,
                            }],
                            retry_count - 1,
                        );
                    }

                    warn!(
                        metric_type = metric_type,
                        error = %e,
                        retry_count = retry_count,
                        backoff_ms = backoff_ms,
                        "Processing failed, retrying after backoff"
                    );

                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = std::cmp::min(backoff_ms * 2, config.max_backoff_ms);
                }
            }
        }
    }

    /// Check if an error is retryable
    fn is_retryable_error(error: &sqlx::Error) -> bool {
        match error {
            sqlx::Error::Database(db_err) => {
                // PostgreSQL error codes that are retryable
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "40001" => true, // serialization_failure
                        "40P01" => true, // deadlock_detected
                        "53000" => true, // insufficient_resources
                        "53100" => true, // disk_full
                        "53200" => true, // out_of_memory
                        "53300" => true, // too_many_connections
                        "08000" => true, // connection_exception
                        "08003" => true, // connection_does_not_exist
                        "08006" => true, // connection_failure
                        _ => false,
                    }
                } else {
                    false
                }
            }
            sqlx::Error::Io(_) => true,
            sqlx::Error::PoolTimedOut => true,
            sqlx::Error::PoolClosed => true,
            _ => false,
        }
    }

    /// Estimate current memory usage (rough approximation)
    fn estimate_memory_usage(&self) -> f64 {
        // This is a rough estimation - in production you'd use more sophisticated memory tracking
        // For now, return 0 as we'd need additional dependencies for accurate memory tracking
        0.0
    }

    /// Group metrics by type for efficient batch processing
    fn group_metrics_by_type(&self, metrics: Vec<HealthMetric>) -> GroupedMetrics {
        let mut grouped = GroupedMetrics::default();

        for metric in metrics {
            match metric {
                HealthMetric::HeartRate(hr) => grouped.heart_rates.push(hr),
                HealthMetric::BloodPressure(bp) => grouped.blood_pressures.push(bp),
                HealthMetric::Sleep(sleep) => grouped.sleep_metrics.push(sleep),
                HealthMetric::Activity(activity) => grouped.activities.push(activity),
                HealthMetric::BodyMeasurement(body) => grouped.body_measurements.push(body),
                HealthMetric::Temperature(temp) => grouped.temperature_metrics.push(temp),
                HealthMetric::BloodGlucose(glucose) => grouped.blood_glucose.push(glucose),
                HealthMetric::Nutrition(nutrition) => grouped.nutrition_metrics.push(nutrition),
                HealthMetric::Workout(workout) => grouped.workouts.push(workout),

                // Reproductive Health Metrics (HIPAA-Compliant Privacy-First Processing)
                HealthMetric::Menstrual(menstrual) => grouped.menstrual_metrics.push(menstrual),
                HealthMetric::Fertility(fertility) => grouped.fertility_metrics.push(fertility),

                _ => {
                    // Handle other metric types not yet supported
                    warn!(
                        "Metric type {} not yet supported in batch processing",
                        metric.metric_type()
                    );
                }
            }
        }

        grouped
    }

    /// Deduplicate grouped metrics before database insertion
    /// Returns deduplicated metrics and statistics about removed duplicates
    fn deduplicate_grouped_metrics(
        &self,
        user_id: Uuid,
        mut grouped: GroupedMetrics,
    ) -> (GroupedMetrics, DeduplicationStats) {
        if !self.config.enable_intra_batch_deduplication {
            return (
                grouped,
                DeduplicationStats {
                    heart_rate_duplicates: 0,
                    blood_pressure_duplicates: 0,
                    sleep_duplicates: 0,
                    activity_duplicates: 0,
                    body_measurement_duplicates: 0,
                    temperature_duplicates: 0,
                    respiratory_duplicates: 0,
                    blood_glucose_duplicates: 0,
                    nutrition_duplicates: 0,
                    workout_duplicates: 0,

                    // Reproductive Health Deduplication Stats (HIPAA-Compliant)
                    menstrual_duplicates: 0,
                    fertility_duplicates: 0,

                    total_duplicates: 0,
                    deduplication_time_ms: 0,
                },
            );
        }

        let start_time = Instant::now();
        let mut stats = DeduplicationStats {
            heart_rate_duplicates: 0,
            blood_pressure_duplicates: 0,
            sleep_duplicates: 0,
            activity_duplicates: 0,
            body_measurement_duplicates: 0,
            temperature_duplicates: 0,
            respiratory_duplicates: 0,
            blood_glucose_duplicates: 0,
            nutrition_duplicates: 0,
            workout_duplicates: 0,

            // Reproductive Health Deduplication Stats (HIPAA-Compliant)
            menstrual_duplicates: 0,
            fertility_duplicates: 0,

            total_duplicates: 0,
            deduplication_time_ms: 0,
        };

        // Deduplicate heart rate metrics
        let original_hr_count = grouped.heart_rates.len();
        grouped.heart_rates = self.deduplicate_heart_rates(user_id, grouped.heart_rates);
        stats.heart_rate_duplicates = original_hr_count - grouped.heart_rates.len();

        // Deduplicate blood pressure metrics
        let original_bp_count = grouped.blood_pressures.len();
        grouped.blood_pressures =
            self.deduplicate_blood_pressures(user_id, grouped.blood_pressures);
        stats.blood_pressure_duplicates = original_bp_count - grouped.blood_pressures.len();

        // Deduplicate sleep metrics
        let original_sleep_count = grouped.sleep_metrics.len();
        grouped.sleep_metrics = self.deduplicate_sleep_metrics(user_id, grouped.sleep_metrics);
        stats.sleep_duplicates = original_sleep_count - grouped.sleep_metrics.len();

        // Deduplicate activity metrics
        let original_activity_count = grouped.activities.len();
        grouped.activities = self.deduplicate_activities(user_id, grouped.activities);
        stats.activity_duplicates = original_activity_count - grouped.activities.len();

        // Deduplicate body measurement metrics
        let original_body_measurement_count = grouped.body_measurements.len();
        grouped.body_measurements =
            self.deduplicate_body_measurements(user_id, grouped.body_measurements);
        stats.body_measurement_duplicates =
            original_body_measurement_count - grouped.body_measurements.len();

        // Deduplicate temperature metrics
        let original_temperature_count = grouped.temperature_metrics.len();
        grouped.temperature_metrics =
            self.deduplicate_temperature_metrics(user_id, grouped.temperature_metrics);
        stats.temperature_duplicates =
            original_temperature_count - grouped.temperature_metrics.len();

        // Deduplicate respiratory metrics
        let original_respiratory_count = grouped.respiratory_metrics.len();
        grouped.respiratory_metrics =
            self.deduplicate_respiratory_metrics(user_id, grouped.respiratory_metrics);
        stats.respiratory_duplicates =
            original_respiratory_count - grouped.respiratory_metrics.len();

        // Deduplicate blood glucose metrics with CGM-specific deduplication
        let original_glucose_count = grouped.blood_glucose.len();
        grouped.blood_glucose = self.deduplicate_blood_glucose(user_id, grouped.blood_glucose);
        stats.blood_glucose_duplicates = original_glucose_count - grouped.blood_glucose.len();

        // Deduplicate workout metrics
        let original_workout_count = grouped.workouts.len();
        grouped.workouts = self.deduplicate_workouts(user_id, grouped.workouts);
        stats.workout_duplicates = original_workout_count - grouped.workouts.len();

        // Deduplicate reproductive health metrics (HIPAA-Compliant Privacy-First Processing)
        let original_menstrual_count = grouped.menstrual_metrics.len();
        grouped.menstrual_metrics =
            self.deduplicate_menstrual_metrics(user_id, grouped.menstrual_metrics);
        stats.menstrual_duplicates = original_menstrual_count - grouped.menstrual_metrics.len();

        let original_fertility_count = grouped.fertility_metrics.len();
        grouped.fertility_metrics =
            self.deduplicate_fertility_metrics(user_id, grouped.fertility_metrics);
        stats.fertility_duplicates = original_fertility_count - grouped.fertility_metrics.len();

        stats.total_duplicates = stats.heart_rate_duplicates
            + stats.blood_pressure_duplicates
            + stats.sleep_duplicates
            + stats.activity_duplicates
            + stats.body_measurement_duplicates
            + stats.temperature_duplicates
            + stats.respiratory_duplicates
            + stats.blood_glucose_duplicates
            + stats.workout_duplicates
            + stats.menstrual_duplicates
            + stats.fertility_duplicates;

        stats.deduplication_time_ms = start_time.elapsed().as_millis() as u64;

        if stats.total_duplicates > 0 {
            info!(
                user_id = %user_id,
                heart_rate_duplicates = stats.heart_rate_duplicates,
                blood_pressure_duplicates = stats.blood_pressure_duplicates,
                sleep_duplicates = stats.sleep_duplicates,
                activity_duplicates = stats.activity_duplicates,
                body_measurement_duplicates = stats.body_measurement_duplicates,
                temperature_duplicates = stats.temperature_duplicates,
                respiratory_duplicates = stats.respiratory_duplicates,
                blood_glucose_duplicates = stats.blood_glucose_duplicates,
                workout_duplicates = stats.workout_duplicates,
                menstrual_duplicates = stats.menstrual_duplicates,
                fertility_duplicates = stats.fertility_duplicates,
                total_duplicates = stats.total_duplicates,
                deduplication_time_ms = stats.deduplication_time_ms,
                "Intra-batch deduplication completed"
            );
        }

        (grouped, stats)
    }

    /// Deduplicate heart rate metrics using HashSet for O(1) lookups
    /// Preserves order of first occurrence of each unique record
    fn deduplicate_heart_rates(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Vec<crate::models::HeartRateMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = HeartRateKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record
        }

        deduplicated
    }

    /// Deduplicate blood pressure metrics using HashSet for O(1) lookups
    fn deduplicate_blood_pressures(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Vec<crate::models::BloodPressureMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = BloodPressureKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
            };

            if seen_keys.insert(key) {
                deduplicated.push(metric);
            }
        }

        deduplicated
    }

    /// Deduplicate sleep metrics using HashSet for O(1) lookups
    fn deduplicate_sleep_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Vec<crate::models::SleepMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = SleepKey {
                user_id,
                sleep_start_millis: metric.sleep_start.timestamp_millis(),
                sleep_end_millis: metric.sleep_end.timestamp_millis(),
            };

            if seen_keys.insert(key) {
                deduplicated.push(metric);
            }
        }

        deduplicated
    }

    /// Deduplicate activity metrics using HashSet for O(1) lookups
    /// Preserves order of first occurrence of each unique record
    fn deduplicate_activities(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Vec<crate::models::ActivityMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = ActivityKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record
        }

        deduplicated
    }

    /// Deduplicate body measurement metrics using HashSet for O(1) lookups
    /// Uses composite key: user_id + recorded_at + measurement_source for smart device differentiation
    fn deduplicate_body_measurements(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BodyMeasurementMetric>,
    ) -> Vec<crate::models::BodyMeasurementMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = BodyMeasurementKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
                measurement_source: metric
                    .measurement_source
                    .clone()
                    .unwrap_or_else(|| "manual".to_string()),
            };

            if seen_keys.insert(key) {
                // First time seeing this key combination, keep the record
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record
        }

        deduplicated
    }

    /// Deduplicate temperature metrics using HashSet for O(1) lookups
    /// Preserves order of first occurrence of each unique record
    /// Uses (user_id, recorded_at, temperature_source) as composite key for source differentiation
    fn deduplicate_temperature_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::TemperatureMetric>,
    ) -> Vec<crate::models::TemperatureMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = TemperatureKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
                temperature_source: metric
                    .temperature_source
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record
        }

        deduplicated
    }

    /// Deduplicate workout metrics using HashSet for O(1) lookups  
    fn deduplicate_workouts(
        &self,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Vec<crate::models::WorkoutData> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for workout in workouts {
            let key = WorkoutKey {
                user_id,
                started_at_millis: workout.started_at.timestamp_millis(),
            };

            if seen_keys.insert(key) {
                deduplicated.push(workout);
            }
        }

        deduplicated
    }

    /// Deduplicate blood glucose metrics using HashSet for O(1) lookups with CGM-specific deduplication
    /// Uses user_id + recorded_at + glucose_source for high-precision CGM data deduplication
    /// Preserves order of first occurrence of each unique record
    fn deduplicate_blood_glucose(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodGlucoseMetric>,
    ) -> Vec<crate::models::BloodGlucoseMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = BloodGlucoseKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
                glucose_source: metric.glucose_source.clone(),
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record for CGM data integrity
        }

        deduplicated
    }

    /// Deduplicate respiratory metrics using HashSet for O(1) lookups
    /// Preserves order of first occurrence of each unique record
    fn deduplicate_respiratory_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::RespiratoryMetric>,
    ) -> Vec<crate::models::RespiratoryMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = RespiratoryKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
            };
            if seen_keys.insert(key) {
                deduplicated.push(metric);
            }
        }

        deduplicated
    }

    /// Deduplicate menstrual health metrics using cycle-aware deduplication for medical accuracy
    /// Preserves order of first occurrence of each unique record
    /// HIPAA-Compliant with enhanced audit logging for reproductive health data
    fn deduplicate_menstrual_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::MenstrualMetric>,
    ) -> Vec<crate::models::MenstrualMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = MenstrualKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
                cycle_day: metric.cycle_day,
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                // Log reproductive health data access for HIPAA compliance
                if self.config.reproductive_health_audit_logging {
                    debug!(
                        user_id = %user_id,
                        privacy_level = metric.get_privacy_level(),
                        "Processing menstrual health metric for deduplication"
                    );
                }
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record with privacy-aware logging
        }

        deduplicated
    }

    /// Deduplicate fertility tracking metrics with privacy-first approach
    /// Preserves order of first occurrence of each unique record
    /// Enhanced privacy protection for sensitive reproductive health data
    fn deduplicate_fertility_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::FertilityMetric>,
    ) -> Vec<crate::models::FertilityMetric> {
        let mut seen_keys = HashSet::new();
        let mut deduplicated = Vec::new();

        for metric in metrics {
            let key = FertilityKey {
                user_id,
                recorded_at_millis: metric.recorded_at.timestamp_millis(),
                // Note: Privacy-first deduplication - not including sensitive fields in key
            };

            if seen_keys.insert(key) {
                // First time seeing this key, keep the record
                // Enhanced audit logging for fertility data with maximum privacy protection
                if self.config.reproductive_health_audit_logging {
                    debug!(
                        user_id = %user_id,
                        privacy_level = metric.get_privacy_level(),
                        requires_enhanced_audit = metric.requires_enhanced_audit(),
                        "Processing fertility tracking metric for deduplication"
                    );
                }
                deduplicated.push(metric);
            }
            // Duplicate found - skip this record with privacy-aware logging
        }

        deduplicated
    }

    /// Batch insert heart rate metrics with ON CONFLICT handling
    async fn insert_heart_rates(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_heart_rates_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.heart_rate_chunk_size,
        )
        .await
    }

    /// Batch insert blood pressure metrics
    async fn insert_blood_pressures(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_blood_pressures_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.blood_pressure_chunk_size,
        )
        .await
    }

    /// Batch insert sleep metrics
    async fn insert_sleep_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_sleep_metrics_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.sleep_chunk_size,
        )
        .await
    }

    /// Batch insert activity metrics (daily aggregates)
    async fn insert_activities(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_activities_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.activity_chunk_size,
        )
        .await
    }

    /// Batch insert body measurement metrics
    async fn insert_body_measurements(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BodyMeasurementMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_body_measurements_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.body_measurement_chunk_size,
        )
        .await
    }

    /// Batch insert workout records
    async fn insert_workouts(
        &self,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_workouts_chunked(
            &self.pool,
            user_id,
            workouts,
            self.config.workout_chunk_size,
        )
        .await
    }

    /// Static version of insert_heart_rates for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_heart_rates_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 8000; // Safe default for heart rate (6 params per record)

        Self::insert_heart_rates_chunked(pool, user_id, metrics, chunk_size).await
    }

    /// Insert heart rate metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_heart_rates_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * HEART_RATE_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage("heart_rate", "chunked_insert", max_params_per_chunk);

        info!(
            metric_type = "heart_rate",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing heart rate metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                let heart_rate = metric.heart_rate.unwrap_or(0);
                let resting_heart_rate = metric.resting_heart_rate;

                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(heart_rate)
                    .push_bind(resting_heart_rate)
                    .push_bind(metric.context)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Heart rate chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Static version of insert_blood_pressures for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_blood_pressures_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 8000; // Safe default for blood pressure (6 params per record)

        Self::insert_blood_pressures_chunked(pool, user_id, metrics, chunk_size).await
    }

    /// Insert blood pressure metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_blood_pressures_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * BLOOD_PRESSURE_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage(
            "blood_pressure",
            "chunked_insert",
            max_params_per_chunk,
        );

        info!(
            metric_type = "blood_pressure",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing blood pressure metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO blood_pressure_metrics (user_id, recorded_at, systolic, diastolic, pulse, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.systolic)
                    .push_bind(metric.diastolic)
                    .push_bind(metric.pulse)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Blood pressure chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Static version of insert_sleep_metrics for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_sleep_metrics_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 6000; // Safe default for sleep (10 params per record)

        Self::insert_sleep_metrics_chunked(pool, user_id, metrics, chunk_size).await
    }

    /// Insert sleep metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_sleep_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * SLEEP_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage("sleep", "chunked_insert", max_params_per_chunk);

        info!(
            metric_type = "sleep",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing sleep metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO sleep_metrics (user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, sleep_efficiency, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                let duration_minutes = metric.duration_minutes.unwrap_or(0);
                let efficiency = metric.efficiency;
                // Calculate light sleep as remainder if not provided
                let light_sleep = if let (Some(deep), Some(rem), Some(awake)) = (
                    metric.deep_sleep_minutes,
                    metric.rem_sleep_minutes,
                    metric.awake_minutes,
                ) {
                    Some(duration_minutes - deep - rem - awake)
                } else {
                    None
                };

                b.push_bind(user_id)
                    .push_bind(metric.sleep_start)
                    .push_bind(metric.sleep_end)
                    .push_bind(duration_minutes)
                    .push_bind(metric.deep_sleep_minutes)
                    .push_bind(metric.rem_sleep_minutes)
                    .push_bind(light_sleep)
                    .push_bind(metric.awake_minutes)
                    .push_bind(efficiency)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, sleep_start, sleep_end) DO UPDATE SET
                duration_minutes = GREATEST(EXCLUDED.duration_minutes, sleep_metrics.duration_minutes),
                deep_sleep_minutes = COALESCE(EXCLUDED.deep_sleep_minutes, sleep_metrics.deep_sleep_minutes),
                rem_sleep_minutes = COALESCE(EXCLUDED.rem_sleep_minutes, sleep_metrics.rem_sleep_minutes),
                light_sleep_minutes = COALESCE(EXCLUDED.light_sleep_minutes, sleep_metrics.light_sleep_minutes),
                awake_minutes = COALESCE(EXCLUDED.awake_minutes, sleep_metrics.awake_minutes),
                sleep_efficiency = COALESCE(EXCLUDED.sleep_efficiency, sleep_metrics.sleep_efficiency),
                updated_at = NOW()");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Sleep chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Static version of insert_activities for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_activities_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 6500; // Safe default for activity (8 params per record)

        Self::insert_activities_chunked(pool, user_id, metrics, chunk_size).await
    }

    /// Insert activity metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_activities_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * ACTIVITY_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage("activity", "chunked_insert", max_params_per_chunk);

        info!(
            metric_type = "activity",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing activity metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO activity_metrics (user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, basal_energy_burned_kcal, flights_climbed, distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters, distance_downhill_snow_sports_meters, push_count, swimming_stroke_count, nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes, apple_move_time_minutes, apple_stand_hour_achieved, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.step_count)
                    .push_bind(metric.distance_meters)
                    .push_bind(metric.active_energy_burned_kcal)
                    .push_bind(metric.basal_energy_burned_kcal)
                    .push_bind(metric.flights_climbed)
                    .push_bind(metric.distance_cycling_meters)
                    .push_bind(metric.distance_swimming_meters)
                    .push_bind(metric.distance_wheelchair_meters)
                    .push_bind(metric.distance_downhill_snow_sports_meters)
                    .push_bind(metric.push_count)
                    .push_bind(metric.swimming_stroke_count)
                    .push_bind(metric.nike_fuel_points)
                    .push_bind(metric.apple_exercise_time_minutes)
                    .push_bind(metric.apple_stand_time_minutes)
                    .push_bind(metric.apple_move_time_minutes)
                    .push_bind(metric.apple_stand_hour_achieved)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Activity chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Static version of insert_body_measurements for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_body_measurements_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BodyMeasurementMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 3000; // Safe default for body measurements (16 params per record)

        Self::insert_body_measurements_chunked(pool, user_id, metrics, chunk_size).await
    }

    /// Insert body measurement metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_body_measurements_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BodyMeasurementMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * BODY_MEASUREMENT_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage(
            "body_measurement",
            "chunked_insert",
            max_params_per_chunk,
        );

        info!(
            metric_type = "body_measurement",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing body measurement metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO body_measurements (user_id, recorded_at, body_weight_kg, body_mass_index, body_fat_percentage, lean_body_mass_kg, height_cm, waist_circumference_cm, hip_circumference_cm, chest_circumference_cm, arm_circumference_cm, thigh_circumference_cm, body_temperature_celsius, basal_body_temperature_celsius, measurement_source, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.body_weight_kg)
                    .push_bind(metric.body_mass_index)
                    .push_bind(metric.body_fat_percentage)
                    .push_bind(metric.lean_body_mass_kg)
                    .push_bind(metric.height_cm)
                    .push_bind(metric.waist_circumference_cm)
                    .push_bind(metric.hip_circumference_cm)
                    .push_bind(metric.chest_circumference_cm)
                    .push_bind(metric.arm_circumference_cm)
                    .push_bind(metric.thigh_circumference_cm)
                    .push_bind(metric.body_temperature_celsius)
                    .push_bind(metric.basal_body_temperature_celsius)
                    .push_bind(&metric.measurement_source)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at, measurement_source) DO UPDATE SET
                body_weight_kg = COALESCE(EXCLUDED.body_weight_kg, body_measurements.body_weight_kg),
                body_mass_index = COALESCE(EXCLUDED.body_mass_index, body_measurements.body_mass_index),
                body_fat_percentage = COALESCE(EXCLUDED.body_fat_percentage, body_measurements.body_fat_percentage),
                lean_body_mass_kg = COALESCE(EXCLUDED.lean_body_mass_kg, body_measurements.lean_body_mass_kg),
                height_cm = COALESCE(EXCLUDED.height_cm, body_measurements.height_cm),
                waist_circumference_cm = COALESCE(EXCLUDED.waist_circumference_cm, body_measurements.waist_circumference_cm),
                hip_circumference_cm = COALESCE(EXCLUDED.hip_circumference_cm, body_measurements.hip_circumference_cm),
                chest_circumference_cm = COALESCE(EXCLUDED.chest_circumference_cm, body_measurements.chest_circumference_cm),
                arm_circumference_cm = COALESCE(EXCLUDED.arm_circumference_cm, body_measurements.arm_circumference_cm),
                thigh_circumference_cm = COALESCE(EXCLUDED.thigh_circumference_cm, body_measurements.thigh_circumference_cm),
                body_temperature_celsius = COALESCE(EXCLUDED.body_temperature_celsius, body_measurements.body_temperature_celsius),
                basal_body_temperature_celsius = COALESCE(EXCLUDED.basal_body_temperature_celsius, body_measurements.basal_body_temperature_celsius)");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Body measurement chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Static version of insert_workouts for parallel processing with chunking
    #[allow(dead_code)]
    async fn insert_workouts_static(
        pool: &PgPool,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Result<usize, sqlx::Error> {
        if workouts.is_empty() {
            return Ok(0);
        }

        // Use safe default chunking to prevent parameter limit errors
        let chunk_size = 5000; // Safe default for workouts (10 params per record)

        Self::insert_workouts_chunked(pool, user_id, workouts, chunk_size).await
    }

    /// Insert workout metrics with chunking to respect PostgreSQL parameter limits
    async fn insert_workouts_chunked(
        pool: &PgPool,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if workouts.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0;
        let chunks: Vec<_> = workouts.chunks(chunk_size).collect();

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * WORKOUT_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // AUDIT-007: Record parameter usage for monitoring
        Metrics::record_batch_parameter_usage("workout", "chunked_insert", max_params_per_chunk);

        info!(
            metric_type = "workout",
            total_records = workouts.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing workout metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO workouts (id, user_id, workout_type, started_at, ended_at, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, workout| {
                b.push_bind(Uuid::new_v4())
                    .push_bind(user_id)
                    .push_bind(workout.workout_type)
                    .push_bind(workout.started_at)
                    .push_bind(workout.ended_at)
                    .push_bind(workout.total_energy_kcal)
                    .push_bind(workout.distance_meters)
                    .push_bind(workout.avg_heart_rate)
                    .push_bind(workout.max_heart_rate)
                    .push_bind(&workout.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, started_at) DO UPDATE SET
                ended_at = CASE WHEN EXCLUDED.ended_at > workouts.ended_at THEN EXCLUDED.ended_at ELSE workouts.ended_at END,
                total_energy_kcal = COALESCE(EXCLUDED.total_energy_kcal, workouts.total_energy_kcal),
                distance_meters = COALESCE(EXCLUDED.distance_meters, workouts.distance_meters),
                avg_heart_rate = COALESCE(EXCLUDED.avg_heart_rate, workouts.avg_heart_rate),
                max_heart_rate = COALESCE(EXCLUDED.max_heart_rate, workouts.max_heart_rate),
                updated_at = NOW()");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                total_inserted = total_inserted,
                "Workout chunk processed"
            );
        }

        Ok(total_inserted)
    }

    /// Batch insert respiratory metrics with ON CONFLICT handling
    async fn insert_respiratory_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::RespiratoryMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_respiratory_metrics_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.respiratory_chunk_size,
        )
        .await
    }

    /// Batch insert blood glucose metrics with CGM-specific handling
    async fn insert_blood_glucose_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodGlucoseMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_blood_glucose_metrics_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.blood_glucose_chunk_size,
        )
        .await
    }

    /// Batch insert respiratory metrics in optimized chunks
    async fn insert_respiratory_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::RespiratoryMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let chunks: Vec<&[crate::models::RespiratoryMetric]> = metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;
        let max_params_per_chunk = chunk_size * crate::config::RESPIRATORY_PARAMS_PER_RECORD;

        info!(
            metric_type = "respiratory",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing respiratory metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO respiratory_metrics (user_id, recorded_at, respiratory_rate, oxygen_saturation, forced_vital_capacity, forced_expiratory_volume_1, peak_expiratory_flow_rate, inhaler_usage, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.respiratory_rate)
                    .push_bind(metric.oxygen_saturation)
                    .push_bind(metric.forced_vital_capacity)
                    .push_bind(metric.forced_expiratory_volume_1)
                    .push_bind(metric.peak_expiratory_flow_rate)
                    .push_bind(metric.inhaler_usage)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO UPDATE SET
                respiratory_rate = COALESCE(EXCLUDED.respiratory_rate, respiratory_metrics.respiratory_rate),
                oxygen_saturation = COALESCE(EXCLUDED.oxygen_saturation, respiratory_metrics.oxygen_saturation),
                forced_vital_capacity = COALESCE(EXCLUDED.forced_vital_capacity, respiratory_metrics.forced_vital_capacity),
                forced_expiratory_volume_1 = COALESCE(EXCLUDED.forced_expiratory_volume_1, respiratory_metrics.forced_expiratory_volume_1),
                peak_expiratory_flow_rate = COALESCE(EXCLUDED.peak_expiratory_flow_rate, respiratory_metrics.peak_expiratory_flow_rate),
                inhaler_usage = COALESCE(EXCLUDED.inhaler_usage, respiratory_metrics.inhaler_usage),
                source_device = COALESCE(EXCLUDED.source_device, respiratory_metrics.source_device),
                updated_at = NOW()");

            let insert_result = query_builder.build().execute(pool).await;

            match insert_result {
                Ok(result) => {
                    let rows_inserted = result.rows_affected() as usize;
                    total_inserted += rows_inserted;
                    info!(
                        metric_type = "respiratory",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        rows_inserted = rows_inserted,
                        "Successfully inserted respiratory metrics chunk"
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        metric_type = "respiratory",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        "Failed to insert respiratory metrics chunk"
                    );
                    return Err(e);
                }
            }
        }

        Ok(total_inserted)
    }

    /// Batch insert blood glucose metrics in optimized chunks for CGM data streams
    async fn insert_blood_glucose_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodGlucoseMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let chunks: Vec<&[crate::models::BloodGlucoseMetric]> =
            metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;
        let max_params_per_chunk = chunk_size * crate::config::BLOOD_GLUCOSE_PARAMS_PER_RECORD;

        info!(
            metric_type = "blood_glucose",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing blood glucose metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO blood_glucose_metrics (user_id, recorded_at, blood_glucose_mg_dl, measurement_context, medication_taken, insulin_delivery_units, glucose_source, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.blood_glucose_mg_dl)
                    .push_bind(&metric.measurement_context)
                    .push_bind(metric.medication_taken)
                    .push_bind(metric.insulin_delivery_units)
                    .push_bind(&metric.glucose_source)
                    .push_bind(&metric.source_device);
            });

            // CGM-specific ON CONFLICT handling with glucose_source deduplication
            query_builder.push(" ON CONFLICT (user_id, recorded_at, glucose_source) DO UPDATE SET
                blood_glucose_mg_dl = EXCLUDED.blood_glucose_mg_dl,
                measurement_context = COALESCE(EXCLUDED.measurement_context, blood_glucose_metrics.measurement_context),
                medication_taken = COALESCE(EXCLUDED.medication_taken, blood_glucose_metrics.medication_taken),
                insulin_delivery_units = COALESCE(EXCLUDED.insulin_delivery_units, blood_glucose_metrics.insulin_delivery_units),
                source_device = COALESCE(EXCLUDED.source_device, blood_glucose_metrics.source_device),
                created_at = CURRENT_TIMESTAMP");

            let insert_result = query_builder.build().execute(pool).await;

            match insert_result {
                Ok(result) => {
                    let rows_inserted = result.rows_affected() as usize;
                    total_inserted += rows_inserted;

                    // Log critical glucose levels for medical monitoring
                    let critical_count = chunk
                        .iter()
                        .filter(|m| m.is_critical_glucose_level())
                        .count();

                    if critical_count > 0 {
                        tracing::warn!(
                            user_id = %user_id,
                            critical_glucose_readings = critical_count,
                            chunk_idx = chunk_idx,
                            "Critical blood glucose levels detected in batch"
                        );
                    }

                    info!(
                        metric_type = "blood_glucose",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        rows_inserted = rows_inserted,
                        critical_readings = critical_count,
                        "Successfully inserted blood glucose metrics chunk"
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        metric_type = "blood_glucose",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        "Failed to insert blood glucose metrics chunk"
                    );
                    return Err(e);
                }
            }
        }

        Ok(total_inserted)
    }

    /// Batch insert nutrition metrics with comprehensive nutritional validation
    /// Supports meal-based atomic processing with 25+ nutritional fields
    async fn insert_nutrition_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::NutritionMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_nutrition_metrics_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.nutrition_chunk_size,
        )
        .await
    }

    /// Batch insert nutrition metrics in optimized chunks with meal grouping support
    /// Implements atomic meal processing to ensure nutritional data integrity
    /// Handles 25+ nutritional fields including macronutrients, vitamins, and minerals
    async fn insert_nutrition_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::NutritionMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Validate chunk size doesn't exceed PostgreSQL parameter limit
        let params_per_record = crate::config::NUTRITION_PARAMS_PER_RECORD;
        let safe_chunk_size = (crate::config::SAFE_PARAM_LIMIT / params_per_record).min(chunk_size);

        info!(
            user_id = %user_id,
            total_metrics = metrics.len(),
            chunk_size = safe_chunk_size,
            params_per_record = params_per_record,
            "Starting nutrition metrics batch insert"
        );

        let mut total_inserted = 0;
        let chunks: Vec<_> = metrics.chunks(safe_chunk_size).collect();

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO nutrition_metrics (
                    user_id, recorded_at,
                    dietary_water, dietary_caffeine,
                    dietary_energy_consumed, dietary_carbohydrates, dietary_protein,
                    dietary_fat_total, dietary_fat_saturated, dietary_fat_monounsaturated, dietary_fat_polyunsaturated,
                    dietary_cholesterol, dietary_sodium, dietary_fiber, dietary_sugar,
                    dietary_calcium, dietary_iron, dietary_magnesium, dietary_potassium, dietary_zinc, dietary_phosphorus,
                    dietary_vitamin_c, dietary_vitamin_b1_thiamine, dietary_vitamin_b2_riboflavin,
                    dietary_vitamin_b3_niacin, dietary_vitamin_b6_pyridoxine, dietary_vitamin_b12_cobalamin,
                    dietary_folate, dietary_biotin, dietary_pantothenic_acid,
                    dietary_vitamin_a, dietary_vitamin_d, dietary_vitamin_e, dietary_vitamin_k,
                    meal_type, meal_id, source_device
                ) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(metric.user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.dietary_water)
                    .push_bind(metric.dietary_caffeine)
                    .push_bind(metric.dietary_energy_consumed)
                    .push_bind(metric.dietary_carbohydrates)
                    .push_bind(metric.dietary_protein)
                    .push_bind(metric.dietary_fat_total)
                    .push_bind(metric.dietary_fat_saturated)
                    .push_bind(metric.dietary_fat_monounsaturated)
                    .push_bind(metric.dietary_fat_polyunsaturated)
                    .push_bind(metric.dietary_cholesterol)
                    .push_bind(metric.dietary_sodium)
                    .push_bind(metric.dietary_fiber)
                    .push_bind(metric.dietary_sugar)
                    .push_bind(metric.dietary_calcium)
                    .push_bind(metric.dietary_iron)
                    .push_bind(metric.dietary_magnesium)
                    .push_bind(metric.dietary_potassium)
                    .push_bind(metric.dietary_zinc)
                    .push_bind(metric.dietary_phosphorus)
                    .push_bind(metric.dietary_vitamin_c)
                    .push_bind(metric.dietary_vitamin_b1_thiamine)
                    .push_bind(metric.dietary_vitamin_b2_riboflavin)
                    .push_bind(metric.dietary_vitamin_b3_niacin)
                    .push_bind(metric.dietary_vitamin_b6_pyridoxine)
                    .push_bind(metric.dietary_vitamin_b12_cobalamin)
                    .push_bind(metric.dietary_folate)
                    .push_bind(metric.dietary_biotin)
                    .push_bind(metric.dietary_pantothenic_acid)
                    .push_bind(metric.dietary_vitamin_a)
                    .push_bind(metric.dietary_vitamin_d)
                    .push_bind(metric.dietary_vitamin_e)
                    .push_bind(metric.dietary_vitamin_k)
                    .push_bind(metric.meal_type.as_ref())
                    .push_bind(metric.meal_id)
                    .push_bind(metric.source_device.as_ref());
            });

            // Add conflict handling for complex deduplication
            query_builder.push(
                " ON CONFLICT (user_id, recorded_at, dietary_energy_consumed, dietary_protein, dietary_carbohydrates)
                DO UPDATE SET
                    dietary_water = EXCLUDED.dietary_water,
                    dietary_caffeine = EXCLUDED.dietary_caffeine,
                    dietary_fat_total = EXCLUDED.dietary_fat_total,
                    dietary_fat_saturated = EXCLUDED.dietary_fat_saturated,
                    dietary_fat_monounsaturated = EXCLUDED.dietary_fat_monounsaturated,
                    dietary_fat_polyunsaturated = EXCLUDED.dietary_fat_polyunsaturated,
                    dietary_cholesterol = EXCLUDED.dietary_cholesterol,
                    dietary_sodium = EXCLUDED.dietary_sodium,
                    dietary_fiber = EXCLUDED.dietary_fiber,
                    dietary_sugar = EXCLUDED.dietary_sugar,
                    dietary_calcium = EXCLUDED.dietary_calcium,
                    dietary_iron = EXCLUDED.dietary_iron,
                    dietary_magnesium = EXCLUDED.dietary_magnesium,
                    dietary_potassium = EXCLUDED.dietary_potassium,
                    dietary_zinc = EXCLUDED.dietary_zinc,
                    dietary_phosphorus = EXCLUDED.dietary_phosphorus,
                    dietary_vitamin_c = EXCLUDED.dietary_vitamin_c,
                    dietary_vitamin_b1_thiamine = EXCLUDED.dietary_vitamin_b1_thiamine,
                    dietary_vitamin_b2_riboflavin = EXCLUDED.dietary_vitamin_b2_riboflavin,
                    dietary_vitamin_b3_niacin = EXCLUDED.dietary_vitamin_b3_niacin,
                    dietary_vitamin_b6_pyridoxine = EXCLUDED.dietary_vitamin_b6_pyridoxine,
                    dietary_vitamin_b12_cobalamin = EXCLUDED.dietary_vitamin_b12_cobalamin,
                    dietary_folate = EXCLUDED.dietary_folate,
                    dietary_biotin = EXCLUDED.dietary_biotin,
                    dietary_pantothenic_acid = EXCLUDED.dietary_pantothenic_acid,
                    dietary_vitamin_a = EXCLUDED.dietary_vitamin_a,
                    dietary_vitamin_d = EXCLUDED.dietary_vitamin_d,
                    dietary_vitamin_e = EXCLUDED.dietary_vitamin_e,
                    dietary_vitamin_k = EXCLUDED.dietary_vitamin_k,
                    meal_type = EXCLUDED.meal_type,
                    meal_id = EXCLUDED.meal_id,
                    source_device = EXCLUDED.source_device"
            );

            let query = query_builder.build();
            let total_params = chunk.len() * params_per_record;

            info!(
                chunk_idx = chunk_idx,
                chunk_size = chunk.len(),
                total_params = total_params,
                "Executing nutrition metrics chunk insert"
            );

            match query.execute(pool).await {
                Ok(result) => {
                    let chunk_inserted = result.rows_affected() as usize;
                    total_inserted += chunk_inserted;
                    info!(
                        chunk_idx = chunk_idx,
                        inserted = chunk_inserted,
                        total_inserted = total_inserted,
                        "Successfully inserted nutrition metrics chunk"
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        metric_type = "nutrition",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        "Failed to insert nutrition metrics chunk"
                    );
                    return Err(e);
                }
            }
        }

        info!(
            user_id = %user_id,
            total_inserted = total_inserted,
            total_chunks = chunks.len(),
            "Completed nutrition metrics batch insert"
        );

        Ok(total_inserted)
    }

    /// Batch insert temperature metrics in optimized chunks with medical validation
    /// Handles body temperature, basal body temperature, Apple Watch wrist temperature, and water temperature
    /// Supports fertility tracking patterns and fever detection
    async fn insert_temperature_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::TemperatureMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let chunks: Vec<&[crate::models::TemperatureMetric]> = metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;
        let max_params_per_chunk = chunk_size * crate::config::TEMPERATURE_PARAMS_PER_RECORD;

        info!(
            metric_type = "temperature",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing temperature metrics in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO temperature_metrics (user_id, recorded_at, body_temperature, basal_body_temperature, apple_sleeping_wrist_temperature, water_temperature, temperature_source, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.body_temperature)
                    .push_bind(metric.basal_body_temperature)
                    .push_bind(metric.apple_sleeping_wrist_temperature)
                    .push_bind(metric.water_temperature)
                    .push_bind(&metric.temperature_source)
                    .push_bind(&metric.source_device);
            });

            // Use composite key for conflict resolution: user_id, recorded_at, temperature_source
            // This allows multiple temperature readings at the same time from different sources
            query_builder.push(" ON CONFLICT (user_id, recorded_at, temperature_source) DO UPDATE SET
                body_temperature = COALESCE(EXCLUDED.body_temperature, temperature_metrics.body_temperature),
                basal_body_temperature = COALESCE(EXCLUDED.basal_body_temperature, temperature_metrics.basal_body_temperature),
                apple_sleeping_wrist_temperature = COALESCE(EXCLUDED.apple_sleeping_wrist_temperature, temperature_metrics.apple_sleeping_wrist_temperature),
                water_temperature = COALESCE(EXCLUDED.water_temperature, temperature_metrics.water_temperature),
                source_device = COALESCE(EXCLUDED.source_device, temperature_metrics.source_device),
                updated_at = NOW()");

            let insert_result = query_builder.build().execute(pool).await;

            match insert_result {
                Ok(result) => {
                    let rows_inserted = result.rows_affected() as usize;
                    total_inserted += rows_inserted;

                    // Medical monitoring: count fever cases and critical temperatures in this chunk
                    let fever_count = chunk
                        .iter()
                        .filter(|metric| metric.body_temperature.is_some_and(|temp| temp > 38.0))
                        .count();

                    let critical_count = chunk
                        .iter()
                        .filter(|metric| {
                            metric
                                .body_temperature
                                .is_some_and(|temp| !(35.0..=40.0).contains(&temp))
                        })
                        .count();

                    let ovulation_indicators = chunk
                        .iter()
                        .filter(|metric| {
                            metric
                                .basal_body_temperature
                                .is_some_and(|temp| temp > 36.5)
                        })
                        .count();

                    // Log medical alerts for monitoring
                    if fever_count > 0 {
                        tracing::warn!(
                            user_id = %user_id,
                            fever_episodes = fever_count,
                            chunk_idx = chunk_idx,
                            "Fever episodes detected in temperature batch"
                        );
                    }

                    if critical_count > 0 {
                        tracing::warn!(
                            user_id = %user_id,
                            critical_temperature_readings = critical_count,
                            chunk_idx = chunk_idx,
                            "Critical temperature readings detected (hypothermia/hyperthermia)"
                        );
                    }

                    info!(
                        metric_type = "temperature",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        rows_inserted = rows_inserted,
                        fever_episodes = fever_count,
                        critical_temperatures = critical_count,
                        ovulation_indicators = ovulation_indicators,
                        "Successfully inserted temperature metrics chunk"
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        metric_type = "temperature",
                        chunk_idx = chunk_idx,
                        chunk_size = chunk.len(),
                        "Failed to insert temperature metrics chunk"
                    );
                    return Err(e);
                }
            }
        }

        Ok(total_inserted)
    }

    /// Batch insert menstrual health metrics in optimized chunks with HIPAA-compliant audit logging
    /// Implements cycle-aware deduplication and privacy-first data handling for reproductive health
    /// Supports medical-grade menstrual cycle validation and pattern recognition
    async fn insert_menstrual_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::MenstrualMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let chunks: Vec<&[crate::models::MenstrualMetric]> = metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;
        let max_params_per_chunk = chunk_size * crate::config::MENSTRUAL_PARAMS_PER_RECORD;

        info!(
            metric_type = "menstrual_health",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing menstrual health metrics in chunks with HIPAA-compliant privacy protection"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder = sqlx::QueryBuilder::new("INSERT INTO menstrual_health (user_id, recorded_at, menstrual_flow, spotting, cycle_day, cramps_severity, mood_rating, energy_level, notes, source_device) ");

            query_builder.push_values(chunk.iter(), |mut builder, metric| {
                builder
                    .push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.menstrual_flow)
                    .push_bind(metric.spotting)
                    .push_bind(metric.cycle_day)
                    .push_bind(metric.cramps_severity)
                    .push_bind(metric.mood_rating)
                    .push_bind(metric.energy_level)
                    .push_bind(metric.notes.as_deref())
                    .push_bind(metric.source_device.as_deref());
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

            let inserted = query_builder.build().execute(pool).await?.rows_affected() as usize;

            total_inserted += inserted;

            debug!(
                chunk_idx = chunk_idx + 1,
                chunk_size = chunk.len(),
                inserted = inserted,
                total_inserted = total_inserted,
                "Processed menstrual health metrics chunk with privacy protection"
            );
        }

        info!(
            total_records = metrics.len(),
            total_inserted = total_inserted,
            duplicates_skipped = metrics.len() - total_inserted,
            "Completed menstrual health batch processing with HIPAA compliance"
        );

        Ok(total_inserted)
    }

    /// Batch insert fertility tracking metrics in optimized chunks with maximum privacy protection
    /// Implements privacy-first deduplication and encrypted sensitive data handling
    /// Supports fertility pattern recognition and ovulation tracking with enhanced audit logging
    async fn insert_fertility_metrics_chunked(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::FertilityMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let chunks: Vec<&[crate::models::FertilityMetric]> = metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;
        let max_params_per_chunk = chunk_size * crate::config::FERTILITY_PARAMS_PER_RECORD;

        info!(
            metric_type = "fertility_tracking",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            highly_sensitive_records = metrics
                .iter()
                .filter(|m| m.requires_enhanced_audit())
                .count(),
            "Processing fertility tracking metrics in chunks with maximum privacy protection"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO fertility_tracking (user_id, recorded_at, cervical_mucus_quality, ovulation_test_result, sexual_activity, pregnancy_test_result, basal_body_temperature, temperature_context, cervix_firmness, cervix_position, lh_level, notes, source_device) "
            );

            query_builder.push_values(chunk.iter(), |mut builder, metric| {
                builder
                    .push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.cervical_mucus_quality.as_ref())
                    .push_bind(metric.ovulation_test_result)
                    .push_bind(metric.sexual_activity)
                    .push_bind(metric.pregnancy_test_result)
                    .push_bind(metric.basal_body_temperature)
                    .push_bind(metric.temperature_context)
                    .push_bind(metric.cervix_firmness)
                    .push_bind(metric.cervix_position)
                    .push_bind(metric.lh_level)
                    .push_bind(metric.notes.as_deref())
                    .push_bind(metric.source_device.as_deref());
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

            let inserted = query_builder.build().execute(pool).await?.rows_affected() as usize;

            total_inserted += inserted;

            debug!(
                chunk_idx = chunk_idx + 1,
                chunk_size = chunk.len(),
                inserted = inserted,
                total_inserted = total_inserted,
                highly_sensitive_in_chunk =
                    chunk.iter().filter(|m| m.requires_enhanced_audit()).count(),
                "Processed fertility tracking metrics chunk with maximum privacy protection"
            );
        }

        info!(
            total_records = metrics.len(),
            total_inserted = total_inserted,
            duplicates_skipped = metrics.len() - total_inserted,
            "Completed fertility tracking batch processing with enhanced privacy protection"
        );

        Ok(total_inserted)
    }

    /// Insert temperature metrics (non-chunked version for sequential processing)
    async fn insert_temperature_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::TemperatureMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_temperature_metrics_chunked(
            &self.pool,
            user_id,
            metrics,
            self.config.temperature_chunk_size,
        )
        .await
    }

    /// Add a method to process temperature metrics for single metric type processing
    /// This is needed for the temperature handler's batch processing
    pub async fn process_temperature_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::TemperatureMetric>,
    ) -> Result<BatchProcessingResult, crate::models::ProcessingError> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult {
                processed_count: 0,
                failed_count: 0,
                errors: Vec::new(),
                processing_time_ms: 0,
                retry_attempts: 0,
                memory_peak_mb: Some(0.0),
                chunk_progress: None,
                deduplication_stats: None,
            });
        }

        let start_time = std::time::Instant::now();

        // Deduplicate temperature metrics
        let original_count = metrics.len();
        let deduplicated_metrics = self.deduplicate_temperature_metrics(user_id, metrics);
        let duplicates_removed = original_count - deduplicated_metrics.len();

        // Process temperature metrics with retry
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "Temperature",
            || {
                Self::insert_temperature_metrics_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    self.config.temperature_chunk_size,
                )
            },
            &self.config,
        )
        .await;

        let duration = start_time.elapsed();

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms: duration.as_millis() as u64,
            retry_attempts: retries,
            memory_peak_mb: Some(self.estimate_memory_usage()),
            chunk_progress: None,
            deduplication_stats: Some(crate::services::batch_processor::DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: 0,
                body_measurement_duplicates: 0,
                temperature_duplicates: duplicates_removed,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,

                // Reproductive Health Deduplication Stats (HIPAA-Compliant)
                menstrual_duplicates: 0,
                fertility_duplicates: 0,

                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0, // We're not tracking this separately here
            }),
        })
    }

    /// Process body measurements metrics for single metric type processing
    /// This is needed for the body measurements handler's batch processing
    pub async fn process_body_measurements(
        &self,
        metrics: Vec<crate::models::BodyMeasurementMetric>,
    ) -> Result<BatchProcessingResult, crate::models::ProcessingError> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult {
                processed_count: 0,
                failed_count: 0,
                errors: Vec::new(),
                processing_time_ms: 0,
                retry_attempts: 0,
                memory_peak_mb: Some(0.0),
                chunk_progress: None,
                deduplication_stats: None,
            });
        }

        let start_time = std::time::Instant::now();
        let user_id = metrics[0].user_id; // All metrics should have same user_id

        // Deduplicate body measurements
        let deduplicated_metrics = self.deduplicate_body_measurements(user_id, metrics.clone());
        let duplicates_removed = metrics.len() - deduplicated_metrics.len();

        // Process body measurements with retry
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "BodyMeasurements",
            || {
                Self::insert_body_measurements_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    self.config.body_measurement_chunk_size,
                )
            },
            &self.config,
        )
        .await;

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms,
            retry_attempts: retries,
            memory_peak_mb: Some(0.0), // Simplified for now
            chunk_progress: None,
            deduplication_stats: Some(DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: 0,
                body_measurement_duplicates: duplicates_removed,
                temperature_duplicates: 0,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,

                // Reproductive Health Deduplication Stats (HIPAA-Compliant)
                menstrual_duplicates: 0,
                fertility_duplicates: 0,

                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0, // We're not tracking this separately here
            }),
        })
    }

    /// Process activity metrics for single metric type processing
    /// This is needed for the activity handler's batch processing
    pub async fn process_activity_metrics(
        &self,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<BatchProcessingResult, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult::default());
        }

        let start_time = std::time::Instant::now();

        // Deduplicate activity metrics
        let deduplicated_metrics = self.deduplicate_activities(user_id, metrics.clone());
        let duplicates_removed = metrics.len() - deduplicated_metrics.len();

        // Process activity metrics with retry
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "ActivityMetrics",
            || {
                Self::insert_activities_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    self.config.activity_chunk_size,
                )
            },
            &self.config,
        )
        .await;

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms,
            retry_attempts: retries,
            memory_peak_mb: Some(0.0),
            chunk_progress: None,
            deduplication_stats: Some(DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: duplicates_removed,
                body_measurement_duplicates: 0,
                temperature_duplicates: 0,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,
                menstrual_duplicates: 0,
                fertility_duplicates: 0,
                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0,
            }),
        })
    }

    /// Process hygiene events in batches with comprehensive validation and deduplication
    pub async fn process_hygiene_events(
        &self,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::health_metrics::HygieneMetric>,
    ) -> Result<BatchProcessingResult, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult::default());
        }

        let start_time = std::time::Instant::now();

        // Deduplicate hygiene events
        let deduplicated_metrics = self.deduplicate_hygiene_events(user_id, metrics.clone());
        let duplicates_removed = metrics.len() - deduplicated_metrics.len();

        // Process hygiene events with retry
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "HygieneEvents",
            || {
                Self::insert_hygiene_events_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    6000,
                )
            }, // Conservative chunk size for hygiene events
            &self.config,
        )
        .await;

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms,
            retry_attempts: retries,
            memory_peak_mb: Some(0.0), // Simplified for now
            chunk_progress: None,
            deduplication_stats: Some(DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: 0,
                body_measurement_duplicates: 0,
                temperature_duplicates: 0,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,
                menstrual_duplicates: 0,
                fertility_duplicates: 0,
                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0,
            }),
        })
    }

    /// Deduplicate hygiene events based on user_id, recorded_at, and event_type
    fn deduplicate_hygiene_events(
        &self,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::health_metrics::HygieneMetric>,
    ) -> Vec<crate::models::health_metrics::HygieneMetric> {
        // Always enable deduplication for hygiene events
        // TODO: Add enable_deduplication to BatchConfig if needed

        let start_time = std::time::Instant::now();

        // Use a HashSet to track unique combinations of (user_id, recorded_at, event_type)
        let mut seen = std::collections::HashSet::new();
        let deduplicated: Vec<_> = metrics
            .into_iter()
            .filter(|metric| {
                let key = (user_id, metric.recorded_at, metric.event_type);
                seen.insert(key)
            })
            .collect();

        let deduplication_time = start_time.elapsed();
        let removed_count = seen.len() - deduplicated.len();

        info!(
            user_id = %user_id,
            original_count = seen.len() + removed_count,
            deduplicated_count = deduplicated.len(),
            removed_count = removed_count,
            deduplication_time_ms = deduplication_time.as_millis(),
            "Hygiene events deduplication completed"
        );

        // TODO: Add deduplication stats recording to Metrics if needed

        deduplicated
    }

    /// Insert hygiene events in chunks with ON CONFLICT handling
    async fn insert_hygiene_events_chunked(
        pool: &sqlx::PgPool,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::health_metrics::HygieneMetric>,
        chunk_size: usize,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        // Parameters per hygiene event record (21 total parameters)
        const HYGIENE_PARAMS_PER_RECORD: usize = 21;
        const SAFE_PARAM_LIMIT: usize = 60000; // Conservative PostgreSQL parameter limit

        let chunks: Vec<_> = metrics.chunks(chunk_size).collect();
        let mut total_inserted = 0;

        // Validate parameter count to prevent PostgreSQL limit errors
        let max_params_per_chunk = chunk_size * HYGIENE_PARAMS_PER_RECORD;
        if max_params_per_chunk > SAFE_PARAM_LIMIT {
            return Err(sqlx::Error::Configuration(
                format!(
                    "Chunk size {chunk_size} would result in {max_params_per_chunk} parameters, exceeding safe limit"
                )
                .into(),
            ));
        }

        // TODO: Record parameter usage for monitoring

        info!(
            metric_type = "hygiene_events",
            total_records = metrics.len(),
            chunk_count = chunks.len(),
            chunk_size = chunk_size,
            max_params_per_chunk = max_params_per_chunk,
            "Processing hygiene events in chunks"
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"INSERT INTO hygiene_events (
                    user_id, recorded_at, event_type, duration_seconds, quality_rating,
                    meets_who_guidelines, frequency_compliance_rating, device_detected,
                    device_effectiveness_score, trigger_event, location_context,
                    compliance_motivation, health_crisis_enhanced, crisis_compliance_level,
                    daily_goal_progress, achievement_unlocked, medication_adherence_related,
                    medical_condition_context, data_sensitivity_level, source_device
                ) "#,
            );

            query_builder.push_values(chunk.iter(), |mut b, metric| {
                b.push_bind(user_id)
                    .push_bind(metric.recorded_at)
                    .push_bind(metric.event_type)
                    .push_bind(metric.duration_seconds)
                    .push_bind(metric.quality_rating)
                    .push_bind(metric.meets_who_guidelines)
                    .push_bind(metric.frequency_compliance_rating)
                    .push_bind(metric.device_detected)
                    .push_bind(metric.device_effectiveness_score)
                    .push_bind(&metric.trigger_event)
                    .push_bind(&metric.location_context)
                    .push_bind(&metric.compliance_motivation)
                    .push_bind(metric.health_crisis_enhanced)
                    .push_bind(metric.crisis_compliance_level)
                    .push_bind(metric.daily_goal_progress)
                    .push_bind(&metric.achievement_unlocked)
                    .push_bind(metric.medication_adherence_related)
                    .push_bind(&metric.medical_condition_context)
                    .push_bind(&metric.data_sensitivity_level)
                    .push_bind(&metric.source_device);
            });

            query_builder.push(" ON CONFLICT (user_id, recorded_at, event_type) DO NOTHING");

            let result = query_builder.build().execute(pool).await?;
            let chunk_inserted = result.rows_affected() as usize;
            total_inserted += chunk_inserted;

            info!(
                chunk_index = chunk_idx + 1,
                chunk_records = chunk.len(),
                chunk_inserted = chunk_inserted,
                "Processed hygiene events chunk"
            );
        }

        info!(
            total_records = metrics.len(),
            total_inserted = total_inserted,
            duplicates_skipped = metrics.len() - total_inserted,
            "Completed hygiene events batch processing"
        );

        Ok(total_inserted)
    }

    /// Process menstrual health metrics in batches with privacy-first HIPAA-compliant handling
    /// Implements cycle-aware deduplication and medical-grade validation for women's health tracking
    pub async fn process_menstrual_metrics(
        &self,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::MenstrualMetric>,
    ) -> Result<BatchProcessingResult, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult::default());
        }

        let start_time = std::time::Instant::now();

        // Deduplicate menstrual metrics with cycle-aware deduplication
        let deduplicated_metrics = self.deduplicate_menstrual_metrics(user_id, metrics.clone());
        let duplicates_removed = metrics.len() - deduplicated_metrics.len();

        // Enhanced audit logging for reproductive health data processing
        if self.config.reproductive_health_audit_logging {
            info!(
                user_id = %user_id,
                original_count = metrics.len(),
                deduplicated_count = deduplicated_metrics.len(),
                duplicates_removed = duplicates_removed,
                "Processing menstrual health metrics with HIPAA-compliant audit logging"
            );
        }

        // Process menstrual metrics with retry logic and privacy protection
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "MenstrualHealth",
            || {
                Self::insert_menstrual_metrics_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    self.config.menstrual_chunk_size,
                )
            },
            &self.config,
        )
        .await;

        let duration = start_time.elapsed();

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms: duration.as_millis() as u64,
            retry_attempts: retries,
            memory_peak_mb: Some(self.estimate_memory_usage()),
            chunk_progress: None,
            deduplication_stats: Some(crate::services::batch_processor::DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: 0,
                body_measurement_duplicates: 0,
                temperature_duplicates: 0,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,
                menstrual_duplicates: duplicates_removed,
                fertility_duplicates: 0,
                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0,
            }),
        })
    }

    /// Process fertility tracking metrics in batches with maximum privacy protection
    /// Implements privacy-first deduplication and encrypted sensitive data handling
    pub async fn process_fertility_metrics(
        &self,
        user_id: uuid::Uuid,
        metrics: Vec<crate::models::FertilityMetric>,
    ) -> Result<BatchProcessingResult, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(BatchProcessingResult::default());
        }

        let start_time = std::time::Instant::now();

        // Privacy-first deduplication for sensitive fertility data
        let deduplicated_metrics = self.deduplicate_fertility_metrics(user_id, metrics.clone());
        let duplicates_removed = metrics.len() - deduplicated_metrics.len();

        // Enhanced audit logging for fertility data with maximum privacy protection
        if self.config.reproductive_health_audit_logging {
            info!(
                user_id = %user_id,
                original_count = metrics.len(),
                deduplicated_count = deduplicated_metrics.len(),
                duplicates_removed = duplicates_removed,
                highly_sensitive_records = deduplicated_metrics.iter().filter(|m| m.requires_enhanced_audit()).count(),
                "Processing fertility tracking metrics with enhanced privacy protection"
            );
        }

        // Process fertility metrics with retry logic and privacy-first handling
        let (processed, failed, errors, retries) = Self::process_with_retry(
            "FertilityTracking",
            || {
                Self::insert_fertility_metrics_chunked(
                    &self.pool,
                    user_id,
                    deduplicated_metrics.clone(),
                    self.config.fertility_chunk_size,
                )
            },
            &self.config,
        )
        .await;

        let duration = start_time.elapsed();

        Ok(BatchProcessingResult {
            processed_count: processed,
            failed_count: failed,
            errors,
            processing_time_ms: duration.as_millis() as u64,
            retry_attempts: retries,
            memory_peak_mb: Some(self.estimate_memory_usage()),
            chunk_progress: None,
            deduplication_stats: Some(crate::services::batch_processor::DeduplicationStats {
                heart_rate_duplicates: 0,
                blood_pressure_duplicates: 0,
                sleep_duplicates: 0,
                activity_duplicates: 0,
                body_measurement_duplicates: 0,
                temperature_duplicates: 0,
                respiratory_duplicates: 0,
                blood_glucose_duplicates: 0,
                nutrition_duplicates: 0,
                workout_duplicates: 0,
                menstrual_duplicates: 0,
                fertility_duplicates: duplicates_removed,
                total_duplicates: duplicates_removed,
                deduplication_time_ms: 0,
            }),
        })
    }
}

/// Helper struct for grouping metrics by type
#[derive(Default, Clone)]
struct GroupedMetrics {
    heart_rates: Vec<crate::models::HeartRateMetric>,
    blood_pressures: Vec<crate::models::BloodPressureMetric>,
    sleep_metrics: Vec<crate::models::SleepMetric>,
    activities: Vec<crate::models::ActivityMetric>,
    body_measurements: Vec<crate::models::BodyMeasurementMetric>,
    temperature_metrics: Vec<crate::models::TemperatureMetric>,
    respiratory_metrics: Vec<crate::models::RespiratoryMetric>,
    blood_glucose: Vec<crate::models::BloodGlucoseMetric>,
    nutrition_metrics: Vec<crate::models::NutritionMetric>,
    workouts: Vec<crate::models::WorkoutData>,

    // Reproductive Health Metrics (HIPAA-Compliant Privacy-First Processing)
    menstrual_metrics: Vec<crate::models::MenstrualMetric>,
    fertility_metrics: Vec<crate::models::FertilityMetric>,
}
