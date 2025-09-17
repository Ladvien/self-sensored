use crate::models::enums::WorkoutType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Models that match the iOS Auto Health Export app JSON structure
/// Root payload structure from iOS app
#[derive(Debug, Deserialize, Serialize)]
pub struct IosIngestPayload {
    pub data: IosIngestData,
}

/// Container for iOS health data - CRITICAL: iOS sends metrics as a HashMap, not Vec!
/// The actual format is: "metrics": { "HKQuantityTypeIdentifierHeartRate": [...] }
#[derive(Debug, Deserialize, Serialize)]
pub struct IosIngestData {
    // FIXME: This should be HashMap<String, Vec<IosMetricDataPoint>> to match actual iOS format
    // Current format is wrong - iOS Auto Health Export sends grouped metrics by HealthKit identifier
    pub metrics: Vec<IosMetric>, // This is the WRONG format - keeping for backward compatibility
    // Correct format would be:
    // pub metrics: std::collections::HashMap<String, Vec<IosMetricDataPoint>>,

    // workouts may not be present in iOS app structure
    #[serde(default)]
    pub workouts: Vec<IosWorkout>,
}

/// DEPRECATED: iOS metric structure - this format is incorrect for actual iOS app
/// The iOS Auto Health Export app sends metrics grouped by HealthKit identifier keys
/// TODO: Replace with correct format that matches Postman collection examples
#[derive(Debug, Deserialize, Serialize)]
pub struct IosMetric {
    pub name: String,
    pub units: Option<String>,
    pub data: Vec<IosMetricData>,
}

/// Individual iOS metric data point
#[derive(Debug, Deserialize, Serialize)]
pub struct IosMetricData {
    // Common fields across all metrics
    pub source: Option<String>,

    // Time fields - iOS uses string dates
    pub date: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,

    // Value fields
    pub qty: Option<f64>,
    pub value: Option<String>, // For categorical data like "Incomplete"

    // Additional fields that may be present
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// iOS workout structure (placeholder - may need to be updated based on actual data)
#[derive(Debug, Deserialize, Serialize)]
pub struct IosWorkout {
    pub name: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub source: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl IosIngestPayload {
    /// Convert iOS payload to our internal format
    pub fn to_internal_format(&self, user_id: uuid::Uuid) -> crate::models::IngestPayload {
        use crate::models::{HealthMetric, IngestData, IngestPayload, WorkoutData};
        use std::collections::HashMap;

        let mut internal_metrics = Vec::new();
        let mut internal_workouts = Vec::new();

        // Track blood pressure readings by timestamp for pairing
        let mut bp_readings: HashMap<String, (Option<i16>, Option<i16>)> = HashMap::new();

        // Convert iOS metrics to internal format
        for ios_metric in &self.data.metrics {
            for data_point in &ios_metric.data {
                // Parse dates with fallback to various timestamp fields
                let recorded_at = parse_ios_date(&data_point.date)
                    .or_else(|| parse_ios_date(&data_point.start))
                    .or_else(|| parse_ios_date(&data_point.end))
                    .unwrap_or_else(Utc::now);

                // CRITICAL LOGGING: Track all iOS metric names for STORY-DATA-002 audit
                tracing::info!(
                    "Processing iOS metric: '{}' with units: {:?}",
                    ios_metric.name,
                    ios_metric.units
                );

                // Convert based on metric name - now supporting ALL HealthKit identifiers from DATA.md
                // CRITICAL: Handle both HealthKit identifiers (HKQuantityTypeIdentifierHeartRate)
                // and simplified names (heart_rate) for backward compatibility
                match ios_metric.name.as_str() {
                    // HEART RATE METRICS - HealthKit identifiers + backward compatibility
                    "HKQuantityTypeIdentifierHeartRate"
                    | "HKQuantityTypeIdentifierRestingHeartRate"
                    | "HKQuantityTypeIdentifierWalkingHeartRateAverage"
                    | "HKQuantityTypeIdentifierHeartRateVariabilitySDNN"
                    | "HKQuantityTypeIdentifierHeartRateRecoveryOneMinute"
                    | "heart_rate"
                    | "heartrate"
                    | "resting_heart_rate"
                    | "walking_heart_rate"
                    | "heart_rate_variability" => {
                        if let Some(qty) = data_point.qty {
                            if qty > 0.0 && qty <= 300.0 {
                                // Basic validation
                                let context_str = match ios_metric.name.to_lowercase().as_str() {
                                    "resting_heart_rate" => Some("resting"),
                                    "walking_heart_rate" => Some("walking"),
                                    _ => data_point.extra.get("context").and_then(|v| v.as_str()),
                                };

                                let context = context_str.and_then(|s| {
                                    crate::models::ActivityContext::from_ios_string(s)
                                });

                                let metric = crate::models::HeartRateMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    heart_rate: Some(qty as i16),
                                    resting_heart_rate: if context_str == Some("resting") {
                                        Some(qty as i16)
                                    } else {
                                        None
                                    },
                                    heart_rate_variability: None,

                                    // Advanced Cardiovascular Metrics (STORY-011) - Initialize to None for basic iOS data
                                    walking_heart_rate_average: None,
                                    heart_rate_recovery_one_minute: None,
                                    atrial_fibrillation_burden_percentage: None,
                                    vo2_max_ml_kg_min: None,

                                    source_device: data_point.source.clone(),
                                    context,
                                    created_at: Utc::now(),
                                };
                                internal_metrics.push(HealthMetric::HeartRate(metric));
                            }
                        }
                    }
                    // BLOOD PRESSURE METRICS - HealthKit identifiers + backward compatibility
                    "HKQuantityTypeIdentifierBloodPressureSystolic"
                    | "blood_pressure_systolic"
                    | "systolic_blood_pressure" => {
                        if let Some(qty) = data_point.qty {
                            let timestamp_key = recorded_at.to_rfc3339();
                            let entry = bp_readings.entry(timestamp_key).or_default();
                            entry.0 = Some(qty as i16);
                        }
                    }
                    "HKQuantityTypeIdentifierBloodPressureDiastolic"
                    | "blood_pressure_diastolic"
                    | "diastolic_blood_pressure" => {
                        if let Some(qty) = data_point.qty {
                            let timestamp_key = recorded_at.to_rfc3339();
                            let entry = bp_readings.entry(timestamp_key).or_default();
                            entry.1 = Some(qty as i16);
                        }
                    }
                    // SLEEP METRICS - HealthKit identifiers + backward compatibility
                    "HKCategoryTypeIdentifierSleepAnalysis"
                    | "sleep_analysis"
                    | "sleep"
                    | "sleep_time" => {
                        let start_time = parse_ios_date(&data_point.start).unwrap_or(recorded_at);
                        let end_time = parse_ios_date(&data_point.end).unwrap_or(recorded_at);

                        let total_minutes = (end_time - start_time).num_minutes() as i32;

                        if total_minutes > 0 && total_minutes <= 24 * 60 {
                            // Max 24 hours
                            // Extract sleep stages from extra fields if available
                            let deep_sleep_minutes = data_point
                                .extra
                                .get("deep_sleep_minutes")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32);
                            let rem_sleep_minutes = data_point
                                .extra
                                .get("rem_sleep_minutes")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32);
                            let awake_minutes = data_point
                                .extra
                                .get("awake_minutes")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32);

                            let metric = crate::models::SleepMetric {
                                id: uuid::Uuid::new_v4(),
                                user_id,
                                sleep_start: start_time,
                                sleep_end: end_time,
                                duration_minutes: Some(total_minutes),
                                deep_sleep_minutes,
                                rem_sleep_minutes,
                                light_sleep_minutes: None, // iOS may not provide this separately
                                awake_minutes,
                                efficiency: None,
                                source_device: data_point.source.clone(),
                                created_at: Utc::now(),
                            };
                            internal_metrics.push(HealthMetric::Sleep(metric));
                        }
                    }
                    // ACTIVITY METRICS - HealthKit identifiers + backward compatibility
                    "HKQuantityTypeIdentifierStepCount"
                    | "HKQuantityTypeIdentifierDistanceWalkingRunning"
                    | "HKQuantityTypeIdentifierDistanceCycling"
                    | "HKQuantityTypeIdentifierDistanceSwimming"
                    | "HKQuantityTypeIdentifierDistanceWheelchair"
                    | "HKQuantityTypeIdentifierDistanceDownhillSnowSports"
                    | "HKQuantityTypeIdentifierActiveEnergyBurned"
                    | "HKQuantityTypeIdentifierBasalEnergyBurned"
                    | "HKQuantityTypeIdentifierFlightsClimbed"
                    | "HKQuantityTypeIdentifierPushCount"
                    | "HKQuantityTypeIdentifierSwimmingStrokeCount"
                    | "HKQuantityTypeIdentifierNikeFuel"
                    | "HKQuantityTypeIdentifierAppleExerciseTime"
                    | "HKQuantityTypeIdentifierAppleStandTime"
                    | "HKQuantityTypeIdentifierAppleMoveTime"
                    | "HKCategoryTypeIdentifierAppleStandHour"
                    | "steps"
                    | "step_count"
                    | "distance_walking_running"
                    | "distance"
                    | "active_energy_burned"
                    | "basal_energy_burned"
                    | "calories"
                    | "flights_climbed" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                // Generate UUID for the activity metric
                                let id = uuid::Uuid::new_v4();

                                // Basic validation - no negative values
                                let metric = crate::models::ActivityMetric {
                                    id,
                                    user_id,
                                    recorded_at,
                                    step_count: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierStepCount"
                                            | "steps"
                                            | "step_count"
                                    ) {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    distance_meters: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierDistanceWalkingRunning"
                                            | "distance_walking_running"
                                            | "distance"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    active_energy_burned_kcal: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierActiveEnergyBurned"
                                            | "active_energy_burned"
                                            | "calories"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    basal_energy_burned_kcal: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBasalEnergyBurned"
                                            | "basal_energy_burned"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    flights_climbed: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierFlightsClimbed"
                                            | "flights_climbed"
                                    ) {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    // Extended activity metrics - now mapped from HealthKit identifiers
                                    distance_cycling_meters: if ios_metric.name
                                        == "HKQuantityTypeIdentifierDistanceCycling"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    distance_swimming_meters: if ios_metric.name
                                        == "HKQuantityTypeIdentifierDistanceSwimming"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    distance_wheelchair_meters: if ios_metric.name
                                        == "HKQuantityTypeIdentifierDistanceWheelchair"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    distance_downhill_snow_sports_meters: if ios_metric.name
                                        == "HKQuantityTypeIdentifierDistanceDownhillSnowSports"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    push_count: if ios_metric.name
                                        == "HKQuantityTypeIdentifierPushCount"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    swimming_stroke_count: if ios_metric.name
                                        == "HKQuantityTypeIdentifierSwimmingStrokeCount"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    nike_fuel_points: if ios_metric.name
                                        == "HKQuantityTypeIdentifierNikeFuel"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    apple_exercise_time_minutes: if ios_metric.name
                                        == "HKQuantityTypeIdentifierAppleExerciseTime"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    apple_stand_time_minutes: if ios_metric.name
                                        == "HKQuantityTypeIdentifierAppleStandTime"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    apple_move_time_minutes: if ios_metric.name
                                        == "HKQuantityTypeIdentifierAppleMoveTime"
                                    {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    apple_stand_hour_achieved: if ios_metric.name
                                        == "HKCategoryTypeIdentifierAppleStandHour"
                                    {
                                        Some(qty > 0.0) // Convert to boolean
                                    } else {
                                        None
                                    },
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics.push(HealthMetric::Activity(metric));
                            }
                        }
                    }
                    // TEMPERATURE METRICS - HealthKit identifiers + backward compatibility
                    "HKQuantityTypeIdentifierBodyTemperature"
                    | "HKQuantityTypeIdentifierBasalBodyTemperature"
                    | "HKQuantityTypeIdentifierAppleSleepingWristTemperature"
                    | "HKQuantityTypeIdentifierWaterTemperature"
                    | "body_temperature"
                    | "basal_body_temperature"
                    | "apple_sleeping_wrist_temperature"
                    | "wrist_temperature"
                    | "water_temperature"
                    | "temperature" => {
                        if let Some(qty) = data_point.qty {
                            // Temperature should be in Celsius, validate range
                            if (-50.0..=100.0).contains(&qty) {
                                let metric = crate::models::TemperatureMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    body_temperature: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBodyTemperature"
                                            | "body_temperature"
                                            | "temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    basal_body_temperature: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBasalBodyTemperature"
                                            | "basal_body_temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    apple_sleeping_wrist_temperature: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierAppleSleepingWristTemperature"
                                            | "apple_sleeping_wrist_temperature"
                                            | "wrist_temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    water_temperature: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierWaterTemperature"
                                            | "water_temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    temperature_source: data_point
                                        .extra
                                        .get("source_type")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .or_else(|| Some("iOS".to_string())),
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics.push(HealthMetric::Temperature(metric));
                            }
                        }
                    }
                    // ENVIRONMENTAL & SAFETY METRICS - HealthKit identifiers + backward compatibility
                    // Note: HealthKit doesn't have standard UV identifiers, these are likely custom
                    "uv_exposure" | "uv_index" | "environmental_uv_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::EnvironmentalMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    environmental_audio_exposure_db: None,
                                    headphone_audio_exposure_db: None,
                                    uv_index: Some(qty),
                                    uv_exposure_minutes: None,
                                    time_in_daylight_minutes: None,
                                    ambient_temperature_celsius: None,
                                    humidity_percent: None,
                                    air_pressure_hpa: None,
                                    altitude_meters: None,
                                    location_latitude: data_point
                                        .extra
                                        .get("latitude")
                                        .and_then(|v| v.as_f64()),
                                    location_longitude: data_point
                                        .extra
                                        .get("longitude")
                                        .and_then(|v| v.as_f64()),
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::Environmental(metric));
                            }
                        }
                    }
                    "time_in_daylight" | "daylight_time" | "sun_exposure_time" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::EnvironmentalMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    environmental_audio_exposure_db: None,
                                    headphone_audio_exposure_db: None,
                                    uv_index: None,
                                    uv_exposure_minutes: None,
                                    time_in_daylight_minutes: Some(qty as i32),
                                    ambient_temperature_celsius: None,
                                    humidity_percent: None,
                                    air_pressure_hpa: None,
                                    altitude_meters: None,
                                    location_latitude: data_point
                                        .extra
                                        .get("latitude")
                                        .and_then(|v| v.as_f64()),
                                    location_longitude: data_point
                                        .extra
                                        .get("longitude")
                                        .and_then(|v| v.as_f64()),
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::Environmental(metric));
                            }
                        }
                    }
                    // AUDIO EXPOSURE METRICS - Critical for data loss prevention
                    "HKQuantityTypeIdentifierEnvironmentalAudioExposure"
                    | "environmental_audio_exposure"
                    | "environmental_sound_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let duration_minutes = data_point
                                    .extra
                                    .get("duration_minutes")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(1)
                                    as i32;

                                let audio_exposure_event = qty >= 85.0; // WHO safe listening threshold

                                let metric = crate::models::AudioExposureMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    environmental_audio_exposure_db: Some(qty),
                                    headphone_audio_exposure_db: None,
                                    exposure_duration_minutes: duration_minutes,
                                    audio_exposure_event,
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::AudioExposure(metric));
                            }
                        }
                    }
                    "HKQuantityTypeIdentifierHeadphoneAudioExposure"
                    | "headphone_audio_exposure"
                    | "headphone_sound_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let duration_minutes = data_point
                                    .extra
                                    .get("duration_minutes")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(1)
                                    as i32;

                                let audio_exposure_event = qty >= 85.0; // WHO safe listening threshold

                                let metric = crate::models::AudioExposureMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    environmental_audio_exposure_db: None,
                                    headphone_audio_exposure_db: Some(qty),
                                    exposure_duration_minutes: duration_minutes,
                                    audio_exposure_event,
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::AudioExposure(metric));
                            }
                        }
                    }
                    "fall_detection" | "number_of_times_fallen" | "falls_detected" => {
                        if let Some(qty) = data_point.qty {
                            if qty > 0.0 {
                                let metric = crate::models::SafetyEventMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    event_type: "fall_detected".to_string(),
                                    severity_level: Some(3), // Default moderate severity for falls
                                    location_latitude: data_point
                                        .extra
                                        .get("latitude")
                                        .and_then(|v| v.as_f64()),
                                    location_longitude: data_point
                                        .extra
                                        .get("longitude")
                                        .and_then(|v| v.as_f64()),
                                    emergency_contacts_notified: data_point
                                        .extra
                                        .get("emergency_contacts_notified")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    resolution_status: Some("pending".to_string()),
                                    notes: data_point
                                        .extra
                                        .get("notes")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::SafetyEvent(metric));
                            }
                        }
                    }
                    // BODY MEASUREMENT METRICS - HealthKit identifiers + backward compatibility
                    "HKQuantityTypeIdentifierBodyMass"
                    | "HKQuantityTypeIdentifierBodyMassIndex"
                    | "HKQuantityTypeIdentifierBodyFatPercentage"
                    | "HKQuantityTypeIdentifierLeanBodyMass"
                    | "HKQuantityTypeIdentifierHeight"
                    | "HKQuantityTypeIdentifierWaistCircumference"
                    | "body_mass"
                    | "weight"
                    | "body_weight"
                    | "body_mass_kg"
                    | "body_mass_index"
                    | "bmi"
                    | "body_fat_percentage"
                    | "body_fat"
                    | "lean_body_mass"
                    | "lean_body_mass_kg"
                    | "muscle_mass"
                    | "height"
                    | "height_cm"
                    | "waist_circumference"
                    | "waist"
                    | "hip_circumference"
                    | "hip"
                    | "chest_circumference"
                    | "chest"
                    | "arm_circumference"
                    | "arm"
                    | "thigh_circumference"
                    | "thigh" => {
                        if let Some(qty) = data_point.qty {
                            // Validate body measurement ranges
                            let is_valid = match ios_metric.name.to_lowercase().as_str() {
                                name if name.contains("weight") || name.contains("mass") => {
                                    (20.0..=500.0).contains(&qty)
                                }
                                name if name.contains("bmi") => (10.0..=60.0).contains(&qty),
                                name if name.contains("fat") => (3.0..=50.0).contains(&qty),
                                name if name.contains("height") => (50.0..=250.0).contains(&qty),
                                name if name.contains("circumference")
                                    || name.contains("waist")
                                    || name.contains("hip")
                                    || name.contains("chest") =>
                                {
                                    (15.0..=200.0).contains(&qty)
                                }
                                _ => (0.0..=1000.0).contains(&qty), // General range
                            };

                            if is_valid {
                                let metric = crate::models::BodyMeasurementMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    // Weight & Body Composition
                                    body_weight_kg: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBodyMass"
                                            | "body_mass"
                                            | "weight"
                                            | "body_weight"
                                            | "body_mass_kg"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    body_mass_index: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBodyMassIndex"
                                            | "body_mass_index"
                                            | "bmi"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    body_fat_percentage: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierBodyFatPercentage"
                                            | "body_fat_percentage"
                                            | "body_fat"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    lean_body_mass_kg: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierLeanBodyMass"
                                            | "lean_body_mass"
                                            | "lean_body_mass_kg"
                                            | "muscle_mass"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    // Physical Measurements
                                    height_cm: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierHeight" | "height" | "height_cm"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    waist_circumference_cm: if matches!(
                                        ios_metric.name.as_str(),
                                        "HKQuantityTypeIdentifierWaistCircumference"
                                            | "waist_circumference"
                                            | "waist"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    hip_circumference_cm: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "hip_circumference" | "hip"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    chest_circumference_cm: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "chest_circumference" | "chest"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    arm_circumference_cm: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "arm_circumference" | "arm"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    thigh_circumference_cm: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "thigh_circumference" | "thigh"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    // Body Temperature (if available in body measurement context)
                                    body_temperature_celsius: None,
                                    basal_body_temperature_celsius: None,
                                    // Metadata
                                    measurement_source: Some("iOS".to_string()),
                                    source_device: data_point.source.clone(),
                                    created_at: Utc::now(),
                                };
                                internal_metrics
                                    .push(crate::models::HealthMetric::BodyMeasurement(metric));
                            } else {
                                tracing::warn!(
                                    "Invalid body measurement value for {}: {} (outside valid range)",
                                    ios_metric.name, qty
                                );
                            }
                        }
                    }
                    _ => {
                        // CRITICAL: Unknown metric type detected - potential data loss!
                        tracing::warn!(
                            "🚨 UNKNOWN iOS METRIC TYPE: '{}' with units: {:?}, qty: {:?} - POTENTIAL DATA LOSS!",
                            ios_metric.name,
                            ios_metric.units,
                            data_point.qty
                        );

                        // Log detailed information for iOS metric analysis
                        tracing::info!(
                            "Unknown metric details - Source: {:?}, Date: {:?}, Start: {:?}, End: {:?}, Extra fields: {:?}",
                            data_point.source,
                            data_point.date,
                            data_point.start,
                            data_point.end,
                            data_point.extra.keys().collect::<Vec<_>>()
                        );

                        // Check for high-priority HealthKit identifiers that are missing mapping
                        let critical_missing_identifiers = [
                            // Respiratory metrics
                            "HKQuantityTypeIdentifierRespiratoryRate",
                            "HKQuantityTypeIdentifierOxygenSaturation",
                            "HKQuantityTypeIdentifierPeakExpiratoryFlowRate",
                            "HKQuantityTypeIdentifierInhalerUsage",
                            // Blood & metabolic metrics
                            "HKQuantityTypeIdentifierBloodGlucose",
                            "HKQuantityTypeIdentifierBloodAlcoholContent",
                            "HKQuantityTypeIdentifierInsulinDelivery",
                            // Nutrition metrics
                            "HKQuantityTypeIdentifierDietaryWater",
                            "HKQuantityTypeIdentifierDietaryEnergyConsumed",
                            "HKQuantityTypeIdentifierDietaryCarbohydrates",
                            "HKQuantityTypeIdentifierDietaryProtein",
                            "HKQuantityTypeIdentifierDietaryFatTotal",
                            "HKQuantityTypeIdentifierDietarySodium",
                            "HKQuantityTypeIdentifierDietaryFiber",
                            "HKQuantityTypeIdentifierDietaryCaffeine",
                            // Mental health & mindfulness
                            "HKCategoryTypeIdentifierMindfulSession",
                            "HKStateOfMind",
                            // Reproductive health
                            "HKCategoryTypeIdentifierMenstrualFlow",
                            "HKCategoryTypeIdentifierSexualActivity",
                            "HKCategoryTypeIdentifierOvulationTestResult",
                            // Symptoms
                            "HKCategoryTypeIdentifierHeadache",
                            "HKCategoryTypeIdentifierNausea",
                            "HKCategoryTypeIdentifierFatigue",
                            "HKCategoryTypeIdentifierAbdominalCramps",
                            "HKCategoryTypeIdentifierFever",
                            "HKCategoryTypeIdentifierCoughing",
                            "HKCategoryTypeIdentifierShortnessOfBreath",
                            // Fall detection & safety
                            "HKCategoryTypeIdentifierAppleWalkingSteadinessEvent",
                            // Advanced cardiovascular
                            "HKQuantityTypeIdentifierAtrialFibrillationBurden",
                            "HKQuantityTypeIdentifierVO2Max",
                            "HKCategoryTypeIdentifierHighHeartRateEvent",
                            "HKCategoryTypeIdentifierLowHeartRateEvent",
                            "HKCategoryTypeIdentifierIrregularHeartRhythmEvent",
                        ];

                        if critical_missing_identifiers.contains(&ios_metric.name.as_str()) {
                            tracing::error!(
                                "🔥 CRITICAL: Missing mapping for high-priority HealthKit identifier '{}' - This is a known iOS metric type that needs immediate implementation!",
                                ios_metric.name
                            );
                        }

                        // Check if it might be a supported but unmapped metric type
                        if ios_metric.name.starts_with("HKQuantityTypeIdentifier")
                            || ios_metric.name.starts_with("HKCategoryTypeIdentifier")
                        {
                            tracing::error!(
                                "💀 CRITICAL DATA LOSS: Valid HealthKit identifier '{}' has no mapping! This metric will be completely lost.",
                                ios_metric.name
                            );
                        }

                        // Log patterns for future implementation
                        if ios_metric.name.to_lowercase().contains("dietary") {
                            tracing::warn!(
                                "Missing nutrition metric mapping: '{}'",
                                ios_metric.name
                            );
                        } else if ios_metric.name.to_lowercase().contains("symptom")
                            || ios_metric.name.to_lowercase().contains("headache")
                            || ios_metric.name.to_lowercase().contains("nausea")
                        {
                            tracing::warn!("Missing symptom metric mapping: '{}'", ios_metric.name);
                        } else if ios_metric.name.to_lowercase().contains("menstrual")
                            || ios_metric.name.to_lowercase().contains("ovulation")
                        {
                            tracing::warn!(
                                "Missing reproductive health metric mapping: '{}'",
                                ios_metric.name
                            );
                        } else if ios_metric.name.to_lowercase().contains("mindful") {
                            tracing::warn!(
                                "Missing mindfulness metric mapping: '{}'",
                                ios_metric.name
                            );
                        }

                        // Count unmapped metrics for monitoring
                        static UNMAPPED_METRIC_COUNT: std::sync::atomic::AtomicUsize =
                            std::sync::atomic::AtomicUsize::new(0);
                        let count = UNMAPPED_METRIC_COUNT
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        if count > 0 && count % 100 == 0 {
                            tracing::error!(
                                "⚠️ ALERT: {} unmapped iOS metrics encountered in this session - significant data loss occurring!",
                                count
                            );
                        }
                    }
                }
            }
        }

        // Create blood pressure metrics from paired readings
        for (timestamp_str, (systolic, diastolic)) in bp_readings {
            if let (Some(sys), Some(dia)) = (systolic, diastolic) {
                if let Ok(timestamp) = DateTime::parse_from_rfc3339(&timestamp_str) {
                    let metric = crate::models::BloodPressureMetric {
                        id: uuid::Uuid::new_v4(),
                        user_id,
                        recorded_at: timestamp.with_timezone(&Utc),
                        systolic: sys,
                        diastolic: dia,
                        pulse: None, // iOS might not provide pulse separately
                        source_device: Some("Auto Health Export iOS".to_string()),
                        created_at: Utc::now(),
                    };
                    internal_metrics.push(HealthMetric::BloodPressure(metric));
                }
            }
        }

        // Convert workouts (if any)
        for ios_workout in &self.data.workouts {
            let start_time = parse_ios_date(&ios_workout.start).unwrap_or_else(Utc::now);
            let end_time = parse_ios_date(&ios_workout.end).unwrap_or_else(Utc::now);

            // Extract additional workout data from extra fields
            let total_energy_kcal = ios_workout
                .extra
                .get("total_energy_kcal")
                .or_else(|| ios_workout.extra.get("calories"))
                .and_then(|v| v.as_f64());

            let distance_meters = ios_workout
                .extra
                .get("distance_meters")
                .or_else(|| ios_workout.extra.get("distance"))
                .and_then(|v| v.as_f64());

            let avg_heart_rate = ios_workout
                .extra
                .get("avg_heart_rate")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);

            let max_heart_rate = ios_workout
                .extra
                .get("max_heart_rate")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);

            let active_energy_kcal = ios_workout
                .extra
                .get("active_energy_kcal")
                .or_else(|| ios_workout.extra.get("active_calories"))
                .and_then(|v| v.as_f64());

            if start_time < end_time {
                // Basic validation
                let workout_type = WorkoutType::from_ios_string(
                    &ios_workout
                        .name
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string()),
                );

                let workout = WorkoutData {
                    id: uuid::Uuid::new_v4(),
                    user_id,
                    workout_type,
                    started_at: start_time,
                    ended_at: end_time,
                    total_energy_kcal,
                    active_energy_kcal,
                    distance_meters,
                    avg_heart_rate,
                    max_heart_rate,
                    source_device: ios_workout.source.clone(),
                    created_at: Utc::now(),
                };
                internal_workouts.push(workout);
            }
        }

        // Log final conversion summary
        tracing::info!(
            "iOS conversion completed: {} internal metrics created, {} workouts created",
            internal_metrics.len(),
            internal_workouts.len()
        );

        IngestPayload {
            data: IngestData {
                metrics: internal_metrics,
                workouts: internal_workouts,
            },
        }
    }
}

/// Parse iOS date string to UTC DateTime
fn parse_ios_date(date_str: &Option<String>) -> Option<DateTime<Utc>> {
    use chrono::NaiveDateTime;

    let date_str = date_str.as_ref()?;

    // Try formats with timezone first
    let tz_formats = [
        "%Y-%m-%d %H:%M:%S %z", // 2025-09-08 00:48:21 -0500
        "%Y-%m-%dT%H:%M:%S%z",  // 2025-09-08T00:48:21-0500
        "%Y-%m-%dT%H:%M:%SZ",   // 2025-09-08T00:48:21Z
    ];

    for format in &tz_formats {
        if let Ok(dt) = DateTime::parse_from_str(date_str, format) {
            return Some(dt.with_timezone(&Utc));
        }
    }

    // Try formats without timezone (assume UTC)
    let naive_formats = [
        "%Y-%m-%d %H:%M:%S", // 2025-09-08 00:48:21
        "%Y-%m-%d",          // 2025-09-08
    ];

    for format in &naive_formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
        // Try parsing as date only
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, format) {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }
    }

    tracing::warn!("Failed to parse iOS date: {}", date_str);
    None
}
