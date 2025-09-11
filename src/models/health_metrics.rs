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
    pub push_count: Option<i32>, // wheelchair pushes
    pub swimming_stroke_count: Option<i32>,
    pub nike_fuel: Option<f64>, // Nike Fuel points

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
    Nutrition(NutritionMetric),
    Symptom(SymptomMetric),
    ReproductiveHealth(ReproductiveHealthMetric),
    Environmental(EnvironmentalMetric),
    MentalHealth(MentalHealthMetric),
    Mobility(MobilityMetric),
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
    // Individual metric collections for easier processing
    #[serde(default)]
    pub nutrition_metrics: Vec<NutritionMetric>,
    #[serde(default)]
    pub symptom_metrics: Vec<SymptomMetric>,
    #[serde(default)]
    pub reproductive_health_metrics: Vec<ReproductiveHealthMetric>,
    #[serde(default)]
    pub environmental_metrics: Vec<EnvironmentalMetric>,
    #[serde(default)]
    pub mental_health_metrics: Vec<MentalHealthMetric>,
    #[serde(default)]
    pub mobility_metrics: Vec<MobilityMetric>,
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
            (
                "distance_walking_running_meters",
                self.distance_walking_running_meters,
            ),
            ("distance_cycling_meters", self.distance_cycling_meters),
            ("distance_swimming_meters", self.distance_swimming_meters),
            (
                "distance_wheelchair_meters",
                self.distance_wheelchair_meters,
            ),
            (
                "distance_downhill_snow_sports_meters",
                self.distance_downhill_snow_sports_meters,
            ),
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
                return Err(format!("nike_fuel {} is out of range (0-50000)", nike_fuel));
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
                if minutes < 0 || minutes > 1440 {
                    // 1440 minutes = 24 hours
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
            HealthMetric::Nutrition(metric) => metric.validate_with_config(config),
            HealthMetric::Symptom(metric) => metric.validate(),
            HealthMetric::ReproductiveHealth(metric) => metric.validate(),
            HealthMetric::Environmental(metric) => metric.validate(),
            HealthMetric::MentalHealth(metric) => metric.validate(),
            HealthMetric::Mobility(metric) => metric.validate(),
        }
    }

    pub fn metric_type(&self) -> &'static str {
        match self {
            HealthMetric::HeartRate(_) => "HeartRate",
            HealthMetric::BloodPressure(_) => "BloodPressure",
            HealthMetric::Sleep(_) => "Sleep",
            HealthMetric::Activity(_) => "Activity",
            HealthMetric::Workout(_) => "Workout",
            HealthMetric::Nutrition(_) => "Nutrition",
            HealthMetric::Symptom(_) => "Symptom",
            HealthMetric::ReproductiveHealth(_) => "ReproductiveHealth",
            HealthMetric::Environmental(_) => "Environmental",
            HealthMetric::MentalHealth(_) => "MentalHealth",
            HealthMetric::Mobility(_) => "Mobility",
        }
    }
}

/// Nutrition metrics with comprehensive macro, vitamin, and mineral tracking
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NutritionMetric {
    pub recorded_at: DateTime<Utc>,
    
    // Hydration
    pub water_ml: Option<f64>,
    
    // Energy & Macronutrients
    pub energy_consumed_kcal: Option<f64>,
    pub carbohydrates_g: Option<f64>,
    pub protein_g: Option<f64>,
    pub fat_total_g: Option<f64>,
    pub fat_saturated_g: Option<f64>,
    pub fat_monounsaturated_g: Option<f64>,
    pub fat_polyunsaturated_g: Option<f64>,
    pub cholesterol_mg: Option<f64>,
    pub fiber_g: Option<f64>,
    pub sugar_g: Option<f64>,
    pub sodium_mg: Option<f64>,
    
    // Vitamins
    pub vitamin_a_mcg: Option<f64>,
    pub vitamin_d_mcg: Option<f64>,
    pub vitamin_e_mg: Option<f64>,
    pub vitamin_k_mcg: Option<f64>,
    pub vitamin_c_mg: Option<f64>,
    pub thiamin_mg: Option<f64>,
    pub riboflavin_mg: Option<f64>,
    pub niacin_mg: Option<f64>,
    pub pantothenic_acid_mg: Option<f64>,
    pub vitamin_b6_mg: Option<f64>,
    pub biotin_mcg: Option<f64>,
    pub folate_mcg: Option<f64>,
    pub vitamin_b12_mcg: Option<f64>,
    
    // Minerals
    pub calcium_mg: Option<f64>,
    pub phosphorus_mg: Option<f64>,
    pub magnesium_mg: Option<f64>,
    pub potassium_mg: Option<f64>,
    pub chloride_mg: Option<f64>,
    pub iron_mg: Option<f64>,
    pub zinc_mg: Option<f64>,
    pub copper_mg: Option<f64>,
    pub manganese_mg: Option<f64>,
    pub iodine_mcg: Option<f64>,
    pub selenium_mcg: Option<f64>,
    pub chromium_mcg: Option<f64>,
    pub molybdenum_mcg: Option<f64>,
    
    // Other nutrients
    pub caffeine_mg: Option<f64>,
    
    // Metadata
    pub aggregation_period: Option<String>, // meal, daily, weekly
    pub source: Option<String>,
}

/// Symptoms tracking with comprehensive Apple Health symptom types
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SymptomMetric {
    pub recorded_at: DateTime<Utc>,
    pub onset_at: Option<DateTime<Utc>>,
    pub symptom_type: String,
    pub severity: String, // not_present, mild, moderate, severe
    pub duration_minutes: Option<i32>,
    pub triggers: Option<Vec<String>>,
    pub treatments: Option<Vec<String>>,
    pub notes: Option<String>,
    pub source: Option<String>,
}

/// Reproductive health tracking with privacy considerations
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReproductiveHealthMetric {
    pub recorded_at: DateTime<Utc>,
    
    // Menstrual tracking
    pub menstrual_flow: Option<String>, // none, light, medium, heavy, very_heavy
    pub spotting: Option<bool>,
    pub cycle_day: Option<i16>,
    pub cycle_length: Option<i16>,
    
    // Fertility tracking
    pub basal_body_temp: Option<f64>,
    pub cervical_mucus_quality: Option<String>, // dry, sticky, creamy, watery, egg_white, none
    pub ovulation_test_result: Option<String>, // negative, positive, peak, high, low, not_tested
    pub fertile_window: Option<bool>,
    
    // Pregnancy tracking
    pub pregnancy_test_result: Option<String>, // negative, positive, indeterminate, not_tested
    pub pregnancy_status: Option<String>, // not_pregnant, trying_to_conceive, pregnant, postpartum, unknown
    pub gestational_age_weeks: Option<i16>,
    
    // Sexual health (stored as regular fields - encryption happens at DB layer)
    pub sexual_activity: Option<bool>,
    pub contraceptive_method: Option<String>,
    
    // Symptoms & mood
    pub symptoms: Option<Vec<String>>,
    pub cycle_related_mood: Option<String>, // very_negative, negative, neutral, positive, very_positive, not_assessed
    
    // Metadata
    pub source: Option<String>,
    pub notes: Option<String>,
}

/// Environmental metrics with Apple Watch Series 8+ compatibility
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnvironmentalMetric {
    pub recorded_at: DateTime<Utc>,
    
    // Audio exposure
    pub environmental_sound_level_db: Option<f64>,
    pub headphone_exposure_db: Option<f64>,
    pub noise_reduction_db: Option<f64>,
    pub exposure_duration_seconds: Option<i32>,
    
    // UV exposure
    pub uv_index: Option<f64>,
    pub time_in_sun_minutes: Option<i32>,
    pub time_in_shade_minutes: Option<i32>,
    pub sunscreen_applied: Option<bool>,
    pub uv_dose_joules_per_m2: Option<f64>,
    
    // Fall detection & safety
    pub fall_detected: Option<bool>,
    pub fall_severity: Option<String>, // low, medium, high, severe
    pub impact_force_g: Option<f64>,
    pub emergency_contacted: Option<bool>,
    pub fall_response_time_seconds: Option<i32>,
    
    // Hygiene tracking
    pub handwashing_events: Option<i32>,
    pub handwashing_duration_seconds: Option<i32>,
    pub toothbrushing_events: Option<i32>,
    pub toothbrushing_duration_seconds: Option<i32>,
    
    // Air quality
    pub pm2_5_micrograms_m3: Option<f64>,
    pub pm10_micrograms_m3: Option<f64>,
    pub air_quality_index: Option<i16>,
    pub ozone_ppb: Option<f64>,
    pub no2_ppb: Option<f64>,
    pub so2_ppb: Option<f64>,
    pub co_ppm: Option<f64>,
    
    // Location & context
    pub altitude_meters: Option<f64>,
    pub barometric_pressure_hpa: Option<f64>,
    pub indoor_outdoor_context: Option<String>, // indoor, outdoor, mixed, unknown
    
    // Aggregation
    pub aggregation_period: Option<String>, // event, hourly, daily
    pub measurement_count: Option<i32>,
    
    // Metadata
    pub source: Option<String>,
    pub device_type: Option<String>,
}

/// Mental health metrics with iOS 17+ State of Mind support
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MentalHealthMetric {
    pub recorded_at: DateTime<Utc>,
    
    // Mindfulness tracking
    pub mindful_minutes: Option<f64>,
    
    // Mood tracking (iOS 17+)
    pub mood_valence: Option<f64>, // -1.0 to 1.0 scale
    pub mood_labels: Option<Vec<String>>, // happy, sad, anxious, calm, etc.
    
    // Daylight exposure
    pub daylight_minutes: Option<f64>,
    
    // Stress level
    pub stress_level: Option<String>, // low, medium, high, critical
    
    // Mental health screening
    pub depression_score: Option<i16>, // PHQ-9 scale (0-27)
    pub anxiety_score: Option<i16>, // GAD-7 scale (0-21)
    pub sleep_quality_score: Option<i16>, // 1-10 scale
    
    // Metadata
    pub source: Option<String>,
    pub notes: Option<String>,
}

/// Mobility metrics for gait analysis and movement tracking
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MobilityMetric {
    pub recorded_at: DateTime<Utc>,
    
    // Gait analysis
    pub walking_speed_m_per_s: Option<f64>,
    pub step_length_cm: Option<f64>,
    pub double_support_percentage: Option<f64>,
    pub walking_asymmetry_percentage: Option<f64>,
    pub walking_steadiness: Option<String>, // ok, low, very_low
    
    // Stair climbing
    pub stair_ascent_speed: Option<f64>,
    pub stair_descent_speed: Option<f64>,
    
    // Balance and stability
    pub six_minute_walk_test_distance: Option<f64>,
    pub walking_heart_rate_recovery: Option<i16>,
    
    // Apple Watch mobility metrics
    pub low_cardio_fitness_event: Option<bool>,
    pub walking_heart_rate_average: Option<i16>,
    
    // Metadata
    pub source: Option<String>,
    pub device_type: Option<String>,
}

impl NutritionMetric {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_config(&ValidationConfig::default())
    }

    pub fn validate_with_config(&self, config: &ValidationConfig) -> Result<(), String> {
        // Hydration validation
        if let Some(water) = self.water_ml {
            if water < 0.0 || water > 20000.0 { // 0-20L max per day
                return Err(format!("water_ml {} is out of range (0-20000)", water));
            }
        }
        
        // Energy validation
        if let Some(energy) = self.energy_consumed_kcal {
            if energy < 0.0 || energy > 20000.0 { // 0-20k kcal max
                return Err(format!("energy_consumed_kcal {} is out of range (0-20000)", energy));
            }
        }
        
        // Macronutrient validation
        if let Some(carbs) = self.carbohydrates_g {
            if carbs < 0.0 || carbs > 3000.0 {
                return Err(format!("carbohydrates_g {} is out of range (0-3000)", carbs));
            }
        }
        
        if let Some(protein) = self.protein_g {
            if protein < 0.0 || protein > 1000.0 {
                return Err(format!("protein_g {} is out of range (0-1000)", protein));
            }
        }
        
        // Validate aggregation period
        if let Some(ref period) = self.aggregation_period {
            if !["meal", "daily", "weekly"].contains(&period.as_str()) {
                return Err(format!("aggregation_period '{}' must be one of: meal, daily, weekly", period));
            }
        }
        
        Ok(())
    }
}

impl SymptomMetric {
    pub fn validate(&self) -> Result<(), String> {
        // Validate severity
        if !["not_present", "mild", "moderate", "severe"].contains(&self.severity.as_str()) {
            return Err(format!("severity '{}' must be one of: not_present, mild, moderate, severe", self.severity));
        }
        
        // Validate symptom type against known types
        let valid_symptoms = [
            // General/Constitutional
            "fever", "fatigue", "weakness", "night_sweats", "chills", "malaise", 
            "appetite_loss", "weight_loss", "weight_gain",
            // Head & Neurological
            "headache", "dizziness", "lightheadedness", "confusion", "memory_issues", 
            "concentration_difficulty", "mood_changes", "anxiety", "depression",
            // Respiratory
            "cough", "shortness_of_breath", "chest_tightness_or_pain", "wheezing", 
            "runny_nose", "sinus_congestion", "sneezing", "sore_throat",
            // Gastrointestinal
            "nausea", "vomiting", "abdominal_cramps", "bloating", "diarrhea", 
            "constipation", "heartburn", "acid_reflux", "stomach_pain", "gas", "indigestion",
            // And more...
        ];
        
        if !valid_symptoms.contains(&self.symptom_type.as_str()) {
            // Allow unknown symptom types but log warning
            tracing::warn!("Unknown symptom type: {}", self.symptom_type);
        }
        
        // Validate duration
        if let Some(duration) = self.duration_minutes {
            if duration < 0 || duration > 10080 { // Max 1 week (7 * 24 * 60)
                return Err(format!("duration_minutes {} is out of range (0-10080)", duration));
            }
        }
        
        // Validate onset_at is not in the future
        if let Some(onset_at) = self.onset_at {
            if onset_at > Utc::now() {
                return Err("onset_at cannot be in the future".to_string());
            }
        }
        
        Ok(())
    }
}

impl ReproductiveHealthMetric {
    pub fn validate(&self) -> Result<(), String> {
        // Validate menstrual flow
        if let Some(ref flow) = self.menstrual_flow {
            if !["none", "light", "medium", "heavy", "very_heavy"].contains(&flow.as_str()) {
                return Err(format!("menstrual_flow '{}' must be one of: none, light, medium, heavy, very_heavy", flow));
            }
        }
        
        // Validate cycle day and length
        if let Some(cycle_day) = self.cycle_day {
            if cycle_day < 1 || cycle_day > 60 {
                return Err(format!("cycle_day {} is out of range (1-60)", cycle_day));
            }
        }
        
        if let Some(cycle_length) = self.cycle_length {
            if cycle_length < 18 || cycle_length > 60 {
                return Err(format!("cycle_length {} is out of range (18-60)", cycle_length));
            }
        }
        
        // Validate basal body temperature
        if let Some(temp) = self.basal_body_temp {
            if temp < 35.0 || temp > 40.0 {
                return Err(format!("basal_body_temp {} is out of range (35.0-40.0)", temp));
            }
        }
        
        // Validate gestational age
        if let Some(weeks) = self.gestational_age_weeks {
            if weeks < 0 || weeks > 50 {
                return Err(format!("gestational_age_weeks {} is out of range (0-50)", weeks));
            }
        }
        
        Ok(())
    }
}

impl EnvironmentalMetric {
    pub fn validate(&self) -> Result<(), String> {
        // Audio exposure validation (WHO safety guidelines)
        if let Some(env_sound) = self.environmental_sound_level_db {
            if env_sound < 0.0 || env_sound > 140.0 {
                return Err(format!("environmental_sound_level_db {} is out of range (0-140)", env_sound));
            }
        }
        
        if let Some(headphone_db) = self.headphone_exposure_db {
            if headphone_db < 0.0 || headphone_db > 140.0 {
                return Err(format!("headphone_exposure_db {} is out of range (0-140)", headphone_db));
            }
        }
        
        // UV exposure validation
        if let Some(uv) = self.uv_index {
            if uv < 0.0 || uv > 15.0 {
                return Err(format!("uv_index {} is out of range (0-15)", uv));
            }
        }
        
        // Fall detection validation
        if let Some(impact) = self.impact_force_g {
            if impact < 0.0 || impact > 50.0 {
                return Err(format!("impact_force_g {} is out of range (0-50)", impact));
            }
        }
        
        // Air quality validation
        if let Some(aqi) = self.air_quality_index {
            if aqi < 0 || aqi > 500 {
                return Err(format!("air_quality_index {} is out of range (0-500)", aqi));
            }
        }
        
        // Validate aggregation period
        if let Some(ref period) = self.aggregation_period {
            if !["event", "hourly", "daily"].contains(&period.as_str()) {
                return Err(format!("aggregation_period '{}' must be one of: event, hourly, daily", period));
            }
        }
        
        Ok(())
    }
}

impl MentalHealthMetric {
    pub fn validate(&self) -> Result<(), String> {
        // Mindful minutes validation
        if let Some(minutes) = self.mindful_minutes {
            if minutes < 0.0 || minutes > 1440.0 { // Max 24 hours
                return Err(format!("mindful_minutes {} is out of range (0-1440)", minutes));
            }
        }
        
        // Mood valence validation
        if let Some(valence) = self.mood_valence {
            if valence < -1.0 || valence > 1.0 {
                return Err(format!("mood_valence {} is out of range (-1.0 to 1.0)", valence));
            }
        }
        
        // Daylight minutes validation
        if let Some(daylight) = self.daylight_minutes {
            if daylight < 0.0 || daylight > 1440.0 { // Max 24 hours
                return Err(format!("daylight_minutes {} is out of range (0-1440)", daylight));
            }
        }
        
        // Validate stress level
        if let Some(ref stress) = self.stress_level {
            if !["low", "medium", "high", "critical"].contains(&stress.as_str()) {
                return Err(format!("stress_level '{}' must be one of: low, medium, high, critical", stress));
            }
        }
        
        // Screening scores validation
        if let Some(depression) = self.depression_score {
            if depression < 0 || depression > 27 { // PHQ-9 scale
                return Err(format!("depression_score {} is out of range (0-27)", depression));
            }
        }
        
        if let Some(anxiety) = self.anxiety_score {
            if anxiety < 0 || anxiety > 21 { // GAD-7 scale
                return Err(format!("anxiety_score {} is out of range (0-21)", anxiety));
            }
        }
        
        if let Some(sleep_quality) = self.sleep_quality_score {
            if sleep_quality < 1 || sleep_quality > 10 {
                return Err(format!("sleep_quality_score {} is out of range (1-10)", sleep_quality));
            }
        }
        
        // Validate mood labels array
        if let Some(ref labels) = self.mood_labels {
            if labels.is_empty() {
                return Err("mood_labels array cannot be empty".to_string());
            }
            for label in labels {
                if label.len() > 30 {
                    return Err(format!("mood label '{}' exceeds 30 characters", label));
                }
            }
        }
        
        Ok(())
    }
}

impl MobilityMetric {
    pub fn validate(&self) -> Result<(), String> {
        // Walking speed validation (typical range 0.5-3.0 m/s)
        if let Some(speed) = self.walking_speed_m_per_s {
            if speed < 0.0 || speed > 5.0 {
                return Err(format!("walking_speed_m_per_s {} is out of range (0-5)", speed));
            }
        }
        
        // Step length validation (typical range 40-80 cm)
        if let Some(length) = self.step_length_cm {
            if length < 0.0 || length > 150.0 {
                return Err(format!("step_length_cm {} is out of range (0-150)", length));
            }
        }
        
        // Percentage validations
        if let Some(support) = self.double_support_percentage {
            if support < 0.0 || support > 100.0 {
                return Err(format!("double_support_percentage {} is out of range (0-100)", support));
            }
        }
        
        if let Some(asymmetry) = self.walking_asymmetry_percentage {
            if asymmetry < 0.0 || asymmetry > 100.0 {
                return Err(format!("walking_asymmetry_percentage {} is out of range (0-100)", asymmetry));
            }
        }
        
        // Validate walking steadiness
        if let Some(ref steadiness) = self.walking_steadiness {
            if !["ok", "low", "very_low"].contains(&steadiness.as_str()) {
                return Err(format!("walking_steadiness '{}' must be one of: ok, low, very_low", steadiness));
            }
        }
        
        // Six minute walk test validation (typical range 150-700m)
        if let Some(distance) = self.six_minute_walk_test_distance {
            if distance < 0.0 || distance > 1000.0 {
                return Err(format!("six_minute_walk_test_distance {} is out of range (0-1000)", distance));
            }
        }
        
        Ok(())
    }
}
