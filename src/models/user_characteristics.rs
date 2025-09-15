// User Characteristics models for personalized health tracking
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::models::enums::{ActivityMoveMode, BiologicalSex, BloodType, FitzpatrickSkinType};

/// User Characteristics model for static user profile data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct UserCharacteristics {
    pub id: Uuid,
    pub user_id: Uuid,

    // Biological Characteristics
    pub biological_sex: BiologicalSex,
    pub date_of_birth: Option<NaiveDate>,
    pub blood_type: BloodType,

    // Physical Characteristics
    pub fitzpatrick_skin_type: FitzpatrickSkinType,
    pub wheelchair_use: bool,

    // Fitness Device Configuration
    pub activity_move_mode: ActivityMoveMode,

    // Privacy and Medical Information
    pub emergency_contact_info: JsonValue,
    pub medical_conditions: Vec<String>,
    pub medications: Vec<String>,

    // Data Management
    pub data_sharing_preferences: JsonValue,

    // Audit Trail
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_verified_at: DateTime<Utc>,
}

impl UserCharacteristics {
    /// Create new user characteristics with default values
    pub fn new(user_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            biological_sex: BiologicalSex::NotSet,
            date_of_birth: None,
            blood_type: BloodType::NotSet,
            fitzpatrick_skin_type: FitzpatrickSkinType::NotSet,
            wheelchair_use: false,
            activity_move_mode: ActivityMoveMode::NotSet,
            emergency_contact_info: serde_json::json!({}),
            medical_conditions: vec![],
            medications: vec![],
            data_sharing_preferences: serde_json::json!({
                "research_participation": false,
                "anonymized_analytics": false,
                "emergency_sharing": true
            }),
            created_at: now,
            updated_at: now,
            last_verified_at: now,
        }
    }

    /// Calculate user's age in years
    pub fn age(&self) -> Option<u32> {
        self.date_of_birth.map(|dob| {
            let today = chrono::Utc::now().date_naive();

            today.years_since(dob).unwrap_or(0)
        })
    }

    /// Check if user characteristics are complete enough for personalization
    pub fn is_complete_for_personalization(&self) -> bool {
        self.biological_sex.is_set()
            && self.date_of_birth.is_some()
            && self.fitzpatrick_skin_type.is_set()
            && self.activity_move_mode.is_set()
    }

    /// Get completeness score (0-100)
    pub fn completeness_score(&self) -> f64 {
        let mut score = 0.0;
        let total_fields = 6.0;

        if self.biological_sex.is_set() {
            score += 1.0;
        }
        if self.date_of_birth.is_some() {
            score += 1.0;
        }
        if self.blood_type.is_set() {
            score += 1.0;
        }
        if self.fitzpatrick_skin_type.is_set() {
            score += 1.0;
        }
        // wheelchair_use is always set (boolean)
        score += 1.0;
        if self.activity_move_mode.is_set() {
            score += 1.0;
        }

        (score / total_fields) * 100.0
    }

    /// Check if emergency information is available
    pub fn has_emergency_info(&self) -> bool {
        !self.emergency_contact_info.is_null()
            && self
                .emergency_contact_info
                .as_object()
                .is_some_and(|obj| !obj.is_empty())
    }

    /// Check if medical conditions may affect health metric validation
    pub fn has_relevant_medical_conditions(&self) -> bool {
        !self.medical_conditions.is_empty()
    }

    /// Get personalized heart rate zones
    pub fn get_heart_rate_zones(&self, resting_hr: Option<u16>) -> JsonValue {
        let age = self.age().unwrap_or(30) as u16;
        let max_hr = 220_u16.saturating_sub(age);
        let resting = resting_hr.unwrap_or(60);

        // Apply biological sex adjustment
        let adjusted_max_hr =
            (max_hr as f64 * self.biological_sex.get_heart_rate_adjustment()) as u16;

        serde_json::json!({
            "max_heart_rate": adjusted_max_hr,
            "resting_heart_rate": resting,
            "zones": {
                "zone_1_fat_burn": {
                    "min": resting + ((adjusted_max_hr - resting) * 50 / 100),
                    "max": resting + ((adjusted_max_hr - resting) * 60 / 100)
                },
                "zone_2_aerobic": {
                    "min": resting + ((adjusted_max_hr - resting) * 60 / 100),
                    "max": resting + ((adjusted_max_hr - resting) * 70 / 100)
                },
                "zone_3_anaerobic": {
                    "min": resting + ((adjusted_max_hr - resting) * 70 / 100),
                    "max": resting + ((adjusted_max_hr - resting) * 80 / 100)
                },
                "zone_4_vo2_max": {
                    "min": resting + ((adjusted_max_hr - resting) * 80 / 100),
                    "max": resting + ((adjusted_max_hr - resting) * 90 / 100)
                },
                "zone_5_neuromuscular": {
                    "min": resting + ((adjusted_max_hr - resting) * 90 / 100),
                    "max": adjusted_max_hr
                }
            }
        })
    }

    /// Get UV protection recommendations based on Fitzpatrick skin type
    pub fn get_uv_recommendations(&self) -> JsonValue {
        serde_json::json!({
            "skin_type": self.fitzpatrick_skin_type.get_description(),
            "recommended_spf": self.fitzpatrick_skin_type.get_recommended_spf(),
            "burn_time_minutes": self.fitzpatrick_skin_type.get_burn_time_minutes(),
            "is_set": self.fitzpatrick_skin_type.is_set()
        })
    }

    /// Get activity personalization based on wheelchair use and move mode
    pub fn get_activity_personalization(&self) -> JsonValue {
        serde_json::json!({
            "wheelchair_use": self.wheelchair_use,
            "move_mode": self.activity_move_mode,
            "daily_goal": self.activity_move_mode.get_default_daily_goal(),
            "goal_unit": self.activity_move_mode.get_unit_string(),
            "is_accessibility_mode": self.activity_move_mode.is_accessibility_mode(),
            "step_count_relevant": !self.wheelchair_use
        })
    }

    /// Get medical emergency information
    pub fn get_emergency_info(&self) -> JsonValue {
        serde_json::json!({
            "blood_type": self.blood_type.to_medical_string(),
            "blood_type_set": self.blood_type.is_set(),
            "emergency_contacts": self.emergency_contact_info,
            "medical_conditions": self.medical_conditions,
            "medications": self.medications,
            "age": self.age()
        })
    }
}

/// Input model for creating/updating user characteristics
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default)]
pub struct UserCharacteristicsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biological_sex: Option<BiologicalSex>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_birth_date"))]
    pub date_of_birth: Option<NaiveDate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blood_type: Option<BloodType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fitzpatrick_skin_type: Option<FitzpatrickSkinType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub wheelchair_use: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_move_mode: Option<ActivityMoveMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_contact_info: Option<JsonValue>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 50, message = "Too many medical conditions"))]
    pub medical_conditions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(max = 100, message = "Too many medications"))]
    pub medications: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_sharing_preferences: Option<JsonValue>,
}

impl UserCharacteristicsInput {
    /// Convert from iOS Health Auto Export format
    pub fn from_ios_data(data: &JsonValue) -> Result<Self, String> {
        let characteristics = data
            .get("characteristics")
            .or_else(|| data.get("user_characteristics"))
            .or_else(|| data.get("profile"))
            .ok_or("No characteristics data found")?;

        Ok(Self {
            biological_sex: characteristics
                .get("biological_sex")
                .or_else(|| characteristics.get("sex"))
                .and_then(|v| v.as_str())
                .map(BiologicalSex::from_ios_string),

            date_of_birth: characteristics
                .get("date_of_birth")
                .or_else(|| characteristics.get("birthdate"))
                .and_then(|v| v.as_str())
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),

            blood_type: characteristics
                .get("blood_type")
                .and_then(|v| v.as_str())
                .map(BloodType::from_ios_string),

            fitzpatrick_skin_type: characteristics
                .get("fitzpatrick_skin_type")
                .or_else(|| characteristics.get("skin_type"))
                .and_then(|v| v.as_str())
                .map(FitzpatrickSkinType::from_ios_string),

            wheelchair_use: characteristics
                .get("wheelchair_use")
                .and_then(|v| v.as_bool()),

            activity_move_mode: characteristics
                .get("activity_move_mode")
                .or_else(|| characteristics.get("move_mode"))
                .and_then(|v| v.as_str())
                .map(ActivityMoveMode::from_ios_string),

            emergency_contact_info: characteristics.get("emergency_contacts").cloned(),

            medical_conditions: characteristics
                .get("medical_conditions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                }),

            medications: characteristics
                .get("medications")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                }),

            data_sharing_preferences: characteristics.get("data_sharing").cloned(),
        })
    }

    /// Apply updates to existing characteristics
    pub fn apply_to(&self, existing: &mut UserCharacteristics) {
        if let Some(sex) = self.biological_sex {
            existing.biological_sex = sex;
        }
        if let Some(dob) = self.date_of_birth {
            existing.date_of_birth = Some(dob);
        }
        if let Some(blood_type) = self.blood_type {
            existing.blood_type = blood_type;
        }
        if let Some(skin_type) = self.fitzpatrick_skin_type {
            existing.fitzpatrick_skin_type = skin_type;
        }
        if let Some(wheelchair) = self.wheelchair_use {
            existing.wheelchair_use = wheelchair;
        }
        if let Some(move_mode) = self.activity_move_mode {
            existing.activity_move_mode = move_mode;
        }
        if let Some(ref contacts) = self.emergency_contact_info {
            existing.emergency_contact_info = contacts.clone();
        }
        if let Some(ref conditions) = self.medical_conditions {
            existing.medical_conditions = conditions.clone();
        }
        if let Some(ref medications) = self.medications {
            existing.medications = medications.clone();
        }
        if let Some(ref preferences) = self.data_sharing_preferences {
            existing.data_sharing_preferences = preferences.clone();
        }

        existing.updated_at = Utc::now();
    }
}

/// Response model for user characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCharacteristicsResponse {
    pub characteristics: UserCharacteristics,
    pub personalization: PersonalizationInfo,
}

/// Personalization information based on user characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationInfo {
    pub completeness_score: f64,
    pub is_complete: bool,
    pub missing_fields: Vec<String>,
    pub personalization_features: Vec<String>,
    pub uv_recommendations: JsonValue,
    pub activity_personalization: JsonValue,
    pub heart_rate_zones: JsonValue,
}

impl PersonalizationInfo {
    pub fn from_characteristics(characteristics: &UserCharacteristics) -> Self {
        let mut missing_fields = Vec::new();
        let mut features = Vec::new();

        if !characteristics.biological_sex.is_set() {
            missing_fields.push("biological_sex".to_string());
        } else {
            features.push("Personalized heart rate zones".to_string());
        }

        if characteristics.date_of_birth.is_none() {
            missing_fields.push("date_of_birth".to_string());
        } else {
            features.push("Age-specific health ranges".to_string());
        }

        if !characteristics.blood_type.is_set() {
            missing_fields.push("blood_type".to_string());
        } else {
            features.push("Emergency medical information".to_string());
        }

        if !characteristics.fitzpatrick_skin_type.is_set() {
            missing_fields.push("fitzpatrick_skin_type".to_string());
        } else {
            features.push("Personalized UV protection".to_string());
        }

        if !characteristics.activity_move_mode.is_set() {
            missing_fields.push("activity_move_mode".to_string());
        } else {
            features.push("Customized fitness tracking".to_string());
        }

        if characteristics.wheelchair_use {
            features.push("Accessibility-adapted metrics".to_string());
        }

        Self {
            completeness_score: characteristics.completeness_score(),
            is_complete: characteristics.is_complete_for_personalization(),
            missing_fields,
            personalization_features: features,
            uv_recommendations: characteristics.get_uv_recommendations(),
            activity_personalization: characteristics.get_activity_personalization(),
            heart_rate_zones: characteristics.get_heart_rate_zones(None),
        }
    }
}

/// Validation function for birth date
fn validate_birth_date(date: &NaiveDate) -> Result<(), validator::ValidationError> {
    let today = chrono::Utc::now().date_naive();
    let min_date = today - chrono::Duration::days(365 * 120); // 120 years ago
    let max_date = today - chrono::Duration::days(365); // At least 1 year old

    if *date < min_date || *date > max_date {
        return Err(validator::ValidationError::new("invalid_birth_date"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_user_characteristics_creation() {
        let user_id = Uuid::new_v4();
        let characteristics = UserCharacteristics::new(user_id);

        assert_eq!(characteristics.user_id, user_id);
        assert_eq!(characteristics.biological_sex, BiologicalSex::NotSet);
        assert!(!characteristics.is_complete_for_personalization());
        assert!((characteristics.completeness_score() - 16.67).abs() < 0.01); // Only wheelchair_use is "set"
    }

    #[test]
    fn test_age_calculation() {
        let user_id = Uuid::new_v4();
        let mut characteristics = UserCharacteristics::new(user_id);

        // Set birth date to 25 years ago
        let birth_date = chrono::Utc::now().date_naive() - chrono::Duration::days(365 * 25);
        characteristics.date_of_birth = Some(birth_date);

        let age = characteristics.age().unwrap();
        assert!((24..=26).contains(&age)); // Account for date variations
    }

    #[test]
    fn test_completeness_score() {
        let user_id = Uuid::new_v4();
        let mut characteristics = UserCharacteristics::new(user_id);

        // Initially only wheelchair_use is set (1/6 = 16.67%)
        assert!((characteristics.completeness_score() - 16.67).abs() < 0.1);

        characteristics.biological_sex = BiologicalSex::Female;
        characteristics.blood_type = BloodType::APositive;
        characteristics.fitzpatrick_skin_type = FitzpatrickSkinType::Type3;
        characteristics.activity_move_mode = ActivityMoveMode::ActiveEnergy;
        characteristics.date_of_birth = Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap());

        // Now all fields are set (6/6 = 100%)
        assert_eq!(characteristics.completeness_score(), 100.0);
        assert!(characteristics.is_complete_for_personalization());
    }

    #[test]
    fn test_heart_rate_zones() {
        let user_id = Uuid::new_v4();
        let mut characteristics = UserCharacteristics::new(user_id);
        characteristics.biological_sex = BiologicalSex::Female;
        characteristics.date_of_birth = Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap());

        let zones = characteristics.get_heart_rate_zones(Some(60));

        // Check that zones are calculated
        assert!(zones["max_heart_rate"].as_u64().unwrap() > 180);
        assert!(zones["zones"]["zone_1_fat_burn"]["min"].as_u64().unwrap() > 60);
    }

    #[test]
    fn test_uv_recommendations() {
        let user_id = Uuid::new_v4();
        let mut characteristics = UserCharacteristics::new(user_id);
        characteristics.fitzpatrick_skin_type = FitzpatrickSkinType::Type2;

        let recommendations = characteristics.get_uv_recommendations();

        assert_eq!(recommendations["recommended_spf"].as_u64().unwrap(), 30);
        assert_eq!(recommendations["burn_time_minutes"].as_u64().unwrap(), 15);
        assert!(recommendations["is_set"].as_bool().unwrap());
    }

    #[test]
    fn test_activity_personalization() {
        let user_id = Uuid::new_v4();
        let mut characteristics = UserCharacteristics::new(user_id);
        characteristics.wheelchair_use = true;
        characteristics.activity_move_mode = ActivityMoveMode::MoveTime;

        let personalization = characteristics.get_activity_personalization();

        assert!(personalization["wheelchair_use"].as_bool().unwrap());
        assert!(personalization["is_accessibility_mode"].as_bool().unwrap());
        assert!(!personalization["step_count_relevant"].as_bool().unwrap());
        assert_eq!(personalization["goal_unit"].as_str().unwrap(), "minutes");
    }

    #[test]
    fn test_ios_data_parsing() {
        let ios_data = serde_json::json!({
            "characteristics": {
                "biological_sex": "female",
                "date_of_birth": "1990-05-15",
                "blood_type": "A+",
                "fitzpatrick_skin_type": "3",
                "wheelchair_use": false,
                "activity_move_mode": "active_energy"
            }
        });

        let input = UserCharacteristicsInput::from_ios_data(&ios_data).unwrap();

        assert_eq!(input.biological_sex.unwrap(), BiologicalSex::Female);
        assert_eq!(input.blood_type.unwrap(), BloodType::APositive);
        assert_eq!(
            input.fitzpatrick_skin_type.unwrap(),
            FitzpatrickSkinType::Type3
        );
        assert!(!input.wheelchair_use.unwrap());
        assert_eq!(
            input.activity_move_mode.unwrap(),
            ActivityMoveMode::ActiveEnergy
        );
    }
}
