-- Missing Tables and API Key Fixes Migration
-- Adds workout_routes table, updates API keys structure, and other missing components

-- Update API keys table to support dual authentication format per ARCHITECTURE.md
ALTER TABLE api_keys 
ADD COLUMN IF NOT EXISTS key_type VARCHAR(20) DEFAULT 'hashed' CHECK (key_type IN ('uuid', 'hashed')),
ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
DROP CONSTRAINT IF EXISTS api_keys_user_id_name_unique,
ADD CONSTRAINT active_key CHECK (revoked_at IS NULL OR revoked_at > created_at);

-- Update indexes for API keys
DROP INDEX IF EXISTS idx_api_keys_key_hash;
CREATE INDEX idx_api_keys_hash_active ON api_keys USING BTREE (key_hash) 
    WHERE is_active = true AND revoked_at IS NULL;

-- Create workout_routes table for GPS point data (separate from workouts table)
CREATE TABLE workout_routes (
    id BIGSERIAL PRIMARY KEY,
    workout_id UUID NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    point_order INTEGER NOT NULL,
    location GEOGRAPHY(POINT, 4326) NOT NULL,
    altitude_meters NUMERIC(7,2),
    recorded_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workout_id, point_order)
);

-- Create spatial indexes for workout routes
CREATE INDEX idx_workout_routes_geography 
    ON workout_routes USING GIST (location);

CREATE INDEX idx_workout_routes_workout 
    ON workout_routes (workout_id, point_order);

CREATE INDEX idx_workout_routes_recorded_at 
    ON workout_routes (recorded_at);

-- Update workouts table to match ARCHITECTURE.md specifications
ALTER TABLE workouts 
ADD COLUMN IF NOT EXISTS duration_seconds INTEGER,
ADD COLUMN IF NOT EXISTS total_energy_kcal NUMERIC(8,2),
ADD COLUMN IF NOT EXISTS active_energy_kcal NUMERIC(8,2),
ADD COLUMN IF NOT EXISTS step_count INTEGER,
DROP COLUMN IF EXISTS duration_minutes CASCADE,
DROP COLUMN IF EXISTS calories_burned CASCADE;

-- Add computed column for duration
ALTER TABLE workouts 
ADD COLUMN IF NOT EXISTS duration_computed INTEGER 
GENERATED ALWAYS AS (EXTRACT(EPOCH FROM (ended_at - started_at))) STORED;

-- Add constraint to ensure end time is after start time
DO $$ BEGIN
    ALTER TABLE workouts ADD CONSTRAINT workout_time_check CHECK (ended_at > started_at);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- Add unique constraint for user and start time
ALTER TABLE workouts
DROP CONSTRAINT IF EXISTS workouts_user_started_unique,
ADD CONSTRAINT workouts_user_started_unique UNIQUE (user_id, started_at);

-- Create table for processing status tracking
CREATE TABLE processing_status (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    raw_ingestion_id UUID NOT NULL, -- Will add FK constraint later
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_processing_status_raw_ingestion ON processing_status(raw_ingestion_id);
CREATE INDEX idx_processing_status_status ON processing_status(status);
CREATE INDEX idx_processing_status_started ON processing_status(started_at);

-- Create rate limiting tracking table
CREATE TABLE rate_limit_tracking (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    request_count INTEGER DEFAULT 0,
    bandwidth_used BIGINT DEFAULT 0, -- bytes
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (api_key_id, window_start)
);

CREATE INDEX idx_rate_limit_api_key ON rate_limit_tracking(api_key_id);
CREATE INDEX idx_rate_limit_window ON rate_limit_tracking(window_start, window_end);

-- Add trigger for updated_at on rate_limit_tracking
CREATE TRIGGER update_rate_limit_tracking_updated_at
    BEFORE UPDATE ON rate_limit_tracking
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Create data quality metrics table for monitoring
CREATE TABLE data_quality_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    metric_type VARCHAR(50) NOT NULL,
    date DATE NOT NULL,
    total_records INTEGER DEFAULT 0,
    duplicate_records INTEGER DEFAULT 0,
    invalid_records INTEGER DEFAULT 0,
    anomalous_records INTEGER DEFAULT 0,
    quality_score DECIMAL(5,2), -- 0-100 quality score
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (user_id, metric_type, date)
);

CREATE INDEX idx_data_quality_user_type ON data_quality_metrics(user_id, metric_type);
CREATE INDEX idx_data_quality_date ON data_quality_metrics(date);
CREATE INDEX idx_data_quality_score ON data_quality_metrics(quality_score);

-- Create user preferences table
CREATE TABLE user_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    timezone VARCHAR(50) DEFAULT 'UTC',
    units_system VARCHAR(10) DEFAULT 'metric' CHECK (units_system IN ('metric', 'imperial')),
    data_retention_days INTEGER DEFAULT 3650, -- 10 years default
    privacy_settings JSONB DEFAULT '{}',
    notification_settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_user_preferences_user ON user_preferences(user_id);

CREATE TRIGGER update_user_preferences_updated_at
    BEFORE UPDATE ON user_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Create API usage statistics table
CREATE TABLE api_usage_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    payload_size_bytes INTEGER,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_api_usage_key_time ON api_usage_stats(api_key_id, timestamp);
CREATE INDEX idx_api_usage_endpoint ON api_usage_stats(endpoint);
CREATE INDEX idx_api_usage_status ON api_usage_stats(status_code);
CREATE INDEX idx_api_usage_timestamp ON api_usage_stats(timestamp);