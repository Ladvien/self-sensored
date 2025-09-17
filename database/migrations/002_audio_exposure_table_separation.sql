-- Migration: Separate Audio Exposure Data from Environmental Metrics
-- Date: 2025-09-17
-- Purpose: Move AudioExposure metrics from environmental_metrics to dedicated audio_exposure_metrics table
-- Related: STORY-DATA-003

-- ============================================================================
-- STEP 1: Create audio_exposure_metrics table if it doesn't exist
-- ============================================================================

CREATE TABLE IF NOT EXISTS audio_exposure_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Audio Exposure Levels (WHO/NIOSH standards)
    environmental_audio_exposure_db DOUBLE PRECISION CHECK (environmental_audio_exposure_db >= 0 AND environmental_audio_exposure_db <= 140),
    headphone_audio_exposure_db DOUBLE PRECISION CHECK (headphone_audio_exposure_db >= 0 AND headphone_audio_exposure_db <= 140),

    -- Exposure Duration and Event Tracking
    exposure_duration_minutes INTEGER NOT NULL DEFAULT 0 CHECK (exposure_duration_minutes >= 0),
    audio_exposure_event BOOLEAN NOT NULL DEFAULT false,

    -- Hearing Health Context
    hearing_protection_used BOOLEAN DEFAULT false,
    environment_type VARCHAR(100),
    activity_during_exposure VARCHAR(100),

    -- Risk Assessment
    daily_noise_dose_percentage DOUBLE PRECISION CHECK (daily_noise_dose_percentage >= 0 AND daily_noise_dose_percentage <= 1000),
    weekly_exposure_hours DOUBLE PRECISION CHECK (weekly_exposure_hours >= 0),

    -- Location Context (for noise mapping)
    location_latitude DOUBLE PRECISION CHECK (location_latitude >= -90 AND location_latitude <= 90),
    location_longitude DOUBLE PRECISION CHECK (location_longitude >= -180 AND location_longitude <= 180),

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint to prevent duplicate entries per time
    UNIQUE (user_id, recorded_at)
);

-- ============================================================================
-- STEP 2: Create indexes for audio_exposure_metrics if they don't exist
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_audio_exposure_user_time_brin ON audio_exposure_metrics USING BRIN (user_id, recorded_at);
CREATE INDEX IF NOT EXISTS idx_audio_exposure_user_date ON audio_exposure_metrics (user_id, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_audio_exposure_dangerous ON audio_exposure_metrics (user_id, recorded_at) WHERE audio_exposure_event = true;
CREATE INDEX IF NOT EXISTS idx_audio_exposure_levels ON audio_exposure_metrics (environmental_audio_exposure_db, headphone_audio_exposure_db, recorded_at) WHERE environmental_audio_exposure_db IS NOT NULL OR headphone_audio_exposure_db IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audio_exposure_location ON audio_exposure_metrics (location_latitude, location_longitude) WHERE location_latitude IS NOT NULL AND location_longitude IS NOT NULL;

-- ============================================================================
-- STEP 3: Migrate existing audio exposure data from environmental_metrics
-- ============================================================================

-- Insert audio exposure data that exists in environmental_metrics
INSERT INTO audio_exposure_metrics (
    id,
    user_id,
    recorded_at,
    environmental_audio_exposure_db,
    headphone_audio_exposure_db,
    exposure_duration_minutes, -- Default to 0 since it wasn't stored before
    audio_exposure_event,      -- Calculate based on WHO standards
    source_device,
    created_at
)
SELECT
    uuid_generate_v4() as id,  -- Generate new UUIDs for audio exposure records
    user_id,
    recorded_at,
    environmental_audio_exposure_db,
    headphone_audio_exposure_db,
    0 as exposure_duration_minutes,  -- Default value since not previously tracked
    CASE
        WHEN environmental_audio_exposure_db > 85 OR headphone_audio_exposure_db > 85 THEN true
        ELSE false
    END as audio_exposure_event,     -- Flag dangerous levels (>85dB WHO threshold)
    source_device,
    created_at
FROM environmental_metrics
WHERE (environmental_audio_exposure_db IS NOT NULL OR headphone_audio_exposure_db IS NOT NULL)
ON CONFLICT (user_id, recorded_at) DO NOTHING;  -- Skip if already exists

-- ============================================================================
-- STEP 4: Log migration results
-- ============================================================================

-- Create a temporary function to log migration results
DO $$
DECLARE
    migrated_count INTEGER;
    environmental_with_audio INTEGER;
BEGIN
    -- Count records that were migrated
    SELECT COUNT(*) INTO migrated_count
    FROM audio_exposure_metrics;

    -- Count environmental records that had audio data
    SELECT COUNT(*) INTO environmental_with_audio
    FROM environmental_metrics
    WHERE environmental_audio_exposure_db IS NOT NULL
       OR headphone_audio_exposure_db IS NOT NULL;

    -- Log results
    RAISE NOTICE 'Audio Exposure Migration Complete:';
    RAISE NOTICE '  - Records in audio_exposure_metrics: %', migrated_count;
    RAISE NOTICE '  - Environmental records with audio data: %', environmental_with_audio;

    IF migrated_count > 0 THEN
        RAISE NOTICE '  - Migration Status: SUCCESS - % audio exposure records created', migrated_count;
    ELSE
        RAISE NOTICE '  - Migration Status: NO DATA - No audio exposure data found to migrate';
    END IF;
END $$;

-- ============================================================================
-- STEP 5: Remove audio exposure columns from environmental_metrics
-- Note: This step is commented out for safety. Uncomment after verifying migration success.
-- ============================================================================

-- WARNING: Only run this after confirming successful migration and updating application code
/*
ALTER TABLE environmental_metrics
DROP COLUMN IF EXISTS environmental_audio_exposure_db,
DROP COLUMN IF EXISTS headphone_audio_exposure_db;

RAISE NOTICE 'Audio exposure columns removed from environmental_metrics table';
*/

-- ============================================================================
-- STEP 6: Verification queries (for manual testing)
-- ============================================================================

-- Verify migration success
-- SELECT
--     'environmental_metrics' as table_name,
--     COUNT(*) as total_records,
--     COUNT(environmental_audio_exposure_db) as env_audio_records,
--     COUNT(headphone_audio_exposure_db) as headphone_audio_records
-- FROM environmental_metrics
-- UNION ALL
-- SELECT
--     'audio_exposure_metrics' as table_name,
--     COUNT(*) as total_records,
--     COUNT(environmental_audio_exposure_db) as env_audio_records,
--     COUNT(headphone_audio_exposure_db) as headphone_audio_records
-- FROM audio_exposure_metrics;

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================