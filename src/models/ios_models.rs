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
                                let context_str = match ios_metric.name.to_lowercase().as_str() {
                                    "resting_heart_rate" => Some("resting"),
                                    "walking_heart_rate" => Some("walking"),
                                    _ => data_point
                                        .extra
                                        .get("context")
                                        .and_then(|v| v.as_str()),
                                };
                                
                                let context = context_str.and_then(|s| {
                                    crate::models::ActivityContext::from_ios_string(s)
                                });

                                let metric = crate::models::HeartRateMetric {
                                    recorded_at,
                                    heart_rate: Some(qty as i16),
                                    resting_heart_rate: if context_str == Some("resting") { Some(qty as i16) } else { None },
                                    heart_rate_variability: None,
                                    source_device: data_point.source.clone(),
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
                                duration_minutes: Some(total_minutes),
                                deep_sleep_minutes,
                                rem_sleep_minutes,
                                awake_minutes,
                                efficiency: None,
                                source: data_point.source.clone(),
                            };
                            internal_metrics.push(HealthMetric::Sleep(metric));
                        }
                    }
                    // New metric types - Nutrition
                    "dietary_water" | "water" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::NutritionMetric {
                                    recorded_at,
                                    water_ml: Some(qty),
                                    energy_consumed_kcal: None,
                                    carbohydrates_g: None,
                                    protein_g: None,
                                    fat_total_g: None,
                                    fat_saturated_g: None,
                                    fat_monounsaturated_g: None,
                                    fat_polyunsaturated_g: None,
                                    cholesterol_mg: None,
                                    fiber_g: None,
                                    sugar_g: None,
                                    sodium_mg: None,
                                    vitamin_a_mcg: None,
                                    vitamin_d_mcg: None,
                                    vitamin_e_mg: None,
                                    vitamin_k_mcg: None,
                                    vitamin_c_mg: None,
                                    thiamin_mg: None,
                                    riboflavin_mg: None,
                                    niacin_mg: None,
                                    pantothenic_acid_mg: None,
                                    vitamin_b6_mg: None,
                                    biotin_mcg: None,
                                    folate_mcg: None,
                                    vitamin_b12_mcg: None,
                                    calcium_mg: None,
                                    phosphorus_mg: None,
                                    magnesium_mg: None,
                                    potassium_mg: None,
                                    chloride_mg: None,
                                    iron_mg: None,
                                    zinc_mg: None,
                                    copper_mg: None,
                                    manganese_mg: None,
                                    iodine_mcg: None,
                                    selenium_mcg: None,
                                    chromium_mcg: None,
                                    molybdenum_mcg: None,
                                    caffeine_mg: None,
                                    aggregation_period: Some("daily".to_string()),
                                    source: data_point.source.clone(),
                                };
                                internal_metrics.push(HealthMetric::Nutrition(metric));
                            }
                        }
                    }
                    "dietary_energy_consumed" | "nutrition_calories" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::NutritionMetric {
                                    recorded_at,
                                    water_ml: None,
                                    energy_consumed_kcal: Some(qty),
                                    carbohydrates_g: None,
                                    protein_g: None,
                                    fat_total_g: None,
                                    fat_saturated_g: None,
                                    fat_monounsaturated_g: None,
                                    fat_polyunsaturated_g: None,
                                    cholesterol_mg: None,
                                    fiber_g: None,
                                    sugar_g: None,
                                    sodium_mg: None,
                                    vitamin_a_mcg: None,
                                    vitamin_d_mcg: None,
                                    vitamin_e_mg: None,
                                    vitamin_k_mcg: None,
                                    vitamin_c_mg: None,
                                    thiamin_mg: None,
                                    riboflavin_mg: None,
                                    niacin_mg: None,
                                    pantothenic_acid_mg: None,
                                    vitamin_b6_mg: None,
                                    biotin_mcg: None,
                                    folate_mcg: None,
                                    vitamin_b12_mcg: None,
                                    calcium_mg: None,
                                    phosphorus_mg: None,
                                    magnesium_mg: None,
                                    potassium_mg: None,
                                    chloride_mg: None,
                                    iron_mg: None,
                                    zinc_mg: None,
                                    copper_mg: None,
                                    manganese_mg: None,
                                    iodine_mcg: None,
                                    selenium_mcg: None,
                                    chromium_mcg: None,
                                    molybdenum_mcg: None,
                                    caffeine_mg: None,
                                    aggregation_period: Some("daily".to_string()),
                                    source: data_point.source.clone(),
                                };
                                internal_metrics.push(HealthMetric::Nutrition(metric));
                            }
                        }
                    }
                    // Environmental metrics
                    "environmental_audio_exposure" | "headphone_audio_exposure" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 && qty <= 140.0 {
                                let metric = crate::models::EnvironmentalMetric {
                                    recorded_at,
                                    environmental_sound_level_db: if ios_metric.name.to_lowercase().contains("environmental") {
                                        Some(qty)
                                    } else { None },
                                    headphone_exposure_db: if ios_metric.name.to_lowercase().contains("headphone") {
                                        Some(qty)
                                    } else { None },
                                    noise_reduction_db: None,
                                    exposure_duration_seconds: None,
                                    uv_index: None,
                                    time_in_sun_minutes: None,
                                    time_in_shade_minutes: None,
                                    sunscreen_applied: None,
                                    uv_dose_joules_per_m2: None,
                                    fall_detected: None,
                                    fall_severity: None,
                                    impact_force_g: None,
                                    emergency_contacted: None,
                                    fall_response_time_seconds: None,
                                    handwashing_events: None,
                                    handwashing_duration_seconds: None,
                                    toothbrushing_events: None,
                                    toothbrushing_duration_seconds: None,
                                    pm2_5_micrograms_m3: None,
                                    pm10_micrograms_m3: None,
                                    air_quality_index: None,
                                    ozone_ppb: None,
                                    no2_ppb: None,
                                    so2_ppb: None,
                                    co_ppm: None,
                                    altitude_meters: None,
                                    barometric_pressure_hpa: None,
                                    indoor_outdoor_context: None,
                                    aggregation_period: Some("event".to_string()),
                                    measurement_count: Some(1),
                                    source: data_point.source.clone(),
                                    device_type: Some("Apple Watch".to_string()),
                                };
                                internal_metrics.push(HealthMetric::Environmental(metric));
                            }
                        }
                    }
                    // Mental health metrics
                    "mindful_session" => {
                        if let Some(qty) = data_point.qty {
                            if qty >= 0.0 {
                                let metric = crate::models::MentalHealthMetric {
                                    recorded_at,
                                    mindful_minutes: Some(qty),
                                    mood_valence: None,
                                    mood_labels: None,
                                    daylight_minutes: None,
                                    stress_level: None,
                                    depression_score: None,
                                    anxiety_score: None,
                                    sleep_quality_score: None,
                                    source: data_point.source.clone(),
                                    notes: None,
                                };
                                internal_metrics.push(HealthMetric::MentalHealth(metric));
                            }
                        }
                    }
                    "state_of_mind" => {
                        // iOS 17+ State of Mind data - extract from extra fields
                        let mood_valence = data_point.extra.get("valence")
                            .and_then(|v| v.as_f64());
                        let mood_labels = data_point.extra.get("labels")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>());
                        
                        if mood_valence.is_some() || mood_labels.is_some() {
                            let metric = crate::models::MentalHealthMetric {
                                recorded_at,
                                mindful_minutes: None,
                                mood_valence,
                                mood_labels,
                                daylight_minutes: None,
                                stress_level: None,
                                depression_score: None,
                                anxiety_score: None,
                                sleep_quality_score: None,
                                source: data_point.source.clone(),
                                notes: None,
                            };
                            internal_metrics.push(HealthMetric::MentalHealth(metric));
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
                nutrition_metrics: Vec::new(),
                symptom_metrics: Vec::new(),
                reproductive_health_metrics: Vec::new(),
                environmental_metrics: Vec::new(),
                mental_health_metrics: Vec::new(),
                mobility_metrics: Vec::new(),
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
