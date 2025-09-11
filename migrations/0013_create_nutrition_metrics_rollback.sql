-- Rollback Migration for 0013_create_nutrition_metrics.sql
-- Safely removes the nutrition_metrics table and all related objects
-- Run this migration to rollback the nutrition metrics implementation

-- === ROLLBACK SEQUENCE ===

-- Step 1: Drop the daily summary view (depends on the table)
DROP VIEW IF EXISTS nutrition_metrics_daily_summary;

-- Step 2: Drop the performance monitoring function
DROP FUNCTION IF EXISTS analyze_nutrition_performance();

-- Step 3: Drop the partition creation function
DROP FUNCTION IF EXISTS create_nutrition_monthly_partitions(integer, integer);

-- Step 4: Update the main partition maintenance function to remove nutrition_metrics
CREATE OR REPLACE FUNCTION maintain_partitions()
RETURNS void AS $$
BEGIN
    -- Maintain partitions for original tables only (removing nutrition_metrics)
    PERFORM create_monthly_partitions('raw_ingestions_partitioned', 'received_at');
    PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
    PERFORM create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('blood_pressure_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('activity_metrics_partitioned', 'recorded_at');
    PERFORM create_monthly_partitions('sleep_metrics_partitioned', 'date');
    
    -- Keep activity_metrics_v2 if it exists (from migration 0012)
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'create_activity_v2_monthly_partitions') THEN
        PERFORM create_activity_v2_monthly_partitions();
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Step 5: Drop all partition tables (must be done before dropping parent table)
-- This will find and drop all nutrition_metrics partition tables
DO $$
DECLARE 
    partition_name text;
BEGIN
    FOR partition_name IN 
        SELECT relname FROM pg_class 
        WHERE relname LIKE 'nutrition_metrics_%'
        AND relkind = 'r'
    LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I CASCADE', partition_name);
        RAISE NOTICE 'Dropped partition table: %', partition_name;
    END LOOP;
END $$;

-- Step 6: Drop main indexes (if they still exist)
DROP INDEX IF EXISTS idx_nutrition_recorded_at_brin;
DROP INDEX IF EXISTS idx_nutrition_user_recorded_brin;
DROP INDEX IF EXISTS idx_nutrition_aggregation_period_brin;
DROP INDEX IF EXISTS idx_nutrition_user_aggregation;

-- Step 7: Drop the main nutrition_metrics table
-- This will cascade to remove any remaining dependent objects
DROP TABLE IF EXISTS nutrition_metrics CASCADE;

-- === VERIFICATION ===

-- Verify the table and all related objects have been removed
DO $$
DECLARE
    table_exists boolean;
    partition_count integer;
    function_count integer;
    view_count integer;
BEGIN
    -- Check if table exists
    SELECT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'nutrition_metrics'
    ) INTO table_exists;
    
    -- Count remaining partitions
    SELECT COUNT(*) INTO partition_count
    FROM pg_class WHERE relname LIKE 'nutrition_metrics_%';
    
    -- Count related functions
    SELECT COUNT(*) INTO function_count
    FROM pg_proc WHERE proname IN (
        'create_nutrition_monthly_partitions', 
        'analyze_nutrition_performance'
    );
    
    -- Count related views
    SELECT COUNT(*) INTO view_count
    FROM information_schema.views 
    WHERE table_name = 'nutrition_metrics_daily_summary';
    
    -- Report results
    IF table_exists THEN
        RAISE EXCEPTION 'ERROR: nutrition_metrics table still exists after rollback';
    END IF;
    
    IF partition_count > 0 THEN
        RAISE EXCEPTION 'ERROR: % nutrition_metrics partition(s) still exist after rollback', partition_count;
    END IF;
    
    IF function_count > 0 THEN
        RAISE EXCEPTION 'ERROR: % nutrition_metrics function(s) still exist after rollback', function_count;
    END IF;
    
    IF view_count > 0 THEN
        RAISE EXCEPTION 'ERROR: % nutrition_metrics view(s) still exist after rollback', view_count;
    END IF;
    
    RAISE NOTICE '✅ ROLLBACK SUCCESSFUL: All nutrition_metrics objects have been removed';
    RAISE NOTICE '   - Main table: REMOVED';
    RAISE NOTICE '   - Partitions: REMOVED (% found)', partition_count;
    RAISE NOTICE '   - Functions: REMOVED (% found)', function_count;
    RAISE NOTICE '   - Views: REMOVED (% found)', view_count;
    RAISE NOTICE '   - Indexes: REMOVED';
    RAISE NOTICE '   - Constraints: REMOVED';
END $$;

-- === CLEANUP LOG ===

-- Log the rollback operation in audit_log if it exists
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_log') THEN
        -- This assumes the audit_log table structure exists from earlier migrations
        INSERT INTO audit_log (
            user_id, 
            action, 
            resource_type, 
            resource_id, 
            metadata, 
            created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000000'::uuid, -- System user
            'rollback_migration',
            'table',
            'nutrition_metrics',
            '{"migration": "0013_create_nutrition_metrics", "operation": "rollback", "timestamp": "' || NOW()::text || '"}'::jsonb,
            NOW()
        );
        RAISE NOTICE '📝 Rollback logged in audit_log table';
    ELSE
        RAISE NOTICE '⚠️  audit_log table not found - rollback not logged';
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE '⚠️  Could not log rollback to audit_log: %', SQLERRM;
END $$;

-- === POST-ROLLBACK INSTRUCTIONS ===

-- Display instructions for manual verification
DO $$
BEGIN
    RAISE NOTICE '';
    RAISE NOTICE '=== POST-ROLLBACK VERIFICATION STEPS ===';
    RAISE NOTICE '1. Verify no nutrition_metrics references in application code';
    RAISE NOTICE '2. Check that maintain_partitions() function works correctly';
    RAISE NOTICE '3. Confirm no orphaned data in related tables';
    RAISE NOTICE '4. Review application logs for any nutrition_metrics errors';
    RAISE NOTICE '5. Update API documentation to remove nutrition endpoints';
    RAISE NOTICE '';
    RAISE NOTICE 'If you need to re-apply the nutrition metrics feature:';
    RAISE NOTICE '- Run migration 0013_create_nutrition_metrics.sql again';
    RAISE NOTICE '- Update application configuration for nutrition fields';
    RAISE NOTICE '- Re-run integration tests';
    RAISE NOTICE '';
END $$;