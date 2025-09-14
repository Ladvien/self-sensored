// Simple ENUM definitions for prototype
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;

// Activity Context ENUM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "activity_context", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityContext {
    Resting,
    Walking,
    Running,
    Cycling,
    Exercise,
    Sleeping,
    Sedentary,
    Active,
    PostMeal,
    Stressed,
    Recovery,
}

impl ActivityContext {
    pub fn from_ios_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "resting" => Some(Self::Resting),
            "walking" => Some(Self::Walking),
            "running" => Some(Self::Running),
            "cycling" => Some(Self::Cycling),
            "exercise" | "exercising" => Some(Self::Exercise),
            "sleeping" | "sleep" => Some(Self::Sleeping),
            "sedentary" => Some(Self::Sedentary),
            "active" => Some(Self::Active),
            "post_meal" | "post-meal" | "after_eating" => Some(Self::PostMeal),
            "stressed" | "stress" => Some(Self::Stressed),
            "recovery" | "recovering" => Some(Self::Recovery),
            _ => None,
        }
    }
}

impl fmt::Display for ActivityContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Resting => "resting",
            Self::Walking => "walking",
            Self::Running => "running",
            Self::Cycling => "cycling",
            Self::Exercise => "exercise",
            Self::Sleeping => "sleeping",
            Self::Sedentary => "sedentary",
            Self::Active => "active",
            Self::PostMeal => "post_meal",
            Self::Stressed => "stressed",
            Self::Recovery => "recovery",
        };
        write!(f, "{s}")
    }
}

// Workout Type ENUM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "workout_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkoutType {
    Walking,
    Running,
    Cycling,
    Swimming,
    StrengthTraining,
    Yoga,
    Pilates,
    Hiit,
    Sports,
    Other,
}

impl WorkoutType {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "walking" | "walk" => Self::Walking,
            "running" | "run" => Self::Running,
            "cycling" | "bike" | "biking" => Self::Cycling,
            "swimming" | "swim" => Self::Swimming,
            "strength_training" | "strength" | "weights" => Self::StrengthTraining,
            "yoga" => Self::Yoga,
            "pilates" => Self::Pilates,
            "hiit" | "high_intensity_interval_training" => Self::Hiit,
            "sports" | "sport" => Self::Sports,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for WorkoutType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Walking => "walking",
            Self::Running => "running",
            Self::Cycling => "cycling",
            Self::Swimming => "swimming",
            Self::StrengthTraining => "strength_training",
            Self::Yoga => "yoga",
            Self::Pilates => "pilates",
            Self::Hiit => "hiit",
            Self::Sports => "sports",
            Self::Other => "other",
        };
        write!(f, "{s}")
    }
}

// Job Status ENUM for background processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Processing => "processing",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
        }
    }
}

// Job Type ENUM for background processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "job_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    IngestBatch,
    DataExport,
    DataCleanup,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::IngestBatch => "ingest_batch",
            JobType::DataExport => "data_export",
            JobType::DataCleanup => "data_cleanup",
        }
    }
}
