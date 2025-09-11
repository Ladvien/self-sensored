-- Rollback for mental_health_metrics Table Migration
-- Safely removes mental_health_metrics table and all associated objects

-- Drop performance monitoring function
DROP FUNCTION IF EXISTS mental_health_metrics_stats();

-- Drop partition management function
DROP FUNCTION IF EXISTS create_mental_health_metrics_partition(DATE);

-- Drop views (must be done before dropping the table)
DROP VIEW IF EXISTS mental_health_mood_trends;
DROP VIEW IF EXISTS mental_health_daily_summary;

-- Drop all partition tables first
DROP TABLE IF EXISTS mental_health_metrics_y2025m09;
DROP TABLE IF EXISTS mental_health_metrics_y2025m10;
DROP TABLE IF EXISTS mental_health_metrics_y2025m11;
DROP TABLE IF EXISTS mental_health_metrics_y2025m12;

-- Drop the main partitioned table
-- This will also drop all indexes automatically
DROP TABLE IF EXISTS mental_health_metrics;