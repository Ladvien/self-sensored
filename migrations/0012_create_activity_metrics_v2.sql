-- Create activity_metrics_v2 Table Migration
-- Creates the new activity_metrics_v2 table with Apple Health standard naming conventions
-- Features: TIMESTAMPTZ for granularity, monthly partitioning, BRIN indexes, Apple Fitness fields

-- Create activity_metrics_v2 table with Apple Health field names
CREATE TABLE activity_metrics_v2 (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- Basic Activity Fields (from Apple Health)
    step_count INTEGER,
    flights_climbed INTEGER,
    
    -- Distance Fields by Activity Type
    distance_walking_running_meters NUMERIC(10,2),
    distance_cycling_meters NUMERIC(10,2),
    distance_swimming_meters NUMERIC(10,2),
    distance_wheelchair_meters NUMERIC(10,2),
    distance_downhill_snow_sports_meters NUMERIC(10,2),
    
    -- Additional Activity Counts
    push_count INTEGER,  -- wheelchair pushes
    swimming_stroke_count INTEGER,
    nike_fuel NUMERIC(8,2),  -- Nike Fuel points
    
    -- Energy Fields (Apple Health standard names)
    active_energy_burned_kcal NUMERIC(8,2),
    basal_energy_burned_kcal NUMERIC(8,2),
    
    -- Apple Fitness Ring Metrics
    exercise_time_minutes INTEGER,
    stand_time_minutes INTEGER,
    move_time_minutes INTEGER,
    stand_hour_achieved BOOLEAN DEFAULT FALSE,
    
    -- Data tracking and aggregation
    aggregation_period VARCHAR(20) DEFAULT 'hourly' CHECK (aggregation_period IN ('minute', 'hourly', 'daily', 'weekly')),
    source VARCHAR(100),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    PRIMARY KEY (user_id, recorded_at),
    
    -- Validation constraints based on Health Export limits
    CONSTRAINT activity_v2_step_count_check 
        CHECK (step_count IS NULL OR (step_count >= 0 AND step_count <= 200000)),
    CONSTRAINT activity_v2_flights_climbed_check 
        CHECK (flights_climbed IS NULL OR (flights_climbed >= 0 AND flights_climbed <= 10000)),
    CONSTRAINT activity_v2_push_count_check 
        CHECK (push_count IS NULL OR (push_count >= 0 AND push_count <= 50000)),
    CONSTRAINT activity_v2_swimming_stroke_count_check 
        CHECK (swimming_stroke_count IS NULL OR (swimming_stroke_count >= 0 AND swimming_stroke_count <= 100000)),
    CONSTRAINT activity_v2_distance_walking_running_check 
        CHECK (distance_walking_running_meters IS NULL OR (distance_walking_running_meters >= 0 AND distance_walking_running_meters <= 500000)),
    CONSTRAINT activity_v2_distance_cycling_check 
        CHECK (distance_cycling_meters IS NULL OR (distance_cycling_meters >= 0 AND distance_cycling_meters <= 1000000)),
    CONSTRAINT activity_v2_distance_swimming_check 
        CHECK (distance_swimming_meters IS NULL OR (distance_swimming_meters >= 0 AND distance_swimming_meters <= 50000)),
    CONSTRAINT activity_v2_distance_wheelchair_check 
        CHECK (distance_wheelchair_meters IS NULL OR (distance_wheelchair_meters >= 0 AND distance_wheelchair_meters <= 500000)),
    CONSTRAINT activity_v2_distance_snow_sports_check 
        CHECK (distance_downhill_snow_sports_meters IS NULL OR (distance_downhill_snow_sports_meters >= 0 AND distance_downhill_snow_sports_meters <= 200000)),
    CONSTRAINT activity_v2_active_energy_check 
        CHECK (active_energy_burned_kcal IS NULL OR (active_energy_burned_kcal >= 0 AND active_energy_burned_kcal <= 20000)),
    CONSTRAINT activity_v2_basal_energy_check 
        CHECK (basal_energy_burned_kcal IS NULL OR (basal_energy_burned_kcal >= 0 AND basal_energy_burned_kcal <= 10000)),
    CONSTRAINT activity_v2_exercise_time_check 
        CHECK (exercise_time_minutes IS NULL OR (exercise_time_minutes >= 0 AND exercise_time_minutes <= 1440)),
    CONSTRAINT activity_v2_stand_time_check 
        CHECK (stand_time_minutes IS NULL OR (stand_time_minutes >= 0 AND stand_time_minutes <= 1440)),
    CONSTRAINT activity_v2_move_time_check 
        CHECK (move_time_minutes IS NULL OR (move_time_minutes >= 0 AND move_time_minutes <= 1440)),
    CONSTRAINT activity_v2_nike_fuel_check 
        CHECK (nike_fuel IS NULL OR (nike_fuel >= 0 AND nike_fuel <= 50000))
    
) PARTITION BY RANGE (recorded_at);

-- Create BRIN indexes for time-series optimization
CREATE INDEX IF NOT EXISTS idx_activity_v2_recorded_at_brin 
    ON activity_metrics_v2 USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_activity_v2_user_recorded_brin 
    ON activity_metrics_v2 USING BRIN (user_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_activity_v2_aggregation_period_brin 
    ON activity_metrics_v2 USING BRIN (aggregation_period, recorded_at);

-- Create B-tree index for frequently queried aggregation period
CREATE INDEX IF NOT EXISTS idx_activity_v2_user_aggregation 
    ON activity_metrics_v2 (user_id, aggregation_period);

-- Function to create monthly partitions specifically for activity_metrics_v2
CREATE OR REPLACE FUNCTION create_activity_v2_monthly_partitions(
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
        partition_name := 'activity_metrics_v2_' || to_char(start_date, 'YYYY_MM');
        
        -- Check if partition already exists
        IF NOT EXISTS (
            SELECT 1 FROM pg_class WHERE relname = partition_name
        ) THEN
            EXECUTE format('
                CREATE TABLE %I PARTITION OF activity_metrics_v2
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
            
            RAISE NOTICE 'Created partition and indexes: %', partition_name;
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Create initial partitions (3 months ahead as per requirements)
SELECT create_activity_v2_monthly_partitions();

-- Update the main partition maintenance function to include activity_metrics_v2
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
    
    -- Add activity_metrics_v2 maintenance
    PERFORM create_activity_v2_monthly_partitions();
END;
$$ LANGUAGE plpgsql;

-- Update partition index creation function to handle activity_metrics_v2
CREATE OR REPLACE FUNCTION create_partition_indexes(
    partition_name text,
    parent_table text
)
RETURNS void AS $$
BEGIN
    -- Create BRIN indexes on new partitions based on parent table type
    IF parent_table = 'raw_ingestions_partitioned' THEN
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (received_at)', 
                      partition_name || '_received_at_brin', partition_name);
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, received_at)', 
                      partition_name || '_user_received_brin', partition_name);
                      
    ELSIF parent_table = 'heart_rate_metrics_partitioned' THEN
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (recorded_at)', 
                      partition_name || '_recorded_at_brin', partition_name);
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, recorded_at)', 
                      partition_name || '_user_recorded_brin', partition_name);
                      
    ELSIF parent_table = 'activity_metrics_partitioned' THEN
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (recorded_at)', 
                      partition_name || '_recorded_at_brin', partition_name);
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, metric_type, recorded_at)', 
                      partition_name || '_user_type_recorded_brin', partition_name);
    
    ELSIF parent_table = 'activity_metrics_v2' THEN
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (recorded_at)', 
                      partition_name || '_recorded_at_brin', partition_name);
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (user_id, recorded_at)', 
                      partition_name || '_user_recorded_brin', partition_name);
        EXECUTE format('CREATE INDEX IF NOT EXISTS %I ON %I USING BRIN (aggregation_period, recorded_at)', 
                      partition_name || '_aggregation_recorded_brin', partition_name);
    END IF;
    
    RAISE NOTICE 'Created indexes for partition: %', partition_name;
END;
$$ LANGUAGE plpgsql;

-- Create a view for easy aggregation queries
CREATE VIEW activity_metrics_v2_daily_summary AS
SELECT 
    user_id,
    date_trunc('day', recorded_at) as activity_date,
    SUM(step_count) as total_steps,
    SUM(flights_climbed) as total_flights_climbed,
    SUM(distance_walking_running_meters) as total_walking_running_meters,
    SUM(distance_cycling_meters) as total_cycling_meters,
    SUM(distance_swimming_meters) as total_swimming_meters,
    SUM(active_energy_burned_kcal) as total_active_energy_kcal,
    SUM(basal_energy_burned_kcal) as total_basal_energy_kcal,
    SUM(exercise_time_minutes) as total_exercise_minutes,
    SUM(stand_time_minutes) as total_stand_minutes,
    COUNT(CASE WHEN stand_hour_achieved THEN 1 END) as stand_hours_achieved,
    COUNT(*) as total_records,
    array_agg(DISTINCT aggregation_period) as aggregation_periods_used
FROM activity_metrics_v2
WHERE aggregation_period IN ('hourly', 'daily')
GROUP BY user_id, date_trunc('day', recorded_at);

-- Grant permissions (assuming standard permission model)
-- Note: This assumes the standard permission grants are handled elsewhere
-- but we'll document what would be needed:

-- GRANT SELECT ON activity_metrics_v2 TO health_export_read_role;
-- GRANT INSERT, UPDATE, DELETE ON activity_metrics_v2 TO health_export_write_role;
-- GRANT SELECT ON activity_metrics_v2_daily_summary TO health_export_read_role;

-- Performance monitoring function for activity_metrics_v2
CREATE OR REPLACE FUNCTION analyze_activity_v2_performance()
RETURNS TABLE (
    table_name text,
    partition_count bigint,
    total_rows bigint,
    avg_rows_per_partition bigint,
    oldest_data timestamptz,
    newest_data timestamptz
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        'activity_metrics_v2'::text as table_name,
        COUNT(*)::bigint as partition_count,
        SUM(n_tup_ins)::bigint as total_rows,
        CASE WHEN COUNT(*) > 0 THEN (SUM(n_tup_ins) / COUNT(*))::bigint ELSE 0 END as avg_rows_per_partition,
        (SELECT MIN(recorded_at) FROM activity_metrics_v2) as oldest_data,
        (SELECT MAX(recorded_at) FROM activity_metrics_v2) as newest_data
    FROM pg_stat_user_tables 
    WHERE relname LIKE 'activity_metrics_v2_%';
END;
$$ LANGUAGE plpgsql;

-- Add comment documentation
COMMENT ON TABLE activity_metrics_v2 IS 'Apple Health activity metrics with standard field names and time-series optimization. Supports Apple Fitness ring goals, multiple distance types, and configurable aggregation periods.';

COMMENT ON COLUMN activity_metrics_v2.recorded_at IS 'Timestamp for the activity measurement with timezone support for proper granularity';
COMMENT ON COLUMN activity_metrics_v2.aggregation_period IS 'Granularity of the data point: minute, hourly, daily, or weekly aggregation';
COMMENT ON COLUMN activity_metrics_v2.active_energy_burned_kcal IS 'Active calories burned (Apple Health: HKQuantityTypeIdentifierActiveEnergyBurned)';
COMMENT ON COLUMN activity_metrics_v2.basal_energy_burned_kcal IS 'Basal/resting calories burned (Apple Health: HKQuantityTypeIdentifierBasalEnergyBurned)';
COMMENT ON COLUMN activity_metrics_v2.exercise_time_minutes IS 'Apple Fitness exercise ring time (Apple Health: HKQuantityTypeIdentifierAppleExerciseTime)';
COMMENT ON COLUMN activity_metrics_v2.stand_time_minutes IS 'Apple Fitness stand ring time (Apple Health: HKQuantityTypeIdentifierAppleStandTime)';
COMMENT ON COLUMN activity_metrics_v2.move_time_minutes IS 'Apple Fitness move ring time (Apple Health: HKQuantityTypeIdentifierAppleMoveTime)';
COMMENT ON COLUMN activity_metrics_v2.stand_hour_achieved IS 'Whether the stand goal was achieved for this hour (Apple Health: HKCategoryTypeIdentifierAppleStandHour)';