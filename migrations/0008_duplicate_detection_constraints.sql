-- Migration: Add unique constraints for duplicate detection
-- Description: Adds unique constraints to prevent duplicate health metrics

-- Sleep metrics: Prevent duplicate sleep sessions
-- A user can only have one sleep session with the same start and end time
DO $$ BEGIN
    ALTER TABLE sleep_metrics 
    ADD CONSTRAINT sleep_metrics_user_time_unique 
    UNIQUE (user_id, sleep_start, sleep_end);
EXCEPTION WHEN duplicate_object THEN
    RAISE NOTICE 'Constraint sleep_metrics_user_time_unique already exists';
END $$;

-- Blood pressure: One reading per timestamp
-- Prevents duplicate blood pressure readings at the exact same time
DO $$ BEGIN
    ALTER TABLE blood_pressure_metrics 
    ADD CONSTRAINT blood_pressure_user_time_unique 
    UNIQUE (user_id, recorded_at);
EXCEPTION WHEN duplicate_object THEN
    RAISE NOTICE 'Constraint blood_pressure_user_time_unique already exists';
END $$;

-- Workouts: Prevent duplicate workouts
-- A user can only have one workout starting at a specific time
DO $$ BEGIN
    ALTER TABLE workouts 
    ADD CONSTRAINT workouts_user_started_unique 
    UNIQUE (user_id, started_at);
EXCEPTION WHEN duplicate_object THEN
    RAISE NOTICE 'Constraint workouts_user_started_unique already exists';
END $$;

-- Add indexes to support the constraints and improve query performance
CREATE INDEX IF NOT EXISTS idx_sleep_metrics_user_times 
ON sleep_metrics(user_id, sleep_start, sleep_end);

CREATE INDEX IF NOT EXISTS idx_blood_pressure_user_time 
ON blood_pressure_metrics(user_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_workouts_user_started 
ON workouts(user_id, started_at);

-- Add updated_at columns to track when records are refreshed
ALTER TABLE heart_rate_metrics 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();

ALTER TABLE sleep_metrics 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();

ALTER TABLE activity_metrics 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();

ALTER TABLE blood_pressure_metrics 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();

ALTER TABLE workouts 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW();

-- Create function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create triggers for automatic updated_at updates
CREATE TRIGGER update_heart_rate_metrics_updated_at 
BEFORE UPDATE ON heart_rate_metrics 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_sleep_metrics_updated_at 
BEFORE UPDATE ON sleep_metrics 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_activity_metrics_updated_at 
BEFORE UPDATE ON activity_metrics 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_blood_pressure_metrics_updated_at 
BEFORE UPDATE ON blood_pressure_metrics 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_workouts_updated_at 
BEFORE UPDATE ON workouts 
FOR EACH ROW 
EXECUTE FUNCTION update_updated_at_column();

-- Add comment to track migration purpose
COMMENT ON CONSTRAINT sleep_metrics_user_time_unique ON sleep_metrics 
IS 'Prevents duplicate sleep sessions for the same user with identical start and end times';

COMMENT ON CONSTRAINT blood_pressure_user_time_unique ON blood_pressure_metrics 
IS 'Prevents duplicate blood pressure readings for the same user at the same timestamp';

COMMENT ON CONSTRAINT workouts_user_started_unique ON workouts 
IS 'Prevents duplicate workouts for the same user starting at the same time';