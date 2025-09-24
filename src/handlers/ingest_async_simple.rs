use actix_web::{web, HttpRequest, HttpResponse, Result};
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::config::BatchConfig;
use crate::middleware::metrics::Metrics;
use crate::models::{ApiResponse, IngestResponse};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;

// Import our new modular architecture components
use super::payload_processor::{PayloadProcessor, PayloadProcessorConfig};
use super::timeout_manager::{TimeoutConfig, TimeoutManager};
// use super::background_coordinator::{
//     BackgroundJobCoordinator, BackgroundJobConfig, JobPriority,
//     should_use_background_processing
// };

/// Configuration constants - reduced from previous values for better performance
const LARGE_BATCH_THRESHOLD: usize = 5_000;
// Timeout reduced from 80s to 30s to prevent Cloudflare 524 errors
// and reduce connection pool pressure

/// Optimized async health data ingest endpoint with improved architecture
/// Uses modular components for timeout management, payload processing, and background jobs
#[instrument(skip(pool, raw_payload))]
pub async fn ingest_async_optimized_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    raw_payload: web::Bytes,
    req: HttpRequest,
) -> Result<HttpResponse> {
    // Initialize timeout manager with reduced timeout
    let timeout_config = TimeoutConfig {
        max_processing_seconds: 30, // Reduced from 80s
        large_batch_threshold: LARGE_BATCH_THRESHOLD,
        ..Default::default()
    };
    let timeout_manager = TimeoutManager::new(timeout_config);

    // Initialize payload processor with security limits
    let payload_config = PayloadProcessorConfig {
        max_payload_size: 200 * 1024 * 1024, // 200MB
        max_json_depth: 50,                  // Prevent deeply nested JSON attacks
        max_json_elements: 1_000_000,        // Prevent JSON bomb attacks
        ..Default::default()
    };
    let payload_processor = PayloadProcessor::new(payload_config);

    // Initialize background job coordinator
    // let background_coordinator = BackgroundJobCoordinator::new(pool.get_ref().clone());

    // Record ingest request start
    Metrics::record_ingest_request();

    // Extract client IP for audit logging
    let client_ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let payload_size = raw_payload.len();
    let payload_size_mb = payload_size as f64 / (1024.0 * 1024.0);

    info!(
        user_id = %auth.user.id,
        client_ip = %client_ip,
        payload_size = payload_size,
        payload_size_mb = payload_size_mb,
        "Starting optimized async ingest processing with improved architecture"
    );

    // Record data volume
    Metrics::record_data_volume("ingest", "received", payload_size as u64);

    // Parse payload using new modular payload processor with security limits
    let internal_payload = match payload_processor
        .parse_payload_with_timeout(&raw_payload, auth.user.id)
        .await
    {
        Ok(payload) => payload,
        Err(parse_error) => {
            error!("Payload processing error: {}", parse_error);
            Metrics::record_error("payload_processing", "/api/v1/ingest-async", "error");
            return Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error(parse_error.to_string()))
            );
        }
    };

    let total_metrics = internal_payload.data.metrics.len() + internal_payload.data.workouts.len();

    info!(
        user_id = %auth.user.id,
        total_metrics = total_metrics,
        payload_size_mb = payload_size_mb,
        threshold = LARGE_BATCH_THRESHOLD,
        "Determining processing approach with improved logic"
    );

    // Check if we should recommend background processing
    // if should_use_background_processing(total_metrics, payload_size_mb) {
    //     warn!(
    //         user_id = %auth.user.id,
    //         total_metrics = total_metrics,
    //         payload_size_mb = payload_size_mb,
    //         "Payload exceeds background processing thresholds - recommending background job"
    //     );
    // }

    // Store raw payload using new payload processor
    let raw_id = match super::payload_processor::store_raw_payload(
        &pool,
        &auth,
        &internal_payload,
        &client_ip,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to store raw payload"
            );
            return Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to process request".to_string(),
                )),
            );
        }
    };

    // If payload is very large, create background job and return early
    // Background processing temporarily disabled - needs database
    /*
    if timeout_manager.should_use_background_processing(total_metrics) {
        let job_config = BackgroundJobConfig {
            priority: if payload_size_mb > 100.0 { JobPriority::High } else { JobPriority::Normal },
            max_retries: 2, // Reduced retries for large payloads
            estimated_duration_mins: Some((total_metrics / 1000).max(5) as i32),
            ..Default::default()
        };

        match background_coordinator.create_ingest_job(&auth, raw_id, total_metrics, job_config).await {
            Ok(job_id) => {
                info!(
                    user_id = %auth.user.id,
                    job_id = %job_id,
                    total_metrics = total_metrics,
                    "Created background job for large payload"
                );

                return Ok(HttpResponse::Accepted().json(ApiResponse::success(IngestResponse {
                    success: true,
                    processed_count: 0,
                    failed_count: 0,
                    processing_time_ms: timeout_manager.elapsed_time().as_millis() as u64,
                    errors: vec![],
                })));
            }
            Err(e) => {
                error!(
                    user_id = %auth.user.id,
                    error = %e,
                    "Failed to create background job, falling back to synchronous processing"
                );
                // Continue with synchronous processing as fallback
            }
        }
    }
    */

    // Configure batch processor for optimal performance
    let batch_config = if total_metrics >= LARGE_BATCH_THRESHOLD {
        BatchConfig {
            max_retries: 2, // Reduce retries for speed
            initial_backoff_ms: 50,
            max_backoff_ms: 1000,
            enable_parallel_processing: true,
            max_concurrent_metric_types: 4, // Reduce concurrency for large batches
            chunk_size: 1000,
            memory_limit_mb: 1000.0,
            // Optimized chunk sizes for large batches - FIXED to prevent PostgreSQL parameter limit violations
            heart_rate_chunk_size: 4766, // 11 params: 52,426 total params (safe with complete cardiovascular data)
            blood_pressure_chunk_size: 8000, // 6 params: 48,000 total params (safe)
            sleep_chunk_size: 5200,      // 10 params: 52,000 total params (safe) - FIXED from 5000
            activity_chunk_size: 1450, // 36 params: 52,200 total params (safe with mobility + cycling + underwater metrics)
            respiratory_chunk_size: 7000,
            body_measurement_chunk_size: 3500,
            temperature_chunk_size: 6500, // 8 params: 52,000 total params (safe) - FIXED from 8000
            workout_chunk_size: 5000,
            blood_glucose_chunk_size: 6500,
            metabolic_chunk_size: 8700, // 6 params: optimized for metabolic data
            nutrition_chunk_size: 1600,
            // Reproductive Health Batch Processing (Privacy-Optimized for Large Batches)
            menstrual_chunk_size: 6500, // 8 params: optimized for high-frequency tracking
            fertility_chunk_size: 4300, // 12 params: privacy-optimized chunks
            // Environmental and Audio Exposure Batch Processing
            environmental_chunk_size: 3700, // 14 params: optimized for environmental data
            audio_exposure_chunk_size: 7000, // 7 params: optimized for audio exposure
            // Mental Health and Safety Batch Processing
            safety_event_chunk_size: 6500, // 8 params: optimized for safety events
            mindfulness_chunk_size: 5800,  // 9 params: optimized for mindfulness data
            mental_health_chunk_size: 5200, // 10 params: optimized for mental health data
            symptom_chunk_size: 5800,      // 9 params: optimized for symptom tracking
            hygiene_chunk_size: 6500,      // 8 params: optimized for hygiene tracking
            enable_progress_tracking: false, // Disable for speed
            enable_intra_batch_deduplication: true,
            enable_dual_write_activity_metrics: false, // Disable for async endpoint to prioritize speed
            // Privacy and Security Configuration for Reproductive Health
            enable_reproductive_health_encryption: true, // Always enable encryption for sensitive data
            reproductive_health_audit_logging: true,     // Always enable audit logging
        }
    } else {
        BatchConfig::default()
    };

    // Process with improved timeout management
    let processor = BatchProcessor::with_config(pool.get_ref().clone(), batch_config);

    let processing_result = match tokio::time::timeout(
        timeout_manager.get_processing_timeout(),
        processor.process_batch(auth.user.id, internal_payload),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            // Processing timed out - recommend background processing
            let timeout_error = timeout_manager.create_timeout_error(total_metrics);
            error!(
                user_id = %auth.user.id,
                total_metrics = total_metrics,
                timeout_seconds = timeout_manager.get_processing_timeout().as_secs(),
                "Processing timed out - payload too large for synchronous processing"
            );

            // Update raw ingestion status
            let _ = sqlx::query!(
                "UPDATE raw_ingestions SET processing_status = 'error', processing_errors = $1 WHERE id = $2",
                serde_json::json!([{"error_type": "timeout", "message": "Processing timeout - requires background processing"}]),
                raw_id
            )
            .execute(pool.get_ref())
            .await;

            Metrics::record_error("processing_timeout", "/api/v1/ingest-async", "error");

            return Ok(HttpResponse::RequestTimeout().json(ApiResponse::<()>::error(timeout_error)));
        }
    };

    // Warn if approaching timeout during processing
    timeout_manager.warn_if_approaching_timeout(auth.user.id, total_metrics);

    let processing_time = timeout_manager.elapsed_time().as_millis() as u64;

    // Update raw ingestion record with processing results
    if let Err(e) = update_processing_status(&pool, raw_id, &processing_result).await {
        error!(
            user_id = %auth.user.id,
            raw_id = %raw_id,
            error = %e,
            "Failed to update processing status"
        );
    }

    // Create response
    let success = processing_result.errors.is_empty();
    let processing_status = if success {
        "processed"
    } else {
        "partial_success"
    };

    let response = IngestResponse {
        success,
        processed_count: processing_result.processed_count,
        failed_count: processing_result.failed_count,
        processing_time_ms: processing_time,
        errors: processing_result.errors,
        processing_status: Some(processing_status.to_string()),
        raw_ingestion_id: None, // Simple async handler doesn't track raw ingestion
    };

    // Determine status using timeout manager
    let timeout_reached = timeout_manager.is_approaching_timeout(1.0);
    let near_timeout = timeout_manager.is_approaching_timeout(0.8);

    let status = if response.success && !timeout_reached {
        "success"
    } else if timeout_reached {
        "timeout"
    } else {
        "partial_failure"
    };

    // Record final metrics with timeout manager
    let duration = timeout_manager.elapsed_time();
    Metrics::record_ingest_duration(duration, status);
    Metrics::record_data_volume("ingest", "processed", response.processed_count as u64);

    if response.failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest-async", "warning");
    }

    // Log final statistics using timeout manager
    timeout_manager.log_final_stats(
        auth.user.id,
        response.processed_count,
        response.failed_count,
    );

    info!(
        user_id = %auth.user.id,
        processed_count = response.processed_count,
        failed_count = response.failed_count,
        processing_time_ms = processing_time,
        timeout_seconds = timeout_manager.get_processing_timeout().as_secs(),
        payload_size_mb = payload_size_mb,
        "Optimized async health data ingestion completed with improved architecture"
    );

    // Return appropriate status code based on processing results and timeout
    if response.success && !timeout_reached {
        Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
    } else if near_timeout || timeout_reached {
        // 202 Accepted indicates partial processing due to time constraints
        Ok(HttpResponse::Accepted().json(ApiResponse::success(response)))
    } else {
        Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
    }
}

// Note: store_raw_payload function moved to payload_processor module

/// Update processing status after batch processing
async fn update_processing_status(
    pool: &PgPool,
    raw_id: Uuid,
    result: &crate::services::batch_processor::BatchProcessingResult,
) -> Result<(), sqlx::Error> {
    let status = if result.errors.is_empty() {
        "processed"
    } else {
        "error"
    };
    let processing_errors = if result.errors.is_empty() {
        None
    } else {
        Some(
            serde_json::to_value(
                result
                    .errors
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "metric_type": e.metric_type,
                            "error_message": e.error_message
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
    };

    sqlx::query!(
        r#"
        UPDATE raw_ingestions 
        SET processed_at = NOW(), processing_status = $2, processing_errors = $3
        WHERE id = $1
        "#,
        raw_id,
        status,
        processing_errors
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Note: parse_ios_payload_enhanced function moved to payload_processor module
