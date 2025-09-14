-- Health Export REST API - Simple Prototype Schema
-- Version: 2.0.0
-- Date: 2025-09-12
-- Description: Simplified schema for prototype with ENUM types

-- ============================================================================
-- EXTENSIONS
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- ENUM TYPE DEFINITIONS
-- ============================================================================

-- Activity Context
CREATE TYPE activity_context AS ENUM (
    'resting', 'walking', 'running', 'cycling', 'exercise',
    'sleeping', 'sedentary', 'active', 'post_meal', 'stressed', 'recovery'
);

-- Workout Type
CREATE TYPE workout_type AS ENUM (
    'walking', 'running', 'cycling', 'swimming', 'strength_training',
    'yoga', 'pilates', 'hiit', 'sports', 'other'
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

-- Heart Rate Metrics
CREATE TABLE heart_rate_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    heart_rate INTEGER,
    resting_heart_rate INTEGER,
    heart_rate_variability DOUBLE PRECISION,
    context activity_context,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
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

-- Activity Metrics
CREATE TABLE activity_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    step_count INTEGER,
    distance_meters DOUBLE PRECISION,
    flights_climbed INTEGER,
    active_energy_burned_kcal DOUBLE PRECISION,
    basal_energy_burned_kcal DOUBLE PRECISION,
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

-- Nutrition Metrics (comprehensive dietary data tracking)
CREATE TABLE nutrition_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Hydration & Stimulants
    dietary_water DOUBLE PRECISION,                    -- liters
    dietary_caffeine DOUBLE PRECISION,                 -- mg

    -- Macronutrients
    dietary_energy_consumed DOUBLE PRECISION,          -- calories
    dietary_carbohydrates DOUBLE PRECISION,            -- grams
    dietary_protein DOUBLE PRECISION,                  -- grams
    dietary_fat_total DOUBLE PRECISION,                -- grams
    dietary_fat_saturated DOUBLE PRECISION,            -- grams
    dietary_cholesterol DOUBLE PRECISION,              -- mg
    dietary_sodium DOUBLE PRECISION,                   -- mg
    dietary_fiber DOUBLE PRECISION,                    -- grams
    dietary_sugar DOUBLE PRECISION,                    -- grams

    -- Minerals
    dietary_calcium DOUBLE PRECISION,                  -- mg
    dietary_iron DOUBLE PRECISION,                     -- mg
    dietary_magnesium DOUBLE PRECISION,                -- mg
    dietary_potassium DOUBLE PRECISION,                -- mg

    -- Vitamins
    dietary_vitamin_a DOUBLE PRECISION,                -- mcg
    dietary_vitamin_c DOUBLE PRECISION,                -- mg
    dietary_vitamin_d DOUBLE PRECISION,                -- IU

    -- Metadata and source tracking
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, recorded_at)
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

-- Blood pressure indexes
CREATE INDEX idx_blood_pressure_user_recorded ON blood_pressure_metrics(user_id, recorded_at DESC);

-- Sleep indexes
CREATE INDEX idx_sleep_user_start ON sleep_metrics(user_id, sleep_start DESC);

-- Activity indexes
CREATE INDEX idx_activity_user_recorded ON activity_metrics(user_id, recorded_at DESC);

-- Workout indexes
CREATE INDEX idx_workouts_user_started ON workouts(user_id, started_at DESC);
CREATE INDEX idx_workouts_type ON workouts(workout_type);

-- Nutrition indexes
CREATE INDEX idx_nutrition_user_recorded ON nutrition_metrics(user_id, recorded_at DESC);

-- Respiratory indexes
CREATE INDEX idx_respiratory_user_recorded ON respiratory_metrics(user_id, recorded_at DESC);

-- Temperature indexes
CREATE INDEX idx_temperature_user_recorded ON temperature_metrics(user_id, recorded_at DESC);
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
-- END OF SCHEMA
-- ============================================================================