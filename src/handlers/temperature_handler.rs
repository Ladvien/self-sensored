use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::ValidationConfig;
use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::TemperatureMetric;
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;

/// Temperature data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct TemperatureIngestPayload {
    pub temperature_metrics: Vec<TemperatureIngestRequest>,
}

/// Individual temperature metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct TemperatureIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub body_temperature: Option<f64>, // celsius (35-42�C range)
    pub basal_body_temperature: Option<f64>, // celsius (fertility tracking)
    pub apple_sleeping_wrist_temperature: Option<f64>, // celsius (Apple Watch)
    pub water_temperature: Option<f64>, // celsius (environmental)
    pub temperature_source: Option<String>, // thermometer type
    pub source_device: Option<String>,
}

/// Temperature data query parameters
#[derive(Debug, Deserialize)]
pub struct TemperatureQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub temperature_type: Option<String>, // body, basal, wrist, water
    pub source: Option<String>,
}

/// Temperature ingestion response
#[derive(Debug, Serialize)]
pub struct TemperatureIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<TemperatureProcessingError>,
    pub temperature_analysis: Option<TemperatureAnalysis>,
}

/// Temperature processing error details
#[derive(Debug, Serialize)]
pub struct TemperatureProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub temperature_value: Option<f64>,
    pub temperature_type: Option<String>,
}

/// Temperature analysis summary
#[derive(Debug, Serialize)]
pub struct TemperatureAnalysis {
    pub fever_detected: bool,
    pub fever_episodes_count: usize,
    pub ovulation_indicators: usize,
    pub temperature_range_body: Option<TemperatureRange>,
    pub temperature_range_basal: Option<TemperatureRange>,
    pub critical_temperatures: Vec<CriticalTemperatureAlert>,
}

/// Temperature range summary
#[derive(Debug, Serialize)]
pub struct TemperatureRange {
    pub min: f64,
    pub max: f64,
    pub average: f64,
    pub readings_count: usize,
}

/// Critical temperature alert
#[derive(Debug, Serialize)]
pub struct CriticalTemperatureAlert {
    pub temperature: f64,
    pub temperature_type: String,
    pub severity: String, // hypothermia, hyperthermia, high_fever
    pub recorded_at: DateTime<Utc>,
}

/// Temperature data response
#[derive(Debug, Serialize)]
pub struct TemperatureDataResponse {
    pub temperature_data: Vec<TemperatureMetric>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub summary: TemperatureSummary,
}

/// Date range for temperature data
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Temperature data summary
#[derive(Debug, Serialize)]
pub struct TemperatureSummary {
    pub total_readings: i64,
    pub fever_episodes: i64,
    pub basal_temp_readings: i64,
    pub average_body_temperature: Option<f64>,
    pub temperature_sources: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Temperature data ingestion handler with comprehensive validation and medical analysis
#[instrument(skip(pool, auth, payload), fields(user_id = %auth.user.id))]
pub async fn ingest_temperature_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<TemperatureIngestPayload>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth.user.id;

    info!(
        user_id = %user_id,
        temperature_count = payload.temperature_metrics.len(),
        "Processing temperature data ingestion"
    );

    // Validate payload size
    if payload.temperature_metrics.is_empty() {
        warn!(user_id = %user_id, "Empty temperature payload received");
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "empty_payload",
            "message": "No temperature metrics provided"
        })));
    }

    if payload.temperature_metrics.len() > 10000 {
        warn!(user_id = %user_id, count = payload.temperature_metrics.len(),
              "Temperature payload exceeds maximum size limit");
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "payload_too_large",
            "message": "Temperature payload exceeds 10,000 metrics limit",
            "max_allowed": 10000,
            "received": payload.temperature_metrics.len()
        })));
    }

    // Convert to internal temperature metrics with validation
    let validation_config = ValidationConfig::default();
    let mut temperature_metrics = Vec::new();
    let mut processing_errors = Vec::new();

    for (index, temp_request) in payload.temperature_metrics.iter().enumerate() {
        match convert_to_temperature_metric(user_id, temp_request, &validation_config) {
            Ok(metric) => temperature_metrics.push(metric),
            Err(error) => {
                processing_errors.push(TemperatureProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: error,
                    temperature_value: temp_request
                        .body_temperature
                        .or(temp_request.basal_body_temperature)
                        .or(temp_request.apple_sleeping_wrist_temperature)
                        .or(temp_request.water_temperature),
                    temperature_type: determine_temperature_type(temp_request),
                });
            }
        }
    }

    // Batch process temperature metrics
    let batch_processor = BatchProcessor::new(pool.get_ref().clone());
    let batch_result = batch_processor
        .process_temperature_metrics(user_id, temperature_metrics)
        .await;

    let processing_time = start_time.elapsed();

    // Update metrics
    Metrics::record_ingest_request();
    Metrics::record_ingest_duration(processing_time, "success");

    match batch_result {
        Ok(result) => {
            // Analyze temperature data for medical insights
            let temperature_analysis = analyze_temperature_data(&payload.temperature_metrics);

            // Log fever detection for medical monitoring
            if temperature_analysis.fever_detected {
                warn!(
                    user_id = %user_id,
                    fever_episodes = temperature_analysis.fever_episodes_count,
                    "Fever detected in temperature data"
                );
            }

            // Log critical temperature alerts
            for critical_temp in &temperature_analysis.critical_temperatures {
                warn!(
                    user_id = %user_id,
                    temperature = critical_temp.temperature,
                    temperature_type = critical_temp.temperature_type,
                    severity = critical_temp.severity,
                    "Critical temperature detected"
                );
            }

            info!(
                user_id = %user_id,
                processed = result.processed_count,
                failed = result.failed_count,
                processing_time_ms = processing_time.as_millis(),
                fever_detected = temperature_analysis.fever_detected,
                "Temperature data ingestion completed successfully"
            );

            Metrics::record_metrics_processed(
                "temperature",
                result.processed_count as u64,
                "success",
            );

            Ok(HttpResponse::Ok().json(TemperatureIngestResponse {
                success: true,
                processed_count: result.processed_count,
                failed_count: result.failed_count + processing_errors.len(),
                processing_time_ms: processing_time.as_millis() as u64,
                errors: processing_errors,
                temperature_analysis: Some(temperature_analysis),
            }))
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                processing_time_ms = processing_time.as_millis(),
                "Temperature data ingestion failed"
            );

            Metrics::record_error("batch_processing", "/api/v1/ingest/temperature", "error");

            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "batch_processing_failed",
                "message": "Failed to process temperature metrics",
                "details": e.to_string(),
                "processed_count": 0,
                "failed_count": payload.temperature_metrics.len()
            })))
        }
    }
}

/// Temperature data retrieval handler with filtering and medical analysis
#[instrument(skip(pool, auth, query), fields(user_id = %auth.user.id))]
pub async fn get_temperature_data_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<TemperatureQueryParams>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth.user.id;

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        limit = ?query.limit,
        temperature_type = ?query.temperature_type,
        "Retrieving temperature data"
    );

    // Set default limits and validate parameters
    let limit = query.limit.unwrap_or(1000).min(10000); // Max 10,000 records
    let start_date = query.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(30) // Default 30 days
    });
    let end_date = query.end_date.unwrap_or_else(Utc::now);

    if end_date <= start_date {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "invalid_date_range",
            "message": "end_date must be after start_date"
        })));
    }

    // Build query based on temperature type filter
    let temperature_data = match query.temperature_type.as_deref() {
        Some("body") => {
            sqlx::query_as!(
                TemperatureMetric,
                r#"
                SELECT id, user_id,
                       recorded_at::timestamptz as "recorded_at!",
                       body_temperature, basal_body_temperature,
                       apple_sleeping_wrist_temperature, water_temperature,
                       temperature_source, source_device,
                       created_at::timestamptz as "created_at!"
                FROM temperature_metrics
                WHERE user_id = $1
                  AND recorded_at >= $2
                  AND recorded_at <= $3
                  AND body_temperature IS NOT NULL
                ORDER BY recorded_at DESC
                LIMIT $4
                "#,
                user_id,
                start_date,
                end_date,
                limit as i64
            )
            .fetch_all(pool.get_ref())
            .await
        }
        Some("basal") => {
            sqlx::query_as!(
                TemperatureMetric,
                r#"
                SELECT id, user_id,
                       recorded_at::timestamptz as "recorded_at!",
                       body_temperature, basal_body_temperature,
                       apple_sleeping_wrist_temperature, water_temperature,
                       temperature_source, source_device,
                       created_at::timestamptz as "created_at!"
                FROM temperature_metrics
                WHERE user_id = $1
                  AND recorded_at >= $2
                  AND recorded_at <= $3
                  AND basal_body_temperature IS NOT NULL
                ORDER BY recorded_at DESC
                LIMIT $4
                "#,
                user_id,
                start_date,
                end_date,
                limit as i64
            )
            .fetch_all(pool.get_ref())
            .await
        }
        Some("wrist") => {
            sqlx::query_as!(
                TemperatureMetric,
                r#"
                SELECT id, user_id,
                       recorded_at::timestamptz as "recorded_at!",
                       body_temperature, basal_body_temperature,
                       apple_sleeping_wrist_temperature, water_temperature,
                       temperature_source, source_device,
                       created_at::timestamptz as "created_at!"
                FROM temperature_metrics
                WHERE user_id = $1
                  AND recorded_at >= $2
                  AND recorded_at <= $3
                  AND apple_sleeping_wrist_temperature IS NOT NULL
                ORDER BY recorded_at DESC
                LIMIT $4
                "#,
                user_id,
                start_date,
                end_date,
                limit as i64
            )
            .fetch_all(pool.get_ref())
            .await
        }
        Some("water") => {
            sqlx::query_as!(
                TemperatureMetric,
                r#"
                SELECT id, user_id,
                       recorded_at::timestamptz as "recorded_at!",
                       body_temperature, basal_body_temperature,
                       apple_sleeping_wrist_temperature, water_temperature,
                       temperature_source, source_device,
                       created_at::timestamptz as "created_at!"
                FROM temperature_metrics
                WHERE user_id = $1
                  AND recorded_at >= $2
                  AND recorded_at <= $3
                  AND water_temperature IS NOT NULL
                ORDER BY recorded_at DESC
                LIMIT $4
                "#,
                user_id,
                start_date,
                end_date,
                limit as i64
            )
            .fetch_all(pool.get_ref())
            .await
        }
        _ => {
            // Default: all temperature types with optional source filter
            if let Some(source) = &query.source {
                sqlx::query_as!(
                    TemperatureMetric,
                    r#"
                    SELECT id, user_id,
                           recorded_at::timestamptz as "recorded_at!",
                           body_temperature, basal_body_temperature,
                           apple_sleeping_wrist_temperature, water_temperature,
                           temperature_source, source_device,
                           created_at::timestamptz as "created_at!"
                    FROM temperature_metrics
                    WHERE user_id = $1
                      AND recorded_at >= $2
                      AND recorded_at <= $3
                      AND temperature_source = $4
                    ORDER BY recorded_at DESC
                    LIMIT $5
                    "#,
                    user_id,
                    start_date,
                    end_date,
                    source,
                    limit as i64
                )
                .fetch_all(pool.get_ref())
                .await
            } else {
                sqlx::query_as!(
                    TemperatureMetric,
                    r#"
                    SELECT id, user_id,
                           recorded_at::timestamptz as "recorded_at!",
                           body_temperature, basal_body_temperature,
                           apple_sleeping_wrist_temperature, water_temperature,
                           temperature_source, source_device,
                           created_at::timestamptz as "created_at!"
                    FROM temperature_metrics
                    WHERE user_id = $1
                      AND recorded_at >= $2
                      AND recorded_at <= $3
                    ORDER BY recorded_at DESC
                    LIMIT $4
                    "#,
                    user_id,
                    start_date,
                    end_date,
                    limit as i64
                )
                .fetch_all(pool.get_ref())
                .await
            }
        }
    };

    let temperature_data = match temperature_data {
        Ok(data) => data,
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve temperature data"
            );
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": "Failed to retrieve temperature data"
            })));
        }
    };

    // Get summary statistics
    let summary = match get_temperature_summary(pool.get_ref(), user_id, start_date, end_date).await
    {
        Ok(summary) => summary,
        Err(e) => {
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to get temperature summary, using defaults"
            );
            TemperatureSummary {
                total_readings: temperature_data.len() as i64,
                fever_episodes: 0,
                basal_temp_readings: 0,
                average_body_temperature: None,
                temperature_sources: Vec::new(),
                last_updated: temperature_data.first().map(|t| t.created_at),
            }
        }
    };

    let processing_time = start_time.elapsed();

    // Update metrics - use existing HTTP request metrics
    Metrics::record_ingest_duration(processing_time, "success");

    info!(
        user_id = %user_id,
        records_returned = temperature_data.len(),
        processing_time_ms = processing_time.as_millis(),
        "Temperature data retrieval completed"
    );

    Ok(HttpResponse::Ok().json(TemperatureDataResponse {
        temperature_data,
        total_count: summary.total_readings,
        date_range: Some(DateRange {
            start: start_date,
            end: end_date,
        }),
        summary,
    }))
}

/// Convert temperature ingest request to internal temperature metric
fn convert_to_temperature_metric(
    user_id: Uuid,
    request: &TemperatureIngestRequest,
    config: &ValidationConfig,
) -> Result<TemperatureMetric, String> {
    let metric = TemperatureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        body_temperature: request.body_temperature,
        basal_body_temperature: request.basal_body_temperature,
        apple_sleeping_wrist_temperature: request.apple_sleeping_wrist_temperature,
        water_temperature: request.water_temperature,
        temperature_source: request.temperature_source.clone(),
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the temperature metric
    metric.validate_with_config(config)?;

    // Additional validation for at least one temperature value
    if metric.body_temperature.is_none()
        && metric.basal_body_temperature.is_none()
        && metric.apple_sleeping_wrist_temperature.is_none()
        && metric.water_temperature.is_none()
    {
        return Err("At least one temperature value must be provided".to_string());
    }

    Ok(metric)
}

/// Determine the primary temperature type from the request
fn determine_temperature_type(request: &TemperatureIngestRequest) -> Option<String> {
    if request.body_temperature.is_some() {
        Some("body".to_string())
    } else if request.basal_body_temperature.is_some() {
        Some("basal".to_string())
    } else if request.apple_sleeping_wrist_temperature.is_some() {
        Some("wrist".to_string())
    } else if request.water_temperature.is_some() {
        Some("water".to_string())
    } else {
        None
    }
}

/// Analyze temperature data for medical insights
fn analyze_temperature_data(
    temperature_metrics: &[TemperatureIngestRequest],
) -> TemperatureAnalysis {
    let mut fever_episodes = 0;
    let mut ovulation_indicators = 0;
    let mut critical_temperatures = Vec::new();
    let mut body_temperatures = Vec::new();
    let mut basal_temperatures = Vec::new();

    for temp_request in temperature_metrics.iter() {
        // Check for fever (body temperature > 38.0�C)
        if let Some(body_temp) = temp_request.body_temperature {
            body_temperatures.push(body_temp);

            if body_temp > 38.0 {
                fever_episodes += 1;
            }

            // Check for critical temperatures
            if body_temp < 35.0 {
                critical_temperatures.push(CriticalTemperatureAlert {
                    temperature: body_temp,
                    temperature_type: "body".to_string(),
                    severity: "hypothermia".to_string(),
                    recorded_at: temp_request.recorded_at,
                });
            } else if body_temp > 40.0 {
                critical_temperatures.push(CriticalTemperatureAlert {
                    temperature: body_temp,
                    temperature_type: "body".to_string(),
                    severity: "hyperthermia".to_string(),
                    recorded_at: temp_request.recorded_at,
                });
            } else if body_temp > 39.0 {
                critical_temperatures.push(CriticalTemperatureAlert {
                    temperature: body_temp,
                    temperature_type: "body".to_string(),
                    severity: "high_fever".to_string(),
                    recorded_at: temp_request.recorded_at,
                });
            }
        }

        // Check basal temperature for ovulation indicators
        if let Some(basal_temp) = temp_request.basal_body_temperature {
            basal_temperatures.push(basal_temp);

            // Simple ovulation detection (temperature spike > 36.5�C for basal temp)
            if basal_temp > 36.5 {
                ovulation_indicators += 1;
            }
        }
    }

    let fever_detected = fever_episodes > 0;

    TemperatureAnalysis {
        fever_detected,
        fever_episodes_count: fever_episodes,
        ovulation_indicators,
        temperature_range_body: calculate_temperature_range(&body_temperatures),
        temperature_range_basal: calculate_temperature_range(&basal_temperatures),
        critical_temperatures,
    }
}

/// Calculate temperature range statistics
fn calculate_temperature_range(temperatures: &[f64]) -> Option<TemperatureRange> {
    if temperatures.is_empty() {
        return None;
    }

    let min = temperatures.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = temperatures
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let sum: f64 = temperatures.iter().sum();
    let average = sum / temperatures.len() as f64;

    Some(TemperatureRange {
        min,
        max,
        average,
        readings_count: temperatures.len(),
    })
}

/// Get temperature summary statistics from database
async fn get_temperature_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<TemperatureSummary, sqlx::Error> {
    let summary = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total_readings,
            COUNT(*) FILTER (WHERE body_temperature > 38.0) as fever_episodes,
            COUNT(*) FILTER (WHERE basal_body_temperature IS NOT NULL) as basal_temp_readings,
            AVG(body_temperature) as average_body_temperature,
            MAX(created_at) as last_updated
        FROM temperature_metrics
        WHERE user_id = $1 AND recorded_at >= $2 AND recorded_at <= $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    // Get temperature sources
    let sources = sqlx::query!(
        r#"
        SELECT DISTINCT temperature_source
        FROM temperature_metrics
        WHERE user_id = $1
          AND recorded_at >= $2
          AND recorded_at <= $3
          AND temperature_source IS NOT NULL
        ORDER BY temperature_source
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await?;

    let temperature_sources: Vec<String> = sources
        .into_iter()
        .filter_map(|row| row.temperature_source)
        .collect();

    Ok(TemperatureSummary {
        total_readings: summary.total_readings.unwrap_or(0),
        fever_episodes: summary.fever_episodes.unwrap_or(0),
        basal_temp_readings: summary.basal_temp_readings.unwrap_or(0),
        average_body_temperature: summary.average_body_temperature,
        temperature_sources,
        last_updated: summary.last_updated,
    })
}
