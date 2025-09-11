-- Rollback Migration: Comprehensive Heart Rate Constraint Update
-- Reverts constraints from 15-300 BPM back to original ranges
-- Rollback for migration 0011_comprehensive_heart_rate_constraints.sql

-- Rollback regular tables constraints
-- =============================================================================

-- Rollback heart_rate_metrics table constraints to original state
ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_heart_rate_check 
        CHECK (heart_rate > 0 AND heart_rate <= 300);

ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_resting_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_resting_heart_rate_check 
        CHECK (resting_heart_rate > 0 AND resting_heart_rate <= 200);

-- Rollback workouts table constraints to original state
ALTER TABLE workouts 
    DROP CONSTRAINT IF EXISTS workouts_average_heart_rate_check,
    ADD CONSTRAINT workouts_average_heart_rate_check 
        CHECK (average_heart_rate > 0 AND average_heart_rate <= 300);

ALTER TABLE workouts 
    DROP CONSTRAINT IF EXISTS workouts_max_heart_rate_check,
    ADD CONSTRAINT workouts_max_heart_rate_check 
        CHECK (max_heart_rate > 0 AND max_heart_rate <= 300);

-- Rollback blood_pressure_metrics table pulse constraint to original state
ALTER TABLE blood_pressure_metrics 
    DROP CONSTRAINT IF EXISTS blood_pressure_metrics_pulse_check,
    ADD CONSTRAINT blood_pressure_metrics_pulse_check 
        CHECK (pulse > 0 AND pulse <= 300);

-- Rollback partitioned tables constraints
-- =============================================================================

-- Rollback heart_rate_metrics_partitioned table constraints to original state
ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_min_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_min_bpm_check 
        CHECK (min_bpm > 0 AND min_bpm <= 300);

ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_avg_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_avg_bpm_check 
        CHECK (avg_bpm > 0 AND avg_bpm <= 300);

ALTER TABLE heart_rate_metrics_partitioned 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_partitioned_max_bpm_check,
    ADD CONSTRAINT heart_rate_metrics_partitioned_max_bpm_check 
        CHECK (max_bpm > 0 AND max_bpm <= 300);

-- Rollback constraints on existing partition tables
-- =============================================================================

DO $$ 
DECLARE
    partition_record RECORD;
    constraint_name TEXT;
BEGIN
    -- Rollback constraints on existing heart rate partition tables
    FOR partition_record IN 
        SELECT schemaname, tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'heart_rate_metrics_partitioned_%'
        AND tablename ~ '_[0-9]{4}_[0-9]{2}$'
    LOOP
        -- Rollback min_bpm constraint
        constraint_name := partition_record.tablename || '_min_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (min_bpm > 0 AND min_bpm <= 300)', 
                      partition_record.tablename, constraint_name);
        
        -- Rollback avg_bpm constraint  
        constraint_name := partition_record.tablename || '_avg_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (avg_bpm > 0 AND avg_bpm <= 300)', 
                      partition_record.tablename, constraint_name);
        
        -- Rollback max_bpm constraint
        constraint_name := partition_record.tablename || '_max_bpm_check';
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT IF EXISTS %I', 
                      partition_record.tablename, constraint_name);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT %I CHECK (max_bpm > 0 AND max_bpm <= 300)', 
                      partition_record.tablename, constraint_name);
                      
        RAISE NOTICE 'Rolled back heart rate constraints for partition: %', partition_record.tablename;
    END LOOP;
END $$;

-- Summary of rollback
-- =============================================================================
RAISE NOTICE 'Rollback migration 0011_comprehensive_heart_rate_constraints_rollback.sql completed successfully.';
RAISE NOTICE 'Reverted constraints back to original ranges (> 0 for minimums, <= 300/200 for maximums).';
RAISE NOTICE 'Database constraints no longer align with application validation rules.';