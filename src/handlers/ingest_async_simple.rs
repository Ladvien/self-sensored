use actix_web::{web, HttpRequest, HttpResponse, Result};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::BatchConfig;
use crate::middleware::metrics::Metrics;
use crate::models::{ApiResponse, IngestPayload, IngestResponse, IosIngestPayload};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;

/// Maximum payload size (200MB)
const MAX_PAYLOAD_SIZE: usize = 200 * 1024 * 1024;
/// Large batch threshold - use optimized processing for batches above this
const LARGE_BATCH_THRESHOLD: usize = 5_000;
/// Timeout for processing to prevent Cloudflare 524 errors (80 seconds)
const PROCESSING_TIMEOUT_SECONDS: u64 = 80;

/// Optimized async health data ingest endpoint
/// Uses chunked processing and optimized batch configuration for large payloads
#[instrument(skip(pool, raw_payload))]
pub async fn ingest_async_optimized_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    raw_payload: web::Bytes,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // Record ingest request start
    Metrics::record_ingest_request();

    // Extract client IP for audit logging
    let client_ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let payload_size = raw_payload.len();

    info!(
        user_id = %auth.user.id,
        client_ip = %client_ip,
        payload_size = payload_size,
        "Starting optimized async ingest processing"
    );

    // Record data volume
    Metrics::record_data_volume("ingest", "received", payload_size as u64);

    // Check payload size limit
    if payload_size > MAX_PAYLOAD_SIZE {
        error!("Payload size {} exceeds limit", payload_size);
        Metrics::record_error("payload_too_large", "/api/v1/ingest-async", "error");
        return Ok(
            HttpResponse::PayloadTooLarge().json(ApiResponse::<()>::error(format!(
                "Payload size {} bytes exceeds maximum of {} MB",
                payload_size,
                MAX_PAYLOAD_SIZE / (1024 * 1024)
            ))),
        );
    }

    // Parse JSON payload with timeout protection
    let internal_payload = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        parse_ios_payload_enhanced(&raw_payload),
    )
    .await
    {
        Ok(Ok(payload)) => payload,
        Ok(Err(parse_error)) => {
            error!("JSON parse error: {}", parse_error);
            Metrics::record_error("json_parse", "/api/v1/ingest-async", "error");
            return Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                    "JSON parsing error: {}",
                    parse_error
                ))),
            );
        }
        Err(_) => {
            error!("JSON parsing timeout after 10 seconds");
            Metrics::record_error("json_parse_timeout", "/api/v1/ingest-async", "error");
            return Ok(
                HttpResponse::RequestTimeout().json(ApiResponse::<()>::error(
                    "JSON parsing took too long".to_string(),
                )),
            );
        }
    };

    let total_metrics = internal_payload.data.metrics.len() + internal_payload.data.workouts.len();

    info!(
        user_id = %auth.user.id,
        total_metrics = total_metrics,
        threshold = LARGE_BATCH_THRESHOLD,
        "Determining processing approach"
    );

    // Store raw payload
    let raw_id = match store_raw_payload(&pool, &auth, &internal_payload, &client_ip).await {
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

    // Configure batch processor for optimal performance
    let batch_config = if total_metrics >= LARGE_BATCH_THRESHOLD {
        BatchConfig {
            max_retries: 2, // Reduce retries for speed
            initial_backoff_ms: 50,
            max_backoff_ms: 1000,
            enable_parallel_processing: true,
            chunk_size: 1000,
            memory_limit_mb: 1000.0,
            // Optimized chunk sizes for large batches
            heart_rate_chunk_size: 8000,
            blood_pressure_chunk_size: 8000,
            sleep_chunk_size: 5000,
            activity_chunk_size: 7000,
            workout_chunk_size: 5000,
            enable_progress_tracking: false, // Disable for speed
            enable_intra_batch_deduplication: true,
            enable_dual_write_activity_metrics: false, // Disable for async endpoint to prioritize speed
        }
    } else {
        BatchConfig::default()
    };

    // Process with timeout to prevent Cloudflare 524 errors
    let processor = BatchProcessor::with_config(pool.get_ref().clone(), batch_config);

    let processing_result = match tokio::time::timeout(
        std::time::Duration::from_secs(PROCESSING_TIMEOUT_SECONDS),
        processor.process_batch(auth.user.id, internal_payload),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            // Processing timed out - this would require background processing in production
            error!(
                user_id = %auth.user.id,
                total_metrics = total_metrics,
                timeout_seconds = PROCESSING_TIMEOUT_SECONDS,
                "Processing timed out - payload too large for synchronous processing"
            );

            // Update raw ingestion status
            let _ = sqlx::query!(
                "UPDATE raw_ingestions SET status = 'error', error_message = 'Processing timeout - requires background processing' WHERE id = $1",
                raw_id
            )
            .execute(pool.get_ref())
            .await;

            Metrics::record_error("processing_timeout", "/api/v1/ingest-async", "error");

            return Ok(HttpResponse::RequestTimeout().json(ApiResponse::<()>::error(
                format!("Processing timed out after {} seconds. Payload with {} metrics is too large for real-time processing. Consider using background processing for large batches.", 
                    PROCESSING_TIMEOUT_SECONDS, total_metrics)
            )));
        }
    };

    let processing_time = start_time.elapsed().as_millis() as u64;

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
    let response = IngestResponse {
        success: processing_result.errors.is_empty(),
        processed_count: processing_result.processed_count,
        failed_count: processing_result.failed_count,
        processing_time_ms: processing_time,
        errors: processing_result.errors,
    };

    // Record final metrics
    let duration = start_time.elapsed();
    let status = if response.success {
        "success"
    } else if processing_time < (PROCESSING_TIMEOUT_SECONDS * 1000) {
        "partial_failure"
    } else {
        "timeout"
    };

    Metrics::record_ingest_duration(duration, status);
    Metrics::record_data_volume("ingest", "processed", response.processed_count as u64);

    if response.failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest-async", "warning");
    }

    info!(
        user_id = %auth.user.id,
        processed_count = response.processed_count,
        failed_count = response.failed_count,
        processing_time_ms = processing_time,
        timeout_seconds = PROCESSING_TIMEOUT_SECONDS,
        "Optimized async health data ingestion completed"
    );

    // Return 200 for successful processing, 202 if we hit performance limits but still processed
    if response.success {
        Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
    } else if processing_time >= (PROCESSING_TIMEOUT_SECONDS * 800) {
        // 80% of timeout
        Ok(HttpResponse::Accepted().json(ApiResponse::success(response))) // Indicate partial processing due to time constraints
    } else {
        Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
    }
}

/// Store raw payload for backup and audit purposes
async fn store_raw_payload(
    pool: &PgPool,
    auth: &AuthContext,
    payload: &IngestPayload,
    _client_ip: &str,
) -> Result<Uuid, sqlx::Error> {
    use sha2::{Digest, Sha256};

    let payload_json = serde_json::to_string(payload).map_err(sqlx::Error::decode)?;
    let payload_hash = format!("{:x}", Sha256::digest(payload_json.as_bytes()));

    let result = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (user_id, api_key_id, raw_data, data_hash) 
        VALUES ($1, $2, $3, $4) 
        ON CONFLICT (user_id, data_hash, ingested_at) DO NOTHING
        RETURNING id
        "#,
        auth.user.id,
        auth.api_key.id,
        serde_json::to_value(payload).map_err(|e| sqlx::Error::decode(e))?,
        payload_hash
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(record) => Ok(record.id),
        None => {
            let existing = sqlx::query!(
                "SELECT id FROM raw_ingestions WHERE data_hash = $1 LIMIT 1",
                payload_hash
            )
            .fetch_one(pool)
            .await?;
            Ok(existing.id)
        }
    }
}

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
    let error_message = if result.errors.is_empty() {
        None
    } else {
        Some(
            result
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.metric_type, e.error_message))
                .collect::<Vec<_>>()
                .join("; "),
        )
    };

    sqlx::query!(
        r#"
        UPDATE raw_ingestions 
        SET processed_at = NOW(), status = $2, error_message = $3
        WHERE id = $1
        "#,
        raw_id,
        status,
        error_message
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Parse iOS payload with enhanced error handling
async fn parse_ios_payload_enhanced(raw_payload: &web::Bytes) -> Result<IngestPayload> {
    let payload_size = raw_payload.len();
    if payload_size > 10 * 1024 * 1024 {
        info!(
            "Processing large payload: {} MB",
            payload_size / (1024 * 1024)
        );
    }

    // Try iOS format first
    match serde_json::from_slice::<IosIngestPayload>(raw_payload) {
        Ok(ios_payload) => {
            info!(
                "Successfully parsed iOS format payload ({} bytes)",
                payload_size
            );
            Ok(ios_payload.to_internal_format())
        }
        Err(ios_error) => {
            warn!("iOS format parse failed: {}", ios_error);

            // Try standard format as fallback
            match serde_json::from_slice::<IngestPayload>(raw_payload) {
                Ok(standard_payload) => {
                    info!(
                        "Successfully parsed standard format payload ({} bytes)",
                        payload_size
                    );
                    Ok(standard_payload)
                }
                Err(standard_error) => {
                    error!("Failed to parse payload in both iOS and standard formats");
                    error!("iOS format error: {}", ios_error);
                    error!("Standard format error: {}", standard_error);

                    Err(actix_web::error::ErrorBadRequest(format!(
                        "Invalid JSON format. iOS error: {}. Standard error: {}",
                        ios_error, standard_error
                    )))
                }
            }
        }
    }
}
