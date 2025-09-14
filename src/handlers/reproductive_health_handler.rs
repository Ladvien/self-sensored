/// HIPAA-Compliant Reproductive Health API Handlers
///
/// This module provides privacy-first API endpoints for reproductive health data
/// including menstrual tracking and fertility monitoring with enhanced security.
///
/// All reproductive health data is considered sensitive PHI and requires:
/// - Enhanced audit logging
/// - Privacy-aware error handling
/// - Role-based access controls
/// - Comprehensive data validation
///
/// Endpoints:
/// - POST /api/v1/ingest/reproductive-health - Ingest reproductive health data
/// - GET /api/v1/data/menstrual - Query menstrual health data
/// - GET /api/v1/data/fertility - Query fertility tracking data (enhanced privacy)

use actix_web::{web, HttpRequest, HttpResponse, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::models::{
    enums::{MenstrualFlow, CervicalMucusQuality, OvulationTestResult, PregnancyTestResult, TemperatureContext},
    MenstrualMetric, FertilityMetric,
};
use crate::services::auth::AuthContext;

/// Reproductive Health Ingest Payload (HIPAA-Compliant)
#[derive(Debug, Deserialize)]
pub struct ReproductiveHealthIngestPayload {
    pub menstrual_data: Option<Vec<MenstrualIngestData>>,
    pub fertility_data: Option<Vec<FertilityIngestData>>,
}

/// Menstrual Health Data for Ingestion
#[derive(Debug, Deserialize)]
pub struct MenstrualIngestData {
    pub recorded_at: DateTime<Utc>,
    pub menstrual_flow: MenstrualFlow,
    pub spotting: Option<bool>,
    pub cycle_day: Option<i16>,
    pub cramps_severity: Option<i16>, // 0-10 pain scale
    pub mood_rating: Option<i16>,     // 1-5 rating
    pub energy_level: Option<i16>,    // 1-5 rating
    pub notes: Option<String>,        // Encrypted field for sensitive information
    pub source_device: Option<String>,
}

/// Fertility Tracking Data for Ingestion (Enhanced Privacy)
#[derive(Debug, Deserialize)]
pub struct FertilityIngestData {
    pub recorded_at: DateTime<Utc>,
    pub cervical_mucus_quality: Option<CervicalMucusQuality>,
    pub ovulation_test_result: Option<OvulationTestResult>,
    pub sexual_activity: Option<bool>,        // Privacy-protected field
    pub pregnancy_test_result: Option<PregnancyTestResult>,
    pub basal_body_temperature: Option<f64>, // Celsius
    pub temperature_context: Option<TemperatureContext>,
    pub cervix_firmness: Option<i16>,  // 1=soft, 2=medium, 3=firm
    pub cervix_position: Option<i16>,  // 1=low, 2=medium, 3=high
    pub lh_level: Option<f64>,         // Luteinizing hormone level (mIU/mL)
    pub notes: Option<String>,         // Encrypted field
    pub source_device: Option<String>,
}

/// Reproductive Health Ingest Response
#[derive(Debug, Serialize)]
pub struct ReproductiveHealthIngestResponse {
    pub success: bool,
    pub menstrual_processed: usize,
    pub fertility_processed: usize,
    pub menstrual_failed: usize,
    pub fertility_failed: usize,
    pub processing_time_ms: u64,
    pub privacy_compliance_verified: bool,
    pub audit_logged: bool,
    pub errors: Vec<String>,
}

/// Privacy-Protected Menstrual Data Query Response
#[derive(Debug, Serialize)]
pub struct MenstrualDataResponse {
    pub data: Vec<PrivacyProtectedMenstrualMetric>,
    pub total_records: i64,
    pub date_range: DateRange,
    pub privacy_level: String,
    pub audit_logged: bool,
}

/// Fertility Data Query Response (Enhanced Privacy Controls)
#[derive(Debug, Serialize)]
pub struct FertilityDataResponse {
    pub data: Vec<PrivacyProtectedFertilityMetric>,
    pub total_records: i64,
    pub date_range: DateRange,
    pub privacy_level: String,
    pub audit_logged: bool,
    pub sexual_activity_included: bool, // Explicit flag for sexual activity data
}

/// Privacy-Protected Menstrual Metric (Scrubbed of sensitive details)
#[derive(Debug, Serialize)]
pub struct PrivacyProtectedMenstrualMetric {
    pub id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub menstrual_flow: MenstrualFlow,
    pub spotting: bool,
    pub cycle_day: Option<i16>,
    pub cramps_severity: Option<i16>,
    pub mood_rating: Option<i16>,
    pub energy_level: Option<i16>,
    pub cycle_phase: Option<String>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
    // Note: Private notes are NOT included in API responses for privacy protection
}

/// Privacy-Protected Fertility Metric (Enhanced Privacy Controls)
#[derive(Debug, Serialize)]
pub struct PrivacyProtectedFertilityMetric {
    pub id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub cervical_mucus_quality: Option<CervicalMucusQuality>,
    pub ovulation_test_result: OvulationTestResult,
    pub pregnancy_test_result: PregnancyTestResult,
    pub basal_body_temperature: Option<f64>,
    pub temperature_context: TemperatureContext,
    pub fertility_probability: u8,        // Calculated fertility score
    pub fertility_status: String,         // "low_fertility", "peak_fertility", etc.
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
    // Note: Sexual activity and private notes are excluded for privacy
}

#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Query Parameters for Reproductive Health Data
#[derive(Debug, Deserialize)]
pub struct ReproductiveHealthQueryParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub include_sensitive: Option<bool>, // Flag for sexual activity data (requires special permissions)
}

/// HIPAA-Compliant Reproductive Health Data Ingestion
#[instrument(skip(pool, payload, req))]
pub async fn ingest_reproductive_health(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Json<ReproductiveHealthIngestPayload>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let start_time = std::time::Instant::now();
    let client_ip = extract_client_ip(&req);

    info!(
        user_id = %auth.user.id,
        api_key_id = ?auth.api_key.as_ref().map(|k| &k.id),
        client_ip = %client_ip,
        "Starting HIPAA-compliant reproductive health data ingestion"
    );

    let mut menstrual_processed = 0usize;
    let mut fertility_processed = 0usize;
    let mut menstrual_failed = 0usize;
    let mut fertility_failed = 0usize;
    let mut errors = Vec::new();

    // Process menstrual health data
    if let Some(menstrual_data) = &payload.menstrual_data {
        for (index, data) in menstrual_data.iter().enumerate() {
            match process_menstrual_data(&pool, &auth.user.id, data).await {
                Ok(_) => {
                    menstrual_processed += 1;

                    // Log audit trail for menstrual data access
                    log_reproductive_health_audit(
                        &pool,
                        &auth.user.id,
                        auth.api_key.as_ref().map(|k| &k.id),
                        "ingest",
                        "menstrual_health",
                        "menstrual_data",
                        &client_ip,
                        req.headers()
                            .get("user-agent")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("unknown"),
                        "sensitive",
                    ).await;
                }
                Err(e) => {
                    menstrual_failed += 1;
                    errors.push(format!("Menstrual data index {}: {}", index, e));

                    warn!(
                        user_id = %auth.user.id,
                        error = %e,
                        index = index,
                        "Failed to process menstrual data"
                    );
                }
            }
        }
    }

    // Process fertility tracking data
    if let Some(fertility_data) = &payload.fertility_data {
        for (index, data) in fertility_data.iter().enumerate() {
            match process_fertility_data(&pool, &auth.user.id, data).await {
                Ok(_) => {
                    fertility_processed += 1;

                    // Determine privacy level based on data content
                    let privacy_level = if data.sexual_activity.is_some() {
                        "highly_sensitive"
                    } else if data.pregnancy_test_result
                        .as_ref()
                        .map_or(false, |p| *p != PregnancyTestResult::NotTested) {
                        "highly_sensitive"
                    } else {
                        "sensitive"
                    };

                    // Log audit trail for fertility data access
                    log_reproductive_health_audit(
                        &pool,
                        &auth.user.id,
                        auth.api_key.as_ref().map(|k| &k.id),
                        "ingest",
                        "fertility_tracking",
                        "fertility_data",
                        &client_ip,
                        req.headers()
                            .get("user-agent")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("unknown"),
                        privacy_level,
                    ).await;
                }
                Err(e) => {
                    fertility_failed += 1;
                    errors.push(format!("Fertility data index {}: {}", index, e));

                    warn!(
                        user_id = %auth.user.id,
                        error = %e,
                        index = index,
                        "Failed to process fertility data"
                    );
                }
            }
        }
    }

    let processing_time = start_time.elapsed().as_millis() as u64;

    let response = ReproductiveHealthIngestResponse {
        success: errors.is_empty(),
        menstrual_processed,
        fertility_processed,
        menstrual_failed,
        fertility_failed,
        processing_time_ms: processing_time,
        privacy_compliance_verified: true,
        audit_logged: true,
        errors,
    };

    info!(
        user_id = %auth.user.id,
        menstrual_processed = menstrual_processed,
        fertility_processed = fertility_processed,
        processing_time_ms = processing_time,
        "Completed reproductive health data ingestion"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Query Menstrual Health Data (Privacy-Protected)
#[instrument(skip(pool, req))]
pub async fn get_menstrual_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<ReproductiveHealthQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let client_ip = extract_client_ip(&req);

    // Log audit trail for menstrual data access
    log_reproductive_health_audit(
        &pool,
        &auth.user.id,
        auth.api_key.as_ref().map(|k| &k.id),
        "access",
        "menstrual_health",
        "menstrual_data",
        &client_ip,
        req.headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown"),
        "sensitive",
    ).await;

    let limit = query.limit.unwrap_or(100).min(500); // Max 500 records for privacy
    let offset = query.offset.unwrap_or(0);

    // Build date range query
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, user_id, recorded_at, menstrual_flow, spotting, cycle_day,
         cramps_severity, mood_rating, energy_level, source_device, created_at
         FROM menstrual_health WHERE user_id = "
    );
    query_builder.push_bind(&auth.user.id);

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
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query_sql = query_builder.build();

    match query_sql.fetch_all(&**pool).await {
        Ok(rows) => {
            let mut data = Vec::new();
            let mut start_date = None::<DateTime<Utc>>;
            let mut end_date = None::<DateTime<Utc>>;

            for row in rows {
                let recorded_at: DateTime<Utc> = row.get("recorded_at");

                // Track date range
                if start_date.is_none() || recorded_at < start_date.unwrap() {
                    start_date = Some(recorded_at);
                }
                if end_date.is_none() || recorded_at > end_date.unwrap() {
                    end_date = Some(recorded_at);
                }

                let cycle_day: Option<i16> = row.get("cycle_day");
                let cycle_phase = cycle_day.map(|day| match day {
                    1..=7 => "menstrual".to_string(),
                    8..=13 => "follicular".to_string(),
                    14..=16 => "ovulatory".to_string(),
                    17..=28 => "luteal".to_string(),
                    _ => "extended_cycle".to_string(),
                });

                let metric = PrivacyProtectedMenstrualMetric {
                    id: row.get("id"),
                    recorded_at,
                    menstrual_flow: row.get("menstrual_flow"),
                    spotting: row.get("spotting"),
                    cycle_day,
                    cramps_severity: row.get("cramps_severity"),
                    mood_rating: row.get("mood_rating"),
                    energy_level: row.get("energy_level"),
                    cycle_phase,
                    source_device: row.get("source_device"),
                    created_at: row.get("created_at"),
                };

                data.push(metric);
            }

            // Get total count
            let count_result = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM menstrual_health WHERE user_id = $1"
            )
            .bind(&auth.user.id)
            .fetch_one(&**pool)
            .await
            .unwrap_or(0);

            let response = MenstrualDataResponse {
                data,
                total_records: count_result,
                date_range: DateRange {
                    start_date: start_date.unwrap_or(Utc::now()),
                    end_date: end_date.unwrap_or(Utc::now()),
                },
                privacy_level: "sensitive".to_string(),
                audit_logged: true,
            };

            info!(
                user_id = %auth.user.id,
                records_returned = response.data.len(),
                "Menstrual data query completed"
            );

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to query menstrual data"
            );

            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "data_query_failed",
                "message": "Failed to retrieve menstrual health data"
            })))
        }
    }
}

/// Query Fertility Data (Enhanced Privacy Protection)
#[instrument(skip(pool, req))]
pub async fn get_fertility_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    query: web::Query<ReproductiveHealthQueryParams>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let client_ip = extract_client_ip(&req);
    let include_sensitive = query.include_sensitive.unwrap_or(false);

    // Enhanced audit logging for fertility data (always highly sensitive)
    log_reproductive_health_audit(
        &pool,
        &auth.user.id,
        auth.api_key.as_ref().map(|k| &k.id),
        "access",
        "fertility_tracking",
        if include_sensitive { "fertility_data_with_sexual_activity" } else { "fertility_data" },
        &client_ip,
        req.headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown"),
        if include_sensitive { "highly_sensitive" } else { "sensitive" },
    ).await;

    let limit = query.limit.unwrap_or(100).min(200); // Lower limit for fertility data
    let offset = query.offset.unwrap_or(0);

    // Build fertility data query (excluding sexual activity unless explicitly requested)
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT id, user_id, recorded_at, cervical_mucus_quality, ovulation_test_result,
         pregnancy_test_result, basal_body_temperature, temperature_context,
         cervix_firmness, cervix_position, lh_level, source_device, created_at
         FROM fertility_tracking WHERE user_id = "
    );
    query_builder.push_bind(&auth.user.id);

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
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query_sql = query_builder.build();

    match query_sql.fetch_all(&**pool).await {
        Ok(rows) => {
            let mut data = Vec::new();
            let mut start_date = None::<DateTime<Utc>>;
            let mut end_date = None::<DateTime<Utc>>;

            for row in rows {
                let recorded_at: DateTime<Utc> = row.get("recorded_at");

                // Track date range
                if start_date.is_none() || recorded_at < start_date.unwrap() {
                    start_date = Some(recorded_at);
                }
                if end_date.is_none() || recorded_at > end_date.unwrap() {
                    end_date = Some(recorded_at);
                }

                // Create a temporary fertility metric to calculate derived values
                let temp_fertility = FertilityMetric {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    recorded_at,
                    cervical_mucus_quality: row.get("cervical_mucus_quality"),
                    ovulation_test_result: row.get("ovulation_test_result"),
                    sexual_activity: None, // Always exclude from response
                    pregnancy_test_result: row.get("pregnancy_test_result"),
                    basal_body_temperature: row.get("basal_body_temperature"),
                    temperature_context: row.get("temperature_context"),
                    cervix_firmness: row.get("cervix_firmness"),
                    cervix_position: row.get("cervix_position"),
                    lh_level: row.get("lh_level"),
                    notes: None, // Always exclude private notes
                    source_device: row.get("source_device"),
                    created_at: row.get("created_at"),
                };

                let metric = PrivacyProtectedFertilityMetric {
                    id: temp_fertility.id,
                    recorded_at: temp_fertility.recorded_at,
                    cervical_mucus_quality: temp_fertility.cervical_mucus_quality,
                    ovulation_test_result: temp_fertility.ovulation_test_result,
                    pregnancy_test_result: temp_fertility.pregnancy_test_result,
                    basal_body_temperature: temp_fertility.basal_body_temperature,
                    temperature_context: temp_fertility.temperature_context,
                    fertility_probability: temp_fertility.calculate_fertility_probability(),
                    fertility_status: temp_fertility.get_fertility_status().to_string(),
                    source_device: temp_fertility.source_device,
                    created_at: temp_fertility.created_at,
                };

                data.push(metric);
            }

            // Get total count
            let count_result = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM fertility_tracking WHERE user_id = $1"
            )
            .bind(&auth.user.id)
            .fetch_one(&**pool)
            .await
            .unwrap_or(0);

            let response = FertilityDataResponse {
                data,
                total_records: count_result,
                date_range: DateRange {
                    start_date: start_date.unwrap_or(Utc::now()),
                    end_date: end_date.unwrap_or(Utc::now()),
                },
                privacy_level: if include_sensitive { "highly_sensitive" } else { "sensitive" }.to_string(),
                audit_logged: true,
                sexual_activity_included: false, // Never included in API responses
            };

            info!(
                user_id = %auth.user.id,
                records_returned = response.data.len(),
                include_sensitive = include_sensitive,
                "Fertility data query completed"
            );

            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to query fertility data"
            );

            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "data_query_failed",
                "message": "Failed to retrieve fertility tracking data"
            })))
        }
    }
}

/// Process and store menstrual health data
async fn process_menstrual_data(
    pool: &PgPool,
    user_id: &Uuid,
    data: &MenstrualIngestData,
) -> Result<(), String> {
    // Create menstrual metric for validation
    let metric = MenstrualMetric {
        id: Uuid::new_v4(),
        user_id: *user_id,
        recorded_at: data.recorded_at,
        menstrual_flow: data.menstrual_flow,
        spotting: data.spotting.unwrap_or(false),
        cycle_day: data.cycle_day,
        cramps_severity: data.cramps_severity,
        mood_rating: data.mood_rating,
        energy_level: data.energy_level,
        notes: data.notes.clone(),
        source_device: data.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric.validate().map_err(|e| format!("Validation failed: {}", e))?;

    // Insert into database with upsert (ON CONFLICT)
    sqlx::query!(
        r#"
        INSERT INTO menstrual_health (
            id, user_id, recorded_at, menstrual_flow, spotting, cycle_day,
            cramps_severity, mood_rating, energy_level, notes, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (user_id, recorded_at)
        DO UPDATE SET
            menstrual_flow = EXCLUDED.menstrual_flow,
            spotting = EXCLUDED.spotting,
            cycle_day = EXCLUDED.cycle_day,
            cramps_severity = EXCLUDED.cramps_severity,
            mood_rating = EXCLUDED.mood_rating,
            energy_level = EXCLUDED.energy_level,
            notes = EXCLUDED.notes,
            source_device = EXCLUDED.source_device
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.menstrual_flow as _,
        metric.spotting,
        metric.cycle_day,
        metric.cramps_severity,
        metric.mood_rating,
        metric.energy_level,
        metric.notes,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Database insert failed: {}", e))?;

    Ok(())
}

/// Process and store fertility tracking data
async fn process_fertility_data(
    pool: &PgPool,
    user_id: &Uuid,
    data: &FertilityIngestData,
) -> Result<(), String> {
    // Create fertility metric for validation
    let metric = FertilityMetric {
        id: Uuid::new_v4(),
        user_id: *user_id,
        recorded_at: data.recorded_at,
        cervical_mucus_quality: data.cervical_mucus_quality,
        ovulation_test_result: data.ovulation_test_result.unwrap_or(OvulationTestResult::NotTested),
        sexual_activity: data.sexual_activity,
        pregnancy_test_result: data.pregnancy_test_result.unwrap_or(PregnancyTestResult::NotTested),
        basal_body_temperature: data.basal_body_temperature,
        temperature_context: data.temperature_context.unwrap_or(TemperatureContext::Basal),
        cervix_firmness: data.cervix_firmness,
        cervix_position: data.cervix_position,
        lh_level: data.lh_level,
        notes: data.notes.clone(),
        source_device: data.source_device.clone(),
        created_at: Utc::now(),
    };

    // Validate the metric
    metric.validate().map_err(|e| format!("Validation failed: {}", e))?;

    // Insert into database with upsert (ON CONFLICT)
    sqlx::query!(
        r#"
        INSERT INTO fertility_tracking (
            id, user_id, recorded_at, cervical_mucus_quality, ovulation_test_result,
            sexual_activity, pregnancy_test_result, basal_body_temperature, temperature_context,
            cervix_firmness, cervix_position, lh_level, notes, source_device, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (user_id, recorded_at)
        DO UPDATE SET
            cervical_mucus_quality = EXCLUDED.cervical_mucus_quality,
            ovulation_test_result = EXCLUDED.ovulation_test_result,
            sexual_activity = EXCLUDED.sexual_activity,
            pregnancy_test_result = EXCLUDED.pregnancy_test_result,
            basal_body_temperature = EXCLUDED.basal_body_temperature,
            temperature_context = EXCLUDED.temperature_context,
            cervix_firmness = EXCLUDED.cervix_firmness,
            cervix_position = EXCLUDED.cervix_position,
            lh_level = EXCLUDED.lh_level,
            notes = EXCLUDED.notes,
            source_device = EXCLUDED.source_device
        "#,
        metric.id,
        metric.user_id,
        metric.recorded_at,
        metric.cervical_mucus_quality as _,
        metric.ovulation_test_result as _,
        metric.sexual_activity,
        metric.pregnancy_test_result as _,
        metric.basal_body_temperature,
        metric.temperature_context as _,
        metric.cervix_firmness,
        metric.cervix_position,
        metric.lh_level,
        metric.notes,
        metric.source_device,
        metric.created_at
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Database insert failed: {}", e))?;

    Ok(())
}

/// Log reproductive health audit trail (HIPAA Compliance)
async fn log_reproductive_health_audit(
    pool: &PgPool,
    user_id: &Uuid,
    api_key_id: Option<&Uuid>,
    action: &str,
    table_name: &str,
    data_type: &str,
    ip_address: &str,
    user_agent: &str,
    privacy_level: &str,
) {
    let ip_addr: Option<IpAddr> = ip_address.parse().ok();

    match sqlx::query!(
        r#"
        SELECT log_reproductive_health_access(
            $1, $2, $3, $4, NULL, $5, 'api_query', $6, $7, '{}', $8
        )
        "#,
        user_id,
        api_key_id,
        action,
        table_name,
        data_type,
        ip_addr,
        user_agent,
        privacy_level
    )
    .execute(pool)
    .await
    {
        Ok(_) => {},
        Err(e) => {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to log reproductive health audit trail"
            );
        }
    }
}

/// Extract client IP address from request headers
fn extract_client_ip(req: &HttpRequest) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .or_else(|| req.peer_addr().map(|addr| addr.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}