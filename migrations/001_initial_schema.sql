-- Health Export REST API - Initial Schema Migration
-- Created from database/schema.sql for SQLx test compatibility

-- ============================================================================
-- EXTENSIONS (Moved to 000_setup_extensions.sql)
-- ============================================================================

-- Extensions are now created in 000_setup_extensions.sql

-- ============================================================================
-- ENUM TYPE DEFINITIONS
-- ============================================================================

-- Activity Context
DO $$ BEGIN
    CREATE TYPE activity_context AS ENUM (
        'resting', 'walking', 'running', 'cycling', 'exercise',
        'sleeping', 'sedentary', 'active', 'post_meal', 'stressed', 'recovery'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Workout Type (Comprehensive HealthKit Support - 70+ Types)
DO $$ BEGIN
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
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Processing Status for Raw Ingestions
DO $$ BEGIN
    CREATE TYPE processing_status AS ENUM ('pending', 'processing', 'completed', 'failed');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Job Type for Background Processing
DO $$ BEGIN
    CREATE TYPE job_type AS ENUM ('health_data_ingest', 'data_export', 'data_cleanup', 'system_maintenance');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Job Status for Background Processing
DO $$ BEGIN
    CREATE TYPE job_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Priority Level for Background Jobs
DO $$ BEGIN
    CREATE TYPE priority_level AS ENUM ('low', 'normal', 'high', 'critical');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- ============================================================================
-- CORE TABLES
-- ============================================================================

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    apple_health_id VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255), -- Optional for API-only users
    is_active BOOLEAN DEFAULT true,
    timezone VARCHAR(50) DEFAULT 'UTC',
    date_format VARCHAR(20) DEFAULT 'YYYY-MM-DD',
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE
);

-- API Keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL, -- Argon2 hash of the API key
    name VARCHAR(100) NOT NULL,
    is_active BOOLEAN DEFAULT true,
    permissions JSONB DEFAULT '{}',
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Raw ingestion data (for backup and replay)
CREATE TABLE raw_ingestions (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    payload_hash VARCHAR(64) NOT NULL, -- SHA-256 of raw payload
    raw_payload JSONB NOT NULL, -- Complete raw payload for backup
    payload_size_bytes INTEGER NOT NULL,
    processing_status processing_status DEFAULT 'pending',
    processing_errors JSONB, -- Array of error details
    processing_attempts INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- ============================================================================
-- HEALTH METRICS TABLES (Core 5 Types)
-- ============================================================================

-- Heart Rate Metrics
CREATE TABLE heart_rate_metrics (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMP WITH TIME ZONE NOT NULL,
    heart_rate INTEGER, -- BPM
    resting_heart_rate INTEGER, -- BPM
    heart_rate_variability DECIMAL(8,2), -- milliseconds
    walking_heart_rate_average INTEGER, -- BPM
    heart_rate_recovery_one_minute INTEGER, -- BPM
    atrial_fibrillation_burden_percentage DECIMAL(5,2), -- Percentage
    vo2_max_ml_kg_min DECIMAL(6,2), -- mL/kg/min
    context activity_context,
    source_device VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Blood Pressure Metrics
CREATE TABLE blood_pressure_metrics (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMP WITH TIME ZONE NOT NULL,
    systolic INTEGER NOT NULL, -- mmHg
    diastolic INTEGER NOT NULL, -- mmHg
    pulse INTEGER, -- BPM
    source_device VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Sleep Metrics
CREATE TABLE sleep_metrics (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sleep_start TIMESTAMP WITH TIME ZONE NOT NULL,
    sleep_end TIMESTAMP WITH TIME ZONE NOT NULL,
    duration_minutes INTEGER NOT NULL,
    deep_sleep_minutes INTEGER,
    rem_sleep_minutes INTEGER,
    light_sleep_minutes INTEGER,
    awake_minutes INTEGER,
    efficiency DECIMAL(5,2), -- Percentage
    source_device VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, sleep_start)
) PARTITION BY RANGE (sleep_start);

-- Activity Metrics (Daily aggregates)
CREATE TABLE activity_metrics (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMP WITH TIME ZONE NOT NULL, -- Date of activity
    step_count INTEGER,
    distance_meters DECIMAL(10,2),
    active_energy_burned_kcal DECIMAL(8,2),
    basal_energy_burned_kcal DECIMAL(8,2),
    flights_climbed INTEGER,
    distance_cycling_meters DECIMAL(10,2),
    distance_swimming_meters DECIMAL(10,2),
    distance_wheelchair_meters DECIMAL(10,2),
    distance_downhill_snow_sports_meters DECIMAL(10,2),
    push_count INTEGER, -- Wheelchair pushes
    swimming_stroke_count INTEGER,
    nike_fuel_points INTEGER,
    apple_exercise_time_minutes INTEGER,
    apple_stand_time_minutes INTEGER,
    apple_move_time_minutes INTEGER,
    apple_stand_hour_achieved INTEGER, -- Hours where stand goal was met
    source_device VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Workout Metrics
CREATE TABLE workout_metrics (
    id UUID DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workout_type workout_type NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ended_at TIMESTAMP WITH TIME ZONE NOT NULL,
    total_energy_kcal DECIMAL(8,2),
    active_energy_kcal DECIMAL(8,2),
    distance_meters DECIMAL(10,2),
    avg_heart_rate INTEGER, -- BPM
    max_heart_rate INTEGER, -- BPM
    route_data GEOMETRY(LINESTRING, 4326), -- GPS route using PostGIS
    source_device VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (id, started_at)
) PARTITION BY RANGE (started_at);

-- ============================================================================
-- BACKGROUND PROCESSING TABLES
-- ============================================================================

-- Background Jobs Queue
CREATE TABLE background_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    job_type job_type NOT NULL,
    status job_status DEFAULT 'pending',
    priority priority_level DEFAULT 'normal',
    data JSONB NOT NULL, -- Job-specific data
    progress INTEGER DEFAULT 0, -- Percentage (0-100)
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    scheduled_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- INDEXES FOR PERFORMANCE
-- ============================================================================

-- User indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);

-- API Key indexes
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_active ON api_keys(is_active) WHERE is_active = true;

-- Raw ingestion indexes
CREATE INDEX idx_raw_ingestions_user_id ON raw_ingestions(user_id);
CREATE INDEX idx_raw_ingestions_status ON raw_ingestions(processing_status);
CREATE INDEX idx_raw_ingestions_hash ON raw_ingestions(payload_hash);

-- Health metrics indexes (will be created on partitions)
-- Note: These will be created by partition creation scripts

-- Background job indexes
CREATE INDEX idx_background_jobs_user_id ON background_jobs(user_id);
CREATE INDEX idx_background_jobs_status ON background_jobs(status);
CREATE INDEX idx_background_jobs_type ON background_jobs(job_type);
CREATE INDEX idx_background_jobs_priority ON background_jobs(priority);
CREATE INDEX idx_background_jobs_scheduled ON background_jobs(scheduled_at);

-- ============================================================================
-- INITIAL PARTITIONS (Current + Next 3 Months)
-- ============================================================================

-- Create initial partitions for raw_ingestions (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'raw_ingestions_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF raw_ingestions
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END $$;

-- Create initial partitions for heart_rate_metrics (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'heart_rate_metrics_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF heart_rate_metrics
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );

    -- Add indexes to partition
    EXECUTE format('CREATE INDEX idx_%I_user_recorded ON %I(user_id, recorded_at)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_recorded ON %I(recorded_at)', partition_name, partition_name);
END $$;

-- Create initial partitions for blood_pressure_metrics (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'blood_pressure_metrics_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF blood_pressure_metrics
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );

    -- Add indexes to partition
    EXECUTE format('CREATE INDEX idx_%I_user_recorded ON %I(user_id, recorded_at)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_recorded ON %I(recorded_at)', partition_name, partition_name);
END $$;

-- Create initial partitions for sleep_metrics (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'sleep_metrics_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF sleep_metrics
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );

    -- Add indexes to partition
    EXECUTE format('CREATE INDEX idx_%I_user_sleep_start ON %I(user_id, sleep_start)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_sleep_start ON %I(sleep_start)', partition_name, partition_name);
END $$;

-- Create initial partitions for activity_metrics (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'activity_metrics_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF activity_metrics
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );

    -- Add indexes to partition
    EXECUTE format('CREATE INDEX idx_%I_user_recorded ON %I(user_id, recorded_at)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_recorded ON %I(recorded_at)', partition_name, partition_name);
END $$;

-- Create initial partitions for workout_metrics (current month)
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE := start_date + INTERVAL '1 month';
    partition_name TEXT := 'workout_metrics_' || TO_CHAR(start_date, 'YYYY_MM');
BEGIN
    EXECUTE format(
        'CREATE TABLE %I PARTITION OF workout_metrics
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );

    -- Add indexes to partition
    EXECUTE format('CREATE INDEX idx_%I_user_started ON %I(user_id, started_at)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_started ON %I(started_at)', partition_name, partition_name);
    EXECUTE format('CREATE INDEX idx_%I_workout_type ON %I(workout_type)', partition_name, partition_name);
END $$;