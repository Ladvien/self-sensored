use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::BloodGlucoseMetric;
use crate::services::auth::AuthContext;

/// Metabolic data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct MetabolicIngestPayload {
    pub blood_glucose_metrics: Option<Vec<BloodGlucoseIngestRequest>>,
    pub metabolic_metrics: Option<Vec<MetabolicIngestRequest>>,
}

/// Blood glucose metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct BloodGlucoseIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub blood_glucose_mg_dl: f64,
    pub measurement_context: Option<String>, // "fasting", "post_meal", "random", etc.
    pub medication_taken: Option<bool>,
    pub insulin_delivery_units: Option<f64>,
    pub glucose_source: Option<String>, // CGM device identifier
    pub notes: Option<String>,
    pub source_device: Option<String>,
}

/// Metabolic metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct MetabolicIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub blood_alcohol_content: Option<f64>,
    pub insulin_delivery_units: Option<f64>,
    pub delivery_method: Option<String>, // "pump", "pen", "syringe"
    pub source_device: Option<String>,
}

/// Metabolic data query parameters
#[derive(Debug, Deserialize)]
pub struct MetabolicQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub metric_type: Option<String>, // "blood_glucose", "metabolic"
    pub glucose_context: Option<String>, // "fasting", "post_meal", etc.
    pub source: Option<String>,
}

/// Blood glucose ingestion response
#[derive(Debug, Serialize)]
pub struct BloodGlucoseIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<BloodGlucoseProcessingError>,
    pub glucose_analysis: Option<BloodGlucoseAnalysis>,
}

/// Blood glucose processing error details
#[derive(Debug, Serialize)]
pub struct BloodGlucoseProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub glucose_value: Option<f64>,
    pub context: Option<String>,
}

/// Blood glucose analysis summary
#[derive(Debug, Serialize)]
pub struct BloodGlucoseAnalysis {
    pub average_glucose_mg_dl: Option<f64>,
    pub glucose_range: Option<GlucoseRange>,
    pub readings_by_context: Vec<ContextReadingCount>,
    pub critical_readings: Vec<CriticalGlucoseReading>,
    pub glucose_category: GlucoseCategory,
    pub time_in_range_percentage: Option<f64>, // 70-180 mg/dL
    pub variability_coefficient: Option<f64>,
}

/// Glucose range summary
#[derive(Debug, Serialize)]
pub struct GlucoseRange {
    pub min: f64,
    pub max: f64,
    pub average: f64,
    pub readings_count: usize,
}

/// Context-based reading counts
#[derive(Debug, Serialize)]
pub struct ContextReadingCount {
    pub context: String,
    pub count: usize,
    pub average_glucose: f64,
}

/// Critical glucose reading alert
#[derive(Debug, Serialize)]
pub struct CriticalGlucoseReading {
    pub glucose_mg_dl: f64,
    pub context: Option<String>,
    pub severity: GlucoseSeverity,
    pub recorded_at: DateTime<Utc>,
    pub recommendation: String,
}

/// Glucose severity levels
#[derive(Debug, Serialize)]
pub enum GlucoseSeverity {
    Hypoglycemic,      // < 70 mg/dL
    SevereHypoglycemic,// < 54 mg/dL
    Hyperglycemic,     // > 250 mg/dL
    SevereHyperglycemic, // > 400 mg/dL
}

/// Glucose category classification
#[derive(Debug, Serialize)]
pub enum GlucoseCategory {
    NormalFasting,        // 70-99 mg/dL fasting
    PreDiabetic,          // 100-125 mg/dL fasting
    DiabeticControlled,   // Managed diabetes with good control
    DiabeticUncontrolled, // Poor glucose control
    Unknown,              // Insufficient data
}

/// Metabolic ingestion response
#[derive(Debug, Serialize)]
pub struct MetabolicIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<MetabolicProcessingError>,
}

/// Metabolic processing error details
#[derive(Debug, Serialize)]
pub struct MetabolicProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub metric_value: Option<f64>,
    pub metric_type: Option<String>,
}

/// Metabolic data response structures
#[derive(Debug, Serialize)]
pub struct MetabolicDataResponse {
    pub blood_glucose_data: Vec<BloodGlucoseMetric>,
    pub metabolic_data: Vec<MetabolicMetric>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub glucose_summary: Option<BloodGlucoseAnalysis>,
}

/// Metabolic metric structure (for database storage)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetabolicMetric {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub blood_alcohol_content: Option<f64>,
    pub insulin_delivery_units: Option<f64>,
    pub delivery_method: Option<String>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Date range for queries
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Blood glucose data ingestion handler
/// POST /api/v1/ingest/blood-glucose
#[instrument(skip(pool, auth_context, payload, metrics))]
pub async fn ingest_blood_glucose_handler(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    payload: web::Json<Vec<BloodGlucoseIngestRequest>>,
    metrics: web::Data<Metrics>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth_context.user_id;

    info!(
        user_id = %user_id,
        glucose_count = payload.len(),
        "Processing blood glucose ingestion"
    );

    // Increment metrics
    metrics.ingest_requests_total.inc();
    metrics
        .ingest_metrics_total
        .with_label_values(&["blood_glucose"])
        .inc();

    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();
    let mut glucose_values = Vec::new();

    // Validate and convert to BloodGlucoseMetric
    for (index, request) in payload.iter().enumerate() {
        match validate_and_convert_glucose_request(user_id, request) {
            Ok(glucose_metric) => {
                glucose_values.push(glucose_metric.blood_glucose_mg_dl);

                // Store individual glucose reading
                match store_blood_glucose_metric(&pool, &glucose_metric).await {
                    Ok(()) => {
                        processed_count += 1;

                        // Log critical glucose levels
                        if is_critical_glucose_level(glucose_metric.blood_glucose_mg_dl) {
                            warn!(
                                user_id = %user_id,
                                glucose_mg_dl = glucose_metric.blood_glucose_mg_dl,
                                context = ?glucose_metric.measurement_context,
                                "Critical blood glucose level detected"
                            );
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        error!(
                            user_id = %user_id,
                            error = %e,
                            glucose_value = request.blood_glucose_mg_dl,
                            "Failed to store blood glucose metric"
                        );

                        errors.push(BloodGlucoseProcessingError {
                            index,
                            error_type: "storage_error".to_string(),
                            message: format!("Failed to store glucose reading: {}", e),
                            glucose_value: Some(request.blood_glucose_mg_dl),
                            context: request.measurement_context.clone(),
                        });
                    }
                }
            }
            Err(validation_error) => {
                failed_count += 1;
                errors.push(BloodGlucoseProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: validation_error,
                    glucose_value: Some(request.blood_glucose_mg_dl),
                    context: request.measurement_context.clone(),
                });
            }
        }
    }

    // Generate glucose analysis
    let glucose_analysis = if processed_count > 0 {
        Some(generate_glucose_analysis(&glucose_values, &payload))
    } else {
        None
    };

    let processing_time = start_time.elapsed();

    // Update processing time metrics
    metrics
        .ingest_duration_seconds
        .with_label_values(&["blood_glucose"])
        .observe(processing_time.as_secs_f64());

    info!(
        user_id = %user_id,
        processed = processed_count,
        failed = failed_count,
        processing_time_ms = processing_time.as_millis(),
        "Blood glucose ingestion completed"
    );

    let response = BloodGlucoseIngestResponse {
        success: failed_count == 0,
        processed_count,
        failed_count,
        processing_time_ms: processing_time.as_millis() as u64,
        errors,
        glucose_analysis,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Metabolic data ingestion handler
/// POST /api/v1/ingest/metabolic
#[instrument(skip(pool, auth_context, payload, metrics))]
pub async fn ingest_metabolic_handler(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    payload: web::Json<Vec<MetabolicIngestRequest>>,
    metrics: web::Data<Metrics>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth_context.user_id;

    info!(
        user_id = %user_id,
        metabolic_count = payload.len(),
        "Processing metabolic data ingestion"
    );

    // Increment metrics
    metrics.ingest_requests_total.inc();
    metrics
        .ingest_metrics_total
        .with_label_values(&["metabolic"])
        .inc();

    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    // Process each metabolic metric
    for (index, request) in payload.iter().enumerate() {
        match validate_and_convert_metabolic_request(user_id, request) {
            Ok(metabolic_metric) => {
                // Store metabolic metric
                match store_metabolic_metric(&pool, &metabolic_metric).await {
                    Ok(()) => {
                        processed_count += 1;

                        // Log significant insulin deliveries
                        if let Some(insulin_units) = metabolic_metric.insulin_delivery_units {
                            if insulin_units > 10.0 {
                                info!(
                                    user_id = %user_id,
                                    insulin_units = insulin_units,
                                    delivery_method = ?metabolic_metric.delivery_method,
                                    "Significant insulin delivery recorded"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        error!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to store metabolic metric"
                        );

                        errors.push(MetabolicProcessingError {
                            index,
                            error_type: "storage_error".to_string(),
                            message: format!("Failed to store metabolic data: {}", e),
                            metric_value: request.insulin_delivery_units.or(request.blood_alcohol_content),
                            metric_type: Some("metabolic".to_string()),
                        });
                    }
                }
            }
            Err(validation_error) => {
                failed_count += 1;
                errors.push(MetabolicProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: validation_error,
                    metric_value: request.insulin_delivery_units.or(request.blood_alcohol_content),
                    metric_type: Some("metabolic".to_string()),
                });
            }
        }
    }

    let processing_time = start_time.elapsed();

    // Update processing time metrics
    metrics
        .ingest_duration_seconds
        .with_label_values(&["metabolic"])
        .observe(processing_time.as_secs_f64());

    info!(
        user_id = %user_id,
        processed = processed_count,
        failed = failed_count,
        processing_time_ms = processing_time.as_millis(),
        "Metabolic data ingestion completed"
    );

    let response = MetabolicIngestResponse {
        success: failed_count == 0,
        processed_count,
        failed_count,
        processing_time_ms: processing_time.as_millis() as u64,
        errors,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Blood glucose data retrieval handler
/// GET /api/v1/data/blood-glucose
#[instrument(skip(pool, auth_context, metrics))]
pub async fn get_blood_glucose_data_handler(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    query: web::Query<MetabolicQueryParams>,
    metrics: web::Data<Metrics>,
) -> ActixResult<HttpResponse> {
    let user_id = auth_context.user_id;

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        limit = ?query.limit,
        "Retrieving blood glucose data"
    );

    // Increment query metrics
    metrics
        .query_requests_total
        .with_label_values(&["blood_glucose"])
        .inc();

    match retrieve_blood_glucose_data(&pool, user_id, &query).await {
        Ok(glucose_data) => {
            info!(
                user_id = %user_id,
                data_count = glucose_data.len(),
                "Blood glucose data retrieved successfully"
            );

            let response = json!({
                "blood_glucose_data": glucose_data,
                "total_count": glucose_data.len(),
                "query_params": {
                    "start_date": query.start_date,
                    "end_date": query.end_date,
                    "limit": query.limit,
                    "glucose_context": query.glucose_context
                }
            });

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve blood glucose data"
            );

            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "database_error",
                "message": "Failed to retrieve blood glucose data"
            })))
        }
    }
}

/// Metabolic data retrieval handler
/// GET /api/v1/data/metabolic
#[instrument(skip(pool, auth_context, metrics))]
pub async fn get_metabolic_data_handler(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    query: web::Query<MetabolicQueryParams>,
    metrics: web::Data<Metrics>,
) -> ActixResult<HttpResponse> {
    let user_id = auth_context.user_id;

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        "Retrieving metabolic data"
    );

    // Increment query metrics
    metrics
        .query_requests_total
        .with_label_values(&["metabolic"])
        .inc();

    match retrieve_metabolic_data(&pool, user_id, &query).await {
        Ok(metabolic_data) => {
            info!(
                user_id = %user_id,
                data_count = metabolic_data.len(),
                "Metabolic data retrieved successfully"
            );

            let response = json!({
                "metabolic_data": metabolic_data,
                "total_count": metabolic_data.len(),
                "query_params": {
                    "start_date": query.start_date,
                    "end_date": query.end_date,
                    "limit": query.limit
                }
            });

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve metabolic data"
            );

            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "database_error",
                "message": "Failed to retrieve metabolic data"
            })))
        }
    }
}

// Helper functions for validation and database operations

/// Validate and convert blood glucose request to metric
fn validate_and_convert_glucose_request(
    user_id: Uuid,
    request: &BloodGlucoseIngestRequest,
) -> Result<BloodGlucoseMetric, String> {
    // Validate glucose range (30-600 mg/dL - medical device range)
    if request.blood_glucose_mg_dl < 30.0 || request.blood_glucose_mg_dl > 600.0 {
        return Err(format!(
            "Blood glucose value {} mg/dL is outside medical range (30-600 mg/dL)",
            request.blood_glucose_mg_dl
        ));
    }

    // Validate insulin units if provided
    if let Some(insulin_units) = request.insulin_delivery_units {
        if insulin_units < 0.0 || insulin_units > 100.0 {
            return Err(format!(
                "Insulin delivery units {} is outside safe range (0-100 units)",
                insulin_units
            ));
        }
    }

    // Validate measurement context if provided
    if let Some(context) = &request.measurement_context {
        let valid_contexts = ["fasting", "post_meal", "random", "bedtime", "pre_meal", "post_workout"];
        if !valid_contexts.contains(&context.as_str()) {
            return Err(format!(
                "Invalid measurement context '{}'. Valid contexts: {:?}",
                context, valid_contexts
            ));
        }
    }

    Ok(BloodGlucoseMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        blood_glucose_mg_dl: request.blood_glucose_mg_dl,
        measurement_context: request.measurement_context.clone(),
        medication_taken: request.medication_taken,
        insulin_delivery_units: request.insulin_delivery_units,
        glucose_source: request.glucose_source.clone(),
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    })
}

/// Validate and convert metabolic request to metric
fn validate_and_convert_metabolic_request(
    user_id: Uuid,
    request: &MetabolicIngestRequest,
) -> Result<MetabolicMetric, String> {
    // Validate blood alcohol content if provided (0.0-0.5%)
    if let Some(bac) = request.blood_alcohol_content {
        if bac < 0.0 || bac > 0.5 {
            return Err(format!(
                "Blood alcohol content {} is outside valid range (0.0-0.5%)",
                bac
            ));
        }
    }

    // Validate insulin units if provided
    if let Some(insulin_units) = request.insulin_delivery_units {
        if insulin_units < 0.0 || insulin_units > 100.0 {
            return Err(format!(
                "Insulin delivery units {} is outside safe range (0-100 units)",
                insulin_units
            ));
        }
    }

    // Validate delivery method if provided
    if let Some(method) = &request.delivery_method {
        let valid_methods = ["pump", "pen", "syringe", "inhaler", "patch"];
        if !valid_methods.contains(&method.as_str()) {
            return Err(format!(
                "Invalid delivery method '{}'. Valid methods: {:?}",
                method, valid_methods
            ));
        }
    }

    Ok(MetabolicMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        blood_alcohol_content: request.blood_alcohol_content,
        insulin_delivery_units: request.insulin_delivery_units,
        delivery_method: request.delivery_method.clone(),
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    })
}

/// Store blood glucose metric in database (uses existing blood_glucose_metrics table)
async fn store_blood_glucose_metric(
    pool: &PgPool,
    metric: &BloodGlucoseMetric,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO blood_glucose_metrics (
            id, user_id, recorded_at, blood_glucose_mg_dl, measurement_context,
            medication_taken, insulin_delivery_units, glucose_source, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (user_id, recorded_at, glucose_source)
        DO UPDATE SET
            blood_glucose_mg_dl = EXCLUDED.blood_glucose_mg_dl,
            measurement_context = EXCLUDED.measurement_context,
            medication_taken = EXCLUDED.medication_taken,
            insulin_delivery_units = EXCLUDED.insulin_delivery_units,
            source_device = EXCLUDED.source_device
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.blood_glucose_mg_dl,
        metric.measurement_context,
        metric.medication_taken,
        metric.insulin_delivery_units,
        metric.glucose_source,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Store metabolic metric in database
async fn store_metabolic_metric(
    pool: &PgPool,
    metric: &MetabolicMetric,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO metabolic_metrics (
            id, user_id, recorded_at, blood_alcohol_content,
            insulin_delivery_units, delivery_method, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (user_id, recorded_at)
        DO UPDATE SET
            blood_alcohol_content = EXCLUDED.blood_alcohol_content,
            insulin_delivery_units = EXCLUDED.insulin_delivery_units,
            delivery_method = EXCLUDED.delivery_method,
            source_device = EXCLUDED.source_device
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.blood_alcohol_content,
        metric.insulin_delivery_units,
        metric.delivery_method,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Retrieve blood glucose data from database
async fn retrieve_blood_glucose_data(
    pool: &PgPool,
    user_id: Uuid,
    query: &MetabolicQueryParams,
) -> Result<Vec<BloodGlucoseMetric>, sqlx::Error> {
    let limit = query.limit.unwrap_or(100).min(1000) as i64;

    let rows = sqlx::query_as!(
        BloodGlucoseMetric,
        r#"
        SELECT id, user_id, recorded_at, blood_glucose_mg_dl, measurement_context,
               medication_taken, insulin_delivery_units, glucose_source, source_device, created_at
        FROM blood_glucose_metrics
        WHERE user_id = $1
        AND ($2::timestamptz IS NULL OR recorded_at >= $2)
        AND ($3::timestamptz IS NULL OR recorded_at <= $3)
        AND ($4::text IS NULL OR measurement_context = $4)
        ORDER BY recorded_at DESC
        LIMIT $5
        "#,
        user_id,
        query.start_date,
        query.end_date,
        query.glucose_context,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Retrieve metabolic data from database
async fn retrieve_metabolic_data(
    pool: &PgPool,
    user_id: Uuid,
    query: &MetabolicQueryParams,
) -> Result<Vec<MetabolicMetric>, sqlx::Error> {
    let limit = query.limit.unwrap_or(100).min(1000) as i64;

    let rows = sqlx::query_as!(
        MetabolicMetric,
        r#"
        SELECT id, user_id, recorded_at, blood_alcohol_content,
               insulin_delivery_units, delivery_method, source_device, created_at
        FROM metabolic_metrics
        WHERE user_id = $1
        AND ($2::timestamptz IS NULL OR recorded_at >= $2)
        AND ($3::timestamptz IS NULL OR recorded_at <= $3)
        ORDER BY recorded_at DESC
        LIMIT $4
        "#,
        user_id,
        query.start_date,
        query.end_date,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Check if glucose level is critical
fn is_critical_glucose_level(glucose_mg_dl: f64) -> bool {
    glucose_mg_dl < 70.0 || glucose_mg_dl > 400.0
}

/// Generate glucose analysis from processed values
fn generate_glucose_analysis(
    glucose_values: &[f64],
    requests: &[BloodGlucoseIngestRequest],
) -> BloodGlucoseAnalysis {
    let readings_count = glucose_values.len();

    if readings_count == 0 {
        return BloodGlucoseAnalysis {
            average_glucose_mg_dl: None,
            glucose_range: None,
            readings_by_context: vec![],
            critical_readings: vec![],
            glucose_category: GlucoseCategory::Unknown,
            time_in_range_percentage: None,
            variability_coefficient: None,
        };
    }

    // Calculate basic statistics
    let sum: f64 = glucose_values.iter().sum();
    let average = sum / readings_count as f64;
    let min = glucose_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = glucose_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Time in range (70-180 mg/dL)
    let in_range_count = glucose_values.iter().filter(|&&v| v >= 70.0 && v <= 180.0).count();
    let time_in_range_percentage = (in_range_count as f64 / readings_count as f64) * 100.0;

    // Coefficient of variation
    let variance = glucose_values
        .iter()
        .map(|&x| (x - average).powi(2))
        .sum::<f64>() / readings_count as f64;
    let std_dev = variance.sqrt();
    let variability_coefficient = (std_dev / average) * 100.0;

    // Critical readings
    let critical_readings: Vec<CriticalGlucoseReading> = requests
        .iter()
        .filter_map(|req| {
            if req.blood_glucose_mg_dl < 54.0 {
                Some(CriticalGlucoseReading {
                    glucose_mg_dl: req.blood_glucose_mg_dl,
                    context: req.measurement_context.clone(),
                    severity: GlucoseSeverity::SevereHypoglycemic,
                    recorded_at: req.recorded_at,
                    recommendation: "Immediate treatment required".to_string(),
                })
            } else if req.blood_glucose_mg_dl < 70.0 {
                Some(CriticalGlucoseReading {
                    glucose_mg_dl: req.blood_glucose_mg_dl,
                    context: req.measurement_context.clone(),
                    severity: GlucoseSeverity::Hypoglycemic,
                    recorded_at: req.recorded_at,
                    recommendation: "Consume fast-acting carbohydrates".to_string(),
                })
            } else if req.blood_glucose_mg_dl > 400.0 {
                Some(CriticalGlucoseReading {
                    glucose_mg_dl: req.blood_glucose_mg_dl,
                    context: req.measurement_context.clone(),
                    severity: GlucoseSeverity::SevereHyperglycemic,
                    recorded_at: req.recorded_at,
                    recommendation: "Seek immediate medical attention".to_string(),
                })
            } else if req.blood_glucose_mg_dl > 250.0 {
                Some(CriticalGlucoseReading {
                    glucose_mg_dl: req.blood_glucose_mg_dl,
                    context: req.measurement_context.clone(),
                    severity: GlucoseSeverity::Hyperglycemic,
                    recorded_at: req.recorded_at,
                    recommendation: "Check ketones, contact healthcare provider".to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    // Glucose category classification
    let fasting_readings: Vec<f64> = requests
        .iter()
        .filter(|req| req.measurement_context.as_ref().map_or(false, |c| c == "fasting"))
        .map(|req| req.blood_glucose_mg_dl)
        .collect();

    let glucose_category = if !fasting_readings.is_empty() {
        let avg_fasting = fasting_readings.iter().sum::<f64>() / fasting_readings.len() as f64;
        if avg_fasting < 100.0 {
            GlucoseCategory::NormalFasting
        } else if avg_fasting < 126.0 {
            GlucoseCategory::PreDiabetic
        } else if time_in_range_percentage >= 70.0 {
            GlucoseCategory::DiabeticControlled
        } else {
            GlucoseCategory::DiabeticUncontrolled
        }
    } else if time_in_range_percentage >= 70.0 {
        GlucoseCategory::DiabeticControlled
    } else if average > 180.0 {
        GlucoseCategory::DiabeticUncontrolled
    } else {
        GlucoseCategory::Unknown
    };

    BloodGlucoseAnalysis {
        average_glucose_mg_dl: Some(average),
        glucose_range: Some(GlucoseRange {
            min,
            max,
            average,
            readings_count,
        }),
        readings_by_context: vec![], // Could be enhanced to group by context
        critical_readings,
        glucose_category,
        time_in_range_percentage: Some(time_in_range_percentage),
        variability_coefficient: Some(variability_coefficient),
    }
}