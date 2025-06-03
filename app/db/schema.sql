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

CREATE TABLE IF NOT EXISTS apple_health.quantity_timestamp (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    source TEXT
);

-- Specialized Metrics
CREATE TABLE IF NOT EXISTS apple_health.blood_pressure_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    systolic DOUBLE PRECISION NOT NULL,
    diastolic DOUBLE PRECISION NOT NULL
);

CREATE TABLE IF NOT EXISTS apple_health.heart_rate_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    min DOUBLE PRECISION,
    avg DOUBLE PRECISION,
    max DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS apple_health.sleep_analysis_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    asleep DOUBLE PRECISION,
    "sleep_start" TIMESTAMP WITH TIME ZONE,
    "sleep_end" TIMESTAMP WITH TIME ZONE,
    sleep_source TEXT,
    in_bed DOUBLE PRECISION,
    "in_bed_start" TIMESTAMP WITH TIME ZONE,
    "in_bed_end" TIMESTAMP WITH TIME ZONE,
    in_bed_source TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.blood_glucose_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION NOT NULL,
    meal_time TEXT CHECK (meal_time IN ('Before Meal', 'After Meal', 'Unspecified'))
);

CREATE TABLE IF NOT EXISTS apple_health.sexual_activity_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    unspecified DOUBLE PRECISION,
    protection_used DOUBLE PRECISION,
    protection_not_used DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS apple_health.hygiene_event_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    value TEXT CHECK (value IN ('Complete', 'Incomplete'))
);

CREATE TABLE IF NOT EXISTS apple_health.insulin_delivery_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    qty DOUBLE PRECISION,
    reason TEXT CHECK (reason IN ('Bolus', 'Basal'))
);

-- Mental Health
CREATE TABLE IF NOT EXISTS apple_health.symptom_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id UUID REFERENCES apple_health.health_metric(id) ON DELETE CASCADE,
    "start" TIMESTAMP WITH TIME ZONE,
    "end" TIMESTAMP WITH TIME ZONE,
    name TEXT,
    severity TEXT,
    user_entered BOOLEAN,
    source TEXT
);

CREATE TABLE IF NOT EXISTS apple_health.state_of_mind_data (
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
CREATE TABLE IF NOT EXISTS apple_health.ecg_data (
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
    stream TEXT CHECK (stream IN ('heart_rate_data', 'heart_rate_recovery')),
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
