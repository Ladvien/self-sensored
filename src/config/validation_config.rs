use std::env;

/// Configuration for health metric validation thresholds
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    // Heart Rate Validation Thresholds
    pub heart_rate_min: i16,
    pub heart_rate_max: i16,

    // Blood Pressure Validation Thresholds
    pub systolic_min: i16,
    pub systolic_max: i16,
    pub diastolic_min: i16,
    pub diastolic_max: i16,

    // Sleep Validation Thresholds
    pub sleep_efficiency_min: f32,
    pub sleep_efficiency_max: f32,
    pub sleep_duration_tolerance_minutes: i32,

    // Activity Validation Thresholds
    pub step_count_min: i32,
    pub step_count_max: i32,
    pub distance_max_km: f64,
    pub calories_max: f64,

    // GPS Coordinate Validation
    pub latitude_min: f64,
    pub latitude_max: f64,
    pub longitude_min: f64,
    pub longitude_max: f64,

    // Workout Validation
    pub workout_heart_rate_min: i16,
    pub workout_heart_rate_max: i16,
    pub workout_max_duration_hours: i64,

    // Blood Glucose Validation (Medical-Critical)
    pub blood_glucose_min: f32,
    pub blood_glucose_max: f32,
    pub insulin_max_units: f64,

    // Respiratory Validation Thresholds (Medical-grade)
    pub respiratory_rate_min: i32,
    pub respiratory_rate_max: i32,
    pub oxygen_saturation_min: f64,
    pub oxygen_saturation_max: f64,
    pub oxygen_saturation_critical: f64, // Critical alert threshold
    pub forced_vital_capacity_min: f64,  // FVC minimum
    pub forced_vital_capacity_max: f64,  // FVC maximum
    pub forced_expiratory_volume_1_min: f64, // FEV1 minimum
    pub forced_expiratory_volume_1_max: f64, // FEV1 maximum
    pub peak_expiratory_flow_rate_min: f64, // PEFR minimum
    pub peak_expiratory_flow_rate_max: f64, // PEFR maximum
    pub inhaler_usage_max: i32,          // Maximum daily inhaler usage

    // Temperature Validation Thresholds (Medical-grade)
    pub body_temperature_min: f32, // Body temperature minimum (°C)
    pub body_temperature_max: f32, // Body temperature maximum (°C)
    pub basal_body_temperature_min: f32, // Basal temp minimum for fertility tracking
    pub basal_body_temperature_max: f32, // Basal temp maximum for fertility tracking
    pub wrist_temperature_min: f32, // Apple Watch wrist temp minimum
    pub wrist_temperature_max: f32, // Apple Watch wrist temp maximum
    pub water_temperature_min: f32, // Environmental water temp minimum
    pub water_temperature_max: f32, // Environmental water temp maximum
    pub fever_threshold: f32,      // Fever threshold (38.0°C / 100.4°F)

    // Body Measurements Validation Thresholds
    pub body_weight_min_kg: f64,           // Minimum body weight (20 kg)
    pub body_weight_max_kg: f64,           // Maximum body weight (500 kg)
    pub bmi_min: f64,                      // Minimum BMI (15.0)
    pub bmi_max: f64,                      // Maximum BMI (50.0)
    pub body_fat_min_percent: f64,         // Minimum body fat percentage (3.0%)
    pub body_fat_max_percent: f64,         // Maximum body fat percentage (50.0%)
    pub body_temperature_min_celsius: f64, // Body temperature minimum for body metrics
    pub body_temperature_max_celsius: f64, // Body temperature maximum for body metrics

    // Reproductive Health Validation Thresholds (HIPAA-Compliant Medical-Grade)
    pub menstrual_cycle_day_min: i16,       // Minimum cycle day (1)
    pub menstrual_cycle_day_max: i16,       // Maximum cycle day (45 for irregular cycles)
    pub menstrual_cramps_severity_min: i16, // Minimum cramps severity (0 - no pain)
    pub menstrual_cramps_severity_max: i16, // Maximum cramps severity (10 - severe pain)
    pub menstrual_mood_rating_min: i16,     // Minimum mood rating (1 - terrible)
    pub menstrual_mood_rating_max: i16,     // Maximum mood rating (5 - great)
    pub menstrual_energy_level_min: i16,    // Minimum energy level (1 - exhausted)
    pub menstrual_energy_level_max: i16,    // Maximum energy level (5 - energetic)

    // Fertility Tracking Validation Thresholds (Medical-Grade)
    pub fertility_basal_temp_min: f32, // Minimum basal body temperature (35.0°C)
    pub fertility_basal_temp_max: f32, // Maximum basal body temperature (39.0°C)
    pub fertility_cervix_firmness_min: i16, // Minimum cervix firmness (1 - soft)
    pub fertility_cervix_firmness_max: i16, // Maximum cervix firmness (3 - firm)
    pub fertility_cervix_position_min: i16, // Minimum cervix position (1 - low)
    pub fertility_cervix_position_max: i16, // Maximum cervix position (3 - high)
    pub fertility_lh_level_min: f64,   // Minimum LH level (0.0 mIU/mL)
    pub fertility_lh_level_max: f64,   // Maximum LH level (100.0 mIU/mL)
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            // Heart rate: physiologically reasonable range
            heart_rate_min: 15,
            heart_rate_max: 300,

            // Blood pressure: medical ranges for extreme values
            systolic_min: 50,
            systolic_max: 250,
            diastolic_min: 30,
            diastolic_max: 150,

            // Sleep efficiency: percentage range
            sleep_efficiency_min: 0.0,
            sleep_efficiency_max: 100.0,
            sleep_duration_tolerance_minutes: 60, // Allow 1 hour variance

            // Activity limits: reasonable daily maximums
            step_count_min: 0,
            step_count_max: 200_000, // Extreme but possible
            distance_max_km: 500.0,  // ~310 miles - marathon+ distance
            calories_max: 20_000.0,  // Extreme athletic events

            // GPS coordinates: global valid ranges
            latitude_min: -90.0,
            latitude_max: 90.0,
            longitude_min: -180.0,
            longitude_max: 180.0,

            // Workout validation
            workout_heart_rate_min: 15,
            workout_heart_rate_max: 300,
            workout_max_duration_hours: 24, // 24-hour ultra events exist

            // Blood glucose validation (medical-critical ranges)
            blood_glucose_min: 30.0,  // Medical emergency below this (mg/dL)
            blood_glucose_max: 600.0, // Medical emergency above this (mg/dL)
            insulin_max_units: 100.0, // Maximum reasonable insulin dose

            // Respiratory validation (Medical-grade thresholds)
            respiratory_rate_min: 5,              // Extreme bradypnea
            respiratory_rate_max: 60,             // Extreme tachypnea
            oxygen_saturation_min: 70.0,          // Severe hypoxemia
            oxygen_saturation_max: 100.0,         // Perfect saturation
            oxygen_saturation_critical: 90.0,     // Critical alert threshold
            forced_vital_capacity_min: 1.0,       // Minimum FVC (severe restriction)
            forced_vital_capacity_max: 8.0,       // Maximum FVC (very tall individuals)
            forced_expiratory_volume_1_min: 0.5,  // Minimum FEV1
            forced_expiratory_volume_1_max: 6.0,  // Maximum FEV1
            peak_expiratory_flow_rate_min: 50.0,  // Severe obstruction
            peak_expiratory_flow_rate_max: 800.0, // Elite athlete maximum
            inhaler_usage_max: 50,                // Maximum daily inhaler usage

            // Temperature validation (Medical-grade thresholds)
            body_temperature_min: 30.0, // Severe hypothermia threshold
            body_temperature_max: 45.0, // Extreme hyperthermia threshold
            basal_body_temperature_min: 35.0, // Low basal temp for fertility tracking
            basal_body_temperature_max: 39.0, // High basal temp for fertility tracking
            wrist_temperature_min: 30.0, // Apple Watch wrist temp range
            wrist_temperature_max: 45.0, // Apple Watch wrist temp range
            water_temperature_min: 0.0, // Environmental water temp (ice)
            water_temperature_max: 100.0, // Environmental water temp (boiling)
            fever_threshold: 38.0,      // Fever threshold (100.4°F)

            // Body measurements validation defaults
            body_weight_min_kg: 20.0,   // Minimum reasonable body weight
            body_weight_max_kg: 500.0,  // Maximum reasonable body weight
            bmi_min: 15.0,              // Minimum BMI (severe underweight)
            bmi_max: 50.0,              // Maximum BMI (severe obesity)
            body_fat_min_percent: 3.0,  // Minimum body fat (essential fat)
            body_fat_max_percent: 50.0, // Maximum body fat (severe obesity)
            body_temperature_min_celsius: 30.0, // Body temperature minimum
            body_temperature_max_celsius: 45.0, // Body temperature maximum

            // Reproductive Health Validation Defaults (Medical-Grade HIPAA-Compliant)
            menstrual_cycle_day_min: 1,        // First day of cycle
            menstrual_cycle_day_max: 45,       // Maximum for very irregular cycles
            menstrual_cramps_severity_min: 0,  // No pain
            menstrual_cramps_severity_max: 10, // Severe pain (0-10 medical scale)
            menstrual_mood_rating_min: 1,      // Terrible mood
            menstrual_mood_rating_max: 5,      // Great mood
            menstrual_energy_level_min: 1,     // Exhausted
            menstrual_energy_level_max: 5,     // Energetic

            // Fertility Tracking Validation Defaults (Medical-Grade)
            fertility_basal_temp_min: 35.0, // Low basal body temperature (°C)
            fertility_basal_temp_max: 39.0, // High basal body temperature (°C)
            fertility_cervix_firmness_min: 1, // Soft cervix
            fertility_cervix_firmness_max: 3, // Firm cervix
            fertility_cervix_position_min: 1, // Low cervix position
            fertility_cervix_position_max: 3, // High cervix position
            fertility_lh_level_min: 0.0,    // Minimum LH level (mIU/mL)
            fertility_lh_level_max: 100.0,  // Maximum LH level (mIU/mL)
        }
    }
}

impl ValidationConfig {
    /// Create ValidationConfig from environment variables with fallback to defaults
    pub fn from_env() -> Self {
        Self {
            // Heart Rate Thresholds
            heart_rate_min: env::var("VALIDATION_HEART_RATE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            heart_rate_max: env::var("VALIDATION_HEART_RATE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),

            // Blood Pressure Thresholds
            systolic_min: env::var("VALIDATION_SYSTOLIC_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            systolic_max: env::var("VALIDATION_SYSTOLIC_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(250),
            diastolic_min: env::var("VALIDATION_DIASTOLIC_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            diastolic_max: env::var("VALIDATION_DIASTOLIC_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(150),

            // Sleep Validation Thresholds
            sleep_efficiency_min: env::var("VALIDATION_SLEEP_EFFICIENCY_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            sleep_efficiency_max: env::var("VALIDATION_SLEEP_EFFICIENCY_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
            sleep_duration_tolerance_minutes: env::var(
                "VALIDATION_SLEEP_DURATION_TOLERANCE_MINUTES",
            )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60),

            // Activity Validation Thresholds
            step_count_min: env::var("VALIDATION_STEP_COUNT_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            step_count_max: env::var("VALIDATION_STEP_COUNT_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200_000),
            distance_max_km: env::var("VALIDATION_DISTANCE_MAX_KM")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500.0),
            calories_max: env::var("VALIDATION_CALORIES_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20_000.0),

            // GPS Coordinate Validation
            latitude_min: env::var("VALIDATION_LATITUDE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(-90.0),
            latitude_max: env::var("VALIDATION_LATITUDE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(90.0),
            longitude_min: env::var("VALIDATION_LONGITUDE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(-180.0),
            longitude_max: env::var("VALIDATION_LONGITUDE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(180.0),

            // Workout Validation
            workout_heart_rate_min: env::var("VALIDATION_WORKOUT_HEART_RATE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            workout_heart_rate_max: env::var("VALIDATION_WORKOUT_HEART_RATE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            workout_max_duration_hours: env::var("VALIDATION_WORKOUT_MAX_DURATION_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(24),

            // Blood Glucose Validation Thresholds
            blood_glucose_min: env::var("VALIDATION_BLOOD_GLUCOSE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            blood_glucose_max: env::var("VALIDATION_BLOOD_GLUCOSE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600.0),
            insulin_max_units: env::var("VALIDATION_INSULIN_MAX_UNITS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),

            // Respiratory Validation from Environment
            respiratory_rate_min: env::var("VALIDATION_RESPIRATORY_RATE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            respiratory_rate_max: env::var("VALIDATION_RESPIRATORY_RATE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            oxygen_saturation_min: env::var("VALIDATION_OXYGEN_SATURATION_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(70.0),
            oxygen_saturation_max: env::var("VALIDATION_OXYGEN_SATURATION_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
            oxygen_saturation_critical: env::var("VALIDATION_OXYGEN_SATURATION_CRITICAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(90.0),
            forced_vital_capacity_min: env::var("VALIDATION_FORCED_VITAL_CAPACITY_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            forced_vital_capacity_max: env::var("VALIDATION_FORCED_VITAL_CAPACITY_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8.0),
            forced_expiratory_volume_1_min: env::var("VALIDATION_FORCED_EXPIRATORY_VOLUME_1_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.5),
            forced_expiratory_volume_1_max: env::var("VALIDATION_FORCED_EXPIRATORY_VOLUME_1_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6.0),
            peak_expiratory_flow_rate_min: env::var("VALIDATION_PEAK_EXPIRATORY_FLOW_RATE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50.0),
            peak_expiratory_flow_rate_max: env::var("VALIDATION_PEAK_EXPIRATORY_FLOW_RATE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(800.0),
            inhaler_usage_max: env::var("VALIDATION_INHALER_USAGE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),

            // Temperature Validation from Environment
            body_temperature_min: env::var("VALIDATION_BODY_TEMPERATURE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            body_temperature_max: env::var("VALIDATION_BODY_TEMPERATURE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(45.0),
            basal_body_temperature_min: env::var("VALIDATION_BASAL_BODY_TEMPERATURE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(35.0),
            basal_body_temperature_max: env::var("VALIDATION_BASAL_BODY_TEMPERATURE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(39.0),
            wrist_temperature_min: env::var("VALIDATION_WRIST_TEMPERATURE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            wrist_temperature_max: env::var("VALIDATION_WRIST_TEMPERATURE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(45.0),
            water_temperature_min: env::var("VALIDATION_WATER_TEMPERATURE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            water_temperature_max: env::var("VALIDATION_WATER_TEMPERATURE_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
            fever_threshold: env::var("VALIDATION_FEVER_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(38.0),

            // Body Measurements Validation from Environment
            body_weight_min_kg: env::var("VALIDATION_BODY_WEIGHT_MIN_KG")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20.0),
            body_weight_max_kg: env::var("VALIDATION_BODY_WEIGHT_MAX_KG")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500.0),
            bmi_min: env::var("VALIDATION_BMI_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15.0),
            bmi_max: env::var("VALIDATION_BMI_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50.0),
            body_fat_min_percent: env::var("VALIDATION_BODY_FAT_MIN_PERCENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3.0),
            body_fat_max_percent: env::var("VALIDATION_BODY_FAT_MAX_PERCENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50.0),
            body_temperature_min_celsius: env::var("VALIDATION_BODY_TEMPERATURE_MIN_CELSIUS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            body_temperature_max_celsius: env::var("VALIDATION_BODY_TEMPERATURE_MAX_CELSIUS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(45.0),

            // Reproductive Health Validation from Environment (HIPAA-Compliant)
            menstrual_cycle_day_min: env::var("VALIDATION_MENSTRUAL_CYCLE_DAY_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            menstrual_cycle_day_max: env::var("VALIDATION_MENSTRUAL_CYCLE_DAY_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(45),
            menstrual_cramps_severity_min: env::var("VALIDATION_MENSTRUAL_CRAMPS_SEVERITY_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            menstrual_cramps_severity_max: env::var("VALIDATION_MENSTRUAL_CRAMPS_SEVERITY_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            menstrual_mood_rating_min: env::var("VALIDATION_MENSTRUAL_MOOD_RATING_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            menstrual_mood_rating_max: env::var("VALIDATION_MENSTRUAL_MOOD_RATING_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            menstrual_energy_level_min: env::var("VALIDATION_MENSTRUAL_ENERGY_LEVEL_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            menstrual_energy_level_max: env::var("VALIDATION_MENSTRUAL_ENERGY_LEVEL_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),

            // Fertility Tracking Validation from Environment (Medical-Grade)
            fertility_basal_temp_min: env::var("VALIDATION_FERTILITY_BASAL_TEMP_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(35.0),
            fertility_basal_temp_max: env::var("VALIDATION_FERTILITY_BASAL_TEMP_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(39.0),
            fertility_cervix_firmness_min: env::var("VALIDATION_FERTILITY_CERVIX_FIRMNESS_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            fertility_cervix_firmness_max: env::var("VALIDATION_FERTILITY_CERVIX_FIRMNESS_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            fertility_cervix_position_min: env::var("VALIDATION_FERTILITY_CERVIX_POSITION_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            fertility_cervix_position_max: env::var("VALIDATION_FERTILITY_CERVIX_POSITION_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            fertility_lh_level_min: env::var("VALIDATION_FERTILITY_LH_LEVEL_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            fertility_lh_level_max: env::var("VALIDATION_FERTILITY_LH_LEVEL_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
        }
    }

    /// Validate that configuration values make sense
    pub fn validate(&self) -> Result<(), String> {
        if self.heart_rate_min >= self.heart_rate_max {
            return Err("heart_rate_min must be less than heart_rate_max".to_string());
        }

        if self.systolic_min >= self.systolic_max {
            return Err("systolic_min must be less than systolic_max".to_string());
        }

        if self.diastolic_min >= self.diastolic_max {
            return Err("diastolic_min must be less than diastolic_max".to_string());
        }

        if self.sleep_efficiency_min >= self.sleep_efficiency_max {
            return Err("sleep_efficiency_min must be less than sleep_efficiency_max".to_string());
        }

        if self.step_count_min >= self.step_count_max {
            return Err("step_count_min must be less than step_count_max".to_string());
        }

        if self.latitude_min >= self.latitude_max {
            return Err("latitude_min must be less than latitude_max".to_string());
        }

        if self.longitude_min >= self.longitude_max {
            return Err("longitude_min must be less than longitude_max".to_string());
        }

        if self.workout_heart_rate_min >= self.workout_heart_rate_max {
            return Err(
                "workout_heart_rate_min must be less than workout_heart_rate_max".to_string(),
            );
        }

        // Temperature validation
        if self.body_temperature_min >= self.body_temperature_max {
            return Err("body_temperature_min must be less than body_temperature_max".to_string());
        }

        if self.basal_body_temperature_min >= self.basal_body_temperature_max {
            return Err(
                "basal_body_temperature_min must be less than basal_body_temperature_max"
                    .to_string(),
            );
        }

        if self.wrist_temperature_min >= self.wrist_temperature_max {
            return Err(
                "wrist_temperature_min must be less than wrist_temperature_max".to_string(),
            );
        }

        if self.water_temperature_min >= self.water_temperature_max {
            return Err(
                "water_temperature_min must be less than water_temperature_max".to_string(),
            );
        }

        // Body measurements validation
        if self.body_weight_min_kg >= self.body_weight_max_kg {
            return Err("body_weight_min_kg must be less than body_weight_max_kg".to_string());
        }

        if self.bmi_min >= self.bmi_max {
            return Err("bmi_min must be less than bmi_max".to_string());
        }

        if self.body_fat_min_percent >= self.body_fat_max_percent {
            return Err("body_fat_min_percent must be less than body_fat_max_percent".to_string());
        }

        if self.body_temperature_min_celsius >= self.body_temperature_max_celsius {
            return Err(
                "body_temperature_min_celsius must be less than body_temperature_max_celsius"
                    .to_string(),
            );
        }

        // Blood glucose validation
        if self.blood_glucose_min >= self.blood_glucose_max {
            return Err("blood_glucose_min must be less than blood_glucose_max".to_string());
        }

        if self.insulin_max_units <= 0.0 {
            return Err("insulin_max_units must be positive".to_string());
        }

        // Reproductive Health Validation Consistency (HIPAA-Compliant Medical-Grade)
        if self.menstrual_cycle_day_min >= self.menstrual_cycle_day_max {
            return Err(
                "menstrual_cycle_day_min must be less than menstrual_cycle_day_max".to_string(),
            );
        }

        if self.menstrual_cramps_severity_min > self.menstrual_cramps_severity_max {
            return Err("menstrual_cramps_severity_min must be less than or equal to menstrual_cramps_severity_max".to_string());
        }

        if self.menstrual_mood_rating_min > self.menstrual_mood_rating_max {
            return Err(
                "menstrual_mood_rating_min must be less than or equal to menstrual_mood_rating_max"
                    .to_string(),
            );
        }

        if self.menstrual_energy_level_min > self.menstrual_energy_level_max {
            return Err("menstrual_energy_level_min must be less than or equal to menstrual_energy_level_max".to_string());
        }

        // Fertility Tracking Validation Consistency (Medical-Grade)
        if self.fertility_basal_temp_min >= self.fertility_basal_temp_max {
            return Err(
                "fertility_basal_temp_min must be less than fertility_basal_temp_max".to_string(),
            );
        }

        if self.fertility_cervix_firmness_min > self.fertility_cervix_firmness_max {
            return Err("fertility_cervix_firmness_min must be less than or equal to fertility_cervix_firmness_max".to_string());
        }

        if self.fertility_cervix_position_min > self.fertility_cervix_position_max {
            return Err("fertility_cervix_position_min must be less than or equal to fertility_cervix_position_max".to_string());
        }

        if self.fertility_lh_level_min >= self.fertility_lh_level_max {
            return Err(
                "fertility_lh_level_min must be less than fertility_lh_level_max".to_string(),
            );
        }

        Ok(())
    }
}
