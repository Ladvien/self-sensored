use crate::config::ValidationConfig;
use crate::models::enums::{
    ActivityContext, WorkoutType, MeditationType, StateOfMind,
    MenstrualFlow, CervicalMucusQuality, OvulationTestResult,
    PregnancyTestResult, TemperatureContext,
};
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

/// Body measurements metric matching body_metrics table schema
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct BodyMeasurementMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,

    // Weight & Composition
    pub body_weight_kg: Option<f64>,
    pub body_mass_index: Option<f64>,
    pub body_fat_percentage: Option<f64>,
    pub lean_body_mass_kg: Option<f64>,

    // Measurements
    pub waist_circumference_cm: Option<f64>,
    pub hip_circumference_cm: Option<f64>,
    pub chest_circumference_cm: Option<f64>,
    pub arm_circumference_cm: Option<f64>,
    pub thigh_circumference_cm: Option<f64>,

    // Body Temperature
    pub body_temperature_celsius: Option<f64>,
    pub basal_body_temperature_celsius: Option<f64>,

    // Metadata
    pub measurement_source: Option<String>, // manual, smart_scale, apple_watch
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Respiratory metrics for critical health monitoring
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct RespiratoryMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub respiratory_rate: Option<i32>,              // breaths per minute (12-20 normal)
    pub oxygen_saturation: Option<f64>,             // SpO2 percentage (90-100% normal, <90% critical)
    pub forced_vital_capacity: Option<f64>,         // FVC in liters (3-5L normal)
    pub forced_expiratory_volume_1: Option<f64>,    // FEV1 in liters (age/gender specific)
    pub peak_expiratory_flow_rate: Option<f64>,     // PEFR in L/min (300-600 normal)
    pub inhaler_usage: Option<i32>,                 // count of inhaler uses/puffs
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Blood glucose context for medical data interpretation
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum GlucoseContext {
    Fasting,
    PostMeal,
    Random,
    BedroomTime,
    PreMeal,
    PostWorkout,
}

/// Blood glucose metric for CGM data streams and diabetes management
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct BloodGlucoseMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub blood_glucose_mg_dl: f64,                       // Blood glucose in mg/dL (70-180 normal, diabetic ranges vary)
    pub measurement_context: Option<String>,            // Store as string in DB: "fasting", "post_meal", etc.
    pub medication_taken: Option<bool>,                 // Whether diabetes medication was taken
    pub insulin_delivery_units: Option<f64>,           // Insulin delivery units (for atomic pairing)
    pub glucose_source: Option<String>,                // CGM device identifier for deduplication
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

/// Temperature metrics (body temperature, fertility tracking, environmental)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct TemperatureMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub body_temperature: Option<f64>,
    pub basal_body_temperature: Option<f64>,
    pub apple_sleeping_wrist_temperature: Option<f64>,
    pub water_temperature: Option<f64>,
    pub temperature_source: Option<String>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
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

/// Environmental metrics for tracking environmental health factors
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct EnvironmentalMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub uv_index: Option<f64>,
    pub uv_exposure_minutes: Option<i32>,
    pub time_in_daylight_minutes: Option<i32>,
    pub ambient_temperature_celsius: Option<f64>,
    pub humidity_percent: Option<f64>,
    pub air_pressure_hpa: Option<f64>,
    pub altitude_meters: Option<f64>,
    pub location_latitude: Option<f64>,
    pub location_longitude: Option<f64>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Audio exposure metrics for hearing health monitoring
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct AudioExposureMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub environmental_audio_exposure_db: Option<f64>,
    pub headphone_audio_exposure_db: Option<f64>,
    pub exposure_duration_minutes: i32,
    pub audio_exposure_event: bool, // true if dangerous level detected
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Safety event metrics for fall detection and safety monitoring
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct SafetyEventMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub event_type: String, // "fall_detected", "emergency_sos", etc.
    pub severity_level: Option<i16>, // 1-5 severity scale
    pub location_latitude: Option<f64>,
    pub location_longitude: Option<f64>,
    pub emergency_contacts_notified: bool,
    pub resolution_status: Option<String>, // "pending", "resolved", "dismissed"
    pub notes: Option<String>,
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Mindfulness session metrics for meditation and mental wellness tracking
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct MindfulnessMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,

    // Core Session Data
    pub session_duration_minutes: Option<i32>,
    pub meditation_type: Option<String>, // Store as string in DB, convert to/from enum
    pub session_quality_rating: Option<i16>, // 1-5 rating

    // Mindful Minutes Tracking
    pub mindful_minutes_today: Option<i32>,
    pub mindful_minutes_week: Option<i32>,

    // Physiological Data During Session
    pub breathing_rate_breaths_per_min: Option<f64>,
    pub heart_rate_variability_during_session: Option<f64>,
    pub focus_rating: Option<i16>, // 1-10 focus rating

    // Session Context
    pub guided_session_instructor: Option<String>,
    pub meditation_app: Option<String>, // calm, headspace, insight_timer, apple_mindfulness
    pub background_sounds: Option<String>, // nature, rain, silence, music
    pub location_type: Option<String>, // home, office, outdoors, studio
    pub session_notes: Option<String>,

    // Metadata
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Mental health metrics for mood tracking and psychological wellness monitoring
/// Includes special privacy protections for sensitive mental health data
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct MentalHealthMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,

    // iOS 17+ State of Mind Integration
    pub state_of_mind_valence: Option<f64>, // -1.0 (very unpleasant) to 1.0 (very pleasant)
    pub state_of_mind_labels: Option<Vec<String>>, // array of mood descriptors
    pub reflection_prompt: Option<String>,

    // General Mental Health Ratings (1-10 scale)
    pub mood_rating: Option<i16>, // Overall mood
    pub anxiety_level: Option<i16>, // Anxiety intensity
    pub stress_level: Option<i16>, // Stress intensity
    pub energy_level: Option<i16>, // Energy level

    // Clinical Screening Scores (when applicable)
    pub depression_screening_score: Option<i16>, // PHQ-9 style (0-27)
    pub anxiety_screening_score: Option<i16>, // GAD-7 style (0-21)

    // Sleep Quality Impact
    pub sleep_quality_impact: Option<i16>, // 1-5 impact rating

    // Context and Coping
    pub trigger_event: Option<String>, // work_stress, relationship, health, financial
    pub coping_strategy: Option<String>, // exercise, meditation, social_support, therapy
    pub medication_taken: Option<bool>,
    pub therapy_session_today: Option<bool>,

    // Privacy Protected Data
    pub private_notes_encrypted: Option<String>, // Encrypted mental health notes
    pub notes_encryption_key_id: Option<uuid::Uuid>,
    pub data_sensitivity_level: Option<String>, // "high", "medical", "therapeutic"

    // Metadata
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Menstrual Health Metric (HIPAA-Compliant)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct MenstrualMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub menstrual_flow: MenstrualFlow,
    pub spotting: bool,
    pub cycle_day: Option<i16>,
    pub cramps_severity: Option<i16>, // 0-10 pain scale
    pub mood_rating: Option<i16>,     // 1-5 rating
    pub energy_level: Option<i16>,    // 1-5 rating
    pub notes: Option<String>,        // Encrypted field for sensitive information
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Fertility Tracking Metric (Enhanced Privacy Protection)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct FertilityMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub cervical_mucus_quality: Option<CervicalMucusQuality>,
    pub ovulation_test_result: OvulationTestResult,

    // Privacy-protected sexual activity tracking (requires special access controls)
    pub sexual_activity: Option<bool>,

    pub pregnancy_test_result: PregnancyTestResult,
    pub basal_body_temperature: Option<f64>, // Celsius
    pub temperature_context: TemperatureContext,

    // Additional fertility indicators
    pub cervix_firmness: Option<i16>,  // 1=soft, 2=medium, 3=firm
    pub cervix_position: Option<i16>,  // 1=low, 2=medium, 3=high
    pub lh_level: Option<f64>,         // Luteinizing hormone level (mIU/mL)

    pub notes: Option<String>,         // Encrypted field
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Tagged union for all health metric types
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum HealthMetric {
    HeartRate(HeartRateMetric),
    BloodPressure(BloodPressureMetric),
    Sleep(SleepMetric),
    Activity(ActivityMetric),
    BodyMeasurement(BodyMeasurementMetric),
    Temperature(TemperatureMetric),
    BloodGlucose(BloodGlucoseMetric),
    Respiratory(RespiratoryMetric),
    Workout(WorkoutData),
    Environmental(EnvironmentalMetric),
    AudioExposure(AudioExposureMetric),
    SafetyEvent(SafetyEventMetric),
    Mindfulness(MindfulnessMetric),
    MentalHealth(MentalHealthMetric),
    Menstrual(MenstrualMetric),
    Fertility(FertilityMetric),
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
#[derive(Debug, Serialize, Deserialize)]
pub struct IngestResponse {
    pub success: bool,
    pub processed_count: usize,
    pub failed_count: usize,
    pub processing_time_ms: u64,
    pub errors: Vec<ProcessingError>,
}

/// Individual processing error
#[derive(Debug, Serialize, Deserialize, Clone)]
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
            if !(0.0..=500.0).contains(&hrv) {
                return Err(format!(
                    "heart_rate_variability {hrv} is out of range (0-500)"
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
            if !(config.sleep_efficiency_min..=config.sleep_efficiency_max).contains(&(eff as f32))
            {
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
                "Sleep components total ({component_total} minutes) exceeds sleep duration ({calculated_duration} minutes)"
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
            if !(0..=10000).contains(&flights) {
                return Err(format!(
                    "flights_climbed {flights} is out of range (0-10000)"
                ));
            }
        }
        Ok(())
    }
}

impl BodyMeasurementMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Body weight validation (20-500 kg range)
        if let Some(weight) = self.body_weight_kg {
            if weight < config.body_weight_min_kg || weight > config.body_weight_max_kg {
                return Err(format!(
                    "body_weight_kg {} is out of range ({}-{})",
                    weight, config.body_weight_min_kg, config.body_weight_max_kg
                ));
            }
        }

        // BMI validation (15-50 range)
        if let Some(bmi) = self.body_mass_index {
            if bmi < config.bmi_min || bmi > config.bmi_max {
                return Err(format!(
                    "body_mass_index {} is out of range ({}-{})",
                    bmi, config.bmi_min, config.bmi_max
                ));
            }
        }

        // Body fat percentage validation (3-50% range)
        if let Some(body_fat) = self.body_fat_percentage {
            if body_fat < config.body_fat_min_percent || body_fat > config.body_fat_max_percent {
                return Err(format!(
                    "body_fat_percentage {} is out of range ({}-{})",
                    body_fat, config.body_fat_min_percent, config.body_fat_max_percent
                ));
            }
        }

        // Lean body mass validation
        if let Some(lean_mass) = self.lean_body_mass_kg {
            if lean_mass < 10.0 || lean_mass > 200.0 {
                return Err(format!(
                    "lean_body_mass_kg {} is out of range (10-200)",
                    lean_mass
                ));
            }

            // Validate lean body mass doesn't exceed total body weight
            if let Some(weight) = self.body_weight_kg {
                if lean_mass > weight {
                    return Err(format!(
                        "lean_body_mass_kg {} cannot exceed body_weight_kg {}",
                        lean_mass, weight
                    ));
                }
            }
        }

        // Circumference measurements validation (positive values, reasonable ranges)
        if let Some(waist) = self.waist_circumference_cm {
            if waist <= 0.0 || waist > 300.0 {
                return Err(format!(
                    "waist_circumference_cm {} is out of range (0-300)",
                    waist
                ));
            }
        }

        if let Some(hip) = self.hip_circumference_cm {
            if hip <= 0.0 || hip > 300.0 {
                return Err(format!(
                    "hip_circumference_cm {} is out of range (0-300)",
                    hip
                ));
            }
        }

        // Body temperature validation (if included)
        if let Some(temp) = self.body_temperature_celsius {
            if temp < config.body_temperature_min_celsius || temp > config.body_temperature_max_celsius {
                return Err(format!(
                    "body_temperature_celsius {} is out of range ({}-{})",
                    temp, config.body_temperature_min_celsius, config.body_temperature_max_celsius
                ));
            }
        }

        if let Some(basal_temp) = self.basal_body_temperature_celsius {
            if basal_temp < 35.0 || basal_temp > 38.0 {
                return Err(format!(
                    "basal_body_temperature_celsius {} is out of range (35-38)",
                    basal_temp
                ));
            }
        }

        Ok(())
    }

    /// Calculate BMI from height (stored elsewhere) and current weight
    /// Returns None if either value is missing
    pub fn calculate_bmi(&self, height_cm: f64) -> Option<f64> {
        if let Some(weight_kg) = self.body_weight_kg {
            let height_m = height_cm / 100.0;
            if height_m > 0.0 {
                Some(weight_kg / (height_m * height_m))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Validate BMI consistency with weight/height if both are available
    pub fn validate_bmi_consistency(&self, height_cm: Option<f64>) -> Result<(), String> {
        if let (Some(reported_bmi), Some(weight), Some(height)) =
            (self.body_mass_index, self.body_weight_kg, height_cm) {
            let calculated_bmi = self.calculate_bmi(height).unwrap_or(0.0);
            let bmi_difference = (reported_bmi - calculated_bmi).abs();

            // Allow 5% tolerance for BMI calculation differences
            if bmi_difference > (calculated_bmi * 0.05) {
                return Err(format!(
                    "BMI consistency check failed: reported BMI {} vs calculated BMI {:.2} (weight: {}kg, height: {}cm)",
                    reported_bmi, calculated_bmi, weight, height
                ));
            }
        }
        Ok(())
    }

    /// Check if this measurement represents smart device multi-metric data
    /// Smart scales typically provide weight, BMI, body fat, and lean mass together
    pub fn is_multi_metric_reading(&self) -> bool {
        // Consider it multi-metric if we have weight plus at least 2 composition metrics
        let has_weight = self.body_weight_kg.is_some();
        let composition_count = [
            self.body_mass_index.is_some(),
            self.body_fat_percentage.is_some(),
            self.lean_body_mass_kg.is_some(),
        ].iter().filter(|&&x| x).count();

        has_weight && composition_count >= 2
    }
}

impl RespiratoryMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Respiratory rate validation
        if let Some(rate) = self.respiratory_rate {
            if rate < config.respiratory_rate_min || rate > config.respiratory_rate_max {
                return Err(format!(
                    "respiratory_rate {} is out of range ({}-{})",
                    rate, config.respiratory_rate_min, config.respiratory_rate_max
                ));
            }
        }

        // SpO2 validation with critical threshold checking
        if let Some(spo2) = self.oxygen_saturation {
            if spo2 < config.oxygen_saturation_min || spo2 > config.oxygen_saturation_max {
                return Err(format!(
                    "oxygen_saturation {} is out of range ({:.1}-{:.1}%)",
                    spo2, config.oxygen_saturation_min, config.oxygen_saturation_max
                ));
            }

            // Check for critical SpO2 levels - this could trigger alerts
            if spo2 < config.oxygen_saturation_critical {
                // Note: In production, this might trigger a real-time alert
                tracing::warn!(
                    user_id = %self.user_id,
                    spo2 = %spo2,
                    "CRITICAL: SpO2 below 90% - potential medical emergency"
                );
            }
        }

        // Forced Vital Capacity (FVC) validation
        if let Some(fvc) = self.forced_vital_capacity {
            if fvc < config.forced_vital_capacity_min || fvc > config.forced_vital_capacity_max {
                return Err(format!(
                    "forced_vital_capacity {} is out of range ({:.1}-{:.1} L)",
                    fvc, config.forced_vital_capacity_min, config.forced_vital_capacity_max
                ));
            }
        }

        // Forced Expiratory Volume in 1 second (FEV1) validation
        if let Some(fev1) = self.forced_expiratory_volume_1 {
            if fev1 < config.forced_expiratory_volume_1_min || fev1 > config.forced_expiratory_volume_1_max {
                return Err(format!(
                    "forced_expiratory_volume_1 {} is out of range ({:.1}-{:.1} L)",
                    fev1, config.forced_expiratory_volume_1_min, config.forced_expiratory_volume_1_max
                ));
            }

            // FEV1/FVC ratio validation (if both are present)
            if let Some(fvc) = self.forced_vital_capacity {
                if fvc > 0.0 {
                    let ratio = fev1 / fvc;
                    if ratio < 0.3 || ratio > 1.0 {
                        return Err(format!(
                            "FEV1/FVC ratio {:.2} is out of normal range (0.3-1.0)",
                            ratio
                        ));
                    }

                    // Check for obstructive pattern (FEV1/FVC < 0.7)
                    if ratio < 0.7 {
                        tracing::info!(
                            user_id = %self.user_id,
                            ratio = %ratio,
                            "Potential obstructive pattern detected (FEV1/FVC < 0.7)"
                        );
                    }
                }
            }
        }

        // Peak Expiratory Flow Rate (PEFR) validation
        if let Some(pefr) = self.peak_expiratory_flow_rate {
            if pefr < config.peak_expiratory_flow_rate_min || pefr > config.peak_expiratory_flow_rate_max {
                return Err(format!(
                    "peak_expiratory_flow_rate {} is out of range ({:.0}-{:.0} L/min)",
                    pefr, config.peak_expiratory_flow_rate_min, config.peak_expiratory_flow_rate_max
                ));
            }
        }

        // Inhaler usage validation
        if let Some(usage) = self.inhaler_usage {
            if usage < 0 {
                return Err("inhaler_usage cannot be negative".to_string());
            }
            if usage > config.inhaler_usage_max {
                return Err(format!(
                    "inhaler_usage {} exceeds maximum daily usage of {}",
                    usage, config.inhaler_usage_max
                ));
            }

            // Alert for excessive inhaler usage
            if usage > 8 {
                tracing::warn!(
                    user_id = %self.user_id,
                    usage = %usage,
                    "High inhaler usage detected - consider medical review"
                );
            }
        }

        Ok(())
    }

    /// Check if this respiratory measurement indicates a critical condition
    pub fn is_critical(&self, config: &ValidationConfig) -> bool {
        // SpO2 below critical threshold
        if let Some(spo2) = self.oxygen_saturation {
            if spo2 < config.oxygen_saturation_critical {
                return true;
            }
        }

        // Extreme respiratory rates
        if let Some(rate) = self.respiratory_rate {
            if rate < 8 || rate > 30 {
                return true;
            }
        }

        // Excessive inhaler usage in short time
        if let Some(usage) = self.inhaler_usage {
            if usage > 10 {
                return true;
            }
        }

        false
    }
}

impl TemperatureMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Validate body temperature ranges (medical-grade validation)
        if let Some(temp) = self.body_temperature {
            if !(config.body_temperature_min..=config.body_temperature_max).contains(&(temp as f32)) {
                return Err(format!(
                    "body_temperature {}°C is out of range ({}°C-{}°C)",
                    temp, config.body_temperature_min, config.body_temperature_max
                ));
            }
        }

        // Validate basal body temperature for fertility tracking
        if let Some(temp) = self.basal_body_temperature {
            if !(config.basal_body_temperature_min..=config.basal_body_temperature_max).contains(&(temp as f32)) {
                return Err(format!(
                    "basal_body_temperature {}°C is out of range ({}°C-{}°C)",
                    temp, config.basal_body_temperature_min, config.basal_body_temperature_max
                ));
            }
        }

        // Validate Apple Watch wrist temperature
        if let Some(temp) = self.apple_sleeping_wrist_temperature {
            if !(config.wrist_temperature_min..=config.wrist_temperature_max).contains(&(temp as f32)) {
                return Err(format!(
                    "apple_sleeping_wrist_temperature {}°C is out of range ({}°C-{}°C)",
                    temp, config.wrist_temperature_min, config.wrist_temperature_max
                ));
            }
        }

        // Validate water temperature (environmental)
        if let Some(temp) = self.water_temperature {
            if !(config.water_temperature_min..=config.water_temperature_max).contains(&(temp as f32)) {
                return Err(format!(
                    "water_temperature {}°C is out of range ({}°C-{}°C)",
                    temp, config.water_temperature_min, config.water_temperature_max
                ));
            }
        }

        Ok(())
    }

    /// Check if temperature indicates fever (body temp > 38.0°C / 100.4°F)
    pub fn has_fever(&self) -> bool {
        self.body_temperature.map_or(false, |temp| temp > 38.0)
    }

    /// Check if basal body temperature indicates ovulation spike
    pub fn basal_temp_spike(&self, baseline_temp: f64) -> bool {
        self.basal_body_temperature.map_or(false, |temp| temp > baseline_temp + 0.2)
    }

    /// Get primary temperature value for general monitoring
    pub fn primary_temperature(&self) -> Option<f64> {
        self.body_temperature
            .or(self.basal_body_temperature)
            .or(self.apple_sleeping_wrist_temperature)
    }
}

impl EnvironmentalMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Validate UV index range (0-11+ scale)
        if let Some(uv) = self.uv_index {
            if uv < 0.0 || uv > 20.0 {
                return Err(format!("uv_index {} is out of valid range (0-20)", uv));
            }
        }

        // Validate UV exposure time
        if let Some(exposure_min) = self.uv_exposure_minutes {
            if exposure_min < 0 || exposure_min > 24 * 60 {
                return Err(format!("uv_exposure_minutes {} is out of range (0-1440)", exposure_min));
            }
        }

        // Validate daylight time
        if let Some(daylight_min) = self.time_in_daylight_minutes {
            if daylight_min < 0 || daylight_min > 24 * 60 {
                return Err(format!("time_in_daylight_minutes {} is out of range (0-1440)", daylight_min));
            }
        }

        // Validate temperature range (-50 to 60 Celsius)
        if let Some(temp) = self.ambient_temperature_celsius {
            if temp < -50.0 || temp > 60.0 {
                return Err(format!("ambient_temperature_celsius {} is out of range (-50 to 60)", temp));
            }
        }

        // Validate humidity percentage
        if let Some(humidity) = self.humidity_percent {
            if humidity < 0.0 || humidity > 100.0 {
                return Err(format!("humidity_percent {} is out of range (0-100)", humidity));
            }
        }

        // Validate GPS coordinates if provided
        if let Some(lat) = self.location_latitude {
            if !(config.latitude_min..=config.latitude_max).contains(&lat) {
                return Err(format!("location_latitude {} is out of range ({} to {})",
                    lat, config.latitude_min, config.latitude_max));
            }
        }

        if let Some(lng) = self.location_longitude {
            if !(config.longitude_min..=config.longitude_max).contains(&lng) {
                return Err(format!("location_longitude {} is out of range ({} to {})",
                    lng, config.longitude_min, config.longitude_max));
            }
        }

        Ok(())
    }
}

impl AudioExposureMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate environmental audio exposure (typical range 0-120 dB)
        if let Some(env_db) = self.environmental_audio_exposure_db {
            if env_db < 0.0 || env_db > 150.0 {
                return Err(format!("environmental_audio_exposure_db {} is out of range (0-150)", env_db));
            }
        }

        // Validate headphone audio exposure (typical range 0-120 dB)
        if let Some(hp_db) = self.headphone_audio_exposure_db {
            if hp_db < 0.0 || hp_db > 150.0 {
                return Err(format!("headphone_audio_exposure_db {} is out of range (0-150)", hp_db));
            }
        }

        // Validate exposure duration (must be positive, max 24 hours)
        if self.exposure_duration_minutes < 0 || self.exposure_duration_minutes > 24 * 60 {
            return Err(format!("exposure_duration_minutes {} is out of range (0-1440)",
                self.exposure_duration_minutes));
        }

        // Check for dangerous audio levels (85+ dB for extended periods)
        if let Some(env_db) = self.environmental_audio_exposure_db {
            if env_db >= 85.0 && self.exposure_duration_minutes > 60 && !self.audio_exposure_event {
                return Err("audio_exposure_event should be true for prolonged high-volume exposure".to_string());
            }
        }

        if let Some(hp_db) = self.headphone_audio_exposure_db {
            if hp_db >= 85.0 && self.exposure_duration_minutes > 60 && !self.audio_exposure_event {
                return Err("audio_exposure_event should be true for prolonged high-volume headphone exposure".to_string());
            }
        }

        Ok(())
    }
}

impl SafetyEventMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Validate event type is not empty
        if self.event_type.is_empty() {
            return Err("event_type cannot be empty".to_string());
        }

        // Validate severity level if provided
        if let Some(severity) = self.severity_level {
            if !(1..=5).contains(&severity) {
                return Err(format!("severity_level {} is out of range (1-5)", severity));
            }
        }

        // Validate GPS coordinates if provided
        if let Some(lat) = self.location_latitude {
            if !(config.latitude_min..=config.latitude_max).contains(&lat) {
                return Err(format!("location_latitude {} is out of range ({} to {})",
                    lat, config.latitude_min, config.latitude_max));
            }
        }

        if let Some(lng) = self.location_longitude {
            if !(config.longitude_min..=config.longitude_max).contains(&lng) {
                return Err(format!("location_longitude {} is out of range ({} to {})",
                    lng, config.longitude_min, config.longitude_max));
            }
        }

        // Validate known event types
        match self.event_type.as_str() {
            "fall_detected" | "emergency_sos" | "crash_detected" | "hard_fall" | "medical_emergency" => {},
            _ => {
                // Allow unknown event types but log for monitoring
                tracing::debug!("Unknown safety event type: {}", self.event_type);
            }
        }

        Ok(())
    }
}

impl MindfulnessMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate session duration (reasonable meditation session lengths)
        if let Some(duration) = self.session_duration_minutes {
            if duration < 1 || duration > 720 { // 1 minute to 12 hours max
                return Err(format!("session_duration_minutes {} is out of range (1-720)", duration));
            }
        }

        // Validate session quality rating
        if let Some(quality) = self.session_quality_rating {
            if !(1..=5).contains(&quality) {
                return Err(format!("session_quality_rating {} is out of range (1-5)", quality));
            }
        }

        // Validate focus rating
        if let Some(focus) = self.focus_rating {
            if !(1..=10).contains(&focus) {
                return Err(format!("focus_rating {} is out of range (1-10)", focus));
            }
        }

        // Validate mindful minutes tracking
        if let Some(minutes) = self.mindful_minutes_today {
            if minutes < 0 || minutes > 24 * 60 {
                return Err(format!("mindful_minutes_today {} is out of range (0-1440)", minutes));
            }
        }

        if let Some(minutes) = self.mindful_minutes_week {
            if minutes < 0 || minutes > 7 * 24 * 60 {
                return Err(format!("mindful_minutes_week {} is out of range (0-10080)", minutes));
            }
        }

        // Validate breathing rate
        if let Some(breathing_rate) = self.breathing_rate_breaths_per_min {
            if breathing_rate < 5.0 || breathing_rate > 40.0 {
                return Err(format!("breathing_rate_breaths_per_min {} is out of range (5.0-40.0)", breathing_rate));
            }
        }

        // Validate heart rate variability during session
        if let Some(hrv) = self.heart_rate_variability_during_session {
            if hrv < 0.0 || hrv > 200.0 {
                return Err(format!("heart_rate_variability_during_session {} is out of range (0.0-200.0)", hrv));
            }
        }

        Ok(())
    }

    /// Get meditation type as enum (convert from string)
    pub fn get_meditation_type(&self) -> Option<MeditationType> {
        self.meditation_type.as_deref().map(MeditationType::from_ios_string)
    }

    /// Set meditation type from enum (convert to string)
    pub fn set_meditation_type(&mut self, meditation_type: MeditationType) {
        self.meditation_type = Some(meditation_type.to_string());
    }

    /// Check if this is a high-quality meditation session
    pub fn is_high_quality_session(&self) -> bool {
        self.session_quality_rating.map_or(false, |rating| rating >= 4) &&
        self.focus_rating.map_or(false, |rating| rating >= 7)
    }

    /// Calculate session effectiveness score (0-100)
    pub fn effectiveness_score(&self) -> i16 {
        let quality_score = self.session_quality_rating.unwrap_or(3) * 20; // 1-5 -> 20-100
        let focus_score = self.focus_rating.unwrap_or(5) * 10; // 1-10 -> 10-100
        let duration_bonus = if self.session_duration_minutes.unwrap_or(0) >= 10 { 10 } else { 0 };

        std::cmp::min(100, (quality_score + focus_score) / 2 + duration_bonus)
    }
}

impl MentalHealthMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate iOS 17+ State of Mind valence
        if let Some(valence) = self.state_of_mind_valence {
            if !(-1.0..=1.0).contains(&valence) {
                return Err(format!("state_of_mind_valence {} is out of range (-1.0 to 1.0)", valence));
            }
        }

        // Validate mood rating
        if let Some(mood) = self.mood_rating {
            if !(1..=10).contains(&mood) {
                return Err(format!("mood_rating {} is out of range (1-10)", mood));
            }
        }

        // Validate anxiety level
        if let Some(anxiety) = self.anxiety_level {
            if !(1..=10).contains(&anxiety) {
                return Err(format!("anxiety_level {} is out of range (1-10)", anxiety));
            }
        }

        // Validate stress level
        if let Some(stress) = self.stress_level {
            if !(1..=10).contains(&stress) {
                return Err(format!("stress_level {} is out of range (1-10)", stress));
            }
        }

        // Validate energy level
        if let Some(energy) = self.energy_level {
            if !(1..=10).contains(&energy) {
                return Err(format!("energy_level {} is out of range (1-10)", energy));
            }
        }

        // Validate clinical screening scores
        if let Some(phq9) = self.depression_screening_score {
            if !(0..=27).contains(&phq9) {
                return Err(format!("depression_screening_score {} is out of range (0-27)", phq9));
            }
        }

        if let Some(gad7) = self.anxiety_screening_score {
            if !(0..=21).contains(&gad7) {
                return Err(format!("anxiety_screening_score {} is out of range (0-21)", gad7));
            }
        }

        // Validate sleep quality impact
        if let Some(sleep_impact) = self.sleep_quality_impact {
            if !(1..=5).contains(&sleep_impact) {
                return Err(format!("sleep_quality_impact {} is out of range (1-5)", sleep_impact));
            }
        }

        // Validate data sensitivity level
        if let Some(ref sensitivity) = self.data_sensitivity_level {
            match sensitivity.as_str() {
                "high" | "medical" | "therapeutic" => {},
                _ => return Err(format!("Invalid data_sensitivity_level: {}", sensitivity)),
            }
        }

        Ok(())
    }

    /// Get state of mind as enum (convert from valence)
    pub fn get_state_of_mind(&self) -> Option<StateOfMind> {
        self.state_of_mind_valence.map(StateOfMind::from_valence)
    }

    /// Set state of mind from enum (convert to valence)
    pub fn set_state_of_mind(&mut self, state: StateOfMind) {
        self.state_of_mind_valence = Some(state.to_valence());
    }

    /// Check if this metric indicates clinical concern (high depression/anxiety scores)
    pub fn indicates_clinical_concern(&self) -> bool {
        self.depression_screening_score.map_or(false, |score| score >= 15) ||
        self.anxiety_screening_score.map_or(false, |score| score >= 15)
    }

    /// Check if this is a positive mental health entry
    pub fn is_positive_entry(&self) -> bool {
        let good_mood = self.mood_rating.map_or(false, |mood| mood >= 7);
        let low_stress = self.stress_level.map_or(false, |stress| stress <= 4);
        let low_anxiety = self.anxiety_level.map_or(false, |anxiety| anxiety <= 4);
        let high_energy = self.energy_level.map_or(false, |energy| energy >= 6);

        good_mood || (low_stress && low_anxiety && high_energy)
    }

    /// Calculate overall mental wellness score (0-100)
    pub fn wellness_score(&self) -> i16 {
        let mood_score = self.mood_rating.map_or(50, |m| (m * 10)) as i16;
        let stress_penalty = self.stress_level.map_or(0, |s| (10 - s) * 5) as i16;
        let anxiety_penalty = self.anxiety_level.map_or(0, |a| (10 - a) * 5) as i16;
        let energy_bonus = self.energy_level.map_or(0, |e| (e * 3)) as i16;

        std::cmp::max(0, std::cmp::min(100, mood_score + stress_penalty + anxiety_penalty + energy_bonus - 100))
    }

    /// Check if encrypted notes require decryption key
    pub fn has_encrypted_notes(&self) -> bool {
        self.private_notes_encrypted.is_some() && self.notes_encryption_key_id.is_some()
    }

    /// Get sensitivity level for privacy controls
    pub fn get_sensitivity_level(&self) -> &str {
        self.data_sensitivity_level.as_deref().unwrap_or("high")
    }
}

/// HIPAA-Compliant Menstrual Health Metric Validation
impl MenstrualMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate cycle day is within reasonable range
        if let Some(cycle_day) = self.cycle_day {
            if !(1..=40).contains(&cycle_day) {
                return Err(format!(
                    "cycle_day {} is out of range (1-40 days)",
                    cycle_day
                ));
            }
        }

        // Validate cramps severity scale
        if let Some(severity) = self.cramps_severity {
            if !(0..=10).contains(&severity) {
                return Err(format!(
                    "cramps_severity {} is out of range (0-10 pain scale)",
                    severity
                ));
            }
        }

        // Validate mood rating
        if let Some(mood) = self.mood_rating {
            if !(1..=5).contains(&mood) {
                return Err(format!(
                    "mood_rating {} is out of range (1-5 scale)",
                    mood
                ));
            }
        }

        // Validate energy level
        if let Some(energy) = self.energy_level {
            if !(1..=5).contains(&energy) {
                return Err(format!(
                    "energy_level {} is out of range (1-5 scale)",
                    energy
                ));
            }
        }

        // Validate flow is compatible with spotting
        if self.menstrual_flow == MenstrualFlow::None && self.spotting {
            return Err("Cannot have spotting with no menstrual flow".to_string());
        }

        Ok(())
    }

    /// Get privacy level for menstrual data
    pub fn get_privacy_level(&self) -> &'static str {
        self.menstrual_flow.privacy_level()
    }

    /// Check if this data requires enhanced audit logging
    pub fn requires_enhanced_audit(&self) -> bool {
        // All menstrual data is considered sensitive
        self.menstrual_flow != MenstrualFlow::None || self.spotting
    }

    /// Calculate cycle phase based on cycle day (if available)
    pub fn get_cycle_phase(&self) -> Option<&'static str> {
        self.cycle_day.map(|day| match day {
            1..=7 => "menstrual",
            8..=13 => "follicular",
            14..=16 => "ovulatory",
            17..=28 => "luteal",
            _ => "extended_cycle",
        })
    }
}

/// Privacy-First Fertility Tracking Metric Validation
impl FertilityMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate basal body temperature range (if provided)
        if let Some(temp) = self.basal_body_temperature {
            if !(35.0..=40.0).contains(&temp) {
                return Err(format!(
                    "basal_body_temperature {} is out of physiological range (35.0-40.0°C)",
                    temp
                ));
            }

            // Validate temperature context is appropriate for fertility tracking
            if !self.temperature_context.is_fertility_relevant() {
                return Err(format!(
                    "temperature_context {} is not appropriate for fertility tracking",
                    self.temperature_context
                ));
            }
        }

        // Validate cervix firmness scale
        if let Some(firmness) = self.cervix_firmness {
            if !(1..=3).contains(&firmness) {
                return Err(format!(
                    "cervix_firmness {} is out of range (1=soft, 2=medium, 3=firm)",
                    firmness
                ));
            }
        }

        // Validate cervix position scale
        if let Some(position) = self.cervix_position {
            if !(1..=3).contains(&position) {
                return Err(format!(
                    "cervix_position {} is out of range (1=low, 2=medium, 3=high)",
                    position
                ));
            }
        }

        // Validate LH level (if provided)
        if let Some(lh) = self.lh_level {
            if !(0.0..=200.0).contains(&lh) {
                return Err(format!(
                    "lh_level {} is out of physiological range (0.0-200.0 mIU/mL)",
                    lh
                ));
            }
        }

        // Validate ovulation test consistency with LH level
        if let Some(lh) = self.lh_level {
            match self.ovulation_test_result {
                OvulationTestResult::Peak if lh < 20.0 => {
                    return Err("Peak ovulation test result inconsistent with low LH level".to_string());
                }
                OvulationTestResult::Negative if lh > 40.0 => {
                    return Err("Negative ovulation test inconsistent with high LH level".to_string());
                }
                _ => {} // Other combinations are acceptable
            }
        }

        Ok(())
    }

    /// Get privacy level - fertility data is always highly sensitive
    pub fn get_privacy_level(&self) -> &'static str {
        if self.sexual_activity.is_some() {
            "highly_sensitive" // Sexual activity data requires maximum protection
        } else if self.pregnancy_test_result.requires_enhanced_audit() {
            self.pregnancy_test_result.privacy_level()
        } else {
            "sensitive" // All other fertility data is sensitive
        }
    }

    /// Check if this data requires enhanced audit logging
    pub fn requires_enhanced_audit(&self) -> bool {
        // Sexual activity and pregnancy tests always require enhanced audit
        self.sexual_activity.is_some() || self.pregnancy_test_result.requires_enhanced_audit()
    }

    /// Calculate fertility probability based on available indicators
    pub fn calculate_fertility_probability(&self) -> u8 {
        let mut score = 0u32;
        let mut factors = 0u32;

        // Ovulation test score
        score += self.ovulation_test_result.fertility_score() as u32;
        factors += 1;

        // Cervical mucus quality score
        if let Some(mucus) = self.cervical_mucus_quality {
            score += (mucus.fertility_indicator() * 20) as u32; // Scale to 0-100
            factors += 1;
        }

        // LH level contribution
        if let Some(lh) = self.lh_level {
            let lh_score = match lh {
                0.0..=10.0 => 10,
                10.1..=25.0 => 40,
                25.1..=50.0 => 70,
                50.1..=100.0 => 90,
                _ => 95,
            };
            score += lh_score;
            factors += 1;
        }

        if factors > 0 {
            std::cmp::min(100, score / factors) as u8
        } else {
            0
        }
    }

    /// Get fertility window status
    pub fn get_fertility_status(&self) -> &'static str {
        match self.calculate_fertility_probability() {
            0..=20 => "low_fertility",
            21..=50 => "moderate_fertility",
            51..=80 => "high_fertility",
            81..=100 => "peak_fertility",
            _ => "unknown_fertility", // Handle any values > 100 (shouldn't happen)
        }
    }

    /// Check if cervical mucus indicates peak fertility
    pub fn is_peak_fertility_mucus(&self) -> bool {
        matches!(
            self.cervical_mucus_quality,
            Some(CervicalMucusQuality::EggWhite)
        )
    }
}

impl BloodGlucoseMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Medical-critical blood glucose validation (70-180 mg/dL normal, diabetic ranges vary)
        if !(config.blood_glucose_min..=config.blood_glucose_max).contains(&(self.blood_glucose_mg_dl as f32)) {
            return Err(format!(
                "blood_glucose {} mg/dL is out of safe range ({}-{} mg/dL)",
                self.blood_glucose_mg_dl, config.blood_glucose_min, config.blood_glucose_max
            ));
        }

        // Validate insulin delivery units if present
        if let Some(insulin_units) = self.insulin_delivery_units {
            if insulin_units < 0.0 {
                return Err("insulin_delivery_units cannot be negative".to_string());
            }
            if insulin_units > config.insulin_max_units {
                return Err(format!(
                    "insulin_delivery_units {} exceeds maximum of {} units",
                    insulin_units, config.insulin_max_units
                ));
            }
        }

        Ok(())
    }

    /// Check if this is a dangerous glucose level requiring immediate attention
    pub fn is_critical_glucose_level(&self) -> bool {
        self.blood_glucose_mg_dl < 70.0 || self.blood_glucose_mg_dl > 400.0
    }

    /// Get glucose level category for medical interpretation
    pub fn glucose_category(&self) -> &'static str {
        match self.blood_glucose_mg_dl as i32 {
            0..=69 => "hypoglycemic_critical", // Dangerous low
            70..=99 => "normal_fasting",        // Normal fasting range
            100..=125 => "pre_diabetic",        // Pre-diabetic range
            126..=180 => "diabetic_controlled", // Diabetic but controlled
            181..=250 => "diabetic_high",       // High diabetic range
            251..=400 => "diabetic_very_high",  // Very high, needs attention
            _ => "critical_emergency",          // >400 or invalid, emergency
        }
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
            HealthMetric::BodyMeasurement(metric) => metric.validate_with_config(config),
            HealthMetric::Temperature(metric) => metric.validate_with_config(config),
            HealthMetric::BloodGlucose(metric) => metric.validate_with_config(config),
            HealthMetric::Respiratory(metric) => metric.validate_with_config(config),
            HealthMetric::Workout(workout) => workout.validate_with_config(config),
            HealthMetric::Environmental(metric) => metric.validate_with_config(config),
            HealthMetric::AudioExposure(metric) => metric.validate_with_config(config),
            HealthMetric::SafetyEvent(metric) => metric.validate_with_config(config),
            HealthMetric::Mindfulness(metric) => metric.validate_with_config(config),
            HealthMetric::MentalHealth(metric) => metric.validate_with_config(config),
            HealthMetric::Menstrual(metric) => metric.validate_with_config(config),
            HealthMetric::Fertility(metric) => metric.validate_with_config(config),
        }
    }

    pub fn metric_type(&self) -> &'static str {
        match self {
            HealthMetric::HeartRate(_) => "HeartRate",
            HealthMetric::BloodPressure(_) => "BloodPressure",
            HealthMetric::Sleep(_) => "Sleep",
            HealthMetric::Activity(_) => "Activity",
            HealthMetric::BodyMeasurement(_) => "BodyMeasurement",
            HealthMetric::Temperature(_) => "Temperature",
            HealthMetric::BloodGlucose(_) => "BloodGlucose",
            HealthMetric::Respiratory(_) => "Respiratory",
            HealthMetric::Workout(_) => "Workout",
            HealthMetric::Environmental(_) => "Environmental",
            HealthMetric::AudioExposure(_) => "AudioExposure",
            HealthMetric::SafetyEvent(_) => "SafetyEvent",
            HealthMetric::Mindfulness(_) => "Mindfulness",
            HealthMetric::MentalHealth(_) => "MentalHealth",
            HealthMetric::Menstrual(_) => "Menstrual",
            HealthMetric::Fertility(_) => "Fertility",
        }
    }
}
