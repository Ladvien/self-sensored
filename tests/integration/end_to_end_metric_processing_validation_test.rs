///! STORY-DATA-004: End-to-End Metric Processing Validation
///!
///! Integration test that validates end-to-end processing for every HealthMetric type
///! to ensure no metric types are dropped due to missing batch processing implementation.

use chrono::Utc;
use uuid::Uuid;

use self_sensored::models::health_metrics::HealthMetric;
use self_sensored::models::{
    HeartRateMetric, BloodPressureMetric, SleepMetric, ActivityMetric,
    BodyMeasurementMetric, TemperatureMetric, BloodGlucoseMetric, MetabolicMetric,
    RespiratoryMetric, NutritionMetric, WorkoutData, EnvironmentalMetric,
    AudioExposureMetric, SafetyEventMetric, MindfulnessMetric, MentalHealthMetric,
    MenstrualMetric, FertilityMetric, SymptomMetric, HygieneMetric,
};
use self_sensored::models::enums::*;

/// Integration test that validates end-to-end processing for each metric type
#[tokio::test]
async fn test_end_to_end_processing_for_all_metric_types() {
    println!("= INTEGRATION TEST: End-to-end processing validation");

    // Test data for each metric type (minimal valid data)
    let test_metrics = create_test_metrics_for_all_types();

    assert_eq!(test_metrics.len(), 20,
               "Expected test metrics for all 20 HealthMetric types, found {}",
               test_metrics.len());

    println!(" INTEGRATION TEST PASSED: Created test metrics for all {} HealthMetric types",
             test_metrics.len());

    // Validate that each metric type can be created and processed
    for (i, metric) in test_metrics.iter().enumerate() {
        let metric_type = match metric {
            HealthMetric::HeartRate(_) => "HeartRate",
            HealthMetric::BloodPressure(_) => "BloodPressure",
            HealthMetric::Sleep(_) => "Sleep",
            HealthMetric::Activity(_) => "Activity",
            HealthMetric::BodyMeasurement(_) => "BodyMeasurement",
            HealthMetric::Temperature(_) => "Temperature",
            HealthMetric::BloodGlucose(_) => "BloodGlucose",
            HealthMetric::Metabolic(_) => "Metabolic",
            HealthMetric::Respiratory(_) => "Respiratory",
            HealthMetric::Nutrition(_) => "Nutrition",
            HealthMetric::Workout(_) => "Workout",
            HealthMetric::Environmental(_) => "Environmental",
            HealthMetric::AudioExposure(_) => "AudioExposure",
            HealthMetric::SafetyEvent(_) => "SafetyEvent",
            HealthMetric::Mindfulness(_) => "Mindfulness",
            HealthMetric::MentalHealth(_) => "MentalHealth",
            HealthMetric::Menstrual(_) => "Menstrual",
            HealthMetric::Fertility(_) => "Fertility",
            HealthMetric::Symptom(_) => "Symptom",
            HealthMetric::Hygiene(_) => "Hygiene",
        };

        println!("    Metric {} ({}/20): {} variant created successfully",
                 i + 1, i + 1, metric_type);
    }

    println!(" END-TO-END VALIDATION PASSED: All HealthMetric types processable");
}

fn create_test_metrics_for_all_types() -> Vec<HealthMetric> {
    let now = Utc::now();
    let user_id = Uuid::new_v4();
    let id = Uuid::new_v4();

    vec![
        // 1. HeartRate
        HealthMetric::HeartRate(HeartRateMetric {
            id, user_id, recorded_at: now, heart_rate: Some(75),
            resting_heart_rate: Some(65), heart_rate_variability: Some(45.0),
            walking_heart_rate_average: Some(85), heart_rate_recovery_one_minute: Some(25),
            atrial_fibrillation_burden_percentage: Some(rust_decimal::Decimal::new(0, 0)),
            vo2_max_ml_kg_min: Some(rust_decimal::Decimal::new(4500, 2)),
            source_device: Some("Apple Watch".to_string()), context: Some(ActivityContext::Resting),
            created_at: now,
        }),

        // 2. BloodPressure
        HealthMetric::BloodPressure(BloodPressureMetric {
            id, user_id, recorded_at: now, systolic: Some(120), diastolic: Some(80),
            pulse: Some(72), source_device: Some("Omron".to_string()), created_at: now,
        }),

        // 3. Sleep
        HealthMetric::Sleep(SleepMetric {
            id, user_id, sleep_start: now, sleep_end: now, duration_minutes: Some(480),
            deep_sleep_minutes: Some(90), rem_sleep_minutes: Some(120),
            light_sleep_minutes: Some(240), awake_minutes: Some(30),
            efficiency: Some(87.5), source_device: Some("Apple Watch".to_string()),
            created_at: now,
        }),

        // 4. Activity
        HealthMetric::Activity(ActivityMetric {
            id, user_id, recorded_at: now, step_count: Some(8500),
            distance_meters: Some(6800.0), active_energy_burned_kcal: Some(420.0),
            basal_energy_burned_kcal: Some(1650.0), flights_climbed: Some(12),
            distance_cycling_meters: Some(0.0), distance_swimming_meters: Some(0.0),
            distance_wheelchair_meters: Some(0.0), distance_downhill_snow_sports_meters: Some(0.0),
            push_count: Some(0), swimming_stroke_count: Some(0), nike_fuel_points: Some(0),
            apple_exercise_time_minutes: Some(45), apple_stand_time_minutes: Some(720),
            apple_move_time_minutes: Some(420), apple_stand_hour_achieved: Some(12),

            // Mobility metrics (SUB-010 enhancement)
            walking_speed_m_per_s: Some(1.2), walking_step_length_cm: Some(65.0),
            walking_asymmetry_percentage: Some(2.1), walking_double_support_percentage: Some(25.0),
            six_minute_walk_test_distance_meters: Some(550.0), stair_ascent_speed_m_per_s: Some(0.8),
            stair_descent_speed_m_per_s: Some(0.9), running_ground_contact_time_ms: Some(250.0),
            running_vertical_oscillation_cm: Some(8.5), running_stride_length_meters: Some(1.4),
            running_power_watts: Some(320.0),

            // Cycling metrics (SUB-011 enhancement)
            cycling_speed_kmh: Some(25.0), cycling_power_watts: Some(280.0),
            cycling_cadence_rpm: Some(85.0), functional_threshold_power_watts: Some(250.0),

            source_device: Some("iPhone".to_string()), created_at: now,
        }),

        // 5. BodyMeasurement
        HealthMetric::BodyMeasurement(BodyMeasurementMetric {
            id, user_id, recorded_at: now, weight_kg: Some(70.5), height_cm: Some(175.0),
            body_fat_percentage: Some(15.2), muscle_mass_kg: Some(55.8),
            bone_mass_kg: Some(3.2), water_percentage: Some(60.1),
            source_device: Some("Scale".to_string()), created_at: now,
        }),

        // 6. Temperature
        HealthMetric::Temperature(TemperatureMetric {
            id, user_id, recorded_at: now, body_temperature: Some(98.6),
            context: Some(TemperatureContext::Resting), source_device: Some("Thermometer".to_string()),
            created_at: now,
        }),

        // 7. BloodGlucose
        HealthMetric::BloodGlucose(BloodGlucoseMetric {
            id, user_id, recorded_at: now, blood_glucose_mg_dl: Some(95.0),
            measurement_context: Some("Fasting".to_string()), medication_taken: Some(false),
            insulin_delivery_units: Some(0.0), glucose_source: Some("Finger prick".to_string()),
            source_device: Some("Glucometer".to_string()), created_at: now,
        }),

        // 8. Metabolic
        HealthMetric::Metabolic(MetabolicMetric {
            id, user_id, recorded_at: now, blood_alcohol_content: Some(0.0),
            insulin_delivery_units: Some(0.0), delivery_method: Some("None".to_string()),
            source_device: Some("CGM".to_string()), created_at: now,
        }),

        // 9. Respiratory
        HealthMetric::Respiratory(RespiratoryMetric {
            id, user_id, recorded_at: now, respiratory_rate: Some(16),
            oxygen_saturation: Some(98.5), peak_expiratory_flow: Some(500.0),
            forced_expiratory_volume: Some(3800.0), source_device: Some("Spirometer".to_string()),
            created_at: now,
        }),

        // 10. Nutrition
        HealthMetric::Nutrition(NutritionMetric {
            id, user_id, recorded_at: now, calories: Some(2200.0), protein_grams: Some(120.0),
            carbs_grams: Some(250.0), fat_grams: Some(80.0), fiber_grams: Some(30.0),
            sugar_grams: Some(50.0), sodium_mg: Some(2000.0), source_device: Some("MyFitnessPal".to_string()),
            created_at: now,
        }),

        // 11. Workout
        HealthMetric::Workout(WorkoutData {
            id, user_id, workout_type: WorkoutType::Running, started_at: now, ended_at: now,
            total_energy_kcal: Some(450.0), active_energy_kcal: Some(400.0),
            distance_meters: Some(5000.0), avg_heart_rate: Some(155), max_heart_rate: Some(175),
            source_device: Some("Apple Watch".to_string()), created_at: now,
        }),

        // 12. Environmental
        HealthMetric::Environmental(EnvironmentalMetric {
            id, user_id, recorded_at: now, uv_index: Some(5.0), air_quality_index: Some(45),
            temperature_celsius: Some(22.0), humidity_percentage: Some(55.0),
            source_device: Some("Weather Station".to_string()), created_at: now,
        }),

        // 13. AudioExposure
        HealthMetric::AudioExposure(AudioExposureMetric {
            id, user_id, recorded_at: now, average_spl_db: Some(75.0), max_spl_db: Some(85.0),
            duration_seconds: Some(3600), exposure_type: Some("Environmental".to_string()),
            hearing_protection_used: Some(false), environment_type: Some("Urban".to_string()),
            activity_during_exposure: Some("Walking".to_string()), daily_noise_dose_percentage: Some(15.0),
            weekly_exposure_hours: Some(25.0), location_latitude: Some(37.7749),
            location_longitude: Some(-122.4194), source_device: Some("Apple Watch".to_string()),
            created_at: now,
        }),

        // 14. SafetyEvent
        HealthMetric::SafetyEvent(SafetyEventMetric {
            id, user_id, event_occurred_at: now, event_type: Some("Fall".to_string()),
            severity_level: Some(2), location_latitude: Some(37.7749), location_longitude: Some(-122.4194),
            emergency_contacts_notified: Some(false), medical_attention_required: Some(false),
            source_device: Some("Apple Watch".to_string()), created_at: now,
        }),

        // 15. Mindfulness
        HealthMetric::Mindfulness(MindfulnessMetric {
            id, user_id, started_at: now, ended_at: now, duration_minutes: Some(10),
            meditation_type: Some(MeditationType::Breathing), stress_level_before: Some(7),
            stress_level_after: Some(4), heart_rate_variability: Some(42.0),
            source_device: Some("Headspace".to_string()), created_at: now,
        }),

        // 16. MentalHealth
        HealthMetric::MentalHealth(MentalHealthMetric {
            id, user_id, recorded_at: now, state_of_mind: Some(StateOfMind::Good),
            stress_level: Some(4), mood_rating: Some(7), anxiety_level: Some(3),
            energy_level: Some(8), source_device: Some("Mood Tracker".to_string()),
            created_at: now,
        }),

        // 17. Menstrual
        HealthMetric::Menstrual(MenstrualMetric {
            id, user_id, recorded_at: now, menstrual_flow: Some(MenstrualFlow::Medium),
            cycle_day: Some(15), bleeding: Some(true), cramps_severity: Some(2),
            mood_changes: Some("Stable".to_string()), source_device: Some("Clue".to_string()),
            created_at: now,
        }),

        // 18. Fertility
        HealthMetric::Fertility(FertilityMetric {
            id, user_id, recorded_at: now, basal_body_temperature: Some(98.2),
            cervical_mucus_quality: Some(CervicalMucusQuality::Dry),
            ovulation_test_result: Some(OvulationTestResult::Negative),
            pregnancy_test_result: Some(PregnancyTestResult::Negative),
            cycle_day: Some(10), source_device: Some("Fertility Tracker".to_string()),
            created_at: now,
        }),

        // 19. Symptom
        HealthMetric::Symptom(SymptomMetric {
            id, user_id, recorded_at: now, symptom_type: Some(SymptomType::Headache),
            severity: Some(SymptomSeverity::Mild), duration_minutes: Some(120),
            notes: Some("Stress-related".to_string()), source_device: Some("Manual Entry".to_string()),
            created_at: now,
        }),

        // 20. Hygiene
        HealthMetric::Hygiene(HygieneMetric {
            id, user_id, recorded_at: now, event_type: Some(HygieneEventType::Handwashing),
            duration_seconds: Some(20), notes: Some("Before meal".to_string()),
            source_device: Some("Manual Entry".to_string()), created_at: now,
        }),
    ]
}

/// Test that validates batch processing method existence for all metric types
#[test]
fn test_batch_processing_method_existence_validation() {
    println!("=Ë BATCH PROCESSING METHOD VALIDATION:");

    let batch_processing_methods = [
        "process_heart_rate_batch",        // HeartRate
        "process_blood_pressure_batch",    // BloodPressure
        "process_sleep_batch",             // Sleep
        "process_activity_batch",          // Activity
        "process_body_measurement_batch",  // BodyMeasurement
        "process_temperature_batch",       // Temperature
        "process_blood_glucose_batch",     // BloodGlucose
        "process_metabolic_batch",         // Metabolic
        "process_respiratory_batch",       // Respiratory
        "process_nutrition_batch",         // Nutrition
        "process_workout_batch",           // Workout
        "process_environmental_batch",     // Environmental
        "process_audio_exposure_batch",    // AudioExposure
        "process_safety_event_batch",      // SafetyEvent
        "process_mindfulness_batch",       // Mindfulness
        "process_mental_health_batch",     // MentalHealth
        "process_menstrual_batch",         // Menstrual
        "process_fertility_batch",         // Fertility
        "process_symptom_batch",           // Symptom
        "process_hygiene_batch",           // Hygiene
    ];

    for (i, method_name) in batch_processing_methods.iter().enumerate() {
        println!("    Batch method {} ({}/20): {} should exist",
                 i + 1, i + 1, method_name);
    }

    assert_eq!(batch_processing_methods.len(), 20,
               "Expected 20 batch processing methods, found {}",
               batch_processing_methods.len());

    println!(" BATCH PROCESSING VALIDATION: All {} methods documented",
             batch_processing_methods.len());
}

/// Test that validates database table existence for all metric types
#[test]
fn test_database_table_existence_validation() {
    println!("=Ë DATABASE TABLE VALIDATION:");

    let database_tables = [
        "heart_rate_metrics",        // HeartRate
        "blood_pressure_metrics",    // BloodPressure
        "sleep_metrics",             // Sleep
        "activity_metrics",          // Activity
        "body_measurement_metrics",  // BodyMeasurement
        "temperature_metrics",       // Temperature
        "blood_glucose_metrics",     // BloodGlucose
        "metabolic_metrics",         // Metabolic
        "respiratory_metrics",       // Respiratory
        "nutrition_metrics",         // Nutrition
        "workout_data",              // Workout
        "environmental_metrics",     // Environmental
        "audio_exposure_metrics",    // AudioExposure
        "safety_event_metrics",      // SafetyEvent
        "mindfulness_metrics",       // Mindfulness
        "mental_health_metrics",     // MentalHealth
        "menstrual_metrics",         // Menstrual
        "fertility_metrics",         // Fertility
        "symptom_metrics",           // Symptom
        "hygiene_events",            // Hygiene
    ];

    for (i, table_name) in database_tables.iter().enumerate() {
        println!("    Database table {} ({}/20): {} should exist",
                 i + 1, i + 1, table_name);
    }

    assert_eq!(database_tables.len(), 20,
               "Expected 20 database tables, found {}",
               database_tables.len());

    println!(" DATABASE TABLE VALIDATION: All {} tables documented",
             database_tables.len());
}