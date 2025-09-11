-- Create mental_health_metrics Table Migration
-- Creates comprehensive mental health tracking table with iOS 17+ support
-- Features: Mindfulness tracking, mood valence, mood labels, daylight exposure, stress levels

-- Create mental_health_metrics table with Apple Health mindfulness and mood tracking
CREATE TABLE mental_health_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === MINDFULNESS TRACKING ===
    -- Mindful session duration tracking
    mindful_minutes NUMERIC(8,2),            -- Minutes spent in mindful sessions (HKCategoryTypeIdentifierMindfulSession)
    
    -- === MOOD TRACKING (iOS 17+) ===
    -- Mood valence scale from -1.0 (very unpleasant) to 1.0 (very pleasant)
    mood_valence NUMERIC(3,2),               -- Mood valence score (HKCategoryTypeIdentifierStateOfMind)
    
    -- Mood labels array for emotional state categorization
    mood_labels TEXT[],                      -- Array of mood descriptors (happy, sad, anxious, calm, etc.)
    
    -- === DAYLIGHT EXPOSURE ===
    -- Time spent in daylight for circadian health
    daylight_minutes NUMERIC(8,2),          -- Minutes exposed to daylight
    
    -- === STRESS LEVEL TRACKING ===
    -- Stress level categorization
    stress_level VARCHAR(20) CHECK (stress_level IN ('low', 'medium', 'high', 'critical')),
    
    -- === MENTAL HEALTH SCREENING ===
    -- Depression and anxiety screening scores (PHQ-9, GAD-7, etc.)
    depression_score SMALLINT,               -- Depression screening score (0-27 for PHQ-9)
    anxiety_score SMALLINT,                  -- Anxiety screening score (0-21 for GAD-7)
    
    -- Sleep quality impact on mental health
    sleep_quality_score SMALLINT,           -- Subjective sleep quality (1-10 scale)
    
    -- === METADATA ===
    source VARCHAR(100),                     -- Data source (iOS Health, third-party apps, manual entry)
    raw_data JSONB,                         -- Store original payload for data recovery
    notes TEXT,                             -- Optional notes about mental state
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    PRIMARY KEY (user_id, recorded_at),
    UNIQUE (user_id, recorded_at),
    
    -- === VALIDATION CONSTRAINTS ===
    -- Mindful minutes constraints (0-1440 minutes max per day)
    CONSTRAINT mental_health_mindful_minutes_check 
        CHECK (mindful_minutes IS NULL OR (mindful_minutes >= 0 AND mindful_minutes <= 1440)),
    
    -- Mood valence constraints (-1.0 to 1.0 scale)
    CONSTRAINT mental_health_mood_valence_check 
        CHECK (mood_valence IS NULL OR (mood_valence >= -1.0 AND mood_valence <= 1.0)),
    
    -- Daylight minutes constraints (0-1440 minutes max per day)
    CONSTRAINT mental_health_daylight_minutes_check 
        CHECK (daylight_minutes IS NULL OR (daylight_minutes >= 0 AND daylight_minutes <= 1440)),
    
    -- Depression score constraints (PHQ-9 scale: 0-27)
    CONSTRAINT mental_health_depression_score_check 
        CHECK (depression_score IS NULL OR (depression_score >= 0 AND depression_score <= 27)),
    
    -- Anxiety score constraints (GAD-7 scale: 0-21)
    CONSTRAINT mental_health_anxiety_score_check 
        CHECK (anxiety_score IS NULL OR (anxiety_score >= 0 AND anxiety_score <= 21)),
    
    -- Sleep quality score constraints (1-10 scale)
    CONSTRAINT mental_health_sleep_quality_check 
        CHECK (sleep_quality_score IS NULL OR (sleep_quality_score >= 1 AND sleep_quality_score <= 10)),
    
    -- Ensure mood_labels array is not empty when provided
    CONSTRAINT mental_health_mood_labels_not_empty_check 
        CHECK (mood_labels IS NULL OR array_length(mood_labels, 1) > 0),
    
    -- Ensure mood_labels contain valid mood descriptors
    CONSTRAINT mental_health_mood_labels_valid_check 
        CHECK (mood_labels IS NULL OR 
               (SELECT bool_and(label ~ '^[a-zA-Z_]+$' AND length(label) <= 30) 
                FROM unnest(mood_labels) AS label))

) PARTITION BY RANGE (recorded_at);

-- === PARTITIONING SETUP ===
-- Create monthly partitions for optimal performance
-- Current month partition
CREATE TABLE mental_health_metrics_y2025m09 PARTITION OF mental_health_metrics
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

-- Next 3 months (auto-creation requirement)
CREATE TABLE mental_health_metrics_y2025m10 PARTITION OF mental_health_metrics
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
    
CREATE TABLE mental_health_metrics_y2025m11 PARTITION OF mental_health_metrics
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
    
CREATE TABLE mental_health_metrics_y2025m12 PARTITION OF mental_health_metrics
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- === INDEXING STRATEGY ===
-- BRIN indexes for time-series data (highly efficient for partitioned tables)
CREATE INDEX mental_health_metrics_recorded_at_brin_idx 
    ON mental_health_metrics USING BRIN (recorded_at);

-- B-tree indexes for common query patterns
CREATE INDEX mental_health_metrics_user_id_recorded_at_idx 
    ON mental_health_metrics (user_id, recorded_at);

CREATE INDEX mental_health_metrics_mood_valence_idx 
    ON mental_health_metrics (mood_valence) WHERE mood_valence IS NOT NULL;

CREATE INDEX mental_health_metrics_stress_level_idx 
    ON mental_health_metrics (stress_level) WHERE stress_level IS NOT NULL;

-- GIN indexes for array operations on mood_labels
CREATE INDEX mental_health_metrics_mood_labels_gin_idx 
    ON mental_health_metrics USING GIN (mood_labels);

-- GIN indexes for JSONB raw_data queries
CREATE INDEX mental_health_metrics_raw_data_gin_idx 
    ON mental_health_metrics USING GIN (raw_data);

-- === PERFORMANCE VIEWS ===
-- Daily mental health summary for trend analysis
CREATE VIEW mental_health_daily_summary AS
SELECT 
    user_id,
    DATE(recorded_at) as summary_date,
    AVG(mood_valence) as avg_mood_valence,
    SUM(mindful_minutes) as total_mindful_minutes,
    SUM(daylight_minutes) as total_daylight_minutes,
    MODE() WITHIN GROUP (ORDER BY stress_level) as most_common_stress_level,
    array_agg(DISTINCT mood_label) as all_mood_labels,
    AVG(depression_score) as avg_depression_score,
    AVG(anxiety_score) as avg_anxiety_score,
    AVG(sleep_quality_score) as avg_sleep_quality_score,
    COUNT(*) as entry_count
FROM mental_health_metrics
CROSS JOIN LATERAL unnest(COALESCE(mood_labels, ARRAY[]::TEXT[])) AS mood_label
GROUP BY user_id, DATE(recorded_at);

-- Mood trend analysis view for tracking emotional patterns
CREATE VIEW mental_health_mood_trends AS
SELECT 
    user_id,
    DATE_TRUNC('week', recorded_at) as week_start,
    AVG(mood_valence) as avg_weekly_mood,
    STDDEV(mood_valence) as mood_volatility,
    COUNT(DISTINCT mood_label) as mood_label_diversity,
    array_agg(DISTINCT stress_level) as stress_levels,
    SUM(mindful_minutes) as total_weekly_mindfulness
FROM mental_health_metrics
CROSS JOIN LATERAL unnest(COALESCE(mood_labels, ARRAY[]::TEXT[])) AS mood_label
WHERE mood_valence IS NOT NULL
GROUP BY user_id, DATE_TRUNC('week', recorded_at);

-- === PARTITION MANAGEMENT FUNCTION ===
-- Function to create new monthly partitions automatically
CREATE OR REPLACE FUNCTION create_mental_health_metrics_partition(partition_date DATE)
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
    partition_name := 'mental_health_metrics_y' || TO_CHAR(start_date, 'YYYY') || 'm' || TO_CHAR(start_date, 'MM');
    
    -- Create partition
    EXECUTE format('CREATE TABLE %I PARTITION OF mental_health_metrics FOR VALUES FROM (%L) TO (%L)',
                  partition_name, start_date, end_date);
                  
    -- Add indexes to new partition
    EXECUTE format('CREATE INDEX %I ON %I USING BRIN (recorded_at)', 
                  partition_name || '_recorded_at_brin_idx', partition_name);
    EXECUTE format('CREATE INDEX %I ON %I (user_id, recorded_at)', 
                  partition_name || '_user_id_recorded_at_idx', partition_name);
                  
    RAISE NOTICE 'Created partition % for mental_health_metrics', partition_name;
END;
$$ LANGUAGE plpgsql;

-- === PERFORMANCE MONITORING ===
-- Function to get mental health metrics table stats
CREATE OR REPLACE FUNCTION mental_health_metrics_stats()
RETURNS TABLE(
    partition_name TEXT,
    row_count BIGINT,
    size_mb NUMERIC,
    avg_mood_valence NUMERIC,
    unique_users BIGINT
) AS $$
BEGIN
    RETURN QUERY
    WITH partition_stats AS (
        SELECT 
            schemaname || '.' || tablename as full_name,
            tablename as partition_name
        FROM pg_tables 
        WHERE tablename LIKE 'mental_health_metrics_%'
    )
    SELECT 
        ps.partition_name::TEXT,
        (SELECT COUNT(*) FROM mental_health_metrics mhm 
         WHERE pg_relation_name(mhm.tableoid) = ps.partition_name)::BIGINT as row_count,
        (pg_total_relation_size(ps.full_name::regclass) / 1024 / 1024)::NUMERIC as size_mb,
        (SELECT AVG(mood_valence) FROM mental_health_metrics mhm 
         WHERE pg_relation_name(mhm.tableoid) = ps.partition_name 
         AND mood_valence IS NOT NULL)::NUMERIC as avg_mood_valence,
        (SELECT COUNT(DISTINCT user_id) FROM mental_health_metrics mhm 
         WHERE pg_relation_name(mhm.tableoid) = ps.partition_name)::BIGINT as unique_users
    FROM partition_stats ps
    ORDER BY ps.partition_name;
END;
$$ LANGUAGE plpgsql;

-- === COMMENTS FOR DOCUMENTATION ===
COMMENT ON TABLE mental_health_metrics IS 'Mental health and mindfulness tracking with iOS 17+ support for mood valence, mood labels, and comprehensive wellness metrics';
COMMENT ON COLUMN mental_health_metrics.mindful_minutes IS 'Duration of mindfulness sessions in minutes (HKCategoryTypeIdentifierMindfulSession)';
COMMENT ON COLUMN mental_health_metrics.mood_valence IS 'Mood valence score from -1.0 (very unpleasant) to 1.0 (very pleasant) - iOS 17+ feature';
COMMENT ON COLUMN mental_health_metrics.mood_labels IS 'Array of mood descriptors like happy, sad, anxious, calm - iOS 17+ feature';
COMMENT ON COLUMN mental_health_metrics.daylight_minutes IS 'Minutes of daylight exposure for circadian health tracking';
COMMENT ON COLUMN mental_health_metrics.stress_level IS 'Categorical stress level: low, medium, high, critical';
COMMENT ON COLUMN mental_health_metrics.depression_score IS 'Depression screening score (0-27, typically PHQ-9 scale)';
COMMENT ON COLUMN mental_health_metrics.anxiety_score IS 'Anxiety screening score (0-21, typically GAD-7 scale)';
COMMENT ON COLUMN mental_health_metrics.sleep_quality_score IS 'Subjective sleep quality rating (1-10 scale)';