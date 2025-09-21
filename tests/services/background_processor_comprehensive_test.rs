use self_sensored::config::BatchConfig;
use self_sensored::models::enums::{JobStatus, JobType};
use self_sensored::models::{
    IngestBatchConfig, IngestData, IngestPayload, JobResultSummary, ProcessingJob,
};
use self_sensored::services::auth::{ApiKey, AuthContext, User};
use self_sensored::services::background_processor::BackgroundProcessor;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

async fn setup_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_auth_context(pool: &PgPool) -> AuthContext {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW()) ON CONFLICT (id) DO NOTHING",
        user_id,
        format!("test_{}@example.com", user_id)
    )
    .execute(pool)
    .await
    .unwrap();

    // Create test API key
    sqlx::query!(
        r#"
        INSERT INTO api_keys (id, user_id, key_hash, name, created_at, last_used_at)
        VALUES ($1, $2, $3, $4, NOW(), NOW())
        ON CONFLICT (id) DO NOTHING
        "#,
        api_key_id,
        user_id,
        "test_hash",
        "Test Key"
    )
    .execute(pool)
    .await
    .unwrap();

    AuthContext {
        user: User {
            id: user_id,
            email: format!("test_{}@example.com", user_id),
            created_at: chrono::Utc::now(),
        },
        api_key: ApiKey {
            id: api_key_id,
            user_id,
            key_hash: "test_hash".to_string(),
            name: "Test Key".to_string(),
            created_at: chrono::Utc::now(),
            last_used_at: Some(chrono::Utc::now()),
            expires_at: None,
            is_active: true,
        },
    }
}

async fn create_test_payload() -> IngestPayload {
    IngestPayload {
        data: IngestData {
            metrics: vec![],
            workouts: vec![],
        },
    }
}

async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Clean up in reverse order of foreign key constraints
    sqlx::query!("DELETE FROM processing_jobs WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM raw_ingestions WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_background_processor_creation(pool: PgPool) -> sqlx::Result<()> {
    let processor = BackgroundProcessor::new(pool);
    assert!(!processor.is_running.load(Ordering::Relaxed));
    Ok(())
}

#[sqlx::test]
async fn test_background_processor_start_stop(pool: PgPool) -> sqlx::Result<()> {
    let mut processor = BackgroundProcessor::new(pool);

    // Start the processor
    processor.start().await?;
    sleep(Duration::from_millis(100)).await;
    assert!(processor.is_running.load(Ordering::Relaxed));

    // Stop the processor
    processor.stop().await;
    sleep(Duration::from_millis(200)).await;
    assert!(!processor.is_running.load(Ordering::Relaxed));

    Ok(())
}

#[sqlx::test]
async fn test_background_processor_double_start(pool: PgPool) -> sqlx::Result<()> {
    let mut processor = BackgroundProcessor::new(pool);

    // First start should succeed
    processor.start().await?;
    sleep(Duration::from_millis(100)).await;

    // Second start should return Ok but not start another loop
    processor.start().await?;

    // Stop the processor
    processor.stop().await;

    Ok(())
}

#[sqlx::test]
async fn test_create_job(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;
    let payload = create_test_payload().await;

    // Create raw ingestion first
    let raw_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        raw_id,
        auth.user.id,
        auth.api_key.id,
        serde_json::to_value(&payload).unwrap(),
        "test_hash"
    )
    .execute(&pool)
    .await?;

    // Create job
    let job_id = BackgroundProcessor::create_job(&pool, &auth, raw_id, &payload, None).await?;
    assert_ne!(job_id, Uuid::nil());

    // Verify job was created
    let job_exists = sqlx::query!("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id = $1)", job_id)
        .fetch_one(&pool)
        .await?
        .exists
        .unwrap_or(false);
    assert!(job_exists);

    // Verify raw_ingestions was updated
    let raw_ingestion = sqlx::query!(
        "SELECT processing_job_id, processing_status FROM raw_ingestions WHERE id = $1",
        raw_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(raw_ingestion.processing_job_id, Some(job_id));
    assert_eq!(raw_ingestion.processing_status, Some("pending".to_string()));

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_create_job_with_config(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;
    let payload = create_test_payload().await;

    // Create raw ingestion
    let raw_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        raw_id,
        auth.user.id,
        auth.api_key.id,
        serde_json::to_value(&payload).unwrap(),
        "test_hash"
    )
    .execute(&pool)
    .await?;

    // Create job with custom config
    let config = IngestBatchConfig {
        enable_parallel_processing: true,
        chunk_size_override: Some(100),
        max_retries: Some(5),
    };

    let job_id = BackgroundProcessor::create_job(&pool, &auth, raw_id, &payload, Some(config))
        .await?;

    // Verify config was stored
    let job_config = sqlx::query!(
        "SELECT config FROM processing_jobs WHERE id = $1",
        job_id
    )
    .fetch_one(&pool)
    .await?
    .config;

    assert!(job_config.is_some());
    let stored_config: IngestBatchConfig = serde_json::from_value(job_config.unwrap()).unwrap();
    assert!(stored_config.enable_parallel_processing);
    assert_eq!(stored_config.chunk_size_override, Some(100));

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_get_job_status(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;
    let payload = create_test_payload().await;

    // Create raw ingestion and job
    let raw_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        raw_id,
        auth.user.id,
        auth.api_key.id,
        serde_json::to_value(&payload).unwrap(),
        "test_hash"
    )
    .execute(&pool)
    .await?;

    let job_id = BackgroundProcessor::create_job(&pool, &auth, raw_id, &payload, None).await?;

    // Get job status
    let job_status = BackgroundProcessor::get_job_status(&pool, job_id, auth.user.id).await?;
    assert!(job_status.is_some());

    let job = job_status.unwrap();
    assert_eq!(job.id, job_id);
    assert_eq!(job.user_id, auth.user.id);
    assert_eq!(job.status, JobStatus::Pending);

    // Try to get status for wrong user (should return None)
    let wrong_user_status = BackgroundProcessor::get_job_status(&pool, job_id, Uuid::new_v4()).await?;
    assert!(wrong_user_status.is_none());

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_cleanup_old_jobs(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;

    // Create an old completed job (manually set completed_at to past date)
    let old_job_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO processing_jobs (
            id, user_id, api_key_id, job_type, status,
            total_metrics, processed_metrics, failed_metrics,
            created_at, completed_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW() - INTERVAL '40 days', NOW() - INTERVAL '40 days')
        "#,
        old_job_id,
        auth.user.id,
        auth.api_key.id,
        JobType::IngestBatch as JobType,
        JobStatus::Completed as JobStatus,
        10,
        10,
        0
    )
    .execute(&pool)
    .await?;

    // Create a recent job (should not be cleaned up)
    let recent_job_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO processing_jobs (
            id, user_id, api_key_id, job_type, status,
            total_metrics, processed_metrics, failed_metrics,
            created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#,
        recent_job_id,
        auth.user.id,
        auth.api_key.id,
        JobType::IngestBatch as JobType,
        JobStatus::Pending as JobStatus,
        5,
        0,
        0
    )
    .execute(&pool)
    .await?;

    // Run cleanup
    let deleted_count = BackgroundProcessor::cleanup_old_jobs(&pool).await?;

    // Old job should be deleted
    let old_exists = sqlx::query!("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id = $1)", old_job_id)
        .fetch_one(&pool)
        .await?
        .exists
        .unwrap_or(false);
    assert!(!old_exists);

    // Recent job should still exist
    let recent_exists = sqlx::query!("SELECT EXISTS(SELECT 1 FROM processing_jobs WHERE id = $1)", recent_job_id)
        .fetch_one(&pool)
        .await?
        .exists
        .unwrap_or(false);
    assert!(recent_exists);

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_process_job_with_invalid_payload(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;

    // Create raw ingestion with invalid JSON
    let raw_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        raw_id,
        auth.user.id,
        auth.api_key.id,
        serde_json::json!({ "invalid": "data" }),
        "invalid_hash"
    )
    .execute(&pool)
    .await?;

    // Create job
    let job_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO processing_jobs (
            id, user_id, api_key_id, raw_ingestion_id, job_type,
            status, total_metrics
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        job_id,
        auth.user.id,
        auth.api_key.id,
        raw_id,
        JobType::IngestBatch as JobType,
        JobStatus::Pending as JobStatus,
        0
    )
    .execute(&pool)
    .await?;

    // Start processor and let it process the invalid job
    let mut processor = BackgroundProcessor::new(pool.clone());
    processor.start().await?;

    // Wait for job to be processed
    let mut attempts = 0;
    let max_attempts = 20;
    let mut job_failed = false;

    while attempts < max_attempts {
        sleep(Duration::from_millis(500)).await;

        let job_status = sqlx::query!(
            "SELECT status, error_message FROM processing_jobs WHERE id = $1",
            job_id
        )
        .fetch_optional(&pool)
        .await?;

        if let Some(status) = job_status {
            if status.status == Some("failed".to_string()) {
                job_failed = true;
                assert!(status.error_message.is_some());
                assert!(status.error_message.unwrap().contains("Failed to deserialize"));
                break;
            }
        }

        attempts += 1;
    }

    processor.stop().await;
    assert!(job_failed, "Job should have failed due to invalid payload");

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_concurrent_job_processing(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;
    let mut processor = BackgroundProcessor::new(pool.clone());

    // Create multiple jobs
    let mut job_ids = Vec::new();
    for i in 0..3 {
        let raw_id = Uuid::new_v4();
        let payload = create_test_payload().await;

        sqlx::query!(
            r#"
            INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            raw_id,
            auth.user.id,
            auth.api_key.id,
            serde_json::to_value(&payload).unwrap(),
            format!("hash_{}", i)
        )
        .execute(&pool)
        .await?;

        let job_id = BackgroundProcessor::create_job(&pool, &auth, raw_id, &payload, None).await?;
        job_ids.push(job_id);
    }

    // Start processor
    processor.start().await?;

    // Wait for jobs to be processed
    sleep(Duration::from_secs(5)).await;

    // Check that jobs were processed
    for job_id in &job_ids {
        let status = sqlx::query!(
            "SELECT status FROM processing_jobs WHERE id = $1",
            job_id
        )
        .fetch_optional(&pool)
        .await?;

        if let Some(row) = status {
            // Job should be either completed or failed (not stuck in pending)
            assert_ne!(row.status, Some("pending".to_string()));
        }
    }

    processor.stop().await;
    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[sqlx::test]
async fn test_job_result_summary_serialization(pool: PgPool) -> sqlx::Result<()> {
    let auth = create_test_auth_context(&pool).await;

    // Create a completed job with result summary
    let job_id = Uuid::new_v4();
    let mut metrics_by_type = std::collections::HashMap::new();
    metrics_by_type.insert("heart_rate".to_string(), 100);
    metrics_by_type.insert("blood_pressure".to_string(), 50);

    let result_summary = JobResultSummary {
        total_metrics_processed: 150,
        metrics_by_type,
        validation_errors: 5,
        database_errors: 2,
        processing_time_ms: 1234,
        duplicates_removed: 10,
        final_status: "partial_failure".to_string(),
    };

    sqlx::query!(
        r#"
        INSERT INTO processing_jobs (
            id, user_id, api_key_id, job_type, status,
            total_metrics, processed_metrics, failed_metrics,
            result_summary
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        job_id,
        auth.user.id,
        auth.api_key.id,
        JobType::IngestBatch as JobType,
        JobStatus::Completed as JobStatus,
        155,
        150,
        5,
        serde_json::to_value(&result_summary).unwrap()
    )
    .execute(&pool)
    .await?;

    // Retrieve and verify result summary
    let job = BackgroundProcessor::get_job_status(&pool, job_id, auth.user.id).await?;
    assert!(job.is_some());

    let retrieved_job = job.unwrap();
    assert!(retrieved_job.result_summary.is_some());

    let retrieved_summary: JobResultSummary = serde_json::from_value(retrieved_job.result_summary.unwrap()).unwrap();
    assert_eq!(retrieved_summary.total_metrics_processed, 150);
    assert_eq!(retrieved_summary.validation_errors, 5);
    assert_eq!(retrieved_summary.processing_time_ms, 1234);

    cleanup_test_data(&pool, auth.user.id).await;
    Ok(())
}

#[tokio::test]
async fn test_semaphore_limiting() {
    // This test verifies that the semaphore correctly limits concurrent jobs
    let pool = setup_test_pool().await;
    let mut processor = BackgroundProcessor::new(pool.clone());

    // The processor has a semaphore with 5 permits
    let auth = create_test_auth_context(&pool).await;

    // Create 10 jobs
    for i in 0..10 {
        let raw_id = Uuid::new_v4();
        let payload = create_test_payload().await;

        sqlx::query!(
            r#"
            INSERT INTO raw_ingestions (id, user_id, api_key_id, raw_payload, payload_hash)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            raw_id,
            auth.user.id,
            auth.api_key.id,
            serde_json::to_value(&payload).unwrap(),
            format!("semaphore_test_{}", i)
        )
        .execute(&pool)
        .await
        .unwrap();

        BackgroundProcessor::create_job(&pool, &auth, raw_id, &payload, None)
            .await
            .unwrap();
    }

    processor.start().await.unwrap();

    // Let processor run briefly
    sleep(Duration::from_millis(500)).await;

    // Check that not all jobs are being processed simultaneously
    let processing_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM processing_jobs WHERE user_id = $1 AND status = 'running'",
        auth.user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .count
    .unwrap_or(0);

    // Should have at most 5 jobs running concurrently (semaphore limit)
    assert!(processing_count <= 5);

    processor.stop().await;
    cleanup_test_data(&pool, auth.user.id).await;
}