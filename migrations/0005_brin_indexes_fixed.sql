-- BRIN Indexes Migration (Fixed for Partitioned Tables)
-- Creates BRIN indexes without CONCURRENTLY for partitioned tables

-- BRIN indexes for regular tables
-- Note: CONCURRENTLY removed for test database compatibility
CREATE INDEX IF NOT EXISTS idx_workouts_time_brin 
    ON workouts USING BRIN (started_at);

CREATE INDEX IF NOT EXISTS idx_workouts_user_time_brin 
    ON workouts USING BRIN (user_id, started_at);

CREATE INDEX IF NOT EXISTS idx_audit_log_time_brin 
    ON audit_log USING BRIN (created_at);

-- BRIN indexes for partitioned tables (cannot use CONCURRENTLY)
CREATE INDEX IF NOT EXISTS idx_raw_ingestions_partitioned_time_brin 
    ON raw_ingestions_partitioned USING BRIN (received_at);

CREATE INDEX IF NOT EXISTS idx_raw_ingestions_partitioned_user_time_brin 
    ON raw_ingestions_partitioned USING BRIN (user_id, received_at);

CREATE INDEX IF NOT EXISTS idx_audit_log_partitioned_time_brin 
    ON audit_log_partitioned USING BRIN (created_at);

CREATE INDEX IF NOT EXISTS idx_heart_rate_partitioned_time_brin 
    ON heart_rate_metrics_partitioned USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_heart_rate_partitioned_user_time_brin 
    ON heart_rate_metrics_partitioned USING BRIN (user_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_blood_pressure_partitioned_time_brin 
    ON blood_pressure_metrics_partitioned USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_sleep_partitioned_time_brin 
    ON sleep_metrics_partitioned USING BRIN (date);

CREATE INDEX IF NOT EXISTS idx_activity_partitioned_time_brin 
    ON activity_metrics_partitioned USING BRIN (recorded_at);

CREATE INDEX IF NOT EXISTS idx_activity_partitioned_user_type_time_brin 
    ON activity_metrics_partitioned USING BRIN (user_id, metric_type, recorded_at);

-- Additional B-tree indexes for frequently looked up values (keep these as B-tree)
CREATE INDEX IF NOT EXISTS idx_workouts_type_user 
    ON workouts (workout_type, user_id);

-- Function to update BRIN indexes (should be called periodically)
CREATE OR REPLACE FUNCTION refresh_brin_indexes()
RETURNS void AS $$
DECLARE
    index_record record;
BEGIN
    -- Refresh all BRIN indexes
    FOR index_record IN 
        SELECT indexname, tablename 
        FROM pg_indexes 
        WHERE indexdef LIKE '%USING brin%'
        AND schemaname = 'public'
    LOOP
        EXECUTE format('REINDEX INDEX %I', index_record.indexname);
        RAISE NOTICE 'Refreshed BRIN index: %', index_record.indexname;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Create index maintenance function for partitioned tables
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
    END IF;
    
    RAISE NOTICE 'Created indexes for partition: %', partition_name;
END;
$$ LANGUAGE plpgsql;