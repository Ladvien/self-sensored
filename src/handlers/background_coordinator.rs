use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::config::BatchConfig;
use crate::models::enums::{JobStatus, JobType};
use crate::services::auth::AuthContext;

/// Background job types supported by the system
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundJobType {
    IngestBatch,
    DataExport,
    DataMigration,
    HealthCheck,
}

impl BackgroundJobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackgroundJobType::IngestBatch => "ingest_batch",
            BackgroundJobType::DataExport => "data_export", 
            BackgroundJobType::DataMigration => "data_migration",
            BackgroundJobType::HealthCheck => "health_check",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ingest_batch" => Some(BackgroundJobType::IngestBatch),
            "data_export" => Some(BackgroundJobType::DataExport),
            "data_migration" => Some(BackgroundJobType::DataMigration),
            "health_check" => Some(BackgroundJobType::HealthCheck),
            _ => None,
        }
    }
}

/// Priority levels for background jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 1,
    Normal = 5,
    High = 10,
}

impl From<JobPriority> for i32 {
    fn from(priority: JobPriority) -> i32 {
        priority as i32
    }
}

/// Configuration for creating a background job
#[derive(Debug, Clone)]
pub struct BackgroundJobConfig {
    pub job_type: BackgroundJobType,
    pub priority: JobPriority,
    pub max_retries: i32,
    pub batch_config: Option<BatchConfig>,
    pub estimated_duration_mins: Option<i32>,
    pub custom_config: HashMap<String, Value>,
}

impl Default for BackgroundJobConfig {
    fn default() -> Self {
        Self {
            job_type: BackgroundJobType::IngestBatch,
            priority: JobPriority::Normal,
            max_retries: 3,
            batch_config: None,
            estimated_duration_mins: None,
            custom_config: HashMap::new(),
        }
    }
}

/// Background job coordinator for managing async processing
pub struct BackgroundJobCoordinator {
    pool: PgPool,
}

impl BackgroundJobCoordinator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new background job for large payload processing
    pub async fn create_ingest_job(
        &self,
        auth: &AuthContext,
        raw_ingestion_id: Uuid,
        total_metrics: usize,
        config: BackgroundJobConfig,
    ) -> Result<Uuid, sqlx::Error> {
        let job_config = serde_json::to_value(&config.custom_config)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

        let _estimated_completion = if let Some(duration_mins) = config.estimated_duration_mins {
            Some(sqlx::types::chrono::Utc::now() + chrono::Duration::minutes(duration_mins as i64))
        } else {
            // Estimate based on metrics count (rough: 1000 metrics per minute)
            let estimated_mins = (total_metrics / 1000).max(1) as i64;
            Some(sqlx::types::chrono::Utc::now() + chrono::Duration::minutes(estimated_mins))
        };

        let job_id = sqlx::query!(
            r#"
            INSERT INTO processing_jobs (
                user_id, api_key_id, raw_ingestion_id, status, job_type, priority,
                total_metrics, config
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
            auth.user.id,
            auth.api_key.id,
            raw_ingestion_id,
            JobStatus::Pending as JobStatus,
            JobType::IngestBatch as JobType,
            i32::from(config.priority),
            total_metrics as i32,
            job_config
        )
        .fetch_one(&self.pool)
        .await?
        .id;

        // Link the raw ingestion to this job
        sqlx::query!(
            "UPDATE raw_ingestions SET processing_job_id = $1 WHERE id = $2",
            job_id,
            raw_ingestion_id
        )
        .execute(&self.pool)
        .await?;

        info!(
            user_id = %auth.user.id,
            job_id = %job_id,
            total_metrics = total_metrics,
            priority = ?config.priority,
            "Created background job for large payload processing"
        );

        Ok(job_id)
    }

    /// Check if a similar job already exists for this raw ingestion
    pub async fn check_existing_job(&self, raw_ingestion_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        let existing = sqlx::query!(
            r#"
            SELECT id FROM processing_jobs 
            WHERE raw_ingestion_id = $1 
            AND status IN ('pending', 'processing')
            LIMIT 1
            "#,
            raw_ingestion_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(existing.map(|row| row.id))
    }

    /// Get job status and progress
    pub async fn get_job_status(&self, job_id: Uuid) -> Result<Option<JobStatusInfo>, sqlx::Error> {
        let job = sqlx::query!(
            r#"
            SELECT 
                status as "status: JobStatus", job_type as "job_type: JobType", priority, total_metrics, processed_metrics, 
                failed_metrics, progress_percentage, created_at, started_at, 
                completed_at, error_message, retry_count,
                result_summary
            FROM processing_jobs 
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(job) = job {
            Ok(Some(JobStatusInfo {
                id: job_id,
                status: job.status.as_str().to_string(),
                job_type: BackgroundJobType::from_str(&job.job_type.as_str()).unwrap_or(BackgroundJobType::IngestBatch),
                priority: match job.priority {
                    1 => JobPriority::Low,
                    10 => JobPriority::High,
                    _ => JobPriority::Normal,
                },
                total_metrics: job.total_metrics.unwrap_or(0) as usize,
                processed_metrics: job.processed_metrics.unwrap_or(0) as usize,
                failed_metrics: job.failed_metrics.unwrap_or(0) as usize,
                progress_percentage: job.progress_percentage.unwrap_or(sqlx::types::BigDecimal::from(0)).to_string().parse::<f64>().unwrap_or(0.0),
                created_at: job.created_at,
                started_at: job.started_at,
                completed_at: job.completed_at,
                error_message: job.error_message,
                retry_count: job.retry_count.unwrap_or(0),
                result_summary: job.result_summary,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update job progress (called by background workers)
    pub async fn update_progress(
        &self,
        job_id: Uuid,
        processed_metrics: usize,
        failed_metrics: usize,
        error_message: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let processed = processed_metrics as i32;
        let failed = failed_metrics as i32;
        
        // Update metrics first
        sqlx::query!(
            r#"
            UPDATE processing_jobs 
            SET 
                processed_metrics = $2,
                failed_metrics = $3,
                error_message = COALESCE($4, error_message)
            WHERE id = $1
            "#,
            job_id,
            processed,
            failed,
            error_message.as_deref()
        )
        .execute(&self.pool)
        .await?;

        // Update progress percentage in separate query
        sqlx::query!(
            r#"
            UPDATE processing_jobs 
            SET progress_percentage = CASE 
                WHEN total_metrics > 0 THEN ((processed_metrics::decimal / total_metrics) * 100.0)
                ELSE 0.0 
            END
            WHERE id = $1
            "#,
            job_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Complete a job with final results
    pub async fn complete_job(
        &self,
        job_id: Uuid,
        success: bool,
        result_summary: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE processing_jobs 
            SET 
                status = $2,
                completed_at = CURRENT_TIMESTAMP,
                result_summary = $3,
                error_message = COALESCE($4, error_message)
            WHERE id = $1
            "#,
            job_id,
            if success { JobStatus::Completed } else { JobStatus::Failed } as JobStatus,
            result_summary,
            error_message
        )
        .execute(&self.pool)
        .await?;

        info!(
            job_id = %job_id,
            success = success,
            "Background job completed"
        );

        Ok(())
    }

    /// Get next job for processing (used by background workers)
    pub async fn get_next_job(&self) -> Result<Option<BackgroundJob>, sqlx::Error> {
        let job = sqlx::query!(
            r#"
            SELECT job_id, user_id, api_key_id, raw_ingestion_id, job_type, total_metrics, config
            FROM get_next_job_for_processing()
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(job) = job {
            let config: HashMap<String, Value> = job.config
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            Ok(Some(BackgroundJob {
                id: job.job_id.unwrap(),
                user_id: job.user_id.unwrap(),
                api_key_id: job.api_key_id.unwrap(),
                raw_ingestion_id: job.raw_ingestion_id.unwrap(),
                job_type: BackgroundJobType::from_str(&job.job_type.unwrap_or_default())
                    .unwrap_or(BackgroundJobType::IngestBatch),
                total_metrics: job.total_metrics.unwrap_or(0) as usize,
                config,
            }))
        } else {
            Ok(None)
        }
    }

    /// Clean up old completed jobs
    pub async fn cleanup_old_jobs(&self) -> Result<usize, sqlx::Error> {
        let result = sqlx::query!(
            "SELECT cleanup_old_jobs() as deleted_count"
        )
        .fetch_one(&self.pool)
        .await?;

        let deleted_count = result.deleted_count.unwrap_or(0) as usize;
        if deleted_count > 0 {
            info!("Cleaned up {} old background jobs", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Get job queue statistics
    pub async fn get_queue_stats(&self) -> Result<QueueStats, sqlx::Error> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
                COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_count,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
                AVG(EXTRACT(EPOCH FROM (completed_at - started_at))) FILTER (WHERE status = 'completed') as avg_processing_time_secs
            FROM processing_jobs
            WHERE created_at > NOW() - INTERVAL '24 hours'
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(QueueStats {
            pending_jobs: stats.pending_count.unwrap_or(0) as usize,
            processing_jobs: stats.processing_count.unwrap_or(0) as usize,
            completed_jobs_24h: stats.completed_count.unwrap_or(0) as usize,
            failed_jobs_24h: stats.failed_count.unwrap_or(0) as usize,
            avg_processing_time_secs: stats.avg_processing_time_secs.unwrap_or(sqlx::types::BigDecimal::from(0)).to_string().parse::<f64>().unwrap_or(0.0),
        })
    }
}

/// Job status information with detailed progress
#[derive(Debug, Clone)]
pub struct JobStatusInfo {
    pub id: Uuid,
    pub status: String,
    pub job_type: BackgroundJobType,
    pub priority: JobPriority,
    pub total_metrics: usize,
    pub processed_metrics: usize,
    pub failed_metrics: usize,
    pub progress_percentage: f64,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub started_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
    pub completed_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub result_summary: Option<serde_json::Value>,
}

/// Background job information for processing
#[derive(Debug, Clone)]
pub struct BackgroundJob {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_key_id: Uuid,
    pub raw_ingestion_id: Uuid,
    pub job_type: BackgroundJobType,
    pub total_metrics: usize,
    pub config: HashMap<String, Value>,
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending_jobs: usize,
    pub processing_jobs: usize,
    pub completed_jobs_24h: usize,
    pub failed_jobs_24h: usize,
    pub avg_processing_time_secs: f64,
}

/// Helper function to determine if payload should use background processing
pub fn should_use_background_processing(total_metrics: usize, payload_size_mb: f64) -> bool {
    const BACKGROUND_METRICS_THRESHOLD: usize = 25_000;
    const BACKGROUND_SIZE_THRESHOLD_MB: f64 = 50.0;
    
    total_metrics > BACKGROUND_METRICS_THRESHOLD || payload_size_mb > BACKGROUND_SIZE_THRESHOLD_MB
}

/// Migration path recommendation for moving to dedicated job queue
pub struct JobQueueMigrationPath {
    pub current_system: &'static str,
    pub recommended_systems: Vec<&'static str>,
    pub migration_steps: Vec<&'static str>,
    pub estimated_effort_days: u32,
}

impl Default for JobQueueMigrationPath {
    fn default() -> Self {
        Self {
            current_system: "Custom PostgreSQL-based job queue",
            recommended_systems: vec!["Sidekiq (Ruby)", "Celery (Python)", "RQ (Python)", "DelayedJob (Ruby)", "Faktory (Language-agnostic)"],
            migration_steps: vec![
                "1. Implement adapter pattern for job queue interface",
                "2. Create parallel implementation with dedicated queue system", 
                "3. Migrate job creation to use new system",
                "4. Update background workers to poll new queue",
                "5. Implement job result reconciliation",
                "6. Monitor both systems during transition period",
                "7. Deprecate PostgreSQL-based queue gradually",
                "8. Clean up database schema and functions"
            ],
            estimated_effort_days: 21, // 3 weeks for a proper migration
        }
    }
}