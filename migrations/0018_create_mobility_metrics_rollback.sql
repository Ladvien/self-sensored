-- Rollback migration for mobility_metrics table creation
-- Safely removes all mobility_metrics related objects in reverse order

-- Drop performance monitoring function
DROP FUNCTION IF EXISTS mobility_metrics_stats();

-- Drop partition management function  
DROP FUNCTION IF EXISTS create_mobility_metrics_partition(DATE);

-- Drop performance views
DROP VIEW IF EXISTS mobility_fall_risk_assessment;
DROP VIEW IF EXISTS mobility_gait_analysis;
DROP VIEW IF EXISTS mobility_daily_summary;

-- Drop all indexes (they will be automatically dropped with table, but explicit for clarity)
DROP INDEX IF EXISTS mobility_metrics_fall_risk_analysis_idx;
DROP INDEX IF EXISTS mobility_metrics_gait_analysis_idx;
DROP INDEX IF EXISTS mobility_metrics_raw_data_gin_idx;
DROP INDEX IF EXISTS mobility_metrics_surface_type_idx;
DROP INDEX IF EXISTS mobility_metrics_stair_speeds_idx;
DROP INDEX IF EXISTS mobility_metrics_six_minute_walk_idx;
DROP INDEX IF EXISTS mobility_metrics_walking_steadiness_idx;
DROP INDEX IF EXISTS mobility_metrics_walking_asymmetry_idx;
DROP INDEX IF EXISTS mobility_metrics_walking_speed_idx;
DROP INDEX IF EXISTS mobility_metrics_user_id_recorded_at_idx;
DROP INDEX IF EXISTS mobility_metrics_recorded_at_brin_idx;

-- Drop all partitions (they will be automatically dropped with parent table)
DROP TABLE IF EXISTS mobility_metrics_y2025m12;
DROP TABLE IF EXISTS mobility_metrics_y2025m11;
DROP TABLE IF EXISTS mobility_metrics_y2025m10;
DROP TABLE IF EXISTS mobility_metrics_y2025m09;

-- Drop main partitioned table
DROP TABLE IF EXISTS mobility_metrics;

-- Note: User table and references are preserved
-- Migration rollback completed successfully