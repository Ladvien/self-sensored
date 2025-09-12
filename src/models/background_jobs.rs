use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::enums::{JobStatus, JobType};

/// Background processing job model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProcessingJob {
    pub id: Uuid,
    pub user_id: Uuid,
    pub api_key_id: Uuid,
    pub raw_ingestion_id: Uuid,
    pub status: JobStatus,
    pub job_type: JobType,
    pub priority: i32,

    // Progress tracking
    pub total_metrics: i32,
    pub processed_metrics: i32,
    pub failed_metrics: i32,
    pub progress_percentage: f64,

    // Timing information
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,

    // Error handling
    pub error_message: Option<String>,
    pub retry_count: i32,

    // Configuration and results
    pub config: serde_json::Value,
    pub result_summary: Option<serde_json::Value>,
}


/// Priority levels for job processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    Low = 1,
    Normal = 5,
    High = 10,
}

impl JobPriority {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Request to create a new background job
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub job_type: JobType,
    pub priority: Option<JobPriority>,
    pub config: Option<serde_json::Value>,
}

/// Response when creating a background job
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub message: String,
}

/// Response for job status inquiries
#[derive(Debug, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub user_id: Uuid,
    pub status: JobStatus,
    pub progress_percentage: f64,
    pub total_metrics: i32,
    pub processed_metrics: i32,
    pub failed_metrics: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub result_summary: Option<serde_json::Value>,
}

impl From<ProcessingJob> for JobStatusResponse {
    fn from(job: ProcessingJob) -> Self {
        Self {
            job_id: job.id,
            user_id: job.user_id,
            status: job.status,
            progress_percentage: job.progress_percentage,
            total_metrics: job.total_metrics,
            processed_metrics: job.processed_metrics,
            failed_metrics: job.failed_metrics,
            created_at: job.created_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            error_message: job.error_message,
            result_summary: job.result_summary,
        }
    }
}

/// Configuration for ingest batch jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestBatchConfig {
    pub enable_parallel_processing: bool,
    pub chunk_size_override: Option<usize>,
    pub timeout_seconds: Option<u64>,
    pub enable_progress_notifications: bool,
}

impl Default for IngestBatchConfig {
    fn default() -> Self {
        Self {
            enable_parallel_processing: true,
            chunk_size_override: None,
            timeout_seconds: Some(300), // 5 minutes default
            enable_progress_notifications: false,
        }
    }
}

/// Job result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultSummary {
    pub total_metrics_processed: usize,
    pub metrics_by_type: std::collections::HashMap<String, usize>,
    pub validation_errors: usize,
    pub database_errors: usize,
    pub processing_time_ms: u64,
    pub duplicates_removed: usize,
    pub final_status: String,
}
