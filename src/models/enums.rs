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

// ============================================================================
// REPRODUCTIVE HEALTH ENUMS (HIPAA-Compliant)
// ============================================================================

/// Menstrual Flow Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "menstrual_flow", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MenstrualFlow {
    None,
    Light,
    Medium,
    Heavy,
    Spotting,
}

impl MenstrualFlow {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "none" | "no_flow" => Self::None,
            "light" | "low" => Self::Light,
            "medium" | "moderate" | "normal" => Self::Medium,
            "heavy" | "high" => Self::Heavy,
            "spotting" | "minimal" => Self::Spotting,
            _ => Self::None, // Default to none for unknown values
        }
    }

    /// Get the privacy level for this flow type
    pub fn privacy_level(&self) -> &'static str {
        match self {
            Self::None => "standard",
            _ => "sensitive", // All flow data is sensitive
        }
    }
}

impl fmt::Display for MenstrualFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "none",
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
            Self::Spotting => "spotting",
        };
        write!(f, "{s}")
    }
}

/// Cervical Mucus Quality Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "cervical_mucus_quality", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CervicalMucusQuality {
    Dry,
    Sticky,
    Creamy,
    Watery,
    EggWhite,
}

impl CervicalMucusQuality {
    pub fn from_ios_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dry" => Some(Self::Dry),
            "sticky" | "tacky" => Some(Self::Sticky),
            "creamy" | "lotion" | "lotiony" => Some(Self::Creamy),
            "watery" | "wet" => Some(Self::Watery),
            "egg_white" | "eggwhite" | "stretchy" | "raw_egg_white" => Some(Self::EggWhite),
            _ => None,
        }
    }

    /// Get fertility indicator level
    pub fn fertility_indicator(&self) -> u8 {
        match self {
            Self::Dry => 1,
            Self::Sticky => 2,
            Self::Creamy => 3,
            Self::Watery => 4,
            Self::EggWhite => 5, // Most fertile
        }
    }
}

impl fmt::Display for CervicalMucusQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Dry => "dry",
            Self::Sticky => "sticky",
            Self::Creamy => "creamy",
            Self::Watery => "watery",
            Self::EggWhite => "egg_white",
        };
        write!(f, "{s}")
    }
}

/// Ovulation Test Results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "ovulation_test_result", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OvulationTestResult {
    NotTested,
    Negative,
    Positive,
    Peak,
    High,
}

impl OvulationTestResult {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "not_tested" | "none" | "" => Self::NotTested,
            "negative" | "low" => Self::Negative,
            "positive" => Self::Positive,
            "peak" | "peak_positive" => Self::Peak,
            "high" | "high_positive" => Self::High,
            _ => Self::NotTested,
        }
    }

    /// Get fertility probability score (0-100)
    pub fn fertility_score(&self) -> u8 {
        match self {
            Self::NotTested => 0,
            Self::Negative => 10,
            Self::Positive => 60,
            Self::High => 80,
            Self::Peak => 95,
        }
    }
}

impl fmt::Display for OvulationTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NotTested => "not_tested",
            Self::Negative => "negative",
            Self::Positive => "positive",
            Self::Peak => "peak",
            Self::High => "high",
        };
        write!(f, "{s}")
    }
}

/// Pregnancy Test Results (HIPAA-Critical Data)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "pregnancy_test_result", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PregnancyTestResult {
    NotTested,
    Negative,
    Positive,
    Indeterminate,
}

impl PregnancyTestResult {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "not_tested" | "none" | "" => Self::NotTested,
            "negative" | "not_pregnant" => Self::Negative,
            "positive" | "pregnant" => Self::Positive,
            "indeterminate" | "unclear" | "invalid" => Self::Indeterminate,
            _ => Self::NotTested,
        }
    }

    /// Get privacy level - pregnancy tests are highly sensitive
    pub fn privacy_level(&self) -> &'static str {
        match self {
            Self::NotTested => "standard",
            _ => "highly_sensitive", // All pregnancy results are highly sensitive
        }
    }

    /// Check if result requires enhanced audit logging
    pub fn requires_enhanced_audit(&self) -> bool {
        matches!(self, Self::Positive | Self::Indeterminate)
    }
}

impl fmt::Display for PregnancyTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NotTested => "not_tested",
            Self::Negative => "negative",
            Self::Positive => "positive",
            Self::Indeterminate => "indeterminate",
        };
        write!(f, "{s}")
    }
}

/// Temperature Context for Basal Body Temperature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "temperature_context", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TemperatureContext {
    Basal,
    Fever,
    General,
    Sleeping,
    Environmental,
}

impl TemperatureContext {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basal" | "basal_body_temperature" | "bbt" => Self::Basal,
            "fever" | "illness" | "sick" => Self::Fever,
            "general" | "body_temperature" => Self::General,
            "sleeping" | "sleep" | "night" => Self::Sleeping,
            "environmental" | "ambient" | "room" => Self::Environmental,
            _ => Self::General,
        }
    }

    /// Check if this context is relevant for fertility tracking
    pub fn is_fertility_relevant(&self) -> bool {
        matches!(self, Self::Basal | Self::Sleeping)
    }
}

impl fmt::Display for TemperatureContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Basal => "basal",
            Self::Fever => "fever",
            Self::General => "general",
            Self::Sleeping => "sleeping",
            Self::Environmental => "environmental",
        };
        write!(f, "{s}")
    }
}

// Meditation Type ENUM for mindfulness metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeditationType {
    Guided,
    Unguided,
    Breathing,
    BodyScan,
    Walking,
    Loving,
    Visualization,
    Mantra,
    Mindfulness,
    Other,
}

impl MeditationType {
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guided" => Self::Guided,
            "unguided" => Self::Unguided,
            "breathing" | "breath" | "breathwork" => Self::Breathing,
            "body_scan" | "body scan" | "bodyscan" => Self::BodyScan,
            "walking" | "walking_meditation" => Self::Walking,
            "loving" | "loving_kindness" | "loving kindness" => Self::Loving,
            "visualization" | "visualisation" => Self::Visualization,
            "mantra" => Self::Mantra,
            "mindfulness" => Self::Mindfulness,
            _ => Self::Other,
        }
    }
}

impl fmt::Display for MeditationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Guided => "guided",
            Self::Unguided => "unguided",
            Self::Breathing => "breathing",
            Self::BodyScan => "body_scan",
            Self::Walking => "walking",
            Self::Loving => "loving",
            Self::Visualization => "visualization",
            Self::Mantra => "mantra",
            Self::Mindfulness => "mindfulness",
            Self::Other => "other",
        };
        write!(f, "{s}")
    }
}

// State of Mind for iOS 17+ mental health tracking
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateOfMind {
    VeryUnpleasant,
    Unpleasant,
    SlightlyUnpleasant,
    Neutral,
    SlightlyPleasant,
    Pleasant,
    VeryPleasant,
}

impl StateOfMind {
    pub fn from_valence(valence: f64) -> Self {
        match valence {
            v if v <= -0.75 => Self::VeryUnpleasant,
            v if v <= -0.25 => Self::Unpleasant,
            v if v <= -0.1 => Self::SlightlyUnpleasant,
            v if v < 0.1 => Self::Neutral,
            v if v < 0.25 => Self::SlightlyPleasant,
            v if v < 0.75 => Self::Pleasant,
            _ => Self::VeryPleasant,
        }
    }

    pub fn to_valence(&self) -> f64 {
        match self {
            Self::VeryUnpleasant => -1.0,
            Self::Unpleasant => -0.5,
            Self::SlightlyUnpleasant => -0.2,
            Self::Neutral => 0.0,
            Self::SlightlyPleasant => 0.2,
            Self::Pleasant => 0.5,
            Self::VeryPleasant => 1.0,
        }
    }

    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "very_unpleasant" | "very unpleasant" => Self::VeryUnpleasant,
            "unpleasant" => Self::Unpleasant,
            "slightly_unpleasant" | "slightly unpleasant" => Self::SlightlyUnpleasant,
            "neutral" => Self::Neutral,
            "slightly_pleasant" | "slightly pleasant" => Self::SlightlyPleasant,
            "pleasant" => Self::Pleasant,
            "very_pleasant" | "very pleasant" => Self::VeryPleasant,
            _ => Self::Neutral, // Default to neutral for unknown values
        }
    }
}

impl fmt::Display for StateOfMind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::VeryUnpleasant => "very_unpleasant",
            Self::Unpleasant => "unpleasant",
            Self::SlightlyUnpleasant => "slightly_unpleasant",
            Self::Neutral => "neutral",
            Self::SlightlyPleasant => "slightly_pleasant",
            Self::Pleasant => "pleasant",
            Self::VeryPleasant => "very_pleasant",
        };
        write!(f, "{s}")
    }
}
