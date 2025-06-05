-- Apple Health Schema with Full Idempotency Support
DROP SCHEMA IF EXISTS apple_health CASCADE;

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create schema
CREATE SCHEMA IF NOT EXISTS apple_health;

-- Core Tables
CREATE TABLE IF NOT EXISTS apple_health.health_payload (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    received_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    payload_hash TEXT NOT NULL,
    CONSTRAINT uq_health_payload_hash UNIQUE (payload_hash)
);

CREATE TABLE IF NOT EXISTS apple_health.health_metric (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payload_id UUID NOT NULL REFERENCES apple_health.health_payload(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    units TEXT NOT NULL,
    data_hash TEXT,
    CONSTRAINT uq_health_metric_payload_name UNIQUE (payload_id, name),
    CONSTRAINT uq_health_metric_data UNIQUE (payload_id, name, data_hash)
);

-- PARTITIONED quantity_timestamp table
CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    source TEXT,
    PRIMARY KEY (id, date),
    CONSTRAINT uq_quantity_timestamp_metric_date_source UNIQUE (metric_id, date, source)
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

-- Specialized Metrics with Unique Constraints
CREATE TABLE IF NOT EXISTS apple_health.blood_pressure(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    systolic DOUBLE PRECISION NOT NULL,
    diastolic DOUBLE PRECISION NOT NULL,
    CONSTRAINT uq_blood_pressure_metric_date UNIQUE (metric_id, date)
);

CREATE TABLE IF NOT EXISTS apple_health.heart_rate (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    min DOUBLE PRECISION,
    avg DOUBLE PRECISION,
    max DOUBLE PRECISION,
    context TEXT,
    source TEXT,
    CONSTRAINT uq_heart_rate_metric_date_context UNIQUE (metric_id, date, context)
);

CREATE TABLE IF NOT EXISTS apple_health.sleep_analysis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    value TEXT,
    qty DOUBLE PRECISION,
    source TEXT,
    CONSTRAINT uq_sleep_analysis_metric_dates UNIQUE (metric_id, start_date, end_date)
);

CREATE TABLE IF NOT EXISTS apple_health.blood_glucose(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    meal_time TEXT CHECK (meal_time IN ('Before Meal', 'After Meal', 'Unspecified')) NOT NULL,
    CONSTRAINT uq_blood_glucose_metric_date_meal UNIQUE (metric_id, date, meal_time)
);

CREATE TABLE IF NOT EXISTS apple_health.sexual_activity(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    unspecified DOUBLE PRECISION,
    protection_used DOUBLE PRECISION,
    protection_not_used DOUBLE PRECISION,
    CONSTRAINT uq_sexual_activity_metric_date UNIQUE (metric_id, date)
);

CREATE TABLE IF NOT EXISTS apple_health.hygiene_event(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    value TEXT CHECK (value IN ('Complete', 'Incomplete')),
    source TEXT,
    CONSTRAINT uq_hygiene_event_metric_date UNIQUE (metric_id, date)
);

CREATE TABLE IF NOT EXISTS apple_health.insulin_delivery(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    reason TEXT CHECK (reason IN ('Bolus', 'Basal')) NOT NULL,
    CONSTRAINT uq_insulin_delivery_metric_date_reason UNIQUE (metric_id, date, reason)
);

-- Mental Health
CREATE TABLE IF NOT EXISTS apple_health.symptom(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    name TEXT NOT NULL,
    severity TEXT NOT NULL,
    user_entered BOOLEAN NOT NULL,
    source TEXT,
    CONSTRAINT uq_symptom_metric_start_name UNIQUE (metric_id, start, name)
);

CREATE TABLE IF NOT EXISTS apple_health.state_of_mind(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    kind TEXT NOT NULL,
    valence DOUBLE PRECISION,
    valence_classification DOUBLE PRECISION,
    metadata JSONB,
    labels TEXT[],
    associations TEXT[],
    CONSTRAINT uq_state_of_mind_metric_start_kind UNIQUE (metric_id, start, kind)
);

-- ECG & Notifications
CREATE TABLE IF NOT EXISTS apple_health.ecg(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    classification TEXT,
    severity TEXT,
    average_heart_rate DOUBLE PRECISION,
    number_of_voltage_measurements INT,
    sampling_frequency DOUBLE PRECISION,
    source TEXT,
    voltage_measurements JSONB,
    CONSTRAINT uq_ecg_metric_start UNIQUE (metric_id, start)
);

CREATE TABLE IF NOT EXISTS apple_health.heart_rate_notification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID NOT NULL REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    threshold DOUBLE PRECISION,
    heart_rate JSONB,
    heart_rate_variation JSONB,
    CONSTRAINT uq_heart_rate_notification_metric_times UNIQUE (metric_id, start, "end")
);

-- Workouts
CREATE TABLE IF NOT EXISTS apple_health.workout (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payload_id UUID NOT NULL REFERENCES apple_health.health_payload(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    "start" TIMESTAMP WITH TIME ZONE NOT NULL,
    "end" TIMESTAMP WITH TIME ZONE NOT NULL,
    elevation JSONB,
    CONSTRAINT uq_workout_payload_name_start UNIQUE (payload_id, name, start)
);

CREATE TABLE IF NOT EXISTS apple_health.workout_value (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID NOT NULL REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    qty DOUBLE PRECISION,
    units TEXT,
    CONSTRAINT uq_workout_value_workout_name UNIQUE (workout_id, name)
);

CREATE TABLE IF NOT EXISTS apple_health.workout_point (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID NOT NULL REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    stream TEXT CHECK (stream IN ('heart_rate', 'heart_rate_recovery')) NOT NULL,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    units TEXT,
    CONSTRAINT uq_workout_point_workout_stream_date UNIQUE (workout_id, stream, date)
);

CREATE TABLE IF NOT EXISTS apple_health.workout_route_point (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_id UUID NOT NULL REFERENCES apple_health.workout(id) ON DELETE CASCADE,
    lat DOUBLE PRECISION NOT NULL,
    lon DOUBLE PRECISION NOT NULL,
    altitude DOUBLE PRECISION,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT uq_workout_route_point_workout_timestamp UNIQUE (workout_id, timestamp)
);

-- Performance Indexes
-- Core performance indexes
CREATE INDEX IF NOT EXISTS idx_health_payload_received_at ON apple_health.health_payload(received_at DESC);
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
CREATE INDEX IF NOT EXISTS idx_blood_glucose_metric_id ON apple_health.blood_glucose(metric_id);
CREATE INDEX IF NOT EXISTS idx_blood_glucose_date ON apple_health.blood_glucose(date DESC);

-- Workout indexes
CREATE INDEX IF NOT EXISTS idx_workout_payload_id ON apple_health.workout(payload_id);
CREATE INDEX IF NOT EXISTS idx_workout_start ON apple_health.workout(start DESC);
CREATE INDEX IF NOT EXISTS idx_workout_value_workout_id ON apple_health.workout_value(workout_id);
CREATE INDEX IF NOT EXISTS idx_workout_point_workout_id ON apple_health.workout_point(workout_id);
CREATE INDEX IF NOT EXISTS idx_workout_route_point_workout_id ON apple_health.workout_route_point(workout_id);