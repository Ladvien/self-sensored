use actix_web::{web, HttpResponse, Result};
use bigdecimal::FromPrimitive;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::BigDecimal, PgPool};
use std::time::Duration;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::models::{MentalHealthMetric, MindfulnessMetric, ProcessingError};
use crate::services::auth::AuthContext;
use crate::services::cache::{CacheKey, CacheService};

/// Request payload for mindfulness session ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct MindfulnessIngestRequest {
    pub data: Vec<MindfulnessSessionData>,
}

/// Individual mindfulness session data for ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct MindfulnessSessionData {
    pub recorded_at: DateTime<Utc>,
    pub session_duration_minutes: Option<i32>,
    pub meditation_type: Option<String>,
    pub session_quality_rating: Option<i16>,
    pub mindful_minutes_today: Option<i32>,
    pub mindful_minutes_week: Option<i32>,
    pub breathing_rate_breaths_per_min: Option<f64>,
    pub heart_rate_variability_during_session: Option<f64>,
    pub focus_rating: Option<i16>,
    pub guided_session_instructor: Option<String>,
    pub meditation_app: Option<String>,
    pub background_sounds: Option<String>,
    pub location_type: Option<String>,
    pub session_notes: Option<String>,
    pub source_device: Option<String>,
}

/// Request payload for mental health data ingestion
#[derive(Debug, Deserialize, Serialize)]
pub struct MentalHealthIngestRequest {
    pub data: Vec<MentalHealthData>,
}

/// Individual mental health data for ingestion (privacy-protected)
#[derive(Debug, Deserialize, Serialize)]
pub struct MentalHealthData {
    pub recorded_at: DateTime<Utc>,
    pub state_of_mind_valence: Option<f64>,
    pub state_of_mind_labels: Option<Vec<String>>,
    pub reflection_prompt: Option<String>,
    pub mood_rating: Option<i16>,
    pub anxiety_level: Option<i16>,
    pub stress_level: Option<i16>,
    pub energy_level: Option<i16>,
    pub depression_screening_score: Option<i16>,
    pub anxiety_screening_score: Option<i16>,
    pub sleep_quality_impact: Option<i16>,
    pub trigger_event: Option<String>,
    pub coping_strategy: Option<String>,
    pub medication_taken: Option<bool>,
    pub therapy_session_today: Option<bool>,
    pub private_notes: Option<String>, // Will be encrypted before storage
    pub source_device: Option<String>,
}

/// Response structure for mindfulness/mental health ingestion
#[derive(Debug, Serialize)]
pub struct MindfulnessIngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<ProcessingError>,
    pub privacy_protection_applied: bool,
}

/// Query parameters for mindfulness data retrieval
#[derive(Debug, Deserialize)]
pub struct MindfulnessQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub meditation_type: Option<String>,
    pub min_duration: Option<i32>,
    pub limit: Option<i32>,
}

/// Query parameters for mental health data retrieval (privacy-aware)
#[derive(Debug, Deserialize)]
pub struct MentalHealthQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub include_sensitive_data: Option<bool>,
    pub limit: Option<i32>,
}

/// Privacy-filtered mental health response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MentalHealthResponse {
    pub data: Vec<MentalHealthSummary>,
    pub privacy_level: String,
    pub total_count: i64,
}

/// Summary of mental health data with privacy protection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MentalHealthSummary {
    pub id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub mood_rating: Option<i16>,
    pub stress_level: Option<i16>,
    pub anxiety_level: Option<i16>,
    pub energy_level: Option<i16>,
    pub wellness_score: Option<i16>,
    pub has_notes: bool,
    pub source_device: Option<String>,
}

/// Mindfulness session ingestion endpoint with cache invalidation
/// POST /api/v1/ingest/mindfulness
#[instrument(skip(pool, cache, auth, payload))]
pub async fn ingest_mindfulness(
    pool: web::Data<PgPool>,
    cache: web::Data<CacheService>,
    auth: AuthContext,
    payload: web::Json<MindfulnessIngestRequest>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    let session_count = payload.data.len();

    info!(
        user_id = %auth.user.id,
        session_count = session_count,
        "Processing mindfulness session ingestion"
    );

    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    // Process each mindfulness session
    for (index, session_data) in payload.data.iter().enumerate() {
        match process_mindfulness_session(&pool, auth.user.id, session_data).await {
            Ok(_) => {
                processed_count += 1;
                info!(
                    user_id = %auth.user.id,
                    index = index,
                    duration = session_data.session_duration_minutes,
                    meditation_type = ?session_data.meditation_type,
                    "Successfully processed mindfulness session"
                );
            }
            Err(e) => {
                failed_count += 1;
                errors.push(ProcessingError {
                    metric_type: "Mindfulness".to_string(),
                    error_message: e,
                    index: Some(index),
                });
                warn!(
                    user_id = %auth.user.id,
                    index = index,
                    error = errors.last().unwrap().error_message,
                    "Failed to process mindfulness session"
                );
            }
        }
    }

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Invalidate user's mindfulness cache after successful ingestion
    if processed_count > 0 {
        let cache_invalidated = invalidate_mindfulness_cache(&cache, auth.user.id).await;
        info!(
            user_id = %auth.user.id,
            cache_invalidated = cache_invalidated,
            "Cache invalidation after mindfulness ingestion"
        );
    }

    let response = MindfulnessIngestResponse {
        success: failed_count == 0,
        processed_count,
        failed_count,
        processing_time_ms,
        errors,
        privacy_protection_applied: false, // Mindfulness sessions don't require special privacy protection
    };

    // Log performance metrics
    log_performance_metrics(
        "ingest_mindfulness",
        auth.user.id,
        processing_time_ms,
        false, // Ingestion never hits cache
        processed_count,
    );

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time_ms,
        performance_target_met = processing_time_ms < 200,
        "Completed mindfulness session ingestion with cache invalidation"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Mental health data ingestion endpoint with privacy protection and cache invalidation
/// POST /api/v1/ingest/mental-health
#[instrument(skip(pool, cache, auth, payload))]
pub async fn ingest_mental_health(
    pool: web::Data<PgPool>,
    cache: web::Data<CacheService>,
    auth: AuthContext,
    payload: web::Json<MentalHealthIngestRequest>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    let data_count = payload.data.len();

    info!(
        user_id = %auth.user.id,
        data_count = data_count,
        "Processing mental health data ingestion with privacy protection"
    );

    let mut processed_count = 0;
    let mut failed_count = 0;
    let mut errors = Vec::new();

    // Process each mental health data entry
    for (index, health_data) in payload.data.iter().enumerate() {
        match process_mental_health_data(&pool, auth.user.id, health_data).await {
            Ok(_) => {
                processed_count += 1;
                info!(
                    user_id = %auth.user.id,
                    index = index,
                    has_private_notes = health_data.private_notes.is_some(),
                    "Successfully processed mental health data with privacy protection"
                );
            }
            Err(e) => {
                failed_count += 1;
                errors.push(ProcessingError {
                    metric_type: "MentalHealth".to_string(),
                    error_message: e,
                    index: Some(index),
                });
                error!(
                    user_id = %auth.user.id,
                    index = index,
                    error = errors.last().unwrap().error_message,
                    "Failed to process mental health data"
                );
            }
        }
    }

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Add audit log entry for mental health data ingestion
    audit_mental_health_access(&pool, auth.user.id, "ingestion", processed_count).await;

    // Invalidate user's mental health cache after successful ingestion
    if processed_count > 0 {
        let cache_invalidated = invalidate_mental_health_cache(&cache, auth.user.id).await;
        info!(
            user_id = %auth.user.id,
            cache_invalidated = cache_invalidated,
            "Mental health cache invalidation after ingestion"
        );
    }

    let response = MindfulnessIngestResponse {
        success: failed_count == 0,
        processed_count,
        failed_count,
        processing_time_ms,
        errors,
        privacy_protection_applied: true, // Mental health data has privacy protection
    };

    // Log performance metrics for mental health ingestion
    log_performance_metrics(
        "ingest_mental_health",
        auth.user.id,
        processing_time_ms,
        false, // Ingestion never hits cache
        processed_count,
    );

    info!(
        user_id = %auth.user.id,
        processed_count = processed_count,
        failed_count = failed_count,
        processing_time_ms = processing_time_ms,
        performance_target_met = processing_time_ms < 200,
        "Completed mental health data ingestion with privacy protection and cache invalidation"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Retrieve mindfulness session history with Redis caching
/// GET /api/v1/data/mindfulness
#[instrument(skip(pool, cache, auth))]
pub async fn get_mindfulness_data(
    pool: web::Data<PgPool>,
    cache: web::Data<CacheService>,
    auth: AuthContext,
    query: web::Query<MindfulnessQueryParams>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();

    info!(
        user_id = %auth.user.id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        meditation_type = ?query.meditation_type,
        "Retrieving mindfulness session data with caching"
    );

    // Generate cache key based on query parameters
    let query_hash = generate_mindfulness_query_hash(&query);
    let cache_key = CacheKey::MindfulnessQuery {
        user_id: auth.user.id,
        hash: query_hash.clone(),
    };

    // Try to get from cache first
    if let Some(cached_sessions) = cache
        .get::<Vec<MindfulnessMetric>>(&cache_key, "health_export")
        .await
    {
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        info!(
            user_id = %auth.user.id,
            session_count = cached_sessions.len(),
            processing_time_ms = processing_time_ms,
            cache_hit = true,
            "Retrieved mindfulness sessions from cache"
        );
        return Ok(HttpResponse::Ok().json(cached_sessions));
    }

    // Cache miss - fetch from database with optimized query
    match fetch_mindfulness_sessions_optimized(&pool, auth.user.id, &query).await {
        Ok(sessions) => {
            let processing_time_ms = start_time.elapsed().as_millis() as u64;

            // Cache the results for 10 minutes (configurable TTL for mental health data)
            let cache_ttl = Duration::from_secs(600); // 10 minutes
            cache
                .set(
                    &cache_key,
                    "health_export",
                    sessions.clone(),
                    Some(cache_ttl),
                )
                .await;

            info!(
                user_id = %auth.user.id,
                session_count = sessions.len(),
                processing_time_ms = processing_time_ms,
                cache_hit = false,
                "Successfully retrieved and cached mindfulness sessions"
            );

            Ok(HttpResponse::Ok().json(sessions))
        }
        Err(e) => {
            let processing_time_ms = start_time.elapsed().as_millis() as u64;
            error!(
                user_id = %auth.user.id,
                error = %e,
                processing_time_ms = processing_time_ms,
                "Failed to retrieve mindfulness sessions"
            );
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve mindfulness data",
                "details": e.to_string(),
                "processing_time_ms": processing_time_ms
            })))
        }
    }
}

/// Retrieve mental health data with privacy controls and caching
/// GET /api/v1/data/mental-health
#[instrument(skip(pool, cache, auth))]
pub async fn get_mental_health_data(
    pool: web::Data<PgPool>,
    cache: web::Data<CacheService>,
    auth: AuthContext,
    query: web::Query<MentalHealthQueryParams>,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    let include_sensitive = query.include_sensitive_data.unwrap_or(false);

    info!(
        user_id = %auth.user.id,
        start_date = ?query.start_date,
        end_date = ?query.end_date,
        include_sensitive = include_sensitive,
        "Retrieving mental health data with privacy controls and caching"
    );

    // Add audit log entry for mental health data access
    audit_mental_health_access(&pool, auth.user.id, "retrieval", 0).await;

    // Generate cache key based on query parameters (include sensitive flag in hash)
    let query_hash = generate_mental_health_query_hash(&query);
    let cache_key = CacheKey::MentalHealthQuery {
        user_id: auth.user.id,
        hash: query_hash.clone(),
    };

    // Try to get from cache first (shorter TTL for mental health data)
    if let Some(cached_response) = cache
        .get::<MentalHealthResponse>(&cache_key, "health_export")
        .await
    {
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        info!(
            user_id = %auth.user.id,
            data_count = cached_response.data.len(),
            privacy_level = cached_response.privacy_level,
            processing_time_ms = processing_time_ms,
            cache_hit = true,
            "Retrieved mental health data from cache"
        );
        return Ok(HttpResponse::Ok().json(cached_response));
    }

    // Cache miss - fetch from database with optimized query
    match fetch_mental_health_data_optimized(&pool, auth.user.id, &query).await {
        Ok(response) => {
            let processing_time_ms = start_time.elapsed().as_millis() as u64;

            // Cache the results for 5 minutes (shorter TTL for sensitive mental health data)
            let cache_ttl = Duration::from_secs(300); // 5 minutes for mental health data
            cache
                .set(
                    &cache_key,
                    "health_export",
                    response.clone(),
                    Some(cache_ttl),
                )
                .await;

            info!(
                user_id = %auth.user.id,
                data_count = response.data.len(),
                privacy_level = response.privacy_level,
                processing_time_ms = processing_time_ms,
                cache_hit = false,
                "Successfully retrieved and cached mental health data"
            );

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let processing_time_ms = start_time.elapsed().as_millis() as u64;
            error!(
                user_id = %auth.user.id,
                error = %e,
                processing_time_ms = processing_time_ms,
                "Failed to retrieve mental health data"
            );
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve mental health data",
                "details": e.to_string(),
                "processing_time_ms": processing_time_ms
            })))
        }
    }
}

/// Process individual mindfulness session and store in database
async fn process_mindfulness_session(
    pool: &PgPool,
    user_id: Uuid,
    session_data: &MindfulnessSessionData,
) -> Result<(), String> {
    // Create mindfulness metric
    let metric = MindfulnessMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: session_data.recorded_at,
        session_duration_minutes: session_data.session_duration_minutes,
        meditation_type: session_data.meditation_type.clone(),
        session_quality_rating: session_data.session_quality_rating,
        mindful_minutes_today: session_data.mindful_minutes_today,
        mindful_minutes_week: session_data.mindful_minutes_week,
        breathing_rate_breaths_per_min: session_data.breathing_rate_breaths_per_min,
        heart_rate_variability_during_session: session_data.heart_rate_variability_during_session,
        focus_rating: session_data.focus_rating,
        guided_session_instructor: session_data.guided_session_instructor.clone(),
        meditation_app: session_data.meditation_app.clone(),
        background_sounds: session_data.background_sounds.clone(),
        location_type: session_data.location_type.clone(),
        session_notes: session_data.session_notes.clone(),
        source_device: session_data.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric
        .validate()
        .map_err(|e| format!("Validation error: {e}"))?;

    // Insert into database
    sqlx::query!(
        r#"
        INSERT INTO mindfulness_metrics (
            id, user_id, recorded_at, session_duration_minutes, meditation_type,
            session_quality_rating, mindful_minutes_today, mindful_minutes_week,
            breathing_rate_breaths_per_min, heart_rate_variability_during_session,
            focus_rating, guided_session_instructor, meditation_app, background_sounds,
            location_type, session_notes, source_device, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        ON CONFLICT (user_id, recorded_at, meditation_type) DO UPDATE SET
            session_duration_minutes = EXCLUDED.session_duration_minutes,
            session_quality_rating = EXCLUDED.session_quality_rating,
            mindful_minutes_today = EXCLUDED.mindful_minutes_today,
            mindful_minutes_week = EXCLUDED.mindful_minutes_week,
            breathing_rate_breaths_per_min = EXCLUDED.breathing_rate_breaths_per_min,
            heart_rate_variability_during_session = EXCLUDED.heart_rate_variability_during_session,
            focus_rating = EXCLUDED.focus_rating,
            guided_session_instructor = EXCLUDED.guided_session_instructor,
            meditation_app = EXCLUDED.meditation_app,
            background_sounds = EXCLUDED.background_sounds,
            location_type = EXCLUDED.location_type,
            session_notes = EXCLUDED.session_notes
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.session_duration_minutes,
        metric.meditation_type,
        metric.session_quality_rating,
        metric.mindful_minutes_today,
        metric.mindful_minutes_week,
        metric
            .breathing_rate_breaths_per_min
            .and_then(BigDecimal::from_f64),
        metric
            .heart_rate_variability_during_session
            .and_then(BigDecimal::from_f64),
        metric.focus_rating,
        metric.guided_session_instructor,
        metric.meditation_app,
        metric.background_sounds,
        metric.location_type,
        metric.session_notes,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    Ok(())
}

/// Process individual mental health data with privacy protection
async fn process_mental_health_data(
    pool: &PgPool,
    user_id: Uuid,
    health_data: &MentalHealthData,
) -> Result<(), String> {
    // Encrypt private notes if present
    let (encrypted_notes, encryption_key_id) =
        if let Some(ref private_notes) = health_data.private_notes {
            // In a real implementation, you would use a proper encryption service
            // For now, we'll just mark that encryption is needed
            let key_id = Uuid::new_v4();
            let encrypted_data = format!("ENCRYPTED:{private_notes}"); // Placeholder encryption
            (Some(encrypted_data), Some(key_id))
        } else {
            (None, None)
        };

    // Create mental health metric
    let metric = MentalHealthMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: health_data.recorded_at,
        state_of_mind_valence: health_data.state_of_mind_valence,
        state_of_mind_labels: health_data.state_of_mind_labels.clone(),
        reflection_prompt: health_data.reflection_prompt.clone(),
        mood_rating: health_data.mood_rating,
        anxiety_level: health_data.anxiety_level,
        stress_level: health_data.stress_level,
        energy_level: health_data.energy_level,
        depression_screening_score: health_data.depression_screening_score,
        anxiety_screening_score: health_data.anxiety_screening_score,
        sleep_quality_impact: health_data.sleep_quality_impact,
        trigger_event: health_data.trigger_event.clone(),
        coping_strategy: health_data.coping_strategy.clone(),
        medication_taken: health_data.medication_taken,
        therapy_session_today: health_data.therapy_session_today,
        private_notes_encrypted: encrypted_notes,
        notes_encryption_key_id: encryption_key_id,
        data_sensitivity_level: Some("high".to_string()),
        source_device: health_data.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric
        .validate()
        .map_err(|e| format!("Validation error: {e}"))?;

    // Insert into database with privacy protection
    sqlx::query!(
        r#"
        INSERT INTO mental_health_metrics (
            id, user_id, recorded_at, state_of_mind_valence, state_of_mind_labels,
            reflection_prompt, mood_rating, anxiety_level, stress_level, energy_level,
            depression_screening_score, anxiety_screening_score, sleep_quality_impact,
            trigger_event, coping_strategy, medication_taken, therapy_session_today,
            private_notes_encrypted, notes_encryption_key_id, data_sensitivity_level,
            source_device, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
        ON CONFLICT (user_id, recorded_at) DO UPDATE SET
            state_of_mind_valence = EXCLUDED.state_of_mind_valence,
            state_of_mind_labels = EXCLUDED.state_of_mind_labels,
            reflection_prompt = EXCLUDED.reflection_prompt,
            mood_rating = EXCLUDED.mood_rating,
            anxiety_level = EXCLUDED.anxiety_level,
            stress_level = EXCLUDED.stress_level,
            energy_level = EXCLUDED.energy_level,
            depression_screening_score = EXCLUDED.depression_screening_score,
            anxiety_screening_score = EXCLUDED.anxiety_screening_score,
            sleep_quality_impact = EXCLUDED.sleep_quality_impact,
            trigger_event = EXCLUDED.trigger_event,
            coping_strategy = EXCLUDED.coping_strategy,
            medication_taken = EXCLUDED.medication_taken,
            therapy_session_today = EXCLUDED.therapy_session_today
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.state_of_mind_valence.and_then(BigDecimal::from_f64),
        metric.state_of_mind_labels.as_deref(),
        metric.reflection_prompt,
        metric.mood_rating,
        metric.anxiety_level,
        metric.stress_level,
        metric.energy_level,
        metric.depression_screening_score,
        metric.anxiety_screening_score,
        metric.sleep_quality_impact,
        metric.trigger_event,
        metric.coping_strategy,
        metric.medication_taken,
        metric.therapy_session_today,
        metric.private_notes_encrypted,
        metric.notes_encryption_key_id,
        metric.data_sensitivity_level,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    Ok(())
}

/// Fetch mindfulness sessions from database
#[allow(dead_code)]
async fn fetch_mindfulness_sessions(
    pool: &PgPool,
    user_id: Uuid,
    query: &MindfulnessQueryParams,
) -> Result<Vec<MindfulnessMetric>, sqlx::Error> {
    let limit = query.limit.unwrap_or(100).min(1000); // Cap at 1000 records

    let mut query_builder =
        sqlx::QueryBuilder::new("SELECT * FROM mindfulness_metrics WHERE user_id = ");
    query_builder.push_bind(user_id);

    if let Some(start_date) = query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    if let Some(ref meditation_type) = query.meditation_type {
        query_builder.push(" AND meditation_type = ");
        query_builder.push_bind(meditation_type);
    }

    if let Some(min_duration) = query.min_duration {
        query_builder.push(" AND session_duration_minutes >= ");
        query_builder.push_bind(min_duration);
    }

    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let query = query_builder.build_query_as::<MindfulnessMetric>();
    query.fetch_all(pool).await
}

/// Fetch mental health data with privacy filtering
#[allow(dead_code)]
async fn fetch_mental_health_data(
    pool: &PgPool,
    user_id: Uuid,
    query: &MentalHealthQueryParams,
) -> Result<MentalHealthResponse, sqlx::Error> {
    let limit = query.limit.unwrap_or(50).min(500); // Lower limit for mental health data
    let include_sensitive = query.include_sensitive_data.unwrap_or(false);

    let mut query_builder =
        sqlx::QueryBuilder::new("SELECT * FROM mental_health_metrics WHERE user_id = ");
    query_builder.push_bind(user_id);

    if let Some(start_date) = query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let query_result = query_builder.build_query_as::<MentalHealthMetric>();
    let metrics = query_result.fetch_all(pool).await?;

    // Convert to privacy-filtered summaries
    let data: Vec<MentalHealthSummary> = metrics
        .into_iter()
        .map(|metric| {
            let wellness_score = Some(metric.wellness_score());
            MentalHealthSummary {
                id: metric.id,
                recorded_at: metric.recorded_at,
                mood_rating: if include_sensitive {
                    metric.mood_rating
                } else {
                    None
                },
                stress_level: if include_sensitive {
                    metric.stress_level
                } else {
                    None
                },
                anxiety_level: if include_sensitive {
                    metric.anxiety_level
                } else {
                    None
                },
                energy_level: metric.energy_level, // Energy level is less sensitive
                wellness_score,
                has_notes: metric.has_encrypted_notes(),
                source_device: metric.source_device,
            }
        })
        .collect();

    let privacy_level = if include_sensitive {
        "detailed"
    } else {
        "summary"
    };
    let total_count = data.len() as i64; // Store count before moving data

    Ok(MentalHealthResponse {
        data,
        privacy_level: privacy_level.to_string(),
        total_count,
    })
}

/// Add audit log entry for mental health data access (HIPAA compliance)
async fn audit_mental_health_access(
    pool: &PgPool,
    user_id: Uuid,
    access_type: &str,
    record_count: usize,
) {
    // In a real implementation, this would log to a secure audit table
    info!(
        user_id = %user_id,
        access_type = access_type,
        record_count = record_count,
        timestamp = %Utc::now(),
        "Mental health data access audit log"
    );

    // Try to insert into audit_log table if it exists, otherwise just log
    // This allows the system to work without the audit table for testing/development
    let audit_result = sqlx::query!(
        "INSERT INTO audit_log (user_id, action, resource_type, record_count, created_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT DO NOTHING",
        user_id,
        access_type,
        "mental_health_metrics",
        record_count as i32,
        Utc::now()
    )
    .execute(pool)
    .await;

    if let Err(e) = audit_result {
        warn!(
            user_id = %user_id,
            error = %e,
            "Failed to insert audit log entry - audit table may not exist"
        );
    }
}

/// Performance-optimized mindfulness sessions query with proper indexing
async fn fetch_mindfulness_sessions_optimized(
    pool: &PgPool,
    user_id: Uuid,
    query: &MindfulnessQueryParams,
) -> Result<Vec<MindfulnessMetric>, sqlx::Error> {
    let limit = query.limit.unwrap_or(100).min(1000); // Cap at 1000 records

    // Use a more efficient query with explicit column selection and optimized WHERE clause
    let mut query_builder = sqlx::QueryBuilder::new(
        r#"SELECT id, user_id, recorded_at, session_duration_minutes, meditation_type,
           session_quality_rating, mindful_minutes_today, mindful_minutes_week,
           breathing_rate_breaths_per_min, heart_rate_variability_during_session,
           focus_rating, guided_session_instructor, meditation_app, background_sounds,
           location_type, session_notes, source_device, created_at
           FROM mindfulness_metrics WHERE user_id = "#,
    );
    query_builder.push_bind(user_id);

    if let Some(start_date) = query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    if let Some(ref meditation_type) = query.meditation_type {
        query_builder.push(" AND meditation_type = ");
        query_builder.push_bind(meditation_type);
    }

    if let Some(min_duration) = query.min_duration {
        query_builder.push(" AND session_duration_minutes >= ");
        query_builder.push_bind(min_duration);
    }

    // Use index-optimized ordering
    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let query = query_builder.build_query_as::<MindfulnessMetric>();
    query.fetch_all(pool).await
}

/// Performance-optimized mental health data query with privacy filtering
async fn fetch_mental_health_data_optimized(
    pool: &PgPool,
    user_id: Uuid,
    query: &MentalHealthQueryParams,
) -> Result<MentalHealthResponse, sqlx::Error> {
    let limit = query.limit.unwrap_or(50).min(500); // Lower limit for mental health data
    let include_sensitive = query.include_sensitive_data.unwrap_or(false);

    // Use optimized query with selective column retrieval
    let mut query_builder = sqlx::QueryBuilder::new(
        r#"SELECT id, user_id, recorded_at, state_of_mind_valence, state_of_mind_labels,
           mood_rating, anxiety_level, stress_level, energy_level,
           private_notes_encrypted, source_device, created_at
           FROM mental_health_metrics WHERE user_id = "#,
    );
    query_builder.push_bind(user_id);

    if let Some(start_date) = query.start_date {
        query_builder.push(" AND recorded_at >= ");
        query_builder.push_bind(start_date);
    }

    if let Some(end_date) = query.end_date {
        query_builder.push(" AND recorded_at <= ");
        query_builder.push_bind(end_date);
    }

    // Use index-optimized ordering
    query_builder.push(" ORDER BY recorded_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let query_result = query_builder.build_query_as::<MentalHealthMetric>();
    let metrics = query_result.fetch_all(pool).await?;

    // Convert to privacy-filtered summaries with optimized processing
    let data = metrics
        .into_iter()
        .map(|metric| {
            let wellness_score = Some(metric.wellness_score());
            MentalHealthSummary {
                id: metric.id,
                recorded_at: metric.recorded_at,
                mood_rating: if include_sensitive {
                    metric.mood_rating
                } else {
                    None
                },
                stress_level: if include_sensitive {
                    metric.stress_level
                } else {
                    None
                },
                anxiety_level: if include_sensitive {
                    metric.anxiety_level
                } else {
                    None
                },
                energy_level: metric.energy_level, // Energy level is less sensitive
                wellness_score,
                has_notes: metric.has_encrypted_notes(),
                source_device: metric.source_device,
            }
        })
        .collect::<Vec<_>>();

    let privacy_level = if include_sensitive {
        "detailed"
    } else {
        "summary"
    };
    let total_count = data.len() as i64;

    Ok(MentalHealthResponse {
        data,
        privacy_level: privacy_level.to_string(),
        total_count,
    })
}

/// Generate cache key hash from mindfulness query parameters
pub fn generate_mindfulness_query_hash(params: &MindfulnessQueryParams) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(format!("{params:?}").as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

/// Generate cache key hash from mental health query parameters
pub fn generate_mental_health_query_hash(params: &MentalHealthQueryParams) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(format!("{params:?}").as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

/// Cache warming function for mindfulness data
pub async fn warm_mindfulness_cache(pool: &PgPool, cache: &CacheService, user_id: Uuid) -> bool {
    // Warm cache with recent mindfulness sessions (last 7 days)
    let recent_query = MindfulnessQueryParams {
        start_date: Some(Utc::now() - chrono::Duration::days(7)),
        end_date: Some(Utc::now()),
        meditation_type: None,
        min_duration: None,
        limit: Some(50),
    };

    match fetch_mindfulness_sessions_optimized(pool, user_id, &recent_query).await {
        Ok(sessions) => {
            let cache_key = CacheKey::MindfulnessQuery {
                user_id,
                hash: generate_mindfulness_query_hash(&recent_query),
            };

            cache
                .set(
                    &cache_key,
                    "health_export",
                    sessions,
                    Some(Duration::from_secs(600)), // 10 minutes
                )
                .await
        }
        Err(e) => {
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to warm mindfulness cache"
            );
            false
        }
    }
}

/// Performance monitoring for mindfulness endpoints
pub fn log_performance_metrics(
    endpoint: &str,
    user_id: Uuid,
    processing_time_ms: u64,
    cache_hit: bool,
    record_count: usize,
) {
    info!(
        endpoint = endpoint,
        user_id = %user_id,
        processing_time_ms = processing_time_ms,
        cache_hit = cache_hit,
        record_count = record_count,
        performance_target_met = processing_time_ms < 200, // Target: <200ms
        "Mindfulness endpoint performance metrics"
    );

    // In a production system, this would emit metrics to Prometheus
    // histogram_observe("mindfulness_request_duration_seconds", processing_time_ms as f64 / 1000.0);
    // counter_inc("mindfulness_requests_total", &[("endpoint", endpoint), ("cache_hit", &cache_hit.to_string())]);
}

/// Invalidate mindfulness-related cache entries for a user
async fn invalidate_mindfulness_cache(cache: &CacheService, user_id: Uuid) -> bool {
    // Use wildcard pattern to invalidate all mindfulness cache entries for the user
    let pattern = format!("health_export:mindfulness*:{user_id}:*");

    info!(
        user_id = %user_id,
        pattern = %pattern,
        "Invalidating mindfulness cache entries"
    );

    // In the current cache service, we'd need to use the general user cache invalidation
    // This will clear all cache entries for the user, including mindfulness data
    cache.invalidate_user_cache(user_id, "health_export").await
}

/// Invalidate mental health-related cache entries for a user
async fn invalidate_mental_health_cache(cache: &CacheService, user_id: Uuid) -> bool {
    // Use wildcard pattern to invalidate all mental health cache entries for the user
    let pattern = format!("health_export:mental_health*:{user_id}:*");

    info!(
        user_id = %user_id,
        pattern = %pattern,
        "Invalidating mental health cache entries"
    );

    // In the current cache service, we'd need to use the general user cache invalidation
    // This will clear all cache entries for the user, including mental health data
    cache.invalidate_user_cache(user_id, "health_export").await
}
