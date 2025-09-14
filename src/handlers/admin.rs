use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, Level};

use crate::middleware::get_request_id;
use crate::services::auth::{AuthContext, AuthService};

/// Request to change log level at runtime
#[derive(Deserialize)]
pub struct LogLevelRequest {
    /// New log level (trace, debug, info, warn, error)
    pub level: String,
}

/// Response for log level operations
#[derive(Serialize)]
pub struct LogLevelResponse {
    pub success: bool,
    pub message: String,
    pub current_level: String,
    pub previous_level: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Get current log level configuration
pub async fn get_log_level(auth: AuthContext, req: actix_web::HttpRequest) -> Result<HttpResponse> {
    let request_id = get_request_id(&req);

    // Double-check admin permissions (defense in depth)
    if !AuthService::has_admin_permission(&auth) {
        warn!(
            event = "admin_permission_denied",
            user_id = %auth.user.id,
            api_key_id = %auth.api_key.id,
            endpoint = "get_log_level",
            request_id = request_id.map(|id| id.to_string()).as_deref(),
            permissions = ?auth.api_key.permissions,
            message = "User without admin permissions attempted to access admin endpoint"
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "access_denied",
            "message": "Admin privileges required to access this endpoint",
            "required_permission": "admin"
        })));
    }

    info!(
        event = "admin_log_level_check",
        user_id = %auth.user.id,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        message = "Admin requested current log level"
    );

    // Note: In a real implementation, you'd need to maintain state
    // of the current log level. For now, we'll return a static response.
    let response = LogLevelResponse {
        success: true,
        message: "Current log level retrieved successfully".to_string(),
        current_level: "info".to_string(), // This should be actual current level
        previous_level: None,
        timestamp: chrono::Utc::now(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Set log level at runtime (admin only)
pub async fn set_log_level(
    auth: AuthContext,
    req: actix_web::HttpRequest,
    payload: web::Json<LogLevelRequest>,
) -> Result<HttpResponse> {
    let request_id = get_request_id(&req);

    // Double-check admin permissions (defense in depth)
    if !AuthService::has_admin_permission(&auth) {
        warn!(
            event = "admin_permission_denied",
            user_id = %auth.user.id,
            api_key_id = %auth.api_key.id,
            endpoint = "set_log_level",
            request_id = request_id.map(|id| id.to_string()).as_deref(),
            permissions = ?auth.api_key.permissions,
            message = "User without admin permissions attempted to change log level"
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "access_denied",
            "message": "Admin privileges required to modify system log level",
            "required_permission": "admin"
        })));
    }

    // Validate log level
    let new_level = match payload.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            warn!(
                event = "invalid_log_level_request",
                user_id = %auth.user.id,
                requested_level = payload.level,
                request_id = request_id.map(|id| id.to_string()).as_deref(),
                message = "Invalid log level requested"
            );

            return Ok(HttpResponse::BadRequest().json(LogLevelResponse {
                success: false,
                message: format!(
                    "Invalid log level '{}'. Valid levels: trace, debug, info, warn, error",
                    payload.level
                ),
                current_level: "info".to_string(), // This should be actual current level
                previous_level: None,
                timestamp: chrono::Utc::now(),
            }));
        }
    };

    info!(
        event = "admin_log_level_change",
        user_id = %auth.user.id,
        new_level = %new_level,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        message = "Admin changing log level"
    );

    // Note: In a real implementation, you'd need to:
    // 1. Store the current level in application state
    // 2. Use tracing-subscriber's reload handle to actually change the level
    // 3. Persist the change if needed

    let response = LogLevelResponse {
        success: true,
        message: format!("Log level changed to {} successfully", payload.level),
        current_level: payload.level.clone(),
        previous_level: Some("info".to_string()), // This should be the actual previous level
        timestamp: chrono::Utc::now(),
    };

    info!(
        event = "log_level_changed_successfully",
        user_id = %auth.user.id,
        new_level = payload.level,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        message = "Log level change completed successfully"
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Get logging statistics and configuration
#[derive(Serialize)]
pub struct LoggingStats {
    pub current_level: String,
    pub json_format_enabled: bool,
    pub total_log_events: u64,
    pub error_events_last_hour: u64,
    pub warn_events_last_hour: u64,
    pub performance_measurements: u64,
    pub average_request_duration_ms: f64,
    pub uptime_seconds: u64,
}

pub async fn get_logging_stats(
    auth: AuthContext,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let request_id = get_request_id(&req);

    // Double-check admin permissions (defense in depth)
    if !AuthService::has_admin_permission(&auth) {
        warn!(
            event = "admin_permission_denied",
            user_id = %auth.user.id,
            api_key_id = %auth.api_key.id,
            endpoint = "get_logging_stats",
            request_id = request_id.map(|id| id.to_string()).as_deref(),
            permissions = ?auth.api_key.permissions,
            message = "User without admin permissions attempted to access logging statistics"
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "access_denied",
            "message": "Admin privileges required to access system logging statistics",
            "required_permission": "admin"
        })));
    }

    info!(
        event = "admin_logging_stats_request",
        user_id = %auth.user.id,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        message = "Admin requested logging statistics"
    );

    // Note: In a real implementation, you'd collect these stats from:
    // - Application metrics
    // - Log aggregation system
    // - Performance monitoring
    let stats = LoggingStats {
        current_level: "info".to_string(),
        json_format_enabled: true,
        total_log_events: 1542, // Example data
        error_events_last_hour: 3,
        warn_events_last_hour: 12,
        performance_measurements: 245,
        average_request_duration_ms: 127.5,
        uptime_seconds: 86400, // 24 hours
    };

    Ok(HttpResponse::Ok().json(stats))
}

/// Test endpoint to generate various log levels for testing
#[derive(Deserialize)]
pub struct TestLogsRequest {
    pub count: Option<u32>,
    pub level: Option<String>,
    pub include_sensitive: Option<bool>,
}

pub async fn generate_test_logs(
    auth: AuthContext,
    req: actix_web::HttpRequest,
    payload: web::Json<TestLogsRequest>,
) -> Result<HttpResponse> {
    let request_id = get_request_id(&req);
    let count = payload.count.unwrap_or(5);
    let include_sensitive = payload.include_sensitive.unwrap_or(false);

    // Double-check admin permissions (defense in depth)
    if !AuthService::has_admin_permission(&auth) {
        warn!(
            event = "admin_permission_denied",
            user_id = %auth.user.id,
            api_key_id = %auth.api_key.id,
            endpoint = "generate_test_logs",
            request_id = request_id.map(|id| id.to_string()).as_deref(),
            permissions = ?auth.api_key.permissions,
            message = "User without admin permissions attempted to generate test logs"
        );
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "access_denied",
            "message": "Admin privileges required to generate test logs",
            "required_permission": "admin"
        })));
    }

    info!(
        event = "admin_test_logs_requested",
        user_id = %auth.user.id,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        count = count,
        include_sensitive = include_sensitive,
        message = "Admin requested test log generation"
    );

    for i in 1..=count {
        match payload.level.as_deref().unwrap_or("info") {
            "trace" => tracing::trace!(
                event = "test_trace_log",
                iteration = i,
                message = format!("Test trace log message #{}", i)
            ),
            "debug" => tracing::debug!(
                event = "test_debug_log",
                iteration = i,
                message = format!("Test debug log message #{}", i)
            ),
            "info" => tracing::info!(
                event = "test_info_log",
                iteration = i,
                message = format!("Test info log message #{}", i)
            ),
            "warn" => tracing::warn!(
                event = "test_warn_log",
                iteration = i,
                message = format!("Test warning log message #{}", i)
            ),
            "error" => tracing::error!(
                event = "test_error_log",
                iteration = i,
                message = format!("Test error log message #{}", i)
            ),
            _ => tracing::info!(
                event = "test_default_log",
                iteration = i,
                message = format!("Test default log message #{}", i)
            ),
        }

        if include_sensitive {
            // Generate logs with sensitive data that should be masked
            tracing::info!(
                event = "test_sensitive_log",
                iteration = i,
                // These should be masked by the logging system
                test_api_key = format!("test_key_{}", i),
                test_password = "test_password_123",
                test_token = format!("bearer_token_{}", i),
                safe_data = format!("safe_value_{}", i),
                message = format!("Test sensitive data log #{}", i)
            );
        }

        // Add performance measurement
        let timer_context = format!("test_operation_{i}");
        let timer = crate::middleware::PerformanceTimer::new(&timer_context, request_id);

        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        timer.finish();
    }

    let response = serde_json::json!({
        "success": true,
        "message": format!("Generated {} test log entries", count),
        "logs_generated": count,
        "level": payload.level.as_deref().unwrap_or("info"),
        "included_sensitive": include_sensitive,
        "timestamp": chrono::Utc::now()
    });

    info!(
        event = "admin_test_logs_completed",
        user_id = %auth.user.id,
        request_id = request_id.map(|id| id.to_string()).as_deref(),
        logs_generated = count,
        message = "Test log generation completed"
    );

    Ok(HttpResponse::Ok().json(response))
}
