use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::ValidationConfig;
use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::RespiratoryMetric;
use crate::services::auth::AuthContext;

/// Respiratory data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct RespiratoryIngestPayload {
    pub respiratory_metrics: Vec<RespiratoryIngestRequest>,
}

/// Individual respiratory metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct RespiratoryIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub respiratory_rate: Option<i32>, // breaths per minute (12-20 normal)
    pub oxygen_saturation: Option<f64>, // SpO2 percentage (90-100% normal, <90% critical)
    pub forced_vital_capacity: Option<f64>, // FVC in liters (spirometry)
    pub forced_expiratory_volume_1: Option<f64>, // FEV1 in liters (age/gender specific)
    pub peak_expiratory_flow_rate: Option<f64>, // PEFR in L/min (asthma monitoring)
    pub inhaler_usage: Option<i32>,    // count of inhaler uses/puffs
    pub source_device: Option<String>,
}

/// Respiratory data query parameters
#[derive(Debug, Deserialize)]
pub struct RespiratoryQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub metric_type: Option<String>, // spo2, respiratory_rate, spirometry
    pub source: Option<String>,
}

/// Respiratory ingestion response
#[derive(Debug, Serialize)]
pub struct RespiratoryIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<RespiratoryProcessingError>,
    pub respiratory_analysis: Option<RespiratoryAnalysis>,
    pub critical_alerts: Vec<RespiratoryAlert>,
}

/// Respiratory processing error details
#[derive(Debug, Serialize)]
pub struct RespiratoryProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub spo2_value: Option<f64>,
    pub respiratory_rate: Option<i32>,
    pub metric_type: Option<String>,
}

/// Respiratory health analysis
#[derive(Debug, Serialize)]
pub struct RespiratoryAnalysis {
    pub critical_spo2_detected: bool,
    pub abnormal_respiratory_rate_detected: bool,
    pub excessive_inhaler_usage: bool,
    pub critical_episodes_count: i32,
    pub average_spo2: Option<f64>,
    pub average_respiratory_rate: Option<f64>,
    pub total_inhaler_uses: i32,
    pub lung_function_assessment: Option<LungFunctionAssessment>,
    pub recommendations: Vec<String>,
}

/// Lung function assessment from spirometry data
#[derive(Debug, Serialize)]
pub struct LungFunctionAssessment {
    pub fev1_fvc_ratio: Option<f64>,
    pub fev1_percentage_predicted: Option<f64>,
    pub lung_function_category: String, // normal, mild, moderate, severe
    pub spirometry_interpretation: String,
}

/// Critical respiratory health alerts
#[derive(Debug, Serialize)]
pub struct RespiratoryAlert {
    pub alert_type: String, // critical_spo2, excessive_inhaler, severe_obstruction
    pub severity: String,   // warning, critical, emergency
    pub message: String,
    pub value: Option<f64>,
    pub medical_recommendation: String,
    pub requires_immediate_attention: bool,
}

/// Respiratory data query response
#[derive(Debug, Serialize)]
pub struct RespiratoryQueryResponse {
    pub respiratory_metrics: Vec<RespiratoryMetric>,
    pub summary: RespiratoryDataSummary,
    pub timeline_analysis: Option<RespiratoryTimelineAnalysis>,
}

/// Date range for respiratory data
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Respiratory data summary
#[derive(Debug, Serialize)]
pub struct RespiratoryDataSummary {
    pub total_readings: i64,
    pub critical_spo2_episodes: i64,
    pub spirometry_readings: i64,
    pub average_spo2: Option<f64>,
    pub average_respiratory_rate: Option<f64>,
    pub total_inhaler_uses: i64,
    pub respiratory_sources: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Respiratory timeline analysis
#[derive(Debug, Serialize)]
pub struct RespiratoryTimelineAnalysis {
    pub spo2_trend: String, // improving, stable, declining
    pub respiratory_rate_trend: String,
    pub inhaler_usage_pattern: String, // increasing, stable, decreasing
    pub critical_periods: Vec<RespiratoryPeriod>,
    pub lung_function_changes: Option<LungFunctionTrend>,
}

/// Period of respiratory concern
#[derive(Debug, Serialize)]
pub struct RespiratoryPeriod {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub concern_type: String, // hypoxia, tachypnea, obstruction
    pub severity: String,
    pub average_spo2: Option<f64>,
    pub average_respiratory_rate: Option<f64>,
}

/// Lung function trend analysis
#[derive(Debug, Serialize)]
pub struct LungFunctionTrend {
    pub fev1_trend: String, // improving, stable, declining
    pub fvc_trend: String,
    pub ratio_trend: String,
    pub disease_progression: Option<String>, // stable, mild_progression, concerning
}

/// Respiratory data ingestion handler with comprehensive validation and medical analysis
#[instrument(skip(pool, auth, payload), fields(user_id = %auth.user.id))]
pub async fn ingest_respiratory_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<RespiratoryIngestPayload>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth.user.id;

    info!(
        user_id = %user_id,
        respiratory_count = payload.respiratory_metrics.len(),
        "Processing respiratory data ingestion"
    );

    // Validate payload size
    if payload.respiratory_metrics.is_empty() {
        warn!(user_id = %user_id, "Empty respiratory payload received");
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "empty_payload",
            "message": "No respiratory metrics provided"
        })));
    }

    if payload.respiratory_metrics.len() > 10000 {
        warn!(user_id = %user_id, count = payload.respiratory_metrics.len(),
              "Respiratory payload exceeds maximum size limit");
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "payload_too_large",
            "message": "Respiratory payload exceeds 10,000 metrics limit",
            "max_allowed": 10000,
            "received": payload.respiratory_metrics.len()
        })));
    }

    // Convert to internal respiratory metrics with validation
    let validation_config = ValidationConfig::default();
    let mut respiratory_metrics = Vec::new();
    let mut processing_errors = Vec::new();
    let mut critical_alerts = Vec::new();

    for (index, respiratory_request) in payload.respiratory_metrics.iter().enumerate() {
        match convert_to_respiratory_metric(user_id, respiratory_request, &validation_config) {
            Ok(metric) => {
                // Check for critical respiratory conditions
                if let Some(alerts) = check_critical_respiratory_conditions(&metric) {
                    critical_alerts.extend(alerts);
                }
                respiratory_metrics.push(metric);
            }
            Err(error) => {
                processing_errors.push(RespiratoryProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: error,
                    spo2_value: respiratory_request.oxygen_saturation,
                    respiratory_rate: respiratory_request.respiratory_rate,
                    metric_type: determine_respiratory_metric_type(respiratory_request),
                });
            }
        }
    }

    // Batch process respiratory metrics using direct database insertion
    let batch_result =
        insert_respiratory_metrics_batch(pool.get_ref(), user_id, respiratory_metrics).await;

    let processing_time = start_time.elapsed();

    // Update metrics
    Metrics::record_ingest_request();
    Metrics::record_ingest_duration(processing_time, "success");

    match batch_result {
        Ok(result) => {
            // Analyze respiratory data for medical insights
            let respiratory_analysis = analyze_respiratory_data(&payload.respiratory_metrics);

            // Log critical respiratory events for medical monitoring
            if respiratory_analysis.critical_spo2_detected {
                error!(
                    user_id = %user_id,
                    critical_episodes = respiratory_analysis.critical_episodes_count,
                    "CRITICAL: SpO2 below 90% detected - potential medical emergency"
                );
            }

            // Log excessive inhaler usage
            if respiratory_analysis.excessive_inhaler_usage {
                warn!(
                    user_id = %user_id,
                    total_uses = respiratory_analysis.total_inhaler_uses,
                    "ALERT: Excessive inhaler usage detected - may indicate poor asthma control"
                );
            }

            info!(
                user_id = %user_id,
                processed = result.processed_count,
                failed = processing_errors.len(),
                processing_time_ms = processing_time.as_millis(),
                critical_alerts = critical_alerts.len(),
                "Respiratory data processing completed"
            );

            Ok(HttpResponse::Ok().json(RespiratoryIngestResponse {
                success: true,
                processed_count: result.processed_count,
                failed_count: processing_errors.len(),
                processing_time_ms: processing_time.as_millis() as u64,
                errors: processing_errors,
                respiratory_analysis: Some(respiratory_analysis),
                critical_alerts,
            }))
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                processing_time_ms = processing_time.as_millis(),
                "Failed to process respiratory data"
            );

            Metrics::record_ingest_duration(processing_time, "error");

            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "processing_failed",
                "message": "Failed to process respiratory data",
                "details": e.to_string(),
                "processing_time_ms": processing_time.as_millis()
            })))
        }
    }
}

/// Respiratory data query handler with medical analysis
#[instrument(skip(pool, auth), fields(user_id = %auth.user.id))]
pub async fn query_respiratory_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<RespiratoryQueryParams>,
) -> ActixResult<HttpResponse> {
    let user_id = auth.user.id;

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        limit = ?query.limit,
        "Querying respiratory data"
    );

    // Set default date range if not provided (last 30 days)
    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let start_date = query
        .start_date
        .unwrap_or_else(|| end_date - chrono::Duration::days(30));
    let limit = query.limit.unwrap_or(1000).min(10000) as i64;

    // Build query based on parameters
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, user_id, recorded_at, respiratory_rate, oxygen_saturation,
         forced_vital_capacity, forced_expiratory_volume_1, peak_expiratory_flow_rate,
         inhaler_usage, source_device, created_at
         FROM respiratory_metrics WHERE user_id = ",
    );
    query_builder.push_bind(user_id);
    query_builder.push(" AND recorded_at BETWEEN ");
    query_builder.push_bind(start_date);
    query_builder.push(" AND ");
    query_builder.push_bind(end_date);

    // Add metric type filter if specified
    if let Some(metric_type) = &query.metric_type {
        match metric_type.as_str() {
            "spo2" => {
                query_builder.push(" AND oxygen_saturation IS NOT NULL");
            }
            "respiratory_rate" => {
                query_builder.push(" AND respiratory_rate IS NOT NULL");
            }
            "spirometry" => {
                query_builder.push(" AND (forced_vital_capacity IS NOT NULL OR forced_expiratory_volume_1 IS NOT NULL OR peak_expiratory_flow_rate IS NOT NULL)");
            }
            _ => {} // No additional filter for unknown types
        }
    }

    // Add source filter if specified
    if let Some(source) = &query.source {
        query_builder.push(" AND source_device = ");
        query_builder.push_bind(source);
    }

    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    match query_builder
        .build_query_as::<RespiratoryMetric>()
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(respiratory_metrics) => {
            // Generate summary statistics
            let summary =
                match generate_respiratory_summary(pool.get_ref(), user_id, start_date, end_date)
                    .await
                {
                    Ok(summary) => summary,
                    Err(e) => {
                        error!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to generate respiratory summary"
                        );
                        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "summary_generation_failed",
                            "message": "Failed to generate respiratory data summary"
                        })));
                    }
                };

            // Generate timeline analysis if sufficient data
            let timeline_analysis = if respiratory_metrics.len() > 5 {
                Some(generate_respiratory_timeline_analysis(&respiratory_metrics))
            } else {
                None
            };

            info!(
                user_id = %user_id,
                returned_count = respiratory_metrics.len(),
                "Respiratory data query completed"
            );

            Ok(HttpResponse::Ok().json(RespiratoryQueryResponse {
                respiratory_metrics,
                summary,
                timeline_analysis,
            }))
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to query respiratory data"
            );

            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "query_failed",
                "message": "Failed to retrieve respiratory data",
                "details": e.to_string()
            })))
        }
    }
}

/// Convert ingestion request to validated respiratory metric
fn convert_to_respiratory_metric(
    user_id: Uuid,
    request: &RespiratoryIngestRequest,
    config: &ValidationConfig,
) -> Result<RespiratoryMetric, String> {
    let metric = RespiratoryMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        respiratory_rate: request.respiratory_rate,
        oxygen_saturation: request.oxygen_saturation,
        forced_vital_capacity: request.forced_vital_capacity,
        forced_expiratory_volume_1: request.forced_expiratory_volume_1,
        peak_expiratory_flow_rate: request.peak_expiratory_flow_rate,
        inhaler_usage: request.inhaler_usage,
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric.validate_with_config(config)?;

    Ok(metric)
}

/// Check for critical respiratory conditions requiring immediate attention
fn check_critical_respiratory_conditions(
    metric: &RespiratoryMetric,
) -> Option<Vec<RespiratoryAlert>> {
    let mut alerts = Vec::new();

    // Critical SpO2 levels (< 90%)
    if let Some(spo2) = metric.oxygen_saturation {
        if spo2 < 90.0 {
            alerts.push(RespiratoryAlert {
                alert_type: "critical_spo2".to_string(),
                severity: "emergency".to_string(),
                message: format!("Critical SpO2 level detected: {:.1}%", spo2),
                value: Some(spo2),
                medical_recommendation:
                    "Seek immediate medical attention. SpO2 below 90% indicates severe hypoxemia."
                        .to_string(),
                requires_immediate_attention: true,
            });
        } else if spo2 < 95.0 {
            alerts.push(RespiratoryAlert {
                alert_type: "low_spo2".to_string(),
                severity: "warning".to_string(),
                message: format!("Low SpO2 level detected: {:.1}%", spo2),
                value: Some(spo2),
                medical_recommendation: "Consider consulting healthcare provider if persistent."
                    .to_string(),
                requires_immediate_attention: false,
            });
        }
    }

    // Abnormal respiratory rates
    if let Some(rate) = metric.respiratory_rate {
        if rate < 8 || rate > 30 {
            let severity = if rate < 6 || rate > 35 {
                "critical"
            } else {
                "warning"
            };
            alerts.push(RespiratoryAlert {
                alert_type: "abnormal_respiratory_rate".to_string(),
                severity: severity.to_string(),
                message: format!("Abnormal respiratory rate: {} breaths/min", rate),
                value: Some(rate as f64),
                medical_recommendation:
                    "Monitor closely and consult healthcare provider if persistent.".to_string(),
                requires_immediate_attention: severity == "critical",
            });
        }
    }

    // Excessive inhaler usage (> 8 puffs/day suggests poor asthma control)
    if let Some(inhaler_uses) = metric.inhaler_usage {
        if inhaler_uses > 8 {
            alerts.push(RespiratoryAlert {
                alert_type: "excessive_inhaler_usage".to_string(),
                severity: "warning".to_string(),
                message: format!("High inhaler usage detected: {} uses", inhaler_uses),
                value: Some(inhaler_uses as f64),
                medical_recommendation: "Excessive inhaler use may indicate poor asthma control. Consider consulting your healthcare provider.".to_string(),
                requires_immediate_attention: false,
            });
        }
    }

    if alerts.is_empty() {
        None
    } else {
        Some(alerts)
    }
}

/// Determine the primary metric type from the request
fn determine_respiratory_metric_type(request: &RespiratoryIngestRequest) -> Option<String> {
    if request.oxygen_saturation.is_some() {
        Some("spo2".to_string())
    } else if request.respiratory_rate.is_some() {
        Some("respiratory_rate".to_string())
    } else if request.forced_vital_capacity.is_some()
        || request.forced_expiratory_volume_1.is_some()
        || request.peak_expiratory_flow_rate.is_some()
    {
        Some("spirometry".to_string())
    } else if request.inhaler_usage.is_some() {
        Some("inhaler_usage".to_string())
    } else {
        None
    }
}

/// Analyze respiratory data for medical insights
fn analyze_respiratory_data(metrics: &[RespiratoryIngestRequest]) -> RespiratoryAnalysis {
    let mut critical_spo2_detected = false;
    let mut abnormal_respiratory_rate_detected = false;
    let mut excessive_inhaler_usage = false;
    let mut critical_episodes_count = 0;
    let mut total_inhaler_uses = 0;

    let mut spo2_values = Vec::new();
    let mut respiratory_rate_values = Vec::new();
    let mut recommendations = Vec::new();

    // Analyze each metric
    for metric in metrics {
        // SpO2 analysis
        if let Some(spo2) = metric.oxygen_saturation {
            spo2_values.push(spo2);
            if spo2 < 90.0 {
                critical_spo2_detected = true;
                critical_episodes_count += 1;
            }
        }

        // Respiratory rate analysis
        if let Some(rate) = metric.respiratory_rate {
            respiratory_rate_values.push(rate as f64);
            if rate < 8 || rate > 30 {
                abnormal_respiratory_rate_detected = true;
            }
        }

        // Inhaler usage analysis
        if let Some(inhaler_uses) = metric.inhaler_usage {
            total_inhaler_uses += inhaler_uses;
            if inhaler_uses > 8 {
                excessive_inhaler_usage = true;
            }
        }
    }

    // Calculate averages
    let average_spo2 = if !spo2_values.is_empty() {
        Some(spo2_values.iter().sum::<f64>() / spo2_values.len() as f64)
    } else {
        None
    };

    let average_respiratory_rate = if !respiratory_rate_values.is_empty() {
        Some(respiratory_rate_values.iter().sum::<f64>() / respiratory_rate_values.len() as f64)
    } else {
        None
    };

    // Generate medical recommendations
    if critical_spo2_detected {
        recommendations.push("Seek immediate medical attention for low oxygen levels".to_string());
    }
    if abnormal_respiratory_rate_detected {
        recommendations.push(
            "Monitor respiratory rate and consult healthcare provider if persistent".to_string(),
        );
    }
    if excessive_inhaler_usage {
        recommendations
            .push("Consider reviewing asthma action plan with healthcare provider".to_string());
    }
    if average_spo2.is_some_and(|avg| avg < 95.0) {
        recommendations.push(
            "Monitor oxygen levels closely and ensure proper pulse oximeter positioning"
                .to_string(),
        );
    }

    // Assess lung function if spirometry data available
    let lung_function_assessment = assess_lung_function(metrics);

    RespiratoryAnalysis {
        critical_spo2_detected,
        abnormal_respiratory_rate_detected,
        excessive_inhaler_usage,
        critical_episodes_count,
        average_spo2,
        average_respiratory_rate,
        total_inhaler_uses,
        lung_function_assessment,
        recommendations,
    }
}

/// Assess lung function from spirometry data
fn assess_lung_function(metrics: &[RespiratoryIngestRequest]) -> Option<LungFunctionAssessment> {
    let mut fev1_values = Vec::new();
    let mut fvc_values = Vec::new();

    for metric in metrics {
        if let Some(fev1) = metric.forced_expiratory_volume_1 {
            fev1_values.push(fev1);
        }
        if let Some(fvc) = metric.forced_vital_capacity {
            fvc_values.push(fvc);
        }
    }

    if !fev1_values.is_empty() && !fvc_values.is_empty() {
        let avg_fev1 = fev1_values.iter().sum::<f64>() / fev1_values.len() as f64;
        let avg_fvc = fvc_values.iter().sum::<f64>() / fvc_values.len() as f64;
        let fev1_fvc_ratio = avg_fev1 / avg_fvc;

        let (category, interpretation) = if fev1_fvc_ratio >= 0.7 {
            ("normal", "FEV1/FVC ratio within normal limits")
        } else if fev1_fvc_ratio >= 0.5 {
            ("mild_obstruction", "Mild airway obstruction detected")
        } else if fev1_fvc_ratio >= 0.3 {
            (
                "moderate_obstruction",
                "Moderate airway obstruction detected",
            )
        } else {
            ("severe_obstruction", "Severe airway obstruction detected")
        };

        Some(LungFunctionAssessment {
            fev1_fvc_ratio: Some(fev1_fvc_ratio),
            fev1_percentage_predicted: None, // Would need age/height/gender for calculation
            lung_function_category: category.to_string(),
            spirometry_interpretation: interpretation.to_string(),
        })
    } else {
        None
    }
}

/// Generate respiratory data summary from database
async fn generate_respiratory_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<RespiratoryDataSummary, sqlx::Error> {
    let summary = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total_readings,
            COUNT(CASE WHEN oxygen_saturation < 90.0 THEN 1 END) as critical_spo2_episodes,
            COUNT(CASE WHEN forced_vital_capacity IS NOT NULL OR forced_expiratory_volume_1 IS NOT NULL THEN 1 END) as spirometry_readings,
            AVG(oxygen_saturation) as avg_spo2,
            AVG(respiratory_rate::float) as avg_respiratory_rate,
            SUM(COALESCE(inhaler_usage, 0)) as total_inhaler_uses,
            MAX(created_at) as last_updated
        FROM respiratory_metrics
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    // Get unique respiratory sources
    let sources = sqlx::query_scalar!(
        "SELECT DISTINCT source_device FROM respiratory_metrics
         WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
         AND source_device IS NOT NULL",
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await?;

    Ok(RespiratoryDataSummary {
        total_readings: summary.total_readings.unwrap_or(0),
        critical_spo2_episodes: summary.critical_spo2_episodes.unwrap_or(0),
        spirometry_readings: summary.spirometry_readings.unwrap_or(0),
        average_spo2: summary.avg_spo2,
        average_respiratory_rate: summary.avg_respiratory_rate,
        total_inhaler_uses: summary.total_inhaler_uses.unwrap_or(0),
        respiratory_sources: sources,
        last_updated: summary.last_updated,
    })
}

/// Generate timeline analysis for respiratory data trends
fn generate_respiratory_timeline_analysis(
    metrics: &[RespiratoryMetric],
) -> RespiratoryTimelineAnalysis {
    // Sort metrics by date for trend analysis
    let mut sorted_metrics = metrics.to_vec();
    sorted_metrics.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));

    // Analyze SpO2 trend
    let spo2_trend = analyze_spo2_trend(&sorted_metrics);

    // Analyze respiratory rate trend
    let respiratory_rate_trend = analyze_respiratory_rate_trend(&sorted_metrics);

    // Analyze inhaler usage pattern
    let inhaler_usage_pattern = analyze_inhaler_usage_pattern(&sorted_metrics);

    // Identify critical periods
    let critical_periods = identify_critical_periods(&sorted_metrics);

    // Analyze lung function changes if spirometry data available
    let lung_function_changes = analyze_lung_function_changes(&sorted_metrics);

    RespiratoryTimelineAnalysis {
        spo2_trend,
        respiratory_rate_trend,
        inhaler_usage_pattern,
        critical_periods,
        lung_function_changes,
    }
}

/// Analyze SpO2 trend over time
fn analyze_spo2_trend(metrics: &[RespiratoryMetric]) -> String {
    let spo2_values: Vec<(DateTime<Utc>, f64)> = metrics
        .iter()
        .filter_map(|m| m.oxygen_saturation.map(|spo2| (m.recorded_at, spo2)))
        .collect();

    if spo2_values.len() < 2 {
        return "insufficient_data".to_string();
    }

    // Calculate trend using simple linear regression approach
    let first_half_avg = spo2_values[..spo2_values.len() / 2]
        .iter()
        .map(|(_, spo2)| *spo2)
        .sum::<f64>()
        / (spo2_values.len() / 2) as f64;
    let second_half_avg = spo2_values[spo2_values.len() / 2..]
        .iter()
        .map(|(_, spo2)| *spo2)
        .sum::<f64>()
        / (spo2_values.len() - spo2_values.len() / 2) as f64;

    if second_half_avg > first_half_avg + 1.0 {
        "improving".to_string()
    } else if second_half_avg < first_half_avg - 1.0 {
        "declining".to_string()
    } else {
        "stable".to_string()
    }
}

/// Analyze respiratory rate trend over time
fn analyze_respiratory_rate_trend(metrics: &[RespiratoryMetric]) -> String {
    let rate_values: Vec<(DateTime<Utc>, i32)> = metrics
        .iter()
        .filter_map(|m| m.respiratory_rate.map(|rate| (m.recorded_at, rate)))
        .collect();

    if rate_values.len() < 2 {
        return "insufficient_data".to_string();
    }

    // Calculate trend
    let first_half_avg = rate_values[..rate_values.len() / 2]
        .iter()
        .map(|(_, rate)| *rate as f64)
        .sum::<f64>()
        / (rate_values.len() / 2) as f64;
    let second_half_avg = rate_values[rate_values.len() / 2..]
        .iter()
        .map(|(_, rate)| *rate as f64)
        .sum::<f64>()
        / (rate_values.len() - rate_values.len() / 2) as f64;

    if second_half_avg > first_half_avg + 2.0 {
        "increasing".to_string()
    } else if second_half_avg < first_half_avg - 2.0 {
        "decreasing".to_string()
    } else {
        "stable".to_string()
    }
}

/// Analyze inhaler usage pattern over time
fn analyze_inhaler_usage_pattern(metrics: &[RespiratoryMetric]) -> String {
    let inhaler_values: Vec<(DateTime<Utc>, i32)> = metrics
        .iter()
        .filter_map(|m| m.inhaler_usage.map(|usage| (m.recorded_at, usage)))
        .collect();

    if inhaler_values.len() < 2 {
        return "insufficient_data".to_string();
    }

    // Calculate trend
    let first_half_sum = inhaler_values[..inhaler_values.len() / 2]
        .iter()
        .map(|(_, usage)| *usage)
        .sum::<i32>();
    let second_half_sum = inhaler_values[inhaler_values.len() / 2..]
        .iter()
        .map(|(_, usage)| *usage)
        .sum::<i32>();

    if second_half_sum > first_half_sum + 5 {
        "increasing".to_string()
    } else if second_half_sum < first_half_sum - 5 {
        "decreasing".to_string()
    } else {
        "stable".to_string()
    }
}

/// Identify periods of respiratory concern
fn identify_critical_periods(metrics: &[RespiratoryMetric]) -> Vec<RespiratoryPeriod> {
    let mut periods = Vec::new();
    let mut current_period: Option<(DateTime<Utc>, Vec<&RespiratoryMetric>)> = None;

    for metric in metrics {
        let is_critical = metric.oxygen_saturation.map_or(false, |spo2| spo2 < 90.0)
            || metric
                .respiratory_rate
                .map_or(false, |rate| rate < 8 || rate > 30);

        if is_critical {
            match &mut current_period {
                Some((_, metrics_in_period)) => {
                    metrics_in_period.push(metric);
                }
                None => {
                    current_period = Some((metric.recorded_at, vec![metric]));
                }
            }
        } else if let Some((start_time, metrics_in_period)) = current_period.take() {
            // End of critical period
            if let Some(last_metric) = metrics_in_period.last() {
                let avg_spo2 = metrics_in_period
                    .iter()
                    .filter_map(|m| m.oxygen_saturation)
                    .sum::<f64>()
                    / metrics_in_period.len() as f64;

                let avg_respiratory_rate = metrics_in_period
                    .iter()
                    .filter_map(|m| m.respiratory_rate.map(|r| r as f64))
                    .sum::<f64>()
                    / metrics_in_period.len() as f64;

                let concern_type = if avg_spo2 < 90.0 {
                    "hypoxia"
                } else if avg_respiratory_rate < 8.0 {
                    "bradypnea"
                } else if avg_respiratory_rate > 30.0 {
                    "tachypnea"
                } else {
                    "other"
                };

                periods.push(RespiratoryPeriod {
                    start_time,
                    end_time: last_metric.recorded_at,
                    concern_type: concern_type.to_string(),
                    severity: if avg_spo2 < 85.0
                        || avg_respiratory_rate < 6.0
                        || avg_respiratory_rate > 35.0
                    {
                        "critical"
                    } else {
                        "warning"
                    }
                    .to_string(),
                    average_spo2: if avg_spo2 > 0.0 { Some(avg_spo2) } else { None },
                    average_respiratory_rate: if avg_respiratory_rate > 0.0 {
                        Some(avg_respiratory_rate)
                    } else {
                        None
                    },
                });
            }
        }
    }

    periods
}

/// Analyze lung function changes over time
fn analyze_lung_function_changes(metrics: &[RespiratoryMetric]) -> Option<LungFunctionTrend> {
    let spirometry_data: Vec<(DateTime<Utc>, Option<f64>, Option<f64>)> = metrics
        .iter()
        .filter(|m| m.forced_vital_capacity.is_some() || m.forced_expiratory_volume_1.is_some())
        .map(|m| {
            (
                m.recorded_at,
                m.forced_expiratory_volume_1,
                m.forced_vital_capacity,
            )
        })
        .collect();

    if spirometry_data.len() < 2 {
        return None;
    }

    // Analyze FEV1 trend
    let fev1_values: Vec<f64> = spirometry_data
        .iter()
        .filter_map(|(_, fev1, _)| *fev1)
        .collect();

    let fev1_trend = if fev1_values.len() >= 2 {
        let first_avg = fev1_values[..fev1_values.len() / 2].iter().sum::<f64>()
            / (fev1_values.len() / 2) as f64;
        let second_avg = fev1_values[fev1_values.len() / 2..].iter().sum::<f64>()
            / (fev1_values.len() - fev1_values.len() / 2) as f64;

        if second_avg > first_avg + 0.1 {
            "improving"
        } else if second_avg < first_avg - 0.1 {
            "declining"
        } else {
            "stable"
        }
    } else {
        "insufficient_data"
    };

    // Similar analysis for FVC
    let fvc_values: Vec<f64> = spirometry_data
        .iter()
        .filter_map(|(_, _, fvc)| *fvc)
        .collect();

    let fvc_trend = if fvc_values.len() >= 2 {
        let first_avg =
            fvc_values[..fvc_values.len() / 2].iter().sum::<f64>() / (fvc_values.len() / 2) as f64;
        let second_avg = fvc_values[fvc_values.len() / 2..].iter().sum::<f64>()
            / (fvc_values.len() - fvc_values.len() / 2) as f64;

        if second_avg > first_avg + 0.1 {
            "improving"
        } else if second_avg < first_avg - 0.1 {
            "declining"
        } else {
            "stable"
        }
    } else {
        "insufficient_data"
    };

    // Calculate FEV1/FVC ratio trend
    let ratio_trend = if fev1_trend == "declining" || fvc_trend == "declining" {
        "concerning"
    } else if fev1_trend == "improving" && fvc_trend == "improving" {
        "improving"
    } else {
        "stable"
    };

    let disease_progression = match (fev1_trend, fvc_trend) {
        ("declining", "declining") => Some("concerning".to_string()),
        ("declining", _) | (_, "declining") => Some("mild_progression".to_string()),
        _ => Some("stable".to_string()),
    };

    Some(LungFunctionTrend {
        fev1_trend: fev1_trend.to_string(),
        fvc_trend: fvc_trend.to_string(),
        ratio_trend: ratio_trend.to_string(),
        disease_progression,
    })
}

/// Direct batch insertion of respiratory metrics to database
async fn insert_respiratory_metrics_batch(
    pool: &PgPool,
    user_id: Uuid,
    metrics: Vec<RespiratoryMetric>,
) -> Result<BatchInsertResult, Box<dyn std::error::Error + Send + Sync>> {
    if metrics.is_empty() {
        return Ok(BatchInsertResult { processed_count: 0 });
    }

    let mut successful_inserts = 0;

    for metric in metrics {
        let result = sqlx::query!(
            r#"
            INSERT INTO respiratory_metrics (
                id, user_id, recorded_at, respiratory_rate, oxygen_saturation,
                forced_vital_capacity, forced_expiratory_volume_1, peak_expiratory_flow_rate,
                inhaler_usage, source_device
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (user_id, recorded_at) DO UPDATE SET
                respiratory_rate = COALESCE(EXCLUDED.respiratory_rate, respiratory_metrics.respiratory_rate),
                oxygen_saturation = COALESCE(EXCLUDED.oxygen_saturation, respiratory_metrics.oxygen_saturation),
                forced_vital_capacity = COALESCE(EXCLUDED.forced_vital_capacity, respiratory_metrics.forced_vital_capacity),
                forced_expiratory_volume_1 = COALESCE(EXCLUDED.forced_expiratory_volume_1, respiratory_metrics.forced_expiratory_volume_1),
                peak_expiratory_flow_rate = COALESCE(EXCLUDED.peak_expiratory_flow_rate, respiratory_metrics.peak_expiratory_flow_rate),
                inhaler_usage = COALESCE(EXCLUDED.inhaler_usage, respiratory_metrics.inhaler_usage),
                source_device = COALESCE(EXCLUDED.source_device, respiratory_metrics.source_device)
            "#,
            metric.id,
            metric.user_id,
            metric.recorded_at,
            metric.respiratory_rate,
            metric.oxygen_saturation,
            metric.forced_vital_capacity,
            metric.forced_expiratory_volume_1,
            metric.peak_expiratory_flow_rate,
            metric.inhaler_usage,
            metric.source_device
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => successful_inserts += 1,
            Err(e) => {
                tracing::warn!(
                    user_id = %user_id,
                    metric_id = %metric.id,
                    error = %e,
                    "Failed to insert respiratory metric"
                );
            }
        }
    }

    Ok(BatchInsertResult {
        processed_count: successful_inserts,
    })
}

/// Simple result struct for batch insertion
#[derive(Debug)]
struct BatchInsertResult {
    processed_count: usize,
}
