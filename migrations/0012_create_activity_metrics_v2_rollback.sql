-- Rollback Migration for activity_metrics_v2 Table
-- This script safely removes all objects created by 0012_create_activity_metrics_v2.sql
-- Run this script to revert the migration if needed

-- Drop the daily summary view first
DROP VIEW IF EXISTS activity_metrics_v2_daily_summary;

-- Drop activity_metrics_v2 specific functions
DROP FUNCTION IF EXISTS analyze_activity_v2_performance();
DROP FUNCTION IF EXISTS create_activity_v2_monthly_partitions(integer, integer);

-- Restore the original maintain_partitions function (without activity_metrics_v2)
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for all partitioned tables (excluding activity_metrics_v2)
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
END;
$$ LANGUAGE plpgsql;

-- Restore the original create_partition_indexes function (without activity_metrics_v2)
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

-- Drop all activity_metrics_v2 partitions
DO $$
DECLARE
    partition_record record;
BEGIN
    FOR partition_record IN 
        SELECT tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'activity_metrics_v2_%'
        AND schemaname = 'public'
    LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I CASCADE', partition_record.tablename);
        RAISE NOTICE 'Dropped partition: %', partition_record.tablename;
    END LOOP;
END $$;

-- Drop the main partitioned table (this will cascade drop any remaining partitions and indexes)
DROP TABLE IF EXISTS activity_metrics_v2 CASCADE;

-- Verify cleanup
DO $$
DECLARE
    remaining_objects integer;
BEGIN
    -- Check for any remaining activity_metrics_v2 related objects
    SELECT COUNT(*) INTO remaining_objects
    FROM (
        SELECT table_name FROM information_schema.tables WHERE table_name LIKE '%activity_metrics_v2%'
        UNION ALL
        SELECT table_name FROM information_schema.views WHERE table_name LIKE '%activity_metrics_v2%'
        UNION ALL
        SELECT routine_name FROM information_schema.routines WHERE routine_name LIKE '%activity_v2%'
    ) remaining;
    
    IF remaining_objects > 0 THEN
        RAISE WARNING 'Warning: % activity_metrics_v2 related objects may still exist', remaining_objects;
    ELSE
        RAISE NOTICE 'Rollback completed successfully. All activity_metrics_v2 objects removed.';
    END IF;
END $$;