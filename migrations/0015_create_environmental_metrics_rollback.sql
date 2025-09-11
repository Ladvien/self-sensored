-- Rollback migration for environmental_metrics table
-- Safely removes environmental_metrics table and related objects

-- Drop triggers first (to avoid dependency issues)
DROP TRIGGER IF EXISTS trigger_environmental_safety_events ON environmental_metrics;

-- Drop views (depend on table)
DROP VIEW IF EXISTS environmental_metrics_daily;
DROP VIEW IF EXISTS environmental_metrics_hourly;

-- Drop functions
DROP FUNCTION IF EXISTS monitor_environmental_metrics_performance();
DROP FUNCTION IF EXISTS log_environmental_safety_event();

-- Drop all partition tables
DO $$
DECLARE
    partition_name TEXT;
BEGIN
    -- Find and drop all environmental_metrics partitions
    FOR partition_name IN 
        SELECT schemaname||'.'||tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'environmental_metrics_%' 
          AND schemaname = 'public'
    LOOP
        EXECUTE 'DROP TABLE IF EXISTS ' || partition_name || ' CASCADE';
        RAISE NOTICE 'Dropped partition: %', partition_name;
    END LOOP;
END $$;

-- Drop the main partitioned table
DROP TABLE IF EXISTS environmental_metrics CASCADE;

-- Drop safety_events table if it was created by this migration
-- (Only drop if it has no other data/dependencies)
DO $$
BEGIN
    -- Check if safety_events table exists and has only environmental event types
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'safety_events') THEN
        -- Only drop if all events are environmental-related
        IF NOT EXISTS (
            SELECT 1 FROM safety_events 
            WHERE event_type NOT IN ('fall_detected', 'dangerous_audio_exposure', 'extreme_uv_exposure', 'dangerous_air_quality')
        ) THEN
            DROP TABLE safety_events CASCADE;
            RAISE NOTICE 'Dropped safety_events table (contained only environmental events)';
        ELSE
            -- Just clean environmental events, keep the table
            DELETE FROM safety_events 
            WHERE event_type IN ('fall_detected', 'dangerous_audio_exposure', 'extreme_uv_exposure', 'dangerous_air_quality');
            RAISE NOTICE 'Cleaned environmental events from safety_events table';
        END IF;
    END IF;
END $$;

-- Rollback completed successfully
-- Removed environmental_metrics table and all related objects
-- Safely handled safety_events table cleanup
-- All partitions and indexes automatically dropped with CASCADE