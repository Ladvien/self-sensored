-- Create mobility_metrics Table Migration
-- Creates comprehensive mobility tracking table with iOS 14+ compatibility
-- Features: Walking speed, step length, asymmetry, stair speeds, six-minute walk test, walking steadiness

-- Create mobility_metrics table with Apple Health iOS 14+ mobility field names
CREATE TABLE mobility_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === WALKING SPEED & PACE ANALYSIS ===
    -- Walking speed measurements for gait analysis (iOS 14+)
    walking_speed_m_s NUMERIC(6,4),                 -- Average walking speed (HKQuantityTypeIdentifierWalkingSpeed) in m/s
    
    -- === GAIT PATTERN ANALYSIS ===
    -- Step length and pattern measurements
    walking_step_length_cm NUMERIC(7,3),            -- Average step length (HKQuantityTypeIdentifierWalkingStepLength) in cm
    walking_asymmetry_percentage NUMERIC(5,2),      -- Walking asymmetry (HKQuantityTypeIdentifierWalkingAsymmetryPercentage) 0-100%
    double_support_percentage NUMERIC(5,2),         -- Double support time (HKQuantityTypeIdentifierWalkingDoubleSupportPercentage) 0-100%
    
    -- === FUNCTIONAL MOBILITY TESTING ===
    -- Six-minute walk test for cardiovascular fitness
    six_minute_walk_distance_m NUMERIC(8,2),        -- Six-minute walk test distance (HKQuantityTypeIdentifierSixMinuteWalkTestDistance) in meters
    
    -- === STAIR CLIMBING PERFORMANCE ===
    -- Functional mobility assessment via stair climbing speeds (iOS 14+)
    stair_ascent_speed_m_s NUMERIC(6,4),           -- Stair climbing speed (HKQuantityTypeIdentifierStairAscentSpeed) in m/s
    stair_descent_speed_m_s NUMERIC(6,4),          -- Stair descending speed (HKQuantityTypeIdentifierStairDescentSpeed) in m/s
    
    -- === WALKING STABILITY ANALYSIS ===
    -- Apple proprietary walking steadiness score (iOS 15+)
    walking_steadiness_score NUMERIC(4,3),         -- Walking steadiness (HKQuantityTypeIdentifierAppleWalkingSteadiness) 0-1 scale
    walking_steadiness_classification VARCHAR(20),  -- Classification: OK, Low, Very Low
    
    -- === ADDITIONAL MOBILITY METRICS ===
    -- Additional gait and mobility parameters
    cadence_steps_per_minute NUMERIC(6,2),         -- Walking cadence (steps per minute)
    stride_length_cm NUMERIC(7,3),                 -- Stride length (double step length)
    ground_contact_time_ms NUMERIC(7,2),           -- Ground contact time per step
    vertical_oscillation_cm NUMERIC(5,2),          -- Vertical movement during walking
    
    -- === BALANCE AND POSTURAL CONTROL ===
    -- Balance-related metrics
    postural_sway_mm NUMERIC(6,2),                 -- Postural sway measurement
    balance_confidence_score SMALLINT,             -- Activities-specific Balance Confidence Scale (0-100)
    fall_risk_score SMALLINT,                      -- Calculated fall risk score (0-100)
    
    -- === ENVIRONMENTAL CONTEXT ===
    -- Context and conditions during measurement
    surface_type VARCHAR(30) CHECK (surface_type IN ('flat', 'inclined', 'uneven', 'treadmill', 'outdoor', 'indoor')),
    measurement_duration_seconds INTEGER,           -- Duration of measurement period
    measurement_distance_m NUMERIC(8,2),           -- Distance covered during measurement
    
    -- === AGGREGATION SUPPORT ===
    -- Support for temporal aggregation
    aggregation_period VARCHAR(20) DEFAULT 'measurement' CHECK (aggregation_period IN ('measurement', 'hourly', 'daily', 'weekly')),
    measurement_count INTEGER DEFAULT 1,           -- Number of measurements in aggregation
    
    -- === METADATA ===
    source VARCHAR(100),                           -- Data source (iPhone, Apple Watch, clinical assessment, etc.)
    device_model VARCHAR(50),                      -- Device model for compatibility tracking
    ios_version VARCHAR(20),                       -- iOS version for feature compatibility
    raw_data JSONB,                               -- Store original payload for data recovery
    notes TEXT,                                   -- Clinical or assessment notes
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    PRIMARY KEY (user_id, recorded_at),
    UNIQUE (user_id, recorded_at),
    
    -- === VALIDATION CONSTRAINTS ===
    -- Walking speed constraints (0.1-5.0 m/s, typical human range)
    CONSTRAINT mobility_walking_speed_check 
        CHECK (walking_speed_m_s IS NULL OR (walking_speed_m_s >= 0.1 AND walking_speed_m_s <= 5.0)),
    
    -- Step length constraints (10-150 cm, biomechanically reasonable)
    CONSTRAINT mobility_step_length_check 
        CHECK (walking_step_length_cm IS NULL OR (walking_step_length_cm >= 10.0 AND walking_step_length_cm <= 150.0)),
    
    -- Walking asymmetry percentage (0-100%, lower is better)
    CONSTRAINT mobility_walking_asymmetry_check 
        CHECK (walking_asymmetry_percentage IS NULL OR (walking_asymmetry_percentage >= 0.0 AND walking_asymmetry_percentage <= 100.0)),
    
    -- Double support percentage (10-50%, typical range for healthy walking)
    CONSTRAINT mobility_double_support_check 
        CHECK (double_support_percentage IS NULL OR (double_support_percentage >= 5.0 AND double_support_percentage <= 60.0)),
    
    -- Six-minute walk test distance (50-1000m, wide range for different fitness levels)
    CONSTRAINT mobility_six_minute_walk_check 
        CHECK (six_minute_walk_distance_m IS NULL OR (six_minute_walk_distance_m >= 50.0 AND six_minute_walk_distance_m <= 1000.0)),
    
    -- Stair ascent speed (0.1-2.0 m/s, reasonable range for stair climbing)
    CONSTRAINT mobility_stair_ascent_speed_check 
        CHECK (stair_ascent_speed_m_s IS NULL OR (stair_ascent_speed_m_s >= 0.1 AND stair_ascent_speed_m_s <= 2.0)),
    
    -- Stair descent speed (0.1-2.5 m/s, typically faster than ascent)
    CONSTRAINT mobility_stair_descent_speed_check 
        CHECK (stair_descent_speed_m_s IS NULL OR (stair_descent_speed_m_s >= 0.1 AND stair_descent_speed_m_s <= 2.5)),
    
    -- Walking steadiness score (0.0-1.0, Apple's proprietary scale)
    CONSTRAINT mobility_walking_steadiness_score_check 
        CHECK (walking_steadiness_score IS NULL OR (walking_steadiness_score >= 0.0 AND walking_steadiness_score <= 1.0)),
    
    -- Walking steadiness classification validation
    CONSTRAINT mobility_walking_steadiness_classification_check 
        CHECK (walking_steadiness_classification IS NULL OR 
               walking_steadiness_classification IN ('OK', 'Low', 'Very Low')),
    
    -- Cadence constraints (60-200 steps per minute, typical walking range)
    CONSTRAINT mobility_cadence_check 
        CHECK (cadence_steps_per_minute IS NULL OR (cadence_steps_per_minute >= 60.0 AND cadence_steps_per_minute <= 200.0)),
    
    -- Stride length constraints (20-300 cm, double the step length range)
    CONSTRAINT mobility_stride_length_check 
        CHECK (stride_length_cm IS NULL OR (stride_length_cm >= 20.0 AND stride_length_cm <= 300.0)),
    
    -- Ground contact time (100-400 ms, typical range for walking)
    CONSTRAINT mobility_ground_contact_time_check 
        CHECK (ground_contact_time_ms IS NULL OR (ground_contact_time_ms >= 100.0 AND ground_contact_time_ms <= 400.0)),
    
    -- Vertical oscillation (2-15 cm, typical bounce during walking)
    CONSTRAINT mobility_vertical_oscillation_check 
        CHECK (vertical_oscillation_cm IS NULL OR (vertical_oscillation_cm >= 1.0 AND vertical_oscillation_cm <= 15.0)),
    
    -- Postural sway (0-50 mm, stability measurement)
    CONSTRAINT mobility_postural_sway_check 
        CHECK (postural_sway_mm IS NULL OR (postural_sway_mm >= 0.0 AND postural_sway_mm <= 50.0)),
    
    -- Balance confidence score (0-100, ABC Scale)
    CONSTRAINT mobility_balance_confidence_check 
        CHECK (balance_confidence_score IS NULL OR (balance_confidence_score >= 0 AND balance_confidence_score <= 100)),
    
    -- Fall risk score (0-100, higher values indicate higher risk)
    CONSTRAINT mobility_fall_risk_check 
        CHECK (fall_risk_score IS NULL OR (fall_risk_score >= 0 AND fall_risk_score <= 100)),
    
    -- Measurement duration (1 second to 24 hours)
    CONSTRAINT mobility_measurement_duration_check 
        CHECK (measurement_duration_seconds IS NULL OR (measurement_duration_seconds >= 1 AND measurement_duration_seconds <= 86400)),
    
    -- Measurement distance (0.1m to 10km)
    CONSTRAINT mobility_measurement_distance_check 
        CHECK (measurement_distance_m IS NULL OR (measurement_distance_m >= 0.1 AND measurement_distance_m <= 10000.0)),
    
    -- Measurement count must be positive
    CONSTRAINT mobility_measurement_count_check 
        CHECK (measurement_count > 0),
    
    -- Logical consistency: stride length should be approximately 2x step length
    CONSTRAINT mobility_stride_step_consistency_check 
        CHECK (walking_step_length_cm IS NULL OR stride_length_cm IS NULL OR 
               (stride_length_cm >= walking_step_length_cm * 1.5 AND stride_length_cm <= walking_step_length_cm * 2.5))

) PARTITION BY RANGE (recorded_at);

-- === PARTITIONING SETUP ===
-- Create monthly partitions for optimal performance with high-frequency mobility sampling
-- Current month partition
CREATE TABLE mobility_metrics_y2025m09 PARTITION OF mobility_metrics
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

-- Next 3 months (auto-creation requirement)
CREATE TABLE mobility_metrics_y2025m10 PARTITION OF mobility_metrics
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
    
CREATE TABLE mobility_metrics_y2025m11 PARTITION OF mobility_metrics
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
    
CREATE TABLE mobility_metrics_y2025m12 PARTITION OF mobility_metrics
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- === INDEXING STRATEGY ===
-- BRIN indexes for time-series data (highly efficient for partitioned tables)
CREATE INDEX mobility_metrics_recorded_at_brin_idx 
    ON mobility_metrics USING BRIN (recorded_at);

-- B-tree indexes for common query patterns
CREATE INDEX mobility_metrics_user_id_recorded_at_idx 
    ON mobility_metrics (user_id, recorded_at);

CREATE INDEX mobility_metrics_walking_speed_idx 
    ON mobility_metrics (walking_speed_m_s) WHERE walking_speed_m_s IS NOT NULL;

CREATE INDEX mobility_metrics_walking_asymmetry_idx 
    ON mobility_metrics (walking_asymmetry_percentage) WHERE walking_asymmetry_percentage IS NOT NULL;

CREATE INDEX mobility_metrics_walking_steadiness_idx 
    ON mobility_metrics (walking_steadiness_score, walking_steadiness_classification) 
    WHERE walking_steadiness_score IS NOT NULL;

CREATE INDEX mobility_metrics_six_minute_walk_idx 
    ON mobility_metrics (six_minute_walk_distance_m) WHERE six_minute_walk_distance_m IS NOT NULL;

CREATE INDEX mobility_metrics_stair_speeds_idx 
    ON mobility_metrics (stair_ascent_speed_m_s, stair_descent_speed_m_s) 
    WHERE stair_ascent_speed_m_s IS NOT NULL OR stair_descent_speed_m_s IS NOT NULL;

CREATE INDEX mobility_metrics_surface_type_idx 
    ON mobility_metrics (surface_type) WHERE surface_type IS NOT NULL;

-- GIN indexes for JSONB raw_data queries
CREATE INDEX mobility_metrics_raw_data_gin_idx 
    ON mobility_metrics USING GIN (raw_data);

-- Composite indexes for clinical analysis
CREATE INDEX mobility_metrics_gait_analysis_idx 
    ON mobility_metrics (user_id, walking_speed_m_s, walking_step_length_cm, walking_asymmetry_percentage, recorded_at) 
    WHERE walking_speed_m_s IS NOT NULL AND walking_step_length_cm IS NOT NULL;

CREATE INDEX mobility_metrics_fall_risk_analysis_idx 
    ON mobility_metrics (user_id, walking_steadiness_score, balance_confidence_score, fall_risk_score, recorded_at) 
    WHERE walking_steadiness_score IS NOT NULL OR balance_confidence_score IS NOT NULL;

-- === PERFORMANCE VIEWS ===
-- Daily mobility summary for trend analysis
CREATE VIEW mobility_daily_summary AS
SELECT 
    user_id,
    DATE(recorded_at) as summary_date,
    AVG(walking_speed_m_s) as avg_walking_speed_m_s,
    AVG(walking_step_length_cm) as avg_walking_step_length_cm,
    AVG(walking_asymmetry_percentage) as avg_walking_asymmetry_percentage,
    AVG(double_support_percentage) as avg_double_support_percentage,
    MAX(six_minute_walk_distance_m) as best_six_minute_walk_distance_m,
    AVG(stair_ascent_speed_m_s) as avg_stair_ascent_speed_m_s,
    AVG(stair_descent_speed_m_s) as avg_stair_descent_speed_m_s,
    AVG(walking_steadiness_score) as avg_walking_steadiness_score,
    MODE() WITHIN GROUP (ORDER BY walking_steadiness_classification) as most_common_steadiness_class,
    AVG(cadence_steps_per_minute) as avg_cadence_steps_per_minute,
    AVG(balance_confidence_score) as avg_balance_confidence_score,
    AVG(fall_risk_score) as avg_fall_risk_score,
    SUM(measurement_duration_seconds) as total_measurement_duration_seconds,
    SUM(measurement_distance_m) as total_measurement_distance_m,
    COUNT(*) as measurement_count
FROM mobility_metrics
GROUP BY user_id, DATE(recorded_at);

-- Gait pattern analysis view for clinical assessment
CREATE VIEW mobility_gait_analysis AS
SELECT 
    user_id,
    DATE_TRUNC('week', recorded_at) as week_start,
    AVG(walking_speed_m_s) as avg_walking_speed,
    STDDEV(walking_speed_m_s) as walking_speed_variability,
    AVG(walking_step_length_cm) as avg_step_length,
    STDDEV(walking_step_length_cm) as step_length_variability,
    AVG(walking_asymmetry_percentage) as avg_asymmetry,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY walking_asymmetry_percentage) as p95_asymmetry,
    AVG(double_support_percentage) as avg_double_support,
    AVG(cadence_steps_per_minute) as avg_cadence,
    COUNT(*) as measurement_count,
    -- Gait quality scoring (lower asymmetry = better, optimal double support ~20%)
    CASE 
        WHEN AVG(walking_asymmetry_percentage) < 3.0 AND AVG(double_support_percentage) BETWEEN 15 AND 25 THEN 'Excellent'
        WHEN AVG(walking_asymmetry_percentage) < 5.0 AND AVG(double_support_percentage) BETWEEN 12 AND 30 THEN 'Good'
        WHEN AVG(walking_asymmetry_percentage) < 10.0 THEN 'Fair'
        ELSE 'Poor'
    END as gait_quality
FROM mobility_metrics
WHERE walking_speed_m_s IS NOT NULL AND walking_asymmetry_percentage IS NOT NULL
GROUP BY user_id, DATE_TRUNC('week', recorded_at);

-- Fall risk assessment view combining multiple mobility factors
CREATE VIEW mobility_fall_risk_assessment AS
SELECT 
    user_id,
    DATE_TRUNC('month', recorded_at) as month_start,
    AVG(walking_steadiness_score) as avg_walking_steadiness,
    AVG(walking_speed_m_s) as avg_walking_speed,
    AVG(walking_asymmetry_percentage) as avg_asymmetry,
    AVG(balance_confidence_score) as avg_balance_confidence,
    AVG(fall_risk_score) as avg_fall_risk_score,
    -- Composite fall risk calculation
    CASE 
        WHEN AVG(walking_steadiness_score) >= 0.8 AND AVG(walking_speed_m_s) >= 1.2 AND 
             AVG(walking_asymmetry_percentage) <= 5.0 THEN 'Low Risk'
        WHEN AVG(walking_steadiness_score) >= 0.6 AND AVG(walking_speed_m_s) >= 1.0 AND 
             AVG(walking_asymmetry_percentage) <= 10.0 THEN 'Moderate Risk'
        ELSE 'High Risk'
    END as composite_fall_risk,
    COUNT(*) as measurement_count
FROM mobility_metrics
WHERE walking_steadiness_score IS NOT NULL OR walking_speed_m_s IS NOT NULL
GROUP BY user_id, DATE_TRUNC('month', recorded_at);

-- === PARTITION MANAGEMENT FUNCTION ===
-- Function to create new monthly partitions automatically
CREATE OR REPLACE FUNCTION create_mobility_metrics_partition(partition_date DATE)
RETURNS void AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    -- Calculate partition bounds (monthly)
    start_date := DATE_TRUNC('month', partition_date)::DATE;
    end_date := (DATE_TRUNC('month', partition_date) + INTERVAL '1 month')::DATE;
    
    -- Generate partition name
    partition_name := 'mobility_metrics_y' || TO_CHAR(start_date, 'YYYY') || 'm' || TO_CHAR(start_date, 'MM');
    
    -- Create partition
    EXECUTE format('CREATE TABLE %I PARTITION OF mobility_metrics FOR VALUES FROM (%L) TO (%L)',
                  partition_name, start_date, end_date);
                  
    -- Add indexes to new partition
    EXECUTE format('CREATE INDEX %I ON %I USING BRIN (recorded_at)', 
                  partition_name || '_recorded_at_brin_idx', partition_name);
    EXECUTE format('CREATE INDEX %I ON %I (user_id, recorded_at)', 
                  partition_name || '_user_id_recorded_at_idx', partition_name);
    EXECUTE format('CREATE INDEX %I ON %I (walking_speed_m_s) WHERE walking_speed_m_s IS NOT NULL', 
                  partition_name || '_walking_speed_idx', partition_name);
                  
    RAISE NOTICE 'Created partition % for mobility_metrics', partition_name;
END;
$$ LANGUAGE plpgsql;

-- === PERFORMANCE MONITORING ===
-- Function to get mobility metrics table stats
CREATE OR REPLACE FUNCTION mobility_metrics_stats()
RETURNS TABLE(
    partition_name TEXT,
    row_count BIGINT,
    size_mb NUMERIC,
    avg_walking_speed NUMERIC,
    avg_asymmetry NUMERIC,
    unique_users BIGINT
) AS $$
BEGIN
    RETURN QUERY
    WITH partition_stats AS (
        SELECT 
            schemaname || '.' || tablename as full_name,
            tablename as partition_name
        FROM pg_tables 
        WHERE tablename LIKE 'mobility_metrics_%'
    )
    SELECT 
        ps.partition_name::TEXT,
        (SELECT COUNT(*) FROM mobility_metrics mm 
         WHERE pg_relation_name(mm.tableoid) = ps.partition_name)::BIGINT as row_count,
        (pg_total_relation_size(ps.full_name::regclass) / 1024 / 1024)::NUMERIC as size_mb,
        (SELECT AVG(walking_speed_m_s) FROM mobility_metrics mm 
         WHERE pg_relation_name(mm.tableoid) = ps.partition_name 
         AND walking_speed_m_s IS NOT NULL)::NUMERIC as avg_walking_speed,
        (SELECT AVG(walking_asymmetry_percentage) FROM mobility_metrics mm 
         WHERE pg_relation_name(mm.tableoid) = ps.partition_name 
         AND walking_asymmetry_percentage IS NOT NULL)::NUMERIC as avg_asymmetry,
        (SELECT COUNT(DISTINCT user_id) FROM mobility_metrics mm 
         WHERE pg_relation_name(mm.tableoid) = ps.partition_name)::BIGINT as unique_users
    FROM partition_stats ps
    ORDER BY ps.partition_name;
END;
$$ LANGUAGE plpgsql;

-- === COMMENTS FOR DOCUMENTATION ===
COMMENT ON TABLE mobility_metrics IS 'Comprehensive mobility tracking with iOS 14+ compatibility including walking speed, gait analysis, stair climbing speeds, and fall risk assessment';
COMMENT ON COLUMN mobility_metrics.walking_speed_m_s IS 'Average walking speed (HKQuantityTypeIdentifierWalkingSpeed) measured on flat ground with iPhone in pocket';
COMMENT ON COLUMN mobility_metrics.walking_step_length_cm IS 'Average step length (HKQuantityTypeIdentifierWalkingStepLength) measured during steady walking';
COMMENT ON COLUMN mobility_metrics.walking_asymmetry_percentage IS 'Walking asymmetry percentage (HKQuantityTypeIdentifierWalkingAsymmetryPercentage) - lower values indicate more symmetric gait';
COMMENT ON COLUMN mobility_metrics.double_support_percentage IS 'Double support time percentage (HKQuantityTypeIdentifierWalkingDoubleSupportPercentage) - time with both feet on ground';
COMMENT ON COLUMN mobility_metrics.six_minute_walk_distance_m IS 'Six-minute walk test distance (HKQuantityTypeIdentifierSixMinuteWalkTestDistance) for cardiovascular fitness assessment';
COMMENT ON COLUMN mobility_metrics.stair_ascent_speed_m_s IS 'Stair climbing speed (HKQuantityTypeIdentifierStairAscentSpeed) for functional mobility assessment';
COMMENT ON COLUMN mobility_metrics.stair_descent_speed_m_s IS 'Stair descending speed (HKQuantityTypeIdentifierStairDescentSpeed) for functional mobility assessment';
COMMENT ON COLUMN mobility_metrics.walking_steadiness_score IS 'Apple walking steadiness score (HKQuantityTypeIdentifierAppleWalkingSteadiness) 0-1 scale, iOS 15+';
COMMENT ON COLUMN mobility_metrics.walking_steadiness_classification IS 'Apple walking steadiness classification: OK, Low, Very Low - for fall risk assessment';