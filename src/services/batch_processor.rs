use futures::future::join_all;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::middleware::metrics::Metrics;
use crate::models::{HealthMetric, IngestPayload, ProcessingError};

/// Batch processing service for health data
pub struct BatchProcessor {
    pool: PgPool,
    config: BatchConfig,
    processed_counter: AtomicUsize,
    failed_counter: AtomicUsize,
}

/// Result of batch processing operation
#[derive(Debug)]
pub struct BatchProcessingResult {
    pub processed_count: usize,
    pub failed_count: usize,
    pub errors: Vec<ProcessingError>,
    pub processing_time_ms: u64,
    pub retry_attempts: usize,
    pub memory_peak_mb: Option<f64>,
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

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub enable_parallel_processing: bool,
    pub chunk_size: usize,
    pub memory_limit_mb: f64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            enable_parallel_processing: true,
            chunk_size: 1000,
            memory_limit_mb: 500.0,
        }
    }
}

impl BatchProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            config: BatchConfig::default(),
            processed_counter: AtomicUsize::new(0),
            failed_counter: AtomicUsize::new(0),
        }
    }

    pub fn with_config(pool: PgPool, config: BatchConfig) -> Self {
        Self {
            pool,
            config,
            processed_counter: AtomicUsize::new(0),
            failed_counter: AtomicUsize::new(0),
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

        let mut result = BatchProcessingResult {
            processed_count: 0,
            failed_count: 0,
            errors: Vec::new(),
            processing_time_ms: 0,
            retry_attempts: 0,
            memory_peak_mb: None,
        };

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

        // Process in parallel if enabled, otherwise sequential
        if self.config.enable_parallel_processing {
            result = self.process_parallel(user_id, grouped, all_workouts).await;
        } else {
            result = self
                .process_sequential(user_id, grouped, all_workouts)
                .await;
        }

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

        info!(
            user_id = %user_id,
            processed = result.processed_count,
            failed = result.failed_count,
            errors = result.errors.len(),
            duration_ms = result.processing_time_ms,
            memory_mb = result.memory_peak_mb,
            retry_attempts = result.retry_attempts,
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
        };

        // Spawn parallel tasks for each metric type
        if !grouped.heart_rates.is_empty() {
            let heart_rates = std::mem::take(&mut grouped.heart_rates);
            let pool = self.pool.clone();
            let config = self.config.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "HeartRate",
                    || Self::insert_heart_rates_static(&pool, user_id, heart_rates.clone()),
                    &config,
                )
                .await
            }));
        }

        if !grouped.blood_pressures.is_empty() {
            let blood_pressures = std::mem::take(&mut grouped.blood_pressures);
            let pool = self.pool.clone();
            let config = self.config.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "BloodPressure",
                    || Self::insert_blood_pressures_static(&pool, user_id, blood_pressures.clone()),
                    &config,
                )
                .await
            }));
        }

        if !grouped.sleep_metrics.is_empty() {
            let sleep_metrics = std::mem::take(&mut grouped.sleep_metrics);
            let pool = self.pool.clone();
            let config = self.config.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Sleep",
                    || Self::insert_sleep_metrics_static(&pool, user_id, sleep_metrics.clone()),
                    &config,
                )
                .await
            }));
        }

        if !grouped.activities.is_empty() {
            let activities = std::mem::take(&mut grouped.activities);
            let pool = self.pool.clone();
            let config = self.config.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Activity",
                    || Self::insert_activities_static(&pool, user_id, activities.clone()),
                    &config,
                )
                .await
            }));
        }

        if !workouts.is_empty() {
            let pool = self.pool.clone();
            let config = self.config.clone();
            tasks.push(tokio::spawn(async move {
                Self::process_with_retry(
                    "Workout",
                    || Self::insert_workouts_static(&pool, user_id, workouts.clone()),
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
                        error_message: format!("Task execution failed: {}", e),
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

        // Process activity metrics
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
                                error_message: format!("Non-retryable error: {}", e),
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
                HealthMetric::Workout(workout) => grouped.workouts.push(workout),
            }
        }

        grouped
    }

    /// Batch insert heart rate metrics with ON CONFLICT handling
    async fn insert_heart_rates(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_heart_rates_static(&self.pool, user_id, metrics).await
    }

    /// Batch insert blood pressure metrics
    async fn insert_blood_pressures(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_blood_pressures_static(&self.pool, user_id, metrics).await
    }

    /// Batch insert sleep metrics
    async fn insert_sleep_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_sleep_metrics_static(&self.pool, user_id, metrics).await
    }

    /// Batch insert activity metrics (daily aggregates)
    async fn insert_activities(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_activities_static(&self.pool, user_id, metrics).await
    }

    /// Batch insert workout records
    async fn insert_workouts(
        &self,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Result<usize, sqlx::Error> {
        Self::insert_workouts_static(&self.pool, user_id, workouts).await
    }

    /// Static version of insert_heart_rates for parallel processing
    async fn insert_heart_rates_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            let heart_rate = metric.avg_bpm.or(metric.max_bpm).unwrap_or(0);
            let resting_heart_rate = if metric.min_bpm.is_some() && metric.min_bpm != metric.avg_bpm
            {
                metric.min_bpm
            } else {
                None
            };

            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(heart_rate)
                .push_bind(resting_heart_rate)
                .push_bind(&metric.context)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Static version of insert_blood_pressures for parallel processing
    async fn insert_blood_pressures_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO blood_pressure_metrics (user_id, recorded_at, systolic, diastolic, pulse, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(metric.systolic)
                .push_bind(metric.diastolic)
                .push_bind(metric.pulse)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Static version of insert_sleep_metrics for parallel processing
    async fn insert_sleep_metrics_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO sleep_metrics (user_id, recorded_at, sleep_start, sleep_end, total_sleep_minutes, deep_sleep_minutes, rem_sleep_minutes, awake_minutes, efficiency_percentage, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(metric.sleep_start)
                .push_bind(metric.sleep_end)
                .push_bind(metric.total_sleep_minutes)
                .push_bind(metric.deep_sleep_minutes)
                .push_bind(metric.rem_sleep_minutes)
                .push_bind(metric.awake_minutes)
                .push_bind(metric.efficiency_percentage)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Static version of insert_activities for parallel processing
    async fn insert_activities_static(
        pool: &PgPool,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO activity_metrics (user_id, date, steps, distance_meters, calories_burned, active_minutes, flights_climbed, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.date)
                .push_bind(metric.steps)
                .push_bind(metric.distance_meters)
                .push_bind(metric.calories_burned)
                .push_bind(metric.active_minutes)
                .push_bind(metric.flights_climbed)
                .push_bind(&metric.source);
        });

        query_builder.push(
            " ON CONFLICT (user_id, date) DO UPDATE SET
              steps = COALESCE(EXCLUDED.steps, activity_metrics.steps),
              distance_meters = COALESCE(EXCLUDED.distance_meters, activity_metrics.distance_meters),
              calories_burned = COALESCE(EXCLUDED.calories_burned, activity_metrics.calories_burned),
              active_minutes = COALESCE(EXCLUDED.active_minutes, activity_metrics.active_minutes),
              flights_climbed = COALESCE(EXCLUDED.flights_climbed, activity_metrics.flights_climbed),
              updated_at = NOW()"
        );

        let result = query_builder.build().execute(pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Static version of insert_workouts for parallel processing
    async fn insert_workouts_static(
        pool: &PgPool,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Result<usize, sqlx::Error> {
        if workouts.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO workouts (id, user_id, workout_type, start_time, end_time, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source) "
        );

        query_builder.push_values(workouts.iter(), |mut b, workout| {
            b.push_bind(Uuid::new_v4())
                .push_bind(user_id)
                .push_bind(&workout.workout_type)
                .push_bind(workout.start_time)
                .push_bind(workout.end_time)
                .push_bind(workout.total_energy_kcal)
                .push_bind(workout.distance_meters)
                .push_bind(workout.avg_heart_rate)
                .push_bind(workout.max_heart_rate)
                .push_bind(&workout.source);
        });

        query_builder.push(" ON CONFLICT (user_id, start_time) DO NOTHING");

        let result = query_builder.build().execute(pool).await?;
        Ok(result.rows_affected() as usize)
    }
}

/// Helper struct for grouping metrics by type
#[derive(Default, Clone)]
struct GroupedMetrics {
    heart_rates: Vec<crate::models::HeartRateMetric>,
    blood_pressures: Vec<crate::models::BloodPressureMetric>,
    sleep_metrics: Vec<crate::models::SleepMetric>,
    activities: Vec<crate::models::ActivityMetric>,
    workouts: Vec<crate::models::WorkoutData>,
}
