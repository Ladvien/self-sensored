use actix_web::{dev::Payload, web, FromRequest, HttpRequest, HttpResponse, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::middleware::metrics::Metrics;
use crate::models::{
    ApiResponse, HealthMetric, IngestData, IngestPayload, IngestResponse, IosIngestPayload,
};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::{BatchProcessingResult, BatchProcessor};

/// Maximum payload size (200MB) - increased to handle large iOS exports
const MAX_PAYLOAD_SIZE: usize = 200 * 1024 * 1024;
/// Maximum number of metrics per request (increased to handle full day exports)
const MAX_METRICS_PER_REQUEST: usize = 100_000;

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
                // Check if adding this chunk would exceed the size limit
                if body.len() + chunk.len() > MAX_PAYLOAD_SIZE {
                    error!(
                        "Payload size exceeds limit: {} bytes",
                        body.len() + chunk.len()
                    );
                    return Err(actix_web::error::ErrorPayloadTooLarge(format!(
                        "Payload exceeds maximum size of {} MB",
                        MAX_PAYLOAD_SIZE / (1024 * 1024)
                    )));
                }
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

/// Main health data ingest endpoint with enhanced JSON parsing
/// Accepts batch health data with Bearer token authentication
#[instrument(skip(pool, raw_payload))]
pub async fn ingest_handler(
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
        "Starting enhanced ingest processing"
    );

    // Record data volume
    Metrics::record_data_volume("ingest", "received", payload_size as u64);

    // Check payload size limit first
    if payload_size > MAX_PAYLOAD_SIZE {
        error!("Payload size {} exceeds limit", payload_size);
        Metrics::record_error("payload_too_large", "/api/v1/ingest", "error");
        return Ok(
            HttpResponse::PayloadTooLarge().json(ApiResponse::<()>::error(format!(
                "Payload size {} bytes exceeds maximum of {} MB",
                payload_size,
                MAX_PAYLOAD_SIZE / (1024 * 1024)
            ))),
        );
    }

    // Use enhanced JSON parsing with better error reporting
    let internal_payload = match parse_ios_payload_enhanced(&raw_payload).await {
        Ok(payload) => payload,
        Err(parse_error) => {
            error!("Enhanced JSON parse error: {}", parse_error);
            Metrics::record_error("enhanced_json_parse", "/api/v1/ingest", "error");
            return Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                    "JSON parsing error: {}",
                    parse_error
                ))),
            );
        }
    };

    // Validate payload constraints
    let total_metrics = internal_payload.data.metrics.len() + internal_payload.data.workouts.len();
    if total_metrics > MAX_METRICS_PER_REQUEST {
        error!(
            "Too many metrics: {} exceeds limit of {}",
            total_metrics, MAX_METRICS_PER_REQUEST
        );
        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                "Too many metrics: {} exceeds limit of {}",
                total_metrics, MAX_METRICS_PER_REQUEST
            ))),
        );
    }

    // Validate individual metrics and workouts
    let mut all_validation_errors = validate_metrics(&internal_payload.data.metrics);
    all_validation_errors.extend(validate_workouts(&internal_payload.data.workouts));

    let mut processed_payload = internal_payload;

    // If we have validation errors, filter out invalid metrics but continue processing valid ones
    if !all_validation_errors.is_empty() {
        warn!(
            "Validation errors found: {} items failed validation, continuing with valid metrics",
            all_validation_errors.len()
        );

        // Filter out invalid metrics for processing, but report the errors
        let valid_metrics: Vec<HealthMetric> = processed_payload
            .data
            .metrics
            .into_iter()
            .enumerate()
            .filter_map(|(index, metric)| match metric.validate() {
                Ok(()) => Some(metric),
                Err(_) => {
                    debug!("Excluding invalid metric at index {}", index);
                    None
                }
            })
            .collect();

        let valid_workouts: Vec<crate::models::WorkoutData> = processed_payload
            .data
            .workouts
            .into_iter()
            .enumerate()
            .filter_map(|(index, workout)| match validate_single_workout(&workout) {
                Ok(()) => Some(workout),
                Err(_) => {
                    debug!("Excluding invalid workout at index {}", index);
                    None
                }
            })
            .collect();

        // Update payload with only valid data
        processed_payload = IngestPayload {
            data: IngestData {
                metrics: valid_metrics,
                workouts: valid_workouts,
            },
        };

        // If no valid data remains after filtering, return error
        if processed_payload.data.metrics.is_empty() && processed_payload.data.workouts.is_empty() {
            error!("No valid metrics remaining after validation");
            return Ok(HttpResponse::BadRequest().json(
                ApiResponse::<IngestResponse>::error_with_data(
                    "All metrics failed validation".to_string(),
                    IngestResponse {
                        success: false,
                        processed_count: 0,
                        failed_count: all_validation_errors.len(),
                        processing_time_ms: start_time.elapsed().as_millis() as u64,
                        errors: all_validation_errors,
                    },
                ),
            ));
        }
    }

    info!(
        user_id = %auth.user.id,
        api_key_id = %auth.api_key.id,
        client_ip = %client_ip,
        metric_count = processed_payload.data.metrics.len(),
        workout_count = processed_payload.data.workouts.len(),
        "Starting health data ingestion"
    );

    // Store raw payload for backup and audit purposes (use processed payload to avoid storing invalid data)
    let raw_id = match store_raw_payload(&pool, &auth, &processed_payload, &client_ip).await {
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
    let mut result = processor
        .process_batch(auth.user.id, processed_payload)
        .await;

    // Add validation errors to the processing result
    if !all_validation_errors.is_empty() {
        result.errors.extend(all_validation_errors);
        result.failed_count += result.errors.len() - result.failed_count; // Add validation failures to count
    }

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

    // Record final metrics
    let duration = start_time.elapsed();
    let status = if response.success {
        "success"
    } else {
        "partial_failure"
    };
    Metrics::record_ingest_duration(duration, status);
    Metrics::record_data_volume("ingest", "processed", response.processed_count as u64);

    if response.failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest", "warning");
    }

    info!(
        user_id = %auth.user.id,
        processed_count = response.processed_count,
        failed_count = response.failed_count,
        processing_time_ms = processing_time,
        "Health data ingestion completed"
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Validate all metrics and return detailed errors
fn validate_metrics(metrics: &[HealthMetric]) -> Vec<crate::models::ProcessingError> {
    let mut errors = Vec::new();

    for (index, metric) in metrics.iter().enumerate() {
        if let Err(validation_error) = metric.validate() {
            errors.push(crate::models::ProcessingError {
                metric_type: metric.metric_type().to_string(),
                error_message: validation_error,
                index: Some(index),
            });
        }
    }

    errors
}

/// Validate all workouts and return detailed errors
fn validate_workouts(
    workouts: &[crate::models::WorkoutData],
) -> Vec<crate::models::ProcessingError> {
    let mut errors = Vec::new();

    for (index, workout) in workouts.iter().enumerate() {
        if let Err(validation_error) = validate_single_workout(workout) {
            errors.push(crate::models::ProcessingError {
                metric_type: "Workout".to_string(),
                error_message: validation_error,
                index: Some(index),
            });
        }
    }

    errors
}

/// Validate a single workout
fn validate_single_workout(workout: &crate::models::WorkoutData) -> Result<(), String> {
    if workout.start_time >= workout.end_time {
        return Err("Workout end_time must be after start_time".to_string());
    }

    let duration = workout.end_time - workout.start_time;
    if duration.num_hours() > 24 {
        return Err("Workout duration cannot exceed 24 hours".to_string());
    }

    if let Some(calories) = workout.total_energy_kcal {
        if calories < 0.0 || calories > 10000.0 {
            return Err(format!(
                "total_energy_kcal {} is out of reasonable range (0-10000)",
                calories
            ));
        }
    }

    if let Some(distance) = workout.distance_meters {
        if distance < 0.0 || distance > 1000000.0 {
            // Max 1000km
            return Err(format!(
                "distance_meters {} is out of reasonable range (0-1000000)",
                distance
            ));
        }
    }

    if let Some(hr) = workout.avg_heart_rate {
        if !(20..=300).contains(&hr) {
            return Err(format!("avg_heart_rate {} is out of range (20-300)", hr));
        }
    }

    if let Some(hr) = workout.max_heart_rate {
        if !(20..=300).contains(&hr) {
            return Err(format!("max_heart_rate {} is out of range (20-300)", hr));
        }
    }

    if workout.workout_type.is_empty() {
        return Err("workout_type cannot be empty".to_string());
    }

    Ok(())
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

/// Parse iOS payload with enhanced error handling and validation
async fn parse_ios_payload_enhanced(raw_payload: &web::Bytes) -> Result<IngestPayload> {
    // Log payload info for debugging large payloads
    let payload_size = raw_payload.len();
    if payload_size > 10 * 1024 * 1024 {
        // > 10MB
        info!(
            "Processing large payload: {} MB",
            payload_size / (1024 * 1024)
        );
    }

    // First validate JSON structure to detect corruption early
    if let Err(validation_error) = validate_json_structure_basic(raw_payload) {
        error!("JSON structure validation failed: {}", validation_error);
        return Err(actix_web::error::ErrorBadRequest(format!(
            "Malformed JSON detected: {}",
            validation_error
        )));
    }

    // Try iOS format first with enhanced error reporting
    match parse_with_error_context::<IosIngestPayload>(raw_payload, "iOS format") {
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
            match parse_with_error_context::<IngestPayload>(raw_payload, "standard format") {
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

                    // Log excerpt for debugging (first 1000 chars)
                    let payload_str = String::from_utf8_lossy(raw_payload);
                    let preview = if payload_str.len() > 1000 {
                        &payload_str[..1000]
                    } else {
                        &payload_str
                    };
                    error!("Payload preview: {}", preview);

                    Err(actix_web::error::ErrorBadRequest(format!(
                        "Invalid JSON format. iOS error: {}. Standard error: {}",
                        ios_error, standard_error
                    )))
                }
            }
        }
    }
}

/// Parse JSON with enhanced error context reporting
fn parse_with_error_context<T: serde::de::DeserializeOwned>(
    data: &[u8],
    format_name: &str,
) -> std::result::Result<T, String> {
    let deserializer = &mut serde_json::Deserializer::from_slice(data);
    match serde_path_to_error::deserialize(deserializer) {
        Ok(parsed) => Ok(parsed),
        Err(err) => {
            let path = err.path().to_string();
            let inner = err.into_inner();
            Err(format!(
                "{} parsing failed at '{}': {}",
                format_name, path, inner
            ))
        }
    }
}

/// Basic JSON structure validation to detect corruption
fn validate_json_structure_basic(data: &[u8]) -> std::result::Result<(), String> {
    if data.is_empty() {
        return Err("Empty payload".to_string());
    }

    // Quick validation - check for balanced braces
    let mut brace_count = 0i32;
    let mut bracket_count = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    let mut quote_count = 0;

    for &byte in data {
        if escape_next {
            escape_next = false;
            continue;
        }

        match byte {
            b'"' if !escape_next => {
                in_string = !in_string;
                quote_count += 1;
            }
            b'\\' if in_string => escape_next = true,
            b'{' if !in_string => brace_count += 1,
            b'}' if !in_string => brace_count -= 1,
            b'[' if !in_string => bracket_count += 1,
            b']' if !in_string => bracket_count -= 1,
            _ => {}
        }

        // Early detection of malformed structure
        if brace_count < 0 || bracket_count < 0 {
            return Err("Unmatched closing brackets detected".to_string());
        }
    }

    if in_string {
        return Err(format!(
            "Unterminated string detected (quotes: {})",
            quote_count
        ));
    }

    if brace_count != 0 {
        return Err(format!("Unmatched braces: {} unclosed", brace_count));
    }

    if bracket_count != 0 {
        return Err(format!("Unmatched brackets: {} unclosed", bracket_count));
    }

    Ok(())
}
