-- Data Migration to Partitioned Tables
-- Safely migrates data from existing tables to new partitioned versions
-- This migration should be run carefully in production with downtime

-- Function to migrate data safely with transaction per batch
CREATE OR REPLACE FUNCTION migrate_table_data(
    source_table text,
    target_table text,
    batch_size integer DEFAULT 10000
)
RETURNS void AS $$
DECLARE
    total_rows integer;
    processed_rows integer := 0;
    batch_start integer := 0;
BEGIN
    -- Get total row count
    EXECUTE format('SELECT COUNT(*) FROM %I', source_table) INTO total_rows;
    RAISE NOTICE 'Migrating % rows from % to %', total_rows, source_table, target_table;
    
    -- Process in batches to avoid long transactions
    LOOP
        EXECUTE format('
            INSERT INTO %I 
            SELECT * FROM %I 
            ORDER BY created_at 
            LIMIT %s OFFSET %s
            ON CONFLICT DO NOTHING',
            target_table, source_table, batch_size, batch_start
        );
        
        GET DIAGNOSTICS processed_rows = ROW_COUNT;
        batch_start := batch_start + batch_size;
        
        RAISE NOTICE 'Migrated batch: % rows (total: %/%)', 
                     processed_rows, batch_start, total_rows;
        
        -- Exit when no more rows to process
        EXIT WHEN processed_rows = 0;
        
        -- Commit after each batch
        COMMIT;
    END LOOP;
    
    RAISE NOTICE 'Migration complete for %', source_table;
END;
$$ LANGUAGE plpgsql;

-- Function to migrate heart rate data with data transformation
CREATE OR REPLACE FUNCTION migrate_heart_rate_data()
RETURNS void AS $$
BEGIN
    -- Migrate heart rate data with structure transformation
    -- Converting single heart_rate field to min/avg/max structure
    INSERT INTO heart_rate_metrics_partitioned (
        user_id, recorded_at, min_bpm, avg_bpm, max_bpm, source, raw_data, created_at
    )
    SELECT 
        user_id,
        recorded_at,
        CASE 
            WHEN resting_heart_rate IS NOT NULL THEN LEAST(heart_rate, resting_heart_rate)
            ELSE heart_rate - 5 -- Estimate min as avg - 5
        END as min_bpm,
        heart_rate as avg_bpm,
        heart_rate + 10 as max_bpm, -- Estimate max as avg + 10
        source_device as source,
        jsonb_build_object(
            'original_heart_rate', heart_rate,
            'resting_heart_rate', resting_heart_rate,
            'context', context,
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM heart_rate_metrics
    ON CONFLICT (user_id, recorded_at) DO NOTHING;
    
    RAISE NOTICE 'Heart rate data migration complete';
END;
$$ LANGUAGE plpgsql;

-- Function to migrate activity data with structure transformation
CREATE OR REPLACE FUNCTION migrate_activity_data()
RETURNS void AS $$
BEGIN
    -- Transform activity data from daily aggregates to individual metrics
    INSERT INTO activity_metrics_partitioned (
        user_id, recorded_at, metric_type, value, unit, source, raw_data, created_at
    )
    SELECT 
        user_id,
        recorded_date::timestamptz, -- Convert date to timestamptz
        'steps' as metric_type,
        steps as value,
        'count' as unit,
        source_device as source,
        jsonb_build_object(
            'original_row', row_to_json(activity_metrics.*),
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM activity_metrics
    WHERE steps IS NOT NULL
    
    UNION ALL
    
    SELECT 
        user_id,
        recorded_date::timestamptz,
        'distance' as metric_type,
        distance_meters as value,
        'meters' as unit,
        source_device as source,
        jsonb_build_object(
            'original_row', row_to_json(activity_metrics.*),
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM activity_metrics
    WHERE distance_meters IS NOT NULL
    
    UNION ALL
    
    SELECT 
        user_id,
        recorded_date::timestamptz,
        'calories_burned' as metric_type,
        calories_burned as value,
        'kcal' as unit,
        source_device as source,
        jsonb_build_object(
            'original_row', row_to_json(activity_metrics.*),
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM activity_metrics
    WHERE calories_burned IS NOT NULL
    
    UNION ALL
    
    SELECT 
        user_id,
        recorded_date::timestamptz,
        'active_minutes' as metric_type,
        active_minutes as value,
        'minutes' as unit,
        source_device as source,
        jsonb_build_object(
            'original_row', row_to_json(activity_metrics.*),
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM activity_metrics
    WHERE active_minutes IS NOT NULL
    
    UNION ALL
    
    SELECT 
        user_id,
        recorded_date::timestamptz,
        'flights_climbed' as metric_type,
        flights_climbed as value,
        'count' as unit,
        source_device as source,
        jsonb_build_object(
            'original_row', row_to_json(activity_metrics.*),
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM activity_metrics
    WHERE flights_climbed IS NOT NULL
    
    ON CONFLICT (user_id, recorded_at, metric_type) DO NOTHING;
    
    RAISE NOTICE 'Activity data migration complete';
END;
$$ LANGUAGE plpgsql;

-- Function to migrate sleep data with structure transformation
CREATE OR REPLACE FUNCTION migrate_sleep_data()
RETURNS void AS $$
BEGIN
    INSERT INTO sleep_metrics_partitioned (
        user_id, date, asleep_duration_minutes, in_bed_duration_minutes,
        sleep_start, sleep_end, in_bed_start, in_bed_end, sleep_source, raw_data, created_at
    )
    SELECT 
        user_id,
        sleep_start::date as date,
        duration_minutes as asleep_duration_minutes,
        duration_minutes + COALESCE(awake_minutes, 0) as in_bed_duration_minutes,
        sleep_start,
        sleep_end,
        sleep_start - INTERVAL '15 minutes' as in_bed_start, -- Estimate
        sleep_end + INTERVAL '5 minutes' as in_bed_end, -- Estimate
        source_device as sleep_source,
        jsonb_build_object(
            'original_row', row_to_json(sleep_metrics.*),
            'deep_sleep_minutes', deep_sleep_minutes,
            'rem_sleep_minutes', rem_sleep_minutes,
            'light_sleep_minutes', light_sleep_minutes,
            'awake_minutes', awake_minutes,
            'sleep_efficiency', sleep_efficiency,
            'migrated_from_old_schema', true
        ) as raw_data,
        created_at
    FROM sleep_metrics
    ON CONFLICT (user_id, date) DO NOTHING;
    
    RAISE NOTICE 'Sleep data migration complete';
END;
$$ LANGUAGE plpgsql;

-- Main migration execution (commented out - run manually when ready)
-- DO $$
-- BEGIN
--     RAISE NOTICE 'Starting data migration to partitioned tables...';
--     
--     -- Migrate raw ingestions (no structure changes)
--     INSERT INTO raw_ingestions_partitioned (
--         user_id, api_key_id, payload, received_at, processed_at, processing_errors
--     )
--     SELECT 
--         user_id, 
--         api_key_id, 
--         raw_data as payload,
--         ingested_at as received_at,
--         processed_at,
--         CASE 
--             WHEN status = 'failed' THEN jsonb_build_object('error', error_message)
--             ELSE NULL 
--         END as processing_errors
--     FROM raw_ingestions
--     ON CONFLICT DO NOTHING;
--     
--     -- Migrate audit log (no structure changes)
--     INSERT INTO audit_log_partitioned (
--         user_id, api_key_id, action, resource_type, resource_id, 
--         metadata, ip_address, user_agent, created_at
--     )
--     SELECT 
--         user_id, api_key_id, action, resource, null as resource_id,
--         metadata, ip_address, user_agent, created_at
--     FROM audit_log
--     ON CONFLICT DO NOTHING;
--     
--     -- Migrate blood pressure (no structure changes)
--     INSERT INTO blood_pressure_metrics_partitioned (
--         user_id, recorded_at, systolic, diastolic, source, raw_data, created_at
--     )
--     SELECT 
--         user_id, recorded_at, systolic, diastolic, source_device as source,
--         jsonb_build_object(
--             'pulse', pulse,
--             'original_metadata', metadata,
--             'migrated_from_old_schema', true
--         ) as raw_data,
--         created_at
--     FROM blood_pressure_metrics
--     ON CONFLICT (user_id, recorded_at) DO NOTHING;
--     
--     -- Run specialized migration functions
--     PERFORM migrate_heart_rate_data();
--     PERFORM migrate_sleep_data();
--     PERFORM migrate_activity_data();
--     
--     RAISE NOTICE 'Data migration to partitioned tables complete!';
-- END
-- $$;

-- Verification queries to compare data integrity
CREATE OR REPLACE FUNCTION verify_migration()
RETURNS TABLE (
    table_name text,
    original_count bigint,
    migrated_count bigint,
    difference bigint
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        'raw_ingestions'::text,
        (SELECT count(*) FROM raw_ingestions),
        (SELECT count(*) FROM raw_ingestions_partitioned),
        (SELECT count(*) FROM raw_ingestions) - (SELECT count(*) FROM raw_ingestions_partitioned);
        
    RETURN QUERY
    SELECT 
        'heart_rate_metrics'::text,
        (SELECT count(*) FROM heart_rate_metrics),
        (SELECT count(*) FROM heart_rate_metrics_partitioned),
        (SELECT count(*) FROM heart_rate_metrics) - (SELECT count(*) FROM heart_rate_metrics_partitioned);
        
    RETURN QUERY
    SELECT 
        'blood_pressure_metrics'::text,
        (SELECT count(*) FROM blood_pressure_metrics),
        (SELECT count(*) FROM blood_pressure_metrics_partitioned),
        (SELECT count(*) FROM blood_pressure_metrics) - (SELECT count(*) FROM blood_pressure_metrics_partitioned);
END;
$$ LANGUAGE plpgsql;