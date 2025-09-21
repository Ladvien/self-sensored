use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde_json;
use uuid::Uuid;

use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{
    ActivityContext, CardiacEventSeverity, CervicalMucusQuality, HeartRateEventType,
    HygieneEventType, MeditationType, MenstrualFlow, OvulationTestResult, PregnancyTestResult,
    StateOfMind, SymptomSeverity, SymptomType, TemperatureContext, WorkoutType,
};
use self_sensored::models::health_metrics::{
    ActivityMetric, BloodGlucoseMetric, BloodPressureMetric, BodyMeasurementMetric,
    FertilityMetric, HeartRateEvent, HeartRateMetric, HygieneMetric, MenstrualHealthMetric,
    MentalHealthMetric, MetabolicMetric, MindfulnessMetric, NutritionMetric, RespiratoryMetric,
    SleepMetric, SymptomMetric, TemperatureMetric, WorkoutMetric,
};
use self_sensored::models::user_characteristics::UserCharacteristics;

fn create_test_user_id() -> Uuid {
    Uuid::new_v4()
}

fn create_test_timestamp() -> DateTime<Utc> {
    Utc::now()
}

#[test]
fn test_heart_rate_metric_creation() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(75),
        resting_heart_rate: Some(60),
        heart_rate_variability: Some(45.5),
        walking_heart_rate_average: Some(95),
        heart_rate_recovery_one_minute: Some(20),
        atrial_fibrillation_burden_percentage: Some(Decimal::new(150, 2)), // 1.50%
        vo2_max_ml_kg_min: Some(Decimal::new(4250, 2)), // 42.50
        source_device: Some("Apple Watch Series 8".to_string()),
        context: Some(ActivityContext::Resting),
        created_at: now,
    };

    assert_eq!(metric.heart_rate, Some(75));
    assert_eq!(metric.resting_heart_rate, Some(60));
    assert_eq!(metric.heart_rate_variability, Some(45.5));
    assert_eq!(metric.walking_heart_rate_average, Some(95));
}

#[test]
fn test_heart_rate_metric_validation() {
    let config = ValidationConfig::default();
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    // Test valid heart rate
    let valid_metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(70),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: now,
    };

    // Should validate successfully
    let validation_result = valid_metric.validate_with_config(&config);
    assert!(validation_result.is_ok());

    // Test invalid heart rate (too low)
    let invalid_low = HeartRateMetric {
        heart_rate: Some(10),
        ..valid_metric.clone()
    };
    assert!(invalid_low.validate_with_config(&config).is_err());

    // Test invalid heart rate (too high)
    let invalid_high = HeartRateMetric {
        heart_rate: Some(350),
        ..valid_metric.clone()
    };
    assert!(invalid_high.validate_with_config(&config).is_err());
}

#[test]
fn test_heart_rate_metric_serialization() {
    let metric = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id: create_test_user_id(),
        recorded_at: create_test_timestamp(),
        heart_rate: Some(72),
        resting_heart_rate: Some(58),
        heart_rate_variability: Some(50.0),
        walking_heart_rate_average: Some(100),
        heart_rate_recovery_one_minute: Some(25),
        atrial_fibrillation_burden_percentage: Some(Decimal::new(0, 2)),
        vo2_max_ml_kg_min: Some(Decimal::new(4500, 2)),
        source_device: Some("Garmin".to_string()),
        context: Some(ActivityContext::Exercise),
        created_at: create_test_timestamp(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&metric).unwrap();
    assert!(json.contains("\"heart_rate\":72"));
    assert!(json.contains("\"resting_heart_rate\":58"));

    // Deserialize back
    let deserialized: HeartRateMetric = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.heart_rate, metric.heart_rate);
    assert_eq!(deserialized.resting_heart_rate, metric.resting_heart_rate);
}

#[test]
fn test_heart_rate_event_creation() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let event = HeartRateEvent {
        id: Uuid::new_v4(),
        user_id,
        event_type: HeartRateEventType::HighHeartRate,
        event_occurred_at: now,
        heart_rate_at_event: 185,
        event_duration_minutes: Some(5),
        context: Some(ActivityContext::Exercise),
        source_device: Some("Apple Watch".to_string()),
        severity: CardiacEventSeverity::Low,
        is_confirmed: false,
        notes: Some("During running workout".to_string()),
        created_at: now,
    };

    assert_eq!(event.event_type, HeartRateEventType::HighHeartRate);
    assert_eq!(event.heart_rate_at_event, 185);
    assert_eq!(event.severity, CardiacEventSeverity::Low);
    assert!(!event.is_confirmed);
}

#[test]
fn test_blood_pressure_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = BloodPressureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        systolic: 120,
        diastolic: 80,
        pulse: Some(72),
        source_device: Some("Omron BP Monitor".to_string()),
        created_at: now,
    };

    assert_eq!(metric.systolic, 120);
    assert_eq!(metric.diastolic, 80);
    assert_eq!(metric.pulse, Some(72));

    // Validate
    let config = ValidationConfig::default();
    assert!(metric.validate_with_config(&config).is_ok());

    // Test invalid values
    let invalid_metric = BloodPressureMetric {
        systolic: 40, // Too low
        diastolic: 200, // Too high
        ..metric
    };
    assert!(invalid_metric.validate_with_config(&config).is_err());
}

#[test]
fn test_sleep_metric() {
    let user_id = create_test_user_id();
    let sleep_start = Utc::now() - Duration::hours(8);
    let sleep_end = Utc::now();

    let metric = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start,
        sleep_end,
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(220),
        awake_minutes: Some(40),
        efficiency: Some(91.7),
        source_device: Some("Oura Ring".to_string()),
        created_at: Utc::now(),
    };

    assert_eq!(metric.duration_minutes, Some(480));
    assert_eq!(metric.deep_sleep_minutes, Some(120));
    assert_eq!(metric.efficiency, Some(91.7));

    // Validate sleep efficiency
    let config = ValidationConfig::default();
    assert!(metric.validate_with_config(&config).is_ok());

    // Test invalid efficiency
    let invalid_metric = SleepMetric {
        efficiency: Some(150.0), // Over 100%
        ..metric
    };
    assert!(invalid_metric.validate_with_config(&config).is_err());
}

#[test]
fn test_activity_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = ActivityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        step_count: Some(10000),
        distance_meters: Some(8000.0),
        flights_climbed: Some(10),
        active_energy_burned_kcal: Some(450.0),
        basal_energy_burned_kcal: Some(1800.0),
        distance_cycling_meters: Some(5000.0),
        distance_swimming_meters: None,
        distance_wheelchair_meters: None,
        distance_downhill_snow_sports_meters: None,
        push_count: None,
        swimming_stroke_count: None,
        nike_fuel_points: Some(3000),
        apple_exercise_time_minutes: Some(45),
        apple_stand_time_minutes: Some(12),
        apple_move_time_minutes: Some(60),
        apple_stand_hour_achieved: Some(true),
        source_device: Some("iPhone 14 Pro".to_string()),
        created_at: now,
    };

    assert_eq!(metric.step_count, Some(10000));
    assert_eq!(metric.distance_meters, Some(8000.0));
    assert_eq!(metric.apple_stand_hour_achieved, Some(true));

    // Validation
    let config = ValidationConfig::default();
    assert!(metric.validate_with_config(&config).is_ok());

    // Test extreme values
    let extreme_metric = ActivityMetric {
        step_count: Some(250000), // Very high but possible for ultra-marathoners
        distance_meters: Some(600000.0), // 600km - beyond daily max
        ..metric
    };
    assert!(extreme_metric.validate_with_config(&config).is_err());
}

#[test]
fn test_workout_metric() {
    let user_id = create_test_user_id();
    let start = Utc::now() - Duration::hours(1);
    let end = Utc::now();

    let metric = WorkoutMetric {
        id: Uuid::new_v4(),
        user_id,
        workout_type: WorkoutType::Running,
        started_at: start,
        ended_at: end,
        duration_minutes: Some(60),
        total_energy_kcal: Some(600.0),
        active_energy_kcal: Some(550.0),
        distance_meters: Some(10000.0),
        elevation_gain_meters: Some(100.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        avg_pace_seconds_per_meter: Some(0.36), // 6 min/km pace
        avg_speed_meters_per_second: Some(2.78), // 10 km/h
        source_device: Some("Apple Watch Ultra".to_string()),
        weather_temperature_celsius: Some(20.0),
        weather_humidity_percentage: Some(65),
        notes: Some("Morning run in the park".to_string()),
        created_at: end,
    };

    assert_eq!(metric.workout_type, WorkoutType::Running);
    assert_eq!(metric.duration_minutes, Some(60));
    assert_eq!(metric.distance_meters, Some(10000.0));

    // Test serialization
    let json = serde_json::to_string(&metric).unwrap();
    assert!(json.contains("\"workout_type\":\"Running\""));
}

#[test]
fn test_body_measurement_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = BodyMeasurementMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        weight_kg: Some(75.5),
        height_cm: Some(180.0),
        body_fat_percentage: Some(18.5),
        muscle_mass_kg: Some(58.2),
        bone_mass_kg: Some(3.2),
        water_percentage: Some(60.5),
        visceral_fat_rating: Some(8),
        metabolic_age_years: Some(28),
        bmi: Some(23.3),
        lean_body_mass_kg: Some(61.4),
        measurement_source: Some("smart_scale".to_string()),
        source_device: Some("Withings Body+".to_string()),
        created_at: now,
    };

    assert_eq!(metric.weight_kg, Some(75.5));
    assert_eq!(metric.body_fat_percentage, Some(18.5));
    assert_eq!(metric.bmi, Some(23.3));
}

#[test]
fn test_temperature_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = TemperatureMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        temperature_celsius: 36.8,
        temperature_context: Some(TemperatureContext::Waking),
        measurement_location: Some("oral".to_string()),
        temperature_source: Some("thermometer".to_string()),
        source_device: Some("Kinsa Smart Thermometer".to_string()),
        created_at: now,
    };

    assert_eq!(metric.temperature_celsius, 36.8);
    assert_eq!(metric.temperature_context, Some(TemperatureContext::Waking));

    // Validate
    let config = ValidationConfig::default();
    assert!(metric.validate_with_config(&config).is_ok());

    // Test fever temperature
    let fever_metric = TemperatureMetric {
        temperature_celsius: 38.5,
        ..metric
    };
    assert!(fever_metric.validate_with_config(&config).is_ok());

    // Test invalid temperature
    let invalid_metric = TemperatureMetric {
        temperature_celsius: 50.0, // Way too high
        ..metric
    };
    assert!(invalid_metric.validate_with_config(&config).is_err());
}

#[test]
fn test_blood_glucose_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = BloodGlucoseMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        glucose_mg_dl: Some(95.0),
        glucose_mmol_l: Some(5.3),
        measurement_context: Some("fasting".to_string()),
        meal_context: None,
        insulin_units: None,
        glucose_source: Some("cgm".to_string()),
        source_device: Some("Dexcom G6".to_string()),
        created_at: now,
    };

    assert_eq!(metric.glucose_mg_dl, Some(95.0));
    assert_eq!(metric.glucose_mmol_l, Some(5.3));
    assert_eq!(metric.glucose_source, Some("cgm".to_string()));
}

#[test]
fn test_respiratory_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = RespiratoryMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        respiratory_rate: Some(16),
        blood_oxygen_percentage: Some(98.5),
        forced_expiratory_volume_liters: None,
        peak_expiratory_flow_lpm: None,
        source_device: Some("Apple Watch Series 8".to_string()),
        created_at: now,
    };

    assert_eq!(metric.respiratory_rate, Some(16));
    assert_eq!(metric.blood_oxygen_percentage, Some(98.5));
}

#[test]
fn test_nutrition_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = NutritionMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        meal_type: Some("lunch".to_string()),
        calories_kcal: Some(650.0),
        protein_g: Some(35.0),
        carbohydrates_g: Some(75.0),
        fat_g: Some(25.0),
        fiber_g: Some(8.0),
        sugar_g: Some(12.0),
        sodium_mg: Some(800.0),
        cholesterol_mg: Some(120.0),
        water_ml: Some(500.0),
        caffeine_mg: Some(95.0),
        alcohol_g: None,
        source: Some("MyFitnessPal".to_string()),
        notes: Some("Grilled chicken salad with quinoa".to_string()),
        created_at: now,
    };

    assert_eq!(metric.calories_kcal, Some(650.0));
    assert_eq!(metric.protein_g, Some(35.0));
    assert_eq!(metric.meal_type, Some("lunch".to_string()));
}

#[test]
fn test_metabolic_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = MetabolicMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        basal_metabolic_rate_kcal: Some(1750.0),
        resting_energy_kcal: Some(1850.0),
        metabolic_efficiency: Some(0.92),
        ketone_mmol_l: Some(0.5),
        lactate_mmol_l: None,
        source_device: Some("InBody 570".to_string()),
        created_at: now,
    };

    assert_eq!(metric.basal_metabolic_rate_kcal, Some(1750.0));
    assert_eq!(metric.metabolic_efficiency, Some(0.92));
}

#[test]
fn test_mindfulness_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = MindfulnessMetric {
        id: Uuid::new_v4(),
        user_id,
        session_start: now - Duration::minutes(20),
        session_end: now,
        duration_minutes: 20,
        meditation_type: Some(MeditationType::Breathing),
        avg_heart_rate: Some(65),
        min_heart_rate: Some(58),
        max_heart_rate: Some(72),
        heart_rate_variability: Some(55.0),
        stress_level_before: Some(7),
        stress_level_after: Some(3),
        notes: Some("Morning meditation session".to_string()),
        source_device: Some("Headspace App".to_string()),
        created_at: now,
    };

    assert_eq!(metric.duration_minutes, 20);
    assert_eq!(metric.meditation_type, Some(MeditationType::Breathing));
    assert_eq!(metric.stress_level_before, Some(7));
    assert_eq!(metric.stress_level_after, Some(3));
}

#[test]
fn test_mental_health_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = MentalHealthMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        mood_score: Some(7),
        anxiety_level: Some(3),
        stress_level: Some(4),
        depression_score: None,
        energy_level: Some(8),
        social_interaction_minutes: Some(120),
        state_of_mind: Some(StateOfMind::Good),
        therapy_minutes: None,
        medication_taken: Some(false),
        notes: Some("Feeling productive today".to_string()),
        source: Some("Daylio".to_string()),
        created_at: now,
    };

    assert_eq!(metric.mood_score, Some(7));
    assert_eq!(metric.state_of_mind, Some(StateOfMind::Good));
    assert_eq!(metric.medication_taken, Some(false));
}

#[test]
fn test_symptom_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = SymptomMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        symptom_type: SymptomType::Headache,
        severity: SymptomSeverity::Mild,
        duration_minutes: Some(30),
        body_location: Some("frontal".to_string()),
        associated_activity: None,
        medication_taken: Some("ibuprofen 400mg".to_string()),
        notes: Some("Tension headache after long meeting".to_string()),
        source: Some("Manual Entry".to_string()),
        created_at: now,
    };

    assert_eq!(metric.symptom_type, SymptomType::Headache);
    assert_eq!(metric.severity, SymptomSeverity::Mild);
    assert_eq!(metric.duration_minutes, Some(30));
}

#[test]
fn test_hygiene_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = HygieneMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        event_type: HygieneEventType::HandWashing,
        duration_seconds: Some(30),
        quality_score: Some(9),
        products_used: Some(vec!["soap".to_string(), "hand sanitizer".to_string()]),
        notes: None,
        source_device: Some("Apple Watch".to_string()),
        created_at: now,
    };

    assert_eq!(metric.event_type, HygieneEventType::HandWashing);
    assert_eq!(metric.duration_seconds, Some(30));
    assert_eq!(metric.quality_score, Some(9));
}

#[test]
fn test_menstrual_health_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = MenstrualHealthMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        cycle_day: Some(14),
        flow: Some(MenstrualFlow::Medium),
        symptoms: Some(vec!["cramps".to_string(), "fatigue".to_string()]),
        basal_body_temperature_celsius: Some(36.7),
        cervical_mucus_quality: Some(CervicalMucusQuality::EggWhite),
        ovulation_test_result: Some(OvulationTestResult::Positive),
        sexual_activity: Some(false),
        mood: Some(StateOfMind::Good),
        notes: Some("Mid-cycle, feeling energetic".to_string()),
        source: Some("Clue App".to_string()),
        created_at: now,
    };

    assert_eq!(metric.cycle_day, Some(14));
    assert_eq!(metric.flow, Some(MenstrualFlow::Medium));
    assert_eq!(metric.ovulation_test_result, Some(OvulationTestResult::Positive));
}

#[test]
fn test_fertility_metric() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = FertilityMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        pregnancy_test_result: Some(PregnancyTestResult::Negative),
        luteinizing_hormone_level: Some(25.5),
        follicle_stimulating_hormone_level: None,
        estrogen_level: None,
        progesterone_level: None,
        fertility_window_start: Some(now - Duration::days(2)),
        fertility_window_end: Some(now + Duration::days(2)),
        conception_attempt: Some(false),
        notes: None,
        source: Some("Clearblue Monitor".to_string()),
        created_at: now,
    };

    assert_eq!(metric.pregnancy_test_result, Some(PregnancyTestResult::Negative));
    assert_eq!(metric.luteinizing_hormone_level, Some(25.5));
    assert_eq!(metric.conception_attempt, Some(false));
}

#[test]
fn test_metric_clone_implementations() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let heart_rate = HeartRateMetric {
        id: Uuid::new_v4(),
        user_id,
        recorded_at: now,
        heart_rate: Some(72),
        resting_heart_rate: None,
        heart_rate_variability: None,
        walking_heart_rate_average: None,
        heart_rate_recovery_one_minute: None,
        atrial_fibrillation_burden_percentage: None,
        vo2_max_ml_kg_min: None,
        source_device: None,
        context: None,
        created_at: now,
    };

    let cloned = heart_rate.clone();
    assert_eq!(cloned.user_id, heart_rate.user_id);
    assert_eq!(cloned.heart_rate, heart_rate.heart_rate);
}

#[test]
fn test_metric_debug_implementations() {
    let user_id = create_test_user_id();
    let now = create_test_timestamp();

    let metric = SleepMetric {
        id: Uuid::new_v4(),
        user_id,
        sleep_start: now - Duration::hours(8),
        sleep_end: now,
        duration_minutes: Some(480),
        deep_sleep_minutes: Some(120),
        rem_sleep_minutes: Some(100),
        light_sleep_minutes: Some(220),
        awake_minutes: Some(40),
        efficiency: Some(91.7),
        source_device: Some("Oura Ring".to_string()),
        created_at: now,
    };

    let debug_str = format!("{:?}", metric);
    assert!(debug_str.contains("SleepMetric"));
    assert!(debug_str.contains("duration_minutes"));
    assert!(debug_str.contains("480"));
}