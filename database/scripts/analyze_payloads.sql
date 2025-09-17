-- Payload Analysis SQL Script
-- This script analyzes raw_ingestions payloads and cross-checks against database entries

\echo '=== PAYLOAD ANALYSIS REPORT ==='
\echo

-- 1. Basic Summary
\echo '=== BASIC SUMMARY ==='
SELECT
    COUNT(*) as total_ingestions,
    COUNT(DISTINCT payload_hash) as unique_payloads,
    COUNT(*) - COUNT(DISTINCT payload_hash) as duplicate_ingestions
FROM raw_ingestions;

\echo

-- 2. Processing Status Summary
\echo '=== PROCESSING STATUS ==='
SELECT
    processing_status,
    COUNT(*) as count
FROM raw_ingestions
GROUP BY processing_status;

\echo

-- 3. Duplicated Payloads
\echo '=== DUPLICATE PAYLOADS ==='
SELECT
    payload_hash,
    COUNT(*) as occurrences,
    string_agg(id::text, ', ') as ingestion_ids,
    min(created_at) as first_seen,
    max(created_at) as last_seen
FROM raw_ingestions
GROUP BY payload_hash
HAVING COUNT(*) > 1
ORDER BY COUNT(*) DESC;

\echo

-- 4. Sample payload structure analysis (first few characters)
\echo '=== PAYLOAD STRUCTURE SAMPLES ==='
SELECT
    id,
    payload_hash,
    LEFT(raw_payload::text, 200) as payload_preview,
    created_at
FROM raw_ingestions
ORDER BY created_at
LIMIT 5;

\echo

-- 5. Get distinct user IDs to check
\echo '=== USER IDS IN PAYLOADS ==='
SELECT DISTINCT user_id FROM raw_ingestions;

\echo

-- 6. Check what tables we have for metrics
\echo '=== AVAILABLE METRIC TABLES ==='
SELECT table_name
FROM information_schema.tables
WHERE table_schema = 'public'
AND table_name LIKE '%metric%'
OR table_name LIKE '%measurement%'
ORDER BY table_name;

\echo

-- 7. Count records in each metric table for the user
\echo '=== RECORD COUNTS IN METRIC TABLES ==='
SELECT 'heart_rate_metrics' as table_name, COUNT(*) as record_count FROM heart_rate_metrics
UNION ALL
SELECT 'blood_pressure_metrics', COUNT(*) FROM blood_pressure_metrics
UNION ALL
SELECT 'sleep_metrics', COUNT(*) FROM sleep_metrics
UNION ALL
SELECT 'activity_metrics', COUNT(*) FROM activity_metrics
UNION ALL
SELECT 'workout_metrics', COUNT(*) FROM workout_metrics
UNION ALL
SELECT 'body_measurements', COUNT(*) FROM body_measurements
UNION ALL
SELECT 'environmental_metrics', COUNT(*) FROM environmental_metrics;

\echo

-- 8. Check for any processing errors
\echo '=== PROCESSING ERRORS ==='
SELECT
    id,
    processing_errors
FROM raw_ingestions
WHERE processing_errors IS NOT NULL AND processing_errors != ''::jsonb;

\echo

-- 9. Time range analysis
\echo '=== TIME RANGE ANALYSIS ==='
SELECT
    MIN(created_at) as first_ingestion,
    MAX(created_at) as last_ingestion,
    COUNT(*) as total_ingestions
FROM raw_ingestions;

\echo

-- 10. Check user's metrics by date range
\echo '=== USER METRICS BY DATE RANGE ==='
SELECT
    'heart_rate_metrics' as table_name,
    MIN(recorded_at) as earliest_metric,
    MAX(recorded_at) as latest_metric,
    COUNT(*) as total_records
FROM heart_rate_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
UNION ALL
SELECT
    'activity_metrics',
    MIN(recorded_at),
    MAX(recorded_at),
    COUNT(*)
FROM activity_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
UNION ALL
SELECT
    'body_measurements',
    MIN(recorded_at),
    MAX(recorded_at),
    COUNT(*)
FROM body_measurements
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';

\echo '=== END OF ANALYSIS ==='