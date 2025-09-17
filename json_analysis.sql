-- JSON-based payload analysis using PostgreSQL JSON functions

\echo '=== JSON-BASED PAYLOAD ANALYSIS ==='
\echo

-- Extract metric types and counts from each payload using JSON functions
WITH payload_metrics AS (
    SELECT
        id,
        payload_hash,
        created_at,
        jsonb_array_length(raw_payload->'data'->'metrics') as metric_count,
        jsonb_array_elements(raw_payload->'data'->'metrics')->>'type' as metric_type
    FROM raw_ingestions
    WHERE raw_payload->'data'->'metrics' IS NOT NULL
),
metric_summary AS (
    SELECT
        metric_type,
        COUNT(*) as payload_occurrences,
        SUM(1) as total_metrics,
        array_agg(DISTINCT payload_hash) as payload_hashes
    FROM payload_metrics
    GROUP BY metric_type
)
SELECT * FROM metric_summary ORDER BY metric_type;

\echo

-- Count records by payload and check database presence
\echo '=== PAYLOAD vs DATABASE CROSS-CHECK ==='
WITH payload_details AS (
    SELECT
        p.id,
        p.payload_hash,
        p.created_at,
        jsonb_array_length(p.raw_payload->'data'->'metrics') as metrics_in_payload,
        jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'type' as metric_type,
        jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'id' as metric_id,
        jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'recorded_at' as metric_recorded_at
    FROM raw_ingestions p
    WHERE p.raw_payload->'data'->'metrics' IS NOT NULL
)
SELECT
    metric_type,
    COUNT(*) as metrics_in_payloads,
    CASE metric_type
        WHEN 'BodyMeasurement' THEN (SELECT COUNT(*) FROM body_measurements WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18')
        WHEN 'HeartRate' THEN (SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18')
        WHEN 'Activity' THEN (SELECT COUNT(*) FROM activity_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18')
        WHEN 'AudioExposure' THEN (SELECT COUNT(*) FROM environmental_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18')
        ELSE 0
    END as records_in_database,
    CASE metric_type
        WHEN 'BodyMeasurement' THEN 'body_measurements'
        WHEN 'HeartRate' THEN 'heart_rate_metrics'
        WHEN 'Activity' THEN 'activity_metrics'
        WHEN 'AudioExposure' THEN 'environmental_metrics'
        ELSE 'NO_TABLE_MAPPING'
    END as target_table
FROM payload_details
GROUP BY metric_type
ORDER BY metric_type;

\echo

-- Detailed breakdown by payload hash
\echo '=== DETAILED PAYLOAD BREAKDOWN ==='
WITH payload_analysis AS (
    SELECT
        p.payload_hash,
        p.created_at,
        jsonb_array_length(p.raw_payload->'data'->'metrics') as total_metrics,
        string_agg(DISTINCT (jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'type'), ', ') as metric_types
    FROM raw_ingestions p
    WHERE p.raw_payload->'data'->'metrics' IS NOT NULL
    GROUP BY p.payload_hash, p.created_at, p.raw_payload
)
SELECT
    left(payload_hash, 16) || '...' as payload_hash_short,
    created_at,
    total_metrics,
    metric_types
FROM payload_analysis
ORDER BY created_at;

\echo

-- Check for specific missing data
\echo '=== MISSING DATA INVESTIGATION ==='

-- Check BodyMeasurement metrics that should be in body_measurements table
WITH body_measurement_payloads AS (
    SELECT
        p.payload_hash,
        p.created_at as ingestion_time,
        jsonb_array_elements(p.raw_payload->'data'->'metrics') as metric,
        (jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'recorded_at')::timestamp as recorded_at
    FROM raw_ingestions p
    WHERE jsonb_array_elements(p.raw_payload->'data'->'metrics')->>'type' = 'BodyMeasurement'
)
SELECT
    'BodyMeasurement' as metric_type,
    COUNT(*) as payload_metrics,
    COUNT(bm.id) as found_in_db,
    COUNT(*) - COUNT(bm.id) as missing_from_db
FROM body_measurement_payloads bmp
LEFT JOIN body_measurements bm ON (
    bm.user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
    AND bm.recorded_at = bmp.recorded_at
);

\echo

-- Check specific payload that was duplicated multiple times
\echo '=== DUPLICATE PAYLOAD ANALYSIS ==='
SELECT
    p.id,
    p.created_at,
    jsonb_array_length(p.raw_payload->'data'->'metrics') as metric_count,
    left(p.raw_payload::text, 200) as preview
FROM raw_ingestions p
WHERE p.payload_hash = 'a59cecaf5372523cdf3c914718417e3bbc9499425f30847aa5f164ecf46b81e7'
ORDER BY p.created_at;

\echo '=== END JSON ANALYSIS ==='