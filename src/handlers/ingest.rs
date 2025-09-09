use actix_web::{dev::Payload, web, FromRequest, HttpRequest, HttpResponse, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::models::{ApiResponse, IngestPayload, IngestResponse, IosIngestPayload};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::{BatchProcessingResult, BatchProcessor};

/// Custom extractor that logs raw JSON payload before deserialization
pub struct LoggedJson<T>(pub T);

impl<T> FromRequest for LoggedJson<T>
where
    T: serde::de::DeserializeOwned,
{
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            let mut body = web::BytesMut::new();
            while let Some(chunk) = payload.next().await {
                let chunk = chunk?;
                body.extend_from_slice(&chunk);
            }

            let body_str = String::from_utf8_lossy(&body);
            debug!("Raw JSON payload received: {}", body_str);
            debug!("Payload length: {} bytes", body.len());
            debug!("Content-Type: {:?}", req.headers().get("content-type"));

            match serde_json::from_slice::<T>(&body) {
                Ok(obj) => Ok(LoggedJson(obj)),
                Err(e) => {
                    error!("JSON deserialization error: {}", e);
                    error!("Failed JSON payload: {}", body_str);
                    Err(actix_web::error::ErrorBadRequest(format!(
                        "JSON parsing error: {e}"
                    )))
                }
            }
        })
    }
}

/// Main health data ingest endpoint
/// Accepts batch health data with Bearer token authentication
#[instrument(skip(pool, raw_payload))]
pub async fn ingest_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    raw_payload: web::Bytes,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // Extract client IP for audit logging
    let client_ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Parse the raw payload - try iOS format first, then standard format
    let payload_str = String::from_utf8_lossy(&raw_payload);
    debug!("Raw JSON payload received: {}", payload_str);
    debug!("Payload length: {} bytes", raw_payload.len());
    debug!("Content-Type: {:?}", req.headers().get("content-type"));

    let internal_payload =
        match serde_json::from_slice::<IosIngestPayload>(&raw_payload) {
            Ok(ios_payload) => {
                info!("Successfully parsed iOS format payload");
                ios_payload.to_internal_format()
            }
            Err(_ios_error) => {
                // Try standard format
                match serde_json::from_slice::<IngestPayload>(&raw_payload) {
                    Ok(standard_payload) => {
                        info!("Successfully parsed standard format payload");
                        standard_payload
                    }
                    Err(standard_error) => {
                        error!("Failed to parse payload in both iOS and standard formats");
                        error!("Standard format error: {}", standard_error);
                        error!("Raw payload: {}", payload_str);
                        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                            format!("Invalid JSON format: {standard_error}"),
                        )));
                    }
                }
            }
        };

    info!(
        user_id = %auth.user.id,
        api_key_id = %auth.api_key.id,
        client_ip = %client_ip,
        metric_count = internal_payload.data.metrics.len(),
        workout_count = internal_payload.data.workouts.len(),
        "Starting health data ingestion"
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

    // Process the batch data
    let processor = BatchProcessor::new(pool.get_ref().clone());
    let result = processor
        .process_batch(auth.user.id, internal_payload)
        .await;

    let processing_time = start_time.elapsed().as_millis() as u64;

    // Update raw ingestion record with processing results
    if let Err(e) = update_processing_status(&pool, raw_id, &result).await {
        error!(
            user_id = %auth.user.id,
            raw_id = %raw_id,
            error = %e,
            "Failed to update processing status"
        );
    }

    // Create response
    let response = IngestResponse {
        success: result.errors.is_empty(),
        processed_count: result.processed_count,
        failed_count: result.failed_count,
        processing_time_ms: processing_time,
        errors: result.errors,
    };

    info!(
        user_id = %auth.user.id,
        processed_count = response.processed_count,
        failed_count = response.failed_count,
        processing_time_ms = processing_time,
        "Health data ingestion completed"
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Store raw payload for backup and audit purposes
async fn store_raw_payload(
    pool: &PgPool,
    auth: &AuthContext,
    payload: &IngestPayload,
    _client_ip: &str,
) -> Result<Uuid, sqlx::Error> {
    // Calculate SHA256 hash of payload for deduplication
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

/// Update processing status after batch processing
async fn update_processing_status(
    pool: &PgPool,
    raw_id: Uuid,
    result: &BatchProcessingResult,
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
