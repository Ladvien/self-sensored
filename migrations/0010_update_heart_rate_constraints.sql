-- Update heart rate validation constraints to allow minimum of 15 BPM instead of 20 BPM
-- This addresses validation errors for legitimate low heart rate values

-- Update heart_rate_metrics table constraints
ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_heart_rate_check 
        CHECK (heart_rate >= 15 AND heart_rate <= 300);

ALTER TABLE heart_rate_metrics 
    DROP CONSTRAINT IF EXISTS heart_rate_metrics_resting_heart_rate_check,
    ADD CONSTRAINT heart_rate_metrics_resting_heart_rate_check 
        CHECK (resting_heart_rate IS NULL OR (resting_heart_rate >= 15 AND resting_heart_rate <= 300));

-- Update workout_metrics table constraints
ALTER TABLE workout_metrics 
    DROP CONSTRAINT IF EXISTS workout_metrics_avg_heart_rate_check,
    ADD CONSTRAINT workout_metrics_avg_heart_rate_check 
        CHECK (avg_heart_rate IS NULL OR (avg_heart_rate >= 15 AND avg_heart_rate <= 300));

ALTER TABLE workout_metrics 
    DROP CONSTRAINT IF EXISTS workout_metrics_max_heart_rate_check,
    ADD CONSTRAINT workout_metrics_max_heart_rate_check 
        CHECK (max_heart_rate IS NULL OR (max_heart_rate >= 15 AND max_heart_rate <= 300));

-- Update blood_pressure_metrics table pulse constraint  
ALTER TABLE blood_pressure_metrics 
    DROP CONSTRAINT IF EXISTS blood_pressure_metrics_pulse_check,
    ADD CONSTRAINT blood_pressure_metrics_pulse_check 
        CHECK (pulse IS NULL OR (pulse >= 15 AND pulse <= 300));

-- Note: The partitioned tables inherit these constraints from their parent tables
-- No need to update partition constraints separately