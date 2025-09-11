-- Comprehensive Heart Rate Constraint Update
-- Ensures all tables (regular and partitioned) have consistent 15-300 BPM constraints
-- Addresses AUDIT-004: Database constraint alignment with application validation

-- First, update regular tables (extending migration 0010 to be comprehensive)
-- =============================================================================

-- Update heart_rate_metrics table constraints
ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_heart_rate_check 
        CHECK (heart_rate >= 15 AND heart_rate <= 300);

ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_resting_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_resting_heart_rate_check 
        CHECK (resting_heart_rate IS NULL OR (resting_heart_rate >= 15 AND resting_heart_rate <= 300));

-- Update workouts table constraints (note: this table uses different column names)
ALTER TABLE workouts 
    DROP CONSTRAINT IF EXISTS workouts_average_heart_rate_check,
    ADD CONSTRAINT workouts_average_heart_rate_check 
        CHECK (average_heart_rate IS NULL OR (average_heart_rate >= 15 AND average_heart_rate <= 300));

ALTER TABLE workouts 
    DROP CONSTRAINT IF EXISTS workouts_max_heart_rate_check,
    ADD CONSTRAINT workouts_max_heart_rate_check 
        CHECK (max_heart_rate IS NULL OR (max_heart_rate >= 15 AND max_heart_rate <= 300));

-- Update blood_pressure_metrics table pulse constraint  
ALTER TABLE blood_pressure_metrics 
    DROP CONSTRAINT IF EXISTS blood_pressure_metrics_pulse_check,
    ADD CONSTRAINT blood_pressure_metrics_pulse_check 
        CHECK (pulse IS NULL OR (pulse >= 15 AND pulse <= 300));

-- Update partitioned tables constraints
-- =============================================================================

-- Update heart_rate_metrics_partitioned table constraints
ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_min_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_min_bpm_check 
        CHECK (min_bpm IS NULL OR (min_bpm >= 15 AND min_bpm <= 300));

ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_avg_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_avg_bpm_check 
        CHECK (avg_bpm IS NULL OR (avg_bpm >= 15 AND avg_bpm <= 300));

ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_max_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_max_bpm_check 
        CHECK (max_bpm IS NULL OR (max_bpm >= 15 AND max_bpm <= 300));

-- Check for any existing partition tables and update their constraints
-- =============================================================================

DO $$ 
DECLARE
    partition_record RECORD;
    constraint_name TEXT;
BEGIN
    -- Update constraints on existing heart rate partition tables
    FOR partition_record IN 
        SELECT schemaname, tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'heart_rate_metrics_partitioned_%'
        AND tablename ~ '_[0-9]{4}_[0-9]{2}$'
    LOOP
        -- Update min_bpm constraint
        constraint_name := partition_record.tablename || '_min_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (min_bpm IS NULL OR (min_bpm >= 15 AND min_bpm <= 300))', 
                      partition_record.tablename, constraint_name);
        
        -- Update avg_bpm constraint  
        constraint_name := partition_record.tablename || '_avg_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (avg_bpm IS NULL OR (avg_bpm >= 15 AND avg_bpm <= 300))', 
                      partition_record.tablename, constraint_name);
        
        -- Update max_bpm constraint
        constraint_name := partition_record.tablename || '_max_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (max_bpm IS NULL OR (max_bpm >= 15 AND max_bpm <= 300))', 
                      partition_record.tablename, constraint_name);
                      
        RAISE NOTICE 'Updated heart rate constraints for partition: %', partition_record.tablename;
    END LOOP;
END $$;

-- Data validation and cleanup
-- =============================================================================

-- Check for any existing data that might violate the new constraints
-- This will help identify any data issues before they cause problems

DO $$
DECLARE
    invalid_count INTEGER;
BEGIN
    -- Check regular heart_rate_metrics table
    SELECT COUNT(*) INTO invalid_count 
    FROM heart_rate_metrics 
    WHERE heart_rate < 15 OR heart_rate > 300 
       OR (resting_heart_rate IS NOT NULL AND (resting_heart_rate < 15 OR resting_heart_rate > 300));
    
    IF invalid_count > 0 THEN
        RAISE WARNING 'Found % rows in heart_rate_metrics with invalid heart rate values (outside 15-300 BPM)', invalid_count;
    END IF;
    
    -- Check workouts table
    SELECT COUNT(*) INTO invalid_count 
    FROM workouts 
    WHERE (average_heart_rate IS NOT NULL AND (average_heart_rate < 15 OR average_heart_rate > 300))
       OR (max_heart_rate IS NOT NULL AND (max_heart_rate < 15 OR max_heart_rate > 300));
    
    IF invalid_count > 0 THEN
        RAISE WARNING 'Found % rows in workouts with invalid heart rate values (outside 15-300 BPM)', invalid_count;
    END IF;
    
    -- Check blood_pressure_metrics pulse
    SELECT COUNT(*) INTO invalid_count 
    FROM blood_pressure_metrics 
    WHERE pulse IS NOT NULL AND (pulse < 15 OR pulse > 300);
    
    IF invalid_count > 0 THEN
        RAISE WARNING 'Found % rows in blood_pressure_metrics with invalid pulse values (outside 15-300 BPM)', invalid_count;
    END IF;
    
    -- Check partitioned heart_rate_metrics table
    SELECT COUNT(*) INTO invalid_count 
    FROM heart_rate_metrics_partitioned 
    WHERE (min_bpm IS NOT NULL AND (min_bpm < 15 OR min_bpm > 300))
       OR (avg_bpm IS NOT NULL AND (avg_bpm < 15 OR avg_bpm > 300))
       OR (max_bpm IS NOT NULL AND (max_bpm < 15 OR max_bpm > 300));
    
    IF invalid_count > 0 THEN
        RAISE WARNING 'Found % rows in heart_rate_metrics_partitioned with invalid BPM values (outside 15-300 BPM)', invalid_count;
    END IF;
    
    RAISE NOTICE 'Heart rate constraint validation completed. Check warnings above for any data issues.';
END $$;

-- Summary of changes
-- =============================================================================
RAISE NOTICE 'Migration 0011_comprehensive_heart_rate_constraints.sql completed successfully.';
RAISE NOTICE 'Updated constraints to require 15-300 BPM range for all heart rate related fields.';
RAISE NOTICE 'This ensures database constraints align with application validation rules.';