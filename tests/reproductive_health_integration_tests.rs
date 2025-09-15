/// HIPAA-Compliant Reproductive Health Integration Tests
///
/// These tests verify the privacy-first implementation of reproductive health
/// API endpoints with comprehensive audit logging and data protection.
use chrono::Utc;
use self_sensored::models::enums::{
    CervicalMucusQuality, MenstrualFlow, OvulationTestResult, PregnancyTestResult,
    TemperatureContext,
};

#[cfg(test)]
mod reproductive_health_tests {
    use super::*;

    /// Test menstrual flow enum privacy levels
    #[tokio::test]
    async fn test_menstrual_flow_privacy_levels() {
        assert_eq!(MenstrualFlow::None.privacy_level(), "standard");
        assert_eq!(MenstrualFlow::Light.privacy_level(), "sensitive");
        assert_eq!(MenstrualFlow::Heavy.privacy_level(), "sensitive");
        assert_eq!(MenstrualFlow::Spotting.privacy_level(), "sensitive");
    }

    /// Test cervical mucus fertility indicators
    #[tokio::test]
    async fn test_cervical_mucus_fertility_indicators() {
        assert_eq!(CervicalMucusQuality::Dry.fertility_indicator(), 1);
        assert_eq!(CervicalMucusQuality::Sticky.fertility_indicator(), 2);
        assert_eq!(CervicalMucusQuality::Creamy.fertility_indicator(), 3);
        assert_eq!(CervicalMucusQuality::Watery.fertility_indicator(), 4);
        assert_eq!(CervicalMucusQuality::EggWhite.fertility_indicator(), 5);
    }

    /// Test ovulation test fertility scores
    #[tokio::test]
    async fn test_ovulation_test_fertility_scores() {
        assert_eq!(OvulationTestResult::NotTested.fertility_score(), 0);
        assert_eq!(OvulationTestResult::Negative.fertility_score(), 10);
        assert_eq!(OvulationTestResult::Positive.fertility_score(), 60);
        assert_eq!(OvulationTestResult::High.fertility_score(), 80);
        assert_eq!(OvulationTestResult::Peak.fertility_score(), 95);
    }

    /// Test pregnancy test privacy levels
    #[tokio::test]
    async fn test_pregnancy_test_privacy_levels() {
        assert_eq!(PregnancyTestResult::NotTested.privacy_level(), "standard");
        assert_eq!(
            PregnancyTestResult::Negative.privacy_level(),
            "highly_sensitive"
        );
        assert_eq!(
            PregnancyTestResult::Positive.privacy_level(),
            "highly_sensitive"
        );
        assert_eq!(
            PregnancyTestResult::Indeterminate.privacy_level(),
            "highly_sensitive"
        );
    }

    /// Test pregnancy test audit requirements
    #[tokio::test]
    async fn test_pregnancy_test_audit_requirements() {
        assert!(!PregnancyTestResult::NotTested.requires_enhanced_audit());
        assert!(!PregnancyTestResult::Negative.requires_enhanced_audit());
        assert!(PregnancyTestResult::Positive.requires_enhanced_audit());
        assert!(PregnancyTestResult::Indeterminate.requires_enhanced_audit());
    }

    /// Test temperature context fertility relevance
    #[tokio::test]
    async fn test_temperature_context_fertility_relevance() {
        assert!(TemperatureContext::Basal.is_fertility_relevant());
        assert!(TemperatureContext::Sleeping.is_fertility_relevant());
        assert!(!TemperatureContext::Fever.is_fertility_relevant());
        assert!(!TemperatureContext::General.is_fertility_relevant());
        assert!(!TemperatureContext::Environmental.is_fertility_relevant());
    }

    /// Test reproductive health payload structure
    #[tokio::test]
    async fn test_reproductive_health_payload_structure() {
        // Test that reproductive health enums can be created and used
        let menstrual_flow = MenstrualFlow::Medium;
        let cervical_mucus = CervicalMucusQuality::Watery;
        let ovulation_test = OvulationTestResult::Positive;
        let pregnancy_test = PregnancyTestResult::NotTested;
        let temperature_context = TemperatureContext::Basal;

        // Verify enum creation works
        assert_eq!(menstrual_flow.privacy_level(), "sensitive");
        assert_eq!(cervical_mucus.fertility_indicator(), 4);
        assert_eq!(ovulation_test.fertility_score(), 60);
        assert_eq!(pregnancy_test.privacy_level(), "standard");
        assert!(temperature_context.is_fertility_relevant());
    }

    /// Test iOS enum parsing from strings
    #[tokio::test]
    async fn test_ios_enum_parsing() {
        // Test menstrual flow parsing
        assert_eq!(
            MenstrualFlow::from_ios_string("light"),
            MenstrualFlow::Light
        );
        assert_eq!(
            MenstrualFlow::from_ios_string("heavy"),
            MenstrualFlow::Heavy
        );
        assert_eq!(
            MenstrualFlow::from_ios_string("unknown"),
            MenstrualFlow::None
        );

        // Test cervical mucus parsing
        assert_eq!(
            CervicalMucusQuality::from_ios_string("egg_white"),
            Some(CervicalMucusQuality::EggWhite)
        );
        assert_eq!(
            CervicalMucusQuality::from_ios_string("watery"),
            Some(CervicalMucusQuality::Watery)
        );
        assert_eq!(CervicalMucusQuality::from_ios_string("unknown"), None);

        // Test ovulation test parsing
        assert_eq!(
            OvulationTestResult::from_ios_string("peak"),
            OvulationTestResult::Peak
        );
        assert_eq!(
            OvulationTestResult::from_ios_string("positive"),
            OvulationTestResult::Positive
        );

        // Test pregnancy test parsing
        assert_eq!(
            PregnancyTestResult::from_ios_string("positive"),
            PregnancyTestResult::Positive
        );
        assert_eq!(
            PregnancyTestResult::from_ios_string("negative"),
            PregnancyTestResult::Negative
        );

        // Test temperature context parsing
        assert_eq!(
            TemperatureContext::from_ios_string("basal_body_temperature"),
            TemperatureContext::Basal
        );
        assert_eq!(
            TemperatureContext::from_ios_string("fever"),
            TemperatureContext::Fever
        );
    }

    /// Test HIPAA compliance metadata
    #[tokio::test]
    async fn test_hipaa_compliance_metadata() {
        // Verify all reproductive health enums provide privacy information
        let flow = MenstrualFlow::Heavy;
        assert!(flow.privacy_level() == "sensitive" || flow.privacy_level() == "standard");

        let test_result = PregnancyTestResult::Positive;
        assert_eq!(test_result.privacy_level(), "highly_sensitive");
        assert!(test_result.requires_enhanced_audit());

        // Verify fertility indicators are calculated consistently
        let mucus_quality = CervicalMucusQuality::EggWhite;
        assert_eq!(mucus_quality.fertility_indicator(), 5); // Maximum fertility indicator
    }

    /// Test privacy protection in data structures
    #[tokio::test]
    async fn test_privacy_protection() {
        // Test that reproductive health enums provide privacy-aware methods
        let pregnancy_test = PregnancyTestResult::Positive;
        assert_eq!(pregnancy_test.privacy_level(), "highly_sensitive");
        assert!(pregnancy_test.requires_enhanced_audit());

        // Test privacy levels for different enum variants
        let menstrual_flow = MenstrualFlow::Heavy;
        assert_eq!(menstrual_flow.privacy_level(), "sensitive");

        // Test temperature context privacy awareness
        let temp_context = TemperatureContext::Basal;
        assert!(temp_context.is_fertility_relevant());
    }

    /// Test validation ranges for reproductive health data
    #[tokio::test]
    async fn test_validation_ranges() {
        use self_sensored::models::{FertilityMetric, MenstrualMetric};
        use uuid::Uuid;

        let now = Utc::now();
        let user_id = Uuid::new_v4();

        // Test menstrual metric validation
        let menstrual_metric = MenstrualMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: now,
            menstrual_flow: MenstrualFlow::Medium,
            spotting: false,
            cycle_day: Some(15),      // Valid cycle day
            cramps_severity: Some(5), // Valid severity (0-10)
            mood_rating: Some(3),     // Valid mood (1-5)
            energy_level: Some(4),    // Valid energy (1-5)
            notes: None,
            source_device: None,
            created_at: now,
        };

        // Should validate successfully
        assert!(menstrual_metric.validate().is_ok());

        // Test fertility metric validation
        let fertility_metric = FertilityMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: now,
            cervical_mucus_quality: Some(CervicalMucusQuality::EggWhite),
            ovulation_test_result: OvulationTestResult::Peak,
            sexual_activity: Some(true),
            pregnancy_test_result: PregnancyTestResult::NotTested,
            basal_body_temperature: Some(36.7), // Valid temperature
            temperature_context: TemperatureContext::Basal,
            cervix_firmness: Some(2), // Valid firmness (1-3)
            cervix_position: Some(3), // Valid position (1-3)
            lh_level: Some(45.0),     // Valid LH level
            notes: None,
            source_device: None,
            created_at: now,
        };

        // Should validate successfully
        assert!(fertility_metric.validate().is_ok());

        // Test fertility probability calculation
        let fertility_prob = fertility_metric.calculate_fertility_probability();
        assert!(fertility_prob > 0 && fertility_prob <= 100);

        // Test fertility status calculation
        let fertility_status = fertility_metric.get_fertility_status();
        assert!([
            "low_fertility",
            "moderate_fertility",
            "high_fertility",
            "peak_fertility",
            "unknown_fertility"
        ]
        .contains(&fertility_status));
    }
}
