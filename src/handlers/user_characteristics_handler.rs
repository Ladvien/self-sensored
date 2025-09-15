// User characteristics API handlers for personalized health tracking
//
// # User Characteristics API Documentation
//
// ## Overview
// The User Characteristics API provides endpoints for managing static user profile data
// that enables personalized health tracking and recommendations. This includes biological
// characteristics, medical information, accessibility settings, and device preferences.
//
// ## Authentication
// All endpoints require API key authentication via the `X-API-Key` header.
//
// ## Base URL
// All endpoints are prefixed with `/api/v1/user/characteristics`
//
// ## Endpoints
//
// ### GET `/api/v1/user/characteristics`
// Retrieve user's current characteristics with personalization information.
//
// **Response 200:**
// ```json
// {
//   "success": true,
//   "data": {
//     "characteristics": {
//       "id": "uuid",
//       "user_id": "uuid",
//       "biological_sex": "female",
//       "date_of_birth": "1990-05-15",
//       "blood_type": "A_positive",
//       "fitzpatrick_skin_type": "type_3",
//       "wheelchair_use": false,
//       "activity_move_mode": "active_energy",
//       "emergency_contact_info": {...},
//       "medical_conditions": ["Asthma"],
//       "medications": ["Albuterol"],
//       "data_sharing_preferences": {...},
//       "created_at": "2024-01-01T00:00:00Z",
//       "updated_at": "2024-01-01T00:00:00Z",
//       "last_verified_at": "2024-01-01T00:00:00Z"
//     },
//     "personalization": {
//       "completeness_score": 100.0,
//       "is_complete": true,
//       "missing_fields": [],
//       "personalization_features": [
//         "Personalized heart rate zones",
//         "Age-specific health ranges",
//         "Emergency medical information",
//         "Personalized UV protection",
//         "Customized fitness tracking"
//       ],
//       "uv_recommendations": {...},
//       "activity_personalization": {...},
//       "heart_rate_zones": {...}
//     }
//   },
//   "timestamp": "2024-01-01T00:00:00Z"
// }
// ```
//
// ### POST `/api/v1/user/characteristics`
// Create new user characteristics (fails if they already exist).
//
// **Request Body:**
// ```json
// {
//   "biological_sex": "female",
//   "date_of_birth": "1990-05-15",
//   "blood_type": "A_positive",
//   "fitzpatrick_skin_type": "type_3",
//   "wheelchair_use": false,
//   "activity_move_mode": "active_energy",
//   "emergency_contact_info": {
//     "name": "Emergency Contact",
//     "phone": "+1-555-0123",
//     "relationship": "spouse"
//   },
//   "medical_conditions": ["Asthma"],
//   "medications": ["Albuterol"],
//   "data_sharing_preferences": {
//     "research_participation": false,
//     "anonymized_analytics": false,
//     "emergency_sharing": true
//   }
// }
// ```
//
// ### PUT `/api/v1/user/characteristics`
// Update existing user characteristics (fails if they don't exist).
//
// ### PATCH `/api/v1/user/characteristics`
// Create or update user characteristics (upsert operation).
//
// ### DELETE `/api/v1/user/characteristics`
// Delete user characteristics permanently.
//
// ### POST `/api/v1/user/characteristics/verify`
// Mark user characteristics as verified (updates last_verified_at timestamp).
//
// ### GET `/api/v1/user/characteristics/validation/{metric_type}`
// Get personalized validation ranges for specific health metric types.
//
// **Parameters:**
// - `metric_type`: One of `heart_rate`, `blood_pressure`, `activity`
//
// **Response for heart_rate:**
// ```json
// {
//   "success": true,
//   "data": {
//     "min_resting": 42,
//     "max_resting": 105,
//     "max_exercise": 189,
//     "personalized": true
//   }
// }
// ```
//
// ### GET `/api/v1/user/characteristics/uv-recommendations`
// Get UV protection recommendations based on Fitzpatrick skin type.
//
// **Response:**
// ```json
// {
//   "success": true,
//   "data": {
//     "skin_type": "Type III - Medium (Sometimes burns, tans gradually)",
//     "recommended_spf": 20,
//     "burn_time_minutes": 20,
//     "is_set": true
//   }
// }
// ```
//
// ### GET `/api/v1/user/characteristics/activity-personalization`
// Get activity tracking personalization settings.
//
// **Response:**
// ```json
// {
//   "success": true,
//   "data": {
//     "wheelchair_use": false,
//     "move_mode": "active_energy",
//     "daily_goal": 400.0,
//     "goal_unit": "calories",
//     "is_accessibility_mode": false,
//     "step_count_relevant": true
//   }
// }
// ```
//
// ### GET `/api/v1/user/characteristics/heart-rate-zones?resting_hr=65`
// Get personalized heart rate training zones.
//
// **Query Parameters:**
// - `resting_hr` (optional): User's resting heart rate for more accurate calculations
//
// **Response:**
// ```json
// {
//   "success": true,
//   "data": {
//     "max_heart_rate": 189,
//     "resting_heart_rate": 65,
//     "zones": {
//       "zone_1_fat_burn": {"min": 127, "max": 139},
//       "zone_2_aerobic": {"min": 139, "max": 152},
//       "zone_3_anaerobic": {"min": 152, "max": 164},
//       "zone_4_vo2_max": {"min": 164, "max": 177},
//       "zone_5_neuromuscular": {"min": 177, "max": 189}
//     }
//   }
// }
// ```
//
// ### GET `/api/v1/user/characteristics/emergency-info`
// Get emergency medical information.
//
// **Response:**
// ```json
// {
//   "success": true,
//   "data": {
//     "blood_type": "A+",
//     "blood_type_set": true,
//     "emergency_contacts": {...},
//     "medical_conditions": ["Asthma"],
//     "medications": ["Albuterol"],
//     "age": 34
//   }
// }
// ```
//
// ## Admin Endpoints
//
// ### GET `/api/v1/admin/characteristics/stats`
// Get aggregated statistics about user characteristics (admin only).
//
// **Response:**
// ```json
// {
//   "success": true,
//   "data": {
//     "total_profiles": 1250,
//     "completion_rates": {
//       "biological_sex": 980,
//       "date_of_birth": 1100,
//       "blood_type": 650,
//       "fitzpatrick_skin_type": 750,
//       "activity_move_mode": 900
//     },
//     "accessibility": {
//       "wheelchair_users": 45
//     },
//     "average_completeness_score": 73.5
//   }
// }
// ```
//
// ## Data Models
//
// ### Biological Sex
// - `male`: Biological male
// - `female`: Biological female
// - `not_set`: Not specified
//
// ### Blood Type
// - `A_positive`, `A_negative`: Type A blood
// - `B_positive`, `B_negative`: Type B blood
// - `AB_positive`, `AB_negative`: Type AB blood
// - `O_positive`, `O_negative`: Type O blood
// - `not_set`: Not specified
//
// ### Fitzpatrick Skin Type (UV Sensitivity)
// - `type_1`: Very fair skin, always burns, never tans
// - `type_2`: Fair skin, usually burns, tans minimally
// - `type_3`: Medium skin, sometimes burns, tans gradually
// - `type_4`: Olive skin, rarely burns, tans well
// - `type_5`: Brown skin, very rarely burns, tans darkly
// - `type_6`: Black skin, never burns, always tans darkly
// - `not_set`: Not specified
//
// ### Activity Move Mode
// - `active_energy`: Calorie-based fitness goals (default)
// - `move_time`: Time-based fitness goals (accessibility mode)
// - `not_set`: Not specified
//
// ## Error Responses
//
// **400 Bad Request - Validation Error:**
// ```json
// {
//   "success": false,
//   "error": "Validation failed: date_of_birth is required",
//   "timestamp": "2024-01-01T00:00:00Z"
// }
// ```
//
// **404 Not Found:**
// ```json
// {
//   "success": false,
//   "error": "User characteristics not found",
//   "timestamp": "2024-01-01T00:00:00Z"
// }
// ```
//
// **409 Conflict:**
// ```json
// {
//   "success": false,
//   "error": "User characteristics already exist. Use PUT to update.",
//   "timestamp": "2024-01-01T00:00:00Z"
// }
// ```
use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use tracing::{error, info, warn};
use validator::Validate;

use crate::handlers::auth::ApiKeyInfo;
use crate::models::user_characteristics::{UserCharacteristicsInput, UserCharacteristicsResponse};
use crate::models::ApiResponse;
use crate::services::auth::AuthContext;
use crate::services::user_characteristics::UserCharacteristicsService;

/// Get user characteristics
pub async fn get_user_characteristics(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Getting user characteristics"
    );

    match characteristics_service
        .get_with_personalization(user.user.id)
        .await
    {
        Ok(Some(response)) => {
            info!(
                user_id = %user.user.id,
                completeness_score = response.personalization.completeness_score,
                "Retrieved user characteristics"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Ok(None) => {
            info!(
                user_id = %user.user.id,
                "No user characteristics found"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to get user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to retrieve user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Create user characteristics
pub async fn create_user_characteristics(
    user: AuthContext,
    input: web::Json<UserCharacteristicsInput>,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Creating user characteristics"
    );

    // Validate input
    if let Err(validation_errors) = input.validate() {
        warn!(
            user_id = %user.user.id,
            errors = ?validation_errors,
            "User characteristics validation failed"
        );

        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                "Validation failed: {validation_errors}"
            ))),
        );
    }

    // Check if characteristics already exist
    match characteristics_service.get_by_user_id(user.user.id).await {
        Ok(Some(_)) => {
            warn!(
                user_id = %user.user.id,
                "Attempting to create characteristics that already exist"
            );

            return Ok(HttpResponse::Conflict().json(ApiResponse::<()>::error(
                "User characteristics already exist. Use PUT to update.".to_string(),
            )));
        }
        Ok(None) => {
            // Proceed with creation
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to check existing characteristics"
            );

            return Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to process request".to_string(),
                )),
            );
        }
    }

    match characteristics_service
        .create(user.user.id, input.into_inner())
        .await
    {
        Ok(characteristics) => {
            let personalization_info =
                crate::models::user_characteristics::PersonalizationInfo::from_characteristics(
                    &characteristics,
                );
            let response = UserCharacteristicsResponse {
                characteristics,
                personalization: personalization_info,
            };

            info!(
                user_id = %user.user.id,
                characteristics_id = %response.characteristics.id,
                "Created user characteristics"
            );

            Ok(HttpResponse::Created().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to create user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to create user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Update user characteristics
pub async fn update_user_characteristics(
    user: AuthContext,
    input: web::Json<UserCharacteristicsInput>,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Updating user characteristics"
    );

    // Validate input
    if let Err(validation_errors) = input.validate() {
        warn!(
            user_id = %user.user.id,
            errors = ?validation_errors,
            "User characteristics validation failed"
        );

        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                "Validation failed: {validation_errors}"
            ))),
        );
    }

    match characteristics_service
        .update(user.user.id, input.into_inner())
        .await
    {
        Ok(Some(characteristics)) => {
            let personalization_info =
                crate::models::user_characteristics::PersonalizationInfo::from_characteristics(
                    &characteristics,
                );
            let response = UserCharacteristicsResponse {
                characteristics,
                personalization: personalization_info,
            };

            info!(
                user_id = %user.user.id,
                characteristics_id = %response.characteristics.id,
                "Updated user characteristics"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Ok(None) => {
            warn!(
                user_id = %user.user.id,
                "Attempting to update non-existent characteristics"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to update user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to update user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Create or update user characteristics (upsert)
pub async fn upsert_user_characteristics(
    user: AuthContext,
    input: web::Json<UserCharacteristicsInput>,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Upserting user characteristics"
    );

    // Validate input
    if let Err(validation_errors) = input.validate() {
        warn!(
            user_id = %user.user.id,
            errors = ?validation_errors,
            "User characteristics validation failed"
        );

        return Ok(
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                "Validation failed: {validation_errors}"
            ))),
        );
    }

    match characteristics_service
        .upsert(user.user.id, input.into_inner())
        .await
    {
        Ok(characteristics) => {
            let personalization_info =
                crate::models::user_characteristics::PersonalizationInfo::from_characteristics(
                    &characteristics,
                );
            let response = UserCharacteristicsResponse {
                characteristics,
                personalization: personalization_info,
            };

            info!(
                user_id = %user.user.id,
                characteristics_id = %response.characteristics.id,
                "Upserted user characteristics"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to upsert user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to process user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Delete user characteristics
pub async fn delete_user_characteristics(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Deleting user characteristics"
    );

    match characteristics_service.delete(user.user.id).await {
        Ok(true) => {
            info!(
                user_id = %user.user.id,
                "Deleted user characteristics"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
                "deleted": true,
                "message": "User characteristics deleted successfully"
            }))))
        }
        Ok(false) => {
            warn!(
                user_id = %user.user.id,
                "Attempting to delete non-existent characteristics"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to delete user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to delete user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Mark user characteristics as verified
pub async fn verify_user_characteristics(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Marking user characteristics as verified"
    );

    match characteristics_service
        .update_last_verified(user.user.id)
        .await
    {
        Ok(_) => {
            info!(
                user_id = %user.user.id,
                "Updated user characteristics verification timestamp"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
                "verified": true,
                "message": "User characteristics verified successfully"
            }))))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to verify user characteristics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to verify user characteristics".to_string(),
                )),
            )
        }
    }
}

/// Get personalized validation ranges for health metrics
pub async fn get_validation_ranges(
    user: AuthContext,
    path: web::Path<String>,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    let metric_type = path.into_inner();

    info!(
        user_id = %user.user.id,
        metric_type = %metric_type,
        "Getting personalized validation ranges"
    );

    match characteristics_service
        .get_validation_ranges(user.user.id, &metric_type)
        .await
    {
        Ok(ranges) => {
            info!(
                user_id = %user.user.id,
                metric_type = %metric_type,
                personalized = ranges.get("personalized").and_then(|v| v.as_bool()).unwrap_or(false),
                "Retrieved validation ranges"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(ranges)))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                metric_type = %metric_type,
                error = %e,
                "Failed to get validation ranges"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get validation ranges".to_string(),
                )),
            )
        }
    }
}

/// Get UV protection recommendations
pub async fn get_uv_recommendations(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Getting UV protection recommendations"
    );

    match characteristics_service
        .get_uv_recommendations(user.user.id)
        .await
    {
        Ok(Some(recommendations)) => {
            info!(
                user_id = %user.user.id,
                "Retrieved UV recommendations"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(recommendations)))
        }
        Ok(None) => {
            info!(
                user_id = %user.user.id,
                "No user characteristics found for UV recommendations"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found. Please set your skin type first.".to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to get UV recommendations"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get UV recommendations".to_string(),
                )),
            )
        }
    }
}

/// Get activity personalization settings
pub async fn get_activity_personalization(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Getting activity personalization settings"
    );

    match characteristics_service
        .get_activity_personalization(user.user.id)
        .await
    {
        Ok(Some(personalization)) => {
            info!(
                user_id = %user.user.id,
                "Retrieved activity personalization"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(personalization)))
        }
        Ok(None) => {
            info!(
                user_id = %user.user.id,
                "No user characteristics found for activity personalization"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found. Please set your activity preferences first."
                    .to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to get activity personalization"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get activity personalization".to_string(),
                )),
            )
        }
    }
}

/// Get personalized heart rate zones
pub async fn get_heart_rate_zones(
    user: AuthContext,
    query: web::Query<HeartRateZoneQuery>,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        resting_hr = query.resting_hr,
        "Getting personalized heart rate zones"
    );

    match characteristics_service
        .get_heart_rate_zones(user.user.id, query.resting_hr)
        .await
    {
        Ok(Some(zones)) => {
            info!(
                user_id = %user.user.id,
                "Retrieved heart rate zones"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(zones)))
        }
        Ok(None) => {
            info!(
                user_id = %user.user.id,
                "No user characteristics found for heart rate zones"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found. Please set your age and biological sex first."
                    .to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to get heart rate zones"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get heart rate zones".to_string(),
                )),
            )
        }
    }
}

/// Get emergency medical information
pub async fn get_emergency_info(
    user: AuthContext,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    info!(
        user_id = %user.user.id,
        "Getting emergency medical information"
    );

    match characteristics_service
        .get_emergency_info(user.user.id)
        .await
    {
        Ok(Some(info)) => {
            info!(
                user_id = %user.user.id,
                "Retrieved emergency information"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(info)))
        }
        Ok(None) => {
            info!(
                user_id = %user.user.id,
                "No user characteristics found for emergency information"
            );

            Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                "User characteristics not found. Please set your medical information first."
                    .to_string(),
            )))
        }
        Err(e) => {
            error!(
                user_id = %user.user.id,
                error = %e,
                "Failed to get emergency information"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get emergency information".to_string(),
                )),
            )
        }
    }
}

/// Get aggregate statistics (admin only)
pub async fn get_aggregate_stats(
    api_key: ApiKeyInfo,
    characteristics_service: web::Data<UserCharacteristicsService>,
) -> Result<HttpResponse> {
    // Check if API key has admin permissions
    if !api_key
        .permissions
        .as_ref()
        .and_then(|p| p.as_array())
        .map(|perms| perms.iter().any(|p| p.as_str() == Some("admin")))
        .unwrap_or(false)
    {
        warn!(
            api_key_id = %api_key.id,
            "Unauthorized attempt to access aggregate statistics"
        );

        return Ok(HttpResponse::Forbidden().json(ApiResponse::<()>::error(
            "Admin permissions required".to_string(),
        )));
    }

    info!(
        api_key_id = %api_key.id,
        "Getting user characteristics aggregate statistics"
    );

    match characteristics_service.get_aggregate_stats().await {
        Ok(stats) => {
            info!(
                api_key_id = %api_key.id,
                "Retrieved aggregate statistics"
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
        }
        Err(e) => {
            error!(
                api_key_id = %api_key.id,
                error = %e,
                "Failed to get aggregate statistics"
            );

            Ok(
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                    "Failed to get aggregate statistics".to_string(),
                )),
            )
        }
    }
}

/// Query parameter for heart rate zones
#[derive(serde::Deserialize)]
pub struct HeartRateZoneQuery {
    pub resting_hr: Option<u16>,
}

/// Configure user characteristics routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/user/characteristics")
            .route("", web::get().to(get_user_characteristics))
            .route("", web::post().to(create_user_characteristics))
            .route("", web::put().to(update_user_characteristics))
            .route("", web::patch().to(upsert_user_characteristics))
            .route("", web::delete().to(delete_user_characteristics))
            .route("/verify", web::post().to(verify_user_characteristics))
            .route(
                "/validation/{metric_type}",
                web::get().to(get_validation_ranges),
            )
            .route("/uv-recommendations", web::get().to(get_uv_recommendations))
            .route(
                "/activity-personalization",
                web::get().to(get_activity_personalization),
            )
            .route("/heart-rate-zones", web::get().to(get_heart_rate_zones))
            .route("/emergency-info", web::get().to(get_emergency_info)),
    )
    .service(
        web::scope("/v1/admin/characteristics").route("/stats", web::get().to(get_aggregate_stats)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::enums::{ActivityMoveMode, BiologicalSex, BloodType, FitzpatrickSkinType};
    use actix_web::test;
    use chrono::NaiveDate;

    // Helper function to create test input
    fn create_test_input() -> UserCharacteristicsInput {
        UserCharacteristicsInput {
            biological_sex: Some(BiologicalSex::Female),
            date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 5, 15).unwrap()),
            blood_type: Some(BloodType::APositive),
            fitzpatrick_skin_type: Some(FitzpatrickSkinType::Type3),
            wheelchair_use: Some(false),
            activity_move_mode: Some(ActivityMoveMode::ActiveEnergy),
            emergency_contact_info: None,
            medical_conditions: Some(vec!["Asthma".to_string()]),
            medications: Some(vec!["Albuterol".to_string()]),
            data_sharing_preferences: None,
        }
    }

    #[test]
    async fn test_input_validation() {
        let mut input = create_test_input();

        // Valid input should pass validation
        assert!(input.validate().is_ok());

        // Test invalid birth date (too many medical conditions)
        input.medical_conditions = Some((0..60).map(|i| format!("Condition {i}")).collect());
        assert!(input.validate().is_err());
    }

    // Note: Integration tests would require database setup and mock authentication
    // These would be added in the tests/ directory for full integration testing
}
