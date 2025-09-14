use actix_web::{web, HttpResponse, Result as ActixResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::config::ValidationConfig;
use crate::middleware::metrics::Metrics;
use crate::models::health_metrics::{SymptomMetric, SymptomAnalysis};
use crate::models::enums::{SymptomType, SymptomSeverity};
use crate::services::auth::AuthContext;

/// Symptom data ingestion payload
#[derive(Debug, Deserialize, Serialize)]
pub struct SymptomIngestPayload {
    pub symptoms: Vec<SymptomIngestRequest>,
}

/// Individual symptom metric for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct SymptomIngestRequest {
    pub recorded_at: DateTime<Utc>,
    pub symptom_type: SymptomType,
    pub severity: SymptomSeverity,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
    pub episode_id: Option<Uuid>, // Link related symptoms in same illness episode
    pub source_device: Option<String>,
}

/// Symptom data query parameters
#[derive(Debug, Deserialize)]
pub struct SymptomQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub symptom_type: Option<SymptomType>,
    pub severity: Option<SymptomSeverity>,
    pub category: Option<String>, // pain, respiratory, digestive, etc.
    pub episode_id: Option<Uuid>,
    pub emergency_only: Option<bool>, // Only show emergency-level symptoms
}

/// Symptom ingestion response
#[derive(Debug, Serialize)]
pub struct SymptomIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<SymptomProcessingError>,
    pub emergency_alerts: Vec<EmergencyAlert>,
    pub symptom_analysis: Option<SymptomBatchAnalysis>,
}

/// Symptom processing error details
#[derive(Debug, Serialize)]
pub struct SymptomProcessingError {
    pub index: usize,
    pub error_type: String,
    pub message: String,
    pub symptom_type: Option<SymptomType>,
    pub severity: Option<SymptomSeverity>,
}

/// Emergency alert for critical symptoms
#[derive(Debug, Serialize)]
pub struct EmergencyAlert {
    pub symptom_type: SymptomType,
    pub severity: SymptomSeverity,
    pub urgency_level: u8,
    pub message: String,
    pub recommendations: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

/// Batch symptom analysis
#[derive(Debug, Serialize)]
pub struct SymptomBatchAnalysis {
    pub total_symptoms: usize,
    pub emergency_count: usize,
    pub categories: std::collections::HashMap<String, usize>,
    pub severity_distribution: SeverityDistribution,
    pub episode_count: usize,
    pub average_severity_score: f64,
}

/// Severity distribution statistics
#[derive(Debug, Serialize)]
pub struct SeverityDistribution {
    pub none: usize,
    pub mild: usize,
    pub moderate: usize,
    pub severe: usize,
    pub critical: usize,
}

/// Symptom data response
#[derive(Debug, Serialize)]
pub struct SymptomDataResponse {
    pub symptoms: Vec<SymptomMetric>,
    pub total_count: i64,
    pub date_range: Option<DateRange>,
    pub analysis: SymptomSummary,
    pub emergency_symptoms: Vec<SymptomMetric>,
}

/// Date range for symptom data
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Symptom summary for health insights
#[derive(Debug, Serialize)]
pub struct SymptomSummary {
    pub total_symptoms: i64,
    pub categories_affected: Vec<String>,
    pub most_common_symptom: Option<SymptomType>,
    pub average_severity_score: Option<f64>,
    pub emergency_count: i64,
    pub chronic_symptoms_count: i64, // Symptoms lasting >1 week
    pub episode_patterns: Vec<EpisodePattern>,
}

/// Episode pattern for illness tracking
#[derive(Debug, Serialize)]
pub struct EpisodePattern {
    pub episode_id: Uuid,
    pub symptom_count: usize,
    pub duration_hours: Option<f64>,
    pub primary_symptoms: Vec<SymptomType>,
    pub severity_pattern: String,
}

/// Symptom data ingestion endpoint
#[instrument(skip(pool, payload))]
pub async fn ingest_symptoms_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<SymptomIngestPayload>,
) -> ActixResult<HttpResponse> {
    let start_time = Instant::now();

    // Record symptom ingest request
    Metrics::record_ingest_request();

    info!(
        user_id = %auth.user.id,
        symptom_count = payload.symptoms.len(),
        "Processing symptom data ingest request"
    );

    if payload.symptoms.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No symptom data provided"
        })));
    }

    // Validate and process symptoms
    let mut processed_count = 0;
    let mut errors = Vec::new();
    let mut emergency_alerts = Vec::new();
    let mut stored_symptoms = Vec::new();

    for (index, symptom_request) in payload.symptoms.iter().enumerate() {
        // Create SymptomMetric from request
        let symptom_metric = SymptomMetric {
            id: Uuid::new_v4(),
            user_id: auth.user.id,
            recorded_at: symptom_request.recorded_at,
            symptom_type: symptom_request.symptom_type,
            severity: symptom_request.severity,
            duration_minutes: symptom_request.duration_minutes,
            notes: symptom_request.notes.clone(),
            episode_id: symptom_request.episode_id,
            source_device: symptom_request.source_device.clone(),
            created_at: Utc::now(),
        };

        // Validate the symptom
        if let Err(e) = symptom_metric.validate() {
            errors.push(SymptomProcessingError {
                index,
                error_type: "ValidationError".to_string(),
                message: e,
                symptom_type: Some(symptom_request.symptom_type),
                severity: Some(symptom_request.severity),
            });
            continue;
        }

        // Check for emergency conditions
        if symptom_metric.is_medical_emergency() {
            emergency_alerts.push(EmergencyAlert {
                symptom_type: symptom_metric.symptom_type,
                severity: symptom_metric.severity,
                urgency_level: symptom_metric.get_urgency_level(),
                message: format!(
                    "Emergency-level {} symptom detected with {} severity",
                    symptom_metric.symptom_type, symptom_metric.severity
                ),
                recommendations: symptom_metric.generate_recommendations(),
                recorded_at: symptom_metric.recorded_at,
            });

            warn!(
                user_id = %auth.user.id,
                symptom_type = %symptom_metric.symptom_type,
                severity = %symptom_metric.severity,
                urgency = symptom_metric.get_urgency_level(),
                "Emergency-level symptom detected"
            );
        }

        // Store symptom in database
        match store_symptom_metric(&pool, &symptom_metric).await {
            Ok(_) => {
                processed_count += 1;
                stored_symptoms.push(symptom_metric);
            }
            Err(e) => {
                error!("Failed to store symptom metric: {}", e);
                errors.push(SymptomProcessingError {
                    index,
                    error_type: "DatabaseError".to_string(),
                    message: format!("Storage failed: {}", e),
                    symptom_type: Some(symptom_request.symptom_type),
                    severity: Some(symptom_request.severity),
                });
            }
        }
    }

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Record metrics
    Metrics::record_ingest_processed_total(processed_count);
    if !errors.is_empty() {
        Metrics::record_ingest_errors_total(errors.len());
    }

    // Generate batch analysis
    let symptom_analysis = if !stored_symptoms.is_empty() {
        Some(generate_batch_analysis(&stored_symptoms))
    } else {
        None
    };

    let response = SymptomIngestResponse {
        success: errors.is_empty(),
        processed_count,
        failed_count: errors.len(),
        processing_time_ms,
        errors,
        emergency_alerts,
        symptom_analysis,
    };

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = response.failed_count,
        emergency_count = response.emergency_alerts.len(),
        processing_time_ms = processing_time_ms,
        "Symptom ingest completed"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Symptom data retrieval endpoint
#[instrument(skip(pool))]
pub async fn get_symptoms_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<SymptomQueryParams>,
) -> ActixResult<HttpResponse> {
    info!(
        user_id = %auth.user.id,
        "Retrieving symptom data"
    );

    // Build query based on parameters
    let limit = query.limit.unwrap_or(100).min(1000); // Max 1000 symptoms per request
    let mut conditions = vec!["user_id = $1".to_string()];
    let mut params: Vec<&(dyn sqlx::Encode<sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Sync)> = vec![&auth.user.id];
    let mut param_count = 1;

    if let Some(start_date) = &query.start_date {
        param_count += 1;
        conditions.push(format!("recorded_at >= ${}", param_count));
        params.push(start_date);
    }

    if let Some(end_date) = &query.end_date {
        param_count += 1;
        conditions.push(format!("recorded_at <= ${}", param_count));
        params.push(end_date);
    }

    if let Some(symptom_type) = &query.symptom_type {
        param_count += 1;
        conditions.push(format!("symptom_type = ${}", param_count));
        params.push(symptom_type);
    }

    if let Some(severity) = &query.severity {
        param_count += 1;
        conditions.push(format!("severity = ${}", param_count));
        params.push(severity);
    }

    if let Some(episode_id) = &query.episode_id {
        param_count += 1;
        conditions.push(format!("episode_id = ${}", param_count));
        params.push(episode_id);
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Build the query
    let query_sql = format!(
        "SELECT id, user_id, recorded_at, symptom_type, severity, duration_minutes, notes, episode_id, source_device, created_at
         FROM symptoms
         {}
         ORDER BY recorded_at DESC
         LIMIT {}",
        where_clause, limit
    );

    // Execute query
    let mut query_builder = sqlx::query_as::<_, SymptomMetric>(&query_sql);

    // Add parameters to the query
    for param in params {
        query_builder = query_builder.bind(param);
    }

    let symptoms = match query_builder.fetch_all(&**pool).await {
        Ok(symptoms) => symptoms,
        Err(e) => {
            error!("Failed to fetch symptoms: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve symptom data"
            })));
        }
    };

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) as count FROM symptoms {}",
        where_clause
    );

    let mut count_query_builder = sqlx::query_scalar::<_, i64>(&count_query);

    // Add parameters for count query
    for param in params {
        count_query_builder = count_query_builder.bind(param);
    }

    let total_count = count_query_builder.fetch_one(&**pool).await.unwrap_or(0);

    // Generate analysis and filter emergency symptoms
    let emergency_symptoms: Vec<SymptomMetric> = symptoms
        .iter()
        .filter(|s| s.is_medical_emergency())
        .cloned()
        .collect();

    let analysis = generate_symptom_summary(&symptoms);
    let date_range = if !symptoms.is_empty() {
        let first = symptoms.last().unwrap();
        let last = symptoms.first().unwrap();
        Some(DateRange {
            start: first.recorded_at,
            end: last.recorded_at,
        })
    } else {
        None
    };

    let response = SymptomDataResponse {
        symptoms,
        total_count,
        date_range,
        analysis,
        emergency_symptoms,
    };

    info!(
        user_id = %auth.user.id,
        total_count = total_count,
        returned_count = response.symptoms.len(),
        emergency_count = response.emergency_symptoms.len(),
        "Symptom data retrieved"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Store a symptom metric in the database
async fn store_symptom_metric(
    pool: &PgPool,
    symptom: &SymptomMetric,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO symptoms (
            id, user_id, recorded_at, symptom_type, severity,
            duration_minutes, notes, episode_id, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (user_id, recorded_at, symptom_type)
        DO UPDATE SET
            severity = EXCLUDED.severity,
            duration_minutes = EXCLUDED.duration_minutes,
            notes = EXCLUDED.notes,
            episode_id = EXCLUDED.episode_id,
            source_device = EXCLUDED.source_device
        "#,
        symptom.id,
        symptom.user_id,
        symptom.recorded_at,
        symptom.symptom_type as SymptomType,
        symptom.severity as SymptomSeverity,
        symptom.duration_minutes,
        symptom.notes,
        symptom.episode_id,
        symptom.source_device,
        symptom.created_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Generate batch analysis for processed symptoms
fn generate_batch_analysis(symptoms: &[SymptomMetric]) -> SymptomBatchAnalysis {
    let total_symptoms = symptoms.len();
    let mut categories: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut severity_counts = SeverityDistribution {
        none: 0,
        mild: 0,
        moderate: 0,
        severe: 0,
        critical: 0,
    };

    let mut emergency_count = 0;
    let mut total_severity_score = 0;
    let mut episodes = std::collections::HashSet::new();

    for symptom in symptoms {
        // Count categories
        let category = symptom.get_category().to_string();
        *categories.entry(category).or_insert(0) += 1;

        // Count severity levels
        match symptom.severity {
            SymptomSeverity::None => severity_counts.none += 1,
            SymptomSeverity::Mild => severity_counts.mild += 1,
            SymptomSeverity::Moderate => severity_counts.moderate += 1,
            SymptomSeverity::Severe => severity_counts.severe += 1,
            SymptomSeverity::Critical => severity_counts.critical += 1,
        }

        // Count emergencies
        if symptom.is_medical_emergency() {
            emergency_count += 1;
        }

        // Accumulate severity scores
        total_severity_score += symptom.severity.to_numeric_score();

        // Track unique episodes
        if let Some(episode_id) = symptom.episode_id {
            episodes.insert(episode_id);
        }
    }

    let average_severity_score = if total_symptoms > 0 {
        total_severity_score as f64 / total_symptoms as f64
    } else {
        0.0
    };

    SymptomBatchAnalysis {
        total_symptoms,
        emergency_count,
        categories,
        severity_distribution: severity_counts,
        episode_count: episodes.len(),
        average_severity_score,
    }
}

/// Generate symptom summary for data response
fn generate_symptom_summary(symptoms: &[SymptomMetric]) -> SymptomSummary {
    let total_symptoms = symptoms.len() as i64;

    if symptoms.is_empty() {
        return SymptomSummary {
            total_symptoms: 0,
            categories_affected: vec![],
            most_common_symptom: None,
            average_severity_score: None,
            emergency_count: 0,
            chronic_symptoms_count: 0,
            episode_patterns: vec![],
        };
    }

    // Count categories
    let mut categories: std::collections::HashSet<String> = std::collections::HashSet::new();
    for symptom in symptoms {
        categories.insert(symptom.get_category().to_string());
    }

    // Find most common symptom
    let mut symptom_counts: std::collections::HashMap<SymptomType, usize> = std::collections::HashMap::new();
    let mut total_severity = 0;
    let mut emergency_count = 0;
    let mut chronic_count = 0;

    for symptom in symptoms {
        *symptom_counts.entry(symptom.symptom_type).or_insert(0) += 1;
        total_severity += symptom.severity.to_numeric_score();

        if symptom.is_medical_emergency() {
            emergency_count += 1;
        }

        // Check for chronic symptoms (>1 week duration)
        if let Some(duration) = symptom.duration_minutes {
            if duration > 7 * 24 * 60 { // 1 week in minutes
                chronic_count += 1;
            }
        }
    }

    let most_common_symptom = symptom_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(symptom_type, _)| *symptom_type);

    let average_severity_score = Some(total_severity as f64 / total_symptoms as f64);

    // Generate episode patterns
    let episode_patterns = generate_episode_patterns(symptoms);

    SymptomSummary {
        total_symptoms,
        categories_affected: categories.into_iter().collect(),
        most_common_symptom,
        average_severity_score,
        emergency_count,
        chronic_symptoms_count: chronic_count,
        episode_patterns,
    }
}

/// Generate episode patterns for illness tracking
fn generate_episode_patterns(symptoms: &[SymptomMetric]) -> Vec<EpisodePattern> {
    let mut episodes: std::collections::HashMap<Uuid, Vec<&SymptomMetric>> = std::collections::HashMap::new();

    // Group symptoms by episode
    for symptom in symptoms {
        if let Some(episode_id) = symptom.episode_id {
            episodes.entry(episode_id).or_insert_with(Vec::new).push(symptom);
        }
    }

    // Generate patterns for each episode
    episodes
        .iter()
        .map(|(episode_id, episode_symptoms)| {
            let symptom_count = episode_symptoms.len();

            // Calculate episode duration
            let mut min_time = episode_symptoms[0].recorded_at;
            let mut max_time = episode_symptoms[0].recorded_at;

            for symptom in episode_symptoms {
                if symptom.recorded_at < min_time {
                    min_time = symptom.recorded_at;
                }
                if symptom.recorded_at > max_time {
                    max_time = symptom.recorded_at;
                }
            }

            let duration_hours = (max_time - min_time).num_minutes() as f64 / 60.0;

            // Get primary symptoms (most severe or most frequent)
            let mut primary_symptoms: Vec<SymptomType> = episode_symptoms
                .iter()
                .map(|s| s.symptom_type)
                .collect();
            primary_symptoms.sort();
            primary_symptoms.dedup();
            primary_symptoms.truncate(3); // Top 3 primary symptoms

            // Determine severity pattern
            let avg_severity: f64 = episode_symptoms
                .iter()
                .map(|s| s.severity.to_numeric_score() as f64)
                .sum::<f64>() / symptom_count as f64;

            let severity_pattern = match avg_severity {
                s if s <= 2.0 => "mild_episode".to_string(),
                s if s <= 5.0 => "moderate_episode".to_string(),
                s if s <= 7.0 => "severe_episode".to_string(),
                _ => "critical_episode".to_string(),
            };

            EpisodePattern {
                episode_id: *episode_id,
                symptom_count,
                duration_hours: if duration_hours > 0.0 {
                    Some(duration_hours)
                } else {
                    None
                },
                primary_symptoms,
                severity_pattern,
            }
        })
        .collect()
}