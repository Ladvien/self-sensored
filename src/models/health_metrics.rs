use crate::config::ValidationConfig;
use crate::models::enums::{
    ActivityContext, WorkoutType, MeditationType, StateOfMind,
    MenstrualFlow, CervicalMucusQuality, OvulationTestResult,
    PregnancyTestResult, TemperatureContext, SymptomType, SymptomSeverity,
    HygieneEventType, HeartRateEventType, CardiacEventSeverity,
};
use crate::models::user_characteristics::UserCharacteristics;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Heart rate metric with validation (STORY-011: Extended for Advanced Cardiovascular Monitoring)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct HeartRateMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub heart_rate: Option<i16>,
    pub resting_heart_rate: Option<i16>,
    pub heart_rate_variability: Option<f64>,

    // Advanced Cardiovascular Metrics (STORY-011)
    pub walking_heart_rate_average: Option<i16>,        // Average HR during walking activities (90-120 BPM normal)
    pub heart_rate_recovery_one_minute: Option<i16>,    // HR recovery after 1 min post-exercise (18+ BPM decrease = good)
    pub atrial_fibrillation_burden_percentage: Option<Decimal>, // AFib burden as % (0.01-100.00%)
    pub vo2_max_ml_kg_min: Option<Decimal>, // VO2 max in ml/kg/min (14.00-65.00 range)

    pub source_device: Option<String>,
    pub context: Option<ActivityContext>,
    pub created_at: DateTime<Utc>,
}

/// Heart Rate Event for cardiac monitoring and detection (STORY-011)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct HeartRateEvent {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub event_type: HeartRateEventType,
    pub event_occurred_at: DateTime<Utc>,
    pub heart_rate_at_event: i16,                       // Heart rate when event occurred
    pub event_duration_minutes: Option<i32>,            // Duration of the event in minutes
    pub context: Option<ActivityContext>,                // Activity context when event occurred
    pub source_device: Option<String>,                   // Device that detected the event
    pub severity: CardiacEventSeverity,                  // Medical severity assessment
    pub is_confirmed: bool,                              // Whether event was medically confirmed
    pub notes: Option<String>,                           // Additional clinical notes or user observations
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

    // Basic Activity Metrics
    pub step_count: Option<i32>,
    pub distance_meters: Option<f64>,
    pub flights_climbed: Option<i32>,
    pub active_energy_burned_kcal: Option<f64>,
    pub basal_energy_burned_kcal: Option<f64>,

    // Specialized Distance Metrics for Different Activities
    pub distance_cycling_meters: Option<f64>,
    pub distance_swimming_meters: Option<f64>,
    pub distance_wheelchair_meters: Option<f64>,
    pub distance_downhill_snow_sports_meters: Option<f64>,

    // Wheelchair Accessibility Metrics
    pub push_count: Option<i32>, // Wheelchair pushes

    // Swimming Analytics
    pub swimming_stroke_count: Option<i32>,

    // Cross-Platform Fitness Integration
    pub nike_fuel_points: Option<i32>,

    // Apple Watch Activity Ring Integration
    pub apple_exercise_time_minutes: Option<i32>,
    pub apple_stand_time_minutes: Option<i32>,
    pub apple_move_time_minutes: Option<i32>,
    pub apple_stand_hour_achieved: Option<bool>,

    // Metadata
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

    // Physical Measurements
    pub height_cm: Option<f64>,
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

/// Metabolic metric for insulin delivery, blood alcohol content, and metabolic tracking
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct MetabolicMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub blood_alcohol_content: Option<f64>,    // Blood alcohol content percentage (0.0-0.5%)
    pub insulin_delivery_units: Option<f64>,   // Insulin delivery units (0-100 units range)
    pub delivery_method: Option<String>,       // "pump", "pen", "syringe", "inhaler", "patch"
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

/// Workout route data with GPS tracking (PostGIS-enabled)
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct WorkoutRoute {
    pub id: uuid::Uuid,
    pub workout_id: uuid::Uuid,
    pub user_id: uuid::Uuid,

    // Route data as JSON array of GPS points
    pub route_points: serde_json::Value, // Array of {lat, lng, timestamp, altitude?, accuracy?, speed?}

    // Calculated route metrics
    pub total_distance_meters: Option<f64>,
    pub elevation_gain_meters: Option<f64>,
    pub elevation_loss_meters: Option<f64>,
    pub max_altitude_meters: Option<f64>,
    pub min_altitude_meters: Option<f64>,

    // Route quality and privacy
    pub point_count: i32,
    pub average_accuracy_meters: Option<f64>,
    pub privacy_level: String, // "full", "approximate", "private"

    pub created_at: DateTime<Utc>,
}

/// GPS route point for detailed tracking
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RoutePoint {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub altitude: Option<f64>,  // meters above sea level
    pub accuracy: Option<f64>,  // GPS accuracy in meters
    pub speed: Option<f64>,     // speed in m/s at this point
}

/// Workout with optional route data for comprehensive tracking
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkoutWithRoute {
    pub workout: WorkoutData,
    pub route: Option<WorkoutRoute>,
}

impl WorkoutRoute {
    /// Calculate route metrics from GPS points
    pub fn calculate_metrics_from_points(points: &[RoutePoint]) -> RouteMetrics {
        if points.is_empty() {
            return RouteMetrics::default();
        }

        let mut total_distance = 0.0;
        let mut elevation_gain = 0.0;
        let mut elevation_loss = 0.0;
        let mut max_altitude = points[0].altitude.unwrap_or(0.0);
        let mut min_altitude = points[0].altitude.unwrap_or(0.0);
        let mut accuracy_sum = 0.0;
        let mut accuracy_count = 0;

        for i in 1..points.len() {
            let prev = &points[i - 1];
            let curr = &points[i];

            // Calculate distance using Haversine formula
            let distance = haversine_distance(
                prev.latitude, prev.longitude,
                curr.latitude, curr.longitude
            );
            total_distance += distance;

            // Calculate elevation changes
            if let (Some(prev_alt), Some(curr_alt)) = (prev.altitude, curr.altitude) {
                let elevation_change = curr_alt - prev_alt;
                if elevation_change > 0.0 {
                    elevation_gain += elevation_change;
                } else {
                    elevation_loss += elevation_change.abs();
                }
                max_altitude = max_altitude.max(curr_alt);
                min_altitude = min_altitude.min(curr_alt);
            }

            // Track GPS accuracy
            if let Some(accuracy) = curr.accuracy {
                accuracy_sum += accuracy;
                accuracy_count += 1;
            }
        }

        RouteMetrics {
            total_distance_meters: total_distance,
            elevation_gain_meters: elevation_gain,
            elevation_loss_meters: elevation_loss,
            max_altitude_meters: Some(max_altitude),
            min_altitude_meters: Some(min_altitude),
            average_accuracy_meters: if accuracy_count > 0 {
                Some(accuracy_sum / accuracy_count as f64)
            } else {
                None
            },
        }
    }

    /// Validate route points for basic GPS data integrity
    pub fn validate_route_points(points: &[RoutePoint]) -> Result<(), String> {
        if points.is_empty() {
            return Err("Route must contain at least one GPS point".to_string());
        }

        for (i, point) in points.iter().enumerate() {
            // Validate GPS coordinates
            if point.latitude < -90.0 || point.latitude > 90.0 {
                return Err(format!("Invalid latitude {} at point {}", point.latitude, i));
            }
            if point.longitude < -180.0 || point.longitude > 180.0 {
                return Err(format!("Invalid longitude {} at point {}", point.longitude, i));
            }

            // Validate altitude if present (reasonable bounds)
            if let Some(altitude) = point.altitude {
                if altitude < -500.0 || altitude > 9000.0 { // Dead Sea to Everest range
                    return Err(format!("Unrealistic altitude {} at point {}", altitude, i));
                }
            }

            // Validate speed if present (reasonable bounds)
            if let Some(speed) = point.speed {
                if speed < 0.0 || speed > 150.0 { // 0 to ~335 mph in m/s
                    return Err(format!("Unrealistic speed {} m/s at point {}", speed, i));
                }
            }

            // Validate GPS accuracy if present
            if let Some(accuracy) = point.accuracy {
                if accuracy < 0.0 || accuracy > 1000.0 { // 0 to 1km accuracy
                    return Err(format!("Invalid GPS accuracy {} at point {}", accuracy, i));
                }
            }
        }

        // Validate timestamp ordering
        for i in 1..points.len() {
            if points[i].timestamp < points[i - 1].timestamp {
                return Err(format!("Timestamps not in chronological order at points {} and {}", i - 1, i));
            }
        }

        Ok(())
    }
}

/// Calculated route metrics
#[derive(Debug, Default, Clone)]
pub struct RouteMetrics {
    pub total_distance_meters: f64,
    pub elevation_gain_meters: f64,
    pub elevation_loss_meters: f64,
    pub max_altitude_meters: Option<f64>,
    pub min_altitude_meters: Option<f64>,
    pub average_accuracy_meters: Option<f64>,
}

/// Calculate distance between two GPS points using Haversine formula
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371000.0; // meters

    let lat1_rad = lat1.to_radians();
    let lon1_rad = lon1.to_radians();
    let lat2_rad = lat2.to_radians();
    let lon2_rad = lon2.to_radians();

    let delta_lat = lat2_rad - lat1_rad;
    let delta_lon = lon2_rad - lon1_rad;

    let a = (delta_lat / 2.0).sin().powi(2) +
            lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

impl GpsCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

/// Comprehensive nutrition metrics for dietary tracking and analysis
/// Supports 25+ nutritional fields including macronutrients, vitamins, and minerals
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct NutritionMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,

    // Hydration & Stimulants
    pub dietary_water: Option<f64>,                    // liters
    pub dietary_caffeine: Option<f64>,                 // mg

    // Macronutrients (Core Energy)
    pub dietary_energy_consumed: Option<f64>,          // calories
    pub dietary_carbohydrates: Option<f64>,            // grams
    pub dietary_protein: Option<f64>,                  // grams
    pub dietary_fat_total: Option<f64>,                // grams
    pub dietary_fat_saturated: Option<f64>,            // grams
    pub dietary_fat_monounsaturated: Option<f64>,      // grams
    pub dietary_fat_polyunsaturated: Option<f64>,      // grams
    pub dietary_cholesterol: Option<f64>,              // mg
    pub dietary_sodium: Option<f64>,                   // mg
    pub dietary_fiber: Option<f64>,                    // grams
    pub dietary_sugar: Option<f64>,                    // grams

    // Essential Minerals
    pub dietary_calcium: Option<f64>,                  // mg
    pub dietary_iron: Option<f64>,                     // mg
    pub dietary_magnesium: Option<f64>,                // mg
    pub dietary_potassium: Option<f64>,                // mg
    pub dietary_zinc: Option<f64>,                     // mg
    pub dietary_phosphorus: Option<f64>,               // mg

    // Essential Vitamins (Water-soluble)
    pub dietary_vitamin_c: Option<f64>,                // mg
    pub dietary_vitamin_b1_thiamine: Option<f64>,      // mg
    pub dietary_vitamin_b2_riboflavin: Option<f64>,    // mg
    pub dietary_vitamin_b3_niacin: Option<f64>,        // mg
    pub dietary_vitamin_b6_pyridoxine: Option<f64>,    // mg
    pub dietary_vitamin_b12_cobalamin: Option<f64>,    // mcg
    pub dietary_folate: Option<f64>,                   // mcg
    pub dietary_biotin: Option<f64>,                   // mcg
    pub dietary_pantothenic_acid: Option<f64>,         // mg

    // Essential Vitamins (Fat-soluble)
    pub dietary_vitamin_a: Option<f64>,                // mcg RAE
    pub dietary_vitamin_d: Option<f64>,                // IU
    pub dietary_vitamin_e: Option<f64>,                // mg
    pub dietary_vitamin_k: Option<f64>,                // mcg

    // Meal Context for atomic processing
    pub meal_type: Option<String>,                     // breakfast, lunch, dinner, snack
    pub meal_id: Option<uuid::Uuid>,                   // Group nutrients from same meal

    // Metadata and source tracking
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Comprehensive symptom tracking metric for illness monitoring and health assessment
/// Supports 50+ symptom types with severity levels and episode-based grouping
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct SymptomMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,
    pub symptom_type: SymptomType,
    pub severity: SymptomSeverity,
    pub duration_minutes: Option<i32>, // Duration of symptom in minutes (can be very long for chronic symptoms)
    pub notes: Option<String>, // Additional context about the symptom
    pub episode_id: Option<uuid::Uuid>, // Link related symptoms in same illness episode
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
    Metabolic(MetabolicMetric),
    Respiratory(RespiratoryMetric),
    Nutrition(NutritionMetric),
    Workout(WorkoutData),
    Environmental(EnvironmentalMetric),
    AudioExposure(AudioExposureMetric),
    SafetyEvent(SafetyEventMetric),
    Mindfulness(MindfulnessMetric),
    MentalHealth(MentalHealthMetric),
    Menstrual(MenstrualMetric),
    Fertility(FertilityMetric),
    Symptom(SymptomMetric),
    Hygiene(HygieneMetric),
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

        // Advanced Cardiovascular Metrics Validation (STORY-011)
        if let Some(walking_hr) = self.walking_heart_rate_average {
            // Walking HR should be between 60-200 BPM (normal walking range with safety margins)
            if !(60..=200).contains(&walking_hr) {
                return Err(format!(
                    "walking_heart_rate_average {} is out of range (60-200 BPM)",
                    walking_hr
                ));
            }
        }

        if let Some(hr_recovery) = self.heart_rate_recovery_one_minute {
            // HR recovery should be 0-100 BPM decrease (18+ is considered good)
            if !(0..=100).contains(&hr_recovery) {
                return Err(format!(
                    "heart_rate_recovery_one_minute {} is out of range (0-100 BPM)",
                    hr_recovery
                ));
            }
        }

        if let Some(afib_burden) = self.atrial_fibrillation_burden_percentage {
            use rust_decimal::prelude::ToPrimitive;
            let burden_f64 = afib_burden.to_f64().unwrap_or(0.0);
            // AFib burden should be 0.0-100.0% (Apple Watch shows "2% or less" as minimum)
            if !(0.0..=100.0).contains(&burden_f64) {
                return Err(format!(
                    "atrial_fibrillation_burden_percentage {} is out of range (0.0-100.0%)",
                    burden_f64
                ));
            }
        }

        if let Some(vo2_max) = self.vo2_max_ml_kg_min {
            use rust_decimal::prelude::ToPrimitive;
            let vo2_f64 = vo2_max.to_f64().unwrap_or(0.0);
            // VO2 max should be 14.0-65.0 ml/kg/min (Apple Watch supported range)
            if !(14.0..=65.0).contains(&vo2_f64) {
                return Err(format!(
                    "vo2_max_ml_kg_min {} is out of range (14.0-65.0 ml/kg/min)",
                    vo2_f64
                ));
            }
        }

        Ok(())
    }

    /// Personalized validation using user characteristics
    pub fn validate_with_characteristics(&self, config: &ValidationConfig, characteristics: Option<&UserCharacteristics>) -> Result<(), String> {
        match characteristics {
            Some(chars) => {
                let age = chars.age().unwrap_or(30);
                let sex_adjustment = chars.biological_sex.get_heart_rate_adjustment();

                // Calculate personalized ranges based on age and biological sex
                let min_resting = match age {
                    age if age < 30 => 40,
                    age if age < 50 => 45,
                    _ => 50,
                };
                let max_resting = match age {
                    age if age < 30 => 100,
                    age if age < 50 => 95,
                    _ => 90,
                };
                let max_exercise = 220_u32.saturating_sub(age);

                // Apply biological sex adjustment
                let adjusted_min_resting = (min_resting as f64 * sex_adjustment) as i16;
                let adjusted_max_resting = (max_resting as f64 * sex_adjustment) as i16;
                let adjusted_max_exercise = (max_exercise as f64 * sex_adjustment) as i16;

                if let Some(bpm) = self.heart_rate {
                    let max_hr = match self.context {
                        Some(ActivityContext::Exercise) | Some(ActivityContext::Running) | Some(ActivityContext::Cycling) => adjusted_max_exercise,
                        _ => adjusted_max_resting,
                    };

                    if bpm < adjusted_min_resting || bpm > max_hr {
                        return Err(format!(
                            "heart_rate {} is outside personalized range ({}-{}) for age {} and biological sex {:?}",
                            bpm, adjusted_min_resting, max_hr, age, chars.biological_sex
                        ));
                    }
                }

                if let Some(bpm) = self.resting_heart_rate {
                    if bpm < adjusted_min_resting || bpm > adjusted_max_resting {
                        return Err(format!(
                            "resting_heart_rate {} is outside personalized range ({}-{}) for age {} and biological sex {:?}",
                            bpm, adjusted_min_resting, adjusted_max_resting, age, chars.biological_sex
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
            None => {
                // Fallback to standard validation
                self.validate_with_config(config)
            }
        }
    }
}

/// Heart Rate Event Validation (STORY-011: Cardiac Event Detection)
impl HeartRateEvent {
    /// Validate heart rate event with medical-grade thresholds
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    /// Validate with configurable parameters
    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate heart rate at event (critical for all event types)
        if !(30..=300).contains(&self.heart_rate_at_event) {
            return Err(format!(
                "heart_rate_at_event {} is out of range (30-300 BPM)",
                self.heart_rate_at_event
            ));
        }

        // Validate event duration if provided
        if let Some(duration) = self.event_duration_minutes {
            if duration < 0 {
                return Err(format!(
                    "event_duration_minutes {} cannot be negative",
                    duration
                ));
            }

            // Maximum event duration: 24 hours for ultra-endurance scenarios
            if duration > 1440 {
                return Err(format!(
                    "event_duration_minutes {} exceeds maximum (1440 minutes = 24 hours)",
                    duration
                ));
            }
        }

        // Validate event type specific thresholds
        match self.event_type {
            HeartRateEventType::High => {
                // High HR events should have elevated heart rates
                if self.heart_rate_at_event < 100 {
                    return Err(format!(
                        "HIGH event type requires heart rate >= 100 BPM, got {}",
                        self.heart_rate_at_event
                    ));
                }
            },
            HeartRateEventType::Low => {
                // Low HR events should have bradycardia range
                if self.heart_rate_at_event > 60 {
                    return Err(format!(
                        "LOW event type requires heart rate <= 60 BPM, got {}",
                        self.heart_rate_at_event
                    ));
                }
            },
            HeartRateEventType::Afib => {
                // AFib typically occurs at irregular elevated rates
                if self.heart_rate_at_event < 60 {
                    return Err(format!(
                        "AFIB event type typically requires heart rate >= 60 BPM, got {}",
                        self.heart_rate_at_event
                    ));
                }
            },
            _ => {
                // Other event types (IRREGULAR, RAPID_INCREASE, SLOW_RECOVERY, EXERCISE_ANOMALY)
                // are pattern-based and don't have strict HR thresholds
            }
        }

        // Validate severity-duration consistency
        match self.severity {
            CardiacEventSeverity::Critical => {
                // Critical events should be detected quickly (within 5 minutes)
                if let Some(duration) = self.event_duration_minutes {
                    if duration > 5 && !self.is_confirmed {
                        return Err(
                            "CRITICAL severity events lasting > 5 minutes require medical confirmation".to_string()
                        );
                    }
                }
            },
            CardiacEventSeverity::High => {
                // High severity events lasting > 30 minutes should be confirmed
                if let Some(duration) = self.event_duration_minutes {
                    if duration > 30 && !self.is_confirmed {
                        return Err(
                            "HIGH severity events lasting > 30 minutes require medical confirmation".to_string()
                        );
                    }
                }
            },
            _ => {} // Low and Moderate severities have no duration constraints
        }

        Ok(())
    }

    /// Validate with personalized user characteristics for age-adjusted thresholds
    pub fn validate_with_characteristics(&self, config: &ValidationConfig, characteristics: Option<&UserCharacteristics>) -> Result<(), String> {
        // First run standard validation
        self.validate_with_config(config)?;

        if let Some(chars) = characteristics {
            let age = chars.age().unwrap_or(30);

            // Age-adjusted validation for HIGH and LOW events
            match self.event_type {
                HeartRateEventType::High => {
                    // Age-adjusted maximum heart rate (220 - age formula)
                    let max_hr = 220 - age as i16;
                    let tachycardia_threshold = (max_hr as f64 * 0.85) as i16; // 85% of max HR

                    if self.heart_rate_at_event < tachycardia_threshold {
                        return Err(format!(
                            "HIGH event for age {} requires heart rate >= {} BPM (85% of max), got {}",
                            age, tachycardia_threshold, self.heart_rate_at_event
                        ));
                    }
                },
                HeartRateEventType::Low => {
                    // Age-adjusted bradycardia thresholds
                    let bradycardia_threshold = match age {
                        age if age < 30 => 50,  // Younger adults: < 50 BPM
                        age if age < 60 => 55,  // Middle-aged: < 55 BPM
                        _ => 60,                // Older adults: < 60 BPM
                    };

                    if self.heart_rate_at_event > bradycardia_threshold {
                        return Err(format!(
                            "LOW event for age {} requires heart rate <= {} BPM, got {}",
                            age, bradycardia_threshold, self.heart_rate_at_event
                        ));
                    }
                },
                _ => {} // Other event types don't require age adjustment
            }
        }

        Ok(())
    }

    /// Get medical urgency assessment based on event characteristics
    pub fn get_medical_urgency(&self) -> &'static str {
        match (&self.event_type, &self.severity) {
            (HeartRateEventType::Afib, CardiacEventSeverity::Critical) =>
                "EMERGENCY: Sustained AFib with critical symptoms - seek immediate medical attention",
            (HeartRateEventType::High, CardiacEventSeverity::Critical) =>
                "EMERGENCY: Critical tachycardia - call emergency services",
            (HeartRateEventType::Low, CardiacEventSeverity::Critical) =>
                "EMERGENCY: Severe bradycardia - immediate cardiac evaluation required",
            (_, CardiacEventSeverity::High) =>
                "URGENT: Seek medical attention within 24 hours",
            (_, CardiacEventSeverity::Moderate) =>
                "MODERATE: Schedule medical consultation within 1-2 weeks",
            _ =>
                "LOW: Continue monitoring, discuss at next routine medical visit"
        }
    }

    /// Calculate cardiac event risk score (0-100 scale)
    pub fn calculate_risk_score(&self) -> u8 {
        let mut score = 0u8;

        // Base severity score
        score += match self.severity {
            CardiacEventSeverity::Low => 10,
            CardiacEventSeverity::Moderate => 30,
            CardiacEventSeverity::High => 60,
            CardiacEventSeverity::Critical => 90,
        };

        // Event type modifier
        score += match self.event_type {
            HeartRateEventType::Afib => 15,
            HeartRateEventType::High | HeartRateEventType::Low => 10,
            HeartRateEventType::Irregular => 8,
            HeartRateEventType::RapidIncrease => 5,
            HeartRateEventType::SlowRecovery => 3,
            HeartRateEventType::ExerciseAnomaly => 2,
        };

        // Duration modifier (longer events are more concerning)
        if let Some(duration) = self.event_duration_minutes {
            score += match duration {
                d if d > 60 => 10,   // > 1 hour
                d if d > 30 => 7,    // 30-60 minutes
                d if d > 10 => 5,    // 10-30 minutes
                d if d > 5 => 3,     // 5-10 minutes
                _ => 0,              // < 5 minutes
            };
        }

        // Cap at 100
        std::cmp::min(score, 100)
    }
}

impl BloodPressureMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

        // Validate specialized distance metrics
        if let Some(cycling_distance) = self.distance_cycling_meters {
            if cycling_distance < 0.0 {
                return Err("distance_cycling_meters cannot be negative".to_string());
            }
            let cycling_distance_km = cycling_distance / 1000.0;
            if cycling_distance_km > config.distance_max_km {
                return Err(format!(
                    "cycling distance {} km exceeds maximum of {} km",
                    cycling_distance_km, config.distance_max_km
                ));
            }
        }

        if let Some(swimming_distance) = self.distance_swimming_meters {
            if swimming_distance < 0.0 {
                return Err("distance_swimming_meters cannot be negative".to_string());
            }
            // Swimming distances typically much shorter than land activities
            let swimming_distance_km = swimming_distance / 1000.0;
            if swimming_distance_km > 50.0 {
                return Err(format!(
                    "swimming distance {} km exceeds reasonable maximum of 50 km",
                    swimming_distance_km
                ));
            }
        }

        if let Some(wheelchair_distance) = self.distance_wheelchair_meters {
            if wheelchair_distance < 0.0 {
                return Err("distance_wheelchair_meters cannot be negative".to_string());
            }
            let wheelchair_distance_km = wheelchair_distance / 1000.0;
            if wheelchair_distance_km > config.distance_max_km {
                return Err(format!(
                    "wheelchair distance {} km exceeds maximum of {} km",
                    wheelchair_distance_km, config.distance_max_km
                ));
            }
        }

        if let Some(snow_sports_distance) = self.distance_downhill_snow_sports_meters {
            if snow_sports_distance < 0.0 {
                return Err("distance_downhill_snow_sports_meters cannot be negative".to_string());
            }
            let snow_sports_distance_km = snow_sports_distance / 1000.0;
            if snow_sports_distance_km > 100.0 {
                return Err(format!(
                    "downhill snow sports distance {} km exceeds reasonable maximum of 100 km",
                    snow_sports_distance_km
                ));
            }
        }

        // Validate accessibility metrics
        if let Some(push_count) = self.push_count {
            if push_count < 0 || push_count > 50000 {
                return Err(format!(
                    "push_count {} is out of range (0-50000)",
                    push_count
                ));
            }
        }

        // Validate swimming analytics
        if let Some(stroke_count) = self.swimming_stroke_count {
            if stroke_count < 0 || stroke_count > 100000 {
                return Err(format!(
                    "swimming_stroke_count {} is out of range (0-100000)",
                    stroke_count
                ));
            }
        }

        // Validate Nike Fuel points
        if let Some(fuel_points) = self.nike_fuel_points {
            if fuel_points < 0 || fuel_points > 10000 {
                return Err(format!(
                    "nike_fuel_points {} is out of range (0-10000)",
                    fuel_points
                ));
            }
        }

        // Validate Apple Watch activity ring metrics
        if let Some(exercise_time) = self.apple_exercise_time_minutes {
            if exercise_time < 0 || exercise_time > 1440 {
                return Err(format!(
                    "apple_exercise_time_minutes {} is out of range (0-1440)",
                    exercise_time
                ));
            }
        }

        if let Some(stand_time) = self.apple_stand_time_minutes {
            if stand_time < 0 || stand_time > 1440 {
                return Err(format!(
                    "apple_stand_time_minutes {} is out of range (0-1440)",
                    stand_time
                ));
            }
        }

        if let Some(move_time) = self.apple_move_time_minutes {
            if move_time < 0 || move_time > 1440 {
                return Err(format!(
                    "apple_move_time_minutes {} is out of range (0-1440)",
                    move_time
                ));
            }
        }

        Ok(())
    }

    /// Personalized validation using user characteristics (wheelchair adaptations)
    pub fn validate_with_characteristics(&self, config: &ValidationConfig, characteristics: Option<&UserCharacteristics>) -> Result<(), String> {
        match characteristics {
            Some(chars) => {
                // Wheelchair users have different activity expectations
                let step_max = if chars.wheelchair_use { 10000 } else { config.step_count_max };
                let distance_max_km = if chars.wheelchair_use { 100.0 } else { config.distance_max_km };

                if let Some(step_count) = self.step_count {
                    // For wheelchair users, step count may be much lower or even zero
                    if chars.wheelchair_use {
                        // More lenient validation for wheelchair users
                        if step_count < 0 || step_count > step_max {
                            return Err(format!(
                                "step_count {} is outside adapted range (0-{}) for wheelchair user",
                                step_count, step_max
                            ));
                        }
                    } else {
                        if step_count < config.step_count_min || step_count > step_max {
                            return Err(format!(
                                "step_count {} is out of range ({}-{})",
                                step_count, config.step_count_min, step_max
                            ));
                        }
                    }
                }

                if let Some(distance) = self.distance_meters {
                    if distance < 0.0 {
                        return Err("distance_meters cannot be negative".to_string());
                    }
                    let distance_km = distance / 1000.0;
                    if distance_km > distance_max_km {
                        return Err(format!(
                            "distance {} km exceeds {} maximum of {} km",
                            distance_km,
                            if chars.wheelchair_use { "wheelchair-adapted" } else { "standard" },
                            distance_max_km
                        ));
                    }
                }

                // Energy validation remains the same but with context
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

                // Flights climbed may not apply to wheelchair users
                if let Some(flights) = self.flights_climbed {
                    if chars.wheelchair_use && flights > 0 {
                        // Note: Some wheelchair users may still climb flights (e.g., manual wheelchair on ramps)
                        // So we allow it but with a lower maximum
                        if flights > 100 {
                            return Err(format!(
                                "flights_climbed {} seems unusually high for wheelchair user (max 100)",
                                flights
                            ));
                        }
                    } else if !(0..=10000).contains(&flights) {
                        return Err(format!(
                            "flights_climbed {flights} is out of range (0-10000)"
                        ));
                    }
                }

                Ok(())
            }
            None => {
                // Fallback to standard validation
                self.validate_with_config(config)
            }
        }
    }
}

impl BodyMeasurementMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
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

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        match self {
            HealthMetric::HeartRate(metric) => metric.validate_with_config(config),
            HealthMetric::BloodPressure(metric) => metric.validate_with_config(config),
            HealthMetric::Sleep(metric) => metric.validate_with_config(config),
            HealthMetric::Activity(metric) => metric.validate_with_config(config),
            HealthMetric::BodyMeasurement(metric) => metric.validate_with_config(config),
            HealthMetric::Temperature(metric) => metric.validate_with_config(config),
            HealthMetric::BloodGlucose(metric) => metric.validate_with_config(config),
            HealthMetric::Metabolic(metric) => metric.validate_with_config(config),
            HealthMetric::Respiratory(metric) => metric.validate_with_config(config),
            HealthMetric::Nutrition(metric) => metric.validate_with_config(config),
            HealthMetric::Workout(workout) => workout.validate_with_config(config),
            HealthMetric::Environmental(metric) => metric.validate_with_config(config),
            HealthMetric::AudioExposure(metric) => metric.validate_with_config(config),
            HealthMetric::SafetyEvent(metric) => metric.validate_with_config(config),
            HealthMetric::Mindfulness(metric) => metric.validate_with_config(config),
            HealthMetric::MentalHealth(metric) => metric.validate_with_config(config),
            HealthMetric::Menstrual(metric) => metric.validate_with_config(config),
            HealthMetric::Fertility(metric) => metric.validate_with_config(config),
            HealthMetric::Hygiene(metric) => metric.validate_with_config(config),
            HealthMetric::Symptom(metric) => metric.validate_with_config(config),
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
            HealthMetric::Metabolic(_) => "Metabolic",
            HealthMetric::Respiratory(_) => "Respiratory",
            HealthMetric::Nutrition(_) => "Nutrition",
            HealthMetric::Workout(_) => "Workout",
            HealthMetric::Environmental(_) => "Environmental",
            HealthMetric::AudioExposure(_) => "AudioExposure",
            HealthMetric::SafetyEvent(_) => "SafetyEvent",
            HealthMetric::Mindfulness(_) => "Mindfulness",
            HealthMetric::MentalHealth(_) => "MentalHealth",
            HealthMetric::Menstrual(_) => "Menstrual",
            HealthMetric::Fertility(_) => "Fertility",
            HealthMetric::Symptom(_) => "Symptom",
            HealthMetric::Hygiene(_) => "Hygiene",
        }
    }
}

/// Macronutrient calorie distribution for nutritional analysis
#[derive(Debug, Serialize, Clone)]
pub struct MacronutrientDistribution {
    pub carbohydrate_percent: u8,
    pub protein_percent: u8,
    pub fat_percent: u8,
}


/// Symptom analysis summary for health insights
#[derive(Debug, Serialize, Clone)]
pub struct SymptomAnalysis {
    pub symptom_type: SymptomType,
    pub severity: SymptomSeverity,
    pub category: String,
    pub is_emergency: bool,
    pub requires_attention: bool,
    pub severity_score: i32,
    pub duration_hours: Option<f64>,
    pub recommendations: Vec<String>,
}


impl NutritionMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate hydration & stimulants
        if let Some(water) = self.dietary_water {
            if water < 0.0 || water > 10.0 {
                return Err(format!(
                    "dietary_water {} liters is out of reasonable range (0-10 L/day)",
                    water
                ));
            }
        }

        if let Some(caffeine) = self.dietary_caffeine {
            if caffeine < 0.0 || caffeine > 1000.0 {
                return Err(format!(
                    "dietary_caffeine {} mg is out of reasonable range (0-1000 mg/day)",
                    caffeine
                ));
            }
        }

        // Validate macronutrients (core energy)
        if let Some(energy) = self.dietary_energy_consumed {
            if energy < 0.0 || energy > 10000.0 {
                return Err(format!(
                    "dietary_energy_consumed {} calories is out of reasonable range (0-10000 cal/day)",
                    energy
                ));
            }
        }

        if let Some(carbs) = self.dietary_carbohydrates {
            if carbs < 0.0 || carbs > 2000.0 {
                return Err(format!(
                    "dietary_carbohydrates {} grams is out of reasonable range (0-2000 g/day)",
                    carbs
                ));
            }
        }

        if let Some(protein) = self.dietary_protein {
            if protein < 0.0 || protein > 1000.0 {
                return Err(format!(
                    "dietary_protein {} grams is out of reasonable range (0-1000 g/day)",
                    protein
                ));
            }
        }

        if let Some(fat) = self.dietary_fat_total {
            if fat < 0.0 || fat > 1000.0 {
                return Err(format!(
                    "dietary_fat_total {} grams is out of reasonable range (0-1000 g/day)",
                    fat
                ));
            }
        }

        if let Some(saturated_fat) = self.dietary_fat_saturated {
            if saturated_fat < 0.0 || saturated_fat > 500.0 {
                return Err(format!(
                    "dietary_fat_saturated {} grams is out of reasonable range (0-500 g/day)",
                    saturated_fat
                ));
            }
        }

        if let Some(mono_fat) = self.dietary_fat_monounsaturated {
            if mono_fat < 0.0 || mono_fat > 500.0 {
                return Err(format!(
                    "dietary_fat_monounsaturated {} grams is out of reasonable range (0-500 g/day)",
                    mono_fat
                ));
            }
        }

        if let Some(poly_fat) = self.dietary_fat_polyunsaturated {
            if poly_fat < 0.0 || poly_fat > 500.0 {
                return Err(format!(
                    "dietary_fat_polyunsaturated {} grams is out of reasonable range (0-500 g/day)",
                    poly_fat
                ));
            }
        }

        if let Some(cholesterol) = self.dietary_cholesterol {
            if cholesterol < 0.0 || cholesterol > 2000.0 {
                return Err(format!(
                    "dietary_cholesterol {} mg is out of reasonable range (0-2000 mg/day)",
                    cholesterol
                ));
            }
        }

        if let Some(sodium) = self.dietary_sodium {
            if sodium < 0.0 || sodium > 10000.0 {
                return Err(format!(
                    "dietary_sodium {} mg is out of reasonable range (0-10000 mg/day)",
                    sodium
                ));
            }
        }

        if let Some(fiber) = self.dietary_fiber {
            if fiber < 0.0 || fiber > 200.0 {
                return Err(format!(
                    "dietary_fiber {} grams is out of reasonable range (0-200 g/day)",
                    fiber
                ));
            }
        }

        if let Some(sugar) = self.dietary_sugar {
            if sugar < 0.0 || sugar > 1000.0 {
                return Err(format!(
                    "dietary_sugar {} grams is out of reasonable range (0-1000 g/day)",
                    sugar
                ));
            }
        }

        // Validate essential minerals
        if let Some(calcium) = self.dietary_calcium {
            if calcium < 0.0 || calcium > 5000.0 {
                return Err(format!(
                    "dietary_calcium {} mg is out of reasonable range (0-5000 mg/day)",
                    calcium
                ));
            }
        }

        if let Some(iron) = self.dietary_iron {
            if iron < 0.0 || iron > 100.0 {
                return Err(format!(
                    "dietary_iron {} mg is out of reasonable range (0-100 mg/day)",
                    iron
                ));
            }
        }

        if let Some(magnesium) = self.dietary_magnesium {
            if magnesium < 0.0 || magnesium > 2000.0 {
                return Err(format!(
                    "dietary_magnesium {} mg is out of reasonable range (0-2000 mg/day)",
                    magnesium
                ));
            }
        }

        if let Some(potassium) = self.dietary_potassium {
            if potassium < 0.0 || potassium > 10000.0 {
                return Err(format!(
                    "dietary_potassium {} mg is out of reasonable range (0-10000 mg/day)",
                    potassium
                ));
            }
        }

        if let Some(zinc) = self.dietary_zinc {
            if zinc < 0.0 || zinc > 100.0 {
                return Err(format!(
                    "dietary_zinc {} mg is out of reasonable range (0-100 mg/day)",
                    zinc
                ));
            }
        }

        if let Some(phosphorus) = self.dietary_phosphorus {
            if phosphorus < 0.0 || phosphorus > 5000.0 {
                return Err(format!(
                    "dietary_phosphorus {} mg is out of reasonable range (0-5000 mg/day)",
                    phosphorus
                ));
            }
        }

        // Validate water-soluble vitamins
        if let Some(vitamin_c) = self.dietary_vitamin_c {
            if vitamin_c < 0.0 || vitamin_c > 5000.0 {
                return Err(format!(
                    "dietary_vitamin_c {} mg is out of reasonable range (0-5000 mg/day)",
                    vitamin_c
                ));
            }
        }

        if let Some(b1) = self.dietary_vitamin_b1_thiamine {
            if b1 < 0.0 || b1 > 100.0 {
                return Err(format!(
                    "dietary_vitamin_b1_thiamine {} mg is out of reasonable range (0-100 mg/day)",
                    b1
                ));
            }
        }

        if let Some(b2) = self.dietary_vitamin_b2_riboflavin {
            if b2 < 0.0 || b2 > 100.0 {
                return Err(format!(
                    "dietary_vitamin_b2_riboflavin {} mg is out of reasonable range (0-100 mg/day)",
                    b2
                ));
            }
        }

        if let Some(b3) = self.dietary_vitamin_b3_niacin {
            if b3 < 0.0 || b3 > 1000.0 {
                return Err(format!(
                    "dietary_vitamin_b3_niacin {} mg is out of reasonable range (0-1000 mg/day)",
                    b3
                ));
            }
        }

        if let Some(b6) = self.dietary_vitamin_b6_pyridoxine {
            if b6 < 0.0 || b6 > 200.0 {
                return Err(format!(
                    "dietary_vitamin_b6_pyridoxine {} mg is out of reasonable range (0-200 mg/day)",
                    b6
                ));
            }
        }

        if let Some(b12) = self.dietary_vitamin_b12_cobalamin {
            if b12 < 0.0 || b12 > 2000.0 {
                return Err(format!(
                    "dietary_vitamin_b12_cobalamin {} mcg is out of reasonable range (0-2000 mcg/day)",
                    b12
                ));
            }
        }

        if let Some(folate) = self.dietary_folate {
            if folate < 0.0 || folate > 5000.0 {
                return Err(format!(
                    "dietary_folate {} mcg is out of reasonable range (0-5000 mcg/day)",
                    folate
                ));
            }
        }

        if let Some(biotin) = self.dietary_biotin {
            if biotin < 0.0 || biotin > 5000.0 {
                return Err(format!(
                    "dietary_biotin {} mcg is out of reasonable range (0-5000 mcg/day)",
                    biotin
                ));
            }
        }

        if let Some(pantothenic) = self.dietary_pantothenic_acid {
            if pantothenic < 0.0 || pantothenic > 100.0 {
                return Err(format!(
                    "dietary_pantothenic_acid {} mg is out of reasonable range (0-100 mg/day)",
                    pantothenic
                ));
            }
        }

        // Validate fat-soluble vitamins
        if let Some(vitamin_a) = self.dietary_vitamin_a {
            if vitamin_a < 0.0 || vitamin_a > 10000.0 {
                return Err(format!(
                    "dietary_vitamin_a {} mcg is out of reasonable range (0-10000 mcg/day)",
                    vitamin_a
                ));
            }
        }

        if let Some(vitamin_d) = self.dietary_vitamin_d {
            if vitamin_d < 0.0 || vitamin_d > 10000.0 {
                return Err(format!(
                    "dietary_vitamin_d {} IU is out of reasonable range (0-10000 IU/day)",
                    vitamin_d
                ));
            }
        }

        if let Some(vitamin_e) = self.dietary_vitamin_e {
            if vitamin_e < 0.0 || vitamin_e > 1000.0 {
                return Err(format!(
                    "dietary_vitamin_e {} mg is out of reasonable range (0-1000 mg/day)",
                    vitamin_e
                ));
            }
        }

        if let Some(vitamin_k) = self.dietary_vitamin_k {
            if vitamin_k < 0.0 || vitamin_k > 5000.0 {
                return Err(format!(
                    "dietary_vitamin_k {} mcg is out of reasonable range (0-5000 mcg/day)",
                    vitamin_k
                ));
            }
        }

        // Validate meal context
        if let Some(meal_type) = &self.meal_type {
            let valid_meal_types = ["breakfast", "lunch", "dinner", "snack", "other"];
            if !valid_meal_types.contains(&meal_type.as_str()) {
                return Err(format!(
                    "meal_type '{}' is not valid. Must be one of: {}",
                    meal_type,
                    valid_meal_types.join(", ")
                ));
            }
        }

        Ok(())
    }

    /// Check if this is a high-hydration reading
    pub fn is_high_hydration(&self) -> bool {
        self.dietary_water.map_or(false, |water| water > 3.0) // >3L per day
    }

    /// Check if this exceeds recommended daily caffeine intake
    pub fn exceeds_caffeine_limit(&self) -> bool {
        self.dietary_caffeine.map_or(false, |caffeine| caffeine > 400.0) // >400mg/day
    }

    /// Calculate approximate macronutrient calorie distribution
    pub fn macronutrient_distribution(&self) -> Option<MacronutrientDistribution> {
        let energy = self.dietary_energy_consumed?;
        if energy <= 0.0 {
            return None;
        }

        let carb_calories = self.dietary_carbohydrates.unwrap_or(0.0) * 4.0;
        let protein_calories = self.dietary_protein.unwrap_or(0.0) * 4.0;
        let fat_calories = self.dietary_fat_total.unwrap_or(0.0) * 9.0;

        Some(MacronutrientDistribution {
            carbohydrate_percent: (carb_calories / energy * 100.0) as u8,
            protein_percent: (protein_calories / energy * 100.0) as u8,
            fat_percent: (fat_calories / energy * 100.0) as u8,
        })
    }

    /// Check if sodium intake is excessive (>2300mg recommended daily limit)
    pub fn has_excessive_sodium(&self) -> bool {
        self.dietary_sodium.map_or(false, |sodium| sodium > 2300.0)
    }

    /// Get hydration status based on water intake
    pub fn hydration_status(&self) -> &'static str {
        match self.dietary_water {
            Some(water) if water < 1.0 => "severely_dehydrated",
            Some(water) if water < 2.0 => "dehydrated",
            Some(water) if water < 3.0 => "adequate",
            Some(water) if water > 5.0 => "overhydrated",
            Some(_) => "well_hydrated",
            None => "unknown",
        }
    }

    /// Check if this represents a balanced meal based on macronutrient distribution
    pub fn is_balanced_meal(&self) -> bool {
        if let Some(distribution) = self.macronutrient_distribution() {
            // Balanced meal: 45-65% carbs, 10-35% protein, 20-35% fat
            distribution.carbohydrate_percent >= 45 && distribution.carbohydrate_percent <= 65 &&
            distribution.protein_percent >= 10 && distribution.protein_percent <= 35 &&
            distribution.fat_percent >= 20 && distribution.fat_percent <= 35
        } else {
            false
        }
    }
}


/// Symptom Metric Validation and Analysis
impl SymptomMetric {
    /// Validate symptom metric with configurable parameters
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    /// Validate symptom metric with custom configuration
    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate duration is reasonable (0 to 2 weeks max for most symptoms)
        if let Some(duration) = self.duration_minutes {
            if duration < 0 {
                return Err("symptom duration cannot be negative".to_string());
            }

            // Most symptoms shouldn't last more than 2 weeks continuously
            let max_duration_minutes = 14 * 24 * 60; // 2 weeks in minutes
            if duration > max_duration_minutes {
                return Err(format!(
                    "symptom duration {} minutes is unreasonably long (max {} minutes = 2 weeks)",
                    duration, max_duration_minutes
                ));
            }
        }

        // Validate severity consistency with symptom type
        if self.symptom_type.is_critical() && self.severity == SymptomSeverity::None {
            return Err(format!(
                "Critical symptom type {} cannot have 'none' severity",
                self.symptom_type
            ));
        }

        // Check for medical emergency combinations
        if self.severity.is_critical() && !self.symptom_type.is_critical() {
            // Log warning for non-critical symptoms with critical severity
            // This might indicate user error or actual medical emergency
        }

        Ok(())
    }

    /// Get symptom category for grouping and analysis
    pub fn get_category(&self) -> &'static str {
        self.symptom_type.get_category()
    }

    /// Check if this symptom indicates a potential medical emergency
    pub fn is_medical_emergency(&self) -> bool {
        // Critical severity always indicates emergency
        if self.severity.is_critical() {
            return true;
        }

        // Critical symptom types with severe+ severity indicate emergency
        if self.symptom_type.is_critical() && self.severity >= SymptomSeverity::Severe {
            return true;
        }

        // Specific combinations that indicate emergency
        match (&self.symptom_type, &self.severity) {
            (SymptomType::ChestTightnessOrPain, SymptomSeverity::Moderate) => true,
            (SymptomType::ShortnessOfBreath, SymptomSeverity::Moderate) => true,
            (SymptomType::ChestPain, SymptomSeverity::Moderate) => true,
            (SymptomType::Fever, SymptomSeverity::Severe) => true, // High fever
            (SymptomType::RapidHeartRate, SymptomSeverity::Severe) => true,
            _ => false,
        }
    }

    /// Check if this symptom requires medical attention (non-emergency)
    pub fn requires_medical_attention(&self) -> bool {
        // Emergency conditions already require attention
        if self.is_medical_emergency() {
            return true;
        }

        // Severe symptoms generally require medical attention
        if self.severity >= SymptomSeverity::Severe {
            return true;
        }

        // Long-duration symptoms might require attention
        if let Some(duration) = self.duration_minutes {
            let hours = duration as f64 / 60.0;
            match self.symptom_type {
                // Persistent pain over 24 hours
                SymptomType::AbdominalCramps | SymptomType::Headache | SymptomType::BackPain |
                SymptomType::MusclePain | SymptomType::JointPain => hours > 24.0,

                // Respiratory symptoms over 12 hours
                SymptomType::Coughing | SymptomType::ShortnessOfBreath | SymptomType::Wheezing => hours > 12.0,

                // Digestive symptoms over 48 hours
                SymptomType::Nausea | SymptomType::Vomiting | SymptomType::Diarrhea => hours > 48.0,

                // Fever over 72 hours
                SymptomType::Fever => hours > 72.0,

                // Other symptoms default to 1 week
                _ => hours > 168.0, // 1 week
            }
        } else {
            false
        }
    }

    /// Generate symptom analysis summary
    pub fn generate_analysis(&self) -> SymptomAnalysis {
        SymptomAnalysis {
            symptom_type: self.symptom_type,
            severity: self.severity,
            category: self.get_category().to_string(),
            is_emergency: self.is_medical_emergency(),
            requires_attention: self.requires_medical_attention(),
            severity_score: self.severity.to_numeric_score(),
            duration_hours: self.duration_minutes.map(|m| m as f64 / 60.0),
            recommendations: self.generate_recommendations(),
        }
    }

    /// Generate contextual recommendations based on symptom
    pub fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.is_medical_emergency() {
            recommendations.push("Seek immediate medical attention".to_string());
            recommendations.push("Consider calling emergency services".to_string());
        } else if self.requires_medical_attention() {
            recommendations.push("Consult with healthcare provider".to_string());
            recommendations.push("Monitor symptom progression".to_string());
        }

        // Category-specific recommendations
        match self.symptom_type.get_category() {
            "respiratory" => {
                recommendations.push("Ensure good air quality".to_string());
                recommendations.push("Stay hydrated".to_string());
                if self.severity >= SymptomSeverity::Moderate {
                    recommendations.push("Avoid strenuous activity".to_string());
                }
            },
            "digestive" => {
                recommendations.push("Stay hydrated".to_string());
                recommendations.push("Consider bland diet".to_string());
                recommendations.push("Monitor for dehydration signs".to_string());
            },
            "pain" => {
                recommendations.push("Rest affected area if possible".to_string());
                recommendations.push("Consider pain management strategies".to_string());
                if self.duration_minutes.map_or(false, |d| d > 720) { // >12 hours
                    recommendations.push("Track pain patterns".to_string());
                }
            },
            "neurological" => {
                recommendations.push("Ensure adequate rest".to_string());
                recommendations.push("Monitor cognitive symptoms".to_string());
                recommendations.push("Consider stress management".to_string());
            },
            "cardiovascular" => {
                recommendations.push("Monitor vital signs".to_string());
                recommendations.push("Avoid strenuous activity".to_string());
                recommendations.push("Consider cardiac evaluation".to_string());
            },
            _ => {
                recommendations.push("Monitor symptom progression".to_string());
                recommendations.push("Rest and self-care".to_string());
            }
        }

        // Duration-based recommendations
        if let Some(duration) = self.duration_minutes {
            let hours = duration as f64 / 60.0;
            if hours > 72.0 { // >3 days
                recommendations.push("Consider medical evaluation for persistent symptoms".to_string());
            }
        }

        recommendations
    }

    /// Check if this symptom is part of a potential illness episode
    pub fn is_episode_symptom(&self) -> bool {
        self.episode_id.is_some()
    }

    /// Get symptom urgency level (0-5 scale)
    pub fn get_urgency_level(&self) -> u8 {
        if self.is_medical_emergency() {
            return 5; // Maximum urgency
        }

        match self.severity {
            SymptomSeverity::None => 0,
            SymptomSeverity::Mild => 1,
            SymptomSeverity::Moderate => 2,
            SymptomSeverity::Severe => 4,
            SymptomSeverity::Critical => 5,
        }
    }
}


impl MetabolicMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate blood alcohol content if provided (0.0-0.5%)
        if let Some(bac) = self.blood_alcohol_content {
            if bac < 0.0 || bac > 0.5 {
                return Err(format!(
                    "blood_alcohol_content {} is outside valid range (0.0-0.5%)",
                    bac
                ));
            }
        }

        // Validate insulin units if provided
        if let Some(insulin_units) = self.insulin_delivery_units {
            if insulin_units < 0.0 || insulin_units > 100.0 {
                return Err(format!(
                    "insulin_delivery_units {} is outside safe range (0-100 units)",
                    insulin_units
                ));
            }
        }

        // Validate delivery method if provided
        if let Some(method) = &self.delivery_method {
            let valid_methods = ["pump", "pen", "syringe", "inhaler", "patch"];
            if !valid_methods.contains(&method.as_str()) {
                return Err(format!(
                    "delivery_method '{}' is invalid. Valid methods: {:?}",
                    method, valid_methods
                ));
            }
        }

        Ok(())
    }

    /// Check if blood alcohol content indicates intoxication (>0.08%)
    pub fn indicates_intoxication(&self) -> bool {
        self.blood_alcohol_content.map_or(false, |bac| bac > 0.08)
    }

    /// Check if this is a significant insulin delivery (>10 units)
    pub fn is_significant_insulin_delivery(&self) -> bool {
        self.insulin_delivery_units.map_or(false, |units| units > 10.0)
    }

    /// Get alcohol impairment level based on BAC
    pub fn alcohol_impairment_level(&self) -> &'static str {
        match self.blood_alcohol_content {
            Some(bac) if bac == 0.0 => "sober",
            Some(bac) if bac < 0.05 => "minimal",
            Some(bac) if bac < 0.08 => "impaired",
            Some(bac) if bac < 0.15 => "intoxicated",
            Some(bac) if bac < 0.30 => "severely_intoxicated",
            Some(_) => "life_threatening",
            None => "unknown",
        }
    }
}

/// Hygiene event metric for behavior tracking and public health monitoring
#[derive(Debug, Deserialize, Serialize, Clone, FromRow)]
pub struct HygieneMetric {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub recorded_at: DateTime<Utc>,

    // Core Hygiene Event Data
    pub event_type: HygieneEventType,
    pub duration_seconds: Option<i32>,
    pub quality_rating: Option<i16>, // 1-5 self-reported quality

    // Public Health & Compliance Tracking
    pub meets_who_guidelines: Option<bool>,
    pub frequency_compliance_rating: Option<i16>, // 1-5 daily frequency adherence

    // Smart Device Integration
    pub device_detected: Option<bool>,
    pub device_effectiveness_score: Option<f64>, // 0-100% device-measured effectiveness

    // Context & Behavioral Analysis
    pub trigger_event: Option<String>,
    pub location_context: Option<String>,
    pub compliance_motivation: Option<String>,

    // Health Crisis Integration
    pub health_crisis_enhanced: Option<bool>,
    pub crisis_compliance_level: Option<i16>, // 1-5 adherence to crisis protocols

    // Gamification & Habit Tracking
    pub streak_count: Option<i32>,
    pub daily_goal_progress: Option<i16>, // 0-200% of daily hygiene goals met
    pub achievement_unlocked: Option<String>,

    // Medical Integration
    pub medication_adherence_related: Option<bool>,
    pub medical_condition_context: Option<String>,

    // Privacy & Data Sensitivity
    pub data_sensitivity_level: Option<String>,

    // Metadata
    pub source_device: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl HygieneMetric {
    /// Validate hygiene metric data with configurable thresholds
    pub fn validate_with_config(&self, _config: &ValidationConfig) -> Result<(), String> {
        // Validate duration if provided
        if let Some(duration) = self.duration_seconds {
            if duration < 1 || duration > 7200 { // 1 second to 2 hours
                return Err(format!(
                    "duration_seconds {} is outside valid range (1-7200 seconds)",
                    duration
                ));
            }
        }

        // Validate quality rating if provided
        if let Some(quality) = self.quality_rating {
            if quality < 1 || quality > 5 {
                return Err(format!(
                    "quality_rating {} is outside valid range (1-5)",
                    quality
                ));
            }
        }

        // Validate frequency compliance rating if provided
        if let Some(freq_rating) = self.frequency_compliance_rating {
            if freq_rating < 1 || freq_rating > 5 {
                return Err(format!(
                    "frequency_compliance_rating {} is outside valid range (1-5)",
                    freq_rating
                ));
            }
        }

        // Validate device effectiveness score if provided
        if let Some(effectiveness) = self.device_effectiveness_score {
            if effectiveness < 0.0 || effectiveness > 100.0 {
                return Err(format!(
                    "device_effectiveness_score {} is outside valid range (0.0-100.0%)",
                    effectiveness
                ));
            }
        }

        // Validate crisis compliance level if provided
        if let Some(crisis_level) = self.crisis_compliance_level {
            if crisis_level < 1 || crisis_level > 5 {
                return Err(format!(
                    "crisis_compliance_level {} is outside valid range (1-5)",
                    crisis_level
                ));
            }
        }

        // Validate daily goal progress if provided
        if let Some(progress) = self.daily_goal_progress {
            if progress < 0 || progress > 200 {
                return Err(format!(
                    "daily_goal_progress {} is outside valid range (0-200%)",
                    progress
                ));
            }
        }

        // Validate data sensitivity level
        if let Some(sensitivity) = &self.data_sensitivity_level {
            let valid_levels = ["standard", "medical", "crisis_tracking"];
            if !valid_levels.contains(&sensitivity.as_str()) {
                return Err(format!(
                    "data_sensitivity_level '{}' is invalid. Valid levels: {:?}",
                    sensitivity, valid_levels
                ));
            }
        }

        Ok(())
    }

    /// Check if hygiene event meets WHO/CDC guidelines
    pub fn meets_health_guidelines(&self) -> bool {
        if let Some(duration) = self.duration_seconds {
            if let Some(recommended_duration) = self.event_type.get_recommended_duration() {
                return duration >= recommended_duration as i32;
            }
        }
        false
    }

    /// Get compliance score based on duration and frequency
    pub fn calculate_compliance_score(&self) -> f64 {
        let mut score = 0.0;
        let mut components = 0;

        // Duration compliance (40% of score)
        if self.meets_health_guidelines() {
            score += 0.4;
        }
        components += 1;

        // Quality rating (30% of score)
        if let Some(quality) = self.quality_rating {
            score += (quality as f64 / 5.0) * 0.3;
            components += 1;
        }

        // Device effectiveness (20% of score if available)
        if let Some(effectiveness) = self.device_effectiveness_score {
            score += (effectiveness / 100.0) * 0.2;
            components += 1;
        }

        // Frequency compliance (10% of score)
        if let Some(freq_rating) = self.frequency_compliance_rating {
            score += (freq_rating as f64 / 5.0) * 0.1;
            components += 1;
        }

        if components > 0 {
            score
        } else {
            0.0
        }
    }

    /// Check if this is a critical hygiene event for infection prevention
    pub fn is_critical_for_infection_prevention(&self) -> bool {
        self.event_type.is_critical_for_infection_prevention()
    }

    /// Get hygiene category for analytics
    pub fn get_hygiene_category(&self) -> &'static str {
        self.event_type.get_category()
    }

    /// Check if event was detected by smart device
    pub fn was_device_detected(&self) -> bool {
        self.device_detected.unwrap_or(false)
    }

    /// Check if event was during health crisis period
    pub fn was_during_health_crisis(&self) -> bool {
        self.health_crisis_enhanced.unwrap_or(false)
    }

    /// Get achievement status if any unlocked
    pub fn has_achievement(&self) -> Option<&str> {
        self.achievement_unlocked.as_deref()
    }

    /// Calculate habit strength based on streak count
    pub fn habit_strength(&self) -> &'static str {
        match self.streak_count.unwrap_or(0) {
            0..=2 => "forming",
            3..=6 => "developing",
            7..=20 => "established",
            21..=65 => "strong",
            _ => "ingrained"
        }
    }

    /// Check if hygiene event requires medical attention context
    pub fn requires_medical_context(&self) -> bool {
        self.medication_adherence_related.unwrap_or(false) ||
        self.medical_condition_context.is_some()
    }
}
