-- Apple Health Schema
DROP SCHEMA IF EXISTS apple_health CASCADE;

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create schema
CREATE SCHEMA IF NOT EXISTS apple_health;

-- Core Tables
CREATE TABLE IF NOT EXISTS apple_health.health_payload (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    received_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

CREATE TABLE IF NOT EXISTS apple_health.health_metric (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payload_id UUID REFERENCES apple_health.health_payload(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    units TEXT NOT NULL
);

-- PARTITIONED quantity_timestamp table
CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    source TEXT,
    PRIMARY KEY (id, date)
) PARTITION BY RANGE (date);

-- Create yearly partitions for quantity_timestamp
CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2012 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2012-01-01') TO ('2013-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2013 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2013-01-01') TO ('2014-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2014 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2014-01-01') TO ('2015-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2015 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2015-01-01') TO ('2016-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2016 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2016-01-01') TO ('2017-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2017 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2017-01-01') TO ('2018-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2018 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2018-01-01') TO ('2019-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2019 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2019-01-01') TO ('2020-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2020 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2020-01-01') TO ('2021-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2021 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2021-01-01') TO ('2022-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2022 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2022-01-01') TO ('2023-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2023 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2023-01-01') TO ('2024-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2024 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2025 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2026 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2027 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2027-01-01') TO ('2028-01-01');

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_2028 
PARTITION OF apple_health.quantity_timestamp
FOR VALUES FROM ('2028-01-01') TO ('2029-01-01');

-- Default partition for dates outside range
CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp_default 
PARTITION OF apple_health.quantity_timestamp DEFAULT;

-- Specialized Metrics
CREATE TABLE IF NOT EXISTS apple_health.blood_pressure(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    systolic DOUBLE PRECISION NOT NULL,
    diastolic DOUBLE PRECISION NOT NULL
);

CREATE TABLE IF NOT EXISTS apple_health.heart_rate (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    min DOUBLE PRECISION,
    avg DOUBLE PRECISION,
    max DOUBLE PRECISION,
    context TEXT,
    source TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.sleep_analysis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    asleep DOUBLE PRECISION,
    sleep_source TEXT,
    in_bed DOUBLE PRECISION,
    "in_bed_start" TIMESTAMP WITH TIME ZONE,
    "in_bed_end" TIMESTAMP WITH TIME ZONE,
    in_bed_source TEXT,
    qty DOUBLE PRECISION,
    value TEXT,
    source TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.blood_glucose(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    meal_time TEXT CHECK (meal_time IN ('Before Meal', 'After Meal', 'Unspecified'))
);

CREATE TABLE IF NOT EXISTS apple_health.sexual_activity(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    unspecified DOUBLE PRECISION,
    protection_used DOUBLE PRECISION,
    protection_not_used DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS apple_health.hygiene_event(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    value TEXT CHECK (value IN ('Complete', 'Incomplete'))
);

CREATE TABLE IF NOT EXISTS apple_health.insulin_delivery(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    reason TEXT CHECK (reason IN ('Bolus', 'Basal'))
);

-- Mental Health
CREATE TABLE IF NOT EXISTS apple_health.symptom(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    name TEXT,
    severity TEXT,
    user_entered BOOLEAN,
    source TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.state_of_mind(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    kind TEXT,
    valence DOUBLE PRECISION,
    valence_classification DOUBLE PRECISION,
    metadata JSONB,
    labels TEXT[],
    associations TEXT[]
);

-- ECG & Notifications
CREATE TABLE IF NOT EXISTS apple_health.ecg(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    classification TEXT,
    severity TEXT,
    average_heart_rate DOUBLE PRECISION,
    number_of_voltage_measurements INT,
    sampling_frequency DOUBLE PRECISION,
    source TEXT,
    voltage_measurements JSONB
);

CREATE TABLE IF NOT EXISTS apple_health.heart_rate_notification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    threshold DOUBLE PRECISION,
    heart_rate JSONB,
    heart_rate_variation JSONB
);

-- Workouts
CREATE TABLE IF NOT EXISTS apple_health.workout (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payload_id UUID REFERENCES apple_health.health_payload(id) ON DELETE CASCADE,
    name TEXT,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    elevation JSONB
);

CREATE TABLE IF NOT EXISTS apple_health.workout_value (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    qty DOUBLE PRECISION,
    units TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.workout_point (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    stream TEXT CHECK (stream IN ('heart_rate', 'heart_rate_recovery')),
    date TIMESTAMP WITH TIME ZONE,
    qty DOUBLE PRECISION,
    units TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.workout_route_point (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    lat DOUBLE PRECISION,
    lon DOUBLE PRECISION,
    altitude DOUBLE PRECISION,
    timestamp TIMESTAMP WITH TIME ZONE
);

-- Performance Indexes
-- Core performance indexes
CREATE INDEX IF NOT EXISTS idx_health_metric_payload_id ON apple_health.health_metric(payload_id);
CREATE INDEX IF NOT EXISTS idx_health_metric_name ON apple_health.health_metric(name);

-- Quantity timestamp indexes (on partitioned table)
CREATE INDEX IF NOT EXISTS idx_quantity_timestamp_metric_id ON apple_health.quantity_timestamp(metric_id);
CREATE INDEX IF NOT EXISTS idx_quantity_timestamp_date ON apple_health.quantity_timestamp(date DESC);
CREATE INDEX IF NOT EXISTS idx_quantity_timestamp_metric_date ON apple_health.quantity_timestamp(metric_id, date DESC);

-- Specialized table indexes
CREATE INDEX IF NOT EXISTS idx_heart_rate_metric_id ON apple_health.heart_rate(metric_id);
CREATE INDEX IF NOT EXISTS idx_heart_rate_date ON apple_health.heart_rate(date DESC);
CREATE INDEX IF NOT EXISTS idx_blood_pressure_metric_id ON apple_health.blood_pressure(metric_id);
CREATE INDEX IF NOT EXISTS idx_blood_pressure_date ON apple_health.blood_pressure(date DESC);
CREATE INDEX IF NOT EXISTS idx_sleep_analysis_metric_id ON apple_health.sleep_analysis(metric_id);
CREATE INDEX IF NOT EXISTS idx_sleep_analysis_start_date ON apple_health.sleep_analysis(start_date DESC);

-- Payload index for cleanup queries
CREATE INDEX IF NOT EXISTS idx_health_payload_received_at ON apple_health.health_payload(received_at DESC);

-- Add unique constraints to enable idempotent operations

-- Add unique constraints
ALTER TABLE apple_health.health_metric 
ADD CONSTRAINT uq_health_metric_payload_name 
UNIQUE (payload_id, name);

ALTER TABLE apple_health.heart_rate 
ADD CONSTRAINT uq_heart_rate_metric_date 
UNIQUE (metric_id, date);

ALTER TABLE apple_health.blood_pressure 
ADD CONSTRAINT uq_blood_pressure_metric_date 
UNIQUE (metric_id, date);

ALTER TABLE apple_health.sleep_analysis 
ADD CONSTRAINT uq_sleep_analysis_metric_start 
UNIQUE (metric_id, start_date);

ALTER TABLE apple_health.quantity_timestamp 
ADD CONSTRAINT uq_quantity_timestamp_metric_date_source 
UNIQUE (metric_id, date, source);

-- Add payload hash column and constraint
ALTER TABLE apple_health.health_payload 
ADD COLUMN payload_hash TEXT;

ALTER TABLE apple_health.health_payload 
ADD CONSTRAINT uq_health_payload_hash 
UNIQUE (payload_hash);