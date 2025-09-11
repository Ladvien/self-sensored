-- Migration Monitoring Script: activity_metrics to activity_metrics_v2
-- Story 5.1: Create Data Migration Scripts - Monitoring Component
--
-- This script provides comprehensive monitoring capabilities for the 
-- activity_metrics migration including progress tracking, performance
-- metrics, data consistency validation, and rollback procedures.
--
-- Features:
-- - Real-time progress tracking with ETA calculations  
-- - Performance monitoring (records/sec, batch times, memory usage)
-- - Data consistency validation between source and target tables
-- - Rollback procedures for safe migration reversal
-- - Production monitoring queries for 100M+ record migrations
--
-- Usage: Run individual queries to monitor different aspects of migration

-- =============================================================================
-- PROGRESS TRACKING QUERIES
-- =============================================================================

-- Current Migration Status Overview
CREATE OR REPLACE VIEW migration_status_overview AS
SELECT 
    migration_name,
    status,
    batch_number,
    total_records_processed,
    total_records_to_migrate,
    ROUND((total_records_processed::NUMERIC / NULLIF(total_records_to_migrate, 0) * 100), 2) as progress_percentage,
    start_time,
    last_updated,
    (last_updated - start_time) as elapsed_time,
    CASE 
        WHEN total_records_processed > 0 AND status = 'running' THEN
            ((total_records_to_migrate - total_records_processed) * 
             EXTRACT(EPOCH FROM (last_updated - start_time)) / total_records_processed)::INTERVAL
        ELSE NULL 
    END as estimated_time_remaining,
    performance_metrics,
    error_message
FROM migration_progress 
WHERE migration_name = 'activity_metrics_to_v2';

-- Real-time Progress Query (run repeatedly during migration)
CREATE OR REPLACE FUNCTION get_migration_progress()
RETURNS TABLE (
    migration_status VARCHAR(20),
    progress_pct NUMERIC,
    records_processed BIGINT,
    records_remaining BIGINT,
    current_batch BIGINT,
    elapsed_time INTERVAL,
    estimated_remaining INTERVAL,
    records_per_second NUMERIC,
    avg_batch_time_sec NUMERIC,
    memory_usage TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        mp.status::VARCHAR(20) as migration_status,
        ROUND((mp.total_records_processed::NUMERIC / NULLIF(mp.total_records_to_migrate, 0) * 100), 2) as progress_pct,
        mp.total_records_processed as records_processed,
        (mp.total_records_to_migrate - mp.total_records_processed) as records_remaining,
        mp.batch_number as current_batch,
        (mp.last_updated - mp.start_time) as elapsed_time,
        CASE 
            WHEN mp.total_records_processed > 0 AND mp.status = 'running' THEN
                ((mp.total_records_to_migrate - mp.total_records_processed)::NUMERIC * 
                 EXTRACT(EPOCH FROM (mp.last_updated - mp.start_time)) / mp.total_records_processed)::INTERVAL
            ELSE NULL
        END as estimated_remaining,
        COALESCE((mp.performance_metrics->>'records_per_second')::NUMERIC, 0) as records_per_second,
        COALESCE((mp.performance_metrics->>'avg_batch_time_seconds')::NUMERIC, 0) as avg_batch_time_sec,
        COALESCE(mp.performance_metrics->>'memory_usage_mb', 'N/A') as memory_usage
    FROM migration_progress mp
    WHERE mp.migration_name = 'activity_metrics_to_v2';
END;
$$ LANGUAGE plpgsql;

-- Detailed Performance Metrics Query
CREATE OR REPLACE FUNCTION get_migration_performance_details()
RETURNS TABLE (
    metric_name VARCHAR(50),
    current_value TEXT,
    description TEXT
) AS $$
DECLARE
    perf_metrics JSONB;
BEGIN
    SELECT performance_metrics INTO perf_metrics 
    FROM migration_progress 
    WHERE migration_name = 'activity_metrics_to_v2';
    
    IF perf_metrics IS NULL THEN
        RETURN QUERY SELECT 'No Data'::VARCHAR(50), 'Migration not started'::TEXT, 'Run migration first'::TEXT;
        RETURN;
    END IF;
    
    RETURN QUERY VALUES
        ('Records/Second', COALESCE(perf_metrics->>'records_per_second', 'N/A'), 'Current processing speed'),
        ('Avg Batch Time', COALESCE(perf_metrics->>'avg_batch_time_seconds', 'N/A') || ' sec', 'Average time per batch'),
        ('Estimated Completion', COALESCE(perf_metrics->>'estimated_completion_time', 'N/A'), 'Projected completion timestamp'),
        ('Memory Usage', COALESCE(perf_metrics->>'memory_usage_mb', 'N/A'), 'Current table memory footprint'),
        ('Est. Remaining Time', COALESCE(perf_metrics->>'estimated_remaining_seconds', 'N/A') || ' sec', 'Time until completion');
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- DATA CONSISTENCY VALIDATION QUERIES  
-- =============================================================================

-- Quick Data Consistency Check
CREATE OR REPLACE FUNCTION quick_consistency_check()
RETURNS TABLE (
    check_name VARCHAR(100),
    source_value BIGINT,
    target_value BIGINT,
    match_status VARCHAR(10),
    difference BIGINT
) AS $$
BEGIN
    -- Record count comparison
    RETURN QUERY
    WITH counts AS (
        SELECT 
            (SELECT COUNT(*) FROM activity_metrics) as src_count,
            (SELECT COUNT(*) FROM activity_metrics_v2) as tgt_count
    )
    SELECT 
        'Total Records'::VARCHAR(100) as check_name,
        src_count as source_value,
        tgt_count as target_value,
        CASE WHEN src_count = tgt_count THEN 'MATCH' ELSE 'MISMATCH' END::VARCHAR(10) as match_status,
        (tgt_count - src_count) as difference
    FROM counts;
    
    -- User count comparison
    RETURN QUERY
    WITH user_counts AS (
        SELECT 
            (SELECT COUNT(DISTINCT user_id) FROM activity_metrics) as src_users,
            (SELECT COUNT(DISTINCT user_id) FROM activity_metrics_v2) as tgt_users
    )
    SELECT 
        'Unique Users'::VARCHAR(100) as check_name,
        src_users as source_value,
        tgt_users as target_value,
        CASE WHEN src_users = tgt_users THEN 'MATCH' ELSE 'MISMATCH' END::VARCHAR(10) as match_status,
        (tgt_users - src_users) as difference
    FROM user_counts;
    
    -- Steps total comparison
    RETURN QUERY
    WITH steps_totals AS (
        SELECT 
            (SELECT COALESCE(SUM(steps), 0) FROM activity_metrics WHERE steps IS NOT NULL) as src_steps,
            (SELECT COALESCE(SUM(step_count), 0) FROM activity_metrics_v2 WHERE step_count IS NOT NULL) as tgt_steps
    )
    SELECT 
        'Total Steps'::VARCHAR(100) as check_name,
        src_steps as source_value,
        tgt_steps as target_value,
        CASE WHEN src_steps = tgt_steps THEN 'MATCH' ELSE 'MISMATCH' END::VARCHAR(10) as match_status,
        (tgt_steps - src_steps) as difference
    FROM steps_totals;
END;
$$ LANGUAGE plpgsql;

-- Detailed Data Validation (Sample-based for large datasets)
CREATE OR REPLACE FUNCTION detailed_data_validation(sample_size INTEGER DEFAULT 1000)
RETURNS TABLE (
    validation_type VARCHAR(100),
    sampled_records INTEGER,
    matching_records INTEGER,
    mismatch_records INTEGER,
    accuracy_percentage NUMERIC,
    sample_mismatches JSONB
) AS $$
DECLARE
    sample_data RECORD;
    total_sampled INTEGER := 0;
    matching_count INTEGER := 0;
    mismatch_examples JSONB := '[]';
    mismatch_count INTEGER := 0;
BEGIN
    -- Sample validation for field mapping accuracy
    FOR sample_data IN (
        SELECT 
            am.id as orig_id,
            am.user_id,
            am.recorded_date,
            am.steps as orig_steps,
            am.flights_climbed as orig_flights,
            am.distance_meters as orig_distance,
            am.calories_burned as orig_calories,
            am.active_minutes as orig_active_mins,
            am.source_device as orig_source,
            amv2.step_count as v2_steps,
            amv2.flights_climbed as v2_flights,
            amv2.distance_walking_running_meters as v2_distance,
            amv2.active_energy_burned_kcal as v2_calories,
            amv2.exercise_time_minutes as v2_active_mins,
            amv2.source as v2_source
        FROM activity_metrics am
        LEFT JOIN activity_metrics_v2 amv2 ON (
            am.user_id = amv2.user_id AND 
            am.recorded_date = amv2.recorded_at::DATE
        )
        ORDER BY RANDOM()
        LIMIT sample_size
    ) LOOP
        total_sampled := total_sampled + 1;
        
        -- Check for exact field mapping matches
        IF (COALESCE(sample_data.orig_steps, 0) = COALESCE(sample_data.v2_steps, 0) AND
            COALESCE(sample_data.orig_flights, 0) = COALESCE(sample_data.v2_flights, 0) AND
            COALESCE(sample_data.orig_distance, 0) = COALESCE(sample_data.v2_distance, 0) AND
            COALESCE(sample_data.orig_calories, 0) = COALESCE(sample_data.v2_calories, 0) AND
            COALESCE(sample_data.orig_active_mins, 0) = COALESCE(sample_data.v2_active_mins, 0) AND
            COALESCE(sample_data.orig_source, '') = COALESCE(sample_data.v2_source, '')) THEN
            matching_count := matching_count + 1;
        ELSE
            mismatch_count := mismatch_count + 1;
            -- Collect sample mismatches (limit to first 5)
            IF jsonb_array_length(mismatch_examples) < 5 THEN
                mismatch_examples := mismatch_examples || jsonb_build_object(
                    'user_id', sample_data.user_id,
                    'recorded_date', sample_data.recorded_date,
                    'mismatches', jsonb_build_object(
                        'steps', jsonb_build_object('original', sample_data.orig_steps, 'migrated', sample_data.v2_steps),
                        'flights', jsonb_build_object('original', sample_data.orig_flights, 'migrated', sample_data.v2_flights),
                        'distance', jsonb_build_object('original', sample_data.orig_distance, 'migrated', sample_data.v2_distance),
                        'calories', jsonb_build_object('original', sample_data.orig_calories, 'migrated', sample_data.v2_calories),
                        'active_minutes', jsonb_build_object('original', sample_data.orig_active_mins, 'migrated', sample_data.v2_active_mins)
                    )
                );
            END IF;
        END IF;
    END LOOP;
    
    RETURN QUERY SELECT
        'Field Mapping Accuracy'::VARCHAR(100) as validation_type,
        total_sampled as sampled_records,
        matching_count as matching_records,
        mismatch_count as mismatch_records,
        ROUND((matching_count::NUMERIC / NULLIF(total_sampled, 0) * 100), 2) as accuracy_percentage,
        mismatch_examples as sample_mismatches;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- PERFORMANCE MONITORING QUERIES
-- =============================================================================

-- Database Performance Impact Assessment
CREATE OR REPLACE FUNCTION assess_migration_performance_impact()
RETURNS TABLE (
    metric_name VARCHAR(100),
    current_value TEXT,
    impact_level VARCHAR(20),
    recommendation TEXT
) AS $$
DECLARE
    v2_size BIGINT;
    original_size BIGINT;
    active_connections INTEGER;
    cpu_usage NUMERIC;
BEGIN
    -- Get table sizes
    SELECT pg_total_relation_size('activity_metrics_v2') INTO v2_size;
    SELECT pg_total_relation_size('activity_metrics') INTO original_size;
    
    -- Get active connections
    SELECT COUNT(*) INTO active_connections 
    FROM pg_stat_activity 
    WHERE state = 'active' AND query NOT LIKE '%pg_stat_activity%';
    
    -- Table size impact
    RETURN QUERY SELECT
        'Target Table Size'::VARCHAR(100) as metric_name,
        pg_size_pretty(v2_size) as current_value,
        CASE 
            WHEN v2_size > original_size * 2 THEN 'HIGH'
            WHEN v2_size > original_size * 1.5 THEN 'MEDIUM'
            ELSE 'LOW'
        END::VARCHAR(20) as impact_level,
        'Monitor disk space during migration'::TEXT as recommendation;
        
    -- Connection usage
    RETURN QUERY SELECT
        'Active Connections'::VARCHAR(100) as metric_name,
        active_connections::TEXT as current_value,
        CASE 
            WHEN active_connections > 80 THEN 'HIGH'
            WHEN active_connections > 50 THEN 'MEDIUM'
            ELSE 'LOW'
        END::VARCHAR(20) as impact_level,
        'Consider reducing concurrent operations if high'::TEXT as recommendation;
        
    -- Migration progress rate
    RETURN QUERY 
    WITH progress_rate AS (
        SELECT 
            COALESCE((performance_metrics->>'records_per_second')::NUMERIC, 0) as rps
        FROM migration_progress 
        WHERE migration_name = 'activity_metrics_to_v2'
    )
    SELECT
        'Processing Rate'::VARCHAR(100) as metric_name,
        (rps || ' records/sec') as current_value,
        CASE 
            WHEN rps < 100 THEN 'HIGH'
            WHEN rps < 500 THEN 'MEDIUM'
            ELSE 'LOW'
        END::VARCHAR(20) as impact_level,
        'Slow processing may indicate resource constraints'::TEXT as recommendation
    FROM progress_rate;
END;
$$ LANGUAGE plpgsql;

-- Real-time Resource Usage Monitoring
CREATE OR REPLACE VIEW migration_resource_usage AS
SELECT 
    NOW() as check_time,
    (SELECT COUNT(*) FROM pg_stat_activity WHERE state = 'active') as active_connections,
    (SELECT COUNT(*) FROM pg_locks WHERE granted = false) as blocked_queries,
    pg_size_pretty(pg_database_size(current_database())) as database_size,
    pg_size_pretty(pg_total_relation_size('activity_metrics')) as source_table_size,
    pg_size_pretty(pg_total_relation_size('activity_metrics_v2')) as target_table_size,
    (SELECT schemaname||'.'||tablename as table_name, 
            n_tup_ins as inserts_today,
            n_tup_upd as updates_today,
            n_tup_del as deletes_today
     FROM pg_stat_user_tables 
     WHERE tablename IN ('activity_metrics', 'activity_metrics_v2')
    ) as table_activity;

-- =============================================================================
-- ROLLBACK PROCEDURES
-- =============================================================================

-- Safe Rollback with Confirmation
CREATE OR REPLACE FUNCTION safe_migration_rollback(
    confirm_rollback BOOLEAN DEFAULT FALSE
)
RETURNS TABLE (
    rollback_step VARCHAR(100),
    records_affected BIGINT,
    status VARCHAR(20),
    details TEXT
) AS $$
DECLARE
    v_records_deleted BIGINT := 0;
    v_migration_status VARCHAR(20);
BEGIN
    -- Safety check
    IF NOT confirm_rollback THEN
        RETURN QUERY SELECT
            'SAFETY CHECK FAILED'::VARCHAR(100) as rollback_step,
            0::BIGINT as records_affected,
            'CANCELLED'::VARCHAR(20) as status,
            'Must set confirm_rollback = TRUE to proceed with rollback'::TEXT as details;
        RETURN;
    END IF;
    
    -- Check migration status
    SELECT status INTO v_migration_status 
    FROM migration_progress 
    WHERE migration_name = 'activity_metrics_to_v2';
    
    RETURN QUERY SELECT
        'Pre-Rollback Status Check'::VARCHAR(100) as rollback_step,
        1::BIGINT as records_affected,
        'INFO'::VARCHAR(20) as status,
        ('Current migration status: ' || COALESCE(v_migration_status, 'NOT_FOUND'))::TEXT as details;
    
    -- Count records before deletion
    SELECT COUNT(*) INTO v_records_deleted FROM activity_metrics_v2;
    
    RETURN QUERY SELECT
        'Record Count Pre-Deletion'::VARCHAR(100) as rollback_step,
        v_records_deleted as records_affected,
        'INFO'::VARCHAR(20) as status,
        'Records in activity_metrics_v2 before rollback'::TEXT as details;
    
    -- Delete migrated data
    DELETE FROM activity_metrics_v2;
    
    RETURN QUERY SELECT
        'Delete Migrated Records'::VARCHAR(100) as rollback_step,
        v_records_deleted as records_affected,
        'COMPLETED'::VARCHAR(20) as status,
        'All migrated records deleted from activity_metrics_v2'::TEXT as details;
        
    -- Reset migration progress
    DELETE FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2';
    
    RETURN QUERY SELECT
        'Reset Migration Progress'::VARCHAR(100) as rollback_step,
        1::BIGINT as records_affected,
        'COMPLETED'::VARCHAR(20) as status,
        'Migration progress tracking reset'::TEXT as details;
    
    -- Vacuum target table to reclaim space
    VACUUM ANALYZE activity_metrics_v2;
    
    RETURN QUERY SELECT
        'Cleanup Target Table'::VARCHAR(100) as rollback_step,
        0::BIGINT as records_affected,
        'COMPLETED'::VARCHAR(20) as status,
        'Target table vacuumed and analyzed'::TEXT as details;
        
    RAISE NOTICE 'Migration rollback completed successfully. Deleted % records.', v_records_deleted;
END;
$$ LANGUAGE plpgsql;

-- Emergency Rollback (Simplified, faster execution)
CREATE OR REPLACE FUNCTION emergency_migration_rollback()
RETURNS TABLE (
    action VARCHAR(50),
    result VARCHAR(20)
) AS $$
DECLARE
    v_deleted BIGINT;
BEGIN
    -- Immediate cleanup
    TRUNCATE activity_metrics_v2;
    DELETE FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2';
    
    RETURN QUERY VALUES
        ('TRUNCATE_TARGET', 'SUCCESS'),
        ('RESET_PROGRESS', 'SUCCESS');
        
    RAISE NOTICE 'Emergency rollback completed';
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- COMPREHENSIVE MONITORING DASHBOARD QUERY
-- =============================================================================

-- Complete Migration Dashboard (Single Query)
CREATE OR REPLACE FUNCTION migration_dashboard()
RETURNS TABLE (
    section VARCHAR(50),
    metric VARCHAR(100),
    value TEXT,
    status VARCHAR(20)
) AS $$
BEGIN
    -- Migration Status Section
    RETURN QUERY
    SELECT 
        'STATUS'::VARCHAR(50) as section,
        'Migration Status'::VARCHAR(100) as metric,
        COALESCE(mp.status, 'NOT_STARTED') as value,
        CASE 
            WHEN mp.status = 'completed' THEN 'SUCCESS'
            WHEN mp.status = 'failed' THEN 'ERROR'
            WHEN mp.status = 'running' THEN 'IN_PROGRESS'
            ELSE 'UNKNOWN'
        END::VARCHAR(20) as status
    FROM migration_progress mp 
    WHERE mp.migration_name = 'activity_metrics_to_v2'
    UNION ALL
    SELECT 'STATUS', 'Progress Percentage', 
           COALESCE(ROUND((mp.total_records_processed::NUMERIC / NULLIF(mp.total_records_to_migrate, 0) * 100), 2)::TEXT || '%', '0%'),
           'INFO'
    FROM migration_progress mp WHERE mp.migration_name = 'activity_metrics_to_v2'
    UNION ALL
    SELECT 'STATUS', 'Records Processed', 
           COALESCE(mp.total_records_processed::TEXT, '0'),
           'INFO'
    FROM migration_progress mp WHERE mp.migration_name = 'activity_metrics_to_v2'
    UNION ALL
    -- Performance Section  
    SELECT 'PERFORMANCE', 'Records/Second',
           COALESCE((mp.performance_metrics->>'records_per_second'), 'N/A'),
           CASE WHEN (mp.performance_metrics->>'records_per_second')::NUMERIC < 100 THEN 'WARNING' ELSE 'OK' END
    FROM migration_progress mp WHERE mp.migration_name = 'activity_metrics_to_v2'
    UNION ALL
    SELECT 'PERFORMANCE', 'Avg Batch Time',
           COALESCE((mp.performance_metrics->>'avg_batch_time_seconds') || ' sec', 'N/A'),
           'INFO'
    FROM migration_progress mp WHERE mp.migration_name = 'activity_metrics_to_v2'
    UNION ALL
    -- Data Consistency Section
    SELECT 'CONSISTENCY', 'Record Count Match',
           CASE WHEN (SELECT COUNT(*) FROM activity_metrics) = (SELECT COUNT(*) FROM activity_metrics_v2) 
                THEN 'MATCH' ELSE 'MISMATCH' END,
           CASE WHEN (SELECT COUNT(*) FROM activity_metrics) = (SELECT COUNT(*) FROM activity_metrics_v2) 
                THEN 'SUCCESS' ELSE 'ERROR' END
    UNION ALL
    SELECT 'CONSISTENCY', 'User Count Match',
           CASE WHEN (SELECT COUNT(DISTINCT user_id) FROM activity_metrics) = (SELECT COUNT(DISTINCT user_id) FROM activity_metrics_v2)
                THEN 'MATCH' ELSE 'MISMATCH' END,
           CASE WHEN (SELECT COUNT(DISTINCT user_id) FROM activity_metrics) = (SELECT COUNT(DISTINCT user_id) FROM activity_metrics_v2)
                THEN 'SUCCESS' ELSE 'ERROR' END;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- USAGE EXAMPLES AND DOCUMENTATION
-- =============================================================================

/*
USAGE EXAMPLES:

1. Monitor Migration Progress:
   SELECT * FROM get_migration_progress();

2. View Migration Status Overview:
   SELECT * FROM migration_status_overview;

3. Check Data Consistency:
   SELECT * FROM quick_consistency_check();

4. Detailed Validation (Sample-based):
   SELECT * FROM detailed_data_validation(5000);

5. Performance Impact Assessment:
   SELECT * FROM assess_migration_performance_impact();

6. Complete Dashboard:
   SELECT * FROM migration_dashboard();

7. Safe Rollback (with confirmation):
   SELECT * FROM safe_migration_rollback(TRUE);

8. Emergency Rollback:
   SELECT * FROM emergency_migration_rollback();

9. Monitor Resource Usage:
   SELECT * FROM migration_resource_usage;

10. Performance Details:
    SELECT * FROM get_migration_performance_details();

MONITORING SCHEDULE RECOMMENDATIONS:
- During migration: Run get_migration_progress() every 30 seconds
- For large datasets: Monitor migration_resource_usage every 5 minutes
- Post-migration: Run quick_consistency_check() and detailed_data_validation()
- Before production: Run complete migration_dashboard() for final verification
*/

-- Add helpful documentation
COMMENT ON VIEW migration_status_overview IS 'Real-time overview of migration progress with ETA calculations';
COMMENT ON FUNCTION get_migration_progress() IS 'Get current migration progress with performance metrics';
COMMENT ON FUNCTION quick_consistency_check() IS 'Fast data consistency validation between source and target tables';
COMMENT ON FUNCTION detailed_data_validation(INTEGER) IS 'Sample-based detailed validation for field mapping accuracy';
COMMENT ON FUNCTION safe_migration_rollback(BOOLEAN) IS 'Safe rollback with confirmation requirement and detailed logging';
COMMENT ON FUNCTION migration_dashboard() IS 'Comprehensive migration monitoring dashboard in single query';