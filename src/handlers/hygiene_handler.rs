use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::ValidationConfig;
use crate::middleware::metrics::Metrics;
use crate::models::enums::HygieneEventType;
use crate::models::health_metrics::HygieneMetric;
use crate::services::auth::AuthContext;
use crate::services::batch_processor::BatchProcessor;

/// Hygiene events ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct HygieneIngestPayload {
    pub hygiene_events: Vec<HygieneIngestRequest>,
}

/// Individual hygiene event for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct HygieneIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub event_type: String, // Will be converted to HygieneEventType
    pub duration_seconds: Option<i32>,
    pub quality_rating: Option<i16>, // 1-5 self-reported quality

    // Public Health & Compliance Tracking
    pub meets_who_guidelines: Option<bool>,
    pub frequency_compliance_rating: Option<i16>, // 1-5 daily frequency adherence

    // Smart Device Integration
    pub device_detected: Option<bool>,
    pub device_effectiveness_score: Option<f64>, // 0-100% device-measured effectiveness

    // Context & Behavioral Analysis
    pub trigger_event: Option<String>,
    pub location_context: Option<String>,
    pub compliance_motivation: Option<String>,

    // Health Crisis Integration
    pub health_crisis_enhanced: Option<bool>,
    pub crisis_compliance_level: Option<i16>, // 1-5 adherence to crisis protocols

    // Gamification & Habit Tracking
    pub daily_goal_progress: Option<i16>, // 0-200% of daily hygiene goals met
    pub achievement_unlocked: Option<String>,

    // Medical Integration
    pub medication_adherence_related: Option<bool>,
    pub medical_condition_context: Option<String>,

    // Privacy & Data Sensitivity
    pub data_sensitivity_level: Option<String>,

    // Metadata
    pub source_device: Option<String>,
}

/// Hygiene data query parameters
#[derive(Debug, Deserialize)]
pub struct HygieneQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub event_type: Option<String>, // handwashing, toothbrushing, etc.
    pub category: Option<String>,   // hand_hygiene, oral_hygiene, etc.
    pub crisis_period: Option<bool>, // Filter for health crisis periods
    pub compliance_only: Option<bool>, // Only events meeting WHO guidelines
}

/// Hygiene ingestion response
#[derive(Debug, Serialize)]
pub struct HygieneIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<HygieneProcessingError>,
    pub hygiene_analysis: Option<HygieneAnalysis>,
}

/// Hygiene processing error details
#[derive(Debug, Serialize)]
pub struct HygieneProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub event_type: Option<String>,
    pub duration: Option<i32>,
}

/// Hygiene analysis summary
#[derive(Debug, Serialize)]
pub struct HygieneAnalysis {
    pub compliance_score: f64, // Overall compliance score (0-100%)
    pub handwashing_compliance: f64, // WHO handwashing guideline compliance
    pub toothbrushing_compliance: f64, // ADA toothbrushing guideline compliance
    pub critical_hygiene_events: usize, // Infection prevention critical events
    pub smart_device_detections: usize, // Events detected by smart devices
    pub health_crisis_events: usize, // Events during health crisis periods
    pub habit_strength_summary: HabitStrengthSummary,
    pub public_health_insights: PublicHealthInsights,
}

/// Habit strength analysis summary
#[derive(Debug, Serialize)]
pub struct HabitStrengthSummary {
    pub average_streak_length: f64,
    pub strongest_habit: Option<String>, // Event type with longest streak
    pub developing_habits: Vec<String>, // Habits in development stage
    pub total_achievements: usize,
}

/// Public health insights for hygiene tracking
#[derive(Debug, Serialize)]
pub struct PublicHealthInsights {
    pub infection_prevention_score: f64, // Critical hygiene compliance
    pub crisis_response_effectiveness: f64, // Compliance during health crises
    pub recommended_improvements: Vec<String>, // Actionable health recommendations
    pub risk_level: String, // low, moderate, high infection risk
}

/// Hygiene data retrieval response
#[derive(Debug, Serialize)]
pub struct HygieneDataResponse {
    pub hygiene_events: Vec<HygieneEventSummary>,
    pub total_count: i64,
    pub date_range: DateRange,
    pub compliance_summary: ComplianceSummary,
    pub analytics: HygieneAnalytics,
}

/// Hygiene event summary for API responses
#[derive(Debug, Serialize)]
pub struct HygieneEventSummary {
    pub id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub event_type: String,
    pub duration_seconds: Option<i32>,
    pub quality_rating: Option<i16>,
    pub meets_who_guidelines: Option<bool>,
    pub compliance_score: f64,
    pub category: String,
    pub streak_count: Option<i32>,
    pub habit_strength: String,
    pub device_detected: Option<bool>,
    pub health_crisis_enhanced: Option<bool>,
    pub achievement_unlocked: Option<String>,
}

/// Date range summary
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub days: i64,
}

/// Compliance summary for hygiene tracking
#[derive(Debug, Serialize)]
pub struct ComplianceSummary {
    pub overall_score: f64,
    pub handwashing_events: usize,
    pub handwashing_compliance: f64,
    pub toothbrushing_events: usize,
    pub toothbrushing_compliance: f64,
    pub critical_events_total: usize,
    pub who_guideline_adherence: f64,
}

/// Hygiene analytics for behavioral insights
#[derive(Debug, Serialize)]
pub struct HygieneAnalytics {
    pub daily_frequency: f64,
    pub category_breakdown: std::collections::HashMap<String, usize>,
    pub trend_analysis: TrendAnalysis,
    pub public_health_score: f64,
    pub smart_device_adoption: f64,
}

/// Trend analysis for hygiene behavior
#[derive(Debug, Serialize)]
pub struct TrendAnalysis {
    pub improving_habits: Vec<String>,
    pub declining_habits: Vec<String>,
    pub stable_habits: Vec<String>,
    pub streak_trends: std::collections::HashMap<String, i32>,
}

/// POST /api/v1/ingest/hygiene - Ingest hygiene events data
#[instrument(skip(pool, batch_processor, auth, payload))]
pub async fn ingest_hygiene(
    pool: web::Data<PgPool>,
    batch_processor: web::Data<BatchProcessor>,
    validation_config: web::Data<ValidationConfig>,
    auth: AuthContext,
    payload: web::Json<HygieneIngestPayload>,
) -> ActixResult<HttpResponse> {
    let start_time = std::time::Instant::now();
    let user_id = auth.user_id();

    info!(
        user_id = %user_id,
        event_count = payload.hygiene_events.len(),
        "Processing hygiene events ingestion request"
    );

    if payload.hygiene_events.is_empty() {
        warn!(user_id = %user_id, "Empty hygiene events payload received");
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "empty_payload",
            "message": "No hygiene events provided for ingestion"
        })));
    }

    let mut hygiene_metrics = Vec::new();
    let mut errors = Vec::new();
    let mut processed_count = 0;

    // Convert and validate each hygiene event
    for (index, event_request) in payload.hygiene_events.iter().enumerate() {
        match convert_hygiene_request_to_metric(user_id, event_request, &validation_config) {
            Ok(metric) => {
                hygiene_metrics.push(metric);
                processed_count += 1;
            }
            Err(error_msg) => {
                errors.push(HygieneProcessingError {
                    index,
                    error_type: "validation_error".to_string(),
                    message: error_msg,
                    event_type: Some(event_request.event_type.clone()),
                    duration: event_request.duration_seconds,
                });
                warn!(
                    user_id = %user_id,
                    index = index,
                    error = %error_msg,
                    "Hygiene event validation failed"
                );
            }
        }
    }

    // Batch process validated hygiene metrics
    let processing_result = if !hygiene_metrics.is_empty() {
        match batch_processor.process_hygiene_events(user_id, hygiene_metrics).await {
            Ok(result) => Some(result),
            Err(e) => {
                error!(
                    user_id = %user_id,
                    error = %e,
                    "Failed to batch process hygiene events"
                );

                errors.push(HygieneProcessingError {
                    index: 0,
                    error_type: "database_error".to_string(),
                    message: format!("Failed to store hygiene events: {}", e),
                    event_type: None,
                    duration: None,
                });
                None
            }
        }
    } else {
        None
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;
    let failed_count = payload.hygiene_events.len() - processed_count;

    // Generate hygiene analysis if events were processed successfully
    let hygiene_analysis = if processed_count > 0 {
        generate_hygiene_analysis(user_id, &payload.hygiene_events, &pool).await.ok()
    } else {
        None
    };

    let response = HygieneIngestResponse {
        success: processed_count > 0,
        processed_count,
        failed_count,
        processing_time_ms,
        errors,
        hygiene_analysis,
    };

    info!(
        user_id = %user_id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time_ms,
        "Completed hygiene events ingestion"
    );

    // Update metrics
    if let Some(metrics) = Metrics::try_get() {
        metrics.ingest_requests_total
            .with_label_values(&["hygiene", if response.success { "success" } else { "failure" }])
            .inc();

        metrics.metrics_processed_total
            .with_label_values(&["hygiene"])
            .inc_by(processed_count as u64);

        if failed_count > 0 {
            metrics.ingest_errors_total
                .with_label_values(&["hygiene", "validation_error"])
                .inc_by(failed_count as u64);
        }

        metrics.ingest_duration_seconds
            .with_label_values(&["hygiene"])
            .observe(processing_time_ms as f64 / 1000.0);
    }

    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/data/hygiene - Retrieve hygiene events data
#[instrument(skip(pool, auth))]
pub async fn get_hygiene_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<HygieneQueryParams>,
) -> ActixResult<HttpResponse> {
    let user_id = auth.user_id();

    info!(
        user_id = %user_id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        event_type = ?query.event_type,
        "Retrieving hygiene events data"
    );

    // Set default date range if not provided (last 30 days)
    let end_date = query.end_date.unwrap_or_else(Utc::now);
    let start_date = query.start_date.unwrap_or_else(|| {
        end_date - chrono::Duration::days(30)
    });

    let limit = query.limit.unwrap_or(1000).min(10000); // Max 10k records

    // Build query with optional filters
    let mut sql_query = r#"
        SELECT
            id, user_id, recorded_at, event_type, duration_seconds,
            quality_rating, meets_who_guidelines, frequency_compliance_rating,
            device_detected, device_effectiveness_score, trigger_event,
            location_context, compliance_motivation, health_crisis_enhanced,
            crisis_compliance_level, streak_count, daily_goal_progress,
            achievement_unlocked, medication_adherence_related,
            medical_condition_context, data_sensitivity_level,
            source_device, created_at
        FROM hygiene_events
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
    "#.to_string();

    let mut param_count = 3;

    // Add event type filter
    if let Some(event_type) = &query.event_type {
        param_count += 1;
        sql_query.push_str(&format!(" AND event_type = ${}", param_count));
    }

    // Add crisis period filter
    if let Some(crisis_only) = query.crisis_period {
        if crisis_only {
            sql_query.push_str(" AND health_crisis_enhanced = true");
        }
    }

    // Add compliance filter
    if let Some(compliance_only) = query.compliance_only {
        if compliance_only {
            sql_query.push_str(" AND meets_who_guidelines = true");
        }
    }

    sql_query.push_str(" ORDER BY recorded_at DESC");
    sql_query.push_str(&format!(" LIMIT {}", limit));

    // Execute query
    let hygiene_events = match query.event_type.as_ref() {
        Some(event_type) => {
            sqlx::query_as::<_, HygieneMetric>(&sql_query)
                .bind(user_id)
                .bind(start_date)
                .bind(end_date)
                .bind(event_type)
                .fetch_all(pool.as_ref())
                .await
        }
        None => {
            sqlx::query_as::<_, HygieneMetric>(&sql_query)
                .bind(user_id)
                .bind(start_date)
                .bind(end_date)
                .fetch_all(pool.as_ref())
                .await
        }
    };

    let hygiene_events = match hygiene_events {
        Ok(events) => events,
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to retrieve hygiene events data"
            );
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "database_error",
                "message": "Failed to retrieve hygiene events data"
            })));
        }
    };

    // Get total count for pagination
    let total_count = get_hygiene_events_count(
        user_id, start_date, end_date, &query, &pool
    ).await.unwrap_or(0);

    // Convert to summary format and generate analytics
    let hygiene_summaries: Vec<HygieneEventSummary> = hygiene_events
        .iter()
        .map(|event| HygieneEventSummary {
            id: event.id,
            recorded_at: event.recorded_at,
            event_type: event.event_type.to_string(),
            duration_seconds: event.duration_seconds,
            quality_rating: event.quality_rating,
            meets_who_guidelines: event.meets_who_guidelines,
            compliance_score: event.calculate_compliance_score(),
            category: event.get_hygiene_category().to_string(),
            streak_count: event.streak_count,
            habit_strength: event.habit_strength().to_string(),
            device_detected: event.device_detected,
            health_crisis_enhanced: event.health_crisis_enhanced,
            achievement_unlocked: event.achievement_unlocked.clone(),
        })
        .collect();

    // Generate compliance summary
    let compliance_summary = generate_compliance_summary(&hygiene_events);

    // Generate analytics
    let analytics = generate_hygiene_analytics(&hygiene_events, start_date, end_date);

    let response = HygieneDataResponse {
        hygiene_events: hygiene_summaries,
        total_count,
        date_range: DateRange {
            start: start_date,
            end: end_date,
            days: (end_date - start_date).num_days(),
        },
        compliance_summary,
        analytics,
    };

    info!(
        user_id = %user_id,
        events_returned = response.hygiene_events.len(),
        total_count = total_count,
        "Successfully retrieved hygiene events data"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Convert hygiene ingest request to validated metric
fn convert_hygiene_request_to_metric(
    user_id: Uuid,
    request: &HygieneIngestRequest,
    validation_config: &ValidationConfig,
) -> Result<HygieneMetric, String> {
    // Parse event type
    let event_type = HygieneEventType::from_ios_string(&request.event_type)
        .ok_or_else(|| format!("Invalid hygiene event type: {}", request.event_type))?;

    let metric = HygieneMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: request.recorded_at,
        event_type,
        duration_seconds: request.duration_seconds,
        quality_rating: request.quality_rating,
        meets_who_guidelines: request.meets_who_guidelines,
        frequency_compliance_rating: request.frequency_compliance_rating,
        device_detected: request.device_detected,
        device_effectiveness_score: request.device_effectiveness_score,
        trigger_event: request.trigger_event.clone(),
        location_context: request.location_context.clone(),
        compliance_motivation: request.compliance_motivation.clone(),
        health_crisis_enhanced: request.health_crisis_enhanced,
        crisis_compliance_level: request.crisis_compliance_level,
        streak_count: None, // Will be calculated by database trigger
        daily_goal_progress: request.daily_goal_progress,
        achievement_unlocked: request.achievement_unlocked.clone(),
        medication_adherence_related: request.medication_adherence_related,
        medical_condition_context: request.medical_condition_context.clone(),
        data_sensitivity_level: request.data_sensitivity_level.clone(),
        source_device: request.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric.validate_with_config(validation_config)?;

    Ok(metric)
}

/// Generate hygiene analysis summary
async fn generate_hygiene_analysis(
    user_id: Uuid,
    events: &[HygieneIngestRequest],
    pool: &PgPool,
) -> Result<HygieneAnalysis, sqlx::Error> {
    // Calculate compliance scores
    let handwashing_events: Vec<_> = events.iter()
        .filter(|e| e.event_type == "handwashing")
        .collect();

    let toothbrushing_events: Vec<_> = events.iter()
        .filter(|e| e.event_type == "toothbrushing")
        .collect();

    let handwashing_compliance = calculate_guideline_compliance(&handwashing_events, 20);
    let toothbrushing_compliance = calculate_guideline_compliance(&toothbrushing_events, 120);

    let overall_compliance = (handwashing_compliance + toothbrushing_compliance) / 2.0;

    // Count various event types
    let critical_events = events.iter()
        .filter(|e| matches!(e.event_type.as_str(), "handwashing" | "hand_sanitizer"))
        .count();

    let smart_device_detections = events.iter()
        .filter(|e| e.device_detected.unwrap_or(false))
        .count();

    let health_crisis_events = events.iter()
        .filter(|e| e.health_crisis_enhanced.unwrap_or(false))
        .count();

    // Generate habit strength summary from recent data
    let habit_summary = generate_habit_strength_summary(user_id, pool).await?;

    // Generate public health insights
    let public_health_insights = generate_public_health_insights(
        overall_compliance,
        critical_events,
        health_crisis_events,
        events.len(),
    );

    Ok(HygieneAnalysis {
        compliance_score: overall_compliance,
        handwashing_compliance,
        toothbrushing_compliance,
        critical_hygiene_events: critical_events,
        smart_device_detections,
        health_crisis_events,
        habit_strength_summary: habit_summary,
        public_health_insights,
    })
}

/// Calculate compliance percentage for guideline duration
fn calculate_guideline_compliance(events: &[&HygieneIngestRequest], min_duration: i32) -> f64 {
    if events.is_empty() {
        return 0.0;
    }

    let compliant_count = events.iter()
        .filter(|e| e.duration_seconds.unwrap_or(0) >= min_duration)
        .count();

    (compliant_count as f64 / events.len() as f64) * 100.0
}

/// Generate habit strength summary from database
async fn generate_habit_strength_summary(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<HabitStrengthSummary, sqlx::Error> {
    // Get recent streak data
    let streak_data = sqlx::query!(
        r#"
        SELECT
            event_type,
            MAX(streak_count) as max_streak,
            AVG(streak_count::DECIMAL) as avg_streak,
            COUNT(*) as total_events,
            COUNT(achievement_unlocked) as achievements
        FROM hygiene_events
        WHERE user_id = $1
            AND recorded_at >= NOW() - INTERVAL '30 days'
        GROUP BY event_type
        ORDER BY max_streak DESC NULLS LAST
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let average_streak_length = streak_data.iter()
        .filter_map(|row| row.avg_streak.map(|s| s.to_string().parse::<f64>().unwrap_or(0.0)))
        .fold(0.0, |acc, x| acc + x) / streak_data.len().max(1) as f64;

    let strongest_habit = streak_data.first()
        .and_then(|row| row.event_type.as_ref())
        .map(|s| s.to_string());

    let developing_habits: Vec<String> = streak_data.iter()
        .filter(|row| {
            row.max_streak.unwrap_or(0) >= 3 && row.max_streak.unwrap_or(0) <= 20
        })
        .filter_map(|row| row.event_type.as_ref())
        .map(|s| s.to_string())
        .collect();

    let total_achievements: usize = streak_data.iter()
        .map(|row| row.achievements.unwrap_or(0) as usize)
        .sum();

    Ok(HabitStrengthSummary {
        average_streak_length,
        strongest_habit,
        developing_habits,
        total_achievements,
    })
}

/// Generate public health insights
fn generate_public_health_insights(
    compliance_score: f64,
    critical_events: usize,
    crisis_events: usize,
    total_events: usize,
) -> PublicHealthInsights {
    let infection_prevention_score = if critical_events > 0 {
        (critical_events as f64 / total_events as f64) * compliance_score
    } else {
        0.0
    };

    let crisis_response_effectiveness = if crisis_events > 0 {
        (crisis_events as f64 / total_events as f64) * 100.0
    } else {
        50.0 // Neutral if no crisis data
    };

    let mut recommendations = Vec::new();

    if compliance_score < 70.0 {
        recommendations.push("Increase hygiene event duration to meet health guidelines".to_string());
    }

    if critical_events < (total_events / 3) {
        recommendations.push("Focus on critical infection prevention hygiene (handwashing, sanitizing)".to_string());
    }

    if crisis_response_effectiveness < 75.0 && crisis_events > 0 {
        recommendations.push("Enhance hygiene vigilance during health crisis periods".to_string());
    }

    let risk_level = match infection_prevention_score {
        score if score >= 80.0 => "low",
        score if score >= 60.0 => "moderate",
        _ => "high",
    }.to_string();

    PublicHealthInsights {
        infection_prevention_score,
        crisis_response_effectiveness,
        recommended_improvements: recommendations,
        risk_level,
    }
}

/// Get total count of hygiene events for pagination
async fn get_hygiene_events_count(
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    query: &HygieneQueryParams,
    pool: &PgPool,
) -> Result<i64, sqlx::Error> {
    let mut count_query = r#"
        SELECT COUNT(*) as total
        FROM hygiene_events
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
    "#.to_string();

    if query.event_type.is_some() {
        count_query.push_str(" AND event_type = $4");
    }

    if let Some(crisis_only) = query.crisis_period {
        if crisis_only {
            count_query.push_str(" AND health_crisis_enhanced = true");
        }
    }

    if let Some(compliance_only) = query.compliance_only {
        if compliance_only {
            count_query.push_str(" AND meets_who_guidelines = true");
        }
    }

    let result = match &query.event_type {
        Some(event_type) => {
            sqlx::query_scalar(&count_query)
                .bind(user_id)
                .bind(start_date)
                .bind(end_date)
                .bind(event_type)
                .fetch_one(pool)
                .await?
        }
        None => {
            sqlx::query_scalar(&count_query)
                .bind(user_id)
                .bind(start_date)
                .bind(end_date)
                .fetch_one(pool)
                .await?
        }
    };

    Ok(result)
}

/// Generate compliance summary
fn generate_compliance_summary(events: &[HygieneMetric]) -> ComplianceSummary {
    let handwashing_events: Vec<_> = events.iter()
        .filter(|e| matches!(e.event_type, HygieneEventType::Handwashing))
        .collect();

    let toothbrushing_events: Vec<_> = events.iter()
        .filter(|e| matches!(e.event_type, HygieneEventType::Toothbrushing))
        .collect();

    let handwashing_compliance = if !handwashing_events.is_empty() {
        handwashing_events.iter()
            .filter(|e| e.meets_who_guidelines.unwrap_or(false))
            .count() as f64 / handwashing_events.len() as f64 * 100.0
    } else {
        0.0
    };

    let toothbrushing_compliance = if !toothbrushing_events.is_empty() {
        toothbrushing_events.iter()
            .filter(|e| e.meets_who_guidelines.unwrap_or(false))
            .count() as f64 / toothbrushing_events.len() as f64 * 100.0
    } else {
        0.0
    };

    let critical_events_total = events.iter()
        .filter(|e| e.is_critical_for_infection_prevention())
        .count();

    let who_guideline_adherence = if !events.is_empty() {
        events.iter()
            .filter(|e| e.meets_who_guidelines.unwrap_or(false))
            .count() as f64 / events.len() as f64 * 100.0
    } else {
        0.0
    };

    let overall_score = if !handwashing_events.is_empty() && !toothbrushing_events.is_empty() {
        (handwashing_compliance + toothbrushing_compliance) / 2.0
    } else if !handwashing_events.is_empty() {
        handwashing_compliance
    } else if !toothbrushing_events.is_empty() {
        toothbrushing_compliance
    } else {
        0.0
    };

    ComplianceSummary {
        overall_score,
        handwashing_events: handwashing_events.len(),
        handwashing_compliance,
        toothbrushing_events: toothbrushing_events.len(),
        toothbrushing_compliance,
        critical_events_total,
        who_guideline_adherence,
    }
}

/// Generate hygiene analytics
fn generate_hygiene_analytics(
    events: &[HygieneMetric],
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> HygieneAnalytics {
    let days = (end_date - start_date).num_days().max(1) as f64;
    let daily_frequency = events.len() as f64 / days;

    // Category breakdown
    let mut category_breakdown = std::collections::HashMap::new();
    for event in events {
        let category = event.get_hygiene_category();
        *category_breakdown.entry(category.to_string()).or_insert(0) += 1;
    }

    // Public health score based on critical events and compliance
    let critical_events = events.iter()
        .filter(|e| e.is_critical_for_infection_prevention())
        .count();

    let compliant_events = events.iter()
        .filter(|e| e.meets_who_guidelines.unwrap_or(false))
        .count();

    let public_health_score = if !events.is_empty() {
        ((critical_events as f64 * 0.6) + (compliant_events as f64 * 0.4)) / events.len() as f64 * 100.0
    } else {
        0.0
    };

    // Smart device adoption
    let device_events = events.iter()
        .filter(|e| e.device_detected.unwrap_or(false))
        .count();

    let smart_device_adoption = if !events.is_empty() {
        device_events as f64 / events.len() as f64 * 100.0
    } else {
        0.0
    };

    // Generate trend analysis (simplified for this implementation)
    let trend_analysis = TrendAnalysis {
        improving_habits: vec![], // Would require historical comparison
        declining_habits: vec![],
        stable_habits: vec![],
        streak_trends: std::collections::HashMap::new(),
    };

    HygieneAnalytics {
        daily_frequency,
        category_breakdown,
        trend_analysis,
        public_health_score,
        smart_device_adoption,
    }
}