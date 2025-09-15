use actix_web::{web, HttpRequest, HttpResponse, Result};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, instrument, warn};

use crate::middleware::metrics::Metrics;
use crate::models::{
    ApiResponse, CreateJobResponse, IngestBatchConfig, IngestPayload, IosIngestPayload, JobStatus,
};
use crate::services::auth::AuthContext;
use crate::services::background_processor::BackgroundProcessor;
use crate::services::streaming_parser::parse_large_json_payload;

// All limits completely removed for personal health app - no restrictions

/// Async health data ingest endpoint that returns immediately with job ID for large payloads
/// Uses background processing to prevent Cloudflare 524 timeout errors
#[instrument(skip(pool, raw_payload))]
pub async fn ingest_async_handler(
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

    // Get payload size and log processing start
    let payload_size = raw_payload.len();
    
    info!(
        user_id = %auth.user.id,
        client_ip = %client_ip,
        payload_size = payload_size,
        "Starting async ingest processing"
    );

    // Record data volume
    Metrics::record_data_volume("ingest", "received", payload_size as u64);

    // No payload size restrictions for personal health app

    // Use enhanced JSON parsing with better error reporting
    let internal_payload = match parse_ios_payload_enhanced(&raw_payload, auth.user.id).await {
        Ok(payload) => payload,
        Err(parse_error) => {
            error!("Enhanced JSON parse error: {}", parse_error);
            Metrics::record_error("enhanced_json_parse", "/api/v1/ingest-async", "error");
            return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                format!("JSON parsing error: {}", parse_error),
            )));
        }
    };

    // Check total metrics count to determine processing approach
    let total_metrics = internal_payload.data.metrics.len() + internal_payload.data.workouts.len();
    
    info!(
        user_id = %auth.user.id,
        total_metrics = total_metrics,
        threshold = "none",
        "Determining processing approach"
    );

    // Store raw payload for backup and audit purposes
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

    // Determine if we should process asynchronously
    // Always process asynchronously to avoid any timeouts
        // Process asynchronously - create background job and return immediately
        info!(
            user_id = %auth.user.id,
            total_metrics = total_metrics,
            "Creating background job for large payload"
        );

        let job_config = IngestBatchConfig {
            enable_parallel_processing: true,
            chunk_size_override: None,
            timeout_seconds: Some(600), // 10 minutes for large batches
            enable_progress_notifications: true,
        };

        match BackgroundProcessor::create_job(&pool, &auth, raw_id, &internal_payload, Some(job_config)).await {
            Ok(job_id) => {
                let processing_time = start_time.elapsed().as_millis() as u64;
                
                let response = CreateJobResponse {
                    job_id,
                    status: JobStatus::Pending,
                    estimated_completion_time: None, // Will be calculated by background processor
                    message: format!(
                        "Large payload with {} metrics queued for background processing. Use job ID to check status.",
                        total_metrics
                    ),
                };

                // Record successful async job creation
                Metrics::record_ingest_duration(start_time.elapsed(), "async_queued");

                info!(
                    user_id = %auth.user.id,
                    job_id = %job_id,
                    total_metrics = total_metrics,
                    processing_time_ms = processing_time,
                    "Background job created successfully"
                );

                Ok(HttpResponse::Accepted().json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!(
                    user_id = %auth.user.id,
                    error = %e,
                    "Failed to create background job"
                );
                Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to create background processing job".to_string(),
                )))
            }
        }
    } else {
        // Process synchronously for smaller payloads (fallback to existing handler logic)
        info!(
            user_id = %auth.user.id,
            total_metrics = total_metrics,
            "Processing synchronously for small payload"
        );
        
        // For now, redirect to the existing synchronous handler
        // In practice, you might want to inline the synchronous processing here
        use crate::handlers::ingest::ingest_handler;
        
        // Create a new request with the raw payload
        ingest_handler(pool, auth, raw_payload, req).await
    }
}

/// Store raw payload for backup and audit purposes (same as main ingest handler)
async fn store_raw_payload(
    pool: &PgPool,
    auth: &AuthContext,
    payload: &IngestPayload,
    _client_ip: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    use sha2::{Digest, Sha256};
    
    // Calculate SHA256 hash of payload for deduplication
    let payload_json = serde_json::to_string(payload).map_err(sqlx::Error::decode)?;
    let payload_hash = format!("{:x}", Sha256::digest(payload_json.as_bytes()));

    let result = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (user_id, api_key_id, raw_data, data_hash) 
        VALUES ($1, $2, $3, $4) 
        ON CONFLICT (user_id, data_hash) DO NOTHING
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
            // Duplicate payload, get existing ID
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

/// Parse iOS payload with enhanced error handling and validation (same as main ingest handler)
async fn parse_ios_payload_enhanced(raw_payload: &web::Bytes, user_id: uuid::Uuid) -> Result<IngestPayload> {
    // Log payload info for debugging large payloads
    let payload_size = raw_payload.len();
    if payload_size > 10 * 1024 * 1024 {  // > 10MB
        info!("Processing large payload: {} MB", payload_size / (1024 * 1024));
    }

    // Try iOS format first with enhanced error reporting
    match serde_json::from_slice::<IosIngestPayload>(raw_payload) {
        Ok(ios_payload) => {
            info!("Successfully parsed iOS format payload ({} bytes)", payload_size);
            Ok(ios_payload.to_internal_format(user_id))
        }
        Err(ios_error) => {
            warn!("iOS format parse failed: {}", ios_error);
            
            // Try standard format as fallback
            match serde_json::from_slice::<IngestPayload>(raw_payload) {
                Ok(standard_payload) => {
                    info!("Successfully parsed standard format payload ({} bytes)", payload_size);
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