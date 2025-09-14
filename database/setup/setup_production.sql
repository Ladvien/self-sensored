-- Production Database Setup for Health Export API
-- Run this script as postgres superuser: sudo -u postgres psql -f setup_production.sql

-- Create user if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'self_sensored') THEN
        CREATE USER self_sensored WITH PASSWORD '37om3i*t3XfSZ0';
    END IF;
END
$$;

-- Grant necessary privileges
ALTER USER self_sensored CREATEDB;
GRANT CONNECT ON DATABASE postgres TO self_sensored;

-- Create production database
DROP DATABASE IF EXISTS self_sensored;
CREATE DATABASE self_sensored
    OWNER self_sensored
    ENCODING 'UTF8'
    LC_COLLATE = 'C.UTF-8'
    LC_CTYPE = 'C.UTF-8'
    TEMPLATE template0;

-- Grant all privileges on the production database
GRANT ALL PRIVILEGES ON DATABASE self_sensored TO self_sensored;

-- Switch to the new database and run schema as self_sensored user
\connect self_sensored self_sensored

-- Health Export REST API - Simple Prototype Schema
-- Version: 2.0.0
-- Date: 2025-09-12
-- Description: Simplified schema for prototype with ENUM types

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";


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

-- Grant all privileges on all tables and sequences to self_sensored user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO self_sensored;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO self_sensored;
GRANT USAGE, CREATE ON SCHEMA public TO self_sensored;
