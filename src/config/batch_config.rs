use std::env;

/// PostgreSQL parameter limit constants for batch processing optimization
const _POSTGRESQL_MAX_PARAMS: usize = 65535; // PostgreSQL absolute maximum
pub const SAFE_PARAM_LIMIT: usize = 52428; // 80% of max for safety margin

/// Parameter counts per metric type for chunk size calculations
pub const HEART_RATE_PARAMS_PER_RECORD: usize = 10; // user_id, recorded_at, heart_rate, resting_heart_rate, heart_rate_variability, walking_heart_rate_average, heart_rate_recovery_one_minute, atrial_fibrillation_burden_percentage, vo2_max_ml_kg_min, context, source_device
pub const BLOOD_PRESSURE_PARAMS_PER_RECORD: usize = 6; // user_id, recorded_at, systolic, diastolic, pulse, source_device
pub const SLEEP_PARAMS_PER_RECORD: usize = 10; // user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, efficiency, source_device
pub const ACTIVITY_PARAMS_PER_RECORD: usize = 19; // user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, basal_energy_burned_kcal, flights_climbed, distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters, distance_downhill_snow_sports_meters, push_count, swimming_stroke_count, nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes, apple_move_time_minutes, apple_stand_hour_achieved, source_device
pub const BODY_MEASUREMENT_PARAMS_PER_RECORD: usize = 16; // user_id, recorded_at, body_weight_kg, body_mass_index, body_fat_percentage, lean_body_mass_kg, height_cm, waist_circumference_cm, hip_circumference_cm, chest_circumference_cm, arm_circumference_cm, thigh_circumference_cm, body_temperature_celsius, basal_body_temperature_celsius, measurement_source, source_device
pub const TEMPERATURE_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, body_temperature, basal_body_temperature, apple_sleeping_wrist_temperature, water_temperature, temperature_source, source_device
pub const RESPIRATORY_PARAMS_PER_RECORD: usize = 7; // user_id, recorded_at, respiratory_rate, oxygen_saturation, forced_vital_capacity, forced_expiratory_volume_1, peak_expiratory_flow_rate, inhaler_usage, source_device
pub const WORKOUT_PARAMS_PER_RECORD: usize = 10; // id, user_id, workout_type, started_at, ended_at, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source_device
pub const BLOOD_GLUCOSE_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, blood_glucose_mg_dl, measurement_context, medication_taken, insulin_delivery_units, glucose_source, source_device
pub const NUTRITION_PARAMS_PER_RECORD: usize = 32; // user_id, recorded_at, 25+ nutrient fields, meal_type, meal_id, source_device, created_at (32 total params for comprehensive nutrition)

// Reproductive Health Parameters (HIPAA-Compliant with Privacy Controls)
pub const MENSTRUAL_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, menstrual_flow, spotting, cycle_day, cramps_severity, mood_rating, energy_level, notes, source_device
pub const FERTILITY_PARAMS_PER_RECORD: usize = 12; // user_id, recorded_at, cervical_mucus_quality, ovulation_test_result, sexual_activity, pregnancy_test_result, basal_body_temperature, temperature_context, cervix_firmness, cervix_position, lh_level, notes, source_device

// Environmental and Audio Exposure Parameters
pub const ENVIRONMENTAL_PARAMS_PER_RECORD: usize = 14; // user_id, recorded_at, environmental_audio_exposure_db, headphone_audio_exposure_db, uv_index, uv_exposure_minutes, ambient_temperature_celsius, humidity_percent, air_pressure_hpa, altitude_meters, time_in_daylight_minutes, location_latitude, location_longitude, source_device
pub const AUDIO_EXPOSURE_PARAMS_PER_RECORD: usize = 7; // user_id, recorded_at, environmental_audio_exposure_db, headphone_audio_exposure_db, exposure_duration_minutes, audio_exposure_event, source_device

// Mental Health and Safety Parameters
pub const SAFETY_EVENT_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, event_type, severity_level, location, description, emergency_contact_notified, source_device
pub const MINDFULNESS_PARAMS_PER_RECORD: usize = 9; // user_id, recorded_at, session_type, duration_minutes, stress_level_before, stress_level_after, focus_rating, notes, source_device
pub const MENTAL_HEALTH_PARAMS_PER_RECORD: usize = 10; // user_id, recorded_at, mood_rating, anxiety_level, stress_level, energy_level, sleep_quality_perception, medication_taken, therapy_session, notes, source_device
pub const SYMPTOM_PARAMS_PER_RECORD: usize = 9; // user_id, recorded_at, symptom_type, severity_rating, location, duration_minutes, triggers, relief_factors, notes, source_device
pub const HYGIENE_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, activity_type, frequency, duration_minutes, quality_rating, notes, source_device

/// Configuration for batch processing operations
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub enable_parallel_processing: bool,
    pub chunk_size: usize,
    pub memory_limit_mb: f64,
    // Chunking configurations to stay under PostgreSQL 65,535 parameter limit
    pub heart_rate_chunk_size: usize, // 6 params per record -> max 10,922
    pub blood_pressure_chunk_size: usize, // 6 params per record -> max 10,922
    pub sleep_chunk_size: usize,      // 10 params per record -> max 6,553
    pub activity_chunk_size: usize,   // 8 params per record -> max 8,178
    pub body_measurement_chunk_size: usize, // 14 params per record -> max 4,681
    pub temperature_chunk_size: usize, // 10 params per record -> max 6,553
    pub respiratory_chunk_size: usize, // 7 params per record -> max 9,362
    pub workout_chunk_size: usize,    // 10 params per record -> max 6,553
    pub blood_glucose_chunk_size: usize, // 8 params per record -> max 8,192
    pub nutrition_chunk_size: usize,  // 32 params per record -> max 2,047

    // Reproductive Health Batch Processing (HIPAA-Compliant with Privacy Controls)
    pub menstrual_chunk_size: usize, // 8 params per record -> max 8,192
    pub fertility_chunk_size: usize, // 12 params per record -> max 5,461

    // Environmental and Audio Exposure Batch Processing
    pub environmental_chunk_size: usize, // 14 params per record -> max 3,730
    pub audio_exposure_chunk_size: usize, // 7 params per record -> max 7,000

    // Mental Health and Safety Batch Processing
    pub safety_event_chunk_size: usize, // 8 params per record -> max 6,553
    pub mindfulness_chunk_size: usize, // 9 params per record -> max 5,825
    pub mental_health_chunk_size: usize, // 10 params per record -> max 5,242
    pub symptom_chunk_size: usize, // 9 params per record -> max 5,825
    pub hygiene_chunk_size: usize, // 8 params per record -> max 6,553

    pub enable_progress_tracking: bool, // Track progress for large batches
    pub enable_intra_batch_deduplication: bool, // Enable deduplication within batches
    // Dual-write configuration for activity_metrics migration
    pub enable_dual_write_activity_metrics: bool, // Feature flag for dual-write pattern

    // Privacy and Security Configuration for Reproductive Health
    pub enable_reproductive_health_encryption: bool, // Enable encryption for sensitive reproductive data
    pub reproductive_health_audit_logging: bool, // Enable enhanced audit logging for reproductive health access
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            enable_parallel_processing: true, // Keep parallel processing enabled
            chunk_size: 1000,
            memory_limit_mb: 500.0,
            // Safe chunk sizes (80% of theoretical max for reliability) - FIXED to prevent PostgreSQL parameter violations
            heart_rate_chunk_size: 4200, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 4,200 (max ~42,000 params) - Updated for expanded cardiovascular metrics
            blood_pressure_chunk_size: 8000, // 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,000 (max ~48,000 params)
            sleep_chunk_size: 5200, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,200 (max ~52,000 params) - FIXED from 6000
            activity_chunk_size: 2700, // 19 params: 65,535 ÷ 19 × 0.8 ≈ 2,700 (max ~51,300 params) - ALREADY CORRECT
            body_measurement_chunk_size: 3000, // 16 params: 65,535 ÷ 16 × 0.8 ≈ 3,275, using 3,000 for safety (max ~48,000 params)
            temperature_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params) - FIXED from 8000
            respiratory_chunk_size: 7000, // 7 params: 65,535 ÷ 7 × 0.8 ≈ 7,000 (max ~49,000 params)
            workout_chunk_size: 5000, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,000 (max ~50,000 params)
            blood_glucose_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)
            nutrition_chunk_size: 1600, // 32 params: 65,535 ÷ 32 × 0.8 ≈ 1,600 (max ~51,200 params)

            // Reproductive Health Batch Processing (Privacy-Optimized Chunk Sizes)
            menstrual_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)
            fertility_chunk_size: 4300, // 12 params: 65,535 ÷ 12 × 0.8 ≈ 4,360 (max ~52,320 params)

            // Environmental and Audio Exposure Batch Processing
            environmental_chunk_size: 3700, // 14 params: 65,535 ÷ 14 × 0.8 ≈ 3,730 (max ~52,220 params)
            audio_exposure_chunk_size: 7000, // 7 params: 65,535 ÷ 7 × 0.8 ≈ 7,000 (max ~49,000 params)

            // Mental Health and Safety Batch Processing
            safety_event_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)
            mindfulness_chunk_size: 5800, // 9 params: 65,535 ÷ 9 × 0.8 ≈ 5,825 (max ~52,425 params)
            mental_health_chunk_size: 5200, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,200 (max ~52,000 params)
            symptom_chunk_size: 5800, // 9 params: 65,535 ÷ 9 × 0.8 ≈ 5,825 (max ~52,425 params)
            hygiene_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)

            enable_progress_tracking: true,
            enable_intra_batch_deduplication: true, // Enable by default for performance
            enable_dual_write_activity_metrics: false, // Disabled by default for safe rollout

            // Privacy and Security Defaults for Reproductive Health
            enable_reproductive_health_encryption: true, // Enable encryption by default for sensitive data
            reproductive_health_audit_logging: true,     // Enable enhanced audit logging by default
        }
    }
}

impl BatchConfig {
    /// Create BatchConfig from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        Self {
            max_retries: env::var("BATCH_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            initial_backoff_ms: env::var("BATCH_INITIAL_BACKOFF_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            max_backoff_ms: env::var("BATCH_MAX_BACKOFF_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            enable_parallel_processing: env::var("BATCH_ENABLE_PARALLEL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            chunk_size: env::var("BATCH_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            memory_limit_mb: env::var("BATCH_MEMORY_LIMIT_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500.0),
            // Metric-specific chunk sizes with environment variable overrides
            heart_rate_chunk_size: env::var("BATCH_HEART_RATE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4200), // Updated for expanded cardiovascular metrics (10 params)
            blood_pressure_chunk_size: env::var("BATCH_BLOOD_PRESSURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            sleep_chunk_size: env::var("BATCH_SLEEP_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5200),
            activity_chunk_size: env::var("BATCH_ACTIVITY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2700), // 19 params: 65,535 ÷ 19 × 0.8 ≈ 2,700 (max ~51,300 params)
            body_measurement_chunk_size: env::var("BATCH_BODY_MEASUREMENT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3500),
            temperature_chunk_size: env::var("BATCH_TEMPERATURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            respiratory_chunk_size: env::var("BATCH_RESPIRATORY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7000),
            workout_chunk_size: env::var("BATCH_WORKOUT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            blood_glucose_chunk_size: env::var("BATCH_BLOOD_GLUCOSE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            nutrition_chunk_size: env::var("BATCH_NUTRITION_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1600),

            // Reproductive Health Batch Processing Environment Configuration
            menstrual_chunk_size: env::var("BATCH_MENSTRUAL_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            fertility_chunk_size: env::var("BATCH_FERTILITY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4300),

            // Environmental and Audio Exposure Batch Processing Environment Configuration
            environmental_chunk_size: env::var("BATCH_ENVIRONMENTAL_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3700),
            audio_exposure_chunk_size: env::var("BATCH_AUDIO_EXPOSURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7000),

            // Mental Health and Safety Batch Processing Environment Configuration
            safety_event_chunk_size: env::var("BATCH_SAFETY_EVENT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            mindfulness_chunk_size: env::var("BATCH_MINDFULNESS_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5800),
            mental_health_chunk_size: env::var("BATCH_MENTAL_HEALTH_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5200),
            symptom_chunk_size: env::var("BATCH_SYMPTOM_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5800),
            hygiene_chunk_size: env::var("BATCH_HYGIENE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),

            enable_progress_tracking: env::var("BATCH_ENABLE_PROGRESS_TRACKING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_intra_batch_deduplication: env::var("BATCH_ENABLE_DEDUPLICATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_dual_write_activity_metrics: env::var("DUAL_WRITE_ACTIVITY_METRICS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),

            // Privacy and Security Environment Configuration for Reproductive Health
            enable_reproductive_health_encryption: env::var("BATCH_REPRODUCTIVE_HEALTH_ENCRYPTION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            reproductive_health_audit_logging: env::var("BATCH_REPRODUCTIVE_HEALTH_AUDIT_LOGGING")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
        }
    }

    /// Validate chunk sizes against PostgreSQL parameter limits
    ///
    /// This validation prevents silent data loss by ensuring all batch operations
    /// stay under PostgreSQL's 65,535 parameter limit per query.
    pub fn validate(&self) -> Result<(), String> {
        let validations = vec![
            (
                "heart_rate",
                self.heart_rate_chunk_size,
                HEART_RATE_PARAMS_PER_RECORD,
            ),
            (
                "blood_pressure",
                self.blood_pressure_chunk_size,
                BLOOD_PRESSURE_PARAMS_PER_RECORD,
            ),
            ("sleep", self.sleep_chunk_size, SLEEP_PARAMS_PER_RECORD),
            (
                "activity",
                self.activity_chunk_size,
                ACTIVITY_PARAMS_PER_RECORD,
            ),
            (
                "body_measurement",
                self.body_measurement_chunk_size,
                BODY_MEASUREMENT_PARAMS_PER_RECORD,
            ),
            (
                "temperature",
                self.temperature_chunk_size,
                TEMPERATURE_PARAMS_PER_RECORD,
            ),
            (
                "respiratory",
                self.respiratory_chunk_size,
                RESPIRATORY_PARAMS_PER_RECORD,
            ),
            (
                "workout",
                self.workout_chunk_size,
                WORKOUT_PARAMS_PER_RECORD,
            ),
            (
                "blood_glucose",
                self.blood_glucose_chunk_size,
                BLOOD_GLUCOSE_PARAMS_PER_RECORD,
            ),
            (
                "nutrition",
                self.nutrition_chunk_size,
                NUTRITION_PARAMS_PER_RECORD,
            ),
            // Reproductive Health Batch Processing Validation (HIPAA-Compliant)
            (
                "menstrual",
                self.menstrual_chunk_size,
                MENSTRUAL_PARAMS_PER_RECORD,
            ),
            (
                "fertility",
                self.fertility_chunk_size,
                FERTILITY_PARAMS_PER_RECORD,
            ),
            // Environmental and Audio Exposure Validation
            (
                "environmental",
                self.environmental_chunk_size,
                ENVIRONMENTAL_PARAMS_PER_RECORD,
            ),
            (
                "audio_exposure",
                self.audio_exposure_chunk_size,
                AUDIO_EXPOSURE_PARAMS_PER_RECORD,
            ),
            // Mental Health and Safety Validation
            (
                "safety_event",
                self.safety_event_chunk_size,
                SAFETY_EVENT_PARAMS_PER_RECORD,
            ),
            (
                "mindfulness",
                self.mindfulness_chunk_size,
                MINDFULNESS_PARAMS_PER_RECORD,
            ),
            (
                "mental_health",
                self.mental_health_chunk_size,
                MENTAL_HEALTH_PARAMS_PER_RECORD,
            ),
            (
                "symptom",
                self.symptom_chunk_size,
                SYMPTOM_PARAMS_PER_RECORD,
            ),
            (
                "hygiene",
                self.hygiene_chunk_size,
                HYGIENE_PARAMS_PER_RECORD,
            ),
        ];

        let mut errors = Vec::new();

        for (metric_type, chunk_size, params_per_record) in validations {
            let total_params = chunk_size * params_per_record;

            // Critical error: exceeds safe limit (would cause data loss)
            if total_params > SAFE_PARAM_LIMIT {
                let max_safe_chunk_size = SAFE_PARAM_LIMIT / params_per_record;
                errors.push(format!(
                    "CRITICAL: {metric_type} chunk size {chunk_size} * {params_per_record} params = {total_params} parameters, exceeding safe limit of {SAFE_PARAM_LIMIT}. Maximum safe chunk size: {max_safe_chunk_size}. THIS CAUSES SILENT DATA LOSS!"
                ));
            }
            // Warning: getting close to the limit
            else if total_params > (SAFE_PARAM_LIMIT * 90 / 100) {
                eprintln!(
                    "WARNING: {metric_type} chunk size {chunk_size} uses {total_params} parameters ({}% of safe limit)",
                    (total_params * 100) / SAFE_PARAM_LIMIT
                );
            }
        }

        if !errors.is_empty() {
            return Err(format!(
                "Batch configuration validation failed:\n{}",
                errors.join("\n")
            ));
        }

        Ok(())
    }
}
