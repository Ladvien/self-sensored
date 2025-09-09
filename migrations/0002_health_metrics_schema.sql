-- Health Metrics Schema Migration
-- Creates tables for storing various health metric types with time-series data
-- Simplified for MVP - partitioning can be added later for production scale

-- Raw ingestions table for backup of all incoming data
CREATE TABLE raw_ingestions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE SET NULL,
    raw_data JSONB NOT NULL,
    data_hash VARCHAR(64) NOT NULL, -- SHA256 hash for deduplication
    ingested_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    status VARCHAR(20) DEFAULT 'pending', -- pending, processed, failed
    error_message TEXT,
    
    CONSTRAINT raw_ingestions_user_hash_unique UNIQUE(user_id, data_hash)
);

-- Heart rate metrics table
CREATE TABLE heart_rate_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_ingestion_id UUID REFERENCES raw_ingestions(id) ON DELETE SET NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    heart_rate INTEGER NOT NULL CHECK (heart_rate > 0 AND heart_rate <= 300),
    resting_heart_rate INTEGER CHECK (resting_heart_rate > 0 AND resting_heart_rate <= 200),
    context VARCHAR(50), -- resting, exercise, recovery, etc.
    source_device VARCHAR(100),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Blood pressure metrics table
CREATE TABLE blood_pressure_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_ingestion_id UUID REFERENCES raw_ingestions(id) ON DELETE SET NULL,
    recorded_at TIMESTAMPTZ NOT NULL,
    systolic INTEGER NOT NULL CHECK (systolic > 0 AND systolic <= 300),
    diastolic INTEGER NOT NULL CHECK (diastolic > 0 AND diastolic <= 200),
    pulse INTEGER CHECK (pulse > 0 AND pulse <= 300),
    source_device VARCHAR(100),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Sleep metrics table
CREATE TABLE sleep_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_ingestion_id UUID REFERENCES raw_ingestions(id) ON DELETE SET NULL,
    sleep_start TIMESTAMPTZ NOT NULL,
    sleep_end TIMESTAMPTZ NOT NULL,
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    deep_sleep_minutes INTEGER CHECK (deep_sleep_minutes >= 0),
    rem_sleep_minutes INTEGER CHECK (rem_sleep_minutes >= 0),
    light_sleep_minutes INTEGER CHECK (light_sleep_minutes >= 0),
    awake_minutes INTEGER CHECK (awake_minutes >= 0),
    sleep_efficiency DECIMAL(5,2) CHECK (sleep_efficiency >= 0 AND sleep_efficiency <= 100),
    source_device VARCHAR(100),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT sleep_duration_consistency CHECK (
        duration_minutes >= COALESCE(deep_sleep_minutes, 0) + 
                           COALESCE(rem_sleep_minutes, 0) + 
                           COALESCE(light_sleep_minutes, 0) + 
                           COALESCE(awake_minutes, 0)
    )
);

-- Activity metrics table (steps, calories, etc.)
CREATE TABLE activity_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_ingestion_id UUID REFERENCES raw_ingestions(id) ON DELETE SET NULL,
    recorded_date DATE NOT NULL,
    steps INTEGER CHECK (steps >= 0),
    distance_meters DECIMAL(10,2) CHECK (distance_meters >= 0),
    calories_burned INTEGER CHECK (calories_burned >= 0),
    active_minutes INTEGER CHECK (active_minutes >= 0),
    flights_climbed INTEGER CHECK (flights_climbed >= 0),
    source_device VARCHAR(100),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT activity_metrics_user_date_unique UNIQUE(user_id, recorded_date)
);

-- Workouts table with GPS support via PostGIS
CREATE TABLE workouts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_ingestion_id UUID REFERENCES raw_ingestions(id) ON DELETE SET NULL,
    workout_type VARCHAR(50) NOT NULL, -- running, cycling, swimming, etc.
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ NOT NULL,
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    distance_meters DECIMAL(10,2) CHECK (distance_meters >= 0),
    calories_burned INTEGER CHECK (calories_burned >= 0),
    average_heart_rate INTEGER CHECK (average_heart_rate > 0 AND average_heart_rate <= 300),
    max_heart_rate INTEGER CHECK (max_heart_rate > 0 AND max_heart_rate <= 300),
    route_geometry GEOMETRY(LINESTRING, 4326), -- GPS track using PostGIS
    source_device VARCHAR(100),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for performance on time-series data
CREATE INDEX idx_raw_ingestions_ingested_at ON raw_ingestions(ingested_at);
CREATE INDEX idx_raw_ingestions_user_id ON raw_ingestions(user_id);
CREATE INDEX idx_raw_ingestions_status ON raw_ingestions(status);
CREATE INDEX idx_raw_ingestions_data_hash ON raw_ingestions(data_hash);

CREATE INDEX idx_heart_rate_recorded_at ON heart_rate_metrics(recorded_at);
CREATE INDEX idx_heart_rate_user_id ON heart_rate_metrics(user_id);
CREATE INDEX idx_heart_rate_user_recorded ON heart_rate_metrics(user_id, recorded_at);

CREATE INDEX idx_blood_pressure_recorded_at ON blood_pressure_metrics(recorded_at);
CREATE INDEX idx_blood_pressure_user_id ON blood_pressure_metrics(user_id);
CREATE INDEX idx_blood_pressure_user_recorded ON blood_pressure_metrics(user_id, recorded_at);

CREATE INDEX idx_sleep_metrics_sleep_start ON sleep_metrics(sleep_start);
CREATE INDEX idx_sleep_metrics_user_id ON sleep_metrics(user_id);
CREATE INDEX idx_sleep_metrics_user_sleep_start ON sleep_metrics(user_id, sleep_start);

CREATE INDEX idx_activity_metrics_recorded_date ON activity_metrics(recorded_date);
CREATE INDEX idx_activity_metrics_user_id ON activity_metrics(user_id);

CREATE INDEX idx_workouts_started_at ON workouts(started_at);
CREATE INDEX idx_workouts_user_id ON workouts(user_id);
CREATE INDEX idx_workouts_workout_type ON workouts(workout_type);
CREATE INDEX idx_workouts_user_started ON workouts(user_id, started_at);

-- Create spatial index for workout routes
CREATE INDEX idx_workouts_route_geometry ON workouts USING GIST (route_geometry);