use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

/// API Key database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub scopes: Option<Vec<String>>,
}

/// Raw ingestion record for backup
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RawIngestion {
    pub id: i64,
    pub user_id: Uuid,
    pub api_key_id: Uuid,
    pub payload: serde_json::Value,
    pub payload_hash: String,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub processing_errors: Option<serde_json::Value>,
}

/// Heart rate database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct HeartRateRecord {
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub min_bpm: Option<i16>,
    pub avg_bpm: Option<i16>,
    pub max_bpm: Option<i16>,
    pub context: Option<String>,
    pub source: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Blood pressure database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BloodPressureRecord {
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub systolic: i16,
    pub diastolic: i16,
    pub pulse: Option<i16>,
    pub source: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Sleep database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SleepRecord {
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub sleep_start: DateTime<Utc>,
    pub sleep_end: DateTime<Utc>,
    pub total_sleep_minutes: i32,
    pub deep_sleep_minutes: Option<i32>,
    pub rem_sleep_minutes: Option<i32>,
    pub awake_minutes: Option<i32>,
    pub efficiency_percentage: Option<f32>,
    pub source: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Activity database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActivityRecord {
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub steps: Option<i32>,
    pub distance_meters: Option<f64>,
    pub calories_burned: Option<f64>,
    pub active_minutes: Option<i32>,
    pub flights_climbed: Option<i32>,
    pub source: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workout database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct WorkoutRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workout_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_energy_kcal: Option<f64>,
    pub distance_meters: Option<f64>,
    pub avg_heart_rate: Option<i16>,
    pub max_heart_rate: Option<i16>,
    pub source: Option<String>,
    pub route_geometry: Option<String>, // PostGIS geometry as WKT
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Audit log for security tracking
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub action: String,
    pub resource: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Conversion functions from API models to database models
impl From<crate::models::health_metrics::HeartRateMetric> for HeartRateRecord {
    fn from(metric: crate::models::health_metrics::HeartRateMetric) -> Self {
        HeartRateRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            min_bpm: metric.min_bpm,
            avg_bpm: metric.avg_bpm,
            max_bpm: metric.max_bpm,
            context: metric.context,
            source: metric.source,
            raw_data: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::BloodPressureMetric> for BloodPressureRecord {
    fn from(metric: crate::models::health_metrics::BloodPressureMetric) -> Self {
        BloodPressureRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            systolic: metric.systolic,
            diastolic: metric.diastolic,
            pulse: metric.pulse,
            source: metric.source,
            raw_data: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::SleepMetric> for SleepRecord {
    fn from(metric: crate::models::health_metrics::SleepMetric) -> Self {
        SleepRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            sleep_start: metric.sleep_start,
            sleep_end: metric.sleep_end,
            total_sleep_minutes: metric.total_sleep_minutes,
            deep_sleep_minutes: metric.deep_sleep_minutes,
            rem_sleep_minutes: metric.rem_sleep_minutes,
            awake_minutes: metric.awake_minutes,
            efficiency_percentage: metric.efficiency_percentage,
            source: metric.source,
            raw_data: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::ActivityMetric> for ActivityRecord {
    fn from(metric: crate::models::health_metrics::ActivityMetric) -> Self {
        ActivityRecord {
            user_id: Uuid::nil(), // Will be set by caller
            date: metric.date,
            steps: metric.steps,
            distance_meters: metric.distance_meters,
            calories_burned: metric.calories_burned,
            active_minutes: metric.active_minutes,
            flights_climbed: metric.flights_climbed,
            source: metric.source,
            raw_data: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::WorkoutData> for WorkoutRecord {
    fn from(workout: crate::models::health_metrics::WorkoutData) -> Self {
        WorkoutRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(), // Will be set by caller
            workout_type: workout.workout_type,
            start_time: workout.start_time,
            end_time: workout.end_time,
            total_energy_kcal: workout.total_energy_kcal,
            distance_meters: workout.distance_meters,
            avg_heart_rate: workout.avg_heart_rate,
            max_heart_rate: workout.max_heart_rate,
            source: workout.source,
            route_geometry: None,
            raw_data: None,
            created_at: Utc::now(),
        }
    }
}
