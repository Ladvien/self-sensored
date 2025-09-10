-- Background Processing Migration
-- Adds support for async/background processing of large payloads to prevent Cloudflare 524 timeouts

-- Processing jobs table for tracking background operations
CREATE TABLE processing_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE SET NULL,
    raw_ingestion_id UUID NOT NULL REFERENCES raw_ingestions(id) ON DELETE CASCADE,
    status VARCHAR(20) DEFAULT 'pending', -- pending, processing, completed, failed
    job_type VARCHAR(50) DEFAULT 'ingest_batch', -- ingest_batch, data_export, etc.
    priority INTEGER DEFAULT 1, -- 1=low, 5=normal, 10=high
    
    -- Progress tracking
    total_metrics INTEGER DEFAULT 0,
    processed_metrics INTEGER DEFAULT 0,
    failed_metrics INTEGER DEFAULT 0,
    progress_percentage DECIMAL(5,2) DEFAULT 0.0 CHECK (progress_percentage >= 0 AND progress_percentage <= 100),
    
    -- Timing information
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    estimated_completion_at TIMESTAMPTZ,
    
    -- Error handling
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    last_retry_at TIMESTAMPTZ,
    
    -- Configuration
    config JSONB DEFAULT '{}',
    
    -- Results summary
    result_summary JSONB
);

-- Create indexes for efficient job processing
CREATE INDEX idx_processing_jobs_status ON processing_jobs(status);
CREATE INDEX idx_processing_jobs_user_id ON processing_jobs(user_id);
CREATE INDEX idx_processing_jobs_created_at ON processing_jobs(created_at);
CREATE INDEX idx_processing_jobs_priority_created ON processing_jobs(priority DESC, created_at ASC);
CREATE INDEX idx_processing_jobs_status_priority ON processing_jobs(status, priority DESC, created_at ASC);

-- Update raw_ingestions table to support background processing
ALTER TABLE raw_ingestions ADD COLUMN IF NOT EXISTS processing_job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL;
ALTER TABLE raw_ingestions ADD COLUMN IF NOT EXISTS received_at TIMESTAMPTZ DEFAULT NOW();

-- Create index for new columns
CREATE INDEX IF NOT EXISTS idx_raw_ingestions_processing_job_id ON raw_ingestions(processing_job_id);
CREATE INDEX IF NOT EXISTS idx_raw_ingestions_received_at ON raw_ingestions(received_at);

-- Function to update job progress
CREATE OR REPLACE FUNCTION update_job_progress(
    p_job_id UUID,
    p_processed_metrics INTEGER,
    p_failed_metrics INTEGER DEFAULT 0,
    p_error_message TEXT DEFAULT NULL
) RETURNS VOID AS $$
DECLARE
    total_count INTEGER;
    new_progress DECIMAL(5,2);
BEGIN
    -- Get total metrics count
    SELECT total_metrics INTO total_count 
    FROM processing_jobs 
    WHERE id = p_job_id;
    
    -- Calculate progress percentage
    IF total_count > 0 THEN
        new_progress := (p_processed_metrics::DECIMAL / total_count::DECIMAL) * 100.0;
    ELSE
        new_progress := 0.0;
    END IF;
    
    -- Update job progress
    UPDATE processing_jobs 
    SET 
        processed_metrics = p_processed_metrics,
        failed_metrics = p_failed_metrics,
        progress_percentage = new_progress,
        error_message = COALESCE(p_error_message, error_message),
        estimated_completion_at = CASE 
            WHEN new_progress > 0 AND new_progress < 100 THEN
                NOW() + (NOW() - started_at) * ((100.0 - new_progress) / new_progress) * INTERVAL '1 second'
            ELSE estimated_completion_at
        END
    WHERE id = p_job_id;
END;
$$ LANGUAGE plpgsql;

-- Function to complete a job
CREATE OR REPLACE FUNCTION complete_job(
    p_job_id UUID,
    p_status VARCHAR(20),
    p_result_summary JSONB DEFAULT NULL,
    p_error_message TEXT DEFAULT NULL
) RETURNS VOID AS $$
BEGIN
    UPDATE processing_jobs 
    SET 
        status = p_status,
        completed_at = NOW(),
        result_summary = COALESCE(p_result_summary, result_summary),
        error_message = COALESCE(p_error_message, error_message)
    WHERE id = p_job_id;
    
    -- Update corresponding raw_ingestions record
    UPDATE raw_ingestions 
    SET 
        status = CASE 
            WHEN p_status = 'completed' THEN 'processed'
            WHEN p_status = 'failed' THEN 'error'
            ELSE p_status
        END,
        processed_at = NOW(),
        error_message = p_error_message
    WHERE processing_job_id = p_job_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get next job for processing (simple FIFO with priority)
CREATE OR REPLACE FUNCTION get_next_job_for_processing()
RETURNS TABLE(
    job_id UUID,
    user_id UUID,
    api_key_id UUID,
    raw_ingestion_id UUID,
    job_type VARCHAR(50),
    total_metrics INTEGER,
    config JSONB
) AS $$
DECLARE
    selected_job_id UUID;
BEGIN
    -- Select and lock the next job atomically
    SELECT id INTO selected_job_id
    FROM processing_jobs 
    WHERE status = 'pending'
    ORDER BY priority DESC, created_at ASC
    LIMIT 1
    FOR UPDATE SKIP LOCKED;
    
    -- If we found a job, mark it as processing and return it
    IF selected_job_id IS NOT NULL THEN
        UPDATE processing_jobs 
        SET status = 'processing', started_at = NOW()
        WHERE id = selected_job_id;
        
        RETURN QUERY
        SELECT 
            pj.id,
            pj.user_id,
            pj.api_key_id, 
            pj.raw_ingestion_id,
            pj.job_type,
            pj.total_metrics,
            pj.config
        FROM processing_jobs pj
        WHERE pj.id = selected_job_id;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Add job cleanup function for completed/failed jobs older than 7 days
CREATE OR REPLACE FUNCTION cleanup_old_jobs() RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM processing_jobs 
    WHERE status IN ('completed', 'failed') 
    AND completed_at < NOW() - INTERVAL '7 days';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Add unique constraint to prevent duplicate job creation for same raw ingestion
ALTER TABLE processing_jobs ADD CONSTRAINT processing_jobs_raw_ingestion_unique UNIQUE(raw_ingestion_id);

-- Comment the tables for documentation
COMMENT ON TABLE processing_jobs IS 'Background job queue for processing large health data payloads asynchronously';
COMMENT ON COLUMN processing_jobs.status IS 'Job status: pending, processing, completed, failed';
COMMENT ON COLUMN processing_jobs.priority IS 'Job priority: 1=low, 5=normal, 10=high';
COMMENT ON COLUMN processing_jobs.progress_percentage IS 'Current progress percentage (0-100)';
COMMENT ON COLUMN processing_jobs.config IS 'Job-specific configuration as JSON';
COMMENT ON COLUMN processing_jobs.result_summary IS 'Summary of job results including metrics processed, errors, etc.';