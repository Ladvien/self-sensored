// Core data models for the Health Export API

pub mod db;
pub mod health_metrics;
pub mod ios_models;

use chrono::{DateTime, Utc};
use serde::Serialize;

// Re-export commonly used types
pub use health_metrics::*;
pub use ios_models::*;

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }

    pub fn error_with_data(message: String, data: T) -> Self {
        Self {
            success: false,
            data: Some(data),
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}
