use actix_web::{
    body::MessageBody,
    dev::{forward_ready, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::{ready, Ready},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info_span, instrument, warn, Span};
use uuid::Uuid;

/// Request ID header name for propagation
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// List of sensitive field names that should be masked in logs
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "api_key", 
    "token",
    "authorization",
    "secret",
    "key",
    "auth",
    "credential",
    "ssn",
    "social_security",
    "email", // Consider PII
    "phone", // Consider PII
    "address", // Consider PII
];

/// Middleware for structured logging with request ID propagation
pub struct StructuredLogger;

impl<S, B> Transform<S, ServiceRequest> for StructuredLogger
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = StructuredLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(StructuredLoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct StructuredLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for StructuredLoggerMiddleware<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        
        Box::pin(async move {
            let start_time = SystemTime::now();
            
            // Generate or extract request ID
            let request_id = req
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            // Insert request ID into request extensions for downstream use
            req.extensions_mut().insert(request_id);

            // Create structured log span with request context
            let span = info_span!(
                "http_request",
                request_id = %request_id,
                method = %req.method(),
                path = req.path(),
                query = req.query_string(),
                user_agent = req
                    .headers()
                    .get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown"),
                client_ip = extract_client_ip(&req),
                content_length = req
                    .headers()
                    .get("content-length")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0),
                timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );

            // Execute request within the span
            let _guard = span.enter();
            
            tracing::info!(
                event = "request_started",
                message = "HTTP request received"
            );

            let result = service.call(req).await;

            // Calculate request duration
            let duration = start_time
                .elapsed()
                .unwrap_or_default()
                .as_millis();

            match &result {
                Ok(res) => {
                    tracing::info!(
                        event = "request_completed",
                        status = res.status().as_u16(),
                        duration_ms = duration,
                        message = "HTTP request completed successfully"
                    );
                }
                Err(err) => {
                    tracing::error!(
                        event = "request_failed", 
                        error = %err,
                        duration_ms = duration,
                        message = "HTTP request failed with error"
                    );
                }
            }

            result
        })
    }
}

/// Extract client IP address from request headers and connection info
fn extract_client_ip(req: &ServiceRequest) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split(',').next())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
        })
        .or_else(|| {
            req.connection_info()
                .peer_addr()
                .map(|addr| addr.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string()
}

/// Mask sensitive data in a JSON value for logging
pub fn mask_sensitive_data(mut value: Value) -> Value {
    match &mut value {
        Value::Object(obj) => {
            for (key, val) in obj.iter_mut() {
                let key_lower = key.to_lowercase();
                
                // Check if this field contains sensitive data
                if SENSITIVE_FIELDS.iter().any(|&sensitive| key_lower.contains(sensitive)) {
                    *val = json!("[MASKED]");
                } else {
                    // Recursively process nested objects and arrays
                    *val = mask_sensitive_data(val.clone());
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                *item = mask_sensitive_data(item.clone());
            }
        }
        _ => {}
    }
    
    value
}

/// Mask sensitive data in a string (for URLs, headers, etc.)
pub fn mask_sensitive_string(input: &str) -> String {
    let input_lower = input.to_lowercase();
    
    // Check for sensitive patterns in the string
    for &sensitive in SENSITIVE_FIELDS {
        if input_lower.contains(sensitive) {
            // For URLs, mask after the sensitive field
            if let Some(pos) = input_lower.find(&format!("{}=", sensitive)) {
                let start = pos + sensitive.len() + 1;
                if let Some(end) = input[start..].find(&['&', ' ', '\n', '\r'][..]) {
                    let end = start + end;
                    let mut masked = input.to_string();
                    masked.replace_range(start..end, "[MASKED]");
                    return masked;
                } else {
                    // Mask to end of string
                    let mut masked = input.to_string();
                    masked.replace_range(start.., "[MASKED]");
                    return masked;
                }
            }
            
            // For headers like "Authorization: Bearer token"
            if input_lower.starts_with(&format!("{}:", sensitive)) {
                return format!("{}: [MASKED]", sensitive);
            }
        }
    }
    
    input.to_string()
}

/// Get current request ID from actix-web extensions
pub fn get_request_id(req: &actix_web::HttpRequest) -> Option<Uuid> {
    req.extensions().get::<Uuid>().copied()
}

/// Structured logging helper for controllers and services
#[macro_export]
macro_rules! log_structured {
    ($level:ident, $event:expr, $($field:ident = $value:expr),*) => {
        tracing::$level!(
            event = $event,
            $($field = $value,)*
        );
    };
}

/// Enhanced error logging with context
pub fn log_error_with_context(
    error: &dyn std::error::Error,
    context: &str,
    request_id: Option<Uuid>,
    additional_fields: Option<HashMap<String, Value>>,
) {
    // Log additional context fields if provided
    if let Some(fields) = additional_fields {
        for (key, value) in fields {
            tracing::error!(
                field_name = key,
                field_value = ?mask_sensitive_data(value),
                "Additional context field"
            );
        }
    }

    tracing::error!(
        event = "error_occurred",
        context = context,
        error = %error,
        error_chain = format_error_chain(error),
        message = format!("Error in {}: {}", context, error)
    );
}

/// Format error chain for comprehensive logging
fn format_error_chain(error: &dyn std::error::Error) -> String {
    let mut chain = Vec::new();
    let mut current = Some(error);
    
    while let Some(err) = current {
        chain.push(err.to_string());
        current = err.source();
    }
    
    chain.join(" -> ")
}

/// Performance monitoring helper
#[derive(Debug)]
pub struct PerformanceTimer {
    start_time: SystemTime,
    context: String,
    request_id: Option<Uuid>,
}

impl PerformanceTimer {
    pub fn new(context: &str, request_id: Option<Uuid>) -> Self {
        Self {
            start_time: SystemTime::now(),
            context: context.to_string(),
            request_id,
        }
    }
    
    pub fn finish(self) -> u128 {
        let duration = self.start_time.elapsed()
            .unwrap_or_default()
            .as_millis();
            
        let mut fields = vec![
            ("event", json!("performance_measurement")),
            ("context", json!(self.context)),
            ("duration_ms", json!(duration)),
        ];
        
        if let Some(id) = self.request_id {
            fields.push(("request_id", json!(id.to_string())));
        }
        
        tracing::info!(
            event = "performance_measurement",
            context = self.context,
            duration_ms = duration,
            message = format!("Performance measurement for {}: {}ms", self.context, duration)
        );
        
        duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mask_sensitive_json() {
        let input = json!({
            "username": "testuser",
            "password": "secret123",
            "api_key": "abc123def456",
            "data": {
                "token": "sensitive_token",
                "value": 42
            },
            "items": [
                {"secret": "hidden", "public": "visible"}
            ]
        });

        let masked = mask_sensitive_data(input);
        
        assert_eq!(masked["username"], json!("testuser"));
        assert_eq!(masked["password"], json!("[MASKED]"));
        assert_eq!(masked["api_key"], json!("[MASKED]"));
        assert_eq!(masked["data"]["token"], json!("[MASKED]"));
        assert_eq!(masked["data"]["value"], json!(42));
        assert_eq!(masked["items"][0]["secret"], json!("[MASKED]"));
        assert_eq!(masked["items"][0]["public"], json!("visible"));
    }

    #[test]
    fn test_mask_sensitive_string() {
        assert_eq!(
            mask_sensitive_string("https://api.example.com?api_key=secret123&other=value"),
            "https://api.example.com?api_key=[MASKED]&other=value"
        );
        
        assert_eq!(
            mask_sensitive_string("Authorization: Bearer token123"),
            "authorization: [MASKED]"
        );
        
        assert_eq!(
            mask_sensitive_string("regular string with no secrets"),
            "regular string with no secrets"
        );
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_operation", None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish();
        assert!(duration >= 10);
    }
}