use actix_web::{web, HttpResponse, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::fmt::Write as FmtWrite;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::{ApiResponse, db::*};
use crate::services::auth::AuthContext;

/// Export format options
#[derive(Debug, Deserialize)]
pub struct ExportParams {
    /// Export format: 'json' or 'csv' (default: json)
    pub format: Option<String>,
    /// Start date for filtering (ISO 8601 format)
    pub start_date: Option<DateTime<Utc>>,
    /// End date for filtering (ISO 8601 format) 
    pub end_date: Option<DateTime<Utc>>,
    /// Metric types to include (comma-separated)
    pub metric_types: Option<String>,
    /// Include raw data fields (default: false)
    pub include_raw: Option<bool>,
}

/// Export response metadata
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub user_id: Uuid,
    pub export_format: String,
    pub date_range: String,
    pub metric_types: Vec<String>,
    pub record_count: usize,
    pub export_timestamp: DateTime<Utc>,
    pub data: String, // JSON string or CSV content
}

/// Export all health data in specified format
#[instrument(skip(pool))]
pub async fn export_health_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<ExportParams>,
) -> Result<HttpResponse> {
    let format = params.format.as_deref().unwrap_or("json").to_lowercase();
    let include_raw = params.include_raw.unwrap_or(false);
    
    let start_date = params.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(365) // Default to last year
    });
    let end_date = params.end_date.unwrap_or_else(|| Utc::now());

    // Determine which metric types to include
    let metric_types = if let Some(types) = &params.metric_types {
        types.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec!["heart_rate".to_string(), "blood_pressure".to_string(), 
             "sleep".to_string(), "activity".to_string(), "workouts".to_string()]
    };

    info!(
        user_id = %auth.user.id,
        format = %format,
        start_date = %start_date,
        end_date = %end_date,
        metric_types = ?metric_types,
        "Starting health data export"
    );

    match format.as_str() {
        "csv" => export_as_csv(&pool, auth, start_date, end_date, &metric_types, include_raw).await,
        "json" => export_as_json(&pool, auth, start_date, end_date, &metric_types, include_raw).await,
        _ => Ok(HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("Invalid format. Supported formats: json, csv".to_string())
        ))
    }
}

/// Export heart rate data only
#[instrument(skip(pool))]
pub async fn export_heart_rate_data(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<ExportParams>,
) -> Result<HttpResponse> {
    let format = params.format.as_deref().unwrap_or("json").to_lowercase();
    let include_raw = params.include_raw.unwrap_or(false);
    
    let start_date = params.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(90) // Default to last 90 days
    });
    let end_date = params.end_date.unwrap_or_else(|| Utc::now());

    match format.as_str() {
        "csv" => export_heart_rate_csv(&pool, auth, start_date, end_date, include_raw).await,
        "json" => export_heart_rate_json(&pool, auth, start_date, end_date, include_raw).await,
        _ => Ok(HttpResponse::BadRequest().json(
            ApiResponse::<()>::error("Invalid format. Supported formats: json, csv".to_string())
        ))
    }
}

/// Export activity data for dashboard/analytics
#[instrument(skip(pool))]
pub async fn export_activity_summary(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    params: web::Query<ExportParams>,
) -> Result<HttpResponse> {
    let start_date = params.start_date.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(30)
    });
    let end_date = params.end_date.unwrap_or_else(|| Utc::now());

    let user_id = auth.user.id;
    match export_activity_analytics(&pool, auth, start_date, end_date).await {
        Ok(response) => {
            info!(
                user_id = %user_id,
                record_count = response.len(),
                "Activity analytics exported"
            );
            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to export activity analytics"
            );
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to export activity data".to_string())
            ))
        }
    }
}

// Implementation functions

async fn export_as_json(
    pool: &PgPool,
    auth: AuthContext,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    metric_types: &[String],
    include_raw: bool,
) -> Result<HttpResponse> {
    let mut export_data = serde_json::Map::new();
    let mut total_records = 0;

    // Export each metric type
    for metric_type in metric_types {
        match metric_type.as_str() {
            "heart_rate" => {
                if let Ok(data) = fetch_heart_rate_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    total_records += data.len();
                    export_data.insert("heart_rate".to_string(), serde_json::to_value(data).unwrap());
                }
            }
            "blood_pressure" => {
                if let Ok(data) = fetch_blood_pressure_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    total_records += data.len();
                    export_data.insert("blood_pressure".to_string(), serde_json::to_value(data).unwrap());
                }
            }
            "sleep" => {
                if let Ok(data) = fetch_sleep_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    total_records += data.len();
                    export_data.insert("sleep".to_string(), serde_json::to_value(data).unwrap());
                }
            }
            "activity" => {
                if let Ok(data) = fetch_activity_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    total_records += data.len();
                    export_data.insert("activity".to_string(), serde_json::to_value(data).unwrap());
                }
            }
            "workouts" => {
                if let Ok(data) = fetch_workout_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    total_records += data.len();
                    export_data.insert("workouts".to_string(), serde_json::to_value(data).unwrap());
                }
            }
            _ => {} // Skip unknown types
        }
    }

    let response = ExportResponse {
        user_id: auth.user.id,
        export_format: "json".to_string(),
        date_range: format!("{} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
        metric_types: metric_types.to_vec(),
        record_count: total_records,
        export_timestamp: Utc::now(),
        data: serde_json::to_string_pretty(&export_data).unwrap(),
    };

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .insert_header((
            "Content-Disposition", 
            format!("attachment; filename=\"health_data_export_{}.json\"", 
                Utc::now().format("%Y%m%d_%H%M%S"))
        ))
        .json(ApiResponse::success(response)))
}

async fn export_as_csv(
    pool: &PgPool,
    auth: AuthContext,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    metric_types: &[String],
    include_raw: bool,
) -> Result<HttpResponse> {
    let mut csv_content = String::new();
    let mut total_records = 0;

    // Create comprehensive CSV with all metric types
    writeln!(csv_content, "metric_type,timestamp,value1,value2,value3,value4,value5,source,context").unwrap();

    for metric_type in metric_types {
        match metric_type.as_str() {
            "heart_rate" => {
                if let Ok(data) = fetch_heart_rate_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    for record in &data {
                        writeln!(
                            csv_content,
                            "heart_rate,{},{},{},,,,,{},{}",
                            record.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                            record.heart_rate,
                            record.resting_heart_rate.map_or("".to_string(), |v| v.to_string()),
                            record.source_device.as_deref().unwrap_or(""),
                            record.context.as_deref().unwrap_or("")
                        ).unwrap();
                    }
                    total_records += data.len();
                }
            }
            "blood_pressure" => {
                if let Ok(data) = fetch_blood_pressure_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    for record in &data {
                        writeln!(
                            csv_content,
                            "blood_pressure,{},{},{},{},,,{},",
                            record.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                            record.systolic,
                            record.diastolic,
                            record.pulse.map_or("".to_string(), |v| v.to_string()),
                            record.source_device.as_deref().unwrap_or("")
                        ).unwrap();
                    }
                    total_records += data.len();
                }
            }
            "sleep" => {
                if let Ok(data) = fetch_sleep_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    for record in &data {
                        writeln!(
                            csv_content,
                            "sleep,{},{},{},{},{},{},{},",
                            record.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                            record.total_sleep_minutes,
                            record.deep_sleep_minutes.map_or("".to_string(), |v| v.to_string()),
                            record.rem_sleep_minutes.map_or("".to_string(), |v| v.to_string()),
                            record.awake_minutes.map_or("".to_string(), |v| v.to_string()),
                            record.efficiency_percentage.map_or("".to_string(), |v| v.to_string()),
                            record.source_device.as_deref().unwrap_or("")
                        ).unwrap();
                    }
                    total_records += data.len();
                }
            }
            "activity" => {
                if let Ok(data) = fetch_activity_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    for record in &data {
                        writeln!(
                            csv_content,
                            "activity,{},{},{},{},{},{},{},",
                            record.date.format("%Y-%m-%d"),
                            record.steps.map_or("".to_string(), |v| v.to_string()),
                            record.distance_meters.map_or("".to_string(), |v| v.to_string()),
                            record.calories_burned.map_or("".to_string(), |v| v.to_string()),
                            record.active_minutes.map_or("".to_string(), |v| v.to_string()),
                            record.flights_climbed.map_or("".to_string(), |v| v.to_string()),
                            record.source_device.as_deref().unwrap_or("")
                        ).unwrap();
                    }
                    total_records += data.len();
                }
            }
            "workouts" => {
                if let Ok(data) = fetch_workout_data(pool, auth.user.id, start_date, end_date, include_raw).await {
                    for record in &data {
                        writeln!(
                            csv_content,
                            "workout,{},{},{},{},{},{},{},{}",
                            record.start_time.format("%Y-%m-%d %H:%M:%S"),
                            record.workout_type,
                            (record.end_time - record.start_time).num_minutes(),
                            record.total_energy_kcal.map_or("".to_string(), |v| v.to_string()),
                            record.distance_meters.map_or("".to_string(), |v| v.to_string()),
                            record.avg_heart_rate.map_or("".to_string(), |v| v.to_string()),
                            record.source_device.as_deref().unwrap_or(""),
                            record.workout_type
                        ).unwrap();
                    }
                    total_records += data.len();
                }
            }
            _ => {} // Skip unknown types
        }
    }

    let response = ExportResponse {
        user_id: auth.user.id,
        export_format: "csv".to_string(),
        date_range: format!("{} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
        metric_types: metric_types.to_vec(),
        record_count: total_records,
        export_timestamp: Utc::now(),
        data: csv_content,
    };

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "text/csv"))
        .insert_header((
            "Content-Disposition", 
            format!("attachment; filename=\"health_data_export_{}.csv\"", 
                Utc::now().format("%Y%m%d_%H%M%S"))
        ))
        .json(ApiResponse::success(response)))
}

async fn export_heart_rate_json(
    pool: &PgPool,
    auth: AuthContext,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    include_raw: bool,
) -> Result<HttpResponse> {
    match fetch_heart_rate_data(pool, auth.user.id, start_date, end_date, include_raw).await {
        Ok(data) => {
            let response = ExportResponse {
                user_id: auth.user.id,
                export_format: "json".to_string(),
                date_range: format!("{} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
                metric_types: vec!["heart_rate".to_string()],
                record_count: data.len(),
                export_timestamp: Utc::now(),
                data: serde_json::to_string_pretty(&data).unwrap(),
            };

            Ok(HttpResponse::Ok()
                .insert_header(("Content-Type", "application/json"))
                .insert_header((
                    "Content-Disposition", 
                    format!("attachment; filename=\"heart_rate_export_{}.json\"", 
                        Utc::now().format("%Y%m%d_%H%M%S"))
                ))
                .json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to export heart rate data"
            );
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to export heart rate data".to_string())
            ))
        }
    }
}

async fn export_heart_rate_csv(
    pool: &PgPool,
    auth: AuthContext,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    include_raw: bool,
) -> Result<HttpResponse> {
    match fetch_heart_rate_data(pool, auth.user.id, start_date, end_date, include_raw).await {
        Ok(data) => {
            let mut csv_content = String::new();
            writeln!(csv_content, "recorded_at,min_bpm,avg_bpm,max_bpm,context,source").unwrap();

            for record in &data {
                writeln!(
                    csv_content,
                    "{},{},{},{},{},{}",
                    record.recorded_at.format("%Y-%m-%d %H:%M:%S"),
                    record.min_bpm.map_or("".to_string(), |v| v.to_string()),
                    record.avg_bpm.map_or("".to_string(), |v| v.to_string()),
                    record.max_bpm.map_or("".to_string(), |v| v.to_string()),
                    record.context.as_deref().unwrap_or(""),
                    record.source.as_deref().unwrap_or("")
                ).unwrap();
            }

            let response = ExportResponse {
                user_id: auth.user.id,
                export_format: "csv".to_string(),
                date_range: format!("{} to {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
                metric_types: vec!["heart_rate".to_string()],
                record_count: data.len(),
                export_timestamp: Utc::now(),
                data: csv_content,
            };

            Ok(HttpResponse::Ok()
                .insert_header(("Content-Type", "text/csv"))
                .insert_header((
                    "Content-Disposition", 
                    format!("attachment; filename=\"heart_rate_export_{}.csv\"", 
                        Utc::now().format("%Y%m%d_%H%M%S"))
                ))
                .json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %auth.user.id,
                error = %e,
                "Failed to export heart rate data"
            );
            Ok(HttpResponse::InternalServerError().json(
                ApiResponse::<()>::error("Failed to export heart rate data".to_string())
            ))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ActivityAnalytics {
    pub date: String,
    pub steps: i32,
    pub distance_km: f64,
    pub calories: f64,
    pub active_minutes: i32,
    pub step_goal_percentage: f32,
}

async fn export_activity_analytics(
    pool: &PgPool,
    auth: AuthContext,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<Vec<ActivityAnalytics>, sqlx::Error> {
    let records = sqlx::query_as!(
        ActivityRecord,
        r#"
        SELECT user_id, date, steps, distance_meters, calories_burned, 
               active_minutes, flights_climbed, source, raw_data, created_at, updated_at
        FROM activity_metrics 
        WHERE user_id = $1 AND recorded_date BETWEEN $2 AND $3
        ORDER BY recorded_date ASC
        "#,
        auth.user.id,
        start_date.date_naive(),
        end_date.date_naive()
    )
    .fetch_all(pool)
    .await?;

    let analytics: Vec<ActivityAnalytics> = records
        .into_iter()
        .map(|record| {
            let steps = record.steps.unwrap_or(0);
            let step_goal_percentage = if steps > 0 { 
                (steps as f32 / 10000.0) * 100.0  // Assuming 10k step goal
            } else { 
                0.0 
            };

            ActivityAnalytics {
                date: record.date.format("%Y-%m-%d").to_string(),
                steps,
                distance_km: record.distance_meters.unwrap_or(0.0) / 1000.0,
                calories: record.calories_burned.unwrap_or(0.0),
                active_minutes: record.active_minutes.unwrap_or(0),
                step_goal_percentage,
            }
        })
        .collect();

    Ok(analytics)
}

// Data fetching helper functions

async fn fetch_heart_rate_data(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    _include_raw: bool,
) -> Result<Vec<HeartRateRecord>, sqlx::Error> {
    sqlx::query_as!(
        HeartRateRecord,
        r#"
        SELECT user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device, metadata, created_at
        FROM heart_rate_metrics 
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        ORDER BY recorded_at ASC
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
}

async fn fetch_blood_pressure_data(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    _include_raw: bool,
) -> Result<Vec<BloodPressureRecord>, sqlx::Error> {
    sqlx::query_as!(
        BloodPressureRecord,
        r#"
        SELECT user_id, recorded_at, systolic, diastolic, pulse, source_device, metadata, created_at
        FROM blood_pressure_metrics 
        WHERE user_id = $1 AND recorded_at BETWEEN $2 AND $3
        ORDER BY recorded_at ASC
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
}

async fn fetch_sleep_data(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    _include_raw: bool,
) -> Result<Vec<SleepRecord>, sqlx::Error> {
    sqlx::query_as!(
        SleepRecord,
        r#"
        SELECT user_id, sleep_start, sleep_end, duration_minutes,
               deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, sleep_efficiency,
               source_device, metadata, created_at
        FROM sleep_metrics 
        WHERE user_id = $1 AND sleep_start BETWEEN $2 AND $3
        ORDER BY sleep_start ASC
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
}

async fn fetch_activity_data(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    _include_raw: bool,
) -> Result<Vec<ActivityRecord>, sqlx::Error> {
    sqlx::query_as!(
        ActivityRecord,
        r#"
        SELECT user_id, recorded_date, steps, distance_meters, calories_burned,
               active_minutes, flights_climbed, source_device, metadata, created_at
        FROM activity_metrics 
        WHERE user_id = $1 AND recorded_date BETWEEN $2 AND $3
        ORDER BY recorded_date ASC
        "#,
        user_id,
        start_date.date_naive(),
        end_date.date_naive()
    )
    .fetch_all(pool)
    .await
}

async fn fetch_workout_data(
    pool: &PgPool,
    user_id: Uuid,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    _include_raw: bool,
) -> Result<Vec<WorkoutRecord>, sqlx::Error> {
    sqlx::query_as!(
        WorkoutRecord,
        r#"
        SELECT id, user_id, workout_type, started_at, ended_at, distance_meters,
               average_heart_rate, max_heart_rate, total_energy_kcal, active_energy_kcal,
               step_count, duration_seconds, route_geometry, source_device, metadata, created_at
        FROM workouts 
        WHERE user_id = $1 AND started_at BETWEEN $2 AND $3
        ORDER BY started_at ASC
        "#,
        user_id,
        start_date,
        end_date
    )
    .fetch_all(pool)
    .await
}