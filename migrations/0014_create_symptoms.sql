-- Create symptoms Table Migration
-- Creates comprehensive symptom tracking table with 39+ Apple Health symptom types
-- Features: Severity tracking, duration measurement, notes field with monthly partitioning

-- Create symptoms table with Apple Health symptom type enumeration
CREATE TABLE symptoms (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === SYMPTOM TYPE ===
    -- Comprehensive Apple Health symptom enumeration (39+ types)
    symptom_type VARCHAR(50) NOT NULL CHECK (symptom_type IN (
        -- General/Constitutional Symptoms
        'fever',
        'fatigue',
        'weakness',
        'night_sweats',
        'chills',
        'malaise',
        'appetite_loss',
        'weight_loss',
        'weight_gain',
        
        -- Head & Neurological Symptoms
        'headache',
        'dizziness',
        'lightheadedness',
        'confusion',
        'memory_issues',
        'concentration_difficulty',
        'mood_changes',
        'anxiety',
        'depression',
        
        -- Respiratory Symptoms
        'cough',
        'shortness_of_breath',
        'chest_tightness_or_pain',
        'wheezing',
        'runny_nose',
        'sinus_congestion',
        'sneezing',
        'sore_throat',
        
        -- Gastrointestinal Symptoms
        'nausea',
        'vomiting',
        'abdominal_cramps',
        'bloating',
        'diarrhea',
        'constipation',
        'heartburn',
        'acid_reflux',
        'stomach_pain',
        'gas',
        'indigestion',
        
        -- Musculoskeletal & Pain
        'body_and_muscle_aches',
        'joint_pain',
        'back_pain',
        'neck_pain',
        'muscle_cramps',
        'stiffness',
        'swelling',
        
        -- Skin & Dermatological
        'dry_skin',
        'rash',
        'itching',
        'acne',
        'skin_irritation',
        
        -- Genitourinary & Reproductive
        'pelvic_pain',
        'vaginal_dryness',
        'bladder_incontinence',
        'frequent_urination',
        'painful_urination',
        
        -- Sleep & Rest
        'sleep_changes',
        'insomnia',
        'excessive_sleepiness',
        'sleep_disturbances',
        
        -- Sensory & Perception
        'vision_changes',
        'hearing_changes',
        'taste_changes',
        'smell_changes',
        
        -- Other Symptoms
        'hot_flashes',
        'cold_intolerance',
        'heat_intolerance',
        'hair_loss',
        'tremor',
        'irregular_heartbeat'
    )),
    
    -- === SEVERITY TRACKING ===
    -- Four-level severity scale matching Apple Health standards
    severity VARCHAR(20) NOT NULL DEFAULT 'not_present' CHECK (severity IN (
        'not_present',  -- Symptom is absent
        'mild',         -- Minimal impact on daily activities
        'moderate',     -- Noticeable impact, some limitation of activities
        'severe'        -- Significant impact, major limitation of activities
    )),
    
    -- === DURATION & TIMING ===
    -- Duration tracking in minutes for temporal analysis
    duration_minutes INTEGER CHECK (duration_minutes IS NULL OR (duration_minutes >= 0 AND duration_minutes <= 10080)), -- Max 1 week
    
    -- Symptom onset (if different from recorded_at)
    onset_at TIMESTAMPTZ,
    
    -- === ADDITIONAL CONTEXT ===
    -- Free-text notes for additional symptom context
    notes TEXT,
    
    -- Triggers or contributing factors (as JSON array)
    triggers JSONB,
    
    -- Medication or treatment taken (as JSON array)
    treatments JSONB,
    
    -- === METADATA ===
    source VARCHAR(100) DEFAULT 'manual',  -- Data source (manual, Apple Watch, third-party app)
    raw_data JSONB,                       -- Store original payload for data recovery
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    PRIMARY KEY (user_id, recorded_at, symptom_type),
    UNIQUE (user_id, recorded_at, symptom_type),
    
    -- === VALIDATION CONSTRAINTS ===
    -- Ensure onset is not in the future
    CONSTRAINT symptoms_onset_check 
        CHECK (onset_at IS NULL OR onset_at <= NOW()),
    
    -- Ensure recorded_at is reasonable (not too far in future/past)
    CONSTRAINT symptoms_recorded_at_check 
        CHECK (recorded_at >= '1900-01-01'::timestamptz AND recorded_at <= NOW() + interval '1 day')
    
) PARTITION BY RANGE (recorded_at);

-- === INDEXES FOR TIME-SERIES & SYMPTOM ANALYSIS ===
-- Create BRIN indexes for time-series optimization (most efficient for partitioned time-series data)
CREATE INDEX IF NOT EXISTS idx_symptoms_recorded_at_brin 
    ON symptoms USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_symptoms_user_recorded_brin 
    ON symptoms USING BRIN (user_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_symptoms_type_recorded_brin 
    ON symptoms USING BRIN (symptom_type, recorded_at);

-- Create composite B-tree indexes for frequent symptom analysis queries
CREATE INDEX IF NOT EXISTS idx_symptoms_user_type_recorded 
    ON symptoms (user_id, symptom_type, recorded_at);

CREATE INDEX IF NOT EXISTS idx_symptoms_user_severity_recorded 
    ON symptoms (user_id, severity, recorded_at) WHERE severity != 'not_present';

-- Index for symptom correlation analysis
CREATE INDEX IF NOT EXISTS idx_symptoms_user_onset 
    ON symptoms (user_id, onset_at) WHERE onset_at IS NOT NULL;

-- GIN index for triggers and treatments JSON queries
CREATE INDEX IF NOT EXISTS idx_symptoms_triggers 
    ON symptoms USING GIN (triggers) WHERE triggers IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_symptoms_treatments 
    ON symptoms USING GIN (treatments) WHERE treatments IS NOT NULL;

-- === MONTHLY PARTITIONING FUNCTIONS ===
-- Function to create monthly partitions specifically for symptoms
CREATE OR REPLACE FUNCTION create_symptoms_monthly_partitions(
    start_months_back integer DEFAULT 1,
    end_months_ahead integer DEFAULT 3
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
        partition_name := 'symptoms_' || to_char(start_date, 'YYYY_MM');
        
        -- Check if partition already exists
        IF NOT EXISTS (
            SELECT 1 FROM pg_class WHERE relname = partition_name
        ) THEN
            EXECUTE format('
                CREATE TABLE %I PARTITION OF symptoms
                FOR VALUES FROM (%L) TO (%L)',
                partition_name, start_date, end_date
            );
            
            -- Create indexes for this partition
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (recorded_at)',
                partition_name || '_recorded_at_brin', partition_name
            );
            
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, recorded_at)',
                partition_name || '_user_recorded_brin', partition_name
            );
            
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (symptom_type, recorded_at)',
                partition_name || '_type_recorded_brin', partition_name
            );
            
            EXECUTE format('
                CREATE INDEX IF NOT EXISTS %I ON %I (user_id, symptom_type, recorded_at)',
                partition_name || '_user_type_recorded', partition_name
            );
            
            RAISE NOTICE 'Created symptoms partition and indexes: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Create initial partitions (3 months ahead as per requirements)
SELECT create_symptoms_monthly_partitions();

-- === UPDATE MAINTENANCE FUNCTION ===
-- Update the main partition maintenance function to include symptoms
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for all partitioned tables
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
    
    -- Add new table maintenance
    PERFORM create_activity_v2_monthly_partitions();
    PERFORM create_nutrition_monthly_partitions();
    PERFORM create_symptoms_monthly_partitions();
END;
$$ LANGUAGE plpgsql;

-- === SYMPTOM ANALYSIS VIEWS ===
-- Create useful views for symptom analysis and tracking
CREATE VIEW symptoms_severity_summary AS
SELECT 
    user_id,
    date_trunc('day', recorded_at) as symptom_date,
    symptom_type,
    
    -- Severity analysis
    MAX(CASE WHEN severity = 'severe' THEN 3
             WHEN severity = 'moderate' THEN 2
             WHEN severity = 'mild' THEN 1
             ELSE 0 END) as max_severity_score,
    
    COUNT(*) FILTER (WHERE severity != 'not_present') as symptom_episodes,
    
    -- Duration analysis
    SUM(duration_minutes) FILTER (WHERE severity != 'not_present') as total_duration_minutes,
    AVG(duration_minutes) FILTER (WHERE severity != 'not_present') as avg_duration_minutes,
    
    -- Temporal analysis
    MIN(recorded_at) FILTER (WHERE severity != 'not_present') as first_episode,
    MAX(recorded_at) FILTER (WHERE severity != 'not_present') as last_episode,
    
    -- Context analysis
    array_agg(DISTINCT source) FILTER (WHERE severity != 'not_present') as data_sources,
    COUNT(*) FILTER (WHERE triggers IS NOT NULL) as episodes_with_triggers,
    COUNT(*) FILTER (WHERE treatments IS NOT NULL) as episodes_with_treatments

FROM symptoms
GROUP BY user_id, date_trunc('day', recorded_at), symptom_type;

-- === SYMPTOM CORRELATION VIEW ===
-- View for analyzing symptom patterns and correlations
CREATE VIEW symptoms_daily_summary AS
SELECT 
    user_id,
    date_trunc('day', recorded_at) as symptom_date,
    
    -- Count of different symptoms experienced
    COUNT(DISTINCT symptom_type) FILTER (WHERE severity != 'not_present') as unique_symptoms_count,
    
    -- Severity distribution
    COUNT(*) FILTER (WHERE severity = 'severe') as severe_symptoms_count,
    COUNT(*) FILTER (WHERE severity = 'moderate') as moderate_symptoms_count,
    COUNT(*) FILTER (WHERE severity = 'mild') as mild_symptoms_count,
    
    -- Most common symptoms
    array_agg(symptom_type ORDER BY 
        CASE WHEN severity = 'severe' THEN 3
             WHEN severity = 'moderate' THEN 2
             WHEN severity = 'mild' THEN 1
             ELSE 0 END DESC
    ) FILTER (WHERE severity != 'not_present') as symptoms_by_severity,
    
    -- Duration insights
    SUM(duration_minutes) FILTER (WHERE severity != 'not_present') as total_symptom_duration_minutes,
    
    -- Context insights
    COUNT(*) FILTER (WHERE triggers IS NOT NULL AND severity != 'not_present') as symptoms_with_triggers,
    COUNT(*) FILTER (WHERE treatments IS NOT NULL AND severity != 'not_present') as symptoms_with_treatments,
    
    -- Overall symptom burden score (weighted by severity)
    SUM(CASE WHEN severity = 'severe' THEN 3
             WHEN severity = 'moderate' THEN 2
             WHEN severity = 'mild' THEN 1
             ELSE 0 END) as daily_symptom_burden_score

FROM symptoms
GROUP BY user_id, date_trunc('day', recorded_at);

-- === PERFORMANCE MONITORING ===
-- Performance monitoring function for symptoms
CREATE OR REPLACE FUNCTION analyze_symptoms_performance()
RETURNS TABLE (
    table_name text,
    partition_count bigint,
    total_rows bigint,
    avg_rows_per_partition bigint,
    oldest_data timestamptz,
    newest_data timestamptz,
    unique_symptoms_tracked bigint,
    severity_distribution jsonb
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        'symptoms'::text as table_name,
        COUNT(*)::bigint as partition_count,
        SUM(n_tup_ins)::bigint as total_rows,
        CASE WHEN COUNT(*) > 0 THEN (SUM(n_tup_ins) / COUNT(*))::bigint ELSE 0 END as avg_rows_per_partition,
        (SELECT MIN(recorded_at) FROM symptoms) as oldest_data,
        (SELECT MAX(recorded_at) FROM symptoms) as newest_data,
        (SELECT COUNT(DISTINCT symptom_type) FROM symptoms WHERE severity != 'not_present') as unique_symptoms_tracked,
        (SELECT jsonb_object_agg(severity, count)
         FROM (SELECT severity, COUNT(*) as count 
               FROM symptoms 
               WHERE severity != 'not_present' 
               GROUP BY severity) s) as severity_distribution
    FROM pg_stat_user_tables 
    WHERE relname LIKE 'symptoms_%';
END;
$$ LANGUAGE plpgsql;

-- === DOCUMENTATION ===
-- Add comprehensive documentation
COMMENT ON TABLE symptoms IS 'Comprehensive Apple Health symptom tracking with 39+ symptom types including severity, duration, and context. Features monthly partitioning and BRIN indexes for time-series optimization.';

-- Field documentation
COMMENT ON COLUMN symptoms.symptom_type IS 'Apple Health symptom type enumeration (39+ types including fever, headache, nausea, fatigue, etc.)';
COMMENT ON COLUMN symptoms.severity IS 'Four-level severity scale: not_present, mild, moderate, severe';
COMMENT ON COLUMN symptoms.duration_minutes IS 'Symptom duration in minutes for temporal analysis';
COMMENT ON COLUMN symptoms.onset_at IS 'Symptom onset timestamp (if different from recorded_at)';
COMMENT ON COLUMN symptoms.notes IS 'Free-text notes for additional symptom context';
COMMENT ON COLUMN symptoms.triggers IS 'JSON array of potential triggers or contributing factors';
COMMENT ON COLUMN symptoms.treatments IS 'JSON array of medications or treatments taken';
COMMENT ON COLUMN symptoms.recorded_at IS 'Timestamp when symptom was recorded with timezone support';
COMMENT ON COLUMN symptoms.raw_data IS 'Original payload stored for data recovery and debugging';

-- Index documentation
COMMENT ON INDEX idx_symptoms_user_type_recorded IS 'Composite index optimized for symptom history queries by user and type';
COMMENT ON INDEX idx_symptoms_user_severity_recorded IS 'Index optimized for severity-based symptom analysis (excludes not_present)';
COMMENT ON INDEX idx_symptoms_triggers IS 'GIN index for efficient JSON queries on symptom triggers';
COMMENT ON INDEX idx_symptoms_treatments IS 'GIN index for efficient JSON queries on symptom treatments';