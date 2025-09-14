use crate::models::enums::WorkoutType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Models that match the iOS Auto Health Export app JSON structure
/// Root payload structure from iOS app
#[derive(Debug, Deserialize, Serialize)]
pub struct IosIngestPayload {
    pub data: IosIngestData,
}

/// Container for iOS health data
#[derive(Debug, Deserialize, Serialize)]
pub struct IosIngestData {
    pub metrics: Vec<IosMetric>,
    // workouts may not be present in iOS app structure
    #[serde(default)]
    pub workouts: Vec<IosWorkout>,
}

/// iOS metric structure with name and data array
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

                // Convert based on metric name (handle more variations including environmental)
                match ios_metric.name.to_lowercase().as_str() {
                    "heart_rate"
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
                                    source_device: data_point.source.clone(),
                                    context,
                                    created_at: Utc::now(),
                                };
                                internal_metrics.push(HealthMetric::HeartRate(metric));
                            }
                        }
                    }
                    "blood_pressure_systolic" | "systolic_blood_pressure" => {
                        if let Some(qty) = data_point.qty {
                            let timestamp_key = recorded_at.to_rfc3339();
                            let entry = bp_readings.entry(timestamp_key).or_default();
                            entry.0 = Some(qty as i16);
                        }
                    }
                    "blood_pressure_diastolic" | "diastolic_blood_pressure" => {
                        if let Some(qty) = data_point.qty {
                            let timestamp_key = recorded_at.to_rfc3339();
                            let entry = bp_readings.entry(timestamp_key).or_default();
                            entry.1 = Some(qty as i16);
                        }
                    }
                    "sleep_analysis" | "sleep" | "sleep_time" => {
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
                    "steps"
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
                                        ios_metric.name.to_lowercase().as_str(),
                                        "steps" | "step_count"
                                    ) {
                                        Some(qty as i32)
                                    } else {
                                        None
                                    },
                                    distance_meters: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "distance_walking_running" | "distance"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    active_energy_burned_kcal: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "active_energy_burned" | "calories"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    basal_energy_burned_kcal: if ios_metric
                                        .name
                                        .to_lowercase()
                                        .as_str()
                                        == "basal_energy_burned"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    flights_climbed: if ios_metric.name.to_lowercase().as_str()
                                        == "flights_climbed"
                                    {
                                        Some(qty as i32)
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
                    // Temperature Metrics - Body, Basal, Apple Watch Wrist, Environmental
                    "body_temperature"
                    | "basal_body_temperature"
                    | "apple_sleeping_wrist_temperature"
                    | "wrist_temperature"
                    | "water_temperature"
                    | "temperature" => {
                        if let Some(qty) = data_point.qty {
                            // Temperature should be in Celsius, validate range
                            if qty >= -50.0 && qty <= 100.0 {
                                let metric = crate::models::TemperatureMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
                                    body_temperature: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "body_temperature" | "temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    basal_body_temperature: if ios_metric
                                        .name
                                        .to_lowercase()
                                        .as_str()
                                        == "basal_body_temperature"
                                    {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    apple_sleeping_wrist_temperature: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "apple_sleeping_wrist_temperature" | "wrist_temperature"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    water_temperature: if ios_metric
                                        .name
                                        .to_lowercase()
                                        .as_str()
                                        == "water_temperature"
                                    {
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
                    // Environmental & Safety Metrics
                    "uv_exposure" | "uv_index" | "environmental_uv_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::EnvironmentalMetric {
                                    id: uuid::Uuid::new_v4(),
                                    user_id,
                                    recorded_at,
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
                                internal_metrics.push(crate::models::HealthMetric::Environmental(metric));
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
                                internal_metrics.push(crate::models::HealthMetric::Environmental(metric));
                            }
                        }
                    }
                    "environmental_audio_exposure" | "environmental_sound_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let duration_minutes = data_point
                                    .extra
                                    .get("duration_minutes")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(1) as i32;

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
                                internal_metrics.push(crate::models::HealthMetric::AudioExposure(metric));
                            }
                        }
                    }
                    "headphone_audio_exposure" | "headphone_sound_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let duration_minutes = data_point
                                    .extra
                                    .get("duration_minutes")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(1) as i32;

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
                                internal_metrics.push(crate::models::HealthMetric::AudioExposure(metric));
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
                                internal_metrics.push(crate::models::HealthMetric::SafetyEvent(metric));
                            }
                        }
                    }
                    _ => {
                        // For unknown metric types, log for debugging and try to extract as activity if numeric
                        tracing::debug!(
                            "Unknown iOS metric type: {} with qty: {:?}",
                            ios_metric.name,
                            data_point.qty
                        );

                        // Check if it might be an environmental metric we don't recognize yet
                        if ios_metric.name.to_lowercase().contains("environmental")
                            || ios_metric.name.to_lowercase().contains("audio")
                            || ios_metric.name.to_lowercase().contains("sound")
                            || ios_metric.name.to_lowercase().contains("uv")
                            || ios_metric.name.to_lowercase().contains("daylight") {
                            tracing::info!(
                                "Potentially environmental metric not yet supported: '{}'",
                                ios_metric.name
                            );
                        }

                        // If it has a numeric quantity, store as generic activity data
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                tracing::info!(
                                    "Converting unknown metric '{}' to generic activity data",
                                    ios_metric.name
                                );
                            }
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
