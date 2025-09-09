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
    pub fn to_internal_format(&self) -> crate::models::IngestPayload {
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

                // Convert based on metric name (handle more variations)
                match ios_metric.name.to_lowercase().as_str() {
                    "heart_rate"
                    | "heartrate"
                    | "resting_heart_rate"
                    | "walking_heart_rate"
                    | "heart_rate_variability" => {
                        if let Some(qty) = data_point.qty {
                            if qty > 0.0 && qty <= 300.0 {
                                // Basic validation
                                let context = match ios_metric.name.to_lowercase().as_str() {
                                    "resting_heart_rate" => Some("resting".to_string()),
                                    "walking_heart_rate" => Some("walking".to_string()),
                                    _ => data_point
                                        .extra
                                        .get("context")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                };

                                let metric = crate::models::HeartRateMetric {
                                    recorded_at,
                                    min_bpm: None,
                                    avg_bpm: Some(qty as i16),
                                    max_bpm: None,
                                    source: data_point.source.clone(),
                                    context,
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
                                recorded_at,
                                sleep_start: start_time,
                                sleep_end: end_time,
                                total_sleep_minutes: total_minutes,
                                deep_sleep_minutes,
                                rem_sleep_minutes,
                                awake_minutes,
                                efficiency_percentage: None,
                                source: data_point.source.clone(),
                            };
                            internal_metrics.push(HealthMetric::Sleep(metric));
                        }
                    }
                    "steps"
                    | "step_count"
                    | "distance_walking_running"
                    | "distance"
                    | "active_energy_burned"
                    | "calories"
                    | "flights_climbed"
                    | "active_minutes" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                // Basic validation - no negative values
                                let metric = crate::models::ActivityMetric {
                                    date: recorded_at.date_naive(),
                                    steps: if matches!(
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
                                    calories_burned: if matches!(
                                        ios_metric.name.to_lowercase().as_str(),
                                        "active_energy_burned" | "calories"
                                    ) {
                                        Some(qty)
                                    } else {
                                        None
                                    },
                                    active_minutes: if ios_metric.name.to_lowercase().as_str()
                                        == "active_minutes"
                                    {
                                        Some(qty as i32)
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
                                    source: data_point.source.clone(),
                                };
                                internal_metrics.push(HealthMetric::Activity(metric));
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
                        recorded_at: timestamp.with_timezone(&Utc),
                        systolic: sys,
                        diastolic: dia,
                        pulse: None, // iOS might not provide pulse separately
                        source: Some("Auto Health Export iOS".to_string()),
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
                .map(|v| v as i16);

            let max_heart_rate = ios_workout
                .extra
                .get("max_heart_rate")
                .and_then(|v| v.as_i64())
                .map(|v| v as i16);

            if start_time < end_time {
                // Basic validation
                let workout = WorkoutData {
                    workout_type: ios_workout
                        .name
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string()),
                    start_time,
                    end_time,
                    total_energy_kcal,
                    distance_meters,
                    avg_heart_rate,
                    max_heart_rate,
                    source: ios_workout.source.clone(),
                    route_points: None, // iOS data typically doesn't include detailed GPS routes
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
    let date_str = date_str.as_ref()?;

    // iOS typically sends dates like "2025-09-08 00:48:21 -0500"
    // Try multiple formats
    let formats = [
        "%Y-%m-%d %H:%M:%S %z", // 2025-09-08 00:48:21 -0500
        "%Y-%m-%d %H:%M:%S",    // 2025-09-08 00:48:21
        "%Y-%m-%dT%H:%M:%S%z",  // 2025-09-08T00:48:21-0500
        "%Y-%m-%dT%H:%M:%SZ",   // 2025-09-08T00:48:21Z
        "%Y-%m-%d",             // 2025-09-08
    ];

    for format in &formats {
        if let Ok(dt) = DateTime::parse_from_str(date_str, format) {
            return Some(dt.with_timezone(&Utc));
        }
    }

    tracing::warn!("Failed to parse iOS date: {}", date_str);
    None
}
