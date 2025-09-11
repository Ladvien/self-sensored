use crate::config::ValidationConfig;
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

/// Activity metrics v2 with Apple Health schema fields
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActivityMetricV2 {
    pub recorded_at: DateTime<Utc>,
    
    // Basic Activity Fields (from Apple Health)
    pub step_count: Option<i32>,
    pub flights_climbed: Option<i32>,
    
    // Distance Fields by Activity Type
    pub distance_walking_running_meters: Option<f64>,
    pub distance_cycling_meters: Option<f64>,
    pub distance_swimming_meters: Option<f64>,
    pub distance_wheelchair_meters: Option<f64>,
    pub distance_downhill_snow_sports_meters: Option<f64>,
    
    // Additional Activity Counts
    pub push_count: Option<i32>,  // wheelchair pushes
    pub swimming_stroke_count: Option<i32>,
    pub nike_fuel: Option<f64>,  // Nike Fuel points
    
    // Energy Fields (Apple Health standard names)
    pub active_energy_burned_kcal: Option<f64>,
    pub basal_energy_burned_kcal: Option<f64>,
    
    // Apple Fitness Ring Metrics
    pub exercise_time_minutes: Option<i32>,
    pub stand_time_minutes: Option<i32>,
    pub move_time_minutes: Option<i32>,
    pub stand_hour_achieved: Option<bool>,
    
    // Data tracking and aggregation
    pub aggregation_period: Option<String>, // 'minute', 'hourly', 'daily', 'weekly'
    pub source: Option<String>,
}

/// GPS coordinate for workout routes
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GpsCoordinate {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: Option<f64>,
    pub recorded_at: DateTime<Utc>,
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
    pub route_points: Option<Vec<GpsCoordinate>>, // GPS route data
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
        if self.end_time <= self.start_time {
            return Err("end_time must be after start_time".to_string());
        }

        if self.workout_type.is_empty() {
            return Err("workout_type cannot be empty".to_string());
        }

        // Check workout duration against configured maximum
        let duration_hours = (self.end_time - self.start_time).num_hours();
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
            if !(config.workout_heart_rate_min..=config.workout_heart_rate_max).contains(&hr) {
                return Err(format!(
                    "avg_heart_rate {} is out of range ({}-{})",
                    hr, config.workout_heart_rate_min, config.workout_heart_rate_max
                ));
            }
        }

        if let Some(hr) = self.max_heart_rate {
            if !(config.workout_heart_rate_min..=config.workout_heart_rate_max).contains(&hr) {
                return Err(format!(
                    "max_heart_rate {} is out of range ({}-{})",
                    hr, config.workout_heart_rate_min, config.workout_heart_rate_max
                ));
            }
        }

        // Validate GPS route if provided
        if let Some(route_points) = &self.route_points {
            for (i, point) in route_points.iter().enumerate() {
                if let Err(e) = point.validate_with_config(config) {
                    return Err(format!("route point {}: {}", i, e));
                }
            }

            // Ensure GPS points are within workout time bounds
            for (i, point) in route_points.iter().enumerate() {
                if point.recorded_at < self.start_time || point.recorded_at > self.end_time {
                    return Err(format!(
                        "route point {} timestamp is outside workout duration",
                        i
                    ));
                }
            }
        }

        Ok(())
    }

    /// Convert route points to PostGIS LINESTRING
    pub fn route_to_linestring(&self) -> Option<String> {
        if let Some(points) = &self.route_points {
            if points.len() < 2 {
                return None; // Need at least 2 points for a line
            }

            let coords: Vec<String> = points
                .iter()
                .map(|p| format!("{} {}", p.longitude, p.latitude))
                .collect();

            Some(format!("LINESTRING({})", coords.join(", ")))
        } else {
            None
        }
    }

    /// Calculate duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        (self.end_time - self.start_time).num_seconds()
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
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if let Some(bpm) = self.min_bpm {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&bpm) {
                return Err(format!(
                    "min_bpm {} is out of range ({}-{})",
                    bpm, config.heart_rate_min, config.heart_rate_max
                ));
            }
        }
        if let Some(bpm) = self.avg_bpm {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&bpm) {
                return Err(format!(
                    "avg_bpm {} is out of range ({}-{})",
                    bpm, config.heart_rate_min, config.heart_rate_max
                ));
            }
        }
        if let Some(bpm) = self.max_bpm {
            if !(config.heart_rate_min..=config.heart_rate_max).contains(&bpm) {
                return Err(format!(
                    "max_bpm {} is out of range ({}-{})",
                    bpm, config.heart_rate_min, config.heart_rate_max
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
        if (self.total_sleep_minutes - calculated_duration).abs()
            > config.sleep_duration_tolerance_minutes
        {
            return Err(format!(
                "total_sleep_minutes doesn't match sleep duration (tolerance: {} minutes)",
                config.sleep_duration_tolerance_minutes
            ));
        }

        if let Some(efficiency) = self.efficiency_percentage {
            if !(config.sleep_efficiency_min..=config.sleep_efficiency_max).contains(&efficiency) {
                return Err(format!(
                    "efficiency_percentage {} is out of range ({}-{})",
                    efficiency, config.sleep_efficiency_min, config.sleep_efficiency_max
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
        let actual_sleep = self.total_sleep_minutes as f32;
        (actual_sleep / total_duration * 100.0).min(100.0).max(0.0)
    }

    /// Get the efficiency percentage, calculating if not provided
    pub fn get_efficiency_percentage(&self) -> f32 {
        self.efficiency_percentage
            .unwrap_or_else(|| self.calculate_efficiency())
    }
}

impl ActivityMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        if let Some(steps) = self.steps {
            if steps < config.steps_min || steps > config.steps_max {
                return Err(format!(
                    "steps {} is out of range ({}-{})",
                    steps, config.steps_min, config.steps_max
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
        if let Some(calories) = self.calories_burned {
            if calories < 0.0 {
                return Err("calories_burned cannot be negative".to_string());
            }
            if calories > config.calories_max {
                return Err(format!(
                    "calories_burned {} exceeds maximum of {}",
                    calories, config.calories_max
                ));
            }
        }
        Ok(())
    }
}

impl ActivityMetricV2 {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Step count validation
        if let Some(steps) = self.step_count {
            if steps < config.steps_min || steps > config.steps_max {
                return Err(format!(
                    "step_count {} is out of range ({}-{})",
                    steps, config.steps_min, config.steps_max
                ));
            }
        }

        // Flights climbed validation
        if let Some(flights) = self.flights_climbed {
            if flights < 0 || flights > 10000 {
                return Err(format!(
                    "flights_climbed {} is out of range (0-10000)",
                    flights
                ));
            }
        }

        // Distance validations - all should be non-negative
        let distance_fields = [
            ("distance_walking_running_meters", self.distance_walking_running_meters),
            ("distance_cycling_meters", self.distance_cycling_meters),
            ("distance_swimming_meters", self.distance_swimming_meters),
            ("distance_wheelchair_meters", self.distance_wheelchair_meters),
            ("distance_downhill_snow_sports_meters", self.distance_downhill_snow_sports_meters),
        ];

        for (field_name, distance_opt) in distance_fields {
            if let Some(distance) = distance_opt {
                if distance < 0.0 {
                    return Err(format!("{} cannot be negative", field_name));
                }
                let distance_km = distance / 1000.0;
                if distance_km > config.distance_max_km {
                    return Err(format!(
                        "{} {} km exceeds maximum of {} km",
                        field_name, distance_km, config.distance_max_km
                    ));
                }
            }
        }

        // Activity count validations
        if let Some(push_count) = self.push_count {
            if push_count < 0 || push_count > 50000 {
                return Err(format!(
                    "push_count {} is out of range (0-50000)",
                    push_count
                ));
            }
        }

        if let Some(stroke_count) = self.swimming_stroke_count {
            if stroke_count < 0 || stroke_count > 100000 {
                return Err(format!(
                    "swimming_stroke_count {} is out of range (0-100000)",
                    stroke_count
                ));
            }
        }

        if let Some(nike_fuel) = self.nike_fuel {
            if nike_fuel < 0.0 || nike_fuel > 50000.0 {
                return Err(format!(
                    "nike_fuel {} is out of range (0-50000)",
                    nike_fuel
                ));
            }
        }

        // Energy validations
        if let Some(active_energy) = self.active_energy_burned_kcal {
            if active_energy < 0.0 || active_energy > config.calories_max {
                return Err(format!(
                    "active_energy_burned_kcal {} is out of range (0-{})",
                    active_energy, config.calories_max
                ));
            }
        }

        if let Some(basal_energy) = self.basal_energy_burned_kcal {
            if basal_energy < 0.0 || basal_energy > 10000.0 {
                return Err(format!(
                    "basal_energy_burned_kcal {} is out of range (0-10000)",
                    basal_energy
                ));
            }
        }

        // Apple Fitness ring validations (minutes should be within daily range)
        let time_fields = [
            ("exercise_time_minutes", self.exercise_time_minutes),
            ("stand_time_minutes", self.stand_time_minutes),
            ("move_time_minutes", self.move_time_minutes),
        ];

        for (field_name, time_opt) in time_fields {
            if let Some(minutes) = time_opt {
                if minutes < 0 || minutes > 1440 { // 1440 minutes = 24 hours
                    return Err(format!(
                        "{} {} is out of range (0-1440 minutes)",
                        field_name, minutes
                    ));
                }
            }
        }

        // Aggregation period validation
        if let Some(ref period) = self.aggregation_period {
            if !["minute", "hourly", "daily", "weekly"].contains(&period.as_str()) {
                return Err(format!(
                    "aggregation_period '{}' must be one of: minute, hourly, daily, weekly",
                    period
                ));
            }
        }

        Ok(())
    }

    /// Convert ActivityMetric to ActivityMetricV2 with field mapping
    pub fn from_activity_metric(metric: &ActivityMetric) -> Self {
        Self {
            recorded_at: metric.date.and_hms_opt(12, 0, 0).unwrap().and_utc(), // Convert date to datetime at noon UTC
            step_count: metric.steps,
            flights_climbed: metric.flights_climbed,
            distance_walking_running_meters: metric.distance_meters,
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel: None,
            active_energy_burned_kcal: metric.calories_burned,
            basal_energy_burned_kcal: None,
            exercise_time_minutes: metric.active_minutes,
            stand_time_minutes: None,
            move_time_minutes: None,
            stand_hour_achieved: None,
            aggregation_period: Some("daily".to_string()),
            source: metric.source.clone(),
        }
    }

    /// Convert ActivityMetricV2 to ActivityMetric for backward compatibility
    pub fn to_activity_metric(&self) -> ActivityMetric {
        ActivityMetric {
            date: self.recorded_at.date_naive(),
            steps: self.step_count,
            distance_meters: self.distance_walking_running_meters,
            calories_burned: self.active_energy_burned_kcal,
            active_minutes: self.exercise_time_minutes,
            flights_climbed: self.flights_climbed,
            source: self.source.clone(),
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
