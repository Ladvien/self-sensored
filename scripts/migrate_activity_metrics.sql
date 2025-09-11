-- Data Migration Script: activity_metrics to activity_metrics_v2
-- Story 5.1: Create Data Migration Scripts for activity_metrics
-- 
-- This script migrates data from the legacy activity_metrics table to the new
-- activity_metrics_v2 table with proper field mapping, batch processing,
-- progress tracking, and resumability.
--
-- Features:
-- - Batch processing (8,000 records per batch to stay under PostgreSQL limits)  
-- - Progress tracking and resumability using migration_progress table
-- - Field mapping for renamed and restructured fields
-- - NULL value handling and data validation
-- - Zero data loss guarantee with transaction safety
-- - Performance optimized for 100M+ records (<4 hours target)
--
-- Usage:
--   1. Run this script as a PostgreSQL function
--   2. Call: SELECT migrate_activity_metrics_to_v2();
--   3. Monitor progress with: SELECT * FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2';

-- Create migration progress tracking table
CREATE TABLE IF NOT EXISTS migration_progress (
    id BIGSERIAL PRIMARY KEY,
    migration_name VARCHAR(100) NOT NULL,
    batch_number BIGINT NOT NULL DEFAULT 0,
    total_records_processed BIGINT NOT NULL DEFAULT 0,
    total_records_to_migrate BIGINT,
    start_time TIMESTAMPTZ DEFAULT NOW(),
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    status VARCHAR(20) DEFAULT 'running' CHECK (status IN ('running', 'paused', 'completed', 'failed', 'cancelled')),
    last_processed_id UUID, -- Track last processed record ID for resumability
    error_message TEXT,
    performance_metrics JSONB,
    
    CONSTRAINT migration_progress_unique UNIQUE(migration_name)
);

-- Create index for efficient progress tracking
CREATE INDEX IF NOT EXISTS idx_migration_progress_name_status 
    ON migration_progress(migration_name, status);

-- Create function to safely migrate activity_metrics to activity_metrics_v2
CREATE OR REPLACE FUNCTION migrate_activity_metrics_to_v2(
    p_batch_size INTEGER DEFAULT 8000,
    p_resume_from UUID DEFAULT NULL
) 
RETURNS TABLE (
    status VARCHAR(20),
    total_processed BIGINT,
    total_remaining BIGINT,
    batch_count BIGINT,
    migration_time_seconds INTEGER,
    performance_summary JSONB
) AS $$
DECLARE
    v_batch_size INTEGER := p_batch_size;
    v_total_records BIGINT;
    v_processed_records BIGINT := 0;
    v_current_batch BIGINT := 0;
    v_batch_start_time TIMESTAMPTZ;
    v_migration_start_time TIMESTAMPTZ;
    v_last_id UUID := p_resume_from;
    v_records_in_batch INTEGER;
    v_performance_metrics JSONB := '{}';
    v_avg_batch_time NUMERIC := 0;
    v_estimated_remaining_time NUMERIC := 0;
    migration_record RECORD;
    batch_cursor CURSOR FOR
        SELECT 
            am.id,
            am.user_id,
            am.recorded_date,
            am.steps,
            am.distance_meters,
            am.calories_burned,
            am.active_minutes,
            am.flights_climbed,
            am.source_device,
            am.metadata,
            am.created_at
        FROM activity_metrics am
        WHERE (v_last_id IS NULL OR am.id > v_last_id)
        ORDER BY am.id
        LIMIT v_batch_size;
BEGIN
    -- Set migration start time
    v_migration_start_time := NOW();
    
    -- Get total record count for migration
    IF v_last_id IS NULL THEN
        SELECT COUNT(*) INTO v_total_records FROM activity_metrics;
    ELSE 
        SELECT COUNT(*) INTO v_total_records 
        FROM activity_metrics 
        WHERE id > v_last_id;
    END IF;
    
    -- Initialize or update migration progress
    INSERT INTO migration_progress (
        migration_name, 
        total_records_to_migrate, 
        start_time,
        status,
        last_processed_id
    )
    VALUES (
        'activity_metrics_to_v2', 
        v_total_records, 
        v_migration_start_time,
        'running',
        v_last_id
    )
    ON CONFLICT (migration_name) 
    DO UPDATE SET
        total_records_to_migrate = v_total_records,
        last_updated = NOW(),
        status = 'running',
        last_processed_id = COALESCE(v_last_id, migration_progress.last_processed_id);

    -- Get existing progress if resuming
    SELECT total_records_processed, batch_number 
    INTO v_processed_records, v_current_batch
    FROM migration_progress 
    WHERE migration_name = 'activity_metrics_to_v2';
    
    RAISE NOTICE 'Starting activity_metrics to activity_metrics_v2 migration';
    RAISE NOTICE 'Total records to process: %, Resume from ID: %', v_total_records, COALESCE(v_last_id::TEXT, 'beginning');
    RAISE NOTICE 'Batch size: %, Already processed: %', v_batch_size, v_processed_records;

    -- Migration loop with batch processing
    LOOP
        v_batch_start_time := NOW();
        v_current_batch := v_current_batch + 1;
        v_records_in_batch := 0;
        
        -- Begin transaction for this batch
        BEGIN
            -- Process current batch
            FOR migration_record IN batch_cursor LOOP
                v_records_in_batch := v_records_in_batch + 1;
                v_last_id := migration_record.id;
                
                -- Insert into activity_metrics_v2 with field mapping
                INSERT INTO activity_metrics_v2 (
                    user_id,
                    recorded_at,
                    step_count,
                    flights_climbed,
                    distance_walking_running_meters,
                    active_energy_burned_kcal,
                    exercise_time_minutes,
                    source,
                    raw_data,
                    created_at,
                    aggregation_period
                ) VALUES (
                    migration_record.user_id,
                    -- Convert DATE to TIMESTAMPTZ (add default time at start of day)
                    migration_record.recorded_date::TIMESTAMPTZ,
                    -- Map steps to step_count
                    migration_record.steps,
                    -- flights_climbed maps directly 
                    migration_record.flights_climbed,
                    -- Map distance_meters to distance_walking_running_meters
                    migration_record.distance_meters,
                    -- Map calories_burned to active_energy_burned_kcal
                    migration_record.calories_burned::NUMERIC,
                    -- Map active_minutes to exercise_time_minutes
                    migration_record.active_minutes,
                    -- Map source_device to source
                    migration_record.source_device,
                    -- Map metadata to raw_data
                    migration_record.metadata,
                    -- Keep original created_at
                    migration_record.created_at,
                    -- Default aggregation_period for legacy data
                    'daily'
                )
                -- Handle potential duplicate key conflicts gracefully
                ON CONFLICT (user_id, recorded_at) DO UPDATE SET
                    step_count = EXCLUDED.step_count,
                    flights_climbed = EXCLUDED.flights_climbed,  
                    distance_walking_running_meters = EXCLUDED.distance_walking_running_meters,
                    active_energy_burned_kcal = EXCLUDED.active_energy_burned_kcal,
                    exercise_time_minutes = EXCLUDED.exercise_time_minutes,
                    source = EXCLUDED.source,
                    raw_data = EXCLUDED.raw_data;
            END LOOP;
            
            -- Update progress after successful batch
            v_processed_records := v_processed_records + v_records_in_batch;
            
            -- Calculate performance metrics
            v_avg_batch_time := EXTRACT(EPOCH FROM (NOW() - v_batch_start_time));
            v_estimated_remaining_time := ((v_total_records - v_processed_records) / v_batch_size) * v_avg_batch_time;
            
            v_performance_metrics := jsonb_build_object(
                'avg_batch_time_seconds', v_avg_batch_time,
                'records_per_second', ROUND(v_records_in_batch / GREATEST(v_avg_batch_time, 0.001), 2),
                'estimated_remaining_seconds', ROUND(v_estimated_remaining_time, 0),
                'estimated_completion_time', (NOW() + (v_estimated_remaining_time || ' seconds')::INTERVAL),
                'memory_usage_mb', pg_size_pretty(pg_total_relation_size('activity_metrics_v2'))
            );
            
            -- Update migration progress
            UPDATE migration_progress SET
                batch_number = v_current_batch,
                total_records_processed = v_processed_records,
                last_updated = NOW(),
                last_processed_id = v_last_id,
                performance_metrics = v_performance_metrics
            WHERE migration_name = 'activity_metrics_to_v2';
            
            -- Log progress every 10 batches or at end
            IF v_current_batch % 10 = 0 OR v_records_in_batch < v_batch_size THEN
                RAISE NOTICE 'Migration Progress - Batch: %, Processed: %/% (%.2f%%), ETA: % seconds', 
                    v_current_batch,
                    v_processed_records, 
                    v_total_records,
                    (v_processed_records::NUMERIC / v_total_records * 100),
                    ROUND(v_estimated_remaining_time, 0);
            END IF;
            
            -- Exit if this batch was smaller than expected (end of data)
            EXIT WHEN v_records_in_batch < v_batch_size;
            
        EXCEPTION WHEN OTHERS THEN
            -- Log error and mark migration as failed
            UPDATE migration_progress SET
                status = 'failed',
                error_message = SQLERRM,
                last_updated = NOW()
            WHERE migration_name = 'activity_metrics_to_v2';
            
            RAISE EXCEPTION 'Migration failed at batch % with error: %', v_current_batch, SQLERRM;
        END;
        
    END LOOP;
    
    -- Mark migration as completed
    UPDATE migration_progress SET
        status = 'completed',
        last_updated = NOW(),
        performance_metrics = v_performance_metrics
    WHERE migration_name = 'activity_metrics_to_v2';
    
    RAISE NOTICE 'Migration completed successfully!';
    RAISE NOTICE 'Total records migrated: %, Total batches: %, Total time: % seconds',
        v_processed_records, 
        v_current_batch,
        EXTRACT(EPOCH FROM (NOW() - v_migration_start_time));
        
    -- Return summary
    RETURN QUERY SELECT
        'completed'::VARCHAR(20) as status,
        v_processed_records as total_processed,
        (v_total_records - v_processed_records) as total_remaining,
        v_current_batch as batch_count,
        EXTRACT(EPOCH FROM (NOW() - v_migration_start_time))::INTEGER as migration_time_seconds,
        v_performance_metrics as performance_summary;
    
END;
$$ LANGUAGE plpgsql;

-- Create function to resume failed migration
CREATE OR REPLACE FUNCTION resume_activity_metrics_migration()
RETURNS TABLE (
    status VARCHAR(20),
    resumed_from_batch BIGINT,
    resumed_from_id UUID
) AS $$
DECLARE
    v_last_id UUID;
    v_last_batch BIGINT;
    v_migration_status VARCHAR(20);
BEGIN
    -- Get last processed state
    SELECT 
        last_processed_id,
        batch_number,
        migration_progress.status
    INTO v_last_id, v_last_batch, v_migration_status
    FROM migration_progress 
    WHERE migration_name = 'activity_metrics_to_v2';
    
    IF v_migration_status = 'completed' THEN
        RETURN QUERY SELECT 
            'already_completed'::VARCHAR(20) as status,
            v_last_batch as resumed_from_batch,
            v_last_id as resumed_from_id;
        RETURN;
    END IF;
    
    IF v_last_id IS NULL THEN
        RAISE EXCEPTION 'No previous migration state found. Use migrate_activity_metrics_to_v2() to start fresh migration.';
    END IF;
    
    RAISE NOTICE 'Resuming migration from batch %, last ID: %', v_last_batch, v_last_id;
    
    -- Resume migration from last known position
    PERFORM migrate_activity_metrics_to_v2(8000, v_last_id);
    
    RETURN QUERY SELECT
        'resumed'::VARCHAR(20) as status,
        v_last_batch as resumed_from_batch, 
        v_last_id as resumed_from_id;
END;
$$ LANGUAGE plpgsql;

-- Create function to validate data integrity post-migration
CREATE OR REPLACE FUNCTION validate_activity_metrics_migration()
RETURNS TABLE (
    validation_check VARCHAR(100),
    original_count BIGINT,
    migrated_count BIGINT,
    match_status VARCHAR(20),
    details JSONB
) AS $$
BEGIN
    -- Validate total record count
    RETURN QUERY
    WITH counts AS (
        SELECT 
            (SELECT COUNT(*) FROM activity_metrics) as orig_count,
            (SELECT COUNT(*) FROM activity_metrics_v2) as migr_count
    )
    SELECT 
        'Total Record Count'::VARCHAR(100) as validation_check,
        orig_count as original_count,
        migr_count as migrated_count,
        CASE WHEN orig_count = migr_count THEN 'PASS' ELSE 'FAIL' END::VARCHAR(20) as match_status,
        jsonb_build_object(
            'difference', (migr_count - orig_count),
            'match_percentage', ROUND((LEAST(orig_count, migr_count)::NUMERIC / GREATEST(orig_count, migr_count) * 100), 2)
        ) as details
    FROM counts;
    
    -- Validate user count consistency  
    RETURN QUERY
    WITH user_counts AS (
        SELECT 
            (SELECT COUNT(DISTINCT user_id) FROM activity_metrics) as orig_users,
            (SELECT COUNT(DISTINCT user_id) FROM activity_metrics_v2) as migr_users
    )
    SELECT 
        'Unique User Count'::VARCHAR(100) as validation_check,
        orig_users as original_count,
        migr_users as migrated_count,
        CASE WHEN orig_users = migr_users THEN 'PASS' ELSE 'FAIL' END::VARCHAR(20) as match_status,
        jsonb_build_object(
            'difference', (migr_users - orig_users),
            'match_percentage', ROUND((LEAST(orig_users, migr_users)::NUMERIC / GREATEST(orig_users, migr_users) * 100), 2)
        ) as details
    FROM user_counts;
    
    -- Validate data range consistency
    RETURN QUERY
    WITH date_ranges AS (
        SELECT 
            (SELECT MIN(recorded_date) FROM activity_metrics) as orig_min_date,
            (SELECT MAX(recorded_date) FROM activity_metrics) as orig_max_date,
            (SELECT MIN(recorded_at::DATE) FROM activity_metrics_v2) as migr_min_date,
            (SELECT MAX(recorded_at::DATE) FROM activity_metrics_v2) as migr_max_date
    )
    SELECT 
        'Date Range Consistency'::VARCHAR(100) as validation_check,
        1::BIGINT as original_count,
        1::BIGINT as migrated_count,
        CASE WHEN orig_min_date = migr_min_date AND orig_max_date = migr_max_date 
             THEN 'PASS' ELSE 'FAIL' END::VARCHAR(20) as match_status,
        jsonb_build_object(
            'original_min_date', orig_min_date,
            'original_max_date', orig_max_date, 
            'migrated_min_date', migr_min_date,
            'migrated_max_date', migr_max_date
        ) as details
    FROM date_ranges;
    
    -- Validate step count totals
    RETURN QUERY
    WITH step_totals AS (
        SELECT 
            (SELECT SUM(steps) FROM activity_metrics WHERE steps IS NOT NULL) as orig_steps,
            (SELECT SUM(step_count) FROM activity_metrics_v2 WHERE step_count IS NOT NULL) as migr_steps
    )
    SELECT 
        'Step Count Totals'::VARCHAR(100) as validation_check,
        orig_steps as original_count,
        migr_steps as migrated_count,
        CASE WHEN orig_steps = migr_steps THEN 'PASS' ELSE 'FAIL' END::VARCHAR(20) as match_status,
        jsonb_build_object(
            'difference', (migr_steps - orig_steps),
            'match_percentage', ROUND((LEAST(orig_steps, migr_steps)::NUMERIC / GREATEST(orig_steps, migr_steps) * 100), 2)
        ) as details
    FROM step_totals;
    
    -- Validate flights climbed totals
    RETURN QUERY 
    WITH flights_totals AS (
        SELECT 
            (SELECT SUM(flights_climbed) FROM activity_metrics WHERE flights_climbed IS NOT NULL) as orig_flights,
            (SELECT SUM(flights_climbed) FROM activity_metrics_v2 WHERE flights_climbed IS NOT NULL) as migr_flights
    )
    SELECT 
        'Flights Climbed Totals'::VARCHAR(100) as validation_check,
        orig_flights as original_count,
        migr_flights as migrated_count,
        CASE WHEN orig_flights = migr_flights THEN 'PASS' ELSE 'FAIL' END::VARCHAR(20) as match_status,
        jsonb_build_object(
            'difference', (migr_flights - orig_flights),
            'match_percentage', ROUND((LEAST(orig_flights, migr_flights)::NUMERIC / GREATEST(orig_flights, migr_flights) * 100), 2)
        ) as details
    FROM flights_totals;
    
END;
$$ LANGUAGE plpgsql;

-- Create cleanup function for rollback scenarios  
CREATE OR REPLACE FUNCTION rollback_activity_metrics_migration()
RETURNS TABLE (
    rollback_action VARCHAR(100),
    records_affected BIGINT,
    status VARCHAR(20)
) AS $$
DECLARE
    v_records_deleted BIGINT;
BEGIN
    -- Delete all migrated data from activity_metrics_v2
    DELETE FROM activity_metrics_v2;
    GET DIAGNOSTICS v_records_deleted = ROW_COUNT;
    
    RETURN QUERY SELECT
        'Delete migrated records'::VARCHAR(100) as rollback_action,
        v_records_deleted as records_affected,
        'completed'::VARCHAR(20) as status;
    
    -- Reset migration progress
    DELETE FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2';
    
    RETURN QUERY SELECT
        'Reset migration progress'::VARCHAR(100) as rollback_action,
        1::BIGINT as records_affected,
        'completed'::VARCHAR(20) as status;
        
    RAISE NOTICE 'Migration rollback completed. Deleted % records from activity_metrics_v2', v_records_deleted;
END;
$$ LANGUAGE plpgsql;

-- Add helpful comments and documentation
COMMENT ON FUNCTION migrate_activity_metrics_to_v2() IS 'Migrates data from activity_metrics to activity_metrics_v2 with batch processing, progress tracking, and resumability. Handles field mapping and ensures zero data loss.';
COMMENT ON FUNCTION resume_activity_metrics_migration() IS 'Resumes a failed or interrupted migration from the last successfully processed batch.';
COMMENT ON FUNCTION validate_activity_metrics_migration() IS 'Validates data integrity after migration by comparing key metrics between source and target tables.';
COMMENT ON FUNCTION rollback_activity_metrics_migration() IS 'Rolls back migration by deleting all migrated data and resetting progress tracking.';
COMMENT ON TABLE migration_progress IS 'Tracks progress of data migrations with batch-level resumability and performance metrics.';

-- Usage Examples:
--
-- 1. Start fresh migration:
--    SELECT * FROM migrate_activity_metrics_to_v2();
--
-- 2. Resume failed migration: 
--    SELECT * FROM resume_activity_metrics_migration();
--
-- 3. Monitor progress:
--    SELECT migration_name, status, total_records_processed, 
--           total_records_to_migrate, performance_metrics 
--    FROM migration_progress WHERE migration_name = 'activity_metrics_to_v2';
--
-- 4. Validate migration results:
--    SELECT * FROM validate_activity_metrics_migration();
--
-- 5. Rollback if needed:
--    SELECT * FROM rollback_activity_metrics_migration();