use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::ValidationConfig;
use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::BodyMeasurementMetric;
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;

/// Body measurements data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct BodyMeasurementsIngestPayload {
    pub body_measurements: Vec<BodyMeasurementsIngestRequest>,
}

/// Individual body measurement metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct BodyMeasurementsIngestRequest {
    pub recorded_at: DateTime<Utc>,

    // Weight & Body Composition (Smart Scale Data)
    pub body_weight_kg: Option<f64>,
    pub body_mass_index: Option<f64>,
    pub body_fat_percentage: Option<f64>,
    pub lean_body_mass_kg: Option<f64>,

    // Physical Measurements
    pub height_cm: Option<f64>,
    pub waist_circumference_cm: Option<f64>,
    pub hip_circumference_cm: Option<f64>,
    pub chest_circumference_cm: Option<f64>,
    pub arm_circumference_cm: Option<f64>,
    pub thigh_circumference_cm: Option<f64>,

    // Body Temperature (from measurement context)
    pub body_temperature_celsius: Option<f64>,
    pub basal_body_temperature_celsius: Option<f64>,

    // Measurement Context
    pub measurement_source: Option<String>,
    pub bmi_calculated: Option<bool>,
    pub measurement_reliability: Option<String>,
    pub body_composition_method: Option<String>,
    pub fitness_phase: Option<String>,
    pub measurement_conditions: Option<String>,
    pub measurement_notes: Option<String>,
    pub source_device: Option<String>,
}

/// Body measurements query parameters
#[derive(Debug, Deserialize)]
pub struct BodyMeasurementsQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub measurement_type: Option<String>, // weight, bmi, body_fat, height, circumference
    pub measurement_source: Option<String>, // manual, smart_scale, apple_watch
    pub include_analysis: Option<bool>,
}

/// Body measurements ingestion response
#[derive(Debug, Serialize)]
pub struct BodyMeasurementsIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<BodyMeasurementProcessingError>,
    pub body_composition_analysis: Option<BodyCompositionAnalysis>,
    pub fitness_insights: Option<FitnessInsights>,
}

/// Body measurement processing error details
#[derive(Debug, Serialize)]
pub struct BodyMeasurementProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub measurement_value: Option<f64>,
    pub measurement_type: Option<String>,
    pub validation_context: Option<String>,
}

/// Body composition analysis summary
#[derive(Debug, Serialize)]
pub struct BodyCompositionAnalysis {
    pub weight_trend: Option<WeightTrend>,
    pub bmi_category: Option<String>, // underweight, normal, overweight, obese
    pub body_fat_category: Option<String>, // essential_fat, athletic, fitness, average, above_average
    pub waist_to_hip_ratio: Option<f64>,
    pub cardiovascular_risk_indicators: Vec<String>,
    pub composition_consistency: bool, // whether measurements are internally consistent
}

/// Weight trend analysis
#[derive(Debug, Serialize)]
pub struct WeightTrend {
    pub current_weight: f64,
    pub weight_change_kg: Option<f64>, // vs previous measurement
    pub weight_change_percentage: Option<f64>,
    pub trend_direction: String, // increasing, decreasing, stable
    pub trend_period_days: i32,
}

/// Fitness insights from body measurements
#[derive(Debug, Serialize)]
pub struct FitnessInsights {
    pub muscle_mass_trends: Option<MuscleMassTrend>,
    pub body_recomposition_score: Option<f64>, // 0-100 scale
    pub fitness_phase_recommendation: Option<String>,
    pub measurement_recommendations: Vec<String>,
    pub progress_indicators: Vec<ProgressIndicator>,
}

/// Muscle mass trend analysis
#[derive(Debug, Serialize)]
pub struct MuscleMassTrend {
    pub lean_mass_change_kg: Option<f64>,
    pub muscle_to_fat_ratio: Option<f64>,
    pub body_recomposition_trend: String, // gaining_muscle, losing_fat, maintaining, mixed
}

/// Progress indicator for fitness tracking
#[derive(Debug, Serialize)]
pub struct ProgressIndicator {
    pub metric: String,
    pub value: f64,
    pub change_since_last: Option<f64>,
    pub percentile_rank: Option<f64>, // compared to healthy population
    pub target_recommendation: Option<String>,
}

/// Body measurements data response
#[derive(Debug, Serialize)]
pub struct BodyMeasurementsDataResponse {
    pub body_measurements: Vec<BodyMeasurementMetric>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub summary: BodyMeasurementsSummary,
    pub trends: Option<BodyMeasurementTrends>,
}

/// Date range for body measurements
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Summary statistics for body measurements
#[derive(Debug, Serialize)]
pub struct BodyMeasurementsSummary {
    pub latest_weight: Option<f64>,
    pub latest_bmi: Option<f64>,
    pub latest_body_fat: Option<f64>,
    pub latest_measurements_date: Option<DateTime<Utc>>,
    pub measurement_frequency_days: Option<f64>,
    pub primary_measurement_source: Option<String>,
    pub total_measurement_sessions: i64,
}

/// Body measurement trends over time
#[derive(Debug, Serialize)]
pub struct BodyMeasurementTrends {
    pub weight_trend_30_days: Option<TrendData>,
    pub bmi_trend_30_days: Option<TrendData>,
    pub body_fat_trend_30_days: Option<TrendData>,
    pub measurement_consistency_score: f64, // 0-100, how consistent are measurements
}

/// Trend data for a specific measurement
#[derive(Debug, Serialize)]
pub struct TrendData {
    pub current_value: f64,
    pub change_absolute: f64,
    pub change_percentage: f64,
    pub trend_direction: String, // up, down, stable
    pub data_points_count: i32,
}

/// Ingest body measurements data
#[instrument(skip(pool, batch_processor, metrics))]
pub async fn ingest_body_measurements(
    auth: AuthContext,
    payload: web::Json<BodyMeasurementsIngestPayload>,
    pool: web::Data<PgPool>,
    batch_processor: web::Data<BatchProcessor>,
    metrics: web::Data<Metrics>,
    validation_config: web::Data<ValidationConfig>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    info!(
        user_id = %auth.user_id,
        measurement_count = payload.body_measurements.len(),
        "Processing body measurements ingestion request"
    );

    // Increment ingestion counter
    metrics.increment_counter("body_measurements_ingest_total", &[]);

    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();
    let mut body_measurements = Vec::new();

    // Process and validate each body measurement
    for (index, measurement_request) in payload.body_measurements.iter().enumerate() {
        match convert_and_validate_body_measurement(
            measurement_request,
            auth.user_id,
            &validation_config,
        ) {
            Ok(body_measurement) => {
                body_measurements.push(body_measurement);
                processed_count += 1;
            }
            Err(error_msg) => {
                failed_count += 1;
                errors.push(BodyMeasurementProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: error_msg,
                    measurement_value: measurement_request.body_weight_kg
                        .or(measurement_request.body_mass_index)
                        .or(measurement_request.body_fat_percentage),
                    measurement_type: determine_primary_measurement_type(measurement_request),
                    validation_context: Some("body_measurement_validation".to_string()),
                });
            }
        }
    }

    // Store measurements using batch processor if we have any valid data
    let mut processing_results = None;
    if !body_measurements.is_empty() {
        match batch_processor.process_body_measurements(&body_measurements).await {
            Ok(stats) => {
                info!(
                    user_id = %auth.user_id,
                    processed = stats.processed,
                    duplicates = stats.duplicates_skipped,
                    "Body measurements batch processing completed"
                );
                processing_results = Some(stats);
            }
            Err(e) => {
                error!(
                    user_id = %auth.user_id,
                    error = %e,
                    "Failed to process body measurements batch"
                );
                metrics.increment_counter("body_measurements_ingest_errors_total", &[]);
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "database_error",
                    "message": "Failed to store body measurements data"
                })));
            }
        }
    }

    // Generate analysis if we have measurements
    let body_composition_analysis = if !body_measurements.is_empty() {
        generate_body_composition_analysis(&body_measurements, &pool).await.ok()
    } else {
        None
    };

    let fitness_insights = if !body_measurements.is_empty() {
        generate_fitness_insights(&body_measurements, auth.user_id, &pool).await.ok()
    } else {
        None
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Record metrics
    metrics.record_histogram("body_measurements_ingest_duration_seconds",
                           processing_time_ms as f64 / 1000.0, &[]);

    info!(
        user_id = %auth.user_id,
        processed_count,
        failed_count,
        processing_time_ms,
        "Body measurements ingestion completed"
    );

    Ok(HttpResponse::Ok().json(BodyMeasurementsIngestResponse {
        success: processed_count > 0,
        processed_count,
        failed_count,
        processing_time_ms,
        errors,
        body_composition_analysis,
        fitness_insights,
    }))
}

/// Retrieve body measurements data
#[instrument(skip(pool))]
pub async fn get_body_measurements_data(
    auth: AuthContext,
    query: web::Query<BodyMeasurementsQueryParams>,
    pool: web::Data<PgPool>,
) -> ActixResult<HttpResponse> {
    info!(
        user_id = %auth.user_id,
        "Retrieving body measurements data"
    );

    // Build query with filters
    let limit = query.limit.unwrap_or(100).min(1000); // Cap at 1000
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT * FROM body_measurements WHERE user_id = "
    );
    query_builder.push_bind(auth.user_id);

    // Add date filters
    if let Some(start_date) = query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }
    if let Some(end_date) = query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    // Add measurement type filter
    if let Some(ref measurement_type) = query.measurement_type {
        match measurement_type.as_str() {
            "weight" => query_builder.push(" AND body_weight_kg IS NOT NULL"),
            "bmi" => query_builder.push(" AND body_mass_index IS NOT NULL"),
            "body_fat" => query_builder.push(" AND body_fat_percentage IS NOT NULL"),
            "height" => query_builder.push(" AND height_cm IS NOT NULL"),
            "circumference" => query_builder.push(" AND (waist_circumference_cm IS NOT NULL OR hip_circumference_cm IS NOT NULL)"),
            _ => {} // Invalid filter, ignore
        };
    }

    // Add measurement source filter
    if let Some(ref source) = query.measurement_source {
        query_builder.push(" AND measurement_source = ");
        query_builder.push_bind(source);
    }

    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let measurements_query = query_builder.build_query_as::<BodyMeasurementMetric>();

    match measurements_query.fetch_all(pool.as_ref()).await {
        Ok(measurements) => {
            // Get total count for pagination
            let count_query = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM body_measurements WHERE user_id = $1",
                auth.user_id
            );
            let total_count = count_query.fetch_one(pool.as_ref()).await.unwrap_or(0);

            // Generate summary
            let summary = generate_body_measurements_summary(&measurements, pool.as_ref(), auth.user_id).await;

            // Generate trends if requested
            let trends = if query.include_analysis.unwrap_or(false) {
                generate_body_measurement_trends(&measurements, auth.user_id, pool.as_ref()).await.ok()
            } else {
                None
            };

            // Determine date range
            let date_range = if !measurements.is_empty() {
                Some(DateRange {
                    start: measurements.last().unwrap().recorded_at,
                    end: measurements.first().unwrap().recorded_at,
                })
            } else {
                None
            };

            Ok(HttpResponse::Ok().json(BodyMeasurementsDataResponse {
                body_measurements: measurements,
                total_count,
                date_range,
                summary,
                trends,
            }))
        }
        Err(e) => {
            error!(
                user_id = %auth.user_id,
                error = %e,
                "Failed to retrieve body measurements data"
            );
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": "Failed to retrieve body measurements data"
            })))
        }
    }
}

/// Convert and validate body measurement request
fn convert_and_validate_body_measurement(
    request: &BodyMeasurementsIngestRequest,
    user_id: Uuid,
    validation_config: &ValidationConfig,
) -> Result<BodyMeasurementMetric, String> {
    // Validate weight
    if let Some(weight) = request.body_weight_kg {
        if weight < 20.0 || weight > 500.0 {
            return Err(format!("Body weight {} kg is outside valid range (20-500 kg)", weight));
        }
    }

    // Validate BMI
    if let Some(bmi) = request.body_mass_index {
        if bmi < 10.0 || bmi > 60.0 {
            return Err(format!("BMI {} is outside valid range (10-60)", bmi));
        }
    }

    // Validate body fat percentage
    if let Some(body_fat) = request.body_fat_percentage {
        if body_fat < 3.0 || body_fat > 50.0 {
            return Err(format!("Body fat percentage {}% is outside valid range (3-50%)", body_fat));
        }
    }

    // Validate height
    if let Some(height) = request.height_cm {
        if height < 50.0 || height > 250.0 {
            return Err(format!("Height {} cm is outside valid range (50-250 cm)", height));
        }
    }

    // Validate BMI consistency if we have weight, height, and BMI
    if let (Some(weight), Some(height), Some(bmi)) =
        (request.body_weight_kg, request.height_cm, request.body_mass_index) {
        let calculated_bmi = weight / ((height / 100.0) * (height / 100.0));
        let bmi_difference = (bmi - calculated_bmi).abs();
        if bmi_difference > 1.0 {
            warn!(
                "BMI inconsistency detected: provided BMI {} vs calculated BMI {} (diff: {})",
                bmi, calculated_bmi, bmi_difference
            );
        }
    }

    // Create the metric
    Ok(BodyMeasurementMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        body_weight_kg: request.body_weight_kg,
        body_mass_index: request.body_mass_index,
        body_fat_percentage: request.body_fat_percentage,
        lean_body_mass_kg: request.lean_body_mass_kg,
        waist_circumference_cm: request.waist_circumference_cm,
        hip_circumference_cm: request.hip_circumference_cm,
        chest_circumference_cm: request.chest_circumference_cm,
        arm_circumference_cm: request.arm_circumference_cm,
        thigh_circumference_cm: request.thigh_circumference_cm,
        body_temperature_celsius: request.body_temperature_celsius,
        basal_body_temperature_celsius: request.basal_body_temperature_celsius,
        measurement_source: request.measurement_source.clone(),
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    })
}

/// Determine the primary measurement type from the request
fn determine_primary_measurement_type(request: &BodyMeasurementsIngestRequest) -> Option<String> {
    if request.body_weight_kg.is_some() {
        Some("weight".to_string())
    } else if request.body_mass_index.is_some() {
        Some("bmi".to_string())
    } else if request.body_fat_percentage.is_some() {
        Some("body_fat".to_string())
    } else if request.height_cm.is_some() {
        Some("height".to_string())
    } else if request.waist_circumference_cm.is_some() {
        Some("waist_circumference".to_string())
    } else {
        None
    }
}

/// Generate body composition analysis
async fn generate_body_composition_analysis(
    measurements: &[BodyMeasurementMetric],
    pool: &PgPool,
) -> Result<BodyCompositionAnalysis, sqlx::Error> {
    let latest_measurement = measurements.first();

    if let Some(latest) = latest_measurement {
        // Determine BMI category
        let bmi_category = latest.body_mass_index.map(|bmi| {
            match bmi {
                bmi if bmi < 18.5 => "underweight".to_string(),
                bmi if bmi < 25.0 => "normal".to_string(),
                bmi if bmi < 30.0 => "overweight".to_string(),
                _ => "obese".to_string(),
            }
        });

        // Determine body fat category (gender-neutral)
        let body_fat_category = latest.body_fat_percentage.map(|bf| {
            match bf {
                bf if bf <= 9.0 => "essential_fat".to_string(),
                bf if bf <= 16.0 => "athletic".to_string(),
                bf if bf <= 20.0 => "fitness".to_string(),
                bf if bf <= 28.0 => "average".to_string(),
                _ => "above_average".to_string(),
            }
        });

        // Calculate waist-to-hip ratio
        let waist_to_hip_ratio = match (latest.waist_circumference_cm, latest.hip_circumference_cm) {
            (Some(waist), Some(hip)) => Some(waist / hip),
            _ => None,
        };

        // Generate cardiovascular risk indicators
        let mut risk_indicators = Vec::new();
        if let Some(ratio) = waist_to_hip_ratio {
            if ratio > 0.9 { // General threshold
                risk_indicators.push("elevated_waist_to_hip_ratio".to_string());
            }
        }
        if let Some(bmi) = latest.body_mass_index {
            if bmi >= 30.0 {
                risk_indicators.push("obesity".to_string());
            }
        }

        Ok(BodyCompositionAnalysis {
            weight_trend: None, // Would require historical data analysis
            bmi_category,
            body_fat_category,
            waist_to_hip_ratio,
            cardiovascular_risk_indicators: risk_indicators,
            composition_consistency: true, // Simplified for now
        })
    } else {
        Ok(BodyCompositionAnalysis {
            weight_trend: None,
            bmi_category: None,
            body_fat_category: None,
            waist_to_hip_ratio: None,
            cardiovascular_risk_indicators: Vec::new(),
            composition_consistency: true,
        })
    }
}

/// Generate fitness insights
async fn generate_fitness_insights(
    measurements: &[BodyMeasurementMetric],
    user_id: Uuid,
    pool: &PgPool,
) -> Result<FitnessInsights, sqlx::Error> {
    // This would typically involve more complex analysis
    // For now, provide basic insights

    let latest_measurement = measurements.first();
    let mut progress_indicators = Vec::new();

    if let Some(latest) = latest_measurement {
        // Add progress indicators for available metrics
        if let Some(weight) = latest.body_weight_kg {
            progress_indicators.push(ProgressIndicator {
                metric: "body_weight_kg".to_string(),
                value: weight,
                change_since_last: None, // Would require historical comparison
                percentile_rank: None,
                target_recommendation: Some("maintain_or_reduce".to_string()),
            });
        }

        if let Some(body_fat) = latest.body_fat_percentage {
            progress_indicators.push(ProgressIndicator {
                metric: "body_fat_percentage".to_string(),
                value: body_fat,
                change_since_last: None,
                percentile_rank: None,
                target_recommendation: Some("optimize_for_health".to_string()),
            });
        }
    }

    Ok(FitnessInsights {
        muscle_mass_trends: None,
        body_recomposition_score: None,
        fitness_phase_recommendation: Some("maintenance".to_string()),
        measurement_recommendations: vec![
            "Take measurements at consistent times".to_string(),
            "Use the same measurement source when possible".to_string(),
            "Consider body composition tracking".to_string(),
        ],
        progress_indicators,
    })
}

/// Generate body measurements summary
async fn generate_body_measurements_summary(
    measurements: &[BodyMeasurementMetric],
    pool: &PgPool,
    user_id: Uuid,
) -> BodyMeasurementsSummary {
    let latest = measurements.first();

    BodyMeasurementsSummary {
        latest_weight: latest.and_then(|m| m.body_weight_kg),
        latest_bmi: latest.and_then(|m| m.body_mass_index),
        latest_body_fat: latest.and_then(|m| m.body_fat_percentage),
        latest_measurements_date: latest.map(|m| m.recorded_at),
        measurement_frequency_days: None, // Would require calculation
        primary_measurement_source: latest.and_then(|m| m.measurement_source.clone()),
        total_measurement_sessions: measurements.len() as i64,
    }
}

/// Generate body measurement trends
async fn generate_body_measurement_trends(
    measurements: &[BodyMeasurementMetric],
    user_id: Uuid,
    pool: &PgPool,
) -> Result<BodyMeasurementTrends, sqlx::Error> {
    // This would involve more sophisticated trend analysis
    // For now, provide basic trend structure

    Ok(BodyMeasurementTrends {
        weight_trend_30_days: None,
        bmi_trend_30_days: None,
        body_fat_trend_30_days: None,
        measurement_consistency_score: 85.0, // Placeholder
    })
}