use actix_web::{web, HttpResponse, Result};
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::{db::*, ApiResponse};
use crate::services::auth::AuthContext;

/// Query parameters for filtering health data
#[derive(Debug, Deserialize, Clone)]
pub struct QueryParams {
    /// Start date for filtering (ISO 8601 format)
    pub start_date: Option<DateTime<Utc>>,
    /// End date for filtering (ISO 8601 format)
    pub end_date: Option<DateTime<Utc>>,
    /// Specific metric types to include
    pub metric_types: Option<String>, // comma-separated list
    /// Page number for pagination (default: 1)
    pub page: Option<u32>,
    /// Number of items per page (default: 100, max: 1000)
    pub limit: Option<u32>,
    /// Sort order: 'asc' or 'desc' (default: desc)
    pub sort: Option<String>,
}

/// Response structure for paginated query results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
    pub total_count: i64,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Summary statistics for health metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthSummary {
    pub user_id: Uuid,
    pub date_range: DateRange,
    pub heart_rate: Option<HeartRateSummary>,
    pub blood_pressure: Option<BloodPressureSummary>,
    pub sleep: Option<SleepSummary>,
    pub activity: Option<ActivitySummary>,
    pub workouts: Option<WorkoutSummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeartRateSummary {
    pub count: i64,
    pub avg_resting: Option<f32>,
    pub avg_active: Option<f32>,
    pub min_bpm: Option<i16>,
    pub max_bpm: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BloodPressureSummary {
    pub count: i64,
    pub avg_systolic: Option<f32>,
    pub avg_diastolic: Option<f32>,
    pub latest_reading: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SleepSummary {
    pub count: i64,
    pub avg_duration_hours: Option<f32>,
    pub avg_efficiency: Option<f32>,
    pub total_sleep_time: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivitySummary {
    pub count: i64,
    pub total_steps: Option<i64>,
    pub total_distance_km: Option<f64>,
    pub total_calories: Option<f64>,
    pub avg_daily_steps: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkoutSummary {
    pub count: i64,
    pub total_duration_hours: Option<f32>,
    pub total_calories: Option<f64>,
    pub workout_types: Vec<String>,
}

/// Get heart rate data with filtering and pagination
#[instrument(skip(pool))]
pub async fn get_heart_rate_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = (page - 1) * limit;
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    // Build dynamic query with date filtering
    let mut query = r#"
        SELECT user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device, metadata, created_at
        FROM heart_rate_metrics 
        WHERE user_id = $1
        "#.to_string();

    let mut param_count = 2;
    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY recorded_at {} LIMIT ${} OFFSET ${}",
        sort_order,
        param_count,
        param_count + 1
    ));

    let mut db_query = sqlx::query_as::<_, HeartRateRecord>(&query).bind(auth.user.id);

    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query = db_query.bind(limit as i64).bind(offset as i64);

    match db_query.fetch_all(pool.get_ref()).await {
        Ok(records) => {
            // Get total count for pagination
            let count_result = get_heart_rate_count(&pool, auth.user.id, &params).await;
            let total_count = count_result.unwrap_or(0);

            let pagination = PaginationInfo {
                page,
                limit,
                has_next: (offset + limit) < total_count as u32,
                has_prev: page > 1,
            };

            let response = QueryResponse {
                data: records,
                pagination,
                total_count,
            };

            info!(
                user_id = %auth.user.id,
                record_count = response.data.len(),
                total_count,
                "Heart rate data retrieved"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to retrieve heart rate data"
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve heart rate data".to_string(),
                )),
            )
        }
    }
}

/// Get blood pressure data with filtering and pagination
#[instrument(skip(pool))]
pub async fn get_blood_pressure_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = (page - 1) * limit;
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let mut query = r#"
        SELECT user_id, recorded_at, systolic, diastolic, pulse, source_device, metadata, created_at
        FROM blood_pressure_metrics 
        WHERE user_id = $1
        "#
    .to_string();

    let mut param_count = 2;
    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY recorded_at {} LIMIT ${} OFFSET ${}",
        sort_order,
        param_count,
        param_count + 1
    ));

    let mut db_query = sqlx::query_as::<_, BloodPressureRecord>(&query).bind(auth.user.id);

    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query = db_query.bind(limit as i64).bind(offset as i64);

    match db_query.fetch_all(pool.get_ref()).await {
        Ok(records) => {
            let count_result = get_blood_pressure_count(&pool, auth.user.id, &params).await;
            let total_count = count_result.unwrap_or(0);

            let pagination = PaginationInfo {
                page,
                limit,
                has_next: (offset + limit) < total_count as u32,
                has_prev: page > 1,
            };

            let response = QueryResponse {
                data: records,
                pagination,
                total_count,
            };

            info!(
                user_id = %auth.user.id,
                record_count = response.data.len(),
                total_count,
                "Blood pressure data retrieved"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to retrieve blood pressure data"
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve blood pressure data".to_string(),
                )),
            )
        }
    }
}

/// Get sleep data with filtering and pagination
#[instrument(skip(pool))]
pub async fn get_sleep_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = (page - 1) * limit;
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let mut query = r#"
        SELECT user_id, sleep_start, sleep_end, duration_minutes,
               deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, sleep_efficiency,
               source_device, metadata, created_at
        FROM sleep_metrics 
        WHERE user_id = $1
        "#.to_string();

    let mut param_count = 2;
    if params.start_date.is_some() {
        query.push_str(&format!(" AND sleep_start >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND sleep_end <= ${param_count}"));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY sleep_start {} LIMIT ${} OFFSET ${}",
        sort_order,
        param_count,
        param_count + 1
    ));

    let mut db_query = sqlx::query_as::<_, SleepRecord>(&query).bind(auth.user.id);

    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query = db_query.bind(limit as i64).bind(offset as i64);

    match db_query.fetch_all(pool.get_ref()).await {
        Ok(records) => {
            let count_result = get_sleep_count(&pool, auth.user.id, &params).await;
            let total_count = count_result.unwrap_or(0);

            let pagination = PaginationInfo {
                page,
                limit,
                has_next: (offset + limit) < total_count as u32,
                has_prev: page > 1,
            };

            let response = QueryResponse {
                data: records,
                pagination,
                total_count,
            };

            info!(
                user_id = %auth.user.id,
                record_count = response.data.len(),
                total_count,
                "Sleep data retrieved"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to retrieve sleep data"
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve sleep data".to_string(),
                )),
            )
        }
    }
}

/// Get activity data with filtering and pagination
#[instrument(skip(pool))]
pub async fn get_activity_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = (page - 1) * limit;
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let mut query = r#"
        SELECT user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal,
               flights_climbed, source_device, metadata, created_at
        FROM activity_metrics 
        WHERE user_id = $1
        "#
    .to_string();

    let mut param_count = 2;
    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY recorded_at {} LIMIT ${} OFFSET ${}",
        sort_order,
        param_count,
        param_count + 1
    ));

    let mut db_query = sqlx::query_as::<_, ActivityRecord>(&query).bind(auth.user.id);

    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query = db_query.bind(limit as i64).bind(offset as i64);

    match db_query.fetch_all(pool.get_ref()).await {
        Ok(records) => {
            let count_result = get_activity_count(&pool, auth.user.id, &params).await;
            let total_count = count_result.unwrap_or(0);

            let pagination = PaginationInfo {
                page,
                limit,
                has_next: (offset + limit) < total_count as u32,
                has_prev: page > 1,
            };

            let response = QueryResponse {
                data: records,
                pagination,
                total_count,
            };

            info!(
                user_id = %auth.user.id,
                record_count = response.data.len(),
                total_count,
                "Activity data retrieved"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to retrieve activity data"
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve activity data".to_string(),
                )),
            )
        }
    }
}

/// Get workout data with filtering and pagination
#[instrument(skip(pool))]
pub async fn get_workout_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = (page - 1) * limit;
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let mut query = r#"
        SELECT id, user_id, workout_type, started_at, ended_at, distance_meters,
               avg_heart_rate, max_heart_rate, total_energy_kcal, active_energy_kcal,
               source_device, metadata, created_at
        FROM workouts 
        WHERE user_id = $1
        "#
    .to_string();

    let mut param_count = 2;
    if params.start_date.is_some() {
        query.push_str(&format!(" AND started_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND ended_at <= ${param_count}"));
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY started_at {} LIMIT ${} OFFSET ${}",
        sort_order,
        param_count,
        param_count + 1
    ));

    let mut db_query = sqlx::query_as::<_, WorkoutRecord>(&query).bind(auth.user.id);

    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query = db_query.bind(limit as i64).bind(offset as i64);

    match db_query.fetch_all(pool.get_ref()).await {
        Ok(records) => {
            let count_result = get_workout_count(&pool, auth.user.id, &params).await;
            let total_count = count_result.unwrap_or(0);

            let pagination = PaginationInfo {
                page,
                limit,
                has_next: (offset + limit) < total_count as u32,
                has_prev: page > 1,
            };

            let response = QueryResponse {
                data: records,
                pagination,
                total_count,
            };

            info!(
                user_id = %auth.user.id,
                record_count = response.data.len(),
                total_count,
                "Workout data retrieved"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to retrieve workout data"
            );
            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve workout data".to_string(),
                )),
            )
        }
    }
}

/// Get comprehensive health summary with statistics
#[instrument(skip(pool))]
pub async fn get_health_summary(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<QueryParams>,
) -> Result<HttpResponse> {
    let start_date = params.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(30) // Default to last 30 days
    });
    let end_date = params.end_date.unwrap_or_else(Utc::now);

    let date_range = DateRange {
        start_date,
        end_date,
    };

    // Fetch all summary data in parallel
    let heart_rate_summary =
        get_heart_rate_summary(&pool, auth.user.id, start_date, end_date).await;
    let bp_summary = get_blood_pressure_summary(&pool, auth.user.id, start_date, end_date).await;
    let sleep_summary = get_sleep_summary(&pool, auth.user.id, start_date, end_date).await;
    let activity_summary = get_activity_summary(&pool, auth.user.id, start_date, end_date).await;
    let workout_summary = get_workout_summary(&pool, auth.user.id, start_date, end_date).await;

    let summary = HealthSummary {
        user_id: auth.user.id,
        date_range,
        heart_rate: heart_rate_summary.ok(),
        blood_pressure: bp_summary.ok(),
        sleep: sleep_summary.ok(),
        activity: activity_summary.ok(),
        workouts: workout_summary.ok(),
    };

    info!(
        user_id = %auth.user.id,
        start_date = %start_date,
        end_date = %end_date,
        "Health summary generated"
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(summary)))
}

// Helper functions for counting records
async fn get_heart_rate_count(
    pool: &PgPool,
    user_id: Uuid,
    params: &QueryParams,
) -> Result<i64, sqlx::Error> {
    let mut query = "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1".to_string();
    let mut param_count = 2;

    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
    }

    let mut db_query = sqlx::query_scalar(&query).bind(user_id);
    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query.fetch_one(pool).await
}

async fn get_blood_pressure_count(
    pool: &PgPool,
    user_id: Uuid,
    params: &QueryParams,
) -> Result<i64, sqlx::Error> {
    let mut query = "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1".to_string();
    let mut param_count = 2;

    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
    }

    let mut db_query = sqlx::query_scalar(&query).bind(user_id);
    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query.fetch_one(pool).await
}

async fn get_sleep_count(
    pool: &PgPool,
    user_id: Uuid,
    params: &QueryParams,
) -> Result<i64, sqlx::Error> {
    let mut query = "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1".to_string();
    let mut param_count = 2;

    if params.start_date.is_some() {
        query.push_str(&format!(" AND sleep_start >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND sleep_end <= ${param_count}"));
    }

    let mut db_query = sqlx::query_scalar(&query).bind(user_id);
    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query.fetch_one(pool).await
}

async fn get_activity_count(
    pool: &PgPool,
    user_id: Uuid,
    params: &QueryParams,
) -> Result<i64, sqlx::Error> {
    let mut query = "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1".to_string();
    let mut param_count = 2;

    if params.start_date.is_some() {
        query.push_str(&format!(" AND recorded_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND recorded_at <= ${param_count}"));
    }

    let mut db_query = sqlx::query_scalar(&query).bind(user_id);
    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query.fetch_one(pool).await
}

async fn get_workout_count(
    pool: &PgPool,
    user_id: Uuid,
    params: &QueryParams,
) -> Result<i64, sqlx::Error> {
    let mut query = "SELECT COUNT(*) FROM workouts WHERE user_id = $1".to_string();
    let mut param_count = 2;

    if params.start_date.is_some() {
        query.push_str(&format!(" AND started_at >= ${param_count}"));
        param_count += 1;
    }
    if params.end_date.is_some() {
        query.push_str(&format!(" AND ended_at <= ${param_count}"));
    }

    let mut db_query = sqlx::query_scalar(&query).bind(user_id);
    if let Some(start_date) = params.start_date {
        db_query = db_query.bind(start_date);
    }
    if let Some(end_date) = params.end_date {
        db_query = db_query.bind(end_date);
    }

    db_query.fetch_one(pool).await
}

// Helper functions for summary statistics
pub async fn get_heart_rate_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<HeartRateSummary, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            AVG(CASE WHEN context = 'resting' THEN resting_heart_rate END) as avg_resting,
            AVG(CASE WHEN context != 'resting' OR context IS NULL THEN heart_rate END) as avg_active,
            MIN(heart_rate) as min_bpm,
            MAX(heart_rate) as max_bpm
        FROM heart_rate_metrics 
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    Ok(HeartRateSummary {
        count: result.count.unwrap_or(0),
        avg_resting: result.avg_resting.and_then(|v| v.to_f32()),
        avg_active: result.avg_active.and_then(|v| v.to_f32()),
        min_bpm: if result.min_bpm.unwrap_or(999) == 999 {
            None
        } else {
            result.min_bpm.map(|v| v as i16)
        },
        max_bpm: if result.max_bpm.unwrap_or(0) == 0 {
            None
        } else {
            result.max_bpm.map(|v| v as i16)
        },
    })
}

pub async fn get_blood_pressure_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<BloodPressureSummary, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            AVG(systolic) as avg_systolic,
            AVG(diastolic) as avg_diastolic,
            MAX(recorded_at) as latest_reading
        FROM blood_pressure_metrics 
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    Ok(BloodPressureSummary {
        count: result.count.unwrap_or(0),
        avg_systolic: result.avg_systolic.and_then(|v| v.to_f32()),
        avg_diastolic: result.avg_diastolic.and_then(|v| v.to_f32()),
        latest_reading: result.latest_reading,
    })
}

pub async fn get_sleep_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<SleepSummary, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            AVG(duration_minutes::float / 60.0) as avg_duration_hours,
            AVG(efficiency) as avg_efficiency,
            SUM(duration_minutes) as total_sleep_time
        FROM sleep_metrics 
        WHERE user_id = $1 AND sleep_start BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    Ok(SleepSummary {
        count: result.count.unwrap_or(0),
        avg_duration_hours: result.avg_duration_hours.map(|v| v as f32),
        avg_efficiency: result.avg_efficiency.and_then(|v| v.to_f32()),
        total_sleep_time: result.total_sleep_time.map(|v| v as i32),
    })
}

pub async fn get_activity_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<ActivitySummary, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            SUM(step_count) as total_steps,
            SUM(distance_meters) as total_distance_meters,
            SUM(active_energy_burned_kcal) as total_calories,
            AVG(step_count) as avg_daily_steps
        FROM activity_metrics 
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    Ok(ActivitySummary {
        count: result.count.unwrap_or(0),
        total_steps: result.total_steps,
        total_distance_km: result
            .total_distance_meters
            .and_then(|v| v.to_f64())
            .map(|v| v / 1000.0),
        total_calories: result.total_calories,
        avg_daily_steps: result.avg_daily_steps.and_then(|v| v.to_f32()),
    })
}

pub async fn get_workout_summary(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<WorkoutSummary, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT 
            COUNT(*) as count,
            SUM(EXTRACT(EPOCH FROM (ended_at - started_at)) / 3600.0) as total_duration_hours,
            SUM(total_energy_kcal) as total_calories,
            array_agg(DISTINCT workout_type::text) as workout_types
        FROM workouts 
        WHERE user_id = $1 AND started_at BETWEEN $2 AND $3
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await?;

    Ok(WorkoutSummary {
        count: result.count.unwrap_or(0),
        total_duration_hours: result.total_duration_hours.and_then(|v| v.to_f32()),
        total_calories: result.total_calories.and_then(|v| v.to_f64()),
        workout_types: result.workout_types.unwrap_or_default(),
    })
}
