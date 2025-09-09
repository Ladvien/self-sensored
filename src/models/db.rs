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
    pub heart_rate: i32,
    pub resting_heart_rate: Option<i32>,
    pub context: Option<String>,
    pub source_device: Option<String>,
    pub raw_data: Option<serde_json::Value>, // Store original JSON for debugging
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Blood pressure database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BloodPressureRecord {
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub systolic: i32,
    pub diastolic: i32,
    pub pulse: Option<i32>,
    pub source_device: Option<String>,
    pub raw_data: Option<serde_json::Value>, // Store original JSON for debugging
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Sleep database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SleepRecord {
    pub user_id: Uuid,
    pub sleep_start: DateTime<Utc>,
    pub sleep_end: DateTime<Utc>,
    pub duration_minutes: i32,
    pub deep_sleep_minutes: Option<i32>,
    pub rem_sleep_minutes: Option<i32>,
    pub light_sleep_minutes: Option<i32>,
    pub awake_minutes: Option<i32>,
    pub sleep_efficiency: Option<sqlx::types::BigDecimal>,
    pub source_device: Option<String>,
    pub raw_data: Option<serde_json::Value>, // Store original JSON for debugging
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Activity database model  
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActivityRecord {
    pub user_id: Uuid,
    pub recorded_date: chrono::NaiveDate,
    pub steps: Option<i32>,
    pub distance_meters: Option<sqlx::types::BigDecimal>,
    pub calories_burned: Option<i32>,
    pub active_minutes: Option<i32>,
    pub flights_climbed: Option<i32>,
    pub source_device: Option<String>,
    pub raw_data: Option<serde_json::Value>, // Store original JSON for debugging
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>, // Track aggregation updates
}

/// Workout database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct WorkoutRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workout_type: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub distance_meters: Option<sqlx::types::BigDecimal>,
    pub average_heart_rate: Option<i32>,
    pub max_heart_rate: Option<i32>,
    pub total_energy_kcal: Option<sqlx::types::BigDecimal>,
    pub active_energy_kcal: Option<sqlx::types::BigDecimal>,
    pub step_count: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub route_geometry: Option<String>, // PostGIS geometry as WKT/LINESTRING
    pub source_device: Option<String>,
    pub raw_data: Option<serde_json::Value>, // Store original JSON for debugging
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Workout route points (separate table for detailed GPS data)
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct WorkoutRoutePoint {
    pub id: i64,
    pub workout_id: Uuid,
    pub point_order: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: Option<sqlx::types::BigDecimal>,
    pub recorded_at: DateTime<Utc>,
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

/// Enhanced conversion functions with raw JSON preservation
impl HeartRateRecord {
    pub fn from_metric_with_raw(metric: crate::models::health_metrics::HeartRateMetric, raw_json: serde_json::Value) -> Self {
        HeartRateRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            heart_rate: metric.avg_bpm.unwrap_or(70) as i32,
            resting_heart_rate: metric.min_bpm.map(|v| v as i32),
            context: metric.context,
            source_device: metric.source,
            raw_data: Some(raw_json),
            metadata: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::HeartRateMetric> for HeartRateRecord {
    fn from(metric: crate::models::health_metrics::HeartRateMetric) -> Self {
        HeartRateRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            heart_rate: metric.avg_bpm.unwrap_or(70) as i32,
            resting_heart_rate: metric.min_bpm.map(|v| v as i32),
            context: metric.context,
            source_device: metric.source,
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::BloodPressureMetric> for BloodPressureRecord {
    fn from(metric: crate::models::health_metrics::BloodPressureMetric) -> Self {
        BloodPressureRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_at: metric.recorded_at,
            systolic: metric.systolic as i32,
            diastolic: metric.diastolic as i32,
            pulse: metric.pulse.map(|v| v as i32),
            source_device: metric.source,
            metadata: None,
            created_at: Utc::now(),
        }
    }
}

impl SleepRecord {
    pub fn from_metric_with_raw(metric: crate::models::health_metrics::SleepMetric, raw_json: serde_json::Value) -> Self {
        let efficiency = metric.get_efficiency_percentage();
        
        SleepRecord {
            user_id: Uuid::nil(), // Will be set by caller
            sleep_start: metric.sleep_start,
            sleep_end: metric.sleep_end,
            duration_minutes: metric.total_sleep_minutes,
            deep_sleep_minutes: metric.deep_sleep_minutes,
            rem_sleep_minutes: metric.rem_sleep_minutes,
            light_sleep_minutes: None, // Not provided by API model
            awake_minutes: metric.awake_minutes,
            sleep_efficiency: Some(sqlx::types::BigDecimal::from_str(&efficiency.to_string()).unwrap()),
            source_device: metric.source,
            raw_data: Some(raw_json),
            metadata: None,
            created_at: Utc::now(),
        }
    }
}

impl From<crate::models::health_metrics::SleepMetric> for SleepRecord {
    fn from(metric: crate::models::health_metrics::SleepMetric) -> Self {
        let efficiency = metric.get_efficiency_percentage();
        
        SleepRecord {
            user_id: Uuid::nil(), // Will be set by caller
            sleep_start: metric.sleep_start,
            sleep_end: metric.sleep_end,
            duration_minutes: metric.total_sleep_minutes,
            deep_sleep_minutes: metric.deep_sleep_minutes,
            rem_sleep_minutes: metric.rem_sleep_minutes,
            light_sleep_minutes: None, // Not provided by API model
            awake_minutes: metric.awake_minutes,
            sleep_efficiency: Some(sqlx::types::BigDecimal::from_str(&efficiency.to_string()).unwrap()),
            source_device: metric.source,
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }
}

impl ActivityRecord {
    pub fn from_metric_with_raw(metric: crate::models::health_metrics::ActivityMetric, raw_json: serde_json::Value) -> Self {
        let now = Utc::now();
        ActivityRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_date: metric.date,
            steps: metric.steps,
            distance_meters: metric.distance_meters.map(|v| sqlx::types::BigDecimal::from_str(&v.to_string()).unwrap()),
            calories_burned: metric.calories_burned.map(|v| v as i32),
            active_minutes: metric.active_minutes,
            flights_climbed: metric.flights_climbed,
            source_device: metric.source,
            raw_data: Some(raw_json),
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Aggregate this activity record with another (for daily summation)
    pub fn aggregate_with(&mut self, other: &ActivityRecord) {
        self.steps = match (self.steps, other.steps) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.distance_meters = match (&self.distance_meters, &other.distance_meters) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        self.calories_burned = match (self.calories_burned, other.calories_burned) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.active_minutes = match (self.active_minutes, other.active_minutes) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.flights_climbed = match (self.flights_climbed, other.flights_climbed) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.updated_at = Utc::now();
    }
}

impl From<crate::models::health_metrics::ActivityMetric> for ActivityRecord {
    fn from(metric: crate::models::health_metrics::ActivityMetric) -> Self {
        let now = Utc::now();
        ActivityRecord {
            user_id: Uuid::nil(), // Will be set by caller
            recorded_date: metric.date,
            steps: metric.steps,
            distance_meters: metric.distance_meters.map(|v| sqlx::types::BigDecimal::from_str(&v.to_string()).unwrap()),
            calories_burned: metric.calories_burned.map(|v| v as i32),
            active_minutes: metric.active_minutes,
            flights_climbed: metric.flights_climbed,
            source_device: metric.source,
            raw_data: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl WorkoutRecord {
    pub fn from_workout_with_raw(workout: crate::models::health_metrics::WorkoutData, raw_json: serde_json::Value) -> Self {
        let duration_seconds = Some(workout.duration_seconds() as i32);
        let route_geometry = workout.route_to_linestring();
        
        WorkoutRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(), // Will be set by caller
            workout_type: workout.workout_type,
            started_at: workout.start_time,
            ended_at: workout.end_time,
            distance_meters: workout.distance_meters.map(|v| sqlx::types::BigDecimal::from(v)),
            average_heart_rate: workout.avg_heart_rate.map(|v| v as i32),
            max_heart_rate: workout.max_heart_rate.map(|v| v as i32),
            total_energy_kcal: workout.total_energy_kcal.map(|v| sqlx::types::BigDecimal::from_str(&v.to_string()).unwrap()),
            active_energy_kcal: None,  // Not provided by API model
            step_count: None,  // Not provided by API model
            duration_seconds,
            route_geometry,
            source_device: workout.source,
            raw_data: Some(raw_json),
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Convert route points to detailed route point records
    pub fn route_points(&self, workout_data: &crate::models::health_metrics::WorkoutData) -> Vec<WorkoutRoutePoint> {
        if let Some(route_points) = &workout_data.route_points {
            route_points.iter().enumerate().map(|(i, point)| {
                WorkoutRoutePoint {
                    id: 0, // Will be set by database
                    workout_id: self.id,
                    point_order: i as i32,
                    latitude: point.latitude,
                    longitude: point.longitude,
                    altitude_meters: point.altitude_meters.map(|v| sqlx::types::BigDecimal::from_str(&v.to_string()).unwrap()),
                    recorded_at: point.recorded_at,
                    created_at: Utc::now(),
                }
            }).collect()
        } else {
            Vec::new()
        }
    }
}

impl From<crate::models::health_metrics::WorkoutData> for WorkoutRecord {
    fn from(workout: crate::models::health_metrics::WorkoutData) -> Self {
        let duration_seconds = Some(workout.duration_seconds() as i32);
        let route_geometry = workout.route_to_linestring();
        
        WorkoutRecord {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(), // Will be set by caller
            workout_type: workout.workout_type,
            started_at: workout.start_time,
            ended_at: workout.end_time,
            distance_meters: workout.distance_meters.map(|v| sqlx::types::BigDecimal::from(v)),
            average_heart_rate: workout.avg_heart_rate.map(|v| v as i32),
            max_heart_rate: workout.max_heart_rate.map(|v| v as i32),
            total_energy_kcal: workout.total_energy_kcal.map(|v| sqlx::types::BigDecimal::from_str(&v.to_string()).unwrap()),
            active_energy_kcal: None,  // Not provided by API model
            step_count: None,  // Not provided by API model
            duration_seconds,
            route_geometry,
            source_device: workout.source,
            raw_data: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }
}
