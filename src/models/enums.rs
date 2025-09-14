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

// ============================================================================
// SYMPTOM TRACKING ENUMS
// ============================================================================

/// Comprehensive symptom type enumeration for illness tracking and health monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "symptom_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SymptomType {
    // Pain Symptoms
    AbdominalCramps,
    Headache,
    BreastPain,
    PelvicPain,
    ChestTightnessOrPain,
    BackPain,
    MusclePain,
    JointPain,
    ToothPain,
    EyePain,

    // Respiratory Symptoms
    Coughing,
    ShortnessOfBreath,
    Wheezing,
    Congestion,
    RunnyNose,
    Sneezing,
    SoreThroat,
    ChestCongestion,

    // Digestive Symptoms
    Bloating,
    Nausea,
    Vomiting,
    Diarrhea,
    Constipation,
    Heartburn,
    LossOfAppetite,
    ExcessiveHunger,

    // Neurological Symptoms
    Dizziness,
    Fatigue,
    MoodChanges,
    SleepDisturbances,
    MemoryIssues,
    ConcentrationProblems,
    Anxiety,
    Depression,
    Irritability,

    // Cardiovascular Symptoms
    Palpitations,
    RapidHeartRate,
    ChestPain,
    HighBloodPressure,
    ColdHandsOrFeet,

    // Reproductive/Hormonal Symptoms
    HotFlashes,
    NightSweats,
    BreastTenderness,
    VaginalDryness,
    IrregularPeriods,
    HeavyPeriods,
    MoodSwings,

    // General/Systemic Symptoms
    Fever,
    Chills,
    Sweating,
    WeightGain,
    WeightLoss,
    HairLoss,
    DrySkin,
    Rash,
    Itching,
    Swelling,
}

impl SymptomType {
    /// Convert from iOS HealthKit symptom strings
    pub fn from_ios_string(s: &str) -> Option<Self> {
        match s.to_lowercase().replace("-", "_").replace(" ", "_").as_str() {
            // Pain symptoms
            "abdominal_cramps" | "abdominalcramps" | "stomach_pain" => Some(Self::AbdominalCramps),
            "headache" | "head_ache" => Some(Self::Headache),
            "breast_pain" | "breastpain" => Some(Self::BreastPain),
            "pelvic_pain" | "pelvicpain" => Some(Self::PelvicPain),
            "chest_tightness_or_pain" | "chest_tightness" => Some(Self::ChestTightnessOrPain),
            "back_pain" | "backpain" => Some(Self::BackPain),
            "muscle_pain" | "musclepain" | "muscle_ache" => Some(Self::MusclePain),
            "joint_pain" | "jointpain" | "joint_ache" => Some(Self::JointPain),
            "tooth_pain" | "toothpain" | "dental_pain" => Some(Self::ToothPain),
            "eye_pain" | "eyepain" => Some(Self::EyePain),

            // Respiratory symptoms
            "coughing" | "cough" => Some(Self::Coughing),
            "shortness_of_breath" | "shortnessofbreath" | "dyspnea" => Some(Self::ShortnessOfBreath),
            "wheezing" => Some(Self::Wheezing),
            "congestion" | "nasal_congestion" => Some(Self::Congestion),
            "runny_nose" | "runnynose" | "rhinorrhea" => Some(Self::RunnyNose),
            "sneezing" => Some(Self::Sneezing),
            "sore_throat" | "sorethroat" => Some(Self::SoreThroat),
            "chest_congestion" | "chestcongestion" => Some(Self::ChestCongestion),

            // Digestive symptoms
            "bloating" => Some(Self::Bloating),
            "nausea" => Some(Self::Nausea),
            "vomiting" | "throwing_up" => Some(Self::Vomiting),
            "diarrhea" | "loose_stools" => Some(Self::Diarrhea),
            "constipation" => Some(Self::Constipation),
            "heartburn" | "acid_reflux" => Some(Self::Heartburn),
            "loss_of_appetite" | "lossofappetite" | "no_appetite" => Some(Self::LossOfAppetite),
            "excessive_hunger" | "excessivehunger" | "increased_appetite" => Some(Self::ExcessiveHunger),

            // Neurological symptoms
            "dizziness" | "dizzy" => Some(Self::Dizziness),
            "fatigue" | "tired" | "exhaustion" => Some(Self::Fatigue),
            "mood_changes" | "moodchanges" => Some(Self::MoodChanges),
            "sleep_disturbances" | "sleepdisturbances" | "insomnia" => Some(Self::SleepDisturbances),
            "memory_issues" | "memoryissues" | "forgetfulness" => Some(Self::MemoryIssues),
            "concentration_problems" | "concentrationproblems" | "brain_fog" => Some(Self::ConcentrationProblems),
            "anxiety" | "anxious" => Some(Self::Anxiety),
            "depression" | "depressed" | "sad" => Some(Self::Depression),
            "irritability" | "irritable" => Some(Self::Irritability),

            // Cardiovascular symptoms
            "palpitations" | "heart_palpitations" => Some(Self::Palpitations),
            "rapid_heart_rate" | "rapidheartrate" | "tachycardia" => Some(Self::RapidHeartRate),
            "chest_pain" | "chestpain" => Some(Self::ChestPain),
            "high_blood_pressure" | "highbloodpressure" | "hypertension" => Some(Self::HighBloodPressure),
            "cold_hands_or_feet" | "coldhandsorfeet" | "cold_extremities" => Some(Self::ColdHandsOrFeet),

            // Reproductive/Hormonal symptoms
            "hot_flashes" | "hotflashes" | "hot_flash" => Some(Self::HotFlashes),
            "night_sweats" | "nightsweats" => Some(Self::NightSweats),
            "breast_tenderness" | "breasttenderness" => Some(Self::BreastTenderness),
            "vaginal_dryness" | "vaginaldryness" => Some(Self::VaginalDryness),
            "irregular_periods" | "irregularperiods" => Some(Self::IrregularPeriods),
            "heavy_periods" | "heavyperiods" | "menorrhagia" => Some(Self::HeavyPeriods),
            "mood_swings" | "moodswings" => Some(Self::MoodSwings),

            // General/Systemic symptoms
            "fever" | "high_temperature" => Some(Self::Fever),
            "chills" | "shivering" => Some(Self::Chills),
            "sweating" | "excessive_sweating" => Some(Self::Sweating),
            "weight_gain" | "weightgain" => Some(Self::WeightGain),
            "weight_loss" | "weightloss" => Some(Self::WeightLoss),
            "hair_loss" | "hairloss" | "alopecia" => Some(Self::HairLoss),
            "dry_skin" | "dryskin" => Some(Self::DrySkin),
            "rash" | "skin_rash" => Some(Self::Rash),
            "itching" | "itchy" | "pruritus" => Some(Self::Itching),
            "swelling" | "edema" | "inflammation" => Some(Self::Swelling),

            _ => None,
        }
    }

    /// Get symptom category for grouping
    pub fn get_category(&self) -> &'static str {
        match self {
            Self::AbdominalCramps | Self::Headache | Self::BreastPain | Self::PelvicPain
            | Self::ChestTightnessOrPain | Self::BackPain | Self::MusclePain | Self::JointPain
            | Self::ToothPain | Self::EyePain => "pain",

            Self::Coughing | Self::ShortnessOfBreath | Self::Wheezing | Self::Congestion
            | Self::RunnyNose | Self::Sneezing | Self::SoreThroat | Self::ChestCongestion => "respiratory",

            Self::Bloating | Self::Nausea | Self::Vomiting | Self::Diarrhea | Self::Constipation
            | Self::Heartburn | Self::LossOfAppetite | Self::ExcessiveHunger => "digestive",

            Self::Dizziness | Self::Fatigue | Self::MoodChanges | Self::SleepDisturbances
            | Self::MemoryIssues | Self::ConcentrationProblems | Self::Anxiety
            | Self::Depression | Self::Irritability => "neurological",

            Self::Palpitations | Self::RapidHeartRate | Self::ChestPain | Self::HighBloodPressure
            | Self::ColdHandsOrFeet => "cardiovascular",

            Self::HotFlashes | Self::NightSweats | Self::BreastTenderness | Self::VaginalDryness
            | Self::IrregularPeriods | Self::HeavyPeriods | Self::MoodSwings => "reproductive_hormonal",

            Self::Fever | Self::Chills | Self::Sweating | Self::WeightGain | Self::WeightLoss
            | Self::HairLoss | Self::DrySkin | Self::Rash | Self::Itching | Self::Swelling => "general_systemic",
        }
    }

    /// Check if symptom indicates potential medical emergency
    pub fn is_critical(&self) -> bool {
        matches!(self,
            Self::ChestTightnessOrPain | Self::ShortnessOfBreath | Self::ChestPain
            | Self::RapidHeartRate | Self::HighBloodPressure
        )
    }
}

impl fmt::Display for SymptomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AbdominalCramps => "abdominal_cramps",
            Self::Headache => "headache",
            Self::BreastPain => "breast_pain",
            Self::PelvicPain => "pelvic_pain",
            Self::ChestTightnessOrPain => "chest_tightness_or_pain",
            Self::BackPain => "back_pain",
            Self::MusclePain => "muscle_pain",
            Self::JointPain => "joint_pain",
            Self::ToothPain => "tooth_pain",
            Self::EyePain => "eye_pain",
            Self::Coughing => "coughing",
            Self::ShortnessOfBreath => "shortness_of_breath",
            Self::Wheezing => "wheezing",
            Self::Congestion => "congestion",
            Self::RunnyNose => "runny_nose",
            Self::Sneezing => "sneezing",
            Self::SoreThroat => "sore_throat",
            Self::ChestCongestion => "chest_congestion",
            Self::Bloating => "bloating",
            Self::Nausea => "nausea",
            Self::Vomiting => "vomiting",
            Self::Diarrhea => "diarrhea",
            Self::Constipation => "constipation",
            Self::Heartburn => "heartburn",
            Self::LossOfAppetite => "loss_of_appetite",
            Self::ExcessiveHunger => "excessive_hunger",
            Self::Dizziness => "dizziness",
            Self::Fatigue => "fatigue",
            Self::MoodChanges => "mood_changes",
            Self::SleepDisturbances => "sleep_disturbances",
            Self::MemoryIssues => "memory_issues",
            Self::ConcentrationProblems => "concentration_problems",
            Self::Anxiety => "anxiety",
            Self::Depression => "depression",
            Self::Irritability => "irritability",
            Self::Palpitations => "palpitations",
            Self::RapidHeartRate => "rapid_heart_rate",
            Self::ChestPain => "chest_pain",
            Self::HighBloodPressure => "high_blood_pressure",
            Self::ColdHandsOrFeet => "cold_hands_or_feet",
            Self::HotFlashes => "hot_flashes",
            Self::NightSweats => "night_sweats",
            Self::BreastTenderness => "breast_tenderness",
            Self::VaginalDryness => "vaginal_dryness",
            Self::IrregularPeriods => "irregular_periods",
            Self::HeavyPeriods => "heavy_periods",
            Self::MoodSwings => "mood_swings",
            Self::Fever => "fever",
            Self::Chills => "chills",
            Self::Sweating => "sweating",
            Self::WeightGain => "weight_gain",
            Self::WeightLoss => "weight_loss",
            Self::HairLoss => "hair_loss",
            Self::DrySkin => "dry_skin",
            Self::Rash => "rash",
            Self::Itching => "itching",
            Self::Swelling => "swelling",
        };
        write!(f, "{s}")
    }
}

/// Symptom severity levels for medical assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, PartialOrd, Ord)]
#[sqlx(type_name = "symptom_severity", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SymptomSeverity {
    None,
    Mild,
    Moderate,
    Severe,
    Critical,
}

impl SymptomSeverity {
    /// Convert from iOS severity ratings (typically 1-10 scale)
    pub fn from_severity_score(score: Option<i32>) -> Self {
        match score {
            Some(s) if s <= 0 => Self::None,
            Some(s) if s <= 3 => Self::Mild,
            Some(s) if s <= 6 => Self::Moderate,
            Some(s) if s <= 8 => Self::Severe,
            Some(_) => Self::Critical,
            None => Self::Mild, // Default if no severity provided
        }
    }

    /// Convert from iOS string values
    pub fn from_ios_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "none" | "0" => Self::None,
            "mild" | "low" | "1" | "2" | "3" => Self::Mild,
            "moderate" | "medium" | "4" | "5" | "6" => Self::Moderate,
            "severe" | "high" | "7" | "8" => Self::Severe,
            "critical" | "emergency" | "9" | "10" => Self::Critical,
            _ => Self::Mild, // Default for unknown values
        }
    }

    /// Get numeric score (0-10 scale)
    pub fn to_numeric_score(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Mild => 2,
            Self::Moderate => 5,
            Self::Severe => 7,
            Self::Critical => 10,
        }
    }

    /// Check if severity requires immediate medical attention
    pub fn requires_medical_attention(&self) -> bool {
        matches!(self, Self::Severe | Self::Critical)
    }

    /// Check if severity is critical medical emergency level
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical)
    }
}

impl fmt::Display for SymptomSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "none",
            Self::Mild => "mild",
            Self::Moderate => "moderate",
            Self::Severe => "severe",
            Self::Critical => "critical",
        };
        write!(f, "{s}")
    }
}
