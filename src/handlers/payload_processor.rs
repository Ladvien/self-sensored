use actix_web::{web, Result};
use serde_json;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{IngestPayload, IosIngestPayload};
use crate::services::auth::AuthContext;

/// Configuration for payload processing
#[derive(Debug, Clone)]
pub struct PayloadProcessorConfig {
    pub max_payload_size: usize,
    pub json_parse_timeout_secs: u64,
    pub max_json_depth: usize,
    pub max_json_elements: usize,
    pub enable_streaming: bool,
}

impl Default for PayloadProcessorConfig {
    fn default() -> Self {
        Self {
            max_payload_size: 200 * 1024 * 1024, // 200MB
            json_parse_timeout_secs: 10,
            max_json_depth: 50,
            max_json_elements: 1_000_000,
            enable_streaming: false,
        }
    }
}

/// Payload processor for handling large JSON payloads safely
pub struct PayloadProcessor {
    config: PayloadProcessorConfig,
}

impl PayloadProcessor {
    pub fn new(config: PayloadProcessorConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(PayloadProcessorConfig::default())
    }

    /// Validate payload size before processing
    pub fn validate_payload_size(&self, payload_size: usize) -> Result<()> {
        if payload_size > self.config.max_payload_size {
            error!(
                "Payload size {} exceeds limit {}",
                payload_size, self.config.max_payload_size
            );
            return Err(actix_web::error::ErrorPayloadTooLarge(format!(
                "Payload size {} bytes exceeds maximum of {} MB",
                payload_size,
                self.config.max_payload_size / (1024 * 1024)
            )));
        }
        Ok(())
    }

    /// Validate JSON structure for security (prevent JSON bombs, deep nesting attacks)
    pub fn validate_json_security(&self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Err(actix_web::error::ErrorBadRequest("Empty payload"));
        }

        // Basic structural validation to prevent JSON bombs
        let validation_result = self.validate_json_structure_security(data);
        if let Err(msg) = validation_result {
            error!("JSON security validation failed: {}", msg);
            return Err(actix_web::error::ErrorBadRequest(format!(
                "JSON security validation failed: {}",
                msg
            )));
        }

        Ok(())
    }

    /// Parse payload with timeout and enhanced error handling
    pub async fn parse_payload_with_timeout(
        &self,
        raw_payload: &web::Bytes,
    ) -> Result<IngestPayload> {
        let payload_size = raw_payload.len();

        // Validate size first
        self.validate_payload_size(payload_size)?;

        // Validate JSON security
        self.validate_json_security(raw_payload)?;

        if payload_size > 10 * 1024 * 1024 {
            info!(
                "Processing large payload: {} MB",
                payload_size / (1024 * 1024)
            );
        }

        // Parse with timeout protection
        let timeout_duration = Duration::from_secs(self.config.json_parse_timeout_secs);
        match tokio::time::timeout(timeout_duration, self.parse_with_fallback(raw_payload)).await {
            Ok(Ok(payload)) => Ok(payload),
            Ok(Err(parse_error)) => {
                error!("JSON parse error: {}", parse_error);
                Err(actix_web::error::ErrorBadRequest(format!(
                    "JSON parsing error: {}",
                    parse_error
                )))
            }
            Err(_) => {
                error!(
                    "JSON parsing timeout after {} seconds",
                    self.config.json_parse_timeout_secs
                );
                Err(actix_web::error::ErrorRequestTimeout(
                    "JSON parsing took too long".to_string(),
                ))
            }
        }
    }

    /// Parse payload with fallback from iOS to standard format
    async fn parse_with_fallback(&self, raw_payload: &web::Bytes) -> Result<IngestPayload> {
        // Try iOS format first
        match self.parse_with_error_context::<IosIngestPayload>(raw_payload, "iOS format") {
            Ok(ios_payload) => {
                info!(
                    "Successfully parsed iOS format payload ({} bytes)",
                    raw_payload.len()
                );
                Ok(ios_payload.to_internal_format())
            }
            Err(ios_error) => {
                warn!("iOS format parse failed: {}", ios_error);

                // Try standard format as fallback
                match self.parse_with_error_context::<IngestPayload>(raw_payload, "standard format")
                {
                    Ok(standard_payload) => {
                        info!(
                            "Successfully parsed standard format payload ({} bytes)",
                            raw_payload.len()
                        );
                        Ok(standard_payload)
                    }
                    Err(standard_error) => {
                        error!("Failed to parse payload in both iOS and standard formats");
                        error!("iOS format error: {}", ios_error);
                        error!("Standard format error: {}", standard_error);

                        // Log excerpt for debugging (first 1000 chars)
                        let payload_str = String::from_utf8_lossy(raw_payload);
                        let preview = if payload_str.len() > 1000 {
                            &payload_str[..1000]
                        } else {
                            &payload_str
                        };
                        error!("Payload preview: {}", preview);

                        Err(actix_web::error::ErrorBadRequest(format!(
                            "Invalid JSON format. iOS error: {}. Standard error: {}",
                            ios_error, standard_error
                        )))
                    }
                }
            }
        }
    }

    /// Parse JSON with enhanced error context reporting
    fn parse_with_error_context<T: serde::de::DeserializeOwned>(
        &self,
        data: &[u8],
        format_name: &str,
    ) -> std::result::Result<T, String> {
        let deserializer = &mut serde_json::Deserializer::from_slice(data);
        match serde_path_to_error::deserialize(deserializer) {
            Ok(parsed) => Ok(parsed),
            Err(err) => {
                let path = err.path().to_string();
                let inner = err.into_inner();
                Err(format!(
                    "{} parsing failed at '{}': {}",
                    format_name, path, inner
                ))
            }
        }
    }

    /// Advanced JSON structure validation to prevent security issues
    fn validate_json_structure_security(&self, data: &[u8]) -> std::result::Result<(), String> {
        let mut brace_count = 0i32;
        let mut bracket_count = 0i32;
        let mut depth = 0u32;
        let mut max_depth = 0u32;
        let mut element_count = 0usize;
        let mut in_string = false;
        let mut escape_next = false;

        for &byte in data {
            if escape_next {
                escape_next = false;
                continue;
            }

            match byte {
                b'"' if !escape_next => {
                    in_string = !in_string;
                }
                b'\\' if in_string => escape_next = true,
                b'{' | b'[' if !in_string => {
                    depth += 1;
                    max_depth = max_depth.max(depth);
                    element_count += 1;

                    if byte == b'{' {
                        brace_count += 1;
                    } else {
                        bracket_count += 1;
                    }

                    // Security checks
                    if depth > self.config.max_json_depth as u32 {
                        return Err(format!(
                            "JSON depth {} exceeds maximum allowed depth of {}",
                            depth, self.config.max_json_depth
                        ));
                    }

                    if element_count > self.config.max_json_elements {
                        return Err(format!(
                            "JSON element count {} exceeds maximum allowed elements of {}",
                            element_count, self.config.max_json_elements
                        ));
                    }
                }
                b'}' | b']' if !in_string => {
                    if depth == 0 {
                        return Err("Unmatched closing brackets detected".to_string());
                    }
                    depth -= 1;

                    if byte == b'}' {
                        brace_count -= 1;
                    } else {
                        bracket_count -= 1;
                    }
                }
                b',' | b':' if !in_string => {
                    element_count += 1;
                    if element_count > self.config.max_json_elements {
                        return Err(format!(
                            "JSON element count {} exceeds maximum allowed elements of {}",
                            element_count, self.config.max_json_elements
                        ));
                    }
                }
                _ => {}
            }

            // Early detection of malformed structure
            if brace_count < 0 || bracket_count < 0 {
                return Err("Unmatched closing brackets detected".to_string());
            }
        }

        if in_string {
            return Err("Unterminated string detected".to_string());
        }

        if brace_count != 0 {
            return Err(format!("Unmatched braces: {} unclosed", brace_count));
        }

        if bracket_count != 0 {
            return Err(format!("Unmatched brackets: {} unclosed", bracket_count));
        }

        info!(
            "JSON security validation passed: max_depth={}, elements={}, size={}",
            max_depth,
            element_count,
            data.len()
        );

        Ok(())
    }
}

/// Store raw payload for backup and audit purposes
pub async fn store_raw_payload(
    pool: &PgPool,
    auth: &AuthContext,
    payload: &IngestPayload,
    _client_ip: &str,
) -> Result<Uuid, sqlx::Error> {
    use sha2::{Digest, Sha256};

    let payload_json = serde_json::to_string(payload).map_err(sqlx::Error::decode)?;
    let payload_hash = format!("{:x}", Sha256::digest(payload_json.as_bytes()));
    let payload_size = payload_json.len() as i32;

    let result = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (user_id, payload_hash, payload_size_bytes, raw_payload) 
        VALUES ($1, $2, $3, $4) 
        RETURNING id
        "#,
        auth.user.id,
        payload_hash,
        payload_size,
        serde_json::to_value(payload).map_err(|e| sqlx::Error::decode(e))?
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}
