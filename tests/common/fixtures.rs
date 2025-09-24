// Common test fixtures for all tests
use chrono::{DateTime, Utc};
use self_sensored::config::BatchConfig;
use self_sensored::models::health_metrics::*;
use self_sensored::models::ios_models::*;
use uuid::Uuid;

/// Create a valid ActivityMetric with all fields populated
pub fn create_test_activity_metric() -> ActivityMetric {
    ActivityMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),

        // Basic Activity Metrics
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(10),
        active_energy_burned_kcal: Some(400.0),
        basal_energy_burned_kcal: Some(1600.0),

        // Specialized Distance Metrics
        distance_cycling_meters: Some(5000.0),
        distance_swimming_meters: Some(1000.0),
        distance_wheelchair_meters: Some(0.0),
        distance_downhill_snow_sports_meters: Some(0.0),

        // Wheelchair Accessibility Metrics
        push_count: Some(0),

        // Swimming Analytics
        swimming_stroke_count: Some(500),

        // Cross-Platform Fitness Integration
        nike_fuel_points: Some(2500),

        // Apple Watch Activity Ring Integration
        apple_exercise_time_minutes: Some(30),
        apple_stand_time_minutes: Some(12),
        apple_move_time_minutes: Some(45),
        apple_stand_hour_achieved: Some(true),

        // Mobility Metrics (iOS 14+ HealthKit)
        walking_speed_m_per_s: Some(1.4),
        walking_step_length_cm: Some(75.0),
        walking_asymmetry_percent: Some(2.5),
        walking_double_support_percent: Some(20.0),
        six_minute_walk_test_distance_m: Some(500.0),

        // Stair Metrics
        stair_ascent_speed_m_per_s: Some(0.5),
        stair_descent_speed_m_per_s: Some(0.6),

        // Running Dynamics
        ground_contact_time_ms: Some(250.0),
        vertical_oscillation_cm: Some(8.0),
        running_stride_length_m: Some(1.2),
        running_power_watts: Some(250.0),
        running_speed_m_per_s: Some(3.5),

        // Cycling Metrics (iOS 17+ HealthKit)
        cycling_speed_kmh: Some(25.0),
        cycling_power_watts: Some(200.0),
        cycling_cadence_rpm: Some(85.0),
        functional_threshold_power_watts: Some(250.0),

        // Underwater Metrics (iOS 16+ HealthKit)
        underwater_depth_meters: Some(5.0),
        diving_duration_seconds: Some(60),

        // Metadata
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    }
}

/// Create a minimal ActivityMetric with only required fields
pub fn create_minimal_activity_metric(user_id: Uuid) -> ActivityMetric {
    ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: Utc::now(),

        // All fields are optional, so we can set them to None
        step_count: None,
        distance_meters: None,
        flights_climbed: None,
        active_energy_burned_kcal: None,
        basal_energy_burned_kcal: None,
        distance_cycling_meters: None,
        distance_swimming_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,
        push_count: None,
        swimming_stroke_count: None,
        nike_fuel_points: None,
        apple_exercise_time_minutes: None,
        apple_stand_time_minutes: None,
        apple_move_time_minutes: None,
        apple_stand_hour_achieved: None,
        walking_speed_m_per_s: None,
        walking_step_length_cm: None,
        walking_asymmetry_percent: None,
        walking_double_support_percent: None,
        six_minute_walk_test_distance_m: None,
        stair_ascent_speed_m_per_s: None,
        stair_descent_speed_m_per_s: None,
        ground_contact_time_ms: None,
        vertical_oscillation_cm: None,
        running_stride_length_m: None,
        running_power_watts: None,
        running_speed_m_per_s: None,
        cycling_speed_kmh: None,
        cycling_power_watts: None,
        cycling_cadence_rpm: None,
        functional_threshold_power_watts: None,
        underwater_depth_meters: None,
        diving_duration_seconds: None,
        source_device: None,
        created_at: Utc::now(),
    }
}

/// Create a test IosMetric with all required fields
pub fn create_test_ios_metric() -> IosMetric {
    IosMetric {
        name: "HeartRate".to_string(),
        units: Some("BPM".to_string()),
        data: vec![IosMetricData {
            qty: Some(72.0),
            date: Some(Utc::now().to_rfc3339()),
            start: None,
            end: None,
            source: Some("Apple Watch".to_string()),
            value: None,
            extra: std::collections::HashMap::new(),
        }],
    }
}

/// Create a test IosMetricData point
pub fn create_test_ios_data_point() -> IosMetricData {
    IosMetricData {
        qty: Some(100.0),
        date: Some(Utc::now().to_rfc3339()),
        start: None,
        end: None,
        source: Some("iPhone".to_string()),
        value: None,
        extra: std::collections::HashMap::new(),
    }
}

/// Create a complete BatchConfig for testing
pub fn create_test_batch_config() -> BatchConfig {
    BatchConfig {
        max_retries: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 5000,
        enable_parallel_processing: false,
        chunk_size: 1000,
        memory_limit_mb: 500.0,
        heart_rate_chunk_size: 8000,
        blood_pressure_chunk_size: 8000,
        sleep_chunk_size: 6000,
        activity_chunk_size: 2700,
        body_measurement_chunk_size: 3000,
        temperature_chunk_size: 6500,
        respiratory_chunk_size: 7000,
        workout_chunk_size: 5000,
        blood_glucose_chunk_size: 6500,
        metabolic_chunk_size: 3000,
        nutrition_chunk_size: 1600,
        menstrual_chunk_size: 6500,
        fertility_chunk_size: 4300,
        environmental_chunk_size: 3700,
        audio_exposure_chunk_size: 7000,
        safety_event_chunk_size: 6500,
        mindfulness_chunk_size: 5800,
        mental_health_chunk_size: 5200,
        symptom_chunk_size: 5800,
        hygiene_chunk_size: 6500,
        enable_progress_tracking: true,
        enable_intra_batch_deduplication: true,
        enable_dual_write_activity_metrics: false,
        enable_reproductive_health_encryption: true,
        reproductive_health_audit_logging: true,
        max_concurrent_metric_types: 8,
    }
}

/// Create test HeartRateMetric
pub fn create_test_heart_rate_metric() -> HeartRateMetric {
    HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        heart_rate: Some(72),
        resting_heart_rate: Some(60),
        heart_rate_variability: Some(45.0),
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: Some("Apple Watch".to_string()),
        context: None,
        created_at: Utc::now(),
    }
}

/// Create test BloodPressureMetric
pub fn create_test_blood_pressure_metric() -> BloodPressureMetric {
    BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        systolic: 120,
        diastolic: 80,
        pulse: Some(70),
        source_device: Some("Blood Pressure Monitor".to_string()),
        created_at: Utc::now(),
    }
}

/// Create test SleepMetric
pub fn create_test_sleep_metric() -> SleepMetric {
    SleepMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        sleep_start: Utc::now() - chrono::Duration::hours(8),
        sleep_end: Utc::now(),
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(240),
        awake_minutes: Some(20),
        efficiency: Some(95.0),
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    }
}

/// Create test WorkoutData
pub fn create_test_workout_metric() -> WorkoutData {
    use self_sensored::models::enums::WorkoutType;

    WorkoutData {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        workout_type: WorkoutType::Running,
        started_at: Utc::now() - chrono::Duration::hours(1),
        ended_at: Utc::now(),
        total_energy_kcal: Some(500.0),
        active_energy_kcal: Some(450.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: Some(145),
        max_heart_rate: Some(175),
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    }
}
