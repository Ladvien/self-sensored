use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Heart rate metric with validation
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HeartRateMetric {
    pub recorded_at: DateTime<Utc>,
    pub min_bpm: Option<i16>,
    pub avg_bpm: Option<i16>,
    pub max_bpm: Option<i16>,
    pub source: Option<String>,
    pub context: Option<String>, // resting, exercise, recovery
}

/// Blood pressure metric
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BloodPressureMetric {
    pub recorded_at: DateTime<Utc>,
    pub systolic: i16,
    pub diastolic: i16,
    pub pulse: Option<i16>,
    pub source: Option<String>,
}

/// Sleep metrics
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SleepMetric {
    pub recorded_at: DateTime<Utc>,
    pub sleep_start: DateTime<Utc>,
    pub sleep_end: DateTime<Utc>,
    pub total_sleep_minutes: i32,
    pub deep_sleep_minutes: Option<i32>,
    pub rem_sleep_minutes: Option<i32>,
    pub awake_minutes: Option<i32>,
    pub efficiency_percentage: Option<f32>,
    pub source: Option<String>,
}

/// Daily activity summary
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityMetric {
    pub date: chrono::NaiveDate,
    pub steps: Option<i32>,
    pub distance_meters: Option<f64>,
    pub calories_burned: Option<f64>,
    pub active_minutes: Option<i32>,
    pub flights_climbed: Option<i32>,
    pub source: Option<String>,
}

/// Workout data with GPS tracking
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkoutData {
    pub workout_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_energy_kcal: Option<f64>,
    pub distance_meters: Option<f64>,
    pub avg_heart_rate: Option<i16>,
    pub max_heart_rate: Option<i16>,
    pub source: Option<String>,
    // GPS route stored separately in PostGIS
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
#[derive(Debug, Deserialize, Serialize)]
pub struct IngestPayload {
    pub data: IngestData,
}

/// Container for all health data in a single request
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct ProcessingError {
    pub metric_type: String,
    pub error_message: String,
    pub index: Option<usize>,
}

/// Validation functions
impl HeartRateMetric {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(bpm) = self.min_bpm {
            if !(20..=300).contains(&bpm) {
                return Err(format!("min_bpm {bpm} is out of range (20-300)"));
            }
        }
        if let Some(bpm) = self.avg_bpm {
            if !(20..=300).contains(&bpm) {
                return Err(format!("avg_bpm {bpm} is out of range (20-300)"));
            }
        }
        if let Some(bpm) = self.max_bpm {
            if !(20..=300).contains(&bpm) {
                return Err(format!("max_bpm {bpm} is out of range (20-300)"));
            }
        }
        Ok(())
    }
}

impl BloodPressureMetric {
    pub fn validate(&self) -> Result<(), String> {
        if self.systolic < 40 || self.systolic > 250 {
            return Err(format!(
                "systolic {} is out of range (40-250)",
                self.systolic
            ));
        }
        if self.diastolic < 30 || self.diastolic > 150 {
            return Err(format!(
                "diastolic {} is out of range (30-150)",
                self.diastolic
            ));
        }
        if let Some(pulse) = self.pulse {
            if !(20..=300).contains(&pulse) {
                return Err(format!("pulse {pulse} is out of range (20-300)"));
            }
        }
        Ok(())
    }
}

impl SleepMetric {
    pub fn validate(&self) -> Result<(), String> {
        if self.sleep_end <= self.sleep_start {
            return Err("sleep_end must be after sleep_start".to_string());
        }

        let calculated_duration = (self.sleep_end - self.sleep_start).num_minutes() as i32;
        if (self.total_sleep_minutes - calculated_duration).abs() > 60 {
            return Err("total_sleep_minutes doesn't match sleep duration".to_string());
        }

        if let Some(efficiency) = self.efficiency_percentage {
            if !(0.0..=100.0).contains(&efficiency) {
                return Err(format!(
                    "efficiency_percentage {efficiency} is out of range (0-100)"
                ));
            }
        }

        Ok(())
    }
}

impl ActivityMetric {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(steps) = self.steps {
            if steps < 0 {
                return Err("steps cannot be negative".to_string());
            }
        }
        if let Some(distance) = self.distance_meters {
            if distance < 0.0 {
                return Err("distance_meters cannot be negative".to_string());
            }
        }
        if let Some(calories) = self.calories_burned {
            if calories < 0.0 {
                return Err("calories_burned cannot be negative".to_string());
            }
        }
        Ok(())
    }
}

impl HealthMetric {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            HealthMetric::HeartRate(metric) => metric.validate(),
            HealthMetric::BloodPressure(metric) => metric.validate(),
            HealthMetric::Sleep(metric) => metric.validate(),
            HealthMetric::Activity(metric) => metric.validate(),
            HealthMetric::Workout(_) => Ok(()), // Workouts have simpler validation
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
