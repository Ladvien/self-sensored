use std::env;
use tracing_subscriber::{
    fmt::format::{Format, JsonFields},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Configuration for structured logging
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub include_target: bool,
    pub include_thread_ids: bool,
    pub include_file_location: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

impl LoggingConfig {
    /// Create logging configuration from environment variables
    pub fn from_env() -> Self {
        let level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        
        let format = match env::var("LOG_FORMAT").as_deref() {
            Ok("json") => LogFormat::Json,
            Ok("pretty") => LogFormat::Pretty,
            Ok("compact") => LogFormat::Compact,
            _ => LogFormat::Json, // Default to JSON for production
        };

        let include_target = env::var("LOG_INCLUDE_TARGET")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);

        let include_thread_ids = env::var("LOG_INCLUDE_THREAD_IDS")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);

        let include_file_location = env::var("LOG_INCLUDE_FILE_LOCATION")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);

        Self {
            level,
            format,
            include_target,
            include_thread_ids,
            include_file_location,
        }
    }

    /// Initialize the logging system with the configured settings
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let env_filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&self.level))?;

        match self.format {
            LogFormat::Json => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .json()
                            .with_target(self.include_target)
                            .with_thread_ids(self.include_thread_ids)
                            .with_file(self.include_file_location)
                            .with_line_number(self.include_file_location)
                    )
                    .init();
            }
            LogFormat::Pretty => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .pretty()
                            .with_target(self.include_target)
                            .with_thread_ids(self.include_thread_ids)
                            .with_file(self.include_file_location)
                            .with_line_number(self.include_file_location)
                    )
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(
                        tracing_subscriber::fmt::layer()
                            .compact()
                            .with_target(self.include_target)
                            .with_thread_ids(self.include_thread_ids)
                            .with_file(self.include_file_location)
                            .with_line_number(self.include_file_location)
                    )
                    .init();
            }
        }

        Ok(())
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            include_target: true,
            include_thread_ids: false,
            include_file_location: false,
        }
    }
}