// User characteristics service for database operations and personalization logic
use anyhow::Result;
use chrono::Utc;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::user_characteristics::{
    PersonalizationInfo, UserCharacteristics, UserCharacteristicsInput, UserCharacteristicsResponse,
};

#[derive(Clone)]
pub struct UserCharacteristicsService {
    pool: PgPool,
}

impl UserCharacteristicsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user characteristics by user ID
    pub async fn get_by_user_id(&self, user_id: Uuid) -> Result<Option<UserCharacteristics>> {
        let result = sqlx::query_as!(
            UserCharacteristics,
            r#"
            SELECT
                id, user_id,
                biological_sex as "biological_sex!: _",
                date_of_birth,
                blood_type as "blood_type!: _",
                fitzpatrick_skin_type as "fitzpatrick_skin_type!: _",
                wheelchair_use,
                activity_move_mode as "activity_move_mode!: _",
                emergency_contact_info,
                medical_conditions,
                medications,
                data_sharing_preferences,
                created_at, updated_at, last_verified_at
            FROM user_characteristics
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Create new user characteristics
    pub async fn create(&self, user_id: Uuid, input: UserCharacteristicsInput) -> Result<UserCharacteristics> {
        let mut characteristics = UserCharacteristics::new(user_id);
        input.apply_to(&mut characteristics);

        let result = sqlx::query_as!(
            UserCharacteristics,
            r#"
            INSERT INTO user_characteristics (
                id, user_id, biological_sex, date_of_birth, blood_type,
                fitzpatrick_skin_type, wheelchair_use, activity_move_mode,
                emergency_contact_info, medical_conditions, medications,
                data_sharing_preferences, created_at, updated_at, last_verified_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            RETURNING
                id, user_id,
                biological_sex as "biological_sex!: _",
                date_of_birth,
                blood_type as "blood_type!: _",
                fitzpatrick_skin_type as "fitzpatrick_skin_type!: _",
                wheelchair_use,
                activity_move_mode as "activity_move_mode!: _",
                emergency_contact_info,
                medical_conditions,
                medications,
                data_sharing_preferences,
                created_at, updated_at, last_verified_at
            "#,
            characteristics.id,
            characteristics.user_id,
            characteristics.biological_sex as _,
            characteristics.date_of_birth,
            characteristics.blood_type as _,
            characteristics.fitzpatrick_skin_type as _,
            characteristics.wheelchair_use,
            characteristics.activity_move_mode as _,
            characteristics.emergency_contact_info,
            &characteristics.medical_conditions,
            &characteristics.medications,
            characteristics.data_sharing_preferences,
            characteristics.created_at,
            characteristics.updated_at,
            characteristics.last_verified_at
        )
        .fetch_one(&self.pool)
        .await?;

        info!(
            user_id = %user_id,
            characteristics_id = %result.id,
            "Created user characteristics"
        );

        Ok(result)
    }

    /// Update existing user characteristics
    pub async fn update(&self, user_id: Uuid, input: UserCharacteristicsInput) -> Result<Option<UserCharacteristics>> {
        let mut tx = self.pool.begin().await?;

        // Get existing characteristics
        let mut existing = match self.get_by_user_id(user_id).await? {
            Some(existing) => existing,
            None => {
                warn!(user_id = %user_id, "Attempting to update non-existent user characteristics");
                return Ok(None);
            }
        };

        // Apply updates
        input.apply_to(&mut existing);

        let result = sqlx::query_as!(
            UserCharacteristics,
            r#"
            UPDATE user_characteristics
            SET
                biological_sex = $3,
                date_of_birth = $4,
                blood_type = $5,
                fitzpatrick_skin_type = $6,
                wheelchair_use = $7,
                activity_move_mode = $8,
                emergency_contact_info = $9,
                medical_conditions = $10,
                medications = $11,
                data_sharing_preferences = $12,
                updated_at = $13
            WHERE user_id = $1
            RETURNING
                id, user_id,
                biological_sex as "biological_sex!: _",
                date_of_birth,
                blood_type as "blood_type!: _",
                fitzpatrick_skin_type as "fitzpatrick_skin_type!: _",
                wheelchair_use,
                activity_move_mode as "activity_move_mode!: _",
                emergency_contact_info,
                medical_conditions,
                medications,
                data_sharing_preferences,
                created_at, updated_at, last_verified_at
            "#,
            user_id,
            existing.id,
            existing.biological_sex as _,
            existing.date_of_birth,
            existing.blood_type as _,
            existing.fitzpatrick_skin_type as _,
            existing.wheelchair_use,
            existing.activity_move_mode as _,
            existing.emergency_contact_info,
            &existing.medical_conditions,
            &existing.medications,
            existing.data_sharing_preferences,
            existing.updated_at
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        info!(
            user_id = %user_id,
            characteristics_id = %result.id,
            "Updated user characteristics"
        );

        Ok(Some(result))
    }

    /// Create or update user characteristics (upsert)
    pub async fn upsert(&self, user_id: Uuid, input: UserCharacteristicsInput) -> Result<UserCharacteristics> {
        match self.get_by_user_id(user_id).await? {
            Some(_) => {
                // Update existing
                self.update(user_id, input).await?
                    .ok_or_else(|| anyhow::anyhow!("Failed to update user characteristics"))
            }
            None => {
                // Create new
                self.create(user_id, input).await
            }
        }
    }

    /// Delete user characteristics
    pub async fn delete(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM user_characteristics WHERE user_id = $1",
            user_id
        )
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;

        if deleted {
            info!(user_id = %user_id, "Deleted user characteristics");
        }

        Ok(deleted)
    }

    /// Update last verified timestamp
    pub async fn update_last_verified(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE user_characteristics SET last_verified_at = $1 WHERE user_id = $2",
            Utc::now(),
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user characteristics with personalization info
    pub async fn get_with_personalization(&self, user_id: Uuid) -> Result<Option<UserCharacteristicsResponse>> {
        match self.get_by_user_id(user_id).await? {
            Some(characteristics) => {
                let personalization = PersonalizationInfo::from_characteristics(&characteristics);
                Ok(Some(UserCharacteristicsResponse {
                    characteristics,
                    personalization,
                }))
            }
            None => Ok(None)
        }
    }

    /// Get personalized validation ranges for health metrics
    pub async fn get_validation_ranges(&self, user_id: Uuid, metric_type: &str) -> Result<JsonValue> {
        let characteristics = self.get_by_user_id(user_id).await?;

        let ranges = match characteristics {
            Some(chars) => {
                let age = chars.age();
                match metric_type {
                    "heart_rate" => {
                        let adjustment = chars.biological_sex.get_heart_rate_adjustment();
                        let min_resting = match age {
                            Some(age) if age < 30 => 40,
                            Some(age) if age < 50 => 45,
                            _ => 50,
                        };
                        let max_resting = match age {
                            Some(age) if age < 30 => 100,
                            Some(age) if age < 50 => 95,
                            _ => 90,
                        };
                        let max_exercise = 220_u32.saturating_sub(age.unwrap_or(30));

                        serde_json::json!({
                            "min_resting": (min_resting as f64 * adjustment) as u32,
                            "max_resting": (max_resting as f64 * adjustment) as u32,
                            "max_exercise": (max_exercise as f64 * adjustment) as u32,
                            "personalized": true
                        })
                    }
                    "blood_pressure" => {
                        let systolic_max = match age {
                            Some(age) if age < 65 => 140,
                            _ => 150,
                        };
                        serde_json::json!({
                            "systolic_min": 90,
                            "systolic_max": systolic_max,
                            "diastolic_min": 60,
                            "diastolic_max": 90,
                            "personalized": true
                        })
                    }
                    "activity" => {
                        let step_max = if chars.wheelchair_use { 10000 } else { 50000 };
                        let distance_max = if chars.wheelchair_use { 100.0 } else { 200.0 };
                        serde_json::json!({
                            "step_count_max": step_max,
                            "distance_max_km": distance_max,
                            "wheelchair_adapted": chars.wheelchair_use,
                            "personalized": true
                        })
                    }
                    _ => serde_json::json!({
                        "personalized": false,
                        "status": "no_personalization_available"
                    })
                }
            }
            None => serde_json::json!({
                "personalized": false,
                "status": "no_characteristics_found"
            })
        };

        Ok(ranges)
    }

    /// Get users who need profile completion reminders
    pub async fn get_incomplete_profiles(&self, limit: Option<i64>) -> Result<Vec<Uuid>> {
        let limit = limit.unwrap_or(100);

        let results = sqlx::query_scalar!(
            r#"
            SELECT user_id
            FROM user_characteristics
            WHERE
                biological_sex = 'not_set'
                OR date_of_birth IS NULL
                OR fitzpatrick_skin_type = 'not_set'
                OR activity_move_mode = 'not_set'
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }

    /// Get aggregate statistics (anonymized)
    pub async fn get_aggregate_stats(&self) -> Result<JsonValue> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total_profiles,
                COUNT(*) FILTER (WHERE biological_sex != 'not_set') as sex_set_count,
                COUNT(*) FILTER (WHERE date_of_birth IS NOT NULL) as age_set_count,
                COUNT(*) FILTER (WHERE blood_type != 'not_set') as blood_type_set_count,
                COUNT(*) FILTER (WHERE fitzpatrick_skin_type != 'not_set') as skin_type_set_count,
                COUNT(*) FILTER (WHERE activity_move_mode != 'not_set') as move_mode_set_count,
                COUNT(*) FILTER (WHERE wheelchair_use = true) as wheelchair_users,
                ROUND(AVG(
                    CASE WHEN biological_sex != 'not_set' THEN 1 ELSE 0 END +
                    CASE WHEN date_of_birth IS NOT NULL THEN 1 ELSE 0 END +
                    CASE WHEN blood_type != 'not_set' THEN 1 ELSE 0 END +
                    CASE WHEN fitzpatrick_skin_type != 'not_set' THEN 1 ELSE 0 END +
                    CASE WHEN activity_move_mode != 'not_set' THEN 1 ELSE 0 END + 1
                ) * 100.0 / 6.0, 2) as avg_completeness_score
            FROM user_characteristics
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(serde_json::json!({
            "total_profiles": stats.total_profiles,
            "completion_rates": {
                "biological_sex": stats.sex_set_count,
                "date_of_birth": stats.age_set_count,
                "blood_type": stats.blood_type_set_count,
                "fitzpatrick_skin_type": stats.skin_type_set_count,
                "activity_move_mode": stats.move_mode_set_count
            },
            "accessibility": {
                "wheelchair_users": stats.wheelchair_users
            },
            "average_completeness_score": stats.avg_completeness_score.unwrap_or(0.0)
        }))
    }

    /// Check if user has characteristics that affect metric validation
    pub async fn has_personalization_data(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_characteristics
                WHERE user_id = $1
                AND (
                    biological_sex != 'not_set'
                    OR date_of_birth IS NOT NULL
                    OR wheelchair_use = true
                    OR medical_conditions != '{}'
                )
            )
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.unwrap_or(false))
    }

    /// Process characteristics from iOS Health Auto Export data
    pub async fn process_ios_data(&self, user_id: Uuid, ios_data: &JsonValue) -> Result<Option<UserCharacteristics>> {
        match UserCharacteristicsInput::from_ios_data(ios_data) {
            Ok(input) => {
                info!(
                    user_id = %user_id,
                    "Processing user characteristics from iOS data"
                );

                let result = self.upsert(user_id, input).await?;
                Ok(Some(result))
            }
            Err(e) => {
                warn!(
                    user_id = %user_id,
                    error = %e,
                    "No valid characteristics data found in iOS payload"
                );
                Ok(None)
            }
        }
    }

    /// Get emergency medical information for a user
    pub async fn get_emergency_info(&self, user_id: Uuid) -> Result<Option<JsonValue>> {
        match self.get_by_user_id(user_id).await? {
            Some(characteristics) => Ok(Some(characteristics.get_emergency_info())),
            None => Ok(None)
        }
    }

    /// Get UV recommendations for current conditions
    pub async fn get_uv_recommendations(&self, user_id: Uuid) -> Result<Option<JsonValue>> {
        match self.get_by_user_id(user_id).await? {
            Some(characteristics) => Ok(Some(characteristics.get_uv_recommendations())),
            None => Ok(None)
        }
    }

    /// Get activity personalization settings
    pub async fn get_activity_personalization(&self, user_id: Uuid) -> Result<Option<JsonValue>> {
        match self.get_by_user_id(user_id).await? {
            Some(characteristics) => Ok(Some(characteristics.get_activity_personalization())),
            None => Ok(None)
        }
    }

    /// Get personalized heart rate zones
    pub async fn get_heart_rate_zones(&self, user_id: Uuid, resting_hr: Option<u16>) -> Result<Option<JsonValue>> {
        match self.get_by_user_id(user_id).await? {
            Some(characteristics) => Ok(Some(characteristics.get_heart_rate_zones(resting_hr))),
            None => Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::enums::{ActivityMoveMode, BiologicalSex, BloodType, FitzpatrickSkinType};
    use chrono::NaiveDate;

    // Note: These tests would need a test database setup
    // They are here as examples of how to test the service

    #[tokio::test]
    #[ignore = "Requires database setup"]
    async fn test_create_user_characteristics() {
        // This test would need proper database setup
        let pool = PgPool::connect("postgresql://test").await.unwrap();
        let service = UserCharacteristicsService::new(pool);
        let user_id = Uuid::new_v4();

        let input = UserCharacteristicsInput {
            biological_sex: Some(BiologicalSex::Female),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 5, 15).unwrap()),
            blood_type: Some(BloodType::APositive),
            fitzpatrick_skin_type: Some(FitzpatrickSkinType::Type3),
            wheelchair_use: Some(false),
            activity_move_mode: Some(ActivityMoveMode::ActiveEnergy),
            emergency_contact_info: None,
            medical_conditions: None,
            medications: None,
            data_sharing_preferences: None,
        };

        let result = service.create(user_id, input).await.unwrap();
        assert_eq!(result.user_id, user_id);
        assert_eq!(result.biological_sex, BiologicalSex::Female);
    }

    #[tokio::test]
    #[ignore = "Requires database setup"]
    async fn test_get_validation_ranges() {
        let pool = PgPool::connect("postgresql://test").await.unwrap();
        let service = UserCharacteristicsService::new(pool);
        let user_id = Uuid::new_v4();

        // Create test user characteristics
        let input = UserCharacteristicsInput {
            biological_sex: Some(BiologicalSex::Female),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 5, 15).unwrap()),
            wheelchair_use: Some(true),
            ..Default::default()
        };

        service.create(user_id, input).await.unwrap();

        // Test heart rate ranges
        let hr_ranges = service.get_validation_ranges(user_id, "heart_rate").await.unwrap();
        assert!(hr_ranges["personalized"].as_bool().unwrap());
        assert!(hr_ranges["max_exercise"].as_u64().unwrap() > 180);

        // Test activity ranges (should be adapted for wheelchair use)
        let activity_ranges = service.get_validation_ranges(user_id, "activity").await.unwrap();
        assert!(activity_ranges["wheelchair_adapted"].as_bool().unwrap());
        assert_eq!(activity_ranges["step_count_max"].as_u64().unwrap(), 10000);
    }
}