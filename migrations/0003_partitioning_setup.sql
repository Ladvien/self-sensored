-- Partitioning Setup Migration
-- Adds monthly partitioning to time-series tables and partition management functions
-- This migration converts existing tables to partitioned tables

-- First, create new partitioned versions of tables that need partitioning
-- We'll migrate data from existing tables to partitioned versions

-- Create partitioned raw_ingestions table
CREATE TABLE raw_ingestions_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(id),
    payload JSONB NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    processing_errors JSONB,
    PRIMARY KEY (id, received_at),
    UNIQUE (user_id, received_at, id)
) PARTITION BY RANGE (received_at);

-- Create partitioned audit_log table  
CREATE TABLE audit_log_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL,
    api_key_id UUID,
    action VARCHAR(50) NOT NULL,
    resource_type VARCHAR(50),
    resource_id VARCHAR(100),
    metadata JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create partitioned heart_rate_metrics table matching ARCHITECTURE.md
CREATE TABLE heart_rate_metrics_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    min_bpm SMALLINT CHECK (min_bpm > 0 AND min_bpm <= 300),
    avg_bpm SMALLINT CHECK (avg_bpm > 0 AND avg_bpm <= 300),
    max_bpm SMALLINT CHECK (max_bpm > 0 AND max_bpm <= 300),
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Create partitioned blood_pressure_metrics table
CREATE TABLE blood_pressure_metrics_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    systolic SMALLINT NOT NULL CHECK (systolic > 0 AND systolic <= 300),
    diastolic SMALLINT NOT NULL CHECK (diastolic > 0 AND diastolic <= 200),
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Create partitioned sleep_metrics table matching ARCHITECTURE.md
CREATE TABLE sleep_metrics_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    asleep_duration_minutes INTEGER CHECK (asleep_duration_minutes >= 0),
    in_bed_duration_minutes INTEGER CHECK (in_bed_duration_minutes >= 0),
    sleep_start TIMESTAMPTZ,
    sleep_end TIMESTAMPTZ,
    in_bed_start TIMESTAMPTZ,
    in_bed_end TIMESTAMPTZ,
    sleep_source VARCHAR(100),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, date)
) PARTITION BY RANGE (date);

-- Create partitioned activity_metrics table matching ARCHITECTURE.md
CREATE TABLE activity_metrics_partitioned (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- 'steps', 'distance', 'flights_climbed', etc.
    value NUMERIC(10,2) NOT NULL,
    unit VARCHAR(20) NOT NULL,
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at, metric_type)
) PARTITION BY RANGE (recorded_at);

-- Function to create monthly partitions for any partitioned table
CREATE OR REPLACE FUNCTION create_monthly_partitions(
    table_name text,
    date_column text,
    start_months_back integer DEFAULT 1,
    end_months_ahead integer DEFAULT 12
)
RETURNS void AS $$
DECLARE
    start_date date;
    end_date date;
    partition_name text;
    i integer;
BEGIN
    FOR i IN -start_months_back..end_months_ahead LOOP
        start_date := date_trunc('month', CURRENT_DATE) + (i || ' months')::interval;
        end_date := start_date + '1 month'::interval;
        partition_name := table_name || '_' || to_char(start_date, 'YYYY_MM');
        
        -- Check if partition already exists
        IF NOT EXISTS (
            SELECT 1 FROM pg_class WHERE relname = partition_name
        ) THEN
            EXECUTE format('
                CREATE TABLE %I PARTITION OF %I
                FOR VALUES FROM (%L) TO (%L)',
                partition_name, table_name, start_date, end_date
            );
            
            RAISE NOTICE 'Created partition: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Function to automatically create future partitions (called by cron)
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for all partitioned tables
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    
    -- For sleep metrics, we need a special case for DATE partitioning
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
END;
$$ LANGUAGE plpgsql;

-- Create initial partitions (12 months ahead from current date)
SELECT create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
SELECT create_monthly_partitions('audit_log_partitioned', 'created_at');  
SELECT create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
SELECT create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
SELECT create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
SELECT create_monthly_partitions('sleep_metrics_partitioned', 'date');

-- Function to drop old partitions (for data retention)
CREATE OR REPLACE FUNCTION drop_old_partitions(
    table_name text,
    months_to_keep integer DEFAULT 24
)
RETURNS void AS $$
DECLARE
    cutoff_date date;
    partition_record record;
    partition_name text;
BEGIN
    cutoff_date := date_trunc('month', CURRENT_DATE) - (months_to_keep || ' months')::interval;
    
    FOR partition_record IN 
        SELECT schemaname, tablename 
        FROM pg_tables 
        WHERE tablename LIKE table_name || '_%'
        AND tablename ~ '[0-9]{4}_[0-9]{2}$'
    LOOP
        -- Extract date from partition name (assumes format: table_YYYY_MM)
        partition_name := partition_record.tablename;
        
        -- Check if partition is older than cutoff
        IF to_date(right(partition_name, 7), 'YYYY_MM') < cutoff_date THEN
            EXECUTE format('DROP TABLE IF EXISTS %I', partition_name);
            RAISE NOTICE 'Dropped old partition: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;