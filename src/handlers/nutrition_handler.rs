use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::{MacronutrientDistribution, NutritionMetric};
use crate::services::auth::AuthContext;

/// Comprehensive nutrition data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct NutritionIngestPayload {
    pub nutrition_metrics: Vec<NutritionIngestRequest>,
}

/// Individual nutrition metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct NutritionIngestRequest {
    pub recorded_at: DateTime<Utc>,

    // Hydration & Stimulants
    pub dietary_water: Option<f64>,    // liters
    pub dietary_caffeine: Option<f64>, // mg

    // Macronutrients (Core Energy)
    pub dietary_energy_consumed: Option<f64>,     // calories
    pub dietary_carbohydrates: Option<f64>,       // grams
    pub dietary_protein: Option<f64>,             // grams
    pub dietary_fat_total: Option<f64>,           // grams
    pub dietary_fat_saturated: Option<f64>,       // grams
    pub dietary_fat_monounsaturated: Option<f64>, // grams
    pub dietary_fat_polyunsaturated: Option<f64>, // grams
    pub dietary_cholesterol: Option<f64>,         // mg
    pub dietary_sodium: Option<f64>,              // mg
    pub dietary_fiber: Option<f64>,               // grams
    pub dietary_sugar: Option<f64>,               // grams

    // Essential Minerals
    pub dietary_calcium: Option<f64>,    // mg
    pub dietary_iron: Option<f64>,       // mg
    pub dietary_magnesium: Option<f64>,  // mg
    pub dietary_potassium: Option<f64>,  // mg
    pub dietary_zinc: Option<f64>,       // mg
    pub dietary_phosphorus: Option<f64>, // mg

    // Essential Vitamins (Water-soluble)
    pub dietary_vitamin_c: Option<f64>,             // mg
    pub dietary_vitamin_b1_thiamine: Option<f64>,   // mg
    pub dietary_vitamin_b2_riboflavin: Option<f64>, // mg
    pub dietary_vitamin_b3_niacin: Option<f64>,     // mg
    pub dietary_vitamin_b6_pyridoxine: Option<f64>, // mg
    pub dietary_vitamin_b12_cobalamin: Option<f64>, // mcg
    pub dietary_folate: Option<f64>,                // mcg
    pub dietary_biotin: Option<f64>,                // mcg
    pub dietary_pantothenic_acid: Option<f64>,      // mg

    // Essential Vitamins (Fat-soluble)
    pub dietary_vitamin_a: Option<f64>, // mcg RAE
    pub dietary_vitamin_d: Option<f64>, // IU
    pub dietary_vitamin_e: Option<f64>, // mg
    pub dietary_vitamin_k: Option<f64>, // mcg

    // Meal Context for atomic processing
    pub meal_type: Option<String>,   // breakfast, lunch, dinner, snack
    pub meal_id: Option<uuid::Uuid>, // Group nutrients from same meal

    // Metadata and source tracking
    pub source_device: Option<String>,
}

/// Nutrition data query parameters
#[derive(Debug, Deserialize)]
pub struct NutritionQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub nutrient_type: Option<String>, // macronutrients, vitamins, minerals, hydration
    pub include_analysis: Option<bool>,
    pub daily_aggregation: Option<bool>,
}

/// Hydration-specific query parameters for separate endpoint
#[derive(Debug, Deserialize)]
pub struct HydrationQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub include_caffeine: Option<bool>,
}

/// Nutrition ingestion response with comprehensive analysis
#[derive(Debug, Serialize)]
pub struct NutritionIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<NutritionProcessingError>,
    pub nutrition_analysis: Option<NutritionAnalysis>,
    pub daily_summary: Option<DailyNutritionSummary>,
}

/// Nutrition processing error details
#[derive(Debug, Serialize)]
pub struct NutritionProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub nutrient_name: Option<String>,
    pub nutrient_value: Option<f64>,
}

/// Comprehensive nutrition analysis
#[derive(Debug, Serialize)]
pub struct NutritionAnalysis {
    pub total_entries: usize,
    pub hydration_status: HydrationStatus,
    pub macronutrient_analysis: MacronutrientAnalysis,
    pub micronutrient_analysis: MicronutrientAnalysis,
    pub dietary_concerns: Vec<DietaryConcern>,
    pub balanced_meals_count: usize,
    pub excessive_sodium_alerts: usize,
    pub caffeine_warnings: usize,
}

/// Hydration status summary
#[derive(Debug, Serialize)]
pub struct HydrationStatus {
    pub total_water_liters: f64,
    pub daily_average_liters: f64,
    pub hydration_level: String, // severely_dehydrated, dehydrated, adequate, well_hydrated, overhydrated
    pub total_caffeine_mg: f64,
    pub exceeds_caffeine_limit: bool,
}

/// Macronutrient analysis summary
#[derive(Debug, Serialize)]
pub struct MacronutrientAnalysis {
    pub total_calories: f64,
    pub average_daily_calories: f64,
    pub total_protein_grams: f64,
    pub total_carbohydrates_grams: f64,
    pub total_fat_grams: f64,
    pub average_distribution: Option<MacronutrientDistribution>,
    pub calorie_range: NutrientRange,
}

/// Micronutrient analysis (vitamins and minerals)
#[derive(Debug, Serialize)]
pub struct MicronutrientAnalysis {
    pub vitamin_totals: VitaminTotals,
    pub mineral_totals: MineralTotals,
    pub deficiency_risks: Vec<String>,
    pub excess_warnings: Vec<String>,
}

/// Vitamin totals summary
#[derive(Debug, Serialize)]
pub struct VitaminTotals {
    pub vitamin_a_mcg: f64,
    pub vitamin_c_mg: f64,
    pub vitamin_d_iu: f64,
}

/// Mineral totals summary
#[derive(Debug, Serialize)]
pub struct MineralTotals {
    pub calcium_mg: f64,
    pub iron_mg: f64,
    pub magnesium_mg: f64,
    pub potassium_mg: f64,
    pub sodium_mg: f64,
}

/// Dietary concern alert
#[derive(Debug, Serialize)]
pub struct DietaryConcern {
    pub concern_type: String, // excessive_sodium, low_fiber, high_sugar, caffeine_overload
    pub severity: String,     // low, medium, high, critical
    pub description: String,
    pub recommendations: Vec<String>,
}

/// Nutrient range for analysis
#[derive(Debug, Serialize)]
pub struct NutrientRange {
    pub min: f64,
    pub max: f64,
    pub average: f64,
}

/// Daily nutrition summary for trend analysis
#[derive(Debug, Serialize)]
pub struct DailyNutritionSummary {
    pub date: DateTime<Utc>,
    pub total_calories: f64,
    pub water_intake_liters: f64,
    pub protein_grams: f64,
    pub carbohydrates_grams: f64,
    pub fat_grams: f64,
    pub fiber_grams: f64,
    pub sodium_mg: f64,
    pub macronutrient_distribution: Option<MacronutrientDistribution>,
    pub meal_count: usize,
}

/// Nutrition data response
#[derive(Debug, Serialize)]
pub struct NutritionDataResponse {
    pub nutrition_data: Vec<NutritionMetric>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub summary: Option<NutritionSummary>,
    pub daily_aggregations: Option<Vec<DailyNutritionSummary>>,
}

/// Hydration-specific data response
#[derive(Debug, Serialize)]
pub struct HydrationDataResponse {
    pub hydration_data: Vec<HydrationEntry>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub hydration_summary: HydrationSummary,
}

/// Hydration entry for specialized hydration endpoint
#[derive(Debug, Serialize)]
pub struct HydrationEntry {
    pub recorded_at: DateTime<Utc>,
    pub water_intake_liters: Option<f64>,
    pub caffeine_mg: Option<f64>,
    pub hydration_status: String,
    pub source_device: Option<String>,
}

/// Hydration summary statistics
#[derive(Debug, Serialize)]
pub struct HydrationSummary {
    pub total_water_liters: f64,
    pub daily_average_liters: f64,
    pub total_caffeine_mg: f64,
    pub hydration_days_count: usize,
    pub well_hydrated_days: usize,
    pub dehydrated_days: usize,
    pub overhydrated_days: usize,
}

/// Overall nutrition summary
#[derive(Debug, Serialize)]
pub struct NutritionSummary {
    pub total_entries: usize,
    pub date_range: DateRange,
    pub average_daily_calories: f64,
    pub average_daily_water_liters: f64,
    pub top_nutrients: Vec<TopNutrient>,
    pub dietary_patterns: Vec<DietaryPattern>,
}

/// Top nutrient consumption
#[derive(Debug, Serialize)]
pub struct TopNutrient {
    pub nutrient_name: String,
    pub total_amount: f64,
    pub unit: String,
    pub daily_average: f64,
}

/// Dietary pattern identification
#[derive(Debug, Serialize)]
pub struct DietaryPattern {
    pub pattern_name: String,
    pub description: String,
    pub frequency: usize,
    pub health_impact: String, // positive, neutral, concerning
}

/// Date range helper
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Ingest nutrition data endpoint
#[instrument(skip(pool, metrics, auth_context, payload))]
pub async fn ingest_nutrition_data(
    pool: web::Data<PgPool>,
    metrics: web::Data<Metrics>,
    auth_context: AuthContext,
    payload: web::Json<NutritionIngestPayload>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth_context.user.id;

    info!(
        user_id = %user_id,
        nutrition_count = payload.nutrition_metrics.len(),
        "Processing nutrition data ingestion"
    );

    // Convert ingest requests to NutritionMetric structs
    let mut nutrition_metrics = Vec::new();
    let mut errors = Vec::new();

    for (index, request) in payload.nutrition_metrics.iter().enumerate() {
        let nutrition_metric = NutritionMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: request.recorded_at,
            // Hydration & Stimulants
            dietary_water: request.dietary_water,
            dietary_caffeine: request.dietary_caffeine,
            // Macronutrients (Core Energy)
            dietary_energy_consumed: request.dietary_energy_consumed,
            dietary_carbohydrates: request.dietary_carbohydrates,
            dietary_protein: request.dietary_protein,
            dietary_fat_total: request.dietary_fat_total,
            dietary_fat_saturated: request.dietary_fat_saturated,
            dietary_fat_monounsaturated: request.dietary_fat_monounsaturated,
            dietary_fat_polyunsaturated: request.dietary_fat_polyunsaturated,
            dietary_cholesterol: request.dietary_cholesterol,
            dietary_sodium: request.dietary_sodium,
            dietary_fiber: request.dietary_fiber,
            dietary_sugar: request.dietary_sugar,
            // Essential Minerals
            dietary_calcium: request.dietary_calcium,
            dietary_iron: request.dietary_iron,
            dietary_magnesium: request.dietary_magnesium,
            dietary_potassium: request.dietary_potassium,
            dietary_zinc: request.dietary_zinc,
            dietary_phosphorus: request.dietary_phosphorus,
            // Essential Vitamins (Water-soluble)
            dietary_vitamin_c: request.dietary_vitamin_c,
            dietary_vitamin_b1_thiamine: request.dietary_vitamin_b1_thiamine,
            dietary_vitamin_b2_riboflavin: request.dietary_vitamin_b2_riboflavin,
            dietary_vitamin_b3_niacin: request.dietary_vitamin_b3_niacin,
            dietary_vitamin_b6_pyridoxine: request.dietary_vitamin_b6_pyridoxine,
            dietary_vitamin_b12_cobalamin: request.dietary_vitamin_b12_cobalamin,
            dietary_folate: request.dietary_folate,
            dietary_biotin: request.dietary_biotin,
            dietary_pantothenic_acid: request.dietary_pantothenic_acid,
            // Essential Vitamins (Fat-soluble)
            dietary_vitamin_a: request.dietary_vitamin_a,
            dietary_vitamin_d: request.dietary_vitamin_d,
            dietary_vitamin_e: request.dietary_vitamin_e,
            dietary_vitamin_k: request.dietary_vitamin_k,
            // Meal Context for atomic processing
            meal_type: request.meal_type.clone(),
            meal_id: request.meal_id,
            // Metadata and source tracking
            source_device: request.source_device.clone(),
            created_at: Utc::now(),
        };

        // Validate nutrition metric
        if let Err(validation_error) = nutrition_metric.validate() {
            errors.push(NutritionProcessingError {
                index,
                error_type: "validation_error".to_string(),
                message: validation_error,
                nutrient_name: None,
                nutrient_value: None,
            });
            continue;
        }

        nutrition_metrics.push(nutrition_metric);
    }

    // Process valid nutrition metrics with database insertion
    let mut processed_count = 0;
    if !nutrition_metrics.is_empty() {
        match insert_nutrition_metrics_batch(pool.get_ref(), &nutrition_metrics).await {
            Ok(count) => {
                processed_count = count;
                info!(
                    user_id = %user_id,
                    processed_count = count,
                    "Successfully processed nutrition metrics"
                );
            }
            Err(e) => {
                error!(
                    user_id = %user_id,
                    error = %e,
                    "Failed to process nutrition metrics"
                );
                errors.push(NutritionProcessingError {
                    index: 0,
                    error_type: "batch_processing_error".to_string(),
                    message: format!("Failed to process nutrition data: {}", e),
                    nutrient_name: None,
                    nutrient_value: None,
                });
            }
        }
    }

    // Generate nutrition analysis
    let nutrition_analysis = generate_nutrition_analysis(&nutrition_metrics);
    let daily_summary = generate_daily_nutrition_summary(&nutrition_metrics);

    // Update metrics
    metrics.record_nutrition_ingest(processed_count as u64);
    for error in &errors {
        metrics.record_nutrition_error(&error.error_type);
    }

    let processing_time = start_time.elapsed().as_millis() as u64;

    let response = NutritionIngestResponse {
        success: errors.is_empty(),
        processed_count,
        failed_count: errors.len(),
        processing_time_ms: processing_time,
        errors,
        nutrition_analysis,
        daily_summary,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get nutrition data endpoint
#[instrument(skip(pool, auth_context))]
pub async fn get_nutrition_data(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    query: web::Query<NutritionQueryParams>,
) -> ActixResult<HttpResponse> {
    let user_id = auth_context.user.id;

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        "Retrieving nutrition data"
    );

    // Build query with optional filters
    let mut sql_query = "SELECT * FROM nutrition_metrics WHERE user_id = $1".to_string();
    let mut query_builder = sqlx::QueryBuilder::new(&sql_query);
    query_builder.push_bind(&user_id);

    // Add date range filters
    if let Some(start_date) = &query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = &query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    // Add ordering and limit
    query_builder.push(" ORDER BY recorded_at DESC");

    if let Some(limit) = query.limit {
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
    }

    // Execute query
    let nutrition_data = match query_builder
        .build_query_as::<NutritionMetric>()
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(data) => data,
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve nutrition data"
            );
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": "Failed to retrieve nutrition data"
            })));
        }
    };

    // Get total count
    let total_count = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM nutrition_metrics WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(count) => count,
        Err(_) => nutrition_data.len() as i64,
    };

    // Generate date range if data exists
    let date_range = if !nutrition_data.is_empty() {
        Some(DateRange {
            start: nutrition_data.last().unwrap().recorded_at,
            end: nutrition_data.first().unwrap().recorded_at,
        })
    } else {
        None
    };

    // Generate summary if requested
    let summary = if query.include_analysis.unwrap_or(false) {
        Some(generate_nutrition_summary(&nutrition_data))
    } else {
        None
    };

    // Generate daily aggregations if requested
    let daily_aggregations = if query.daily_aggregation.unwrap_or(false) {
        Some(generate_daily_aggregations(&nutrition_data))
    } else {
        None
    };

    let response = NutritionDataResponse {
        nutrition_data,
        total_count,
        date_range,
        summary,
        daily_aggregations,
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get hydration data endpoint (specialized for water and caffeine tracking)
#[instrument(skip(pool, auth_context))]
pub async fn get_hydration_data(
    pool: web::Data<PgPool>,
    auth_context: AuthContext,
    query: web::Query<HydrationQueryParams>,
) -> ActixResult<HttpResponse> {
    let user_id = auth_context.user.id;

    info!(
        user_id = %user_id,
        "Retrieving hydration data"
    );

    // Build hydration-specific query
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT recorded_at, dietary_water, dietary_caffeine, source_device FROM nutrition_metrics WHERE user_id = "
    );
    query_builder.push_bind(&user_id);

    if let Some(start_date) = &query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = &query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    query_builder.push(" AND (dietary_water IS NOT NULL OR dietary_caffeine IS NOT NULL)");
    query_builder.push(" ORDER BY recorded_at DESC");

    if let Some(limit) = query.limit {
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
    }

    // Execute hydration-specific query
    let hydration_entries = match query_builder
        .build_query_as::<(DateTime<Utc>, Option<f64>, Option<f64>, Option<String>)>()
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(data) => data
            .into_iter()
            .map(|(recorded_at, water, caffeine, device)| HydrationEntry {
                recorded_at,
                water_intake_liters: water,
                caffeine_mg: caffeine,
                hydration_status: get_hydration_status_from_value(water),
                source_device: device,
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve hydration data"
            );
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": "Failed to retrieve hydration data"
            })));
        }
    };

    let total_count = hydration_entries.len() as i64;

    // Generate hydration summary
    let hydration_summary = generate_hydration_summary(&hydration_entries);

    let date_range = if !hydration_entries.is_empty() {
        Some(DateRange {
            start: hydration_entries.last().unwrap().recorded_at,
            end: hydration_entries.first().unwrap().recorded_at,
        })
    } else {
        None
    };

    let response = HydrationDataResponse {
        hydration_data: hydration_entries,
        total_count,
        date_range,
        hydration_summary,
    };

    Ok(HttpResponse::Ok().json(response))
}

// Database operations

/// Insert nutrition metrics in batches with conflict resolution
async fn insert_nutrition_metrics_batch(
    pool: &PgPool,
    metrics: &[NutritionMetric],
) -> Result<usize, sqlx::Error> {
    if metrics.is_empty() {
        return Ok(0);
    }

    let chunk_size = 1000; // Process in chunks to avoid parameter limits
    let mut total_processed = 0;

    for chunk in metrics.chunks(chunk_size) {
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO nutrition_metrics (
                id, user_id, recorded_at,
                dietary_water, dietary_caffeine,
                dietary_energy_consumed, dietary_carbohydrates, dietary_protein,
                dietary_fat_total, dietary_fat_saturated, dietary_fat_monounsaturated, dietary_fat_polyunsaturated,
                dietary_cholesterol, dietary_sodium, dietary_fiber, dietary_sugar,
                dietary_calcium, dietary_iron, dietary_magnesium, dietary_potassium, dietary_zinc, dietary_phosphorus,
                dietary_vitamin_c, dietary_vitamin_b1_thiamine, dietary_vitamin_b2_riboflavin, dietary_vitamin_b3_niacin,
                dietary_vitamin_b6_pyridoxine, dietary_vitamin_b12_cobalamin, dietary_folate, dietary_biotin, dietary_pantothenic_acid,
                dietary_vitamin_a, dietary_vitamin_d, dietary_vitamin_e, dietary_vitamin_k,
                meal_type, meal_id, source_device, created_at
            ) ",
        );

        query_builder.push_values(chunk.iter(), |mut b, metric| {
            b.push_bind(&metric.id)
                .push_bind(&metric.user_id)
                .push_bind(&metric.recorded_at)
                // Hydration & Stimulants
                .push_bind(&metric.dietary_water)
                .push_bind(&metric.dietary_caffeine)
                // Macronutrients (Core Energy)
                .push_bind(&metric.dietary_energy_consumed)
                .push_bind(&metric.dietary_carbohydrates)
                .push_bind(&metric.dietary_protein)
                .push_bind(&metric.dietary_fat_total)
                .push_bind(&metric.dietary_fat_saturated)
                .push_bind(&metric.dietary_fat_monounsaturated)
                .push_bind(&metric.dietary_fat_polyunsaturated)
                .push_bind(&metric.dietary_cholesterol)
                .push_bind(&metric.dietary_sodium)
                .push_bind(&metric.dietary_fiber)
                .push_bind(&metric.dietary_sugar)
                // Essential Minerals
                .push_bind(&metric.dietary_calcium)
                .push_bind(&metric.dietary_iron)
                .push_bind(&metric.dietary_magnesium)
                .push_bind(&metric.dietary_potassium)
                .push_bind(&metric.dietary_zinc)
                .push_bind(&metric.dietary_phosphorus)
                // Essential Vitamins (Water-soluble)
                .push_bind(&metric.dietary_vitamin_c)
                .push_bind(&metric.dietary_vitamin_b1_thiamine)
                .push_bind(&metric.dietary_vitamin_b2_riboflavin)
                .push_bind(&metric.dietary_vitamin_b3_niacin)
                .push_bind(&metric.dietary_vitamin_b6_pyridoxine)
                .push_bind(&metric.dietary_vitamin_b12_cobalamin)
                .push_bind(&metric.dietary_folate)
                .push_bind(&metric.dietary_biotin)
                .push_bind(&metric.dietary_pantothenic_acid)
                // Essential Vitamins (Fat-soluble)
                .push_bind(&metric.dietary_vitamin_a)
                .push_bind(&metric.dietary_vitamin_d)
                .push_bind(&metric.dietary_vitamin_e)
                .push_bind(&metric.dietary_vitamin_k)
                // Meal Context for atomic processing
                .push_bind(&metric.meal_type)
                .push_bind(&metric.meal_id)
                // Metadata and source tracking
                .push_bind(&metric.source_device)
                .push_bind(&metric.created_at);
        });

        // Add ON CONFLICT handling for deduplication
        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO UPDATE SET
            dietary_water = COALESCE(EXCLUDED.dietary_water, nutrition_metrics.dietary_water),
            dietary_caffeine = COALESCE(EXCLUDED.dietary_caffeine, nutrition_metrics.dietary_caffeine),
            dietary_energy_consumed = COALESCE(EXCLUDED.dietary_energy_consumed, nutrition_metrics.dietary_energy_consumed),
            dietary_carbohydrates = COALESCE(EXCLUDED.dietary_carbohydrates, nutrition_metrics.dietary_carbohydrates),
            dietary_protein = COALESCE(EXCLUDED.dietary_protein, nutrition_metrics.dietary_protein),
            dietary_fat_total = COALESCE(EXCLUDED.dietary_fat_total, nutrition_metrics.dietary_fat_total),
            dietary_fat_saturated = COALESCE(EXCLUDED.dietary_fat_saturated, nutrition_metrics.dietary_fat_saturated),
            dietary_fat_monounsaturated = COALESCE(EXCLUDED.dietary_fat_monounsaturated, nutrition_metrics.dietary_fat_monounsaturated),
            dietary_fat_polyunsaturated = COALESCE(EXCLUDED.dietary_fat_polyunsaturated, nutrition_metrics.dietary_fat_polyunsaturated),
            dietary_cholesterol = COALESCE(EXCLUDED.dietary_cholesterol, nutrition_metrics.dietary_cholesterol),
            dietary_sodium = COALESCE(EXCLUDED.dietary_sodium, nutrition_metrics.dietary_sodium),
            dietary_fiber = COALESCE(EXCLUDED.dietary_fiber, nutrition_metrics.dietary_fiber),
            dietary_sugar = COALESCE(EXCLUDED.dietary_sugar, nutrition_metrics.dietary_sugar),
            dietary_calcium = COALESCE(EXCLUDED.dietary_calcium, nutrition_metrics.dietary_calcium),
            dietary_iron = COALESCE(EXCLUDED.dietary_iron, nutrition_metrics.dietary_iron),
            dietary_magnesium = COALESCE(EXCLUDED.dietary_magnesium, nutrition_metrics.dietary_magnesium),
            dietary_potassium = COALESCE(EXCLUDED.dietary_potassium, nutrition_metrics.dietary_potassium),
            dietary_zinc = COALESCE(EXCLUDED.dietary_zinc, nutrition_metrics.dietary_zinc),
            dietary_phosphorus = COALESCE(EXCLUDED.dietary_phosphorus, nutrition_metrics.dietary_phosphorus),
            dietary_vitamin_c = COALESCE(EXCLUDED.dietary_vitamin_c, nutrition_metrics.dietary_vitamin_c),
            dietary_vitamin_b1_thiamine = COALESCE(EXCLUDED.dietary_vitamin_b1_thiamine, nutrition_metrics.dietary_vitamin_b1_thiamine),
            dietary_vitamin_b2_riboflavin = COALESCE(EXCLUDED.dietary_vitamin_b2_riboflavin, nutrition_metrics.dietary_vitamin_b2_riboflavin),
            dietary_vitamin_b3_niacin = COALESCE(EXCLUDED.dietary_vitamin_b3_niacin, nutrition_metrics.dietary_vitamin_b3_niacin),
            dietary_vitamin_b6_pyridoxine = COALESCE(EXCLUDED.dietary_vitamin_b6_pyridoxine, nutrition_metrics.dietary_vitamin_b6_pyridoxine),
            dietary_vitamin_b12_cobalamin = COALESCE(EXCLUDED.dietary_vitamin_b12_cobalamin, nutrition_metrics.dietary_vitamin_b12_cobalamin),
            dietary_folate = COALESCE(EXCLUDED.dietary_folate, nutrition_metrics.dietary_folate),
            dietary_biotin = COALESCE(EXCLUDED.dietary_biotin, nutrition_metrics.dietary_biotin),
            dietary_pantothenic_acid = COALESCE(EXCLUDED.dietary_pantothenic_acid, nutrition_metrics.dietary_pantothenic_acid),
            dietary_vitamin_a = COALESCE(EXCLUDED.dietary_vitamin_a, nutrition_metrics.dietary_vitamin_a),
            dietary_vitamin_d = COALESCE(EXCLUDED.dietary_vitamin_d, nutrition_metrics.dietary_vitamin_d),
            dietary_vitamin_e = COALESCE(EXCLUDED.dietary_vitamin_e, nutrition_metrics.dietary_vitamin_e),
            dietary_vitamin_k = COALESCE(EXCLUDED.dietary_vitamin_k, nutrition_metrics.dietary_vitamin_k),
            meal_type = COALESCE(EXCLUDED.meal_type, nutrition_metrics.meal_type),
            meal_id = COALESCE(EXCLUDED.meal_id, nutrition_metrics.meal_id),
            source_device = COALESCE(EXCLUDED.source_device, nutrition_metrics.source_device)
        ");

        let result = query_builder.build().execute(pool).await?;
        total_processed += result.rows_affected() as usize;
    }

    Ok(total_processed)
}

// Helper functions for nutrition analysis

fn generate_nutrition_analysis(metrics: &[NutritionMetric]) -> Option<NutritionAnalysis> {
    if metrics.is_empty() {
        return None;
    }

    let total_entries = metrics.len();

    // Calculate hydration status
    let total_water: f64 = metrics.iter().filter_map(|m| m.dietary_water).sum();
    let total_caffeine: f64 = metrics.iter().filter_map(|m| m.dietary_caffeine).sum();
    let daily_average_water = total_water / (total_entries as f64);

    let hydration_status = HydrationStatus {
        total_water_liters: total_water,
        daily_average_liters: daily_average_water,
        hydration_level: get_hydration_status_from_value(Some(daily_average_water)),
        total_caffeine_mg: total_caffeine,
        exceeds_caffeine_limit: total_caffeine > 400.0,
    };

    // Calculate macronutrient analysis
    let total_calories: f64 = metrics
        .iter()
        .filter_map(|m| m.dietary_energy_consumed)
        .sum();
    let total_protein: f64 = metrics.iter().filter_map(|m| m.dietary_protein).sum();
    let total_carbs: f64 = metrics.iter().filter_map(|m| m.dietary_carbohydrates).sum();
    let total_fat: f64 = metrics.iter().filter_map(|m| m.dietary_fat_total).sum();

    let calorie_values: Vec<f64> = metrics
        .iter()
        .filter_map(|m| m.dietary_energy_consumed)
        .collect();
    let calorie_range = if !calorie_values.is_empty() {
        NutrientRange {
            min: calorie_values.iter().cloned().fold(f64::INFINITY, f64::min),
            max: calorie_values
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max),
            average: total_calories / calorie_values.len() as f64,
        }
    } else {
        NutrientRange {
            min: 0.0,
            max: 0.0,
            average: 0.0,
        }
    };

    let macronutrient_analysis = MacronutrientAnalysis {
        total_calories,
        average_daily_calories: total_calories / total_entries as f64,
        total_protein_grams: total_protein,
        total_carbohydrates_grams: total_carbs,
        total_fat_grams: total_fat,
        average_distribution: calculate_average_macronutrient_distribution(metrics),
        calorie_range,
    };

    // Calculate micronutrient analysis
    let vitamin_totals = VitaminTotals {
        vitamin_a_mcg: metrics.iter().filter_map(|m| m.dietary_vitamin_a).sum(),
        vitamin_c_mg: metrics.iter().filter_map(|m| m.dietary_vitamin_c).sum(),
        vitamin_d_iu: metrics.iter().filter_map(|m| m.dietary_vitamin_d).sum(),
    };

    let mineral_totals = MineralTotals {
        calcium_mg: metrics.iter().filter_map(|m| m.dietary_calcium).sum(),
        iron_mg: metrics.iter().filter_map(|m| m.dietary_iron).sum(),
        magnesium_mg: metrics.iter().filter_map(|m| m.dietary_magnesium).sum(),
        potassium_mg: metrics.iter().filter_map(|m| m.dietary_potassium).sum(),
        sodium_mg: metrics.iter().filter_map(|m| m.dietary_sodium).sum(),
    };

    let micronutrient_analysis = MicronutrientAnalysis {
        vitamin_totals,
        mineral_totals,
        deficiency_risks: identify_deficiency_risks(metrics),
        excess_warnings: identify_excess_warnings(metrics),
    };

    // Identify dietary concerns
    let dietary_concerns = identify_dietary_concerns(metrics);

    // Count special conditions
    let balanced_meals_count = metrics.iter().filter(|m| m.is_balanced_meal()).count();
    let excessive_sodium_alerts = metrics.iter().filter(|m| m.has_excessive_sodium()).count();
    let caffeine_warnings = metrics
        .iter()
        .filter(|m| m.exceeds_caffeine_limit())
        .count();

    Some(NutritionAnalysis {
        total_entries,
        hydration_status,
        macronutrient_analysis,
        micronutrient_analysis,
        dietary_concerns,
        balanced_meals_count,
        excessive_sodium_alerts,
        caffeine_warnings,
    })
}

fn generate_daily_nutrition_summary(metrics: &[NutritionMetric]) -> Option<DailyNutritionSummary> {
    if metrics.is_empty() {
        return None;
    }

    // For simplicity, calculate summary for the most recent day
    let latest_date = metrics.iter().map(|m| m.recorded_at).max()?;
    let day_start = latest_date.date_naive().and_hms_opt(0, 0, 0)?.and_utc();
    let day_end = latest_date.date_naive().and_hms_opt(23, 59, 59)?.and_utc();

    let day_metrics: Vec<&NutritionMetric> = metrics
        .iter()
        .filter(|m| m.recorded_at >= day_start && m.recorded_at <= day_end)
        .collect();

    if day_metrics.is_empty() {
        return None;
    }

    let total_calories: f64 = day_metrics
        .iter()
        .filter_map(|m| m.dietary_energy_consumed)
        .sum();
    let water_intake: f64 = day_metrics.iter().filter_map(|m| m.dietary_water).sum();
    let protein: f64 = day_metrics.iter().filter_map(|m| m.dietary_protein).sum();
    let carbs: f64 = day_metrics
        .iter()
        .filter_map(|m| m.dietary_carbohydrates)
        .sum();
    let fat: f64 = day_metrics.iter().filter_map(|m| m.dietary_fat_total).sum();
    let fiber: f64 = day_metrics.iter().filter_map(|m| m.dietary_fiber).sum();
    let sodium: f64 = day_metrics.iter().filter_map(|m| m.dietary_sodium).sum();

    let macronutrient_distribution = if total_calories > 0.0 {
        Some(MacronutrientDistribution {
            carbohydrate_percent: ((carbs * 4.0) / total_calories * 100.0) as u8,
            protein_percent: ((protein * 4.0) / total_calories * 100.0) as u8,
            fat_percent: ((fat * 9.0) / total_calories * 100.0) as u8,
        })
    } else {
        None
    };

    Some(DailyNutritionSummary {
        date: latest_date,
        total_calories,
        water_intake_liters: water_intake,
        protein_grams: protein,
        carbohydrates_grams: carbs,
        fat_grams: fat,
        fiber_grams: fiber,
        sodium_mg: sodium,
        macronutrient_distribution,
        meal_count: day_metrics.len(),
    })
}

fn generate_nutrition_summary(metrics: &[NutritionMetric]) -> NutritionSummary {
    let total_entries = metrics.len();

    let date_range = if !metrics.is_empty() {
        DateRange {
            start: metrics.iter().map(|m| m.recorded_at).min().unwrap(),
            end: metrics.iter().map(|m| m.recorded_at).max().unwrap(),
        }
    } else {
        DateRange {
            start: Utc::now(),
            end: Utc::now(),
        }
    };

    let total_calories: f64 = metrics
        .iter()
        .filter_map(|m| m.dietary_energy_consumed)
        .sum();
    let total_water: f64 = metrics.iter().filter_map(|m| m.dietary_water).sum();

    let days_span = (date_range.end - date_range.start).num_days().max(1) as f64;

    let top_nutrients = vec![
        TopNutrient {
            nutrient_name: "Calories".to_string(),
            total_amount: total_calories,
            unit: "kcal".to_string(),
            daily_average: total_calories / days_span,
        },
        TopNutrient {
            nutrient_name: "Water".to_string(),
            total_amount: total_water,
            unit: "L".to_string(),
            daily_average: total_water / days_span,
        },
    ];

    let dietary_patterns = identify_dietary_patterns(metrics);

    NutritionSummary {
        total_entries,
        date_range,
        average_daily_calories: total_calories / days_span,
        average_daily_water_liters: total_water / days_span,
        top_nutrients,
        dietary_patterns,
    }
}

fn generate_hydration_summary(entries: &[HydrationEntry]) -> HydrationSummary {
    let total_water: f64 = entries.iter().filter_map(|e| e.water_intake_liters).sum();
    let total_caffeine: f64 = entries.iter().filter_map(|e| e.caffeine_mg).sum();

    let hydration_days_count = entries.len();
    let daily_average = if hydration_days_count > 0 {
        total_water / hydration_days_count as f64
    } else {
        0.0
    };

    let well_hydrated_days = entries
        .iter()
        .filter(|e| e.hydration_status == "well_hydrated")
        .count();
    let dehydrated_days = entries
        .iter()
        .filter(|e| e.hydration_status == "dehydrated")
        .count();
    let overhydrated_days = entries
        .iter()
        .filter(|e| e.hydration_status == "overhydrated")
        .count();

    HydrationSummary {
        total_water_liters: total_water,
        daily_average_liters: daily_average,
        total_caffeine_mg: total_caffeine,
        hydration_days_count,
        well_hydrated_days,
        dehydrated_days,
        overhydrated_days,
    }
}

fn generate_daily_aggregations(metrics: &[NutritionMetric]) -> Vec<DailyNutritionSummary> {
    use std::collections::HashMap;

    // Group metrics by date
    let mut daily_metrics: HashMap<String, Vec<&NutritionMetric>> = HashMap::new();

    for metric in metrics {
        let date_key = metric.recorded_at.date_naive().to_string();
        daily_metrics
            .entry(date_key)
            .or_insert_with(Vec::new)
            .push(metric);
    }

    // Generate daily summaries
    daily_metrics
        .iter()
        .map(|(date_str, day_metrics)| {
            let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_utc();

            let total_calories: f64 = day_metrics
                .iter()
                .filter_map(|m| m.dietary_energy_consumed)
                .sum();
            let water_intake: f64 = day_metrics.iter().filter_map(|m| m.dietary_water).sum();
            let protein: f64 = day_metrics.iter().filter_map(|m| m.dietary_protein).sum();
            let carbs: f64 = day_metrics
                .iter()
                .filter_map(|m| m.dietary_carbohydrates)
                .sum();
            let fat: f64 = day_metrics.iter().filter_map(|m| m.dietary_fat_total).sum();
            let fiber: f64 = day_metrics.iter().filter_map(|m| m.dietary_fiber).sum();
            let sodium: f64 = day_metrics.iter().filter_map(|m| m.dietary_sodium).sum();

            let macronutrient_distribution = if total_calories > 0.0 {
                Some(MacronutrientDistribution {
                    carbohydrate_percent: ((carbs * 4.0) / total_calories * 100.0) as u8,
                    protein_percent: ((protein * 4.0) / total_calories * 100.0) as u8,
                    fat_percent: ((fat * 9.0) / total_calories * 100.0) as u8,
                })
            } else {
                None
            };

            DailyNutritionSummary {
                date,
                total_calories,
                water_intake_liters: water_intake,
                protein_grams: protein,
                carbohydrates_grams: carbs,
                fat_grams: fat,
                fiber_grams: fiber,
                sodium_mg: sodium,
                macronutrient_distribution,
                meal_count: day_metrics.len(),
            }
        })
        .collect()
}

// Helper functions for analysis

fn get_hydration_status_from_value(water: Option<f64>) -> String {
    match water {
        Some(w) if w < 1.0 => "severely_dehydrated".to_string(),
        Some(w) if w < 2.0 => "dehydrated".to_string(),
        Some(w) if w < 3.0 => "adequate".to_string(),
        Some(w) if w > 5.0 => "overhydrated".to_string(),
        Some(_) => "well_hydrated".to_string(),
        None => "unknown".to_string(),
    }
}

fn calculate_average_macronutrient_distribution(
    metrics: &[NutritionMetric],
) -> Option<MacronutrientDistribution> {
    let distributions: Vec<MacronutrientDistribution> = metrics
        .iter()
        .filter_map(|m| m.macronutrient_distribution())
        .collect();

    if distributions.is_empty() {
        return None;
    }

    let avg_carb = distributions
        .iter()
        .map(|d| d.carbohydrate_percent as u16)
        .sum::<u16>()
        / distributions.len() as u16;
    let avg_protein = distributions
        .iter()
        .map(|d| d.protein_percent as u16)
        .sum::<u16>()
        / distributions.len() as u16;
    let avg_fat = distributions
        .iter()
        .map(|d| d.fat_percent as u16)
        .sum::<u16>()
        / distributions.len() as u16;

    Some(MacronutrientDistribution {
        carbohydrate_percent: avg_carb as u8,
        protein_percent: avg_protein as u8,
        fat_percent: avg_fat as u8,
    })
}

fn identify_deficiency_risks(metrics: &[NutritionMetric]) -> Vec<String> {
    let mut risks = Vec::new();

    let total_vitamin_c: f64 = metrics.iter().filter_map(|m| m.dietary_vitamin_c).sum();
    if total_vitamin_c < 90.0 * metrics.len() as f64 {
        // 90mg RDA * number of entries
        risks.push("Vitamin C deficiency risk".to_string());
    }

    let total_iron: f64 = metrics.iter().filter_map(|m| m.dietary_iron).sum();
    if total_iron < 18.0 * metrics.len() as f64 {
        // 18mg RDA * number of entries
        risks.push("Iron deficiency risk".to_string());
    }

    risks
}

fn identify_excess_warnings(metrics: &[NutritionMetric]) -> Vec<String> {
    let mut warnings = Vec::new();

    let avg_sodium: f64 =
        metrics.iter().filter_map(|m| m.dietary_sodium).sum::<f64>() / metrics.len() as f64;
    if avg_sodium > 2300.0 {
        warnings.push("Excessive sodium intake".to_string());
    }

    warnings
}

fn identify_dietary_concerns(metrics: &[NutritionMetric]) -> Vec<DietaryConcern> {
    let mut concerns = Vec::new();

    // Check for excessive sodium
    let high_sodium_count = metrics.iter().filter(|m| m.has_excessive_sodium()).count();
    if high_sodium_count > metrics.len() / 2 {
        concerns.push(DietaryConcern {
            concern_type: "excessive_sodium".to_string(),
            severity: "high".to_string(),
            description: "More than half of your meals contain excessive sodium".to_string(),
            recommendations: vec![
                "Reduce processed food consumption".to_string(),
                "Use herbs and spices instead of salt".to_string(),
                "Check nutrition labels for sodium content".to_string(),
            ],
        });
    }

    // Check for low fiber
    let avg_fiber: f64 =
        metrics.iter().filter_map(|m| m.dietary_fiber).sum::<f64>() / metrics.len() as f64;
    if avg_fiber < 25.0 {
        // 25g daily recommendation
        concerns.push(DietaryConcern {
            concern_type: "low_fiber".to_string(),
            severity: "medium".to_string(),
            description: "Your average fiber intake is below recommended levels".to_string(),
            recommendations: vec![
                "Include more fruits and vegetables".to_string(),
                "Choose whole grain options".to_string(),
                "Add beans and legumes to meals".to_string(),
            ],
        });
    }

    concerns
}

fn identify_dietary_patterns(metrics: &[NutritionMetric]) -> Vec<DietaryPattern> {
    let mut patterns = Vec::new();

    // High protein pattern
    let high_protein_meals = metrics
        .iter()
        .filter(|m| {
            if let Some(dist) = m.macronutrient_distribution() {
                dist.protein_percent > 25
            } else {
                false
            }
        })
        .count();

    if high_protein_meals > metrics.len() / 3 {
        patterns.push(DietaryPattern {
            pattern_name: "High Protein".to_string(),
            description: "Consistently high protein intake across meals".to_string(),
            frequency: high_protein_meals,
            health_impact: "positive".to_string(),
        });
    }

    // Low carb pattern
    let low_carb_meals = metrics
        .iter()
        .filter(|m| {
            if let Some(dist) = m.macronutrient_distribution() {
                dist.carbohydrate_percent < 30
            } else {
                false
            }
        })
        .count();

    if low_carb_meals > metrics.len() / 2 {
        patterns.push(DietaryPattern {
            pattern_name: "Low Carbohydrate".to_string(),
            description: "Consistently low carbohydrate intake".to_string(),
            frequency: low_carb_meals,
            health_impact: "neutral".to_string(),
        });
    }

    patterns
}
