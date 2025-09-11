-- Rollback Migration for Symptoms Table
-- Removes symptoms table, views, functions, and associated indexes

-- === DROP VIEWS ===
DROP VIEW IF EXISTS symptoms_daily_summary CASCADE;
DROP VIEW IF EXISTS symptoms_severity_summary CASCADE;

-- === DROP PERFORMANCE FUNCTIONS ===
DROP FUNCTION IF EXISTS analyze_symptoms_performance() CASCADE;

-- === UPDATE MAINTENANCE FUNCTION ===
-- Restore partition maintenance function to previous state
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for all partitioned tables (excluding symptoms)
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
    
    -- Maintain new tables (excluding symptoms)
    PERFORM create_activity_v2_monthly_partitions();
    PERFORM create_nutrition_monthly_partitions();
END;
$$ LANGUAGE plpgsql;

-- === DROP PARTITION MANAGEMENT FUNCTION ===
DROP FUNCTION IF EXISTS create_symptoms_monthly_partitions(integer, integer) CASCADE;

-- === DROP ALL SYMPTOM PARTITIONS ===
-- Get list of symptom partitions and drop them
DO $$
DECLARE
    partition_record RECORD;
BEGIN
    FOR partition_record IN 
        SELECT schemaname, tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'symptoms_%'
    LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I.%I CASCADE', 
                      partition_record.schemaname, 
                      partition_record.tablename);
        RAISE NOTICE 'Dropped partition: %', partition_record.tablename;
    END LOOP;
END $$;

-- === DROP MAIN TABLE ===
DROP TABLE IF EXISTS symptoms CASCADE;

-- === LOG ROLLBACK ===
DO $$
BEGIN
    RAISE NOTICE 'Successfully rolled back symptoms table migration';
    RAISE NOTICE 'Removed: symptoms table, partitions, indexes, views, and functions';
    RAISE NOTICE 'Restored: maintain_partitions() function to previous state';
END $$;