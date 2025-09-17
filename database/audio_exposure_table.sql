-- Create audio_exposure_metrics table for audio exposure data
CREATE TABLE IF NOT EXISTS audio_exposure_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Audio exposure measurements
    environmental_audio_exposure_db DOUBLE PRECISION,
    headphone_audio_exposure_db DOUBLE PRECISION,
    exposure_duration_minutes INTEGER NOT NULL,
    audio_exposure_event BOOLEAN NOT NULL DEFAULT FALSE, -- true if dangerous level detected

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at)
);

-- Create index for performance
CREATE INDEX IF NOT EXISTS idx_audio_exposure_user_recorded ON audio_exposure_metrics(user_id, recorded_at DESC);