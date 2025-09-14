-- Create background job processing functions
-- Filename: 0016_background_job_functions.sql

-- Function to get the next job for processing (replaces missing get_next_job_for_processing)
CREATE OR REPLACE FUNCTION get_next_job_for_processing()
RETURNS TABLE(
    job_id UUID,
    user_id UUID,
    api_key_id UUID,
    raw_ingestion_id UUID,
    job_type TEXT,
    total_metrics INTEGER,
    config JSONB
) AS $$
DECLARE
    selected_job_id UUID;
BEGIN
    -- Atomically select and update the next pending job
    SELECT id INTO selected_job_id
    FROM processing_jobs
    WHERE status = 'pending'
    ORDER BY priority DESC, created_at ASC
    LIMIT 1
    FOR UPDATE SKIP LOCKED;
    
    -- If no job found, return empty result
    IF selected_job_id IS NULL THEN
        RETURN;
    END IF;
    
    -- Update job status to processing
    UPDATE processing_jobs
    SET status = 'processing',
        started_at = NOW()
    WHERE id = selected_job_id;
    
    -- Return job details
    RETURN QUERY
    SELECT 
        pj.id,
        pj.user_id,
        pj.api_key_id,
        pj.raw_ingestion_id,
        pj.job_type::TEXT,
        pj.total_metrics,
        pj.config
    FROM processing_jobs pj
    WHERE pj.id = selected_job_id;
END;
$$ LANGUAGE plpgsql;

-- Function to update job progress
CREATE OR REPLACE FUNCTION update_job_progress(
    p_job_id UUID,
    p_processed_metrics INTEGER,
    p_failed_metrics INTEGER,
    p_error_message TEXT DEFAULT NULL
)
RETURNS VOID AS $$
DECLARE
    p_total_metrics INTEGER;
    p_progress_percentage DECIMAL(5,2);
BEGIN
    -- Get total metrics for the job
    SELECT total_metrics INTO p_total_metrics
    FROM processing_jobs
    WHERE id = p_job_id;
    
    -- Calculate progress percentage
    IF p_total_metrics > 0 THEN
        p_progress_percentage := (p_processed_metrics::DECIMAL / p_total_metrics::DECIMAL) * 100.0;
    ELSE
        p_progress_percentage := 0.0;
    END IF;
    
    -- Ensure progress doesn't exceed 100%
    p_progress_percentage := LEAST(p_progress_percentage, 100.0);
    
    -- Update the job
    UPDATE processing_jobs
    SET 
        processed_metrics = p_processed_metrics,
        failed_metrics = p_failed_metrics,
        progress_percentage = p_progress_percentage,
        error_message = COALESCE(p_error_message, error_message)
    WHERE id = p_job_id;
    
    -- Update raw_ingestions status if we have significant progress
    IF p_progress_percentage >= 100.0 THEN
        UPDATE raw_ingestions
        SET processing_status = CASE 
            WHEN p_failed_metrics > 0 THEN 'partial_success'
            ELSE 'completed'
        END
        WHERE processing_job_id = p_job_id;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Function to complete a job
CREATE OR REPLACE FUNCTION complete_job(
    p_job_id UUID,
    p_status TEXT,
    p_result_summary JSONB DEFAULT NULL,
    p_error_message TEXT DEFAULT NULL
)
RETURNS VOID AS $$
BEGIN
    -- Update job status and completion time
    UPDATE processing_jobs
    SET 
        status = p_status::job_status,
        completed_at = NOW(),
        result_summary = p_result_summary,
        error_message = COALESCE(p_error_message, error_message),
        progress_percentage = CASE 
            WHEN p_status = 'completed' THEN 100.0
            ELSE progress_percentage
        END
    WHERE id = p_job_id;
    
    -- Update raw_ingestions status
    UPDATE raw_ingestions
    SET processing_status = CASE 
        WHEN p_status = 'completed' THEN 'completed'
        WHEN p_status = 'failed' THEN 'error'
        ELSE 'processing'
    END,
    processing_errors = CASE 
        WHEN p_error_message IS NOT NULL THEN 
            jsonb_build_object('error', p_error_message, 'completed_at', NOW())
        ELSE processing_errors
    END
    WHERE processing_job_id = p_job_id;
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old jobs (older than 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_jobs()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
BEGIN
    -- Delete old completed/failed jobs and their associated raw_ingestions
    WITH deleted_jobs AS (
        DELETE FROM processing_jobs 
        WHERE status IN ('completed', 'failed')
        AND completed_at < NOW() - INTERVAL '30 days'
        RETURNING id, raw_ingestion_id
    ),
    deleted_raw_ingestions AS (
        DELETE FROM raw_ingestions
        WHERE id IN (SELECT raw_ingestion_id FROM deleted_jobs)
        RETURNING 1
    )
    SELECT COUNT(*) INTO deleted_count FROM deleted_jobs;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Create indexes to support the background job functions
CREATE INDEX IF NOT EXISTS idx_processing_jobs_pending_priority 
ON processing_jobs (priority DESC, created_at ASC) 
WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_processing_jobs_old_completed
ON processing_jobs (completed_at)
WHERE status IN ('completed', 'failed');