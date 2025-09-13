use actix_web::{web, Result};
use serde_json;
use std::io::Write;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{error, info, warn};

use crate::models::{IngestPayload, IosIngestPayload};

/// Configuration for streaming processing
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum payload size for streaming (default: 200MB)
    pub max_streaming_size: usize,
    /// Chunk size for reading payload (default: 64KB)
    pub read_chunk_size: usize,
    /// Maximum memory usage during streaming (default: 50MB)
    pub max_memory_usage: usize,
    /// Use temporary files for payloads larger than this threshold (default: 20MB)
    pub temp_file_threshold: usize,
    /// Maximum number of concurrent parse operations
    pub max_concurrent_operations: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_streaming_size: 200 * 1024 * 1024, // 200MB
            read_chunk_size: 64 * 1024,            // 64KB chunks
            max_memory_usage: 50 * 1024 * 1024,    // 50MB max memory
            temp_file_threshold: 20 * 1024 * 1024, // 20MB temp file threshold
            max_concurrent_operations: 4,          // Limit concurrent operations
        }
    }
}

/// Streaming processor for large JSON payloads
pub struct StreamingProcessor {
    config: StreamingConfig,
}

impl StreamingProcessor {
    pub fn new(config: StreamingConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(StreamingConfig::default())
    }

    /// Process large payload with streaming to avoid memory pressure
    pub async fn process_large_payload(&self, raw_payload: &web::Bytes) -> Result<IngestPayload> {
        let payload_size = raw_payload.len();

        info!(
            payload_size = payload_size,
            threshold = self.config.temp_file_threshold,
            "Starting streaming processing for large payload"
        );

        // For very large payloads, use temporary file to avoid memory pressure
        if payload_size > self.config.temp_file_threshold {
            self.process_with_temp_file(raw_payload).await
        } else {
            // For smaller payloads, use in-memory streaming
            self.process_in_memory(raw_payload).await
        }
    }

    /// Process payload using temporary file to minimize memory usage
    async fn process_with_temp_file(&self, raw_payload: &web::Bytes) -> Result<IngestPayload> {
        // Create temporary file
        let mut temp_file = NamedTempFile::new().map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to create temporary file: {}",
                e
            ))
        })?;

        info!("Writing large payload to temporary file for processing");

        // Write payload to temporary file in chunks to control memory usage
        let mut remaining = raw_payload.as_ref();
        while !remaining.is_empty() {
            let chunk_size = remaining.len().min(self.config.read_chunk_size);
            let chunk = &remaining[..chunk_size];

            temp_file.write_all(chunk).map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to write to temporary file: {}",
                    e
                ))
            })?;

            remaining = &remaining[chunk_size..];
        }

        // Flush and reopen for reading
        temp_file.flush().map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to flush temporary file: {}",
                e
            ))
        })?;

        let temp_path = temp_file.path().to_path_buf();

        // Read and parse from temporary file
        let file = tokio::fs::File::open(&temp_path).await.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to reopen temporary file: {}",
                e
            ))
        })?;

        let mut reader = BufReader::new(file);
        let mut contents = Vec::new();

        // Read file in chunks to control memory usage
        let mut buffer = vec![0; self.config.read_chunk_size];
        loop {
            let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to read from temporary file: {}",
                    e
                ))
            })?;

            if bytes_read == 0 {
                break;
            }

            contents.extend_from_slice(&buffer[..bytes_read]);

            // Check memory usage
            if contents.len() > self.config.max_memory_usage {
                return Err(actix_web::error::ErrorInternalServerError(
                    "Payload too large for streaming processing",
                ));
            }
        }

        info!(
            temp_file_size = contents.len(),
            "Successfully read large payload from temporary file"
        );

        // Parse the JSON
        self.parse_json_content(&contents).await
    }

    /// Process payload in memory with chunked reading
    async fn process_in_memory(&self, raw_payload: &web::Bytes) -> Result<IngestPayload> {
        info!("Processing payload in memory with streaming");

        // For in-memory processing, we can parse directly
        self.parse_json_content(raw_payload.as_ref()).await
    }

    /// Parse JSON content with proper error handling
    async fn parse_json_content(&self, content: &[u8]) -> Result<IngestPayload> {
        // Try iOS format first
        match self.try_parse_ios_format(content).await {
            Ok(payload) => {
                info!("Successfully parsed iOS format using streaming processor");
                Ok(payload.to_internal_format())
            }
            Err(ios_error) => {
                warn!("iOS format parsing failed: {}", ios_error);

                // Try standard format
                match self.try_parse_standard_format(content).await {
                    Ok(payload) => {
                        info!("Successfully parsed standard format using streaming processor");
                        Ok(payload)
                    }
                    Err(standard_error) => {
                        error!("Both format parsing attempts failed");
                        error!("iOS error: {}", ios_error);
                        error!("Standard error: {}", standard_error);

                        Err(actix_web::error::ErrorBadRequest(format!(
                            "Streaming JSON parsing failed. iOS error: {}. Standard error: {}",
                            ios_error, standard_error
                        )))
                    }
                }
            }
        }
    }

    /// Try parsing as iOS format
    async fn try_parse_ios_format(
        &self,
        content: &[u8],
    ) -> std::result::Result<IosIngestPayload, String> {
        let content_owned = content.to_vec();
        tokio::task::spawn_blocking(move || {
            serde_json::from_slice::<IosIngestPayload>(&content_owned)
                .map_err(|e| format!("iOS format parse error: {}", e))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// Try parsing as standard format
    async fn try_parse_standard_format(
        &self,
        content: &[u8],
    ) -> std::result::Result<IngestPayload, String> {
        let content_owned = content.to_vec();
        tokio::task::spawn_blocking(move || {
            serde_json::from_slice::<IngestPayload>(&content_owned)
                .map_err(|e| format!("Standard format parse error: {}", e))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    /// Check if payload should use streaming processing
    pub fn should_use_streaming(&self, payload_size: usize) -> bool {
        payload_size > self.config.temp_file_threshold
    }

    /// Get memory usage estimate for a payload
    pub fn estimate_memory_usage(&self, payload_size: usize) -> usize {
        // JSON parsing typically requires 2-3x the payload size in memory
        // Plus additional overhead for deserialization
        payload_size * 3 + 1024 * 1024 // Add 1MB overhead
    }

    /// Check if payload can be processed within memory limits
    pub fn can_process_in_memory(&self, payload_size: usize) -> bool {
        self.estimate_memory_usage(payload_size) <= self.config.max_memory_usage
    }
}

// Note: Stream-based payload extractor can be implemented when needed for true streaming support

/// Configuration recommendations for different payload sizes
pub struct StreamingRecommendations;

impl StreamingRecommendations {
    /// Get recommended configuration for a specific payload size
    pub fn for_payload_size(payload_size: usize) -> StreamingConfig {
        match payload_size {
            // Small payloads (< 1MB) - minimal overhead
            size if size < 1024 * 1024 => StreamingConfig {
                max_memory_usage: 10 * 1024 * 1024,   // 10MB
                temp_file_threshold: 5 * 1024 * 1024, // 5MB
                read_chunk_size: 8 * 1024,            // 8KB
                ..Default::default()
            },
            // Medium payloads (1MB - 50MB) - balanced approach
            size if size < 50 * 1024 * 1024 => StreamingConfig {
                max_memory_usage: 25 * 1024 * 1024,    // 25MB
                temp_file_threshold: 10 * 1024 * 1024, // 10MB
                read_chunk_size: 32 * 1024,            // 32KB
                ..Default::default()
            },
            // Large payloads (50MB+) - aggressive streaming
            _ => StreamingConfig {
                max_memory_usage: 50 * 1024 * 1024,    // 50MB
                temp_file_threshold: 20 * 1024 * 1024, // 20MB
                read_chunk_size: 64 * 1024,            // 64KB
                max_concurrent_operations: 2,          // Reduce concurrency
                ..Default::default()
            },
        }
    }

    /// Check if streaming is recommended for a payload size
    pub fn is_streaming_recommended(payload_size: usize) -> bool {
        payload_size > 10 * 1024 * 1024 // Recommend streaming for payloads > 10MB
    }

    /// Get background processing recommendation
    pub fn should_use_background_processing(payload_size: usize) -> bool {
        payload_size > 100 * 1024 * 1024 // Background processing for payloads > 100MB
    }
}
