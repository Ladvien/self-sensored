use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::rate_limiter::{RateLimitError, RateLimiter};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("API key expired")]
    ApiKeyExpired,
    #[error("API key inactive")]
    ApiKeyInactive,
    #[error("User inactive")]
    UserInactive,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Hashing error: {0}")]
    HashingError(String),
    #[error("Invalid UUID: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(#[from] RateLimitError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub apple_health_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub permissions: Option<serde_json::Value>,
    pub rate_limit_per_hour: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user: User,
    pub api_key: ApiKey,
}

impl AuthContext {
    /// Create a new AuthContext for testing purposes
    pub fn new_for_testing(user_id: uuid::Uuid) -> Self {
        Self {
            user: User {
                id: user_id,
                email: format!("test-{user_id}@example.com"),
                apple_health_id: None,
                created_at: Some(chrono::Utc::now()),
                updated_at: None,
                is_active: Some(true),
                metadata: None,
            },
            api_key: ApiKey {
                id: uuid::Uuid::new_v4(),
                user_id,
                name: Some("Test API Key".to_string()),
                created_at: Some(chrono::Utc::now()),
                last_used_at: None,
                expires_at: None,
                is_active: Some(true),
                permissions: None,
                rate_limit_per_hour: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthService {
    pool: PgPool,
    argon2: Argon2<'static>,
    rate_limiter: Option<RateLimiter>,
}

impl AuthService {
    pub fn new(pool: PgPool) -> Self {
        Self::new_with_rate_limiter(pool, None)
    }

    pub fn new_with_rate_limiter(pool: PgPool, rate_limiter: Option<RateLimiter>) -> Self {
        // Configure Argon2 with recommended settings for API key hashing
        let argon2 = Argon2::default();

        Self {
            pool,
            argon2,
            rate_limiter,
        }
    }

    /// Get a reference to the database pool for testing purposes
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Generate a new API key with secure random generation
    pub fn generate_api_key() -> String {
        // Generate a secure 32-byte API key
        let key = Uuid::new_v4();
        format!("hea_{}", key.simple())
    }

    /// Hash an API key using Argon2
    pub fn hash_api_key(&self, api_key: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self
            .argon2
            .hash_password(api_key.as_bytes(), &salt)
            .map_err(|e| AuthError::HashingError(format!("Hash generation failed: {e}")))?
            .to_string();
        Ok(password_hash)
    }

    /// Verify an API key against its hash
    pub fn verify_api_key(&self, api_key: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::HashingError(format!("Hash parsing failed: {e}")))?;
        match self
            .argon2
            .verify_password(api_key.as_bytes(), &parsed_hash)
        {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::HashingError(format!("Hashing error: {e}"))),
        }
    }

    /// Robust detection of Argon2 hash format
    /// This replaces simple LIKE '$argon2%' pattern matching with proper validation
    fn is_argon2_hash(hash: &str) -> bool {
        // Argon2 hashes follow the format: $argon2{variant}${parameters}${salt}${hash}
        // Variants include: argon2i, argon2d, argon2id
        hash.starts_with("$argon2")
            && (hash.starts_with("$argon2i$")
                || hash.starts_with("$argon2d$")
                || hash.starts_with("$argon2id$"))
            && hash.matches('$').count() >= 5 // Minimum structure validation
    }

    /// Create a new API key for a user
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        name: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        permissions: Option<serde_json::Value>,
        rate_limit_per_hour: Option<i32>,
    ) -> Result<(String, ApiKey), AuthError> {
        // Generate the plain API key
        let plain_key = Self::generate_api_key();

        // Hash the API key
        let key_hash = self.hash_api_key(&plain_key)?;

        // Insert into database
        let row = sqlx::query!(
            r#"
            INSERT INTO api_keys (user_id, name, key_hash, expires_at, permissions, rate_limit_per_hour)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING 
                id,
                user_id,
                name,
                created_at,
                last_used_at,
                expires_at,
                is_active,
                permissions,
                rate_limit_per_hour
            "#,
            user_id,
            name,
            key_hash,
            expires_at,
            permissions,
            rate_limit_per_hour
        )
        .fetch_one(&self.pool)
        .await?;

        let api_key = ApiKey {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            expires_at: row.expires_at,
            is_active: row.is_active,
            permissions: row.permissions,
            rate_limit_per_hour: row.rate_limit_per_hour,
        };

        Ok((plain_key, api_key))
    }

    /// Authenticate an API key and return the auth context
    /// Supports both UUID-based keys (Auto Export format) and hashed keys
    /// Includes comprehensive audit logging for all authentication attempts
    /// Enforces rate limiting per API key if rate limiter is configured
    /// Includes brute force protection for failed authentication attempts
    pub async fn authenticate(
        &self,
        api_key: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<&str>,
    ) -> Result<AuthContext, AuthError> {
        // Apply IP-based rate limiting for authentication attempts to prevent brute force attacks
        if let Some(ref rate_limiter) = self.rate_limiter {
            if let Some(ip) = ip_address {
                rate_limiter.check_ip_rate_limit(&ip.to_string()).await?;
            }
        }

        // Check if the API key is a UUID (Auto Export format)
        // Auto Export sends the API key ID directly as the Bearer token
        if let Ok(api_key_uuid) = Uuid::parse_str(api_key) {
            // Direct UUID lookup for Auto Export compatibility
            let row = sqlx::query!(
                r#"
                SELECT 
                    ak.id,
                    ak.user_id,
                    ak.name,
                    ak.key_hash,
                    ak.created_at,
                    ak.last_used_at,
                    ak.expires_at,
                    ak.is_active,
                    ak.permissions,
                    ak.rate_limit_per_hour,
                    u.id as user_id_check,
                    u.email,
                    u.apple_health_id,
                    u.created_at as user_created_at,
                    u.updated_at as user_updated_at,
                    u.is_active as user_is_active,
                    u.metadata
                FROM api_keys ak
                JOIN users u ON ak.user_id = u.id
                WHERE ak.id = $1
                    AND (ak.is_active IS NULL OR ak.is_active = true) 
                    AND (u.is_active IS NULL OR u.is_active = true)
                "#,
                api_key_uuid
            )
            .fetch_optional(&self.pool)
            .await?;

            if let Some(row) = row {
                // Check if key is expired
                if let Some(expires_at) = row.expires_at {
                    if expires_at < Utc::now() {
                        // Log failed authentication due to expiration
                        self.log_audit_event(
                            Some(row.user_id),
                            Some(row.id),
                            "authentication_failed",
                            Some("api_key_expired"),
                            ip_address,
                            user_agent,
                            Some(serde_json::json!({
                                "reason": "api_key_expired",
                                "key_type": "uuid",
                                "expires_at": expires_at
                            })),
                        )
                        .await
                        .ok(); // Don't fail auth on audit log failure

                        // Track failed authentication attempt for this IP
                        if let Some(ref rate_limiter) = self.rate_limiter {
                            if let Some(ip) = ip_address {
                                let _ = rate_limiter
                                    .check_ip_rate_limit(&format!("failed_auth:{ip}"))
                                    .await;
                            }
                        }

                        return Err(AuthError::ApiKeyExpired);
                    }
                }

                // Check rate limiting before allowing authentication
                if let Some(ref rate_limiter) = self.rate_limiter {
                    rate_limiter.check_rate_limit(row.id).await?;
                }

                // Update last_used_at
                self.update_last_used(row.id).await?;

                // Create auth context
                let user = User {
                    id: row.user_id,
                    email: row.email,
                    apple_health_id: row.apple_health_id,
                    created_at: row.user_created_at,
                    updated_at: row.user_updated_at,
                    is_active: row.user_is_active,
                    metadata: row.metadata,
                };

                let api_key = ApiKey {
                    id: row.id,
                    user_id: row.user_id,
                    name: row.name,
                    created_at: row.created_at,
                    last_used_at: row.last_used_at,
                    expires_at: row.expires_at,
                    is_active: row.is_active,
                    permissions: row.permissions,
                    rate_limit_per_hour: row.rate_limit_per_hour,
                };

                // Log successful authentication
                self.log_audit_event(
                    Some(user.id),
                    Some(api_key.id),
                    "authentication_success",
                    Some("uuid_api_key"),
                    ip_address,
                    user_agent,
                    Some(serde_json::json!({
                        "key_type": "uuid",
                        "key_name": api_key.name,
                        "permissions": api_key.permissions
                    })),
                )
                .await
                .ok(); // Don't fail auth on audit log failure

                tracing::info!(
                    user_id = %user.id,
                    api_key_id = %api_key.id,
                    "Auto Export API key authenticated successfully"
                );
                return Ok(AuthContext { user, api_key });
            } else {
                // UUID not found in database - track failed attempt
                if let Some(ref rate_limiter) = self.rate_limiter {
                    if let Some(ip) = ip_address {
                        let _ = rate_limiter
                            .check_ip_rate_limit(&format!("failed_auth:{ip}"))
                            .await;
                    }
                }
            }
        }

        // If not a UUID, check against hashed keys (for our generated keys)
        // This maintains backward compatibility with hashed API keys
        if api_key.starts_with("hea_") {
            let api_keys = sqlx::query!(
                r#"
                SELECT 
                    ak.id,
                    ak.user_id,
                    ak.name,
                    ak.key_hash,
                    ak.created_at,
                    ak.last_used_at,
                    ak.expires_at,
                    ak.is_active,
                    ak.permissions,
                    ak.rate_limit_per_hour,
                    u.id as user_id_check,
                    u.email,
                    u.apple_health_id,
                    u.created_at as user_created_at,
                    u.updated_at as user_updated_at,
                    u.is_active as user_is_active,
                    u.metadata
                FROM api_keys ak
                JOIN users u ON ak.user_id = u.id
                WHERE (ak.is_active IS NULL OR ak.is_active = true) 
                    AND (u.is_active IS NULL OR u.is_active = true)
                "#
            )
            .fetch_all(&self.pool)
            .await?;

            // Find the matching API key by verifying hashes
            // Only process keys with valid Argon2 hash format (robust replacement for LIKE '$argon2%')
            for row in api_keys {
                // Skip keys that don't have valid Argon2 hash format
                if !Self::is_argon2_hash(&row.key_hash) {
                    tracing::debug!(
                        key_id = %row.id,
                        hash_preview = &row.key_hash[..std::cmp::min(20, row.key_hash.len())],
                        "Skipping key with invalid Argon2 hash format"
                    );
                    continue;
                }

                match self.verify_api_key(api_key, &row.key_hash) {
                    Ok(true) => {
                        // Check if key is expired
                        if let Some(expires_at) = row.expires_at {
                            if expires_at < Utc::now() {
                                // Log failed authentication due to expiration
                                self.log_audit_event(
                                    Some(row.user_id),
                                    Some(row.id),
                                    "authentication_failed",
                                    Some("api_key_expired"),
                                    ip_address,
                                    user_agent,
                                    Some(serde_json::json!({
                                        "reason": "api_key_expired",
                                        "key_type": "hashed",
                                        "expires_at": expires_at
                                    })),
                                )
                                .await
                                .ok(); // Don't fail auth on audit log failure

                                // Track failed authentication attempt for this IP
                                if let Some(ref rate_limiter) = self.rate_limiter {
                                    if let Some(ip) = ip_address {
                                        let _ = rate_limiter
                                            .check_ip_rate_limit(&format!("failed_auth:{ip}"))
                                            .await;
                                    }
                                }

                                return Err(AuthError::ApiKeyExpired);
                            }
                        }

                        // Check rate limiting before allowing authentication
                        if let Some(ref rate_limiter) = self.rate_limiter {
                            rate_limiter.check_rate_limit(row.id).await?;
                        }

                        // Update last_used_at
                        self.update_last_used(row.id).await?;

                        // Create auth context
                        let user = User {
                            id: row.user_id,
                            email: row.email,
                            apple_health_id: row.apple_health_id,
                            created_at: row.user_created_at,
                            updated_at: row.user_updated_at,
                            is_active: row.user_is_active,
                            metadata: row.metadata,
                        };

                        let api_key = ApiKey {
                            id: row.id,
                            user_id: row.user_id,
                            name: row.name,
                            created_at: row.created_at,
                            last_used_at: row.last_used_at,
                            expires_at: row.expires_at,
                            is_active: row.is_active,
                            permissions: row.permissions,
                            rate_limit_per_hour: row.rate_limit_per_hour,
                        };

                        // Log successful authentication
                        self.log_audit_event(
                            Some(user.id),
                            Some(api_key.id),
                            "authentication_success",
                            Some("hashed_api_key"),
                            ip_address,
                            user_agent,
                            Some(serde_json::json!({
                                "key_type": "hashed",
                                "key_name": api_key.name,
                                "permissions": api_key.permissions
                            })),
                        )
                        .await
                        .ok(); // Don't fail auth on audit log failure

                        tracing::info!(
                            user_id = %user.id,
                            api_key_id = %api_key.id,
                            "Hashed API key authenticated successfully"
                        );
                        return Ok(AuthContext { user, api_key });
                    }
                    Ok(false) => {
                        // Wrong password, continue to next key
                        continue;
                    }
                    Err(e) => {
                        // Hash parsing or other error, log and continue to next key
                        tracing::warn!("Failed to verify API key {}: {}", row.id, e);
                        continue;
                    }
                }
            }
        }

        // Log failed authentication attempt
        self.log_audit_event(
            None,
            None,
            "authentication_failed",
            Some("invalid_api_key"),
            ip_address,
            user_agent,
            Some(serde_json::json!({
                "reason": "invalid_api_key",
                "key_format": if api_key.len() == 36 && Uuid::parse_str(api_key).is_ok() {
                    "uuid"
                } else if api_key.starts_with("hea_") {
                    "hashed"
                } else {
                    "unknown"
                }
            })),
        )
        .await
        .ok(); // Don't fail auth on audit log failure

        // Track failed authentication attempt for this IP to prevent brute force attacks
        if let Some(ref rate_limiter) = self.rate_limiter {
            if let Some(ip) = ip_address {
                let _ = rate_limiter
                    .check_ip_rate_limit(&format!("failed_auth:{ip}"))
                    .await;
            }
        }

        tracing::warn!(
            "Authentication failed for invalid API key with format: {}",
            if api_key.len() == 36 && Uuid::parse_str(api_key).is_ok() {
                "uuid"
            } else if api_key.starts_with("hea_") {
                "hashed"
            } else {
                "unknown"
            }
        );

        Err(AuthError::InvalidApiKey)
    }

    /// Update the last_used_at timestamp for an API key
    async fn update_last_used(&self, api_key_id: Uuid) -> Result<(), AuthError> {
        sqlx::query!(
            "UPDATE api_keys SET last_used_at = NOW() WHERE id = $1",
            api_key_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, apple_health_id, created_at, updated_at, is_active, metadata
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| User {
            id: r.id,
            email: r.email,
            apple_health_id: r.apple_health_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
            is_active: r.is_active,
            metadata: r.metadata,
        }))
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        email: &str,
        apple_health_id: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<User, AuthError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (email, apple_health_id, metadata)
            VALUES ($1, $2, $3)
            RETURNING id, email, apple_health_id, created_at, updated_at, is_active, metadata
            "#,
            email,
            apple_health_id,
            metadata
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.id,
            email: row.email,
            apple_health_id: row.apple_health_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            is_active: row.is_active,
            metadata: row.metadata,
        })
    }

    /// List API keys for a user
    pub async fn list_api_keys(&self, user_id: Uuid) -> Result<Vec<ApiKey>, AuthError> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                id,
                user_id,
                name,
                created_at,
                last_used_at,
                expires_at,
                is_active,
                permissions,
                rate_limit_per_hour
            FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let api_keys = rows
            .into_iter()
            .map(|row| ApiKey {
                id: row.id,
                user_id: row.user_id,
                name: row.name,
                created_at: row.created_at,
                last_used_at: row.last_used_at,
                expires_at: row.expires_at,
                is_active: row.is_active,
                permissions: row.permissions,
                rate_limit_per_hour: row.rate_limit_per_hour,
            })
            .collect();

        Ok(api_keys)
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, api_key_id: Uuid, user_id: Uuid) -> Result<bool, AuthError> {
        let result = sqlx::query!(
            "UPDATE api_keys SET is_active = false WHERE id = $1 AND user_id = $2",
            api_key_id,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Log an audit event (using tracing since audit_log table doesn't exist in schema)
    #[allow(clippy::too_many_arguments)]
    pub async fn log_audit_event(
        &self,
        user_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
        action: &str,
        resource: Option<&str>,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AuthError> {
        // Since audit_log table doesn't exist in the simplified schema,
        // we'll log to structured logs instead
        tracing::info!(
            user_id = ?user_id,
            api_key_id = ?api_key_id,
            action = action,
            resource = ?resource,
            ip_address = ?ip_address,
            user_agent = ?user_agent,
            metadata = ?metadata,
            "Audit event logged"
        );

        Ok(())
    }

    /// Get rate limit status for an API key
    pub async fn get_rate_limit_status(
        &self,
        api_key_id: Uuid,
    ) -> Result<Option<super::rate_limiter::RateLimitInfo>, AuthError> {
        if let Some(ref rate_limiter) = self.rate_limiter {
            Ok(Some(rate_limiter.get_rate_limit_status(api_key_id).await?))
        } else {
            Ok(None)
        }
    }

    /// Check if rate limiting is enabled
    pub fn is_rate_limiting_enabled(&self) -> bool {
        self.rate_limiter.is_some()
    }

    /// Check if a user has admin permissions
    pub fn has_admin_permission(auth_context: &AuthContext) -> bool {
        // Check if the permissions field contains "admin" permission
        if let Some(permissions) = &auth_context.api_key.permissions {
            // Support both array format ["read", "write", "admin"] and object format {"admin": true}
            match permissions {
                serde_json::Value::Array(perms) => {
                    perms.iter().any(|p| p.as_str() == Some("admin"))
                }
                serde_json::Value::Object(perms) => perms
                    .get("admin")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if a user has a specific permission
    pub fn has_permission(auth_context: &AuthContext, permission: &str) -> bool {
        // Admin users have all permissions
        if Self::has_admin_permission(auth_context) {
            return true;
        }

        if let Some(permissions) = &auth_context.api_key.permissions {
            match permissions {
                serde_json::Value::Array(perms) => {
                    perms.iter().any(|p| p.as_str() == Some(permission))
                }
                serde_json::Value::Object(perms) => perms
                    .get(permission)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                _ => false,
            }
        } else {
            false
        }
    }
}
