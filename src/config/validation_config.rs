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
    pub steps_min: i32,
    pub steps_max: i32,
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
            steps_min: 0,
            steps_max: 200_000,     // Extreme but possible
            distance_max_km: 500.0, // ~310 miles - marathon+ distance
            calories_max: 20_000.0, // Extreme athletic events

            // GPS coordinates: global valid ranges
            latitude_min: -90.0,
            latitude_max: 90.0,
            longitude_min: -180.0,
            longitude_max: 180.0,

            // Workout validation
            workout_heart_rate_min: 15,
            workout_heart_rate_max: 300,
            workout_max_duration_hours: 24, // 24-hour ultra events exist
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
            steps_min: env::var("VALIDATION_STEPS_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            steps_max: env::var("VALIDATION_STEPS_MAX")
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

        if self.steps_min >= self.steps_max {
            return Err("steps_min must be less than steps_max".to_string());
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

        Ok(())
    }
}
