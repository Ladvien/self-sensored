// Core data models for the Health Export API

pub mod background_jobs;
pub mod db;
pub mod enums;
pub mod health_metrics;
pub mod ios_models;
pub mod user_characteristics;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Re-export commonly used types
pub use background_jobs::*;
pub use enums::*;
pub use health_metrics::*;
pub use ios_models::*;
pub use user_characteristics::*;

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
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
