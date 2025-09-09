use actix_web::{web, HttpRequest, HttpResponse, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::auth::AuthContext;
use crate::services::auth::AuthService;

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub success: bool,
    pub api_key: Option<String>, // Plain key - only returned once
    pub key_info: Option<ApiKeyInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ListApiKeysResponse {
    pub success: bool,
    pub api_keys: Vec<ApiKeyInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeApiKeyRequest {
    pub api_key_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RevokeApiKeyResponse {
    pub success: bool,
    pub revoked: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitStatusResponse {
    pub success: bool,
    pub rate_limit_enabled: bool,
    pub status: Option<RateLimitStatus>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub requests_remaining: i32,
    pub requests_limit: i32,
    pub reset_time: DateTime<Utc>,
    pub retry_after: Option<i32>,
}

/// Create a new API key for the authenticated user
/// POST /api/v1/auth/keys
pub async fn create_api_key(
    req: HttpRequest,
    auth_context: AuthContext,
    request: web::Json<CreateApiKeyRequest>,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse> {
    // Extract client information for audit logging
    let client_ip = req.connection_info()
        .peer_addr()
        .and_then(|ip| ip.parse().ok());
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok());

    // Validate request
    if request.name.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(CreateApiKeyResponse {
            success: false,
            api_key: None,
            key_info: None,
            error: Some("API key name cannot be empty".to_string()),
        }));
    }

    if request.name.len() > 100 {
        return Ok(HttpResponse::BadRequest().json(CreateApiKeyResponse {
            success: false,
            api_key: None,
            key_info: None,
            error: Some("API key name cannot exceed 100 characters".to_string()),
        }));
    }

    if request.scopes.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CreateApiKeyResponse {
            success: false,
            api_key: None,
            key_info: None,
            error: Some("At least one scope must be specified".to_string()),
        }));
    }

    // Valid scopes
    let valid_scopes = ["read", "write", "admin"];
    for scope in &request.scopes {
        if !valid_scopes.contains(&scope.as_str()) {
            return Ok(HttpResponse::BadRequest().json(CreateApiKeyResponse {
                success: false,
                api_key: None,
                key_info: None,
                error: Some(format!("Invalid scope '{}'. Valid scopes: {}", scope, valid_scopes.join(", "))),
            }));
        }
    }

    // Check if expiration is in the future
    if let Some(expires_at) = request.expires_at {
        if expires_at <= Utc::now() {
            return Ok(HttpResponse::BadRequest().json(CreateApiKeyResponse {
                success: false,
                api_key: None,
                key_info: None,
                error: Some("Expiration date must be in the future".to_string()),
            }));
        }
    }

    match auth_service.create_api_key(
        auth_context.user.id,
        &request.name,
        request.expires_at,
        request.scopes.clone(),
    ).await {
        Ok((plain_key, api_key)) => {
            // Log API key creation
            auth_service.log_audit_event(
                Some(auth_context.user.id),
                Some(api_key.id),
                "api_key_created",
                Some("create_endpoint"),
                client_ip,
                user_agent,
                Some(serde_json::json!({
                    "key_name": api_key.name,
                    "scopes": api_key.scopes,
                    "expires_at": api_key.expires_at
                })),
            ).await.ok(); // Don't fail the request if audit logging fails

            Ok(HttpResponse::Created().json(CreateApiKeyResponse {
                success: true,
                api_key: Some(plain_key), // Only returned once!
                key_info: Some(ApiKeyInfo {
                    id: api_key.id,
                    name: api_key.name,
                    created_at: api_key.created_at,
                    expires_at: api_key.expires_at,
                    scopes: api_key.scopes,
                    is_active: api_key.is_active,
                }),
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to create API key: {}", e);
            
            // Log failed attempt
            auth_service.log_audit_event(
                Some(auth_context.user.id),
                None,
                "api_key_creation_failed",
                Some("create_endpoint"),
                client_ip,
                user_agent,
                Some(serde_json::json!({
                    "error": e.to_string(),
                    "requested_name": request.name,
                    "requested_scopes": request.scopes
                })),
            ).await.ok();

            Ok(HttpResponse::InternalServerError().json(CreateApiKeyResponse {
                success: false,
                api_key: None,
                key_info: None,
                error: Some("Failed to create API key".to_string()),
            }))
        }
    }
}

/// List API keys for the authenticated user
/// GET /api/v1/auth/keys
pub async fn list_api_keys(
    auth_context: AuthContext,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse> {
    match auth_service.list_api_keys(auth_context.user.id).await {
        Ok(api_keys) => {
            let key_infos: Vec<ApiKeyInfo> = api_keys
                .into_iter()
                .map(|key| ApiKeyInfo {
                    id: key.id,
                    name: key.name,
                    created_at: key.created_at,
                    expires_at: key.expires_at,
                    scopes: key.scopes,
                    is_active: key.is_active,
                })
                .collect();

            Ok(HttpResponse::Ok().json(ListApiKeysResponse {
                success: true,
                api_keys: key_infos,
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to list API keys for user {}: {}", auth_context.user.id, e);
            Ok(HttpResponse::InternalServerError().json(ListApiKeysResponse {
                success: false,
                api_keys: vec![],
                error: Some("Failed to retrieve API keys".to_string()),
            }))
        }
    }
}

/// Revoke an API key
/// DELETE /api/v1/auth/keys
pub async fn revoke_api_key(
    req: HttpRequest,
    auth_context: AuthContext,
    request: web::Json<RevokeApiKeyRequest>,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse> {
    // Extract client information for audit logging
    let client_ip = req.connection_info()
        .peer_addr()
        .and_then(|ip| ip.parse().ok());
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok());

    match auth_service.revoke_api_key(request.api_key_id, auth_context.user.id).await {
        Ok(revoked) => {
            // Log revocation attempt
            auth_service.log_audit_event(
                Some(auth_context.user.id),
                Some(request.api_key_id),
                if revoked { "api_key_revoked" } else { "api_key_revoke_failed" },
                Some("revoke_endpoint"),
                client_ip,
                user_agent,
                Some(serde_json::json!({
                    "revoked": revoked,
                    "reason": if revoked { "user_requested" } else { "key_not_found" }
                })),
            ).await.ok();

            if revoked {
                Ok(HttpResponse::Ok().json(RevokeApiKeyResponse {
                    success: true,
                    revoked: true,
                    error: None,
                }))
            } else {
                Ok(HttpResponse::NotFound().json(RevokeApiKeyResponse {
                    success: false,
                    revoked: false,
                    error: Some("API key not found or does not belong to user".to_string()),
                }))
            }
        }
        Err(e) => {
            tracing::error!("Failed to revoke API key {}: {}", request.api_key_id, e);
            
            // Log error
            auth_service.log_audit_event(
                Some(auth_context.user.id),
                Some(request.api_key_id),
                "api_key_revoke_error",
                Some("revoke_endpoint"),
                client_ip,
                user_agent,
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            ).await.ok();

            Ok(HttpResponse::InternalServerError().json(RevokeApiKeyResponse {
                success: false,
                revoked: false,
                error: Some("Failed to revoke API key".to_string()),
            }))
        }
    }
}

/// Get rate limit status for the current API key
/// GET /api/v1/auth/rate-limit
pub async fn get_rate_limit_status(
    auth_context: AuthContext,
    auth_service: web::Data<AuthService>,
) -> Result<HttpResponse> {
    let rate_limit_enabled = auth_service.is_rate_limiting_enabled();

    if !rate_limit_enabled {
        return Ok(HttpResponse::Ok().json(RateLimitStatusResponse {
            success: true,
            rate_limit_enabled: false,
            status: None,
            error: None,
        }));
    }

    match auth_service.get_rate_limit_status(auth_context.api_key.id).await {
        Ok(Some(rate_limit_info)) => {
            Ok(HttpResponse::Ok().json(RateLimitStatusResponse {
                success: true,
                rate_limit_enabled: true,
                status: Some(RateLimitStatus {
                    requests_remaining: rate_limit_info.requests_remaining,
                    requests_limit: rate_limit_info.requests_limit,
                    reset_time: rate_limit_info.reset_time,
                    retry_after: rate_limit_info.retry_after,
                }),
                error: None,
            }))
        }
        Ok(None) => {
            Ok(HttpResponse::Ok().json(RateLimitStatusResponse {
                success: true,
                rate_limit_enabled: false,
                status: None,
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get rate limit status: {}", e);
            Ok(HttpResponse::InternalServerError().json(RateLimitStatusResponse {
                success: false,
                rate_limit_enabled,
                status: None,
                error: Some("Failed to retrieve rate limit status".to_string()),
            }))
        }
    }
}

/// Configure the auth routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/auth")
            .route("/keys", web::post().to(create_api_key))
            .route("/keys", web::get().to(list_api_keys))
            .route("/keys", web::delete().to(revoke_api_key))
            .route("/rate-limit", web::get().to(get_rate_limit_status))
    );
}