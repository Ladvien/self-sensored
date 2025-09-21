-- Add extended activity metrics columns to existing activity_metrics table
-- These columns support comprehensive activity tracking beyond basic metrics

ALTER TABLE activity_metrics
ADD COLUMN IF NOT EXISTS walking_speed_m_per_s DOUBLE PRECISION CHECK (walking_speed_m_per_s IS NULL OR walking_speed_m_per_s >= 0.0),
ADD COLUMN IF NOT EXISTS walking_step_length_cm DOUBLE PRECISION CHECK (walking_step_length_cm IS NULL OR walking_step_length_cm >= 0.0),
ADD COLUMN IF NOT EXISTS walking_asymmetry_percent DOUBLE PRECISION CHECK (walking_asymmetry_percent IS NULL OR (walking_asymmetry_percent >= 0.0 AND walking_asymmetry_percent <= 100.0)),
ADD COLUMN IF NOT EXISTS walking_double_support_percent DOUBLE PRECISION CHECK (walking_double_support_percent IS NULL OR (walking_double_support_percent >= 0.0 AND walking_double_support_percent <= 100.0)),
ADD COLUMN IF NOT EXISTS six_minute_walk_test_distance_m DOUBLE PRECISION CHECK (six_minute_walk_test_distance_m IS NULL OR six_minute_walk_test_distance_m >= 0.0),
ADD COLUMN IF NOT EXISTS stair_ascent_speed_m_per_s DOUBLE PRECISION CHECK (stair_ascent_speed_m_per_s IS NULL OR stair_ascent_speed_m_per_s >= 0.0),
ADD COLUMN IF NOT EXISTS stair_descent_speed_m_per_s DOUBLE PRECISION CHECK (stair_descent_speed_m_per_s IS NULL OR stair_descent_speed_m_per_s >= 0.0),
ADD COLUMN IF NOT EXISTS ground_contact_time_ms DOUBLE PRECISION CHECK (ground_contact_time_ms IS NULL OR ground_contact_time_ms >= 0.0),
ADD COLUMN IF NOT EXISTS vertical_oscillation_cm DOUBLE PRECISION CHECK (vertical_oscillation_cm IS NULL OR vertical_oscillation_cm >= 0.0),
ADD COLUMN IF NOT EXISTS running_stride_length_m DOUBLE PRECISION CHECK (running_stride_length_m IS NULL OR running_stride_length_m >= 0.0),
ADD COLUMN IF NOT EXISTS running_power_watts DOUBLE PRECISION CHECK (running_power_watts IS NULL OR running_power_watts >= 0.0),
ADD COLUMN IF NOT EXISTS running_speed_m_per_s DOUBLE PRECISION CHECK (running_speed_m_per_s IS NULL OR running_speed_m_per_s >= 0.0),
ADD COLUMN IF NOT EXISTS cycling_speed_kmh DOUBLE PRECISION CHECK (cycling_speed_kmh IS NULL OR cycling_speed_kmh >= 0.0),
ADD COLUMN IF NOT EXISTS cycling_power_watts DOUBLE PRECISION CHECK (cycling_power_watts IS NULL OR cycling_power_watts >= 0.0),
ADD COLUMN IF NOT EXISTS cycling_cadence_rpm DOUBLE PRECISION CHECK (cycling_cadence_rpm IS NULL OR cycling_cadence_rpm >= 0.0),
ADD COLUMN IF NOT EXISTS functional_threshold_power_watts DOUBLE PRECISION CHECK (functional_threshold_power_watts IS NULL OR functional_threshold_power_watts >= 0.0),
ADD COLUMN IF NOT EXISTS underwater_depth_meters DOUBLE PRECISION CHECK (underwater_depth_meters IS NULL OR (underwater_depth_meters >= 0.0 AND underwater_depth_meters <= 1000.0)),
ADD COLUMN IF NOT EXISTS diving_duration_seconds INTEGER CHECK (diving_duration_seconds IS NULL OR (diving_duration_seconds >= 0 AND diving_duration_seconds <= 86400));