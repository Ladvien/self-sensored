use crate::config::BatchConfig;
use crate::models::enums::{JobStatus, JobType};
use crate::models::{IngestBatchConfig, IngestPayload, JobResultSummary, ProcessingJob};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::sleep;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Background job processor that handles async processing of health data
pub struct BackgroundProcessor {
    pool: PgPool,
    batch_processor: BatchProcessor,
    job_semaphore: Arc<Semaphore>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl BackgroundProcessor {
    pub fn new(pool: PgPool) -> Self {
        let batch_processor = BatchProcessor::new(pool.clone());
        let job_semaphore = Arc::new(Semaphore::new(5)); // Limit concurrent background jobs

        Self {
            pool,
            batch_processor,
            job_semaphore,
            shutdown_tx: None,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the background processing loop
    pub async fn start(&mut self) -> Result<(), sqlx::Error> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("Background processor is already running");
            return Ok(());
        }

        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let pool = self.pool.clone();
        let job_semaphore = self.job_semaphore.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            info!("Background processor started");

            loop {
                // Check for shutdown signal with timeout
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Background processor received shutdown signal");
                        break;
                    }
                    _ = sleep(Duration::from_secs(5)) => {
                        // Continue with normal processing
                    }
                }

                if !is_running.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // Try to acquire a permit for job processing
                if let Ok(_permit) = job_semaphore.clone().try_acquire_owned() {
                    let pool_clone = pool.clone();

                    // Spawn a task to process the next job
                    tokio::spawn(async move {
                        // The permit will be automatically released when the task completes
                        if let Err(e) = Self::process_next_job(pool_clone).await {
                            error!("Error processing background job: {}", e);
                        }
                    });
                }

                // Small delay to prevent tight loop
                sleep(Duration::from_millis(100)).await;
            }

            is_running.store(false, std::sync::atomic::Ordering::Relaxed);
            info!("Background processor stopped");
        });

        Ok(())
    }

    /// Stop the background processing loop
    pub async fn stop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
        self.is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Wait for any running jobs to complete (with timeout)
        let start = Instant::now();
        while self.is_running.load(std::sync::atomic::Ordering::Relaxed)
            && start.elapsed() < Duration::from_secs(30)
        {
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Create a new background job for processing
    #[instrument(skip(pool, payload))]
    pub async fn create_job(
        pool: &PgPool,
        auth: &AuthContext,
        raw_ingestion_id: Uuid,
        payload: &IngestPayload,
        config: Option<IngestBatchConfig>,
    ) -> Result<Uuid, sqlx::Error> {
        let total_metrics = payload.data.metrics.len() + payload.data.workouts.len();
        let job_config = config.unwrap_or_default();

        let job_id = sqlx::query!(
            r#"
            INSERT INTO processing_jobs (
                user_id, api_key_id, raw_ingestion_id, job_type, 
                total_metrics, config
            ) 
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
            auth.user.id,
            auth.api_key.id,
            raw_ingestion_id,
            JobType::IngestBatch as JobType,
            total_metrics as i32,
            serde_json::to_value(job_config).unwrap_or(serde_json::Value::Null)
        )
        .fetch_one(pool)
        .await?
        .id;

        // Update raw_ingestions record with job_id
        sqlx::query!(
            "UPDATE raw_ingestions SET processing_job_id = $1, processing_status = 'pending' WHERE id = $2",
            job_id,
            raw_ingestion_id
        )
        .execute(pool)
        .await?;

        info!(
            job_id = %job_id,
            user_id = %auth.user.id,
            total_metrics = total_metrics,
            "Created background processing job"
        );

        Ok(job_id)
    }

    /// Get job status
    pub async fn get_job_status(
        pool: &PgPool,
        job_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ProcessingJob>, sqlx::Error> {
        let job = sqlx::query_as!(
            ProcessingJob,
            r#"
            SELECT 
                id, user_id, api_key_id, raw_ingestion_id, status as "status: JobStatus", job_type as "job_type: JobType", priority,
                total_metrics, processed_metrics, failed_metrics, progress_percentage,
                created_at, started_at, completed_at,
                error_message, retry_count,
                config, result_summary
            FROM processing_jobs 
            WHERE id = $1 AND user_id = $2
            "#,
            job_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(job)
    }

    /// Process the next pending job
    async fn process_next_job(pool: PgPool) -> Result<(), sqlx::Error> {
        // Get the next job to process
        let job_result = sqlx::query!(
            r#"
            SELECT job_id, user_id, api_key_id, raw_ingestion_id, job_type, total_metrics, config
            FROM get_next_job_for_processing()
            "#
        )
        .fetch_optional(&pool)
        .await?;

        let Some(job_row) = job_result else {
            // No pending jobs
            return Ok(());
        };

        let Some(job_id) = job_row.job_id else {
            error!("Background job missing job_id");
            return Err(sqlx::Error::Protocol(
                "Invalid job data: missing job_id".into(),
            ));
        };
        let Some(user_id) = job_row.user_id else {
            error!(job_id = %job_id, "Background job missing user_id");
            return Err(sqlx::Error::Protocol(
                "Invalid job data: missing user_id".into(),
            ));
        };
        let Some(raw_ingestion_id) = job_row.raw_ingestion_id else {
            error!(job_id = %job_id, user_id = %user_id, "Background job missing raw_ingestion_id");
            return Err(sqlx::Error::Protocol(
                "Invalid job data: missing raw_ingestion_id".into(),
            ));
        };
        let total_metrics = job_row.total_metrics.unwrap_or(0);

        info!(
            job_id = %job_id,
            user_id = %user_id,
            total_metrics = total_metrics,
            "Processing background job"
        );

        let start_time = Instant::now();

        // Get the raw payload data
        let raw_data = sqlx::query!(
            "SELECT raw_payload FROM raw_ingestions WHERE id = $1",
            raw_ingestion_id
        )
        .fetch_one(&pool)
        .await?;

        // Deserialize the payload
        let payload: IngestPayload = match serde_json::from_value(raw_data.raw_payload) {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("Failed to deserialize payload: {}", e);
                error!(job_id = %job_id, error = %error_msg, "Job failed");

                Self::complete_job(&pool, job_id, JobStatus::Failed, None, Some(error_msg)).await?;

                return Ok(());
            }
        };

        // Configure batch processor based on job config
        let mut batch_config = BatchConfig::default();
        if let Some(config_value) = job_row.config {
            if let Ok(job_config) = serde_json::from_value::<IngestBatchConfig>(config_value) {
                batch_config.enable_parallel_processing = job_config.enable_parallel_processing;
                if let Some(chunk_override) = job_config.chunk_size_override {
                    batch_config.chunk_size = chunk_override;
                }
            }
        }

        let batch_processor = BatchProcessor::with_config(pool.clone(), batch_config);

        // Process the batch
        let processing_result = batch_processor.process_batch(user_id, payload).await;

        let processing_time = start_time.elapsed();

        // Create result summary
        let mut metrics_by_type = std::collections::HashMap::new();
        // Note: This is a simplified summary - in practice you'd want more detailed metrics
        metrics_by_type.insert("total".to_string(), processing_result.processed_count);

        let result_summary = JobResultSummary {
            total_metrics_processed: processing_result.processed_count,
            metrics_by_type,
            validation_errors: processing_result.failed_count,
            database_errors: 0, // Could be tracked separately
            processing_time_ms: processing_time.as_millis() as u64,
            duplicates_removed: processing_result
                .deduplication_stats
                .map(|s| s.total_duplicates)
                .unwrap_or(0),
            final_status: if processing_result.errors.is_empty() {
                "success".to_string()
            } else {
                "partial_failure".to_string()
            },
        };

        let final_status = if processing_result.errors.is_empty() {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };

        let error_message = if !processing_result.errors.is_empty() {
            Some(format!(
                "Processing completed with {} errors",
                processing_result.errors.len()
            ))
        } else {
            None
        };

        // Complete the job
        Self::complete_job(
            &pool,
            job_id,
            final_status,
            match serde_json::to_value(result_summary) {
                Ok(value) => Some(value),
                Err(e) => {
                    error!(job_id = %job_id, error = %e, "Failed to serialize result summary");
                    None
                }
            },
            error_message,
        )
        .await?;

        info!(
            job_id = %job_id,
            processed_count = processing_result.processed_count,
            failed_count = processing_result.failed_count,
            processing_time_ms = processing_time.as_millis(),
            "Background job completed"
        );

        Ok(())
    }

    /// Update job progress
    async fn update_job_progress(
        pool: &PgPool,
        job_id: Uuid,
        processed_metrics: i32,
        failed_metrics: i32,
        error_message: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "SELECT update_job_progress($1, $2, $3, $4)",
            job_id,
            processed_metrics,
            failed_metrics,
            error_message
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Complete a job with final status and results
    async fn complete_job(
        pool: &PgPool,
        job_id: Uuid,
        status: JobStatus,
        result_summary: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "SELECT complete_job($1, $2, $3, $4)",
            job_id,
            status.as_str(),
            result_summary,
            error_message
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Clean up old completed jobs
    pub async fn cleanup_old_jobs(pool: &PgPool) -> Result<i32, sqlx::Error> {
        let result = sqlx::query!("SELECT cleanup_old_jobs() as deleted_count")
            .fetch_one(pool)
            .await?;

        let deleted_count = result.deleted_count.unwrap_or(0);

        if deleted_count > 0 {
            info!(
                deleted_count = deleted_count,
                "Cleaned up old background jobs"
            );
        }

        Ok(deleted_count)
    }
}
