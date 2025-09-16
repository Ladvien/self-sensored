-- Reprocess Failed Raw Ingestions
-- This script resets failed raw_ingestions to pending status for reprocessing
-- with the corrected activity chunk size (2,700 instead of 6,500)

BEGIN;

-- Show current status before reprocessing
SELECT
    processing_status,
    COUNT(*) as count,
    ROUND(AVG(payload_size_bytes::numeric)/1024/1024, 2) as avg_size_mb,
    ROUND(SUM(payload_size_bytes::numeric)/1024/1024, 2) as total_size_mb
FROM raw_ingestions
GROUP BY processing_status
ORDER BY processing_status;

-- List the specific errors we're addressing
SELECT DISTINCT
    processing_errors->0->>'error_message' as error_type,
    COUNT(*) as occurrence_count
FROM raw_ingestions
WHERE processing_status = 'error'
  AND processing_errors IS NOT NULL
GROUP BY processing_errors->0->>'error_message'
ORDER BY occurrence_count DESC;

-- Reset failed ingestions to pending for reprocessing
-- Only reset those with PostgreSQL parameter limit errors
UPDATE raw_ingestions
SET
    processing_status = 'pending',
    processing_errors = NULL,
    processed_at = NULL
WHERE processing_status = 'error'
  AND (
    processing_errors::text ILIKE '%123500 parameters%' OR
    processing_errors::text ILIKE '%too many arguments for query%' OR
    processing_errors::text ILIKE '%exceeding safe limit%'
  );

-- Show what we updated
GET DIAGNOSTICS affected_rows = ROW_COUNT;
SELECT affected_rows as "Records reset for reprocessing";

-- Show updated status
SELECT
    processing_status,
    COUNT(*) as count,
    ROUND(AVG(payload_size_bytes::numeric)/1024/1024, 2) as avg_size_mb,
    ROUND(SUM(payload_size_bytes::numeric)/1024/1024, 2) as total_size_mb
FROM raw_ingestions
GROUP BY processing_status
ORDER BY processing_status;

COMMIT;