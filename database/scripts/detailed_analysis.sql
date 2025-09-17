-- Detailed Analysis of Individual Payloads
-- This will help us understand what's in each payload and cross-check with database

\echo '=== DETAILED PAYLOAD ANALYSIS ==='
\echo

-- Get one instance of each unique payload hash with full JSON
CREATE TEMP TABLE unique_payloads AS
SELECT DISTINCT ON (payload_hash)
    id,
    user_id,
    payload_hash,
    raw_payload,
    created_at
FROM raw_ingestions
ORDER BY payload_hash, created_at;

\echo 'Created temporary table with unique payloads'
\echo

-- Analyze payload contents for AudioExposure metrics
\echo '=== AUDIOEXPOSURE METRICS ANALYSIS ==='
SELECT
    'AudioExposure Payloads' as metric_type,
    COUNT(*) as payload_count
FROM unique_payloads
WHERE raw_payload::text LIKE '%"type":"AudioExposure"%';

-- Check if we have environmental_metrics table data for AudioExposure
SELECT
    'environmental_metrics table' as location,
    COUNT(*) as record_count
FROM environmental_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';

\echo

-- Analyze BodyMeasurement metrics
\echo '=== BODYMEASUREMENT METRICS ANALYSIS ==='
SELECT
    'BodyMeasurement Payloads' as metric_type,
    COUNT(*) as payload_count
FROM unique_payloads
WHERE raw_payload::text LIKE '%"type":"BodyMeasurement"%';

-- Check body_measurements table
SELECT
    'body_measurements table' as location,
    COUNT(*) as record_count
FROM body_measurements
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';

\echo

-- Analyze HeartRate metrics
\echo '=== HEARTRATE METRICS ANALYSIS ==='
SELECT
    'HeartRate Payloads' as metric_type,
    COUNT(*) as payload_count
FROM unique_payloads
WHERE raw_payload::text LIKE '%"type":"HeartRate"%';

-- Check heart_rate_metrics table
SELECT
    'heart_rate_metrics table' as location,
    COUNT(*) as record_count
FROM heart_rate_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';

\echo

-- Analyze Activity metrics
\echo '=== ACTIVITY METRICS ANALYSIS ==='
SELECT
    'Activity Payloads' as metric_type,
    COUNT(*) as payload_count
FROM unique_payloads
WHERE raw_payload::text LIKE '%"type":"Activity"%';

-- Check activity_metrics table
SELECT
    'activity_metrics table' as location,
    COUNT(*) as record_count
FROM activity_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';

\echo

-- Check for unknown metric types
\echo '=== UNKNOWN OR UNMAPPED METRICS ==='
SELECT
    payload_hash,
    CASE
        WHEN raw_payload::text LIKE '%"type":"AudioExposure"%' THEN 'AudioExposure (-> environmental_metrics)'
        WHEN raw_payload::text LIKE '%"type":"BodyMeasurement"%' THEN 'BodyMeasurement (-> body_measurements)'
        WHEN raw_payload::text LIKE '%"type":"HeartRate"%' THEN 'HeartRate (-> heart_rate_metrics)'
        WHEN raw_payload::text LIKE '%"type":"Activity"%' THEN 'Activity (-> activity_metrics)'
        ELSE 'UNKNOWN TYPE'
    END as detected_type
FROM unique_payloads
ORDER BY detected_type;

\echo

-- Check for any payloads that might have multiple metric types
\echo '=== MULTI-TYPE PAYLOAD ANALYSIS ==='
SELECT
    payload_hash,
    CASE WHEN raw_payload::text LIKE '%"type":"AudioExposure"%' THEN 1 ELSE 0 END +
    CASE WHEN raw_payload::text LIKE '%"type":"BodyMeasurement"%' THEN 1 ELSE 0 END +
    CASE WHEN raw_payload::text LIKE '%"type":"HeartRate"%' THEN 1 ELSE 0 END +
    CASE WHEN raw_payload::text LIKE '%"type":"Activity"%' THEN 1 ELSE 0 END as type_count,
    length(raw_payload::text) as payload_size
FROM unique_payloads
WHERE
    (CASE WHEN raw_payload::text LIKE '%"type":"AudioExposure"%' THEN 1 ELSE 0 END +
     CASE WHEN raw_payload::text LIKE '%"type":"BodyMeasurement"%' THEN 1 ELSE 0 END +
     CASE WHEN raw_payload::text LIKE '%"type":"HeartRate"%' THEN 1 ELSE 0 END +
     CASE WHEN raw_payload::text LIKE '%"type":"Activity"%' THEN 1 ELSE 0 END) > 1
ORDER BY type_count DESC;

\echo

-- Check the empty payload
\echo '=== EMPTY OR PROBLEMATIC PAYLOADS ==='
SELECT
    payload_hash,
    length(raw_payload::text) as payload_size,
    LEFT(raw_payload::text, 100) as preview
FROM unique_payloads
WHERE raw_payload::text NOT LIKE '%"type":"%'
   OR length(raw_payload::text) < 50;

\echo

-- Time-based analysis - check if metrics around ingestion time exist
\echo '=== TIME-BASED CROSS-CHECK ==='
\echo 'Checking if metrics exist within 2 hours of payload ingestion times'

SELECT
    'Around AudioExposure ingestions' as check_type,
    COUNT(*) as found_metrics
FROM environmental_metrics e
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
AND EXISTS (
    SELECT 1 FROM unique_payloads up
    WHERE up.raw_payload::text LIKE '%"type":"AudioExposure"%'
    AND e.recorded_at BETWEEN (up.created_at - interval '2 hours') AND (up.created_at + interval '2 hours')
);

SELECT
    'Around BodyMeasurement ingestions' as check_type,
    COUNT(*) as found_metrics
FROM body_measurements b
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
AND EXISTS (
    SELECT 1 FROM unique_payloads up
    WHERE up.raw_payload::text LIKE '%"type":"BodyMeasurement"%'
    AND b.recorded_at BETWEEN (up.created_at - interval '2 hours') AND (up.created_at + interval '2 hours')
);

\echo '=== END DETAILED ANALYSIS ==='