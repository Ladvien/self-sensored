use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub apple_health_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
}

/// API Key database model
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub permissions: serde_json::Value,
    pub rate_limit_per_hour: Option<i32>,
}

/// Raw ingestion record for backup
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RawIngestion {
    pub id: Uuid,
    pub user_id: Uuid,
    pub processing_job_id: Option<Uuid>,
    pub payload_hash: String,
    pub payload_size_bytes: i32,
    pub raw_payload: serde_json::Value,
    pub processing_status: Option<String>,
    pub processing_errors: Option<serde_json::Value>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Heart rate database model
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct HeartRateRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub heart_rate: Option<i32>,
    pub resting_heart_rate: Option<i32>,
    pub heart_rate_variability: Option<f64>,
    pub context: Option<String>, // activity_context enum as string
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Blood pressure database model
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BloodPressureRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub systolic: i32,
    pub diastolic: i32,
    pub pulse: Option<i32>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Sleep database model
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SleepRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub sleep_start: DateTime<Utc>,
    pub sleep_end: DateTime<Utc>,
    pub duration_minutes: Option<i32>,
    pub deep_sleep_minutes: Option<i32>,
    pub rem_sleep_minutes: Option<i32>,
    pub light_sleep_minutes: Option<i32>,
    pub awake_minutes: Option<i32>,
    pub efficiency: Option<f64>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Activity database model  
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ActivityRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recorded_at: DateTime<Utc>,
    pub step_count: Option<i32>,
    pub distance_meters: Option<f64>,
    pub flights_climbed: Option<i32>,
    pub active_energy_burned_kcal: Option<f64>,
    pub basal_energy_burned_kcal: Option<f64>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Workout database model
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct WorkoutRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub workout_type: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub total_energy_kcal: Option<f64>,
    pub active_energy_kcal: Option<f64>,
    pub distance_meters: Option<f64>,
    pub avg_heart_rate: Option<i32>,
    pub max_heart_rate: Option<i32>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Enhanced conversion functions with raw JSON preservation
impl HeartRateRecord {
    pub fn from_metric_with_raw(
        metric: crate::models::health_metrics::HeartRateMetric,
        _raw_json: serde_json::Value,
    ) -> Self {
        HeartRateRecord {
            id: metric.id,
            user_id: metric.user_id,
            recorded_at: metric.recorded_at,
            heart_rate: metric.heart_rate.map(|v| v as i32),
            resting_heart_rate: metric.resting_heart_rate.map(|v| v as i32),
            heart_rate_variability: None, // Not provided in current metric
            context: metric.context.map(|c| c.to_string()),
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl From<crate::models::health_metrics::HeartRateMetric> for HeartRateRecord {
    fn from(metric: crate::models::health_metrics::HeartRateMetric) -> Self {
        HeartRateRecord {
            id: metric.id,
            user_id: metric.user_id,
            recorded_at: metric.recorded_at,
            heart_rate: metric.heart_rate.map(|v| v as i32),
            resting_heart_rate: metric.resting_heart_rate.map(|v| v as i32),
            heart_rate_variability: None,
            context: metric.context.map(|c| c.to_string()),
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl From<crate::models::health_metrics::BloodPressureMetric> for BloodPressureRecord {
    fn from(metric: crate::models::health_metrics::BloodPressureMetric) -> Self {
        BloodPressureRecord {
            id: metric.id,
            user_id: metric.user_id,
            recorded_at: metric.recorded_at,
            systolic: metric.systolic as i32,
            diastolic: metric.diastolic as i32,
            pulse: metric.pulse.map(|v| v as i32),
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl SleepRecord {
    pub fn from_metric_with_raw(
        metric: crate::models::health_metrics::SleepMetric,
        _raw_json: serde_json::Value,
    ) -> Self {
        SleepRecord {
            id: metric.id,
            user_id: metric.user_id,
            sleep_start: metric.sleep_start,
            sleep_end: metric.sleep_end,
            duration_minutes: metric.duration_minutes,
            deep_sleep_minutes: metric.deep_sleep_minutes,
            rem_sleep_minutes: metric.rem_sleep_minutes,
            light_sleep_minutes: None, // Not provided by API model
            awake_minutes: metric.awake_minutes,
            efficiency: metric.efficiency.map(|e| e as f64),
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl From<crate::models::health_metrics::SleepMetric> for SleepRecord {
    fn from(metric: crate::models::health_metrics::SleepMetric) -> Self {
        SleepRecord {
            id: metric.id,
            user_id: metric.user_id,
            sleep_start: metric.sleep_start,
            sleep_end: metric.sleep_end,
            duration_minutes: metric.duration_minutes,
            deep_sleep_minutes: metric.deep_sleep_minutes,
            rem_sleep_minutes: metric.rem_sleep_minutes,
            light_sleep_minutes: None, // Not provided by API model
            awake_minutes: metric.awake_minutes,
            efficiency: metric.efficiency.map(|e| e as f64),
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl ActivityRecord {
    pub fn from_metric_with_raw(
        metric: crate::models::health_metrics::ActivityMetric,
        _raw_json: serde_json::Value,
    ) -> Self {
        ActivityRecord {
            id: metric.id,
            user_id: metric.user_id,
            recorded_at: metric.recorded_at,
            step_count: metric.step_count,
            distance_meters: metric.distance_meters,
            flights_climbed: metric.flights_climbed,
            active_energy_burned_kcal: metric.active_energy_burned_kcal,
            basal_energy_burned_kcal: metric.basal_energy_burned_kcal,
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }

    /// Aggregate this activity record with another (for daily summation)
    pub fn aggregate_with(&mut self, other: &ActivityRecord) {
        self.step_count = match (self.step_count, other.step_count) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.distance_meters = match (self.distance_meters, other.distance_meters) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.active_energy_burned_kcal = match (
            self.active_energy_burned_kcal,
            other.active_energy_burned_kcal,
        ) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.basal_energy_burned_kcal = match (
            self.basal_energy_burned_kcal,
            other.basal_energy_burned_kcal,
        ) {
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

        // Update the recorded_at to the latest time
        if other.recorded_at > self.recorded_at {
            self.recorded_at = other.recorded_at;
        }
    }
}

impl From<crate::models::health_metrics::ActivityMetric> for ActivityRecord {
    fn from(metric: crate::models::health_metrics::ActivityMetric) -> Self {
        ActivityRecord {
            id: metric.id,
            user_id: metric.user_id,
            recorded_at: metric.recorded_at,
            step_count: metric.step_count,
            distance_meters: metric.distance_meters,
            flights_climbed: metric.flights_climbed,
            active_energy_burned_kcal: metric.active_energy_burned_kcal,
            basal_energy_burned_kcal: metric.basal_energy_burned_kcal,
            source_device: metric.source_device,
            created_at: metric.created_at,
        }
    }
}

impl WorkoutRecord {
    pub fn from_workout_with_raw(
        workout: crate::models::health_metrics::WorkoutData,
        _raw_json: serde_json::Value,
    ) -> Self {
        WorkoutRecord {
            id: workout.id,
            user_id: workout.user_id,
            workout_type: workout.workout_type.to_string(),
            started_at: workout.started_at,
            ended_at: workout.ended_at,
            total_energy_kcal: workout.total_energy_kcal,
            active_energy_kcal: workout.active_energy_kcal,
            distance_meters: workout.distance_meters,
            avg_heart_rate: workout.avg_heart_rate.map(|v| v as i32),
            max_heart_rate: workout.max_heart_rate.map(|v| v as i32),
            source_device: workout.source_device,
            created_at: workout.created_at,
        }
    }
}

impl From<crate::models::health_metrics::WorkoutData> for WorkoutRecord {
    fn from(workout: crate::models::health_metrics::WorkoutData) -> Self {
        WorkoutRecord {
            id: workout.id,
            user_id: workout.user_id,
            workout_type: workout.workout_type.to_string(),
            started_at: workout.started_at,
            ended_at: workout.ended_at,
            total_energy_kcal: workout.total_energy_kcal,
            active_energy_kcal: workout.active_energy_kcal,
            distance_meters: workout.distance_meters,
            avg_heart_rate: workout.avg_heart_rate.map(|v| v as i32),
            max_heart_rate: workout.max_heart_rate.map(|v| v as i32),
            source_device: workout.source_device,
            created_at: workout.created_at,
        }
    }
}
