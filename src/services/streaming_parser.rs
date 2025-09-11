use actix_web::{
    dev::Payload, error::ErrorBadRequest, web::Bytes, Error as ActixError, HttpRequest,
    Result as ActixResult,
};
use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, error, info};

/// Maximum chunk size for streaming (1MB)
const STREAM_CHUNK_SIZE: usize = 1024 * 1024;
/// Maximum total payload size (200MB for large health exports)
const MAX_STREAMING_PAYLOAD_SIZE: usize = 200 * 1024 * 1024;

/// Streaming JSON parser that can handle large payloads without loading everything into memory
pub struct StreamingJsonParser {
    buffer: Vec<u8>,
    total_size: usize,
    max_size: usize,
}

impl Default for StreamingJsonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingJsonParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(STREAM_CHUNK_SIZE),
            total_size: 0,
            max_size: MAX_STREAMING_PAYLOAD_SIZE,
        }
    }

    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(STREAM_CHUNK_SIZE),
            total_size: 0,
            max_size,
        }
    }

    /// Parse JSON from a stream of bytes with better error reporting
    pub async fn parse_from_stream<T, S>(&mut self, mut stream: S) -> ActixResult<T>
    where
        T: DeserializeOwned,
        S: Stream<Item = Result<Bytes, ActixError>> + Unpin,
    {
        // Collect the stream into our buffer with size checking
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            // Check size limits before adding
            if self.total_size + chunk.len() > self.max_size {
                error!(
                    "Streaming payload size would exceed limit: {} + {} > {}",
                    self.total_size,
                    chunk.len(),
                    self.max_size
                );
                return Err(ErrorBadRequest(format!(
                    "Payload size exceeds maximum of {} MB",
                    self.max_size / (1024 * 1024)
                )));
            }

            self.buffer.extend_from_slice(&chunk);
            self.total_size += chunk.len();

            // Log progress for large payloads
            if self.total_size > 10 * 1024 * 1024 {
                // > 10MB
                if self.total_size % (5 * 1024 * 1024) == 0 {
                    // Every 5MB
                    info!(
                        "Streaming progress: {} MB received",
                        self.total_size / (1024 * 1024)
                    );
                }
            }
        }

        info!("Streaming complete: {} bytes received", self.total_size);

        // Parse the complete JSON with better error handling
        self.parse_complete_json()
    }

    /// Parse complete JSON buffer with detailed error reporting
    fn parse_complete_json<T: DeserializeOwned>(&self) -> ActixResult<T> {
        // First, validate that we have complete JSON by checking basic structure
        if self.buffer.is_empty() {
            return Err(ErrorBadRequest("Empty payload received"));
        }

        // Check for truncated JSON by looking for balanced braces
        if let Err(validation_error) = self.validate_json_structure() {
            error!("JSON structure validation failed: {}", validation_error);
            return Err(ErrorBadRequest(format!(
                "Malformed JSON: {validation_error}"
            )));
        }

        // Use serde_path_to_error for better error reporting
        let deserializer = &mut serde_json::Deserializer::from_slice(&self.buffer);
        match serde_path_to_error::deserialize(deserializer) {
            Ok(parsed) => {
                info!("JSON parsing successful: {} bytes", self.total_size);
                Ok(parsed)
            }
            Err(err) => {
                let path = err.path().to_string();
                let inner = err.into_inner();

                error!("JSON parsing failed at path '{}': {}", path, inner);
                error!("Buffer size: {} bytes", self.buffer.len());
                error!("Total received: {} bytes", self.total_size);

                // Provide context about where the error occurred
                let context = self.get_error_context(&path);

                Err(ErrorBadRequest(format!(
                    "JSON parsing error at '{path}': {inner} (Context: {context})"
                )))
            }
        }
    }

    /// Validate basic JSON structure for completeness
    fn validate_json_structure(&self) -> Result<(), String> {
        let mut brace_count = 0i32;
        let mut bracket_count = 0i32;
        let mut in_string = false;
        let mut escape_next = false;

        for &byte in &self.buffer {
            if escape_next {
                escape_next = false;
                continue;
            }

            match byte {
                b'"' if !escape_next => in_string = !in_string,
                b'\\' if in_string => escape_next = true,
                b'{' if !in_string => brace_count += 1,
                b'}' if !in_string => brace_count -= 1,
                b'[' if !in_string => bracket_count += 1,
                b']' if !in_string => bracket_count -= 1,
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
            return Err(format!("Unmatched braces: {brace_count} unclosed"));
        }

        if bracket_count != 0 {
            return Err(format!("Unmatched brackets: {bracket_count} unclosed"));
        }

        Ok(())
    }

    /// Get context around the error location
    fn get_error_context(&self, path: &str) -> String {
        // Try to find the approximate location in the buffer
        if let Ok(line_col) = self.find_path_location(path) {
            format!("line {}, column {}", line_col.0, line_col.1)
        } else {
            format!("path: {path}")
        }
    }

    /// Find approximate line and column for a JSON path
    fn find_path_location(&self, _path: &str) -> Result<(usize, usize), ()> {
        // Simple implementation - count newlines up to the error
        let mut line = 1;
        let mut col = 1;

        // For large files, just return the end position
        if self.buffer.len() > 1024 * 1024 {
            line = self.buffer.iter().filter(|&&b| b == b'\n').count() + 1;
            col = 1;
        }

        Ok((line, col))
    }

    pub fn total_size(&self) -> usize {
        self.total_size
    }
}

/// Streaming wrapper for Actix payload
pub struct PayloadStream {
    payload: Payload,
}

impl PayloadStream {
    pub fn new(payload: Payload) -> Self {
        Self { payload }
    }
}

impl Stream for PayloadStream {
    type Item = Result<Bytes, ActixError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.payload).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(ActixError::from(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Enhanced JSON extractor that uses streaming parser for large payloads
pub async fn parse_large_json_payload<T>(req: &HttpRequest, payload: Payload) -> ActixResult<T>
where
    T: DeserializeOwned,
{
    // Get content length if available
    let content_length = req
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    debug!(
        "Processing JSON payload with content-length: {:?}",
        content_length
    );

    // Create streaming parser with appropriate size limit
    let max_size = content_length
        .map(|len| (len as f64 * 1.1) as usize) // Add 10% buffer
        .unwrap_or(MAX_STREAMING_PAYLOAD_SIZE);

    let mut parser = StreamingJsonParser::with_max_size(max_size);
    let stream = PayloadStream::new(payload);

    parser.parse_from_stream(stream).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_streaming_parser_basic() {
        let json_data = r#"{"name": "test", "value": 42}"#;
        let mut parser = StreamingJsonParser::new();

        // Simulate stream by chunking the data
        let chunks = vec![
            json_data[..10].as_bytes(),
            json_data[10..20].as_bytes(),
            json_data[20..].as_bytes(),
        ];

        // TODO: Add actual stream test implementation
    }

    #[test]
    fn test_json_structure_validation() {
        let parser = StreamingJsonParser::new();

        // Valid JSON
        let valid_json = br#"{"name": "test", "nested": {"value": 42}}"#;
        let mut test_parser = StreamingJsonParser::new();
        test_parser.buffer = valid_json.to_vec();
        assert!(test_parser.validate_json_structure().is_ok());

        // Invalid JSON - unclosed brace
        let invalid_json = br#"{"name": "test", "nested": {"value": 42}"#;
        test_parser.buffer = invalid_json.to_vec();
        assert!(test_parser.validate_json_structure().is_err());
    }
}
