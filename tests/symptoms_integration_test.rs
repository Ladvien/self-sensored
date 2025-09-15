use chrono::Utc;
use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{SymptomSeverity, SymptomType};
use self_sensored::models::SymptomMetric;
use uuid::Uuid;

/// Symptoms tracking integration tests
/// Tests comprehensive symptom tracking functionality including:
/// - 50+ symptom types with medical validation
/// - Severity level assessment (none to critical)
/// - Episode-based illness tracking
/// - Emergency symptom detection
/// - Medical recommendation generation
/// - Duration tracking and analysis
/// - iOS HealthKit symptom parsing

#[cfg(test)]
mod symptom_tests {
    use super::*;

    /// Test symptom metric validation with medical assessment
    #[tokio::test]
    async fn test_symptom_validation_basic() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test basic symptom with mild severity
        let mild_headache = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Headache,
            severity: SymptomSeverity::Mild,
            duration_minutes: Some(120), // 2 hours
            notes: Some("Started after working on computer".to_string()),
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(mild_headache.validate_with_config(&config).is_ok());
        assert_eq!(mild_headache.get_category(), "pain");
        assert!(!mild_headache.is_medical_emergency());
        assert!(!mild_headache.requires_medical_attention());
    }

    /// Test emergency symptom detection for critical conditions
    #[tokio::test]
    async fn test_emergency_symptom_detection() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test critical chest pain (medical emergency)
        let chest_pain = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::ChestTightnessOrPain,
            severity: SymptomSeverity::Severe,
            duration_minutes: Some(30),
            notes: Some("Severe crushing chest pain with radiation to left arm".to_string()),
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(chest_pain.is_medical_emergency());
        assert!(chest_pain.requires_medical_attention());
        assert_eq!(chest_pain.get_urgency_level(), 5); // Maximum urgency
        assert_eq!(chest_pain.get_category(), "cardiovascular");

        let recommendations = chest_pain.generate_recommendations();
        assert!(recommendations
            .iter()
            .any(|r| r.contains("immediate medical attention")));
        assert!(recommendations
            .iter()
            .any(|r| r.contains("emergency services")));
    }

    /// Test respiratory symptom assessment
    #[tokio::test]
    async fn test_respiratory_symptoms() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test severe shortness of breath (emergency condition)
        let dyspnea = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::ShortnessOfBreath,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(45),
            notes: Some("Difficulty breathing at rest, getting worse".to_string()),
            episode_id: Some(Uuid::new_v4()),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        };

        assert!(dyspnea.is_medical_emergency()); // Moderate dyspnea is emergency
        assert_eq!(dyspnea.get_category(), "respiratory");
        assert!(dyspnea.is_episode_symptom());

        let analysis = dyspnea.generate_analysis();
        assert_eq!(analysis.symptom_type, SymptomType::ShortnessOfBreath);
        assert_eq!(analysis.severity, SymptomSeverity::Moderate);
        assert!(analysis.is_emergency);
        assert_eq!(analysis.category, "respiratory");
    }

    /// Test digestive symptom tracking
    #[tokio::test]
    async fn test_digestive_symptoms() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();
        let episode_id = Uuid::new_v4();

        // Test persistent nausea (requires medical attention after 48 hours)
        let nausea = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Nausea,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(2880), // 48 hours
            notes: Some("Persistent nausea, unable to keep food down".to_string()),
            episode_id: Some(episode_id),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(!nausea.is_medical_emergency()); // Not emergency, but requires attention
        assert!(nausea.requires_medical_attention()); // Due to 48+ hour duration
        assert_eq!(nausea.get_category(), "digestive");

        let recommendations = nausea.generate_recommendations();
        assert!(recommendations.iter().any(|r| r.contains("Stay hydrated")));
        assert!(recommendations.iter().any(|r| r.contains("bland diet")));
    }

    /// Test neurological symptom patterns
    #[tokio::test]
    async fn test_neurological_symptoms() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test severe fatigue with cognitive symptoms
        let fatigue = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Fatigue,
            severity: SymptomSeverity::Severe,
            duration_minutes: Some(10080), // 1 week
            notes: Some("Overwhelming exhaustion, brain fog, difficulty concentrating".to_string()),
            episode_id: Some(Uuid::new_v4()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(fatigue.requires_medical_attention()); // Severe fatigue requires attention
        assert_eq!(fatigue.get_category(), "neurological");

        let recommendations = fatigue.generate_recommendations();
        assert!(recommendations.iter().any(|r| r.contains("adequate rest")));
        assert!(recommendations
            .iter()
            .any(|r| r.contains("cognitive symptoms")));
        assert!(recommendations
            .iter()
            .any(|r| r.contains("medical evaluation"))); // Due to 1 week duration
    }

    /// Test pain symptom severity tracking
    #[tokio::test]
    async fn test_pain_symptoms() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test severe back pain requiring medical attention
        let back_pain = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::BackPain,
            severity: SymptomSeverity::Severe,
            duration_minutes: Some(1440), // 24 hours
            notes: Some("Severe lower back pain, radiating down leg".to_string()),
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(!back_pain.is_medical_emergency());
        assert!(back_pain.requires_medical_attention()); // Severe pain requires attention
        assert_eq!(back_pain.get_category(), "pain");

        let recommendations = back_pain.generate_recommendations();
        assert!(recommendations
            .iter()
            .any(|r| r.contains("Rest affected area")));
        assert!(recommendations
            .iter()
            .any(|r| r.contains("pain management")));
    }

    /// Test reproductive health symptoms
    #[tokio::test]
    async fn test_reproductive_symptoms() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test severe menstrual cramps
        let cramps = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::AbdominalCramps,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(480), // 8 hours
            notes: Some("Severe menstrual cramps, affecting daily activities".to_string()),
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert_eq!(cramps.get_category(), "pain");
        assert!(!cramps.is_medical_emergency());
        assert!(!cramps.requires_medical_attention()); // 8 hours not long enough for cramps

        let recommendations = cramps.generate_recommendations();
        assert!(recommendations
            .iter()
            .any(|r| r.contains("Rest affected area")));
    }

    /// Test episode-based symptom grouping
    #[tokio::test]
    async fn test_symptom_episode_tracking() {
        let user_id = Uuid::new_v4();
        let episode_id = Uuid::new_v4();
        let base_time = Utc::now();

        // Create multiple symptoms for same flu episode
        let fever = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time,
            symptom_type: SymptomType::Fever,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(1440), // 24 hours
            notes: Some("102°F fever".to_string()),
            episode_id: Some(episode_id),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let cough = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time,
            symptom_type: SymptomType::Coughing,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(2880), // 48 hours
            notes: Some("Dry persistent cough".to_string()),
            episode_id: Some(episode_id),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let fatigue = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time,
            symptom_type: SymptomType::Fatigue,
            severity: SymptomSeverity::Severe,
            duration_minutes: Some(4320), // 72 hours
            notes: Some("Extreme exhaustion".to_string()),
            episode_id: Some(episode_id),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        // All symptoms should be part of the same episode
        assert!(fever.is_episode_symptom());
        assert!(cough.is_episode_symptom());
        assert!(fatigue.is_episode_symptom());

        assert_eq!(fever.episode_id, Some(episode_id));
        assert_eq!(cough.episode_id, Some(episode_id));
        assert_eq!(fatigue.episode_id, Some(episode_id));

        // Validate different categories
        assert_eq!(fever.get_category(), "general_systemic");
        assert_eq!(cough.get_category(), "respiratory");
        assert_eq!(fatigue.get_category(), "neurological");
    }

    /// Test symptom validation edge cases
    #[tokio::test]
    async fn test_symptom_validation_edge_cases() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test invalid negative duration
        let invalid_duration = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Headache,
            severity: SymptomSeverity::Mild,
            duration_minutes: Some(-60), // Negative duration
            notes: None,
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(invalid_duration.validate_with_config(&config).is_err());
        let error = invalid_duration.validate_with_config(&config).unwrap_err();
        assert!(error.contains("cannot be negative"));

        // Test unreasonably long duration (more than 2 weeks)
        let too_long_duration = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Headache,
            severity: SymptomSeverity::Mild,
            duration_minutes: Some(20160 + 1), // 2 weeks + 1 minute
            notes: None,
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(too_long_duration.validate_with_config(&config).is_err());
        let error = too_long_duration.validate_with_config(&config).unwrap_err();
        assert!(error.contains("unreasonably long"));

        // Test critical symptom with none severity (invalid)
        let invalid_severity = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::ChestTightnessOrPain, // Critical symptom type
            severity: SymptomSeverity::None,                 // Invalid for critical symptom
            duration_minutes: Some(30),
            notes: None,
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(invalid_severity.validate_with_config(&config).is_err());
        let error = invalid_severity.validate_with_config(&config).unwrap_err();
        assert!(error.contains("cannot have 'none' severity"));
    }

    /// Test iOS symptom type parsing
    #[tokio::test]
    async fn test_ios_symptom_parsing() {
        // Test various iOS symptom string formats
        assert_eq!(
            SymptomType::from_ios_string("headache"),
            Some(SymptomType::Headache)
        );
        assert_eq!(
            SymptomType::from_ios_string("head_ache"),
            Some(SymptomType::Headache)
        );
        assert_eq!(
            SymptomType::from_ios_string("HEADACHE"),
            Some(SymptomType::Headache)
        );

        // Test respiratory symptoms
        assert_eq!(
            SymptomType::from_ios_string("shortness_of_breath"),
            Some(SymptomType::ShortnessOfBreath)
        );
        assert_eq!(
            SymptomType::from_ios_string("shortnessofbreath"),
            Some(SymptomType::ShortnessOfBreath)
        );
        assert_eq!(
            SymptomType::from_ios_string("dyspnea"),
            Some(SymptomType::ShortnessOfBreath)
        );

        // Test digestive symptoms
        assert_eq!(
            SymptomType::from_ios_string("throwing_up"),
            Some(SymptomType::Vomiting)
        );
        assert_eq!(
            SymptomType::from_ios_string("loose_stools"),
            Some(SymptomType::Diarrhea)
        );

        // Test pain symptoms
        assert_eq!(
            SymptomType::from_ios_string("stomach_pain"),
            Some(SymptomType::AbdominalCramps)
        );
        assert_eq!(
            SymptomType::from_ios_string("muscle_ache"),
            Some(SymptomType::MusclePain)
        );

        // Test unknown symptom
        assert_eq!(SymptomType::from_ios_string("unknown_symptom"), None);
    }

    /// Test symptom severity conversion from iOS
    #[tokio::test]
    async fn test_ios_severity_parsing() {
        // Test numeric severity scores
        assert_eq!(
            SymptomSeverity::from_severity_score(Some(0)),
            SymptomSeverity::None
        );
        assert_eq!(
            SymptomSeverity::from_severity_score(Some(2)),
            SymptomSeverity::Mild
        );
        assert_eq!(
            SymptomSeverity::from_severity_score(Some(5)),
            SymptomSeverity::Moderate
        );
        assert_eq!(
            SymptomSeverity::from_severity_score(Some(7)),
            SymptomSeverity::Severe
        );
        assert_eq!(
            SymptomSeverity::from_severity_score(Some(10)),
            SymptomSeverity::Critical
        );

        // Test string severity parsing
        assert_eq!(
            SymptomSeverity::from_ios_string("mild"),
            SymptomSeverity::Mild
        );
        assert_eq!(
            SymptomSeverity::from_ios_string("MODERATE"),
            SymptomSeverity::Moderate
        );
        assert_eq!(
            SymptomSeverity::from_ios_string("severe"),
            SymptomSeverity::Severe
        );
        assert_eq!(
            SymptomSeverity::from_ios_string("emergency"),
            SymptomSeverity::Critical
        );

        // Test numeric string values
        assert_eq!(SymptomSeverity::from_ios_string("3"), SymptomSeverity::Mild);
        assert_eq!(
            SymptomSeverity::from_ios_string("6"),
            SymptomSeverity::Moderate
        );
        assert_eq!(
            SymptomSeverity::from_ios_string("9"),
            SymptomSeverity::Critical
        );
    }

    /// Test symptom urgency level calculation
    #[tokio::test]
    async fn test_symptom_urgency_levels() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        let mild_symptom = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Headache,
            severity: SymptomSeverity::Mild,
            duration_minutes: Some(60),
            notes: None,
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let emergency_symptom = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::ChestTightnessOrPain,
            severity: SymptomSeverity::Critical,
            duration_minutes: Some(30),
            notes: None,
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert_eq!(mild_symptom.get_urgency_level(), 1); // Mild severity
        assert_eq!(emergency_symptom.get_urgency_level(), 5); // Emergency condition
    }

    /// Test symptom analysis generation
    #[tokio::test]
    async fn test_symptom_analysis() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        let symptom = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::Fever,
            severity: SymptomSeverity::Severe,
            duration_minutes: Some(2160), // 36 hours
            notes: Some("High fever, 103°F".to_string()),
            episode_id: Some(Uuid::new_v4()),
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        let analysis = symptom.generate_analysis();

        assert_eq!(analysis.symptom_type, SymptomType::Fever);
        assert_eq!(analysis.severity, SymptomSeverity::Severe);
        assert_eq!(analysis.category, "general_systemic");
        assert!(analysis.is_emergency); // High fever is emergency
        assert!(analysis.requires_attention);
        assert_eq!(analysis.severity_score, 7);
        assert_eq!(analysis.duration_hours, Some(36.0));
        assert!(!analysis.recommendations.is_empty());
    }

    /// Test chronic symptom identification
    #[tokio::test]
    async fn test_chronic_symptom_identification() {
        let user_id = Uuid::new_v4();
        let recorded_at = Utc::now();

        // Test chronic pain (>1 week duration)
        let chronic_pain = SymptomMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at,
            symptom_type: SymptomType::BackPain,
            severity: SymptomSeverity::Moderate,
            duration_minutes: Some(10080), // 1 week
            notes: Some("Persistent lower back pain".to_string()),
            episode_id: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        };

        assert!(chronic_pain.requires_medical_attention()); // Chronic pain requires attention

        let recommendations = chronic_pain.generate_recommendations();
        assert!(recommendations
            .iter()
            .any(|r| r.contains("medical evaluation for persistent symptoms")));
    }
}
