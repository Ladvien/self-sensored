use std::env;
use std::io;
use tracing::Level;
use tracing_subscriber::{
    fmt::time::SystemTime, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

/// Logging configuration structure
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: Level,
    /// Enable JSON output format
    pub json_format: bool,
    /// Enable pretty printing for development
    pub pretty_print: bool,
    /// Application name for structured logs
    pub app_name: String,
    /// Application version for structured logs
    pub app_version: String,
    /// Environment (development, staging, production)
    pub environment: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            json_format: true,
            pretty_print: false,
            app_name: "health-export-api".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
        }
    }
}

impl LoggingConfig {
    /// Create logging configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Parse log level from RUST_LOG or LOG_LEVEL
        if let Ok(level_str) = env::var("RUST_LOG").or_else(|_| env::var("LOG_LEVEL")) {
            config.level = parse_log_level(&level_str);
        }

        // JSON format (default: true for production)
        config.json_format = env::var("LOG_JSON")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);

        // Pretty print (default: false, useful for development)
        config.pretty_print = env::var("LOG_PRETTY")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);

        // Application name
        if let Ok(name) = env::var("APP_NAME") {
            config.app_name = name;
        }

        // Environment
        config.environment = env::var("ENVIRONMENT")
            .or_else(|_| env::var("RUST_ENV"))
            .unwrap_or_else(|_| "development".to_string());

        config
    }

    /// Initialize structured logging with the current configuration
    pub fn init(self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Create a more sophisticated default filter
            let level = self.level.as_str();
            let filter_str = format!(
                "{}={},sqlx=warn,actix_web=info,actix_server=info,mio=warn,hyper=warn,h2=warn",
                env!("CARGO_PKG_NAME").replace('-', "_"),
                level
            );
            EnvFilter::new(filter_str)
        });

        if self.json_format {
            // JSON structured logging for production
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_timer(SystemTime)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .flatten_event(false)
                .with_current_span(true)
                .with_span_list(false);

            Registry::default()
                .with(env_filter)
                .with(fmt_layer.with_writer(io::stdout))
                .init();
        } else if self.pretty_print {
            // Pretty printed logs for development
            let fmt_layer = tracing_subscriber::fmt::layer()
                .pretty()
                .with_timer(SystemTime)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true);

            Registry::default()
                .with(env_filter)
                .with(fmt_layer.with_writer(io::stdout))
                .init();
        } else {
            // Compact logs for development/testing
            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_timer(SystemTime)
                .with_target(false)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false);

            Registry::default()
                .with(env_filter)
                .with(fmt_layer.with_writer(io::stdout))
                .init();
        }

        // Log configuration on startup
        tracing::info!(
            event = "logging_initialized",
            config = ?self,
            message = "Structured logging initialized successfully"
        );

        Ok(())
    }

    /// Update log level at runtime (for dynamic configuration)
    pub fn update_log_level(&mut self, new_level: Level) {
        self.level = new_level;
        tracing::info!(
            event = "log_level_changed",
            old_level = %self.level,
            new_level = %new_level,
            message = "Log level updated at runtime"
        );

        // Note: In a real production system, you'd need to reinitialize
        // the subscriber or use a reload handle for runtime updates
    }
}

/// Parse log level from string with fallback to INFO
fn parse_log_level(level_str: &str) -> Level {
    match level_str.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            eprintln!(
                "Warning: Invalid log level '{}', defaulting to 'info'",
                level_str
            );
            Level::INFO
        }
    }
}

/// Runtime log level management for API endpoints
pub struct LogLevelManager {
    current_level: Level,
}

impl LogLevelManager {
    pub fn new(initial_level: Level) -> Self {
        Self {
            current_level: initial_level,
        }
    }

    pub fn get_current_level(&self) -> Level {
        self.current_level
    }

    pub fn set_level(&mut self, new_level: Level) -> Result<(), String> {
        let old_level = self.current_level;
        self.current_level = new_level;

        tracing::info!(
            event = "runtime_log_level_change",
            old_level = %old_level,
            new_level = %new_level,
            timestamp = %chrono::Utc::now(),
            message = "Log level changed via API"
        );

        Ok(())
    }

    pub fn set_level_from_string(&mut self, level_str: &str) -> Result<(), String> {
        let new_level = parse_log_level(level_str);
        self.set_level(new_level)
    }
}

/// Logging context for consistent structured logging
#[derive(Debug, Clone)]
pub struct LogContext {
    pub service_name: String,
    pub version: String,
    pub environment: String,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

impl LogContext {
    pub fn new(service_name: &str, version: &str, environment: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            version: version.to_string(),
            environment: environment.to_string(),
            request_id: None,
            user_id: None,
            session_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Macro for structured application logging with context
#[macro_export]
macro_rules! log_with_context {
    ($level:ident, $context:expr, $event:expr, $($field:ident = $value:expr),*) => {
        tracing::$level!(
            service_name = $context.service_name,
            version = $context.version,
            environment = $context.environment,
            request_id = $context.request_id.as_deref().unwrap_or("unknown"),
            user_id = $context.user_id.as_deref(),
            session_id = $context.session_id.as_deref(),
            event = $event,
            timestamp = %chrono::Utc::now(),
            $($field = $value,)*
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert!(matches!(parse_log_level("trace"), Level::TRACE));
        assert!(matches!(parse_log_level("DEBUG"), Level::DEBUG));
        assert!(matches!(parse_log_level("Info"), Level::INFO));
        assert!(matches!(parse_log_level("WARN"), Level::WARN));
        assert!(matches!(parse_log_level("error"), Level::ERROR));
        assert!(matches!(parse_log_level("invalid"), Level::INFO));
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert!(matches!(config.level, Level::INFO));
        assert!(config.json_format);
        assert!(!config.pretty_print);
        assert_eq!(config.app_name, "health-export-api");
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_log_level_manager() {
        let mut manager = LogLevelManager::new(Level::INFO);
        assert!(matches!(manager.get_current_level(), Level::INFO));

        manager.set_level(Level::DEBUG).unwrap();
        assert!(matches!(manager.get_current_level(), Level::DEBUG));

        manager.set_level_from_string("error").unwrap();
        assert!(matches!(manager.get_current_level(), Level::ERROR));
    }

    #[test]
    fn test_log_context_builder() {
        let context = LogContext::new("test-service", "1.0.0", "test")
            .with_request_id("req-123".to_string())
            .with_user_id("user-456".to_string());

        assert_eq!(context.service_name, "test-service");
        assert_eq!(context.version, "1.0.0");
        assert_eq!(context.environment, "test");
        assert_eq!(context.request_id, Some("req-123".to_string()));
        assert_eq!(context.user_id, Some("user-456".to_string()));
        assert_eq!(context.session_id, None);
    }
}
