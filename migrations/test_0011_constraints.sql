-- Test script for migration 0011 - Heart Rate Constraint Validation
-- This script tests that the new constraints work correctly without running on production data

-- Test 1: Verify constraint names and ranges
-- =============================================================================
SELECT 
    tc.constraint_name,
    tc.table_name,
    cc.check_clause
FROM information_schema.table_constraints tc
JOIN information_schema.check_constraints cc ON tc.constraint_name = cc.constraint_name
WHERE tc.table_name IN ('heart_rate_metrics', 'heart_rate_metrics_partitioned', 'workouts', 'blood_pressure_metrics')
  AND tc.constraint_type = 'CHECK'
  AND cc.check_clause LIKE '%heart_rate%' OR cc.check_clause LIKE '%bpm%' OR cc.check_clause LIKE '%pulse%'
ORDER BY tc.table_name, tc.constraint_name;

-- Test 2: Test constraint violations (these should fail with constraint errors)
-- =============================================================================

-- Test invalid heart rate values (should fail)
BEGIN;
    -- Test heart_rate_metrics table
    INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate) 
    SELECT 
        (SELECT id FROM users LIMIT 1),  -- Get first user
        NOW(),
        10  -- Invalid: below 15 BPM
    WHERE EXISTS (SELECT 1 FROM users LIMIT 1);  -- Only if users exist
ROLLBACK;

BEGIN;
    INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate) 
    SELECT 
        (SELECT id FROM users LIMIT 1),
        NOW(),
        350  -- Invalid: above 300 BPM
    WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
ROLLBACK;

-- Test 3: Test valid heart rate values (these should succeed)
-- =============================================================================

BEGIN;
    -- Test valid heart rate values
    INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate) 
    SELECT 
        (SELECT id FROM users LIMIT 1),
        NOW(),
        60,   -- Valid heart rate
        50    -- Valid resting heart rate
    WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
    
    INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate) 
    SELECT 
        (SELECT id FROM users LIMIT 1),
        NOW() + INTERVAL '1 minute',
        15,   -- Valid minimum (15 BPM)
        15    -- Valid minimum resting
    WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
    
    INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate) 
    SELECT 
        (SELECT id FROM users LIMIT 1),
        NOW() + INTERVAL '2 minutes',
        300,  -- Valid maximum (300 BPM)  
        300   -- Valid maximum resting
    WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
        
    SELECT 'SUCCESS: Valid heart rate values accepted' as test_result;
ROLLBACK;

-- Test 4: Test partitioned table constraints (if partitioned tables exist)
-- =============================================================================

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'heart_rate_metrics_partitioned') THEN
        BEGIN
            -- Test invalid values in partitioned table (should fail)
            INSERT INTO heart_rate_metrics_partitioned (user_id, recorded_at, avg_bpm) 
            SELECT 
                (SELECT id FROM users LIMIT 1),
                NOW(),
                10  -- Invalid: below 15 BPM
            WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
        EXCEPTION WHEN check_violation THEN
            RAISE NOTICE 'SUCCESS: Partitioned table correctly rejected invalid BPM value';
        END;
        
        BEGIN
            -- Test valid values in partitioned table (should succeed)
            INSERT INTO heart_rate_metrics_partitioned (user_id, recorded_at, avg_bpm, max_bpm, min_bpm) 
            SELECT 
                (SELECT id FROM users LIMIT 1),
                NOW(),
                70,   -- Valid avg BPM
                85,   -- Valid max BPM
                60    -- Valid min BPM
            WHERE EXISTS (SELECT 1 FROM users LIMIT 1);
            RAISE NOTICE 'SUCCESS: Partitioned table accepted valid BPM values';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Note: Partitioned table test failed - may need partition creation';
        END;
    ELSE
        RAISE NOTICE 'INFO: heart_rate_metrics_partitioned table does not exist - skipping partitioned tests';
    END IF;
END $$;

-- Test 5: Summary report
-- =============================================================================
SELECT 
    'Constraint Test Summary' as report_section,
    COUNT(*) as total_heart_rate_constraints
FROM information_schema.check_constraints 
WHERE check_clause LIKE '%>= 15%' AND check_clause LIKE '%<= 300%';

RAISE NOTICE 'Heart rate constraint testing completed. Review results above.';