use actix_web::{web, HttpResponse, Result};
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::{
    ApiResponse, JobStatusResponse,
};
use crate::models::enums::{JobStatus, JobType};
use crate::services::auth::AuthContext;
use crate::services::background_processor::BackgroundProcessor;

/// Get background job status by job ID
#[instrument(skip(pool))]
pub async fn get_job_status(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    info!(
        user_id = %auth.user.id,
        job_id = %job_id,
        "Getting background job status"
    );

    match BackgroundProcessor::get_job_status(&pool, job_id, auth.user.id).await {
        Ok(Some(job)) => {
            let response = JobStatusResponse::from(job);
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Ok(None) => {
            error!(
                user_id = %auth.user.id,
                job_id = %job_id,
                "Job not found"
            );
            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "Job not found or access denied".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                job_id = %job_id,
                error = %e,
                "Failed to get job status"
            );
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to get job status".to_string(),
            )))
        }
    }
}

/// List background jobs for the authenticated user
#[instrument(skip(pool))]
pub async fn list_user_jobs(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<ListJobsQuery>,
) -> Result<HttpResponse> {
    info!(
        user_id = %auth.user.id,
        limit = query.limit,
        offset = query.offset,
        status_filter = ?query.status,
        "Listing background jobs for user"
    );

    let limit = query.limit.unwrap_or(20).min(100) as i64; // Max 100 results
    let offset = query.offset.unwrap_or(0) as i64;

    let status_filter = query.status;

    let jobs_result = match status_filter {
        Some(status) => {
            sqlx::query_as!(
                crate::models::ProcessingJob,
                r#"
                SELECT 
                    id, user_id, api_key_id, raw_ingestion_id, status as "status: JobStatus", job_type as "job_type: JobType", priority,
                    total_metrics, processed_metrics, failed_metrics, progress_percentage,
                    created_at, started_at, completed_at,
                    error_message, retry_count,
                    config, result_summary
                FROM processing_jobs 
                WHERE user_id = $1 AND status = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
                auth.user.id,
                status as JobStatus,
                limit,
                offset
            )
            .fetch_all(pool.get_ref())
            .await
        }
        None => {
            sqlx::query_as!(
                crate::models::ProcessingJob,
                r#"
                SELECT 
                    id, user_id, api_key_id, raw_ingestion_id, status as "status: JobStatus", job_type as "job_type: JobType", priority,
                    total_metrics, processed_metrics, failed_metrics, progress_percentage,
                    created_at, started_at, completed_at,
                    error_message, retry_count,
                    config, result_summary
                FROM processing_jobs 
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                auth.user.id,
                limit,
                offset
            )
            .fetch_all(pool.get_ref())
            .await
        }
    };

    match jobs_result {
        Ok(jobs) => {
            let job_responses: Vec<JobStatusResponse> = jobs
                .into_iter()
                .map(JobStatusResponse::from)
                .collect();

            Ok(HttpResponse::Ok().json(ApiResponse::success(job_responses)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to list user jobs"
            );
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to list jobs".to_string(),
            )))
        }
    }
}

/// Cancel a background job (if it's still pending)
#[instrument(skip(pool))]
pub async fn cancel_job(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    info!(
        user_id = %auth.user.id,
        job_id = %job_id,
        "Attempting to cancel background job"
    );

    // Only allow canceling pending jobs
    let update_result = sqlx::query!(
        r#"
        UPDATE processing_jobs 
        SET status = 'failed', 
            error_message = 'Cancelled by user',
            completed_at = NOW()
        WHERE id = $1 AND user_id = $2 AND status = 'pending'
        "#,
        job_id,
        auth.user.id
    )
    .execute(pool.get_ref())
    .await;

    match update_result {
        Ok(result) if result.rows_affected() > 0 => {
            info!(
                user_id = %auth.user.id,
                job_id = %job_id,
                "Background job cancelled successfully"
            );

            // Also update the raw_ingestions status
            let _ = sqlx::query!(
                "UPDATE raw_ingestions SET processing_status = 'error', processing_errors = jsonb_build_object('error', 'Cancelled by user') WHERE processing_job_id = $1",
                job_id
            )
            .execute(pool.get_ref())
            .await;

            Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "job_id": job_id,
                "status": "cancelled",
                "message": "Job cancelled successfully"
            }))))
        }
        Ok(_) => {
            error!(
                user_id = %auth.user.id,
                job_id = %job_id,
                "Job not found or cannot be cancelled"
            );
            Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                "Job not found, access denied, or job cannot be cancelled (already processing/completed)".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                job_id = %job_id,
                error = %e,
                "Failed to cancel job"
            );
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to cancel job".to_string(),
            )))
        }
    }
}

/// Get job statistics for the authenticated user
#[instrument(skip(pool))]
pub async fn get_job_statistics(
    pool: web::Data<PgPool>,
    auth: AuthContext,
) -> Result<HttpResponse> {
    info!(
        user_id = %auth.user.id,
        "Getting job statistics for user"
    );

    let stats_result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as total_jobs,
            COUNT(*) FILTER (WHERE status = 'pending') as pending_jobs,
            COUNT(*) FILTER (WHERE status = 'processing') as processing_jobs,
            COUNT(*) FILTER (WHERE status = 'completed') as completed_jobs,
            COUNT(*) FILTER (WHERE status = 'failed') as failed_jobs,
            AVG(progress_percentage) FILTER (WHERE status = 'processing') as avg_progress,
            SUM(total_metrics) FILTER (WHERE status = 'completed') as total_metrics_processed
        FROM processing_jobs 
        WHERE user_id = $1 AND created_at > NOW() - INTERVAL '30 days'
        "#,
        auth.user.id
    )
    .fetch_one(pool.get_ref())
    .await;

    match stats_result {
        Ok(stats) => {
            let response = serde_json::json!({
                "total_jobs": stats.total_jobs.unwrap_or(0),
                "pending_jobs": stats.pending_jobs.unwrap_or(0),
                "processing_jobs": stats.processing_jobs.unwrap_or(0),
                "completed_jobs": stats.completed_jobs.unwrap_or(0),
                "failed_jobs": stats.failed_jobs.unwrap_or(0),
                "average_processing_progress": stats.avg_progress
                    .map(|p| p.to_string().parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(0.0),
                "total_metrics_processed_30d": stats.total_metrics_processed.unwrap_or(0),
                "period": "last_30_days"
            });

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to get job statistics"
            );
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to get job statistics".to_string(),
            )))
        }
    }
}

/// Query parameters for listing jobs
#[derive(Debug, serde::Deserialize)]
pub struct ListJobsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<JobStatus>,
}