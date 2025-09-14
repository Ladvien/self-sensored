use actix_web::{web, HttpResponse, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::middleware::metrics::Metrics;
use crate::models::{
    AudioExposureMetric, EnvironmentalMetric, SafetyEventMetric, ApiResponse, ProcessingError
};
use crate::services::auth::AuthContext;

/// Environmental data ingest payload
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnvironmentalIngestPayload {
    pub data: Vec<EnvironmentalMetric>,
}

/// Audio exposure data ingest payload
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AudioExposureIngestPayload {
    pub data: Vec<AudioExposureMetric>,
}

/// Safety events data ingest payload
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SafetyEventIngestPayload {
    pub data: Vec<SafetyEventMetric>,
}

/// Environmental data query parameters
#[derive(Debug, Deserialize)]
pub struct EnvironmentalQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Environmental data response
#[derive(Debug, Serialize)]
pub struct EnvironmentalDataResponse {
    pub data: Vec<EnvironmentalMetric>,
    pub total_count: Option<i64>,
    pub has_more: bool,
}

/// Environmental data ingest endpoint
#[instrument(skip(pool, payload))]
pub async fn ingest_environmental_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<EnvironmentalIngestPayload>,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // Record environmental ingest request
    Metrics::record_ingest_request();

    info!(
        user_id = %auth.user.id,
        metric_count = payload.data.len(),
        "Processing environmental data ingest request"
    );

    if payload.data.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "No environmental data provided".to_string(),
        )));
    }

    // Validate metrics and prepare for storage
    let mut processed_count = 0;
    let mut errors = Vec::new();

    for (index, metric) in payload.data.iter().enumerate() {
        // Validate the metric
        if let Err(e) = metric.validate() {
            errors.push(ProcessingError {
                metric_type: "Environmental".to_string(),
                error_message: e,
                index: Some(index),
            });
            continue;
        }

        // Store environmental metric
        match store_environmental_metric(&pool, &auth.user.id, metric).await {
            Ok(_) => processed_count += 1,
            Err(e) => {
                error!("Failed to store environmental metric: {}", e);
                errors.push(ProcessingError {
                    metric_type: "Environmental".to_string(),
                    error_message: format!("Storage error: {}", e),
                    index: Some(index),
                });
            }
        }
    }

    let processing_time = start_time.elapsed().as_millis() as u64;
    let failed_count = payload.data.len() - processed_count;

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time,
        "Completed environmental data ingest"
    );

    // Record processing metrics
    Metrics::record_metrics_processed("environmental", processed_count as u64, "success");
    if failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest/environmental", "warning");
    }

    Ok(HttpResponse::Ok().json(crate::models::IngestResponse {
        success: errors.is_empty(),
        processed_count,
        failed_count,
        processing_time_ms: processing_time,
        errors,
    }))
}

/// Audio exposure data ingest endpoint
#[instrument(skip(pool, payload))]
pub async fn ingest_audio_exposure_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<AudioExposureIngestPayload>,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // Record audio exposure ingest request
    Metrics::record_ingest_request();

    info!(
        user_id = %auth.user.id,
        metric_count = payload.data.len(),
        "Processing audio exposure data ingest request"
    );

    if payload.data.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "No audio exposure data provided".to_string(),
        )));
    }

    // Validate metrics and prepare for storage
    let mut processed_count = 0;
    let mut errors = Vec::new();

    for (index, metric) in payload.data.iter().enumerate() {
        // Validate the metric
        if let Err(e) = metric.validate() {
            errors.push(ProcessingError {
                metric_type: "AudioExposure".to_string(),
                error_message: e,
                index: Some(index),
            });
            continue;
        }

        // Check for dangerous audio levels and log warnings
        if metric.audio_exposure_event {
            warn!(
                user_id = %auth.user.id,
                environmental_db = ?metric.environmental_audio_exposure_db,
                headphone_db = ?metric.headphone_audio_exposure_db,
                duration_minutes = metric.exposure_duration_minutes,
                "Dangerous audio exposure level detected"
            );
        }

        // Store audio exposure metric
        match store_audio_exposure_metric(&pool, &auth.user.id, metric).await {
            Ok(_) => processed_count += 1,
            Err(e) => {
                error!("Failed to store audio exposure metric: {}", e);
                errors.push(ProcessingError {
                    metric_type: "AudioExposure".to_string(),
                    error_message: format!("Storage error: {}", e),
                    index: Some(index),
                });
            }
        }
    }

    let processing_time = start_time.elapsed().as_millis() as u64;
    let failed_count = payload.data.len() - processed_count;

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time,
        "Completed audio exposure data ingest"
    );

    // Record processing metrics
    Metrics::record_metrics_processed("audio_exposure", processed_count as u64, "success");
    if failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest/audio-exposure", "warning");
    }

    Ok(HttpResponse::Ok().json(crate::models::IngestResponse {
        success: errors.is_empty(),
        processed_count,
        failed_count,
        processing_time_ms: processing_time,
        errors,
    }))
}

/// Safety events data ingest endpoint
#[instrument(skip(pool, payload))]
pub async fn ingest_safety_events_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<SafetyEventIngestPayload>,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // Record safety events ingest request
    Metrics::record_ingest_request();

    info!(
        user_id = %auth.user.id,
        event_count = payload.data.len(),
        "Processing safety events data ingest request"
    );

    if payload.data.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
            "No safety events data provided".to_string(),
        )));
    }

    // Validate metrics and prepare for storage
    let mut processed_count = 0;
    let mut errors = Vec::new();

    for (index, metric) in payload.data.iter().enumerate() {
        // Validate the metric
        if let Err(e) = metric.validate() {
            errors.push(ProcessingError {
                metric_type: "SafetyEvent".to_string(),
                error_message: e,
                index: Some(index),
            });
            continue;
        }

        // Log critical safety events
        match metric.event_type.as_str() {
            "fall_detected" | "emergency_sos" | "crash_detected" => {
                warn!(
                    user_id = %auth.user.id,
                    event_type = %metric.event_type,
                    severity_level = ?metric.severity_level,
                    emergency_contacts_notified = metric.emergency_contacts_notified,
                    "Critical safety event detected"
                );
            }
            _ => {
                info!(
                    user_id = %auth.user.id,
                    event_type = %metric.event_type,
                    "Safety event recorded"
                );
            }
        }

        // Store safety event metric
        match store_safety_event_metric(&pool, &auth.user.id, metric).await {
            Ok(_) => processed_count += 1,
            Err(e) => {
                error!("Failed to store safety event metric: {}", e);
                errors.push(ProcessingError {
                    metric_type: "SafetyEvent".to_string(),
                    error_message: format!("Storage error: {}", e),
                    index: Some(index),
                });
            }
        }
    }

    let processing_time = start_time.elapsed().as_millis() as u64;
    let failed_count = payload.data.len() - processed_count;

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time,
        "Completed safety events data ingest"
    );

    // Record processing metrics
    Metrics::record_metrics_processed("safety_events", processed_count as u64, "success");
    if failed_count > 0 {
        Metrics::record_error("validation", "/api/v1/ingest/safety-events", "warning");
    }

    Ok(HttpResponse::Ok().json(crate::models::IngestResponse {
        success: errors.is_empty(),
        processed_count,
        failed_count,
        processing_time_ms: processing_time,
        errors,
    }))
}

/// Get environmental data endpoint
#[instrument(skip(pool))]
pub async fn get_environmental_data_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<EnvironmentalQueryParams>,
) -> Result<HttpResponse> {
    info!(
        user_id = %auth.user.id,
        "Retrieving environmental data"
    );

    let start_date = query.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(30) // Default to last 30 days
    });
    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let limit = query.limit.unwrap_or(1000).min(10000); // Max 10k records
    let offset = query.offset.unwrap_or(0);

    // Query environmental data
    match get_environmental_data(&pool, &auth.user.id, start_date, end_date, limit, offset).await {
        Ok((data, total_count)) => {
            let has_more = data.len() as i32 == limit;

            Ok(HttpResponse::Ok().json(EnvironmentalDataResponse {
                data,
                total_count: Some(total_count),
                has_more,
            }))
        }
        Err(e) => {
            error!("Failed to retrieve environmental data: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to retrieve environmental data".to_string(),
            )))
        }
    }
}

// Database operations

/// Store environmental metric in database
async fn store_environmental_metric(
    pool: &PgPool,
    user_id: &Uuid,
    metric: &EnvironmentalMetric,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO environmental_metrics (
            id, user_id, recorded_at, uv_index, uv_exposure_minutes,
            time_in_daylight_minutes, ambient_temperature_celsius,
            humidity_percent, air_pressure_hpa, altitude_meters,
            location_latitude, location_longitude, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (user_id, recorded_at) DO UPDATE SET
            uv_index = COALESCE(EXCLUDED.uv_index, environmental_metrics.uv_index),
            uv_exposure_minutes = COALESCE(EXCLUDED.uv_exposure_minutes, environmental_metrics.uv_exposure_minutes),
            time_in_daylight_minutes = COALESCE(EXCLUDED.time_in_daylight_minutes, environmental_metrics.time_in_daylight_minutes),
            ambient_temperature_celsius = COALESCE(EXCLUDED.ambient_temperature_celsius, environmental_metrics.ambient_temperature_celsius),
            humidity_percent = COALESCE(EXCLUDED.humidity_percent, environmental_metrics.humidity_percent),
            air_pressure_hpa = COALESCE(EXCLUDED.air_pressure_hpa, environmental_metrics.air_pressure_hpa),
            altitude_meters = COALESCE(EXCLUDED.altitude_meters, environmental_metrics.altitude_meters),
            location_latitude = COALESCE(EXCLUDED.location_latitude, environmental_metrics.location_latitude),
            location_longitude = COALESCE(EXCLUDED.location_longitude, environmental_metrics.location_longitude),
            source_device = COALESCE(EXCLUDED.source_device, environmental_metrics.source_device)
        "#,
        metric.id,
        user_id,
        metric.recorded_at,
        metric.uv_index,
        metric.uv_exposure_minutes,
        metric.time_in_daylight_minutes,
        metric.ambient_temperature_celsius,
        metric.humidity_percent,
        metric.air_pressure_hpa,
        metric.altitude_meters,
        metric.location_latitude,
        metric.location_longitude,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Store audio exposure metric in database
async fn store_audio_exposure_metric(
    pool: &PgPool,
    user_id: &Uuid,
    metric: &AudioExposureMetric,
) -> Result<(), sqlx::Error> {
    // Note: We need to create a separate table for audio exposure or extend environmental_metrics
    // For now, storing in environmental_metrics table with audio-specific fields
    sqlx::query!(
        r#"
        INSERT INTO environmental_metrics (
            id, user_id, recorded_at, environmental_audio_exposure_db,
            headphone_audio_exposure_db, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id, recorded_at) DO UPDATE SET
            environmental_audio_exposure_db = COALESCE(EXCLUDED.environmental_audio_exposure_db, environmental_metrics.environmental_audio_exposure_db),
            headphone_audio_exposure_db = COALESCE(EXCLUDED.headphone_audio_exposure_db, environmental_metrics.headphone_audio_exposure_db),
            source_device = COALESCE(EXCLUDED.source_device, environmental_metrics.source_device)
        "#,
        metric.id,
        user_id,
        metric.recorded_at,
        metric.environmental_audio_exposure_db,
        metric.headphone_audio_exposure_db,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Store safety event metric in database
async fn store_safety_event_metric(
    pool: &PgPool,
    user_id: &Uuid,
    metric: &SafetyEventMetric,
) -> Result<(), sqlx::Error> {
    // We would need a separate safety_events table for this
    // For the demonstration, we'll store as a health symptom with event type
    sqlx::query!(
        r#"
        INSERT INTO symptoms (
            id, user_id, recorded_at, symptom_type, severity,
            duration_minutes, notes, episode_id, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        metric.id,
        user_id,
        metric.recorded_at,
        metric.event_type,
        metric.severity_level.map(|s| s as i32),
        None::<i32>, // duration_minutes
        metric.notes,
        None::<uuid::Uuid>, // episode_id
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get environmental data from database
async fn get_environmental_data(
    pool: &PgPool,
    user_id: &Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: i32,
    offset: i32,
) -> Result<(Vec<EnvironmentalMetric>, i64), sqlx::Error> {
    let data = sqlx::query_as!(
        EnvironmentalMetric,
        r#"
        SELECT
            id, user_id, recorded_at, uv_index, uv_exposure_minutes,
            time_in_daylight_minutes, ambient_temperature_celsius,
            humidity_percent, air_pressure_hpa, altitude_meters,
            location_latitude, location_longitude,
            environmental_audio_exposure_db, headphone_audio_exposure_db,
            source_device, created_at
        FROM environmental_metrics
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        ORDER BY recorded_at DESC
        LIMIT $4 OFFSET $5
        "#,
        user_id,
        start_date,
        end_date,
        limit as i64,
        offset as i64
    )
    .fetch_all(pool)
    .await?;

    let total_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM environmental_metrics WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3",
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap_or(0);

    Ok((data, total_count))
}