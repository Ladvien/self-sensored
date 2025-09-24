use std::env;

/// PostgreSQL parameter limit constants for batch processing optimization
const _POSTGRESQL_MAX_PARAMS: usize = 65535; // PostgreSQL absolute maximum
pub const SAFE_PARAM_LIMIT: usize = 52428; // 80% of max for safety margin

/// Parameter counts per metric type for chunk size calculations
pub const HEART_RATE_PARAMS_PER_RECORD: usize = 11; // user_id, recorded_at, heart_rate, resting_heart_rate, heart_rate_variability, walking_heart_rate_average, heart_rate_recovery_one_minute, atrial_fibrillation_burden_percentage, vo2_max_ml_kg_min, context, source_device
pub const BLOOD_PRESSURE_PARAMS_PER_RECORD: usize = 6; // user_id, recorded_at, systolic, diastolic, pulse, source_device
pub const SLEEP_PARAMS_PER_RECORD: usize = 10; // user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, efficiency, source_device
pub const ACTIVITY_PARAMS_PER_RECORD: usize = 36; // user_id, recorded_at, step_count, distance_meters, active_energy_burned_kcal, basal_energy_burned_kcal, flights_climbed, distance_cycling_meters, distance_swimming_meters, distance_wheelchair_meters, distance_downhill_snow_sports_meters, push_count, swimming_stroke_count, nike_fuel_points, apple_exercise_time_minutes, apple_stand_time_minutes, apple_move_time_minutes, apple_stand_hour_achieved, walking_speed_m_per_s, walking_step_length_cm, walking_asymmetry_percent, walking_double_support_percent, six_minute_walk_test_distance_m, stair_ascent_speed_m_per_s, stair_descent_speed_m_per_s, ground_contact_time_ms, vertical_oscillation_cm, running_stride_length_m, running_power_watts, running_speed_m_per_s, cycling_speed_kmh, cycling_power_watts, cycling_cadence_rpm, functional_threshold_power_watts, underwater_depth_meters, diving_duration_seconds, source_device
pub const BODY_MEASUREMENT_PARAMS_PER_RECORD: usize = 16; // user_id, recorded_at, body_weight_kg, body_mass_index, body_fat_percentage, lean_body_mass_kg, height_cm, waist_circumference_cm, hip_circumference_cm, chest_circumference_cm, arm_circumference_cm, thigh_circumference_cm, body_temperature_celsius, basal_body_temperature_celsius, measurement_source, source_device
pub const TEMPERATURE_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, body_temperature, basal_body_temperature, apple_sleeping_wrist_temperature, water_temperature, temperature_source, source_device
pub const RESPIRATORY_PARAMS_PER_RECORD: usize = 7; // user_id, recorded_at, respiratory_rate, oxygen_saturation, forced_vital_capacity, forced_expiratory_volume_1, peak_expiratory_flow_rate, inhaler_usage, source_device
pub const WORKOUT_PARAMS_PER_RECORD: usize = 10; // id, user_id, workout_type, started_at, ended_at, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source_device
pub const BLOOD_GLUCOSE_PARAMS_PER_RECORD: usize = 8; // user_id, recorded_at, blood_glucose_mg_dl, measurement_context, medication_taken, insulin_delivery_units, glucose_source, source_device
pub const METABOLIC_PARAMS_PER_RECORD: usize = 6; // user_id, recorded_at, blood_alcohol_content, insulin_delivery_units, delivery_method, source_device
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
    pub max_concurrent_metric_types: usize, // Limit concurrent metric type processing to reduce connection pressure
    pub chunk_size: usize,
    pub memory_limit_mb: f64,
    // Chunking configurations to stay under PostgreSQL 65,535 parameter limit
    pub heart_rate_chunk_size: usize, // 11 params per record -> max 4,766
    pub blood_pressure_chunk_size: usize, // 6 params per record -> max 10,922
    pub sleep_chunk_size: usize,      // 10 params per record -> max 6,553
    pub activity_chunk_size: usize,   // 8 params per record -> max 8,178
    pub body_measurement_chunk_size: usize, // 14 params per record -> max 4,681
    pub temperature_chunk_size: usize, // 10 params per record -> max 6,553
    pub respiratory_chunk_size: usize, // 7 params per record -> max 9,362
    pub workout_chunk_size: usize,    // 10 params per record -> max 6,553
    pub blood_glucose_chunk_size: usize, // 8 params per record -> max 8,192
    pub metabolic_chunk_size: usize,  // 6 params per record -> max 8,738
    pub nutrition_chunk_size: usize,  // 32 params per record -> max 2,047

    // Reproductive Health Batch Processing (HIPAA-Compliant with Privacy Controls)
    pub menstrual_chunk_size: usize, // 8 params per record -> max 8,192
    pub fertility_chunk_size: usize, // 12 params per record -> max 5,461

    // Environmental and Audio Exposure Batch Processing
    pub environmental_chunk_size: usize, // 14 params per record -> max 3,730
    pub audio_exposure_chunk_size: usize, // 7 params per record -> max 7,000

    // Mental Health and Safety Batch Processing
    pub safety_event_chunk_size: usize, // 8 params per record -> max 6,553
    pub mindfulness_chunk_size: usize,  // 9 params per record -> max 5,825
    pub mental_health_chunk_size: usize, // 10 params per record -> max 5,242
    pub symptom_chunk_size: usize,      // 9 params per record -> max 5,825
    pub hygiene_chunk_size: usize,      // 8 params per record -> max 6,553

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
            max_concurrent_metric_types: 8, // Limit to 8 concurrent metric types to reduce connection pressure
            chunk_size: 1000,
            memory_limit_mb: 500.0,
            // Optimized chunk sizes for maximum safe throughput - STORY-OPTIMIZATION-001
            heart_rate_chunk_size: 4766, // 11 params: 52,426 params (max safe) - Reduced for complete data capture
            blood_pressure_chunk_size: 8738, // 6 params: 52,428 params (max safe) - +9% throughput improvement
            sleep_chunk_size: 5242, // 10 params: 52,420 params (max safe) - Safety fix from unsafe 6000
            activity_chunk_size: 1450, // 36 params: 52,200 params (safe with mobility + cycling + underwater metrics)
            body_measurement_chunk_size: 3276, // 16 params: 52,416 params (max safe) - +9% throughput improvement
            temperature_chunk_size: 6553, // 8 params: 52,424 params (max safe) - Safety fix from unsafe 8000
            respiratory_chunk_size: 7489, // 7 params: 52,423 params (max safe) - +7% throughput improvement
            workout_chunk_size: 5000, // 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,000 (max ~50,000 params)
            blood_glucose_chunk_size: 6500, // 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500 (max ~52,000 params)
            metabolic_chunk_size: 8700, // 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,700 (max ~52,200 params)
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
            max_concurrent_metric_types: env::var("BATCH_MAX_CONCURRENT_METRIC_TYPES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8),
            chunk_size: env::var("BATCH_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            memory_limit_mb: env::var("BATCH_MEMORY_LIMIT_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500.0),
            // Optimized metric-specific chunk sizes - STORY-OPTIMIZATION-001
            heart_rate_chunk_size: env::var("BATCH_HEART_RATE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4766), // 11 params: max safe 52,426 params (complete data capture)
            blood_pressure_chunk_size: env::var("BATCH_BLOOD_PRESSURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8738), // 6 params: max safe 52,428 params (+9% throughput)
            sleep_chunk_size: env::var("BATCH_SLEEP_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5242), // 10 params: max safe 52,420 params (safety fix)
            activity_chunk_size: env::var("BATCH_ACTIVITY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1450), // 36 params: 52,200 params (cycling + underwater support)
            body_measurement_chunk_size: env::var("BATCH_BODY_MEASUREMENT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3276), // 16 params: max safe 52,416 params (+9% throughput)
            temperature_chunk_size: env::var("BATCH_TEMPERATURE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6553), // 8 params: max safe 52,424 params (safety fix)
            respiratory_chunk_size: env::var("BATCH_RESPIRATORY_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7489), // 7 params: max safe 52,423 params (+7% throughput)
            workout_chunk_size: env::var("BATCH_WORKOUT_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            blood_glucose_chunk_size: env::var("BATCH_BLOOD_GLUCOSE_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6500),
            metabolic_chunk_size: env::var("BATCH_METABOLIC_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8700),
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

    /// Validate chunk sizes against PostgreSQL parameter limits with detailed diagnostics
    ///
    /// This validation prevents silent data loss by ensuring all batch operations
    /// stay under PostgreSQL's 65,535 parameter limit per query.
    /// Enhanced for STORY-OPTIMIZATION-001 with comprehensive error reporting.
    pub fn validate(&self) -> Result<(), String> {
        // Basic validation for required fields
        if self.max_retries == 0 {
            return Err("max_retries must be greater than 0".to_string());
        }

        if self.max_retries > 50 {
            return Err(
                "max_retries must not exceed 50 (excessive retries can cause performance issues)"
                    .to_string(),
            );
        }

        if self.initial_backoff_ms == 0 {
            return Err("initial_backoff_ms must be greater than 0".to_string());
        }

        if self.initial_backoff_ms > self.max_backoff_ms {
            return Err("initial_backoff_ms must not exceed max_backoff_ms".to_string());
        }

        if self.memory_limit_mb <= 0.0 {
            return Err("memory_limit_mb must be greater than 0".to_string());
        }

        if self.memory_limit_mb > 8192.0 {
            return Err("memory_limit_mb must not exceed 8192 MB (8 GB)".to_string());
        }

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
                "metabolic",
                self.metabolic_chunk_size,
                METABOLIC_PARAMS_PER_RECORD,
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
        let mut warnings = Vec::new();
        let mut optimizations = Vec::new();

        for (metric_type, chunk_size, params_per_record) in validations {
            // Check for invalid chunk sizes
            if chunk_size == 0 {
                errors.push(format!(
                    "🚨 CRITICAL: {metric_type} chunk size is 0\n\
                    ❌ Chunk size must be greater than 0 for batch processing to work"
                ));
                continue; // Skip further processing for this metric
            }

            let total_params = chunk_size * params_per_record;
            let max_safe_chunk_size = SAFE_PARAM_LIMIT / params_per_record;
            let usage_percentage = (total_params * 100) / SAFE_PARAM_LIMIT;

            // Critical error: exceeds safe limit (would cause data loss)
            if total_params > SAFE_PARAM_LIMIT {
                errors.push(format!(
                    "🚨 CRITICAL: {metric_type} chunk size {chunk_size} * {params_per_record} params = {total_params} parameters\n\
                    ❌ Exceeds safe limit of {SAFE_PARAM_LIMIT} ({}% over limit)\n\
                    ✅ Maximum safe chunk size: {max_safe_chunk_size}\n\
                    💥 THIS CAUSES SILENT DATA LOSS - PostgreSQL queries will fail!",
                    (total_params * 100 / SAFE_PARAM_LIMIT) - 100
                ));
            }
            // Warning: getting close to the limit (90%+)
            else if total_params > (SAFE_PARAM_LIMIT * 90 / 100) {
                warnings.push(format!(
                    "⚠️  WARNING: {metric_type} chunk size {chunk_size} uses {total_params} parameters ({usage_percentage}% of safe limit)\n\
                    📊 Approaching PostgreSQL parameter limit - consider monitoring query performance"
                ));
            }
            // Optimization opportunity: could increase chunk size significantly
            else if total_params < (SAFE_PARAM_LIMIT * 70 / 100) && chunk_size > 0 {
                let potential_increase = ((max_safe_chunk_size - chunk_size) * 100) / chunk_size;
                if potential_increase > 5 {
                    // Only suggest if >5% improvement
                    optimizations.push(format!(
                        "🚀 OPTIMIZATION: {metric_type} could increase from {chunk_size} to {max_safe_chunk_size} (+{potential_increase}% throughput)\n\
                        📈 Current: {total_params} params ({usage_percentage}% of limit) | Potential: {max_safe_chunk_size} chunks"
                    ));
                }
            }
        }

        // Print diagnostics
        if !optimizations.is_empty() {
            eprintln!("🔧 Batch Configuration Optimization Opportunities:");
            for opt in &optimizations {
                eprintln!("{opt}");
            }
            eprintln!();
        }

        if !warnings.is_empty() {
            eprintln!("⚠️  Batch Configuration Warnings:");
            for warning in &warnings {
                eprintln!("{warning}");
            }
            eprintln!();
        }

        if !errors.is_empty() {
            return Err(format!(
                "🚨 Batch configuration validation failed with {} critical errors:\n\n{}",
                errors.len(),
                errors.join("\n\n")
            ));
        }

        eprintln!("✅ Batch configuration validation passed - all chunk sizes are within safe PostgreSQL parameter limits");
        Ok(())
    }

    /// Performance benchmark for chunk size optimizations
    ///
    /// Creates a detailed analysis report showing throughput improvements
    /// and safety validations for STORY-OPTIMIZATION-001
    pub fn performance_benchmark(&self) -> String {
        let metrics = vec![
            (
                "Heart Rate",
                self.heart_rate_chunk_size,
                HEART_RATE_PARAMS_PER_RECORD,
                4200,
            ),
            (
                "Blood Pressure",
                self.blood_pressure_chunk_size,
                BLOOD_PRESSURE_PARAMS_PER_RECORD,
                8000,
            ),
            (
                "Sleep",
                self.sleep_chunk_size,
                SLEEP_PARAMS_PER_RECORD,
                6000,
            ), // Note: was unsafe
            (
                "Activity",
                self.activity_chunk_size,
                ACTIVITY_PARAMS_PER_RECORD,
                1700,
            ),
            (
                "Body Measurement",
                self.body_measurement_chunk_size,
                BODY_MEASUREMENT_PARAMS_PER_RECORD,
                3000,
            ),
            (
                "Temperature",
                self.temperature_chunk_size,
                TEMPERATURE_PARAMS_PER_RECORD,
                8000,
            ), // Note: was unsafe
            (
                "Respiratory",
                self.respiratory_chunk_size,
                RESPIRATORY_PARAMS_PER_RECORD,
                7000,
            ),
        ];

        let mut report = String::new();
        report.push_str(
            "🚀 STORY-OPTIMIZATION-001: Batch Processing Parameter Optimization Results\n",
        );
        report.push_str(
            "═══════════════════════════════════════════════════════════════════════════\n\n",
        );

        report.push_str("📊 OPTIMIZATION SUMMARY:\n");

        let mut total_throughput_improvement = 0.0;
        let mut safety_fixes = 0;
        let mut optimizations = 0;

        for (name, new_chunk, params_per_record, old_chunk) in &metrics {
            let old_params = old_chunk * params_per_record;
            let new_params = new_chunk * params_per_record;
            let max_safe_chunk = SAFE_PARAM_LIMIT / params_per_record;

            let throughput_change = if new_chunk > old_chunk {
                let improvement = ((new_chunk - old_chunk) as f64 / *old_chunk as f64) * 100.0;
                total_throughput_improvement += improvement;
                optimizations += 1;
                format!("+{:.1}% throughput", improvement)
            } else if new_chunk < old_chunk {
                safety_fixes += 1;
                "SAFETY FIX".to_string()
            } else {
                "No change".to_string()
            };

            let safety_status = if old_params > SAFE_PARAM_LIMIT {
                "🚨 WAS UNSAFE"
            } else {
                "✅ Was safe"
            };

            let current_status = if new_params <= SAFE_PARAM_LIMIT {
                "✅ SAFE"
            } else {
                "❌ UNSAFE"
            };

            report.push_str(&format!(
                "{:<18} | {:>5} → {:>5} chunks | {} → {} | {} | {}\n",
                name, old_chunk, new_chunk, old_params, new_params, safety_status, current_status
            ));

            if throughput_change != "No change" && throughput_change != "SAFETY FIX" {
                report.push_str(&format!(
                    "                   | Improvement: {} (max possible: {})\n",
                    throughput_change, max_safe_chunk
                ));
            }
        }

        report.push_str(&format!("\n🎯 IMPACT ANALYSIS:\n"));
        report.push_str(&format!(
            "  • Safety Fixes Applied: {} critical PostgreSQL parameter violations resolved\n",
            safety_fixes
        ));
        report.push_str(&format!(
            "  • Performance Optimizations: {} metric types improved\n",
            optimizations
        ));
        report.push_str(&format!(
            "  • Average Throughput Gain: {:.1}% across optimized metrics\n",
            total_throughput_improvement / optimizations as f64
        ));

        report.push_str(&format!("\n📈 POSTGRESQL PARAMETER USAGE:\n"));
        for (name, chunk_size, params_per_record, _) in &metrics {
            let total_params = chunk_size * params_per_record;
            let usage_pct = (total_params * 100) / SAFE_PARAM_LIMIT;
            let safety_margin = SAFE_PARAM_LIMIT - total_params;

            report.push_str(&format!(
                "  {:<18} | {:>5} params ({:>2}% of limit) | Safety margin: {:>5} params\n",
                name, total_params, usage_pct, safety_margin
            ));
        }

        report.push_str(&format!("\n✅ VALIDATION RESULTS:\n"));
        match self.validate() {
            Ok(_) => {
                report.push_str("  All chunk sizes are within safe PostgreSQL parameter limits\n")
            }
            Err(e) => report.push_str(&format!("  ❌ Validation failed: {}\n", e)),
        }

        report.push_str(&format!("\n🔬 DETAILED ANALYSIS:\n"));
        report.push_str(&format!("  • PostgreSQL Max Parameters: 65,535\n"));
        report.push_str(&format!(
            "  • Safe Parameter Limit (80%): {:>5}\n",
            SAFE_PARAM_LIMIT
        ));
        report.push_str(&format!(
            "  • Safety Margin Strategy: 20% buffer for query complexity\n"
        ));
        report.push_str(&format!(
            "  • Optimization Target: Maximum throughput within safety limits\n"
        ));

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_chunk_sizes_are_safe() {
        let config = BatchConfig::default();
        config
            .validate()
            .expect("Optimized chunk sizes should be safe");
    }

    #[test]
    fn test_chunk_size_calculations() {
        let config = BatchConfig::default();

        // Test that all chunk sizes stay within safe limits
        let test_cases = vec![
            (
                "heart_rate",
                config.heart_rate_chunk_size,
                HEART_RATE_PARAMS_PER_RECORD,
            ),
            (
                "blood_pressure",
                config.blood_pressure_chunk_size,
                BLOOD_PRESSURE_PARAMS_PER_RECORD,
            ),
            ("sleep", config.sleep_chunk_size, SLEEP_PARAMS_PER_RECORD),
            (
                "activity",
                config.activity_chunk_size,
                ACTIVITY_PARAMS_PER_RECORD,
            ),
            (
                "body_measurement",
                config.body_measurement_chunk_size,
                BODY_MEASUREMENT_PARAMS_PER_RECORD,
            ),
            (
                "temperature",
                config.temperature_chunk_size,
                TEMPERATURE_PARAMS_PER_RECORD,
            ),
            (
                "respiratory",
                config.respiratory_chunk_size,
                RESPIRATORY_PARAMS_PER_RECORD,
            ),
        ];

        for (metric_name, chunk_size, params_per_record) in test_cases {
            let total_params = chunk_size * params_per_record;
            assert!(
                total_params <= SAFE_PARAM_LIMIT,
                "{} chunk size {} * {} params = {} exceeds safe limit {}",
                metric_name,
                chunk_size,
                params_per_record,
                total_params,
                SAFE_PARAM_LIMIT
            );
        }
    }

    #[test]
    fn test_performance_benchmark() {
        let config = BatchConfig::default();
        let report = config.performance_benchmark();

        // Should contain key optimization information
        assert!(report.contains("STORY-OPTIMIZATION-001"));
        assert!(report.contains("Safety Fixes Applied"));
        assert!(report.contains("Performance Optimizations"));
        assert!(report.contains("POSTGRESQL PARAMETER USAGE"));
    }

    #[test]
    fn test_environment_variable_overrides() {
        // Test that environment variables can override defaults
        std::env::set_var("BATCH_HEART_RATE_CHUNK_SIZE", "1000");
        let config = BatchConfig::from_env();
        assert_eq!(config.heart_rate_chunk_size, 1000);
        std::env::remove_var("BATCH_HEART_RATE_CHUNK_SIZE");
    }
}
