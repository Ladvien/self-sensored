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

        let mut internal_metrics = Vec::new();
        let mut internal_workouts = Vec::new();

        // Convert iOS metrics to internal format
        for ios_metric in &self.data.metrics {
            for data_point in &ios_metric.data {
                // Parse dates
                let recorded_at = parse_ios_date(&data_point.date).unwrap_or_else(Utc::now);

                // Convert based on metric name
                match ios_metric.name.as_str() {
                    "heart_rate" | "resting_heart_rate" | "walking_heart_rate" => {
                        if let Some(qty) = data_point.qty {
                            let metric = crate::models::HeartRateMetric {
                                recorded_at,
                                min_bpm: None,
                                avg_bpm: Some(qty as i16),
                                max_bpm: None,
                                source: data_point.source.clone(),
                                context: None,
                            };
                            internal_metrics.push(HealthMetric::HeartRate(metric));
                        }
                    }
                    "blood_pressure_systolic" | "blood_pressure_diastolic" => {
                        // Handle blood pressure (would need both systolic and diastolic)
                        // For now, skip as we need both values
                    }
                    "sleep_analysis" => {
                        // Handle sleep data
                        let start_time = parse_ios_date(&data_point.start).unwrap_or(recorded_at);
                        let end_time = parse_ios_date(&data_point.end).unwrap_or(recorded_at);

                        let total_minutes = (end_time - start_time).num_minutes() as i32;

                        if total_minutes > 0 {
                            let metric = crate::models::SleepMetric {
                                recorded_at,
                                sleep_start: start_time,
                                sleep_end: end_time,
                                total_sleep_minutes: total_minutes,
                                deep_sleep_minutes: None,
                                rem_sleep_minutes: None,
                                awake_minutes: None,
                                efficiency_percentage: None,
                                source: data_point.source.clone(),
                            };
                            internal_metrics.push(HealthMetric::Sleep(metric));
                        }
                    }
                    "steps" | "distance_walking_running" | "active_energy_burned" => {
                        // Handle activity data
                        let metric = crate::models::ActivityMetric {
                            date: recorded_at.date_naive(),
                            steps: if ios_metric.name == "steps" {
                                data_point.qty.map(|q| q as i32)
                            } else {
                                None
                            },
                            distance_meters: if ios_metric.name == "distance_walking_running" {
                                data_point.qty
                            } else {
                                None
                            },
                            calories_burned: if ios_metric.name == "active_energy_burned" {
                                data_point.qty
                            } else {
                                None
                            },
                            active_minutes: None,
                            flights_climbed: None,
                            source: data_point.source.clone(),
                        };
                        internal_metrics.push(HealthMetric::Activity(metric));
                    }
                    _ => {
                        // For unknown metric types, create a generic activity record
                        // or log for debugging
                        tracing::debug!("Unknown metric type: {}", ios_metric.name);
                    }
                }
            }
        }

        // Convert workouts (if any)
        for ios_workout in &self.data.workouts {
            let start_time = parse_ios_date(&ios_workout.start).unwrap_or_else(Utc::now);
            let end_time = parse_ios_date(&ios_workout.end).unwrap_or_else(Utc::now);

            let workout = WorkoutData {
                workout_type: ios_workout
                    .name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string()),
                start_time,
                end_time,
                total_energy_kcal: None,
                distance_meters: None,
                avg_heart_rate: None,
                max_heart_rate: None,
                source: ios_workout.source.clone(),
            };
            internal_workouts.push(workout);
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
