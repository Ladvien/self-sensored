-- Create environmental_metrics Table Migration
-- Creates comprehensive environmental health tracking table with Apple Watch Series 8+ compatibility
-- Features: Audio exposure, UV tracking, fall detection, hygiene monitoring, air quality metrics

-- Create environmental_metrics table with Apple Health field names
CREATE TABLE environmental_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === AUDIO EXPOSURE METRICS ===
    -- Sound level exposure tracking (Apple Watch Series 8+)
    environmental_sound_level_db NUMERIC(5,2),     -- Environmental audio exposure (HKQuantityTypeIdentifierEnvironmentalAudioExposure)
    headphone_exposure_db NUMERIC(5,2),            -- Headphone audio exposure (HKQuantityTypeIdentifierHeadphoneAudioExposure)
    noise_reduction_db NUMERIC(5,2),               -- AirPods Pro noise reduction effectiveness
    exposure_duration_seconds INTEGER,              -- Duration of exposure measurement
    
    -- === UV EXPOSURE TRACKING ===
    -- UV index and sun exposure monitoring
    uv_index NUMERIC(4,2),                         -- UV Index (HKQuantityTypeIdentifierUVExposure) 0.0-11+
    time_in_sun_minutes INTEGER,                   -- Direct sun exposure duration
    time_in_shade_minutes INTEGER,                 -- Time spent in shade/indoors
    sunscreen_applied BOOLEAN,                     -- Sunscreen application tracking
    uv_dose_joules_per_m2 NUMERIC(10,2),          -- Total UV dose received
    
    -- === FALL DETECTION & SAFETY ===
    -- Fall detection events and safety metrics (Apple Watch Series 4+)
    fall_detected BOOLEAN DEFAULT FALSE,           -- Fall detection event occurred
    fall_severity VARCHAR(20) CHECK (fall_severity IN ('low', 'medium', 'high', 'severe')),
    impact_force_g NUMERIC(6,3),                  -- Impact force in G-forces (0-50G range)
    emergency_contacted BOOLEAN DEFAULT FALSE,     -- Emergency services contacted
    fall_response_time_seconds INTEGER,           -- Time to respond to fall detection
    
    -- === HYGIENE TRACKING ===
    -- Personal hygiene and health habits
    handwashing_events INTEGER,                   -- Hand washing frequency (Apple Watch Series 6+)
    handwashing_duration_seconds INTEGER,         -- Average handwashing duration (20+ seconds recommended)
    toothbrushing_events INTEGER,                 -- Toothbrushing sessions
    toothbrushing_duration_seconds INTEGER,       -- Toothbrushing duration tracking
    
    -- === AIR QUALITY METRICS ===
    -- Environmental air quality monitoring
    pm2_5_micrograms_m3 NUMERIC(8,2),            -- Fine particulate matter PM2.5
    pm10_micrograms_m3 NUMERIC(8,2),             -- Coarse particulate matter PM10
    air_quality_index INTEGER,                    -- AQI score (0-500)
    ozone_ppb NUMERIC(6,2),                       -- Ground-level ozone (parts per billion)
    no2_ppb NUMERIC(6,2),                         -- Nitrogen dioxide
    so2_ppb NUMERIC(6,2),                         -- Sulfur dioxide
    co_ppm NUMERIC(6,2),                          -- Carbon monoxide (parts per million)
    
    -- === LOCATION & CONTEXT ===
    -- Geographic and environmental context
    altitude_meters NUMERIC(8,2),                 -- Elevation above sea level
    barometric_pressure_hpa NUMERIC(7,2),         -- Atmospheric pressure
    indoor_outdoor_context VARCHAR(20) CHECK (indoor_outdoor_context IN ('indoor', 'outdoor', 'mixed', 'unknown')),
    
    -- === AGGREGATION SUPPORT ===
    -- Support for hourly, daily aggregation
    aggregation_period VARCHAR(20) DEFAULT 'event' CHECK (aggregation_period IN ('event', 'hourly', 'daily')),
    measurement_count INTEGER DEFAULT 1,           -- Number of measurements in aggregation
    
    -- === METADATA ===
    source VARCHAR(100),                          -- Data source (Apple Watch, iPhone, AirQuality API, etc.)
    device_type VARCHAR(50),                      -- Device type for compatibility tracking
    raw_data JSONB,                              -- Store original payload for data recovery
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    PRIMARY KEY (user_id, recorded_at),
    UNIQUE (user_id, recorded_at),
    
    -- === VALIDATION CONSTRAINTS ===
    -- Audio exposure constraints (0-140 dB, WHO safety guidelines)
    CONSTRAINT env_audio_environmental_check 
        CHECK (environmental_sound_level_db IS NULL OR 
               (environmental_sound_level_db >= 0 AND environmental_sound_level_db <= 140)),
    
    CONSTRAINT env_audio_headphone_check 
        CHECK (headphone_exposure_db IS NULL OR 
               (headphone_exposure_db >= 0 AND headphone_exposure_db <= 140)),
    
    CONSTRAINT env_audio_noise_reduction_check 
        CHECK (noise_reduction_db IS NULL OR 
               (noise_reduction_db >= 0 AND noise_reduction_db <= 60)),  -- Max ~60dB reduction
    
    CONSTRAINT env_exposure_duration_check 
        CHECK (exposure_duration_seconds IS NULL OR 
               (exposure_duration_seconds >= 0 AND exposure_duration_seconds <= 86400)),  -- Max 24 hours
    
    -- UV exposure constraints (UV Index 0-11+, extreme cases up to 15)
    CONSTRAINT env_uv_index_check 
        CHECK (uv_index IS NULL OR 
               (uv_index >= 0.0 AND uv_index <= 15.0)),
    
    CONSTRAINT env_sun_time_check 
        CHECK (time_in_sun_minutes IS NULL OR 
               (time_in_sun_minutes >= 0 AND time_in_sun_minutes <= 1440)),  -- Max 24 hours
    
    CONSTRAINT env_shade_time_check 
        CHECK (time_in_shade_minutes IS NULL OR 
               (time_in_shade_minutes >= 0 AND time_in_shade_minutes <= 1440)),  -- Max 24 hours
    
    CONSTRAINT env_uv_dose_check 
        CHECK (uv_dose_joules_per_m2 IS NULL OR 
               (uv_dose_joules_per_m2 >= 0 AND uv_dose_joules_per_m2 <= 100000)),  -- Extreme UV dose
    
    -- Fall detection constraints (impact force typically 0-50G for survivable falls)
    CONSTRAINT env_impact_force_check 
        CHECK (impact_force_g IS NULL OR 
               (impact_force_g >= 0 AND impact_force_g <= 50.0)),
    
    CONSTRAINT env_fall_response_check 
        CHECK (fall_response_time_seconds IS NULL OR 
               (fall_response_time_seconds >= 0 AND fall_response_time_seconds <= 3600)),  -- Max 1 hour
    
    -- Hygiene constraints (reasonable daily limits)
    CONSTRAINT env_handwashing_events_check 
        CHECK (handwashing_events IS NULL OR 
               (handwashing_events >= 0 AND handwashing_events <= 100)),  -- Max 100 times/day
    
    CONSTRAINT env_handwashing_duration_check 
        CHECK (handwashing_duration_seconds IS NULL OR 
               (handwashing_duration_seconds >= 0 AND handwashing_duration_seconds <= 300)),  -- Max 5 minutes
    
    CONSTRAINT env_toothbrushing_events_check 
        CHECK (toothbrushing_events IS NULL OR 
               (toothbrushing_events >= 0 AND toothbrushing_events <= 10)),  -- Max 10 times/day
    
    CONSTRAINT env_toothbrushing_duration_check 
        CHECK (toothbrushing_duration_seconds IS NULL OR 
               (toothbrushing_duration_seconds >= 0 AND toothbrushing_duration_seconds <= 600)),  -- Max 10 minutes
    
    -- Air quality constraints (WHO/EPA standards)
    CONSTRAINT env_pm2_5_check 
        CHECK (pm2_5_micrograms_m3 IS NULL OR 
               (pm2_5_micrograms_m3 >= 0 AND pm2_5_micrograms_m3 <= 1000)),  -- Extreme pollution
    
    CONSTRAINT env_pm10_check 
        CHECK (pm10_micrograms_m3 IS NULL OR 
               (pm10_micrograms_m3 >= 0 AND pm10_micrograms_m3 <= 1000)),
    
    CONSTRAINT env_aqi_check 
        CHECK (air_quality_index IS NULL OR 
               (air_quality_index >= 0 AND air_quality_index <= 500)),  -- EPA AQI scale
    
    CONSTRAINT env_ozone_check 
        CHECK (ozone_ppb IS NULL OR 
               (ozone_ppb >= 0 AND ozone_ppb <= 1000)),  -- Extreme ozone levels
    
    CONSTRAINT env_no2_check 
        CHECK (no2_ppb IS NULL OR 
               (no2_ppb >= 0 AND no2_ppb <= 2000)),  -- Extreme NO2 levels
    
    CONSTRAINT env_so2_check 
        CHECK (so2_ppb IS NULL OR 
               (so2_ppb >= 0 AND so2_ppb <= 1000)),
    
    CONSTRAINT env_co_check 
        CHECK (co_ppm IS NULL OR 
               (co_ppm >= 0 AND co_ppm <= 100)),  -- Dangerous CO levels
    
    -- Geographic constraints
    CONSTRAINT env_altitude_check 
        CHECK (altitude_meters IS NULL OR 
               (altitude_meters >= -500 AND altitude_meters <= 9000)),  -- Dead Sea to Everest range
    
    CONSTRAINT env_pressure_check 
        CHECK (barometric_pressure_hpa IS NULL OR 
               (barometric_pressure_hpa >= 800 AND barometric_pressure_hpa <= 1100)),  -- Realistic pressure range
    
    CONSTRAINT env_measurement_count_check 
        CHECK (measurement_count >= 1 AND measurement_count <= 10000)  -- Reasonable aggregation size
) PARTITION BY RANGE (recorded_at);

-- Create monthly partitions (current + 3 months ahead)
DO $$
DECLARE
    start_date DATE;
    end_date DATE;
    partition_name TEXT;
    month_start DATE;
    month_end DATE;
    i INTEGER;
BEGIN
    -- Start from beginning of current month, create 4 months of partitions
    start_date := DATE_TRUNC('month', CURRENT_DATE);
    
    FOR i IN 0..3 LOOP
        month_start := start_date + (i || ' months')::INTERVAL;
        month_end := month_start + '1 month'::INTERVAL;
        
        partition_name := 'environmental_metrics_' || TO_CHAR(month_start, 'YYYY_MM');
        
        EXECUTE format('CREATE TABLE %I PARTITION OF environmental_metrics 
                       FOR VALUES FROM (%L) TO (%L)',
                       partition_name, month_start, month_end);
        
        RAISE NOTICE 'Created partition: % for range % to %', partition_name, month_start, month_end;
    END LOOP;
END $$;

-- === INDEXES ===
-- Primary access patterns for environmental metrics

-- BRIN indexes for time-series data (efficient for time-based queries)
CREATE INDEX idx_environmental_metrics_recorded_at_brin 
    ON environmental_metrics USING BRIN (recorded_at);

CREATE INDEX idx_environmental_metrics_created_at_brin 
    ON environmental_metrics USING BRIN (created_at);

-- B-tree indexes for exact lookups and filtering
CREATE INDEX idx_environmental_metrics_user_recorded 
    ON environmental_metrics (user_id, recorded_at DESC);

CREATE INDEX idx_environmental_metrics_source 
    ON environmental_metrics (source);

CREATE INDEX idx_environmental_metrics_aggregation 
    ON environmental_metrics (aggregation_period, recorded_at);

-- Safety and alerting indexes
CREATE INDEX idx_environmental_metrics_fall_detection 
    ON environmental_metrics (user_id, fall_detected, recorded_at DESC) 
    WHERE fall_detected = TRUE;

CREATE INDEX idx_environmental_metrics_emergency_contact 
    ON environmental_metrics (user_id, emergency_contacted, recorded_at DESC) 
    WHERE emergency_contacted = TRUE;

-- Audio safety monitoring indexes
CREATE INDEX idx_environmental_metrics_high_audio_exposure 
    ON environmental_metrics (user_id, recorded_at DESC) 
    WHERE environmental_sound_level_db > 85 OR headphone_exposure_db > 85;

-- Air quality monitoring indexes
CREATE INDEX idx_environmental_metrics_poor_air_quality 
    ON environmental_metrics (user_id, recorded_at DESC) 
    WHERE air_quality_index > 100;

-- UV exposure monitoring indexes
CREATE INDEX idx_environmental_metrics_high_uv 
    ON environmental_metrics (user_id, recorded_at DESC) 
    WHERE uv_index > 6;

-- GIN indexes for JSONB raw_data queries
CREATE INDEX idx_environmental_metrics_raw_data_gin 
    ON environmental_metrics USING GIN (raw_data);

-- === SAFETY EVENT ALERTING HOOKS ===
-- Function to log safety events for monitoring and alerting

CREATE OR REPLACE FUNCTION log_environmental_safety_event()
RETURNS TRIGGER AS $$
BEGIN
    -- Log fall detection events
    IF NEW.fall_detected = TRUE THEN
        INSERT INTO safety_events (user_id, event_type, event_data, occurred_at)
        VALUES (NEW.user_id, 'fall_detected', 
                jsonb_build_object(
                    'severity', NEW.fall_severity,
                    'impact_force_g', NEW.impact_force_g,
                    'emergency_contacted', NEW.emergency_contacted,
                    'response_time_seconds', NEW.fall_response_time_seconds
                ), NEW.recorded_at);
    END IF;
    
    -- Log dangerous audio exposure levels (>85dB for extended periods)
    IF (NEW.environmental_sound_level_db > 85 AND NEW.exposure_duration_seconds > 900) OR  -- 15+ minutes
       (NEW.headphone_exposure_db > 85 AND NEW.exposure_duration_seconds > 900) THEN
        INSERT INTO safety_events (user_id, event_type, event_data, occurred_at)
        VALUES (NEW.user_id, 'dangerous_audio_exposure',
                jsonb_build_object(
                    'environmental_db', NEW.environmental_sound_level_db,
                    'headphone_db', NEW.headphone_exposure_db,
                    'duration_seconds', NEW.exposure_duration_seconds
                ), NEW.recorded_at);
    END IF;
    
    -- Log extreme UV exposure events (UV Index > 8)
    IF NEW.uv_index > 8 AND NEW.time_in_sun_minutes > 30 THEN
        INSERT INTO safety_events (user_id, event_type, event_data, occurred_at)
        VALUES (NEW.user_id, 'extreme_uv_exposure',
                jsonb_build_object(
                    'uv_index', NEW.uv_index,
                    'time_in_sun_minutes', NEW.time_in_sun_minutes,
                    'sunscreen_applied', NEW.sunscreen_applied
                ), NEW.recorded_at);
    END IF;
    
    -- Log dangerous air quality events (AQI > 200)
    IF NEW.air_quality_index > 200 THEN
        INSERT INTO safety_events (user_id, event_type, event_data, occurred_at)
        VALUES (NEW.user_id, 'dangerous_air_quality',
                jsonb_build_object(
                    'aqi', NEW.air_quality_index,
                    'pm2_5', NEW.pm2_5_micrograms_m3,
                    'pm10', NEW.pm10_micrograms_m3
                ), NEW.recorded_at);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create safety_events table if it doesn't exist
CREATE TABLE IF NOT EXISTS safety_events (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type VARCHAR(50) NOT NULL,
    event_data JSONB,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Index for quick safety event lookups
    INDEX idx_safety_events_user_type (user_id, event_type, occurred_at DESC)
);

-- Create trigger for safety event logging
CREATE TRIGGER trigger_environmental_safety_events
    AFTER INSERT ON environmental_metrics
    FOR EACH ROW
    EXECUTE FUNCTION log_environmental_safety_event();

-- === ANALYTICS VIEWS ===
-- Create helpful views for environmental data analysis

-- Hourly aggregation view for environmental monitoring
CREATE VIEW environmental_metrics_hourly AS
SELECT 
    user_id,
    DATE_TRUNC('hour', recorded_at) AS hour_bucket,
    COUNT(*) AS measurement_count,
    
    -- Audio exposure averages
    AVG(environmental_sound_level_db) AS avg_environmental_sound_db,
    MAX(environmental_sound_level_db) AS max_environmental_sound_db,
    AVG(headphone_exposure_db) AS avg_headphone_exposure_db,
    MAX(headphone_exposure_db) AS max_headphone_exposure_db,
    SUM(exposure_duration_seconds) AS total_exposure_duration_seconds,
    
    -- UV exposure totals
    AVG(uv_index) AS avg_uv_index,
    MAX(uv_index) AS max_uv_index,
    SUM(time_in_sun_minutes) AS total_sun_minutes,
    SUM(time_in_shade_minutes) AS total_shade_minutes,
    
    -- Safety events
    SUM(CASE WHEN fall_detected THEN 1 ELSE 0 END) AS fall_events,
    SUM(CASE WHEN emergency_contacted THEN 1 ELSE 0 END) AS emergency_contacts,
    
    -- Hygiene tracking
    SUM(handwashing_events) AS total_handwashing_events,
    AVG(handwashing_duration_seconds) AS avg_handwashing_duration,
    SUM(toothbrushing_events) AS total_toothbrushing_events,
    AVG(toothbrushing_duration_seconds) AS avg_toothbrushing_duration,
    
    -- Air quality averages
    AVG(pm2_5_micrograms_m3) AS avg_pm2_5,
    MAX(pm2_5_micrograms_m3) AS max_pm2_5,
    AVG(air_quality_index) AS avg_aqi,
    MAX(air_quality_index) AS max_aqi
    
FROM environmental_metrics
WHERE aggregation_period = 'event'  -- Only aggregate event-level data
GROUP BY user_id, DATE_TRUNC('hour', recorded_at);

-- Daily summary view for environmental health analysis
CREATE VIEW environmental_metrics_daily AS
SELECT 
    user_id,
    DATE_TRUNC('day', recorded_at) AS day_bucket,
    COUNT(*) AS measurement_count,
    
    -- Daily audio exposure summary
    AVG(environmental_sound_level_db) AS avg_environmental_sound_db,
    MAX(environmental_sound_level_db) AS max_environmental_sound_db,
    SUM(CASE WHEN environmental_sound_level_db > 85 THEN exposure_duration_seconds ELSE 0 END) AS dangerous_audio_exposure_seconds,
    
    -- Daily UV exposure summary
    MAX(uv_index) AS max_uv_index,
    SUM(time_in_sun_minutes) AS total_sun_minutes,
    AVG(CASE WHEN sunscreen_applied THEN 1.0 ELSE 0.0 END) AS sunscreen_usage_rate,
    
    -- Daily safety summary
    SUM(CASE WHEN fall_detected THEN 1 ELSE 0 END) AS fall_events,
    MAX(impact_force_g) AS max_impact_force_g,
    
    -- Daily hygiene summary
    SUM(handwashing_events) AS total_handwashing_events,
    SUM(toothbrushing_events) AS total_toothbrushing_events,
    
    -- Daily air quality summary
    AVG(air_quality_index) AS avg_aqi,
    MAX(air_quality_index) AS max_aqi,
    AVG(pm2_5_micrograms_m3) AS avg_pm2_5
    
FROM environmental_metrics
GROUP BY user_id, DATE_TRUNC('day', recorded_at);

-- === PERFORMANCE MONITORING ===
-- Function to monitor environmental metrics table performance

CREATE OR REPLACE FUNCTION monitor_environmental_metrics_performance()
RETURNS TABLE (
    metric_name TEXT,
    metric_value NUMERIC,
    last_updated TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        'total_records'::TEXT,
        COUNT(*)::NUMERIC,
        NOW()
    FROM environmental_metrics
    
    UNION ALL
    
    SELECT 
        'records_last_24h'::TEXT,
        COUNT(*)::NUMERIC,
        NOW()
    FROM environmental_metrics
    WHERE recorded_at > NOW() - INTERVAL '24 hours'
    
    UNION ALL
    
    SELECT 
        'fall_events_last_24h'::TEXT,
        SUM(CASE WHEN fall_detected THEN 1 ELSE 0 END)::NUMERIC,
        NOW()
    FROM environmental_metrics
    WHERE recorded_at > NOW() - INTERVAL '24 hours'
    
    UNION ALL
    
    SELECT 
        'dangerous_audio_events_last_24h'::TEXT,
        COUNT(*)::NUMERIC,
        NOW()
    FROM environmental_metrics
    WHERE recorded_at > NOW() - INTERVAL '24 hours'
      AND (environmental_sound_level_db > 85 OR headphone_exposure_db > 85);
END;
$$ LANGUAGE plpgsql;

-- === COMMENTS ===
COMMENT ON TABLE environmental_metrics IS 'Environmental health metrics from Apple Watch Series 8+ including audio exposure, UV tracking, fall detection, hygiene monitoring, and air quality data';

COMMENT ON COLUMN environmental_metrics.environmental_sound_level_db IS 'Environmental audio exposure in decibels (0-140 dB range, WHO safety guidelines)';
COMMENT ON COLUMN environmental_metrics.headphone_exposure_db IS 'Headphone audio exposure in decibels (0-140 dB range)';
COMMENT ON COLUMN environmental_metrics.uv_index IS 'UV Index measurement (0.0-15.0 range, WHO UV Index scale)';
COMMENT ON COLUMN environmental_metrics.fall_detected IS 'Fall detection event occurred (Apple Watch Series 4+)';
COMMENT ON COLUMN environmental_metrics.impact_force_g IS 'Impact force measurement in G-forces (0-50G survivable range)';
COMMENT ON COLUMN environmental_metrics.air_quality_index IS 'Air Quality Index (AQI) score (0-500 EPA scale)';
COMMENT ON COLUMN environmental_metrics.aggregation_period IS 'Data aggregation level: event (raw), hourly, or daily';

-- Migration completed successfully
-- Created environmental_metrics table with Apple Watch Series 8+ compatibility
-- Implemented comprehensive validation constraints for all metric types
-- Added monthly partitioning with BRIN indexes for time-series optimization
-- Created safety event alerting system for fall detection, audio exposure, UV, and air quality
-- Added analytics views for hourly and daily environmental health monitoring
-- Implemented performance monitoring functions