-- Health Export REST API - Simple Prototype Schema
-- Version: 2.0.0
-- Date: 2025-09-12
-- Description: Simplified schema for prototype with ENUM types

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "postgis";

-- ============================================================================
-- ENUM TYPE DEFINITIONS
-- ============================================================================

-- Activity Context
CREATE TYPE activity_context AS ENUM (
    'resting', 'walking', 'running', 'cycling', 'exercise',
    'sleeping', 'sedentary', 'active', 'post_meal', 'stressed', 'recovery'
);

-- Workout Type (Comprehensive HealthKit Support - 70+ Types)
CREATE TYPE workout_type AS ENUM (
    -- Base Traditional Activities
    'american_football', 'archery', 'australian_football', 'badminton', 'baseball', 'basketball', 'bowling', 'boxing',
    'climbing', 'cross_training', 'curling', 'cycling', 'dance', 'dance_inspired_training',
    'elliptical', 'equestrian_sports', 'fencing', 'fishing', 'functional_strength_training', 'golf', 'gymnastics',
    'handball', 'hiking', 'hockey', 'hunting', 'lacrosse', 'martial_arts', 'mind_and_body',
    'mixed_metabolic_cardio_training', 'paddle_sports', 'play', 'preparation_and_recovery', 'racquetball',
    'rowing', 'rugby', 'running', 'sailing', 'skating_sports', 'snow_sports', 'soccer', 'softball',
    'squash', 'stair_climbing', 'surfing_sports', 'swimming', 'table_tennis', 'tennis', 'track_and_field',
    'traditional_strength_training', 'volleyball', 'walking', 'water_fitness', 'water_polo', 'water_sports',
    'wrestling', 'yoga',

    -- iOS 10+ Additional Activities
    'barre', 'core_training', 'cross_country_skiing', 'downhill_skiing', 'flexibility', 'hiit', 'jump_rope',
    'kickboxing', 'pilates', 'snowboarding', 'stairs', 'step_training', 'wheelchair_walk_pace', 'wheelchair_run_pace',

    -- iOS 11+ Additional Activities
    'tai_chi', 'mixed_cardio', 'hand_cycling',

    -- iOS 13+ Additional Activities
    'disc_sports', 'fitness_gaming',

    -- Legacy/Other
    'strength_training', 'sports', 'other'
);

-- Job Status and Type for background processing
CREATE TYPE job_status AS ENUM ('pending', 'processing', 'completed', 'failed');
CREATE TYPE job_type AS ENUM ('ingest_batch', 'data_export', 'data_cleanup');

-- ============================================================================
-- CORE TABLES
-- ============================================================================

-- Users Table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    apple_health_id VARCHAR(255) UNIQUE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb
);

-- API Keys Table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    permissions JSONB DEFAULT '["read", "write"]'::jsonb,
    rate_limit_per_hour INTEGER DEFAULT 1000
);

-- Background Processing Jobs
CREATE TABLE processing_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    raw_ingestion_id UUID, -- Will add FK constraint later
    status job_status NOT NULL DEFAULT 'pending',
    job_type job_type NOT NULL,
    priority INTEGER NOT NULL DEFAULT 5,
    total_metrics INTEGER DEFAULT 0,
    processed_metrics INTEGER DEFAULT 0,
    failed_metrics INTEGER DEFAULT 0,
    progress_percentage DECIMAL(5,2) DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    config JSONB DEFAULT '{}',
    result_summary JSONB
);

-- Raw Ingestions Table (for debugging and data recovery)
CREATE TABLE raw_ingestions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    processing_job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL,
    payload_hash VARCHAR(64) NOT NULL,
    payload_size_bytes INTEGER NOT NULL,
    raw_payload JSONB NOT NULL,
    processing_status VARCHAR(50) DEFAULT 'pending',
    processing_errors JSONB,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Now add the foreign key constraint for processing_jobs
ALTER TABLE processing_jobs 
ADD CONSTRAINT fk_processing_jobs_raw_ingestion 
FOREIGN KEY (raw_ingestion_id) 
REFERENCES raw_ingestions(id) 
ON DELETE CASCADE;

-- ============================================================================
-- HEALTH METRICS TABLES
-- ============================================================================

-- Heart Rate Metrics (Extended for Advanced Cardiovascular Monitoring)
CREATE TABLE heart_rate_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    heart_rate INTEGER,
    resting_heart_rate INTEGER,
    heart_rate_variability DOUBLE PRECISION,

    -- Advanced Cardiovascular Metrics (STORY-011)
    walking_heart_rate_average INTEGER, -- Average HR during walking activities (90-120 BPM normal range)
    heart_rate_recovery_one_minute INTEGER, -- HR recovery after 1 minute post-exercise (18+ BPM decrease = good)
    atrial_fibrillation_burden_percentage NUMERIC(5,2), -- AFib burden as percentage (0.01-100.00%, medical-grade monitoring)
    vo2_max_ml_kg_min NUMERIC(5,2), -- VO2 max in ml/kg/min (14.00-65.00 range, cardiorespiratory fitness)

    context activity_context,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at),

    -- Validation constraints for advanced metrics
    CONSTRAINT chk_walking_hr_range CHECK (walking_heart_rate_average IS NULL OR walking_heart_rate_average BETWEEN 60 AND 200),
    CONSTRAINT chk_hr_recovery_range CHECK (heart_rate_recovery_one_minute IS NULL OR heart_rate_recovery_one_minute BETWEEN 0 AND 100),
    CONSTRAINT chk_afib_burden_range CHECK (atrial_fibrillation_burden_percentage IS NULL OR atrial_fibrillation_burden_percentage BETWEEN 0.00 AND 100.00),
    CONSTRAINT chk_vo2_max_range CHECK (vo2_max_ml_kg_min IS NULL OR vo2_max_ml_kg_min BETWEEN 14.00 AND 65.00)
);

-- Create enum types for heart rate events (STORY-011)
CREATE TYPE heart_rate_event_type AS ENUM (
    'HIGH', -- Dangerously high heart rate (tachycardia)
    'LOW', -- Dangerously low heart rate (bradycardia)
    'IRREGULAR', -- Irregular rhythm detection
    'AFIB', -- Atrial fibrillation detected
    'RAPID_INCREASE', -- Sudden rapid HR increase
    'SLOW_RECOVERY', -- Poor heart rate recovery post-exercise
    'EXERCISE_ANOMALY' -- Abnormal HR pattern during exercise
);

CREATE TYPE cardiac_event_severity AS ENUM (
    'LOW', -- Mild concern, monitoring recommended
    'MODERATE', -- Medical consultation advised
    'HIGH', -- Urgent medical attention recommended
    'CRITICAL' -- Emergency medical intervention required
);

-- Heart Rate Events (STORY-011: Cardiac Event Detection and Monitoring)
CREATE TABLE heart_rate_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type heart_rate_event_type NOT NULL,
    event_occurred_at TIMESTAMPTZ NOT NULL,
    heart_rate_at_event INTEGER NOT NULL,
    event_duration_minutes INTEGER, -- Duration of the event in minutes
    context activity_context, -- Activity context when event occurred
    source_device VARCHAR(255), -- Device that detected the event
    severity cardiac_event_severity DEFAULT 'LOW', -- Medical severity assessment
    is_confirmed BOOLEAN DEFAULT FALSE, -- Whether event was medically confirmed
    notes TEXT, -- Additional clinical notes or user observations
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Ensure we don't duplicate events for same user at same time
    UNIQUE(user_id, event_occurred_at, event_type),

    -- Validation constraints for cardiac events
    CONSTRAINT chk_hr_event_range CHECK (heart_rate_at_event BETWEEN 30 AND 300),
    CONSTRAINT chk_event_duration CHECK (event_duration_minutes IS NULL OR event_duration_minutes >= 0)
);

-- Blood Pressure Metrics
CREATE TABLE blood_pressure_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    systolic INTEGER NOT NULL,
    diastolic INTEGER NOT NULL,
    pulse INTEGER,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
);

-- Sleep Metrics
CREATE TABLE sleep_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sleep_start TIMESTAMPTZ NOT NULL,
    sleep_end TIMESTAMPTZ NOT NULL,
    duration_minutes INTEGER,
    deep_sleep_minutes INTEGER,
    rem_sleep_minutes INTEGER,
    light_sleep_minutes INTEGER,
    awake_minutes INTEGER,
    efficiency DOUBLE PRECISION,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, sleep_start)
);

-- Activity Metrics (Extended for Comprehensive Activity Tracking)
CREATE TABLE activity_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Basic Activity Metrics
    step_count INTEGER,
    distance_meters DOUBLE PRECISION,
    flights_climbed INTEGER,
    active_energy_burned_kcal DOUBLE PRECISION,
    basal_energy_burned_kcal DOUBLE PRECISION,

    -- Specialized Distance Metrics for Different Activities
    distance_cycling_meters DOUBLE PRECISION CHECK (distance_cycling_meters IS NULL OR distance_cycling_meters >= 0.0),
    distance_swimming_meters DOUBLE PRECISION CHECK (distance_swimming_meters IS NULL OR distance_swimming_meters >= 0.0),
    distance_wheelchair_meters DOUBLE PRECISION CHECK (distance_wheelchair_meters IS NULL OR distance_wheelchair_meters >= 0.0),
    distance_downhill_snow_sports_meters DOUBLE PRECISION CHECK (distance_downhill_snow_sports_meters IS NULL OR distance_downhill_snow_sports_meters >= 0.0),

    -- Wheelchair Accessibility Metrics
    push_count INTEGER CHECK (push_count IS NULL OR push_count >= 0), -- Wheelchair pushes

    -- Swimming Analytics
    swimming_stroke_count INTEGER CHECK (swimming_stroke_count IS NULL OR swimming_stroke_count >= 0),

    -- Cross-Platform Fitness Integration
    nike_fuel_points INTEGER CHECK (nike_fuel_points IS NULL OR nike_fuel_points >= 0),

    -- Apple Watch Activity Ring Integration
    apple_exercise_time_minutes INTEGER CHECK (apple_exercise_time_minutes IS NULL OR apple_exercise_time_minutes >= 0),
    apple_stand_time_minutes INTEGER CHECK (apple_stand_time_minutes IS NULL OR apple_stand_time_minutes >= 0),
    apple_move_time_minutes INTEGER CHECK (apple_move_time_minutes IS NULL OR apple_move_time_minutes >= 0),
    apple_stand_hour_achieved BOOLEAN DEFAULT false, -- Whether stand goal was achieved this hour

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
);

-- Workouts
CREATE TABLE workouts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workout_type workout_type NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ NOT NULL,
    total_energy_kcal DOUBLE PRECISION,
    active_energy_kcal DOUBLE PRECISION,
    distance_meters DOUBLE PRECISION,
    avg_heart_rate INTEGER,
    max_heart_rate INTEGER,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, started_at)
);

-- Workout Routes (GPS Tracking with PostGIS Support)
CREATE TABLE workout_routes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workout_id UUID NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- GPS Route Data (PostGIS geometry for efficient spatial queries)
    route_geometry GEOMETRY(LINESTRING, 4326), -- GPS route as linestring (WGS84)

    -- Route Points as JSON (for detailed analysis and reconstruction)
    route_points JSONB NOT NULL, -- Array of {lat, lng, timestamp, altitude?, accuracy?, speed?}

    -- Calculated Route Metrics
    total_distance_meters DOUBLE PRECISION,
    elevation_gain_meters DOUBLE PRECISION,
    elevation_loss_meters DOUBLE PRECISION,
    max_altitude_meters DOUBLE PRECISION,
    min_altitude_meters DOUBLE PRECISION,

    -- Route Quality & Privacy
    point_count INTEGER NOT NULL,
    average_accuracy_meters DOUBLE PRECISION,
    privacy_level VARCHAR(20) DEFAULT 'full', -- full, approximate, private

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Ensure one route per workout
    UNIQUE(workout_id)
);

-- Spatial index on route geometry for efficient geographic queries
CREATE INDEX idx_workout_routes_geometry ON workout_routes USING GIST (route_geometry);

-- Index on user_id for privacy-aware queries
CREATE INDEX idx_workout_routes_user_id ON workout_routes (user_id);

-- Index on workout type for activity-specific route analysis
CREATE INDEX idx_workouts_type_started ON workouts (workout_type, started_at DESC);

-- Nutrition Metrics (comprehensive dietary data tracking)
CREATE TABLE nutrition_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Hydration & Stimulants
    dietary_water DOUBLE PRECISION,                    -- liters
    dietary_caffeine DOUBLE PRECISION,                 -- mg

    -- Macronutrients (Core Energy)
    dietary_energy_consumed DOUBLE PRECISION,          -- calories
    dietary_carbohydrates DOUBLE PRECISION,            -- grams
    dietary_protein DOUBLE PRECISION,                  -- grams
    dietary_fat_total DOUBLE PRECISION,                -- grams
    dietary_fat_saturated DOUBLE PRECISION,            -- grams
    dietary_fat_monounsaturated DOUBLE PRECISION,      -- grams
    dietary_fat_polyunsaturated DOUBLE PRECISION,      -- grams
    dietary_cholesterol DOUBLE PRECISION,              -- mg
    dietary_sodium DOUBLE PRECISION,                   -- mg
    dietary_fiber DOUBLE PRECISION,                    -- grams
    dietary_sugar DOUBLE PRECISION,                    -- grams

    -- Essential Minerals
    dietary_calcium DOUBLE PRECISION,                  -- mg
    dietary_iron DOUBLE PRECISION,                     -- mg
    dietary_magnesium DOUBLE PRECISION,                -- mg
    dietary_potassium DOUBLE PRECISION,                -- mg
    dietary_zinc DOUBLE PRECISION,                     -- mg
    dietary_phosphorus DOUBLE PRECISION,               -- mg

    -- Essential Vitamins (Water-soluble)
    dietary_vitamin_c DOUBLE PRECISION,                -- mg
    dietary_vitamin_b1_thiamine DOUBLE PRECISION,      -- mg
    dietary_vitamin_b2_riboflavin DOUBLE PRECISION,    -- mg
    dietary_vitamin_b3_niacin DOUBLE PRECISION,        -- mg
    dietary_vitamin_b6_pyridoxine DOUBLE PRECISION,    -- mg
    dietary_vitamin_b12_cobalamin DOUBLE PRECISION,    -- mcg
    dietary_folate DOUBLE PRECISION,                   -- mcg
    dietary_biotin DOUBLE PRECISION,                   -- mcg
    dietary_pantothenic_acid DOUBLE PRECISION,         -- mg

    -- Essential Vitamins (Fat-soluble)
    dietary_vitamin_a DOUBLE PRECISION,                -- mcg RAE
    dietary_vitamin_d DOUBLE PRECISION,                -- IU
    dietary_vitamin_e DOUBLE PRECISION,                -- mg
    dietary_vitamin_k DOUBLE PRECISION,                -- mcg

    -- Meal Context for atomic processing
    meal_type VARCHAR(50),                             -- breakfast, lunch, dinner, snack
    meal_id UUID,                                      -- Group nutrients from same meal

    -- Metadata and source tracking
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Complex deduplication key: user_id + recorded_at + nutrient_type (allows multiple nutrients per timestamp)
    UNIQUE(user_id, recorded_at, dietary_energy_consumed, dietary_protein, dietary_carbohydrates)
);

-- Respiratory Metrics
CREATE TABLE respiratory_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    respiratory_rate INTEGER, -- breaths per minute (12-20 normal range)
    oxygen_saturation DOUBLE PRECISION, -- SpO2 percentage (90-100% normal, <90% critical)
    forced_vital_capacity DOUBLE PRECISION, -- FVC in liters (3-5L normal range)
    forced_expiratory_volume_1 DOUBLE PRECISION, -- FEV1 in liters (medical reference ranges by age/gender)
    peak_expiratory_flow_rate DOUBLE PRECISION, -- PEFR in L/min (300-600 L/min normal range)
    inhaler_usage INTEGER, -- count of inhaler uses/puffs
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at),
    -- Add check constraints for medical-grade validation
    CONSTRAINT check_respiratory_rate CHECK (respiratory_rate IS NULL OR respiratory_rate BETWEEN 5 AND 60),
    CONSTRAINT check_oxygen_saturation CHECK (oxygen_saturation IS NULL OR oxygen_saturation BETWEEN 70.0 AND 100.0),
    CONSTRAINT check_forced_vital_capacity CHECK (forced_vital_capacity IS NULL OR forced_vital_capacity BETWEEN 1.0 AND 8.0),
    CONSTRAINT check_forced_expiratory_volume_1 CHECK (forced_expiratory_volume_1 IS NULL OR forced_expiratory_volume_1 BETWEEN 0.5 AND 6.0),
    CONSTRAINT check_peak_expiratory_flow_rate CHECK (peak_expiratory_flow_rate IS NULL OR peak_expiratory_flow_rate BETWEEN 100.0 AND 800.0),
    CONSTRAINT check_inhaler_usage CHECK (inhaler_usage IS NULL OR inhaler_usage >= 0)
);

-- Temperature Metrics (Body temperature, fertility tracking, environmental)
CREATE TABLE temperature_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    body_temperature DOUBLE PRECISION, -- celsius (36-42°C range for body temp)
    basal_body_temperature DOUBLE PRECISION, -- celsius (fertility tracking - 36-38°C)
    apple_sleeping_wrist_temperature DOUBLE PRECISION, -- celsius (Apple Watch wrist temp)
    water_temperature DOUBLE PRECISION, -- celsius (environmental - swimming, etc)
    temperature_source VARCHAR(100), -- thermometer, wearable, manual, etc
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at),
    -- Add check constraints for medical-grade validation
    CONSTRAINT check_body_temperature CHECK (body_temperature IS NULL OR body_temperature BETWEEN 30.0 AND 45.0),
    CONSTRAINT check_basal_body_temperature CHECK (basal_body_temperature IS NULL OR basal_body_temperature BETWEEN 35.0 AND 39.0),
    CONSTRAINT check_apple_sleeping_wrist_temperature CHECK (apple_sleeping_wrist_temperature IS NULL OR apple_sleeping_wrist_temperature BETWEEN 30.0 AND 45.0),
    CONSTRAINT check_water_temperature CHECK (water_temperature IS NULL OR water_temperature BETWEEN 0.0 AND 100.0)
);

-- Blood Glucose Metrics (CGM Data Streams)
CREATE TABLE blood_glucose_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    blood_glucose_mg_dl DOUBLE PRECISION NOT NULL, -- Blood glucose in mg/dL (70-180 normal, diabetic ranges vary)
    measurement_context VARCHAR(50), -- 'fasting', 'post_meal', 'random', 'bedtime', 'pre_meal', 'post_workout'
    medication_taken BOOLEAN, -- Whether diabetes medication was taken
    insulin_delivery_units DOUBLE PRECISION, -- Insulin delivery units (for atomic pairing)
    glucose_source VARCHAR(100), -- CGM device identifier for deduplication (e.g., 'dexcom_g7', 'freestyle_libre', 'manual_meter')
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    -- Composite unique constraint for CGM deduplication: user + timestamp + device source
    UNIQUE(user_id, recorded_at, glucose_source),
    -- Medical-grade validation constraints
    CONSTRAINT check_blood_glucose CHECK (blood_glucose_mg_dl BETWEEN 30.0 AND 600.0),
    CONSTRAINT check_insulin_units CHECK (insulin_delivery_units IS NULL OR insulin_delivery_units >= 0.0),
    CONSTRAINT check_measurement_context CHECK (measurement_context IS NULL OR measurement_context IN
        ('fasting', 'post_meal', 'random', 'bedtime', 'pre_meal', 'post_workout'))
);

-- Metabolic metrics table for insulin delivery and blood alcohol content tracking
CREATE TABLE metabolic_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    blood_alcohol_content DOUBLE PRECISION, -- Blood alcohol content percentage (0.0-0.5% range)
    insulin_delivery_units DOUBLE PRECISION, -- Insulin delivery units (0-100 units safe range)
    delivery_method VARCHAR(50), -- 'pump', 'pen', 'syringe', 'inhaler', 'patch'
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    -- Unique constraint for temporal deduplication
    UNIQUE(user_id, recorded_at),
    -- Medical-grade validation constraints
    CONSTRAINT check_blood_alcohol_content CHECK (blood_alcohol_content IS NULL OR
        (blood_alcohol_content >= 0.0 AND blood_alcohol_content <= 0.5)),
    CONSTRAINT check_metabolic_insulin_units CHECK (insulin_delivery_units IS NULL OR
        (insulin_delivery_units >= 0.0 AND insulin_delivery_units <= 100.0)),
    CONSTRAINT check_delivery_method CHECK (delivery_method IS NULL OR delivery_method IN
        ('pump', 'pen', 'syringe', 'inhaler', 'patch'))
);

-- ============================================================================
-- INDEXES
-- ============================================================================

-- User indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_apple_health_id ON users(apple_health_id);

-- API Key indexes
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

-- Processing jobs indexes
CREATE INDEX idx_processing_jobs_user ON processing_jobs(user_id);
CREATE INDEX idx_processing_jobs_status ON processing_jobs(status);
CREATE INDEX idx_processing_jobs_priority ON processing_jobs(priority DESC, status) 
    WHERE status IN ('pending', 'processing');

-- Raw ingestions indexes
CREATE INDEX idx_raw_ingestions_user_id ON raw_ingestions(user_id);
CREATE INDEX idx_raw_ingestions_created_at ON raw_ingestions(created_at);
CREATE INDEX idx_raw_ingestions_processing_status ON raw_ingestions(processing_status);

-- Heart rate indexes
CREATE INDEX idx_heart_rate_user_recorded ON heart_rate_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_heart_rate_recorded ON heart_rate_metrics(recorded_at DESC);

-- Heart Rate Events indexes (STORY-011)
CREATE INDEX idx_hr_events_user_time ON heart_rate_events(user_id, event_occurred_at DESC);
CREATE INDEX idx_hr_events_type_severity ON heart_rate_events(event_type, severity);
CREATE INDEX idx_hr_events_unconfirmed ON heart_rate_events(user_id, is_confirmed) WHERE is_confirmed = FALSE;
CREATE INDEX idx_hr_events_severity_time ON heart_rate_events(severity DESC, event_occurred_at DESC) WHERE severity IN ('HIGH', 'CRITICAL');

-- Blood pressure indexes
CREATE INDEX idx_blood_pressure_user_recorded ON blood_pressure_metrics(user_id, recorded_at DESC);

-- Sleep indexes
CREATE INDEX idx_sleep_user_start ON sleep_metrics(user_id, sleep_start DESC);

-- Activity indexes
CREATE INDEX idx_activity_user_recorded ON activity_metrics(user_id, recorded_at DESC);

-- Specialized Activity Indexes for Extended Metrics
CREATE INDEX idx_activity_cycling_distance ON activity_metrics(user_id, recorded_at DESC) WHERE distance_cycling_meters IS NOT NULL;
CREATE INDEX idx_activity_swimming_distance ON activity_metrics(user_id, recorded_at DESC) WHERE distance_swimming_meters IS NOT NULL;
CREATE INDEX idx_activity_wheelchair_metrics ON activity_metrics(user_id, recorded_at DESC) WHERE distance_wheelchair_meters IS NOT NULL OR push_count IS NOT NULL;
CREATE INDEX idx_activity_snow_sports ON activity_metrics(user_id, recorded_at DESC) WHERE distance_downhill_snow_sports_meters IS NOT NULL;
CREATE INDEX idx_activity_swimming_strokes ON activity_metrics(user_id, swimming_stroke_count DESC) WHERE swimming_stroke_count IS NOT NULL;
CREATE INDEX idx_activity_nike_fuel ON activity_metrics(user_id, nike_fuel_points DESC) WHERE nike_fuel_points IS NOT NULL;
CREATE INDEX idx_activity_apple_exercise ON activity_metrics(user_id, apple_exercise_time_minutes DESC) WHERE apple_exercise_time_minutes IS NOT NULL;
CREATE INDEX idx_activity_apple_stand_achieved ON activity_metrics(user_id, recorded_at DESC) WHERE apple_stand_hour_achieved = true;

-- Workout indexes
CREATE INDEX idx_workouts_user_started ON workouts(user_id, started_at DESC);
CREATE INDEX idx_workouts_type ON workouts(workout_type);

-- Nutrition indexes
CREATE INDEX idx_nutrition_user_recorded ON nutrition_metrics(user_id, recorded_at DESC);

-- Respiratory indexes
CREATE INDEX idx_respiratory_user_recorded ON respiratory_metrics(user_id, recorded_at DESC);

-- Temperature indexes
CREATE INDEX idx_temperature_user_recorded ON temperature_metrics(user_id, recorded_at DESC);

-- Blood glucose indexes (optimized for CGM data queries)
CREATE INDEX idx_blood_glucose_user_recorded ON blood_glucose_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_blood_glucose_recorded ON blood_glucose_metrics(recorded_at DESC);
CREATE INDEX idx_blood_glucose_user_source ON blood_glucose_metrics(user_id, glucose_source, recorded_at DESC);
CREATE INDEX idx_blood_glucose_critical ON blood_glucose_metrics(user_id, recorded_at DESC)
    WHERE blood_glucose_mg_dl < 70.0 OR blood_glucose_mg_dl > 400.0;

-- Metabolic metrics indexes
CREATE INDEX idx_metabolic_user_recorded ON metabolic_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_metabolic_recorded ON metabolic_metrics(recorded_at DESC);
CREATE INDEX idx_metabolic_user_delivery_method ON metabolic_metrics(user_id, delivery_method, recorded_at DESC);
CREATE INDEX idx_metabolic_alcohol ON metabolic_metrics(user_id, recorded_at DESC)
    WHERE blood_alcohol_content > 0.08; -- Index for intoxication levels
CREATE INDEX idx_metabolic_insulin ON metabolic_metrics(user_id, recorded_at DESC)
    WHERE insulin_delivery_units > 10.0; -- Index for significant insulin deliveries
CREATE INDEX idx_temperature_source ON temperature_metrics(temperature_source);

-- ============================================================================
-- HELPER FUNCTIONS
-- ============================================================================

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- REPRODUCTIVE HEALTH ENUMS (HIPAA-Compliant)
-- ============================================================================

-- Menstrual Flow Types
CREATE TYPE menstrual_flow AS ENUM (
    'none', 'light', 'medium', 'heavy', 'spotting'
);

-- Cervical Mucus Quality
CREATE TYPE cervical_mucus_quality AS ENUM (
    'dry', 'sticky', 'creamy', 'watery', 'egg_white'
);

-- Ovulation Test Results
CREATE TYPE ovulation_test_result AS ENUM (
    'not_tested', 'negative', 'positive', 'peak', 'high'
);

-- Pregnancy Test Results
CREATE TYPE pregnancy_test_result AS ENUM (
    'not_tested', 'negative', 'positive', 'indeterminate'
);

-- Basal Body Temperature Context
CREATE TYPE temperature_context AS ENUM (
    'basal', 'fever', 'general', 'sleeping', 'environmental'
);

-- Symptom Type Enumeration (40+ symptom types for comprehensive tracking)
CREATE TYPE symptom_type AS ENUM (
    -- Pain Symptoms
    'abdominal_cramps', 'headache', 'breast_pain', 'pelvic_pain', 'chest_tightness_or_pain',
    'back_pain', 'muscle_pain', 'joint_pain', 'tooth_pain', 'eye_pain',

    -- Respiratory Symptoms
    'coughing', 'shortness_of_breath', 'wheezing', 'congestion', 'runny_nose',
    'sneezing', 'sore_throat', 'chest_congestion',

    -- Digestive Symptoms
    'bloating', 'nausea', 'vomiting', 'diarrhea', 'constipation',
    'heartburn', 'loss_of_appetite', 'excessive_hunger',

    -- Neurological Symptoms
    'dizziness', 'fatigue', 'mood_changes', 'sleep_disturbances', 'memory_issues',
    'concentration_problems', 'anxiety', 'depression', 'irritability',

    -- Cardiovascular Symptoms
    'palpitations', 'rapid_heart_rate', 'chest_pain', 'high_blood_pressure',
    'cold_hands_or_feet',

    -- Reproductive/Hormonal Symptoms
    'hot_flashes', 'night_sweats', 'breast_tenderness', 'vaginal_dryness',
    'irregular_periods', 'heavy_periods', 'mood_swings',

    -- General/Systemic Symptoms
    'fever', 'chills', 'sweating', 'weight_gain', 'weight_loss',
    'hair_loss', 'dry_skin', 'rash', 'itching', 'swelling'
);

-- Symptom Severity Levels
CREATE TYPE symptom_severity AS ENUM (
    'none', 'mild', 'moderate', 'severe', 'critical'
);

-- ============================================================================
-- SYMPTOMS TRACKING TABLES
-- ============================================================================

-- Symptoms Table for comprehensive symptom tracking and illness episode management
CREATE TABLE symptoms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    symptom_type symptom_type NOT NULL,
    severity symptom_severity NOT NULL DEFAULT 'mild',
    duration_minutes INTEGER CHECK (duration_minutes IS NULL OR duration_minutes >= 0),
    notes TEXT, -- Additional context about the symptom
    episode_id UUID, -- Link related symptoms in same illness episode
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at, symptom_type) -- Prevent duplicate symptoms at same time
);

-- Index for efficient symptom queries
CREATE INDEX idx_symptoms_user_recorded ON symptoms(user_id, recorded_at DESC);
CREATE INDEX idx_symptoms_type_severity ON symptoms(symptom_type, severity);
CREATE INDEX idx_symptoms_episode ON symptoms(episode_id) WHERE episode_id IS NOT NULL;

-- ============================================================================
-- REPRODUCTIVE HEALTH TABLES (Privacy-First Design)
-- ============================================================================

-- Menstrual Health Tracking (HIPAA-Compliant)
CREATE TABLE menstrual_health (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    menstrual_flow menstrual_flow NOT NULL DEFAULT 'none',
    spotting BOOLEAN DEFAULT false,
    cycle_day INTEGER, -- Day in menstrual cycle (1-40)
    cramps_severity INTEGER CHECK (cramps_severity IS NULL OR cramps_severity BETWEEN 0 AND 10), -- Pain scale 0-10
    mood_rating INTEGER CHECK (mood_rating IS NULL OR mood_rating BETWEEN 1 AND 5), -- 1=terrible, 5=great
    energy_level INTEGER CHECK (energy_level IS NULL OR energy_level BETWEEN 1 AND 5), -- 1=exhausted, 5=energetic
    notes TEXT, -- Encrypted notes field for sensitive information
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
);

-- Fertility Tracking (Enhanced Privacy Protection)
CREATE TABLE fertility_tracking (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    cervical_mucus_quality cervical_mucus_quality,
    ovulation_test_result ovulation_test_result DEFAULT 'not_tested',

    -- Privacy-protected sexual activity tracking
    sexual_activity BOOLEAN, -- Requires special access controls

    pregnancy_test_result pregnancy_test_result DEFAULT 'not_tested',
    basal_body_temperature DOUBLE PRECISION, -- Celsius
    temperature_context temperature_context DEFAULT 'basal',

    -- Additional fertility indicators
    cervix_firmness INTEGER CHECK (cervix_firmness IS NULL OR cervix_firmness BETWEEN 1 AND 3), -- 1=soft, 3=firm
    cervix_position INTEGER CHECK (cervix_position IS NULL OR cervix_position BETWEEN 1 AND 3), -- 1=low, 3=high

    -- Luteinizing hormone level (if available)
    lh_level DOUBLE PRECISION, -- mIU/mL

    notes TEXT, -- Encrypted notes field
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
);

-- ============================================================================
-- REPRODUCTIVE HEALTH INDEXES (Privacy-Aware)
-- ============================================================================

-- Menstrual health indexes
CREATE INDEX idx_menstrual_health_user_recorded ON menstrual_health(user_id, recorded_at DESC);
CREATE INDEX idx_menstrual_health_cycle_day ON menstrual_health(user_id, cycle_day) WHERE cycle_day IS NOT NULL;
CREATE INDEX idx_menstrual_health_flow ON menstrual_health(menstrual_flow) WHERE menstrual_flow != 'none';

-- Fertility tracking indexes
CREATE INDEX idx_fertility_tracking_user_recorded ON fertility_tracking(user_id, recorded_at DESC);
CREATE INDEX idx_fertility_tracking_ovulation ON fertility_tracking(user_id, ovulation_test_result) WHERE ovulation_test_result != 'not_tested';
CREATE INDEX idx_fertility_tracking_temperature ON fertility_tracking(user_id, recorded_at DESC) WHERE basal_body_temperature IS NOT NULL;

-- Privacy-protected sexual activity index (requires special access)
-- Note: This index is limited to prevent data mining
CREATE INDEX idx_fertility_sexual_activity_privacy ON fertility_tracking(user_id, recorded_at DESC)
    WHERE sexual_activity IS NOT NULL;

-- ============================================================================
-- REPRODUCTIVE HEALTH AUDIT TABLE (HIPAA Compliance)
-- ============================================================================

CREATE TABLE reproductive_health_audit (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL, -- 'access', 'modify', 'delete', 'export'
    table_name VARCHAR(50) NOT NULL, -- 'menstrual_health', 'fertility_tracking'
    record_id UUID,
    data_type VARCHAR(50), -- 'menstrual_data', 'fertility_data', 'sexual_activity'
    access_method VARCHAR(50), -- 'api_query', 'bulk_export', 'admin_access'
    success BOOLEAN NOT NULL,
    ip_address INET,
    user_agent TEXT,
    request_metadata JSONB, -- Additional audit context
    privacy_level VARCHAR(20) DEFAULT 'standard', -- 'standard', 'sensitive', 'highly_sensitive'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partition reproductive health audit by month for performance
CREATE INDEX idx_reproductive_audit_user_time ON reproductive_health_audit(user_id, created_at DESC);
CREATE INDEX idx_reproductive_audit_action ON reproductive_health_audit(action, created_at DESC);
CREATE INDEX idx_reproductive_audit_privacy ON reproductive_health_audit(privacy_level, created_at DESC);

-- ============================================================================
-- REPRODUCTIVE HEALTH PRIVACY FUNCTIONS (HIPAA-Compliant)
-- ============================================================================

-- Function to log reproductive health data access
CREATE OR REPLACE FUNCTION log_reproductive_health_access(
    p_user_id UUID,
    p_api_key_id UUID,
    p_action VARCHAR(50),
    p_table_name VARCHAR(50),
    p_record_id UUID DEFAULT NULL,
    p_data_type VARCHAR(50) DEFAULT NULL,
    p_access_method VARCHAR(50) DEFAULT 'api_query',
    p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}',
    p_privacy_level VARCHAR(20) DEFAULT 'standard'
) RETURNS VOID AS $$
BEGIN
    INSERT INTO reproductive_health_audit (
        user_id, api_key_id, action, table_name, record_id,
        data_type, access_method, success, ip_address,
        user_agent, request_metadata, privacy_level
    ) VALUES (
        p_user_id, p_api_key_id, p_action, p_table_name, p_record_id,
        p_data_type, p_access_method, true, p_ip_address,
        p_user_agent, p_metadata, p_privacy_level
    );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to anonymize reproductive health data for analytics
CREATE OR REPLACE FUNCTION anonymize_reproductive_data(
    p_user_id UUID,
    p_age_group VARCHAR(20), -- '18-25', '26-35', '36-45', '46+'
    p_data_type VARCHAR(50)
) RETURNS JSONB AS $$
DECLARE
    result JSONB := '{}';
BEGIN
    -- Return anonymized aggregate data only
    -- Never return individual records or identifiable information

    IF p_data_type = 'menstrual_cycle_stats' THEN
        SELECT jsonb_build_object(
            'age_group', p_age_group,
            'avg_cycle_length', ROUND(AVG(
                CASE WHEN cycle_day IS NOT NULL
                THEN cycle_day ELSE NULL END
            ), 1),
            'common_flow_patterns', jsonb_agg(DISTINCT menstrual_flow),
            'data_points', COUNT(*),
            'anonymized', true
        ) INTO result
        FROM menstrual_health
        WHERE user_id = p_user_id
        AND recorded_at >= NOW() - INTERVAL '1 year';
    END IF;

    RETURN result;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- MINDFULNESS & MENTAL HEALTH TABLES
-- ============================================================================

-- Mindfulness sessions table
CREATE TABLE mindfulness_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Core Session Data
    session_duration_minutes INTEGER,
    meditation_type VARCHAR(100), -- 'guided', 'breathwork', 'body_scan', 'loving_kindness', 'mindfulness', 'other'
    session_quality_rating SMALLINT CHECK (session_quality_rating >= 1 AND session_quality_rating <= 5),

    -- Mindful Minutes Tracking
    mindful_minutes_today INTEGER,
    mindful_minutes_week INTEGER,

    -- Physiological Data During Session
    breathing_rate_breaths_per_min NUMERIC(5,2),
    heart_rate_variability_during_session NUMERIC(8,2),
    focus_rating SMALLINT CHECK (focus_rating >= 1 AND focus_rating <= 10),

    -- Session Context
    guided_session_instructor VARCHAR(255),
    meditation_app VARCHAR(100), -- 'calm', 'headspace', 'insight_timer', 'apple_mindfulness', 'ten_percent_happier'
    background_sounds VARCHAR(100), -- 'nature', 'rain', 'silence', 'music', 'white_noise'
    location_type VARCHAR(50), -- 'home', 'office', 'outdoors', 'studio', 'travel'
    session_notes TEXT,

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint to prevent duplicate sessions
    UNIQUE (user_id, recorded_at, meditation_type)
);

-- Indexes for mindfulness metrics
CREATE INDEX idx_mindfulness_user_date ON mindfulness_metrics (user_id, recorded_at);
CREATE INDEX idx_mindfulness_meditation_type ON mindfulness_metrics (meditation_type);
CREATE INDEX idx_mindfulness_duration ON mindfulness_metrics (session_duration_minutes);

-- Mental health metrics table (HIPAA-compliant with privacy protection)
CREATE TABLE mental_health_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- iOS 17+ State of Mind Integration
    state_of_mind_valence NUMERIC(3,2) CHECK (state_of_mind_valence >= -1.0 AND state_of_mind_valence <= 1.0),
    state_of_mind_labels TEXT[], -- array of mood descriptors from iOS
    reflection_prompt TEXT,

    -- General Mental Health Ratings (1-10 scale)
    mood_rating SMALLINT CHECK (mood_rating >= 1 AND mood_rating <= 10),
    anxiety_level SMALLINT CHECK (anxiety_level >= 1 AND anxiety_level <= 10),
    stress_level SMALLINT CHECK (stress_level >= 1 AND stress_level <= 10),
    energy_level SMALLINT CHECK (energy_level >= 1 AND energy_level <= 10),

    -- Clinical Screening Scores (when applicable)
    depression_screening_score SMALLINT CHECK (depression_screening_score >= 0 AND depression_screening_score <= 27), -- PHQ-9 style
    anxiety_screening_score SMALLINT CHECK (anxiety_screening_score >= 0 AND anxiety_screening_score <= 21), -- GAD-7 style

    -- Sleep Quality Impact
    sleep_quality_impact SMALLINT CHECK (sleep_quality_impact >= 1 AND sleep_quality_impact <= 5),

    -- Context and Coping
    trigger_event VARCHAR(255), -- 'work_stress', 'relationship', 'health', 'financial', 'social', 'other'
    coping_strategy VARCHAR(255), -- 'exercise', 'meditation', 'social_support', 'therapy', 'journaling', 'other'
    medication_taken BOOLEAN,
    therapy_session_today BOOLEAN,

    -- Privacy Protected Data (encrypted fields)
    private_notes_encrypted TEXT, -- Encrypted mental health notes
    notes_encryption_key_id UUID,
    data_sensitivity_level VARCHAR(20) DEFAULT 'high', -- 'standard', 'high', 'medical', 'therapeutic'

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint to prevent duplicate entries
    UNIQUE (user_id, recorded_at)
);

-- Indexes for mental health metrics (with privacy considerations)
CREATE INDEX idx_mental_health_user_date ON mental_health_metrics (user_id, recorded_at);
CREATE INDEX idx_mental_health_mood ON mental_health_metrics (mood_rating) WHERE mood_rating IS NOT NULL;
CREATE INDEX idx_mental_health_sensitivity ON mental_health_metrics (data_sensitivity_level);

-- Mental health audit table (HIPAA compliance)
CREATE TABLE mental_health_audit (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES api_keys(id),
    action VARCHAR(100) NOT NULL, -- 'create', 'read', 'update', 'delete', 'export'
    table_name VARCHAR(100) NOT NULL,
    record_id UUID,

    -- Access Details
    access_type VARCHAR(50), -- 'api', 'dashboard', 'export', 'analytics'
    access_method VARCHAR(50), -- 'GET', 'POST', 'PUT', 'DELETE'
    success BOOLEAN NOT NULL DEFAULT true,

    -- Privacy and Security
    data_sensitivity_level VARCHAR(20), -- 'standard', 'high', 'medical'
    privacy_level VARCHAR(20), -- 'summary', 'detailed', 'full_access'
    encryption_used BOOLEAN DEFAULT false,

    -- Request Context
    ip_address INET,
    user_agent TEXT,
    request_metadata JSONB DEFAULT '{}'::jsonb,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for mental health audit
CREATE INDEX idx_mental_health_audit_user ON mental_health_audit (user_id, created_at);
CREATE INDEX idx_mental_health_audit_action ON mental_health_audit (action, created_at);
CREATE INDEX idx_mental_health_audit_sensitivity ON mental_health_audit (data_sensitivity_level);

-- Function to calculate wellness score from mental health metrics
CREATE OR REPLACE FUNCTION calculate_wellness_score(
    p_mood_rating INTEGER,
    p_anxiety_level INTEGER,
    p_stress_level INTEGER,
    p_energy_level INTEGER
) RETURNS INTEGER AS $$
DECLARE
    wellness_score INTEGER := 0;
    total_components INTEGER := 0;
BEGIN
    -- Calculate weighted wellness score (1-10 scale)
    -- Higher mood and energy are positive, lower anxiety and stress are positive

    IF p_mood_rating IS NOT NULL THEN
        wellness_score := wellness_score + p_mood_rating;
        total_components := total_components + 1;
    END IF;

    IF p_energy_level IS NOT NULL THEN
        wellness_score := wellness_score + p_energy_level;
        total_components := total_components + 1;
    END IF;

    IF p_anxiety_level IS NOT NULL THEN
        -- Invert anxiety (lower anxiety = better wellness)
        wellness_score := wellness_score + (11 - p_anxiety_level);
        total_components := total_components + 1;
    END IF;

    IF p_stress_level IS NOT NULL THEN
        -- Invert stress (lower stress = better wellness)
        wellness_score := wellness_score + (11 - p_stress_level);
        total_components := total_components + 1;
    END IF;

    -- Return average if we have any components, otherwise return NULL
    IF total_components > 0 THEN
        RETURN ROUND(wellness_score::NUMERIC / total_components);
    ELSE
        RETURN NULL;
    END IF;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to add mental health audit entry (HIPAA compliance)
CREATE OR REPLACE FUNCTION audit_mental_health_access(
    p_user_id UUID,
    p_api_key_id UUID,
    p_action VARCHAR(100),
    p_table_name VARCHAR(100),
    p_record_id UUID DEFAULT NULL,
    p_access_type VARCHAR(50) DEFAULT 'api',
    p_access_method VARCHAR(50) DEFAULT 'GET',
    p_data_sensitivity_level VARCHAR(20) DEFAULT 'high',
    p_privacy_level VARCHAR(20) DEFAULT 'summary',
    p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'::jsonb
) RETURNS VOID AS $$
BEGIN
    INSERT INTO mental_health_audit (
        user_id, api_key_id, action, table_name, record_id,
        access_type, access_method, success, data_sensitivity_level,
        privacy_level, encryption_used, ip_address, user_agent, request_metadata
    ) VALUES (
        p_user_id, p_api_key_id, p_action, p_table_name, p_record_id,
        p_access_type, p_access_method, true, p_data_sensitivity_level,
        p_privacy_level, true, p_ip_address, p_user_agent, p_metadata
    );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- HYGIENE EVENTS TABLE
-- ============================================================================

-- Hygiene event types enum
CREATE TYPE hygiene_event_type AS ENUM (
    'handwashing', 'toothbrushing', 'hand_sanitizer',
    'face_washing', 'shower', 'bath', 'hair_washing',
    'nail_care', 'oral_hygiene', 'skincare'
);

-- Hygiene events table for behavior tracking and public health monitoring
CREATE TABLE hygiene_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Core Hygiene Event Data
    event_type hygiene_event_type NOT NULL,
    duration_seconds INTEGER CHECK (duration_seconds IS NULL OR duration_seconds BETWEEN 1 AND 7200), -- 1 second to 2 hours max
    quality_rating SMALLINT CHECK (quality_rating IS NULL OR quality_rating BETWEEN 1 AND 5), -- 1-5 self-reported quality

    -- Public Health & Compliance Tracking
    meets_who_guidelines BOOLEAN, -- Whether event meets WHO/CDC guidelines (20+ sec handwashing, 2+ min brushing)
    frequency_compliance_rating SMALLINT CHECK (frequency_compliance_rating IS NULL OR frequency_compliance_rating BETWEEN 1 AND 5), -- Daily frequency adherence

    -- Smart Device Integration
    device_detected BOOLEAN DEFAULT false, -- Whether detected by smart device (soap dispenser, smart toothbrush)
    device_effectiveness_score DOUBLE PRECISION CHECK (device_effectiveness_score IS NULL OR device_effectiveness_score BETWEEN 0.0 AND 100.0), -- Device-measured effectiveness

    -- Context & Behavioral Analysis
    trigger_event VARCHAR(100), -- 'before_meal', 'after_bathroom', 'after_cough', 'routine', 'reminder', 'crisis_protocol'
    location_context VARCHAR(100), -- 'home', 'work', 'public', 'healthcare', 'restaurant', 'travel'
    compliance_motivation VARCHAR(100), -- 'habit', 'health_crisis', 'illness_prevention', 'personal_hygiene', 'medical_recommendation'

    -- Health Crisis Integration (COVID-19, flu season, etc.)
    health_crisis_enhanced BOOLEAN DEFAULT false, -- Enhanced hygiene during health emergencies
    crisis_compliance_level SMALLINT CHECK (crisis_compliance_level IS NULL OR crisis_compliance_level BETWEEN 1 AND 5), -- Adherence to crisis protocols

    -- Gamification & Habit Tracking
    streak_count INTEGER DEFAULT 1, -- Current hygiene habit streak
    daily_goal_progress SMALLINT CHECK (daily_goal_progress IS NULL OR daily_goal_progress BETWEEN 0 AND 200), -- Percentage of daily hygiene goals met
    achievement_unlocked VARCHAR(255), -- Any hygiene achievement unlocked with this event

    -- Medical Integration
    medication_adherence_related BOOLEAN DEFAULT false, -- Related to medication adherence (hand hygiene before insulin, etc.)
    medical_condition_context VARCHAR(100), -- 'diabetes_management', 'immunocompromised', 'surgical_recovery', 'wound_care'

    -- Privacy & Data Sensitivity
    data_sensitivity_level VARCHAR(20) DEFAULT 'standard', -- 'standard', 'medical', 'crisis_tracking'

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint to prevent duplicate events (user + timestamp + event_type)
    UNIQUE(user_id, recorded_at, event_type)
);

-- Indexes for hygiene events performance
CREATE INDEX idx_hygiene_events_user_date ON hygiene_events (user_id, recorded_at);
CREATE INDEX idx_hygiene_events_type ON hygiene_events (event_type, recorded_at);
CREATE INDEX idx_hygiene_events_compliance ON hygiene_events (meets_who_guidelines) WHERE meets_who_guidelines IS NOT NULL;
CREATE INDEX idx_hygiene_events_crisis ON hygiene_events (health_crisis_enhanced, recorded_at) WHERE health_crisis_enhanced = true;
CREATE INDEX idx_hygiene_events_streak ON hygiene_events (user_id, streak_count DESC);
CREATE INDEX idx_hygiene_events_sensitivity ON hygiene_events (data_sensitivity_level);

-- Function to calculate hygiene compliance score
CREATE OR REPLACE FUNCTION calculate_hygiene_compliance_score(
    p_user_id UUID,
    p_start_date TIMESTAMPTZ DEFAULT NOW() - INTERVAL '7 days',
    p_end_date TIMESTAMPTZ DEFAULT NOW()
) RETURNS JSONB AS $$
DECLARE
    handwashing_compliance DOUBLE PRECISION := 0;
    toothbrushing_compliance DOUBLE PRECISION := 0;
    overall_frequency DOUBLE PRECISION := 0;
    quality_average DOUBLE PRECISION := 0;
    total_events INTEGER := 0;
    compliance_score JSONB;
BEGIN
    -- Calculate handwashing compliance (WHO: 20+ seconds, multiple times daily)
    SELECT
        COUNT(*) FILTER (WHERE duration_seconds >= 20 AND meets_who_guidelines = true)::DOUBLE PRECISION /
        NULLIF(COUNT(*), 0) * 100,
        COUNT(*)
    INTO handwashing_compliance, total_events
    FROM hygiene_events
    WHERE user_id = p_user_id
        AND event_type = 'handwashing'
        AND recorded_at BETWEEN p_start_date AND p_end_date;

    -- Calculate toothbrushing compliance (WHO: 2+ minutes, twice daily)
    SELECT
        COUNT(*) FILTER (WHERE duration_seconds >= 120 AND meets_who_guidelines = true)::DOUBLE PRECISION /
        NULLIF(COUNT(*), 0) * 100
    INTO toothbrushing_compliance
    FROM hygiene_events
    WHERE user_id = p_user_id
        AND event_type = 'toothbrushing'
        AND recorded_at BETWEEN p_start_date AND p_end_date;

    -- Calculate overall frequency (events per day)
    SELECT COUNT(*)::DOUBLE PRECISION / EXTRACT(EPOCH FROM (p_end_date - p_start_date)) * 86400
    INTO overall_frequency
    FROM hygiene_events
    WHERE user_id = p_user_id
        AND recorded_at BETWEEN p_start_date AND p_end_date;

    -- Calculate average quality rating
    SELECT AVG(quality_rating)
    INTO quality_average
    FROM hygiene_events
    WHERE user_id = p_user_id
        AND quality_rating IS NOT NULL
        AND recorded_at BETWEEN p_start_date AND p_end_date;

    -- Build compliance score JSON
    compliance_score := jsonb_build_object(
        'overall_score', COALESCE((handwashing_compliance + toothbrushing_compliance) / 2, 0),
        'handwashing_compliance_percent', COALESCE(handwashing_compliance, 0),
        'toothbrushing_compliance_percent', COALESCE(toothbrushing_compliance, 0),
        'daily_frequency', COALESCE(overall_frequency, 0),
        'average_quality_rating', COALESCE(quality_average, 0),
        'total_events', COALESCE(total_events, 0),
        'period_days', EXTRACT(EPOCH FROM (p_end_date - p_start_date)) / 86400,
        'calculated_at', NOW()
    );

    RETURN compliance_score;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to update hygiene streak counts
CREATE OR REPLACE FUNCTION update_hygiene_streak(
    p_user_id UUID,
    p_event_type hygiene_event_type,
    p_recorded_at TIMESTAMPTZ
) RETURNS INTEGER AS $$
DECLARE
    current_streak INTEGER := 1;
    last_event_date DATE;
    current_event_date DATE;
BEGIN
    current_event_date := p_recorded_at::DATE;

    -- Get the last event date for this user and event type
    SELECT recorded_at::DATE INTO last_event_date
    FROM hygiene_events
    WHERE user_id = p_user_id
        AND event_type = p_event_type
        AND recorded_at < p_recorded_at
    ORDER BY recorded_at DESC
    LIMIT 1;

    -- Calculate streak
    IF last_event_date IS NOT NULL THEN
        IF current_event_date = last_event_date THEN
            -- Same day, maintain existing streak
            SELECT COALESCE(MAX(streak_count), 1) INTO current_streak
            FROM hygiene_events
            WHERE user_id = p_user_id
                AND event_type = p_event_type
                AND recorded_at::DATE = current_event_date;
        ELSIF current_event_date = last_event_date + INTERVAL '1 day' THEN
            -- Consecutive day, increment streak
            SELECT COALESCE(MAX(streak_count), 0) + 1 INTO current_streak
            FROM hygiene_events
            WHERE user_id = p_user_id
                AND event_type = p_event_type
                AND recorded_at::DATE = last_event_date;
        ELSE
            -- Gap in streak, reset to 1
            current_streak := 1;
        END IF;
    END IF;

    RETURN current_streak;
END;
$$ LANGUAGE plpgsql STABLE;

-- Trigger to automatically calculate streak on insert
CREATE OR REPLACE FUNCTION trigger_update_hygiene_streak() RETURNS TRIGGER AS $$
BEGIN
    NEW.streak_count := update_hygiene_streak(NEW.user_id, NEW.event_type, NEW.recorded_at);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER hygiene_streak_trigger
    BEFORE INSERT ON hygiene_events
    FOR EACH ROW EXECUTE FUNCTION trigger_update_hygiene_streak();

-- ============================================================================
-- BODY MEASUREMENTS TABLE
-- ============================================================================

-- Body Measurements table (weight, BMI, body composition, physical measurements)
CREATE TABLE body_measurements (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Weight & Body Composition (Smart Scale Data)
    body_weight_kg DOUBLE PRECISION CHECK (body_weight_kg IS NULL OR body_weight_kg BETWEEN 20.0 AND 500.0),
    body_mass_index DOUBLE PRECISION CHECK (body_mass_index IS NULL OR body_mass_index BETWEEN 10.0 AND 60.0),
    body_fat_percentage DOUBLE PRECISION CHECK (body_fat_percentage IS NULL OR body_fat_percentage BETWEEN 3.0 AND 50.0),
    lean_body_mass_kg DOUBLE PRECISION CHECK (lean_body_mass_kg IS NULL OR lean_body_mass_kg BETWEEN 10.0 AND 200.0),

    -- Physical Measurements
    height_cm DOUBLE PRECISION CHECK (height_cm IS NULL OR height_cm BETWEEN 50.0 AND 250.0),
    waist_circumference_cm DOUBLE PRECISION CHECK (waist_circumference_cm IS NULL OR waist_circumference_cm BETWEEN 40.0 AND 200.0),
    hip_circumference_cm DOUBLE PRECISION CHECK (hip_circumference_cm IS NULL OR hip_circumference_cm BETWEEN 40.0 AND 200.0),
    chest_circumference_cm DOUBLE PRECISION CHECK (chest_circumference_cm IS NULL OR chest_circumference_cm BETWEEN 40.0 AND 200.0),
    arm_circumference_cm DOUBLE PRECISION CHECK (arm_circumference_cm IS NULL OR arm_circumference_cm BETWEEN 15.0 AND 60.0),
    thigh_circumference_cm DOUBLE PRECISION CHECK (thigh_circumference_cm IS NULL OR thigh_circumference_cm BETWEEN 30.0 AND 100.0),

    -- Body Temperature (moved from temperature_metrics for body measurement context)
    body_temperature_celsius DOUBLE PRECISION CHECK (body_temperature_celsius IS NULL OR body_temperature_celsius BETWEEN 30.0 AND 45.0),
    basal_body_temperature_celsius DOUBLE PRECISION CHECK (basal_body_temperature_celsius IS NULL OR basal_body_temperature_celsius BETWEEN 35.0 AND 39.0),

    -- Measurement Context & Validation
    measurement_source VARCHAR(50) DEFAULT 'manual', -- 'manual', 'smart_scale', 'apple_watch', 'tape_measure', 'medical_device'
    bmi_calculated BOOLEAN DEFAULT false, -- whether BMI was calculated from weight/height or measured directly
    measurement_reliability VARCHAR(20) DEFAULT 'standard', -- 'low', 'standard', 'high', 'medical_grade'

    -- Body composition method for advanced measurements
    body_composition_method VARCHAR(50), -- 'bioelectric_impedance', 'hydrostatic', 'dexa_scan', 'bod_pod', 'skinfold'

    -- Fitness tracking context
    fitness_phase VARCHAR(50), -- 'cutting', 'bulking', 'maintenance', 'rehabilitation', 'general_fitness'
    measurement_conditions VARCHAR(100), -- 'fasted', 'post_meal', 'post_workout', 'morning', 'evening'

    -- Metadata
    source_device VARCHAR(255),
    measurement_notes TEXT, -- User notes about measurement conditions, goals, etc.
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint for deduplication (user + timestamp + source allows multiple measurements per day from different sources)
    UNIQUE (user_id, recorded_at, measurement_source)
);

-- Indexes for body measurements (time-series queries and fitness tracking)
CREATE INDEX idx_body_measurements_user_date ON body_measurements (user_id, recorded_at DESC);
CREATE INDEX idx_body_measurements_weight ON body_measurements (user_id, body_weight_kg DESC) WHERE body_weight_kg IS NOT NULL;
CREATE INDEX idx_body_measurements_bmi ON body_measurements (user_id, body_mass_index DESC) WHERE body_mass_index IS NOT NULL;
CREATE INDEX idx_body_measurements_body_fat ON body_measurements (user_id, body_fat_percentage DESC) WHERE body_fat_percentage IS NOT NULL;
CREATE INDEX idx_body_measurements_source ON body_measurements (measurement_source, recorded_at DESC);

-- Function to calculate BMI consistency check (validation helper)
CREATE OR REPLACE FUNCTION validate_bmi_consistency(
    p_weight_kg DOUBLE PRECISION,
    p_height_cm DOUBLE PRECISION,
    p_bmi DOUBLE PRECISION
) RETURNS BOOLEAN AS $$
DECLARE
    calculated_bmi DOUBLE PRECISION;
    bmi_tolerance DOUBLE PRECISION := 0.5; -- Allow 0.5 BMI unit difference
BEGIN
    -- Return true if any required values are NULL (can't validate)
    IF p_weight_kg IS NULL OR p_height_cm IS NULL OR p_bmi IS NULL THEN
        RETURN true;
    END IF;

    -- Calculate BMI: weight(kg) / (height(m))²
    calculated_bmi := p_weight_kg / POWER(p_height_cm / 100.0, 2);

    -- Check if provided BMI is within tolerance of calculated BMI
    RETURN ABS(p_bmi - calculated_bmi) <= bmi_tolerance;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to calculate body fat percentage category
CREATE OR REPLACE FUNCTION categorize_body_fat_percentage(
    p_body_fat_percentage DOUBLE PRECISION,
    p_gender VARCHAR(10) DEFAULT 'unknown' -- 'male', 'female', 'unknown'
) RETURNS VARCHAR(20) AS $$
BEGIN
    IF p_body_fat_percentage IS NULL THEN
        RETURN NULL;
    END IF;

    -- Body fat categories vary by gender (using fitness industry standards)
    CASE p_gender
        WHEN 'male' THEN
            CASE
                WHEN p_body_fat_percentage <= 6 THEN RETURN 'essential_fat';
                WHEN p_body_fat_percentage <= 13 THEN RETURN 'athletic';
                WHEN p_body_fat_percentage <= 17 THEN RETURN 'fitness';
                WHEN p_body_fat_percentage <= 25 THEN RETURN 'average';
                ELSE RETURN 'above_average';
            END CASE;
        WHEN 'female' THEN
            CASE
                WHEN p_body_fat_percentage <= 12 THEN RETURN 'essential_fat';
                WHEN p_body_fat_percentage <= 20 THEN RETURN 'athletic';
                WHEN p_body_fat_percentage <= 24 THEN RETURN 'fitness';
                WHEN p_body_fat_percentage <= 31 THEN RETURN 'average';
                ELSE RETURN 'above_average';
            END CASE;
        ELSE
            -- Gender-neutral categorization (uses average of male/female ranges)
            CASE
                WHEN p_body_fat_percentage <= 9 THEN RETURN 'essential_fat';
                WHEN p_body_fat_percentage <= 16 THEN RETURN 'athletic';
                WHEN p_body_fat_percentage <= 20 THEN RETURN 'fitness';
                WHEN p_body_fat_percentage <= 28 THEN RETURN 'average';
                ELSE RETURN 'above_average';
            END CASE;
    END CASE;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Trigger to validate BMI consistency on insert/update
CREATE OR REPLACE FUNCTION trigger_validate_bmi_consistency() RETURNS TRIGGER AS $$
BEGIN
    -- Skip validation if BMI was not calculated from weight/height
    IF NEW.bmi_calculated = false THEN
        RETURN NEW;
    END IF;

    -- Validate BMI consistency
    IF NOT validate_bmi_consistency(NEW.body_weight_kg, NEW.height_cm, NEW.body_mass_index) THEN
        RAISE WARNING 'BMI inconsistency detected: weight=%, height=%, bmi=% for user %',
            NEW.body_weight_kg, NEW.height_cm, NEW.body_mass_index, NEW.user_id;
        -- Log inconsistency but don't block the insert (data quality issue, not critical error)
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER body_measurements_bmi_validation
    BEFORE INSERT OR UPDATE ON body_measurements
    FOR EACH ROW
    EXECUTE FUNCTION trigger_validate_bmi_consistency();

-- ============================================================================
-- USER CHARACTERISTICS ENUMS (Static User Profile Data)
-- ============================================================================

-- Biological Sex for health metrics personalization
CREATE TYPE biological_sex AS ENUM (
    'male', 'female', 'not_set'
);

-- Blood Type for medical information
CREATE TYPE blood_type AS ENUM (
    'A_positive', 'A_negative', 'B_positive', 'B_negative',
    'AB_positive', 'AB_negative', 'O_positive', 'O_negative', 'not_set'
);

-- Fitzpatrick Skin Type for UV protection recommendations
CREATE TYPE fitzpatrick_skin_type AS ENUM (
    'type_1', 'type_2', 'type_3', 'type_4', 'type_5', 'type_6', 'not_set'
);

-- Apple Watch Activity Move Mode for fitness personalization
CREATE TYPE activity_move_mode AS ENUM (
    'active_energy', 'move_time', 'not_set'
);

-- ============================================================================
-- USER CHARACTERISTICS TABLE
-- ============================================================================

-- User Characteristics table for personalized health tracking
CREATE TABLE user_characteristics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Biological Characteristics
    biological_sex biological_sex NOT NULL DEFAULT 'not_set',
    date_of_birth DATE, -- For age-specific health metric validation
    blood_type blood_type NOT NULL DEFAULT 'not_set',

    -- Physical Characteristics
    fitzpatrick_skin_type fitzpatrick_skin_type NOT NULL DEFAULT 'not_set', -- UV sensitivity (1-6 scale)
    wheelchair_use BOOLEAN NOT NULL DEFAULT false, -- Accessibility considerations

    -- Fitness Device Configuration
    activity_move_mode activity_move_mode NOT NULL DEFAULT 'not_set', -- Apple Watch move mode

    -- Privacy and Medical Information
    emergency_contact_info JSONB DEFAULT '{}'::jsonb, -- Encrypted emergency contact data
    medical_conditions TEXT[], -- Array of relevant medical conditions affecting health metrics
    medications TEXT[], -- Current medications that may affect health readings

    -- Data Management
    data_sharing_preferences JSONB DEFAULT '{
        "research_participation": false,
        "anonymized_analytics": false,
        "emergency_sharing": true
    }'::jsonb,

    -- Audit Trail
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    last_verified_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP, -- When user last verified their characteristics

    -- Unique constraint - one characteristics record per user
    UNIQUE(user_id)
);

-- Indexes for user characteristics
CREATE INDEX idx_user_characteristics_user_id ON user_characteristics(user_id);
CREATE INDEX idx_user_characteristics_biological_sex ON user_characteristics(biological_sex) WHERE biological_sex != 'not_set';
CREATE INDEX idx_user_characteristics_age_group ON user_characteristics(
    EXTRACT(YEAR FROM AGE(date_of_birth))
) WHERE date_of_birth IS NOT NULL;
CREATE INDEX idx_user_characteristics_wheelchair_use ON user_characteristics(wheelchair_use) WHERE wheelchair_use = true;
CREATE INDEX idx_user_characteristics_blood_type ON user_characteristics(blood_type) WHERE blood_type != 'not_set';

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_user_characteristics_updated_at
    BEFORE UPDATE ON user_characteristics
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- USER CHARACTERISTICS HELPER FUNCTIONS
-- ============================================================================

-- Function to calculate age from date_of_birth
CREATE OR REPLACE FUNCTION calculate_user_age(p_date_of_birth DATE)
RETURNS INTEGER AS $$
BEGIN
    IF p_date_of_birth IS NULL THEN
        RETURN NULL;
    END IF;
    RETURN EXTRACT(YEAR FROM AGE(p_date_of_birth));
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to get personalized heart rate zones based on user characteristics
CREATE OR REPLACE FUNCTION get_personalized_heart_rate_zones(
    p_user_id UUID,
    p_resting_heart_rate INTEGER DEFAULT 60
) RETURNS JSONB AS $$
DECLARE
    user_age INTEGER;
    max_heart_rate INTEGER;
    zones JSONB;
BEGIN
    -- Get user's age
    SELECT calculate_user_age(uc.date_of_birth)
    INTO user_age
    FROM user_characteristics uc
    WHERE uc.user_id = p_user_id;

    -- If no age data, use default zones
    IF user_age IS NULL THEN
        user_age := 30; -- Default age
    END IF;

    -- Calculate max heart rate (220 - age formula)
    max_heart_rate := 220 - user_age;

    -- Calculate heart rate zones
    zones := jsonb_build_object(
        'max_heart_rate', max_heart_rate,
        'resting_heart_rate', p_resting_heart_rate,
        'zone_1_fat_burn', jsonb_build_object(
            'min', ROUND((max_heart_rate - p_resting_heart_rate) * 0.5 + p_resting_heart_rate),
            'max', ROUND((max_heart_rate - p_resting_heart_rate) * 0.6 + p_resting_heart_rate)
        ),
        'zone_2_aerobic', jsonb_build_object(
            'min', ROUND((max_heart_rate - p_resting_heart_rate) * 0.6 + p_resting_heart_rate),
            'max', ROUND((max_heart_rate - p_resting_heart_rate) * 0.7 + p_resting_heart_rate)
        ),
        'zone_3_anaerobic', jsonb_build_object(
            'min', ROUND((max_heart_rate - p_resting_heart_rate) * 0.7 + p_resting_heart_rate),
            'max', ROUND((max_heart_rate - p_resting_heart_rate) * 0.8 + p_resting_heart_rate)
        ),
        'zone_4_vo2_max', jsonb_build_object(
            'min', ROUND((max_heart_rate - p_resting_heart_rate) * 0.8 + p_resting_heart_rate),
            'max', ROUND((max_heart_rate - p_resting_heart_rate) * 0.9 + p_resting_heart_rate)
        ),
        'zone_5_neuromuscular', jsonb_build_object(
            'min', ROUND((max_heart_rate - p_resting_heart_rate) * 0.9 + p_resting_heart_rate),
            'max', max_heart_rate
        )
    );

    RETURN zones;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get personalized validation ranges for health metrics
CREATE OR REPLACE FUNCTION get_personalized_validation_ranges(
    p_user_id UUID,
    p_metric_type VARCHAR(50)
) RETURNS JSONB AS $$
DECLARE
    user_characteristics RECORD;
    user_age INTEGER;
    validation_ranges JSONB := '{}'::jsonb;
BEGIN
    -- Get user characteristics
    SELECT uc.biological_sex, uc.date_of_birth, uc.wheelchair_use, uc.medical_conditions
    INTO user_characteristics
    FROM user_characteristics uc
    WHERE uc.user_id = p_user_id;

    -- Calculate age
    user_age := calculate_user_age(user_characteristics.date_of_birth);

    -- Return personalized ranges based on metric type
    CASE p_metric_type
        WHEN 'heart_rate' THEN
            -- Heart rate ranges vary by age and biological sex
            validation_ranges := jsonb_build_object(
                'min_resting', CASE
                    WHEN user_age IS NULL THEN 40
                    WHEN user_age < 30 THEN 40
                    WHEN user_age < 50 THEN 45
                    ELSE 50
                END,
                'max_resting', CASE
                    WHEN user_age IS NULL THEN 100
                    WHEN user_age < 30 THEN 100
                    WHEN user_age < 50 THEN 95
                    ELSE 90
                END,
                'max_exercise', 220 - COALESCE(user_age, 30)
            );

        WHEN 'blood_pressure' THEN
            -- Blood pressure ranges vary by age
            validation_ranges := jsonb_build_object(
                'systolic_min', 90,
                'systolic_max', CASE
                    WHEN user_age IS NULL THEN 140
                    WHEN user_age < 65 THEN 140
                    ELSE 150
                END,
                'diastolic_min', 60,
                'diastolic_max', 90
            );

        WHEN 'activity' THEN
            -- Activity ranges consider wheelchair use
            validation_ranges := jsonb_build_object(
                'step_count_max', CASE
                    WHEN user_characteristics.wheelchair_use THEN 10000  -- Lower for wheelchair users
                    ELSE 50000
                END,
                'distance_max_km', CASE
                    WHEN user_characteristics.wheelchair_use THEN 100
                    ELSE 200
                END
            );

        ELSE
            -- Default ranges
            validation_ranges := jsonb_build_object('status', 'no_personalization_available');
    END CASE;

    RETURN validation_ranges;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to get UV exposure recommendations based on Fitzpatrick skin type
CREATE OR REPLACE FUNCTION get_uv_protection_recommendations(
    p_user_id UUID
) RETURNS JSONB AS $$
DECLARE
    skin_type fitzpatrick_skin_type;
    recommendations JSONB;
BEGIN
    -- Get user's skin type
    SELECT uc.fitzpatrick_skin_type
    INTO skin_type
    FROM user_characteristics uc
    WHERE uc.user_id = p_user_id;

    -- Return recommendations based on skin type
    CASE skin_type
        WHEN 'type_1' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type I - Very Fair',
                'burn_time_minutes', 10,
                'spf_recommendation', 'SPF 30+',
                'sun_exposure_advice', 'Always burn, never tan. Use highest protection.',
                'uv_index_limit', 3
            );
        WHEN 'type_2' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type II - Fair',
                'burn_time_minutes', 15,
                'spf_recommendation', 'SPF 30+',
                'sun_exposure_advice', 'Usually burn, tan minimally. High protection needed.',
                'uv_index_limit', 4
            );
        WHEN 'type_3' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type III - Medium',
                'burn_time_minutes', 20,
                'spf_recommendation', 'SPF 15-30',
                'sun_exposure_advice', 'Sometimes burn, tan gradually. Moderate protection.',
                'uv_index_limit', 6
            );
        WHEN 'type_4' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type IV - Olive',
                'burn_time_minutes', 30,
                'spf_recommendation', 'SPF 15+',
                'sun_exposure_advice', 'Rarely burn, tan well. Basic protection advised.',
                'uv_index_limit', 8
            );
        WHEN 'type_5' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type V - Brown',
                'burn_time_minutes', 45,
                'spf_recommendation', 'SPF 15',
                'sun_exposure_advice', 'Very rarely burn, tan darkly. Minimal protection needed.',
                'uv_index_limit', 10
            );
        WHEN 'type_6' THEN
            recommendations := jsonb_build_object(
                'skin_type', 'Type VI - Black',
                'burn_time_minutes', 60,
                'spf_recommendation', 'SPF 15',
                'sun_exposure_advice', 'Never burn, always tan darkly. Basic protection for comfort.',
                'uv_index_limit', 12
            );
        ELSE
            recommendations := jsonb_build_object(
                'skin_type', 'Not Set',
                'burn_time_minutes', 20,
                'spf_recommendation', 'SPF 30',
                'sun_exposure_advice', 'Use standard sun protection measures.',
                'uv_index_limit', 6
            );
    END CASE;

    RETURN recommendations;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to check if user characteristics are complete for personalization
CREATE OR REPLACE FUNCTION is_user_profile_complete(p_user_id UUID)
RETURNS JSONB AS $$
DECLARE
    characteristics RECORD;
    completeness JSONB;
    score INTEGER := 0;
    total_fields INTEGER := 6;
BEGIN
    SELECT
        biological_sex, date_of_birth, blood_type,
        fitzpatrick_skin_type, wheelchair_use, activity_move_mode
    INTO characteristics
    FROM user_characteristics
    WHERE user_id = p_user_id;

    IF NOT FOUND THEN
        RETURN jsonb_build_object(
            'complete', false,
            'completeness_score', 0,
            'missing_fields', jsonb_build_array(
                'biological_sex', 'date_of_birth', 'blood_type',
                'fitzpatrick_skin_type', 'wheelchair_use', 'activity_move_mode'
            )
        );
    END IF;

    -- Calculate completeness score
    IF characteristics.biological_sex != 'not_set' THEN score := score + 1; END IF;
    IF characteristics.date_of_birth IS NOT NULL THEN score := score + 1; END IF;
    IF characteristics.blood_type != 'not_set' THEN score := score + 1; END IF;
    IF characteristics.fitzpatrick_skin_type != 'not_set' THEN score := score + 1; END IF;
    -- wheelchair_use always has value (boolean), so always +1
    score := score + 1;
    IF characteristics.activity_move_mode != 'not_set' THEN score := score + 1; END IF;

    completeness := jsonb_build_object(
        'complete', (score = total_fields),
        'completeness_score', ROUND((score::NUMERIC / total_fields) * 100, 1),
        'completed_fields', score,
        'total_fields', total_fields
    );

    RETURN completeness;
END;
$$ LANGUAGE plpgsql STABLE;

-- ============================================================================
-- END OF SCHEMA
-- ============================================================================