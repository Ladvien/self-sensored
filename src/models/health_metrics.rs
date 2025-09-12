use crate::config::ValidationConfig;
use crate::models::enums::{ActivityContext, WorkoutType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Heart rate metric with validation
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct HeartRateMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub heart_rate: Option<i16>,
    pub resting_heart_rate: Option<i16>,
    pub heart_rate_variability: Option<f64>,
    pub source_device: Option<String>,
    pub context: Option<ActivityContext>,
    pub created_at: DateTime<Utc>,
}

/// Blood pressure metric
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct BloodPressureMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub systolic: i16,
    pub diastolic: i16,
    pub pulse: Option<i16>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Sleep metrics
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct SleepMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
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

/// Activity metrics (simplified schema matching database)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct ActivityMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub step_count: Option<i32>,
    pub distance_meters: Option<f64>,
    pub flights_climbed: Option<i32>,
    pub active_energy_burned_kcal: Option<f64>,
    pub basal_energy_burned_kcal: Option<f64>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}


/// GPS coordinate for workout routes
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GpsCoordinate {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: Option<f64>,
    pub recorded_at: DateTime<Utc>,
}

/// Workout data matching workouts table schema
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct WorkoutData {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub workout_type: WorkoutType,
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

impl GpsCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if !(config.latitude_min..=config.latitude_max).contains(&self.latitude) {
            return Err(format!(
                "latitude {} is out of range ({} to {})",
                self.latitude, config.latitude_min, config.latitude_max
            ));
        }
        if !(config.longitude_min..=config.longitude_max).contains(&self.longitude) {
            return Err(format!(
                "longitude {} is out of range ({} to {})",
                self.longitude, config.longitude_min, config.longitude_max
            ));
        }
        Ok(())
    }

    /// Convert to PostGIS POINT string
    pub fn to_postgis_point(&self) -> String {
        format!("POINT({} {})", self.longitude, self.latitude)
    }
}

impl WorkoutData {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if self.ended_at <= self.started_at {
            return Err("ended_at must be after started_at".to_string());
        }

        // Check workout duration against configured maximum
        let duration_hours = (self.ended_at - self.started_at).num_hours();
        if duration_hours > config.workout_max_duration_hours {
            return Err(format!(
                "workout duration {} hours exceeds maximum of {} hours",
                duration_hours, config.workout_max_duration_hours
            ));
        }

        if let Some(energy) = self.total_energy_kcal {
            if energy < 0.0 {
                return Err("total_energy_kcal cannot be negative".to_string());
            }
            if energy > config.calories_max {
                return Err(format!(
                    "total_energy_kcal {} exceeds maximum of {}",
                    energy, config.calories_max
                ));
            }
        }

        if let Some(energy) = self.active_energy_kcal {
            if energy < 0.0 {
                return Err("active_energy_kcal cannot be negative".to_string());
            }
            if energy > config.calories_max {
                return Err(format!(
                    "active_energy_kcal {} exceeds maximum of {}",
                    energy, config.calories_max
                ));
            }
        }

        if let Some(distance) = self.distance_meters {
            if distance < 0.0 {
                return Err("distance_meters cannot be negative".to_string());
            }
            let distance_km = distance / 1000.0;
            if distance_km > config.distance_max_km {
                return Err(format!(
                    "distance {} km exceeds maximum of {} km",
                    distance_km, config.distance_max_km
                ));
            }
        }

        if let Some(hr) = self.avg_heart_rate {
            let hr_i16 = hr as i16;
            if !(config.workout_heart_rate_min..=config.workout_heart_rate_max).contains(&hr_i16) {
                return Err(format!(
                    "avg_heart_rate {} is out of range ({}-{})",
                    hr, config.workout_heart_rate_min, config.workout_heart_rate_max
                ));
            }
        }

        if let Some(hr) = self.max_heart_rate {
            let hr_i16 = hr as i16;
            if !(config.workout_heart_rate_min..=config.workout_heart_rate_max).contains(&hr_i16) {
                return Err(format!(
                    "max_heart_rate {} is out of range ({}-{})",
                    hr, config.workout_heart_rate_min, config.workout_heart_rate_max
                ));
            }
        }

        Ok(())
    }


    /// Calculate duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        (self.ended_at - self.started_at).num_seconds()
    }
}

/// Tagged union for all health metric types
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum HealthMetric {
    HeartRate(HeartRateMetric),
    BloodPressure(BloodPressureMetric),
    Sleep(SleepMetric),
    Activity(ActivityMetric),
    Workout(WorkoutData),
}

/// Main ingest payload structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IngestPayload {
    pub data: IngestData,
}

/// Container for all health data in a single request
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IngestData {
    pub metrics: Vec<HealthMetric>,
    pub workouts: Vec<WorkoutData>,
}

/// Response from ingest endpoint
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<ProcessingError>,
}

/// Individual processing error
#[derive(Debug, Serialize, Clone)]
pub struct ProcessingError {
    pub metric_type: String,
    pub error_message: String,
    pub index: Option<usize>,
}

/// Validation functions
impl HeartRateMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if let Some(bpm) = self.heart_rate {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&bpm) {
                return Err(format!(
                    "heart_rate {} is out of range ({}-{})",
                    bpm, config.heart_rate_min, config.heart_rate_max
                ));
            }
        }
        if let Some(bpm) = self.resting_heart_rate {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&bpm) {
                return Err(format!(
                    "resting_heart_rate {} is out of range ({}-{})",
                    bpm, config.heart_rate_min, config.heart_rate_max
                ));
            }
        }
        if let Some(hrv) = self.heart_rate_variability {
            if hrv < 0.0 || hrv > 500.0 {
                return Err(format!(
                    "heart_rate_variability {} is out of range (0-500)",
                    hrv
                ));
            }
        }
        Ok(())
    }
}

impl BloodPressureMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Medical ranges as specified in story requirements
        if self.systolic < config.systolic_min || self.systolic > config.systolic_max {
            return Err(format!(
                "systolic {} is out of range ({}-{})",
                self.systolic, config.systolic_min, config.systolic_max
            ));
        }
        if self.diastolic < config.diastolic_min || self.diastolic > config.diastolic_max {
            return Err(format!(
                "diastolic {} is out of range ({}-{})",
                self.diastolic, config.diastolic_min, config.diastolic_max
            ));
        }

        // Validate systolic is higher than diastolic (basic medical check)
        if self.systolic <= self.diastolic {
            return Err(format!(
                "systolic {} must be higher than diastolic {}",
                self.systolic, self.diastolic
            ));
        }

        if let Some(pulse) = self.pulse {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&pulse) {
                return Err(format!(
                    "pulse {} is out of range ({}-{})",
                    pulse, config.heart_rate_min, config.heart_rate_max
                ));
            }
        }
        Ok(())
    }
}

impl SleepMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if self.sleep_end <= self.sleep_start {
            return Err("sleep_end must be after sleep_start".to_string());
        }

        let calculated_duration = (self.sleep_end - self.sleep_start).num_minutes() as i32;
        if let Some(duration) = self.duration_minutes {
            if (duration - calculated_duration).abs() > config.sleep_duration_tolerance_minutes {
                return Err(format!(
                    "duration_minutes doesn't match sleep duration (tolerance: {} minutes)",
                    config.sleep_duration_tolerance_minutes
                ));
            }
        }

        if let Some(eff) = self.efficiency {
            if !(config.sleep_efficiency_min..=config.sleep_efficiency_max).contains(&(eff as f32)) {
                return Err(format!(
                    "efficiency {} is out of range ({}-{})",
                    eff, config.sleep_efficiency_min, config.sleep_efficiency_max
                ));
            }
        }

        // Validate sleep component totals don't exceed total sleep time
        let component_total = self.deep_sleep_minutes.unwrap_or(0)
            + self.rem_sleep_minutes.unwrap_or(0)
            + self.awake_minutes.unwrap_or(0);

        if component_total > calculated_duration {
            return Err(format!(
                "Sleep components total ({} minutes) exceeds sleep duration ({} minutes)",
                component_total, calculated_duration
            ));
        }

        Ok(())
    }

    /// Calculate sleep efficiency based on sleep components
    pub fn calculate_efficiency(&self) -> f32 {
        let total_duration = (self.sleep_end - self.sleep_start).num_minutes() as f32;
        if total_duration <= 0.0 {
            return 0.0;
        }

        // Efficiency = (actual sleep time / time in bed) * 100
        let actual_sleep = self.duration_minutes.unwrap_or(total_duration as i32) as f32;
        (actual_sleep / total_duration * 100.0).min(100.0).max(0.0)
    }

    /// Get the efficiency, calculating if not provided
    pub fn get_efficiency(&self) -> f32 {
        self.efficiency
            .map(|e| e as f32)
            .unwrap_or_else(|| self.calculate_efficiency())
    }
}

impl ActivityMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if let Some(step_count) = self.step_count {
            if step_count < config.step_count_min || step_count > config.step_count_max {
                return Err(format!(
                    "step_count {} is out of range ({}-{})",
                    step_count, config.step_count_min, config.step_count_max
                ));
            }
        }
        if let Some(distance) = self.distance_meters {
            if distance < 0.0 {
                return Err("distance_meters cannot be negative".to_string());
            }
            let distance_km = distance / 1000.0;
            if distance_km > config.distance_max_km {
                return Err(format!(
                    "distance {} km exceeds maximum of {} km",
                    distance_km, config.distance_max_km
                ));
            }
        }
        if let Some(active_energy) = self.active_energy_burned_kcal {
            if active_energy < 0.0 {
                return Err("active_energy_burned_kcal cannot be negative".to_string());
            }
            if active_energy > config.calories_max {
                return Err(format!(
                    "active_energy_burned_kcal {} exceeds maximum of {}",
                    active_energy, config.calories_max
                ));
            }
        }
        if let Some(basal_energy) = self.basal_energy_burned_kcal {
            if basal_energy < 0.0 {
                return Err("basal_energy_burned_kcal cannot be negative".to_string());
            }
            if basal_energy > config.calories_max {
                return Err(format!(
                    "basal_energy_burned_kcal {} exceeds maximum of {}",
                    basal_energy, config.calories_max
                ));
            }
        }
        if let Some(flights) = self.flights_climbed {
            if flights < 0 || flights > 10000 {
                return Err(format!(
                    "flights_climbed {} is out of range (0-10000)",
                    flights
                ));
            }
        }
        Ok(())
    }
}


impl HealthMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        match self {
            HealthMetric::HeartRate(metric) => metric.validate_with_config(config),
            HealthMetric::BloodPressure(metric) => metric.validate_with_config(config),
            HealthMetric::Sleep(metric) => metric.validate_with_config(config),
            HealthMetric::Activity(metric) => metric.validate_with_config(config),
            HealthMetric::Workout(workout) => workout.validate_with_config(config),
        }
    }

    pub fn metric_type(&self) -> &'static str {
        match self {
            HealthMetric::HeartRate(_) => "HeartRate",
            HealthMetric::BloodPressure(_) => "BloodPressure",
            HealthMetric::Sleep(_) => "Sleep",
            HealthMetric::Activity(_) => "Activity",
            HealthMetric::Workout(_) => "Workout",
        }
    }
}












