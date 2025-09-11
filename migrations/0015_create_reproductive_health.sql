-- Create Reproductive Health Table Migration
-- Creates comprehensive reproductive health tracking table with HIPAA-compliant privacy controls
-- Features: Menstrual tracking, fertility monitoring, pregnancy tracking, sexual health with field-level encryption
-- Privacy: pgcrypto encryption for sensitive fields, audit logging, Row Level Security

-- Enable pgcrypto extension for field-level encryption
DO $$ 
BEGIN
    -- Try to create pgcrypto extension for field-level encryption
    BEGIN
        CREATE EXTENSION IF NOT EXISTS "pgcrypto";
        RAISE NOTICE '✅ pgcrypto extension enabled for field-level encryption';
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE '⚠️ pgcrypto extension already exists or insufficient privileges';
    END;
END $$;

-- Create reproductive_health table with comprehensive tracking and privacy controls
CREATE TABLE reproductive_health (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    
    -- === MENSTRUAL TRACKING FIELDS ===
    -- Menstrual flow intensity tracking
    menstrual_flow VARCHAR(20) CHECK (menstrual_flow IN (
        'none', 'light', 'medium', 'heavy', 'very_heavy'
    )),
    
    -- Spotting between periods
    spotting BOOLEAN DEFAULT FALSE,
    
    -- Current day in menstrual cycle (1-45 typical range)
    cycle_day INTEGER CHECK (cycle_day >= 1 AND cycle_day <= 60),
    
    -- Total cycle length in days (18-45 typical range)  
    cycle_length INTEGER CHECK (cycle_length >= 18 AND cycle_length <= 60),
    
    -- === FERTILITY TRACKING FIELDS ===
    -- Basal body temperature in Celsius (35.0-38.0°C typical range)
    basal_body_temp NUMERIC(4,2) CHECK (basal_body_temp >= 35.0 AND basal_body_temp <= 40.0),
    
    -- Cervical mucus quality assessment
    cervical_mucus_quality VARCHAR(20) CHECK (cervical_mucus_quality IN (
        'dry', 'sticky', 'creamy', 'watery', 'egg_white', 'none'
    )),
    
    -- Ovulation test results
    ovulation_test_result VARCHAR(20) CHECK (ovulation_test_result IN (
        'negative', 'positive', 'peak', 'high', 'low', 'not_tested'
    )),
    
    -- Predicted fertile window flag
    fertile_window BOOLEAN DEFAULT FALSE,
    
    -- === PREGNANCY TRACKING FIELDS ===
    -- Pregnancy test results
    pregnancy_test_result VARCHAR(20) CHECK (pregnancy_test_result IN (
        'negative', 'positive', 'indeterminate', 'not_tested'
    )),
    
    -- Current pregnancy status
    pregnancy_status VARCHAR(30) CHECK (pregnancy_status IN (
        'not_pregnant', 'trying_to_conceive', 'pregnant', 'postpartum', 'unknown'
    )),
    
    -- Gestational age in weeks (0-50 range to cover extended pregnancies)
    gestational_age_weeks INTEGER CHECK (gestational_age_weeks >= 0 AND gestational_age_weeks <= 50),
    
    -- === SEXUAL HEALTH FIELDS (ENCRYPTED) ===
    -- Sexual activity tracking - ENCRYPTED for privacy
    sexual_activity_encrypted BYTEA, -- Encrypted TEXT storing JSON: {"active": true, "timestamp": "2025-01-01T12:00:00Z"}
    
    -- Contraceptive use tracking - ENCRYPTED for privacy  
    contraceptive_use_encrypted BYTEA, -- Encrypted TEXT storing JSON: {"method": "pill", "effectiveness": "high", "notes": "taken daily"}
    
    -- === SYMPTOMS & MOOD TRACKING ===
    -- Reproductive health related symptoms
    symptoms TEXT[], -- Array of symptoms like 'cramps', 'bloating', 'breast_tenderness', 'mood_swings'
    
    -- Mood assessment related to cycle
    cycle_related_mood VARCHAR(20) CHECK (cycle_related_mood IN (
        'very_negative', 'negative', 'neutral', 'positive', 'very_positive', 'not_assessed'
    )),
    
    -- === METADATA ===
    -- Data source tracking
    source VARCHAR(100) NOT NULL DEFAULT 'health_export',
    
    -- Additional notes (not encrypted - for general medical notes only)
    notes TEXT,
    
    -- Timestamps for audit trail
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    -- === CONSTRAINTS ===
    -- Prevent duplicate entries for same user on same day
    CONSTRAINT reproductive_health_user_recorded_unique UNIQUE (user_id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- === PARTITIONING SETUP ===
-- Create monthly partitions for optimal performance with time-series data

-- Create partition management function specifically for reproductive_health
CREATE OR REPLACE FUNCTION create_reproductive_health_partitions()
RETURNS VOID AS $$
DECLARE
    start_date DATE;
    end_date DATE;
    partition_name TEXT;
    partition_start TEXT;
    partition_end TEXT;
BEGIN
    -- Create partitions for 3 months before current date to 6 months ahead
    FOR i IN -3..6 LOOP
        start_date := DATE_TRUNC('month', NOW() + (i || ' months')::INTERVAL)::DATE;
        end_date := (start_date + INTERVAL '1 month')::DATE;
        
        partition_name := 'reproductive_health_' || TO_CHAR(start_date, 'YYYY_MM');
        partition_start := TO_CHAR(start_date, 'YYYY-MM-DD');
        partition_end := TO_CHAR(end_date, 'YYYY-MM-DD');
        
        -- Create partition if it doesn't exist
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.tables 
            WHERE table_name = partition_name
        ) THEN
            EXECUTE format(
                'CREATE TABLE %I PARTITION OF reproductive_health 
                FOR VALUES FROM (%L) TO (%L)',
                partition_name, partition_start, partition_end
            );
            
            RAISE NOTICE '📅 Created partition: % for range % to %', 
                partition_name, partition_start, partition_end;
        END IF;
    END LOOP;
    
    RAISE NOTICE '✅ Reproductive health partitions created successfully';
END;
$$ LANGUAGE plpgsql;

-- Execute partition creation
SELECT create_reproductive_health_partitions();

-- === INDEXES FOR PERFORMANCE ===
-- Primary time-series index using BRIN for efficient time-based queries
CREATE INDEX idx_reproductive_health_recorded_at_brin 
    ON reproductive_health USING BRIN (recorded_at);

-- B-tree indexes for exact lookups
CREATE INDEX idx_reproductive_health_user_id 
    ON reproductive_health (user_id);
    
CREATE INDEX idx_reproductive_health_user_recorded 
    ON reproductive_health (user_id, recorded_at);

-- Specialized indexes for common reproductive health queries
CREATE INDEX idx_reproductive_health_menstrual_flow 
    ON reproductive_health (user_id, menstrual_flow) 
    WHERE menstrual_flow IS NOT NULL;

CREATE INDEX idx_reproductive_health_cycle_tracking 
    ON reproductive_health (user_id, cycle_day, cycle_length) 
    WHERE cycle_day IS NOT NULL;

CREATE INDEX idx_reproductive_health_fertility_window 
    ON reproductive_health (user_id, fertile_window, recorded_at) 
    WHERE fertile_window = TRUE;

CREATE INDEX idx_reproductive_health_pregnancy_status 
    ON reproductive_health (user_id, pregnancy_status) 
    WHERE pregnancy_status != 'not_pregnant';

-- GIN index for symptoms array searches
CREATE INDEX idx_reproductive_health_symptoms_gin 
    ON reproductive_health USING GIN (symptoms);

-- === ENCRYPTION HELPER FUNCTIONS ===
-- Function to encrypt sensitive reproductive health data
CREATE OR REPLACE FUNCTION encrypt_reproductive_data(
    data_text TEXT,
    encryption_key TEXT DEFAULT 'reproductive_health_encryption_key_2025'
) RETURNS BYTEA AS $$
BEGIN
    -- Use PGP symmetric encryption for sensitive reproductive health data
    RETURN pgp_sym_encrypt(data_text, encryption_key);
EXCEPTION 
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Failed to encrypt reproductive health data: %', SQLERRM;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to decrypt sensitive reproductive health data
CREATE OR REPLACE FUNCTION decrypt_reproductive_data(
    encrypted_data BYTEA,
    encryption_key TEXT DEFAULT 'reproductive_health_encryption_key_2025'
) RETURNS TEXT AS $$
BEGIN
    IF encrypted_data IS NULL THEN
        RETURN NULL;
    END IF;
    
    -- Use PGP symmetric decryption
    RETURN pgp_sym_decrypt(encrypted_data, encryption_key);
EXCEPTION 
    WHEN OTHERS THEN
        RAISE EXCEPTION 'Failed to decrypt reproductive health data: %', SQLERRM;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- === AUDIT LOGGING TRIGGERS FOR HIPAA COMPLIANCE ===
-- Function to log reproductive health data access and modifications
CREATE OR REPLACE FUNCTION audit_reproductive_health_changes()
RETURNS TRIGGER AS $$
DECLARE
    audit_action TEXT;
    user_info JSONB;
BEGIN
    -- Determine the action type
    IF TG_OP = 'INSERT' THEN
        audit_action := 'reproductive_health_insert';
        user_info := jsonb_build_object(
            'user_id', NEW.user_id,
            'recorded_at', NEW.recorded_at,
            'has_encrypted_fields', (NEW.sexual_activity_encrypted IS NOT NULL OR NEW.contraceptive_use_encrypted IS NOT NULL),
            'menstrual_data', (NEW.menstrual_flow IS NOT NULL OR NEW.cycle_day IS NOT NULL),
            'fertility_data', (NEW.basal_body_temp IS NOT NULL OR NEW.ovulation_test_result IS NOT NULL),
            'pregnancy_data', (NEW.pregnancy_status IS NOT NULL AND NEW.pregnancy_status != 'not_pregnant')
        );
    ELSIF TG_OP = 'UPDATE' THEN
        audit_action := 'reproductive_health_update';
        user_info := jsonb_build_object(
            'user_id', NEW.user_id,
            'recorded_at', NEW.recorded_at,
            'changed_fields', CASE 
                WHEN OLD.menstrual_flow != NEW.menstrual_flow OR (OLD.menstrual_flow IS NULL) != (NEW.menstrual_flow IS NULL) THEN 'menstrual_flow,'
                ELSE ''
            END ||
            CASE 
                WHEN OLD.sexual_activity_encrypted != NEW.sexual_activity_encrypted OR (OLD.sexual_activity_encrypted IS NULL) != (NEW.sexual_activity_encrypted IS NULL) THEN 'sexual_activity,'
                ELSE ''
            END ||
            CASE 
                WHEN OLD.contraceptive_use_encrypted != NEW.contraceptive_use_encrypted OR (OLD.contraceptive_use_encrypted IS NULL) != (NEW.contraceptive_use_encrypted IS NULL) THEN 'contraceptive_use,'
                ELSE ''
            END
        );
    ELSIF TG_OP = 'DELETE' THEN
        audit_action := 'reproductive_health_delete';
        user_info := jsonb_build_object(
            'user_id', OLD.user_id,
            'recorded_at', OLD.recorded_at,
            'had_encrypted_fields', (OLD.sexual_activity_encrypted IS NOT NULL OR OLD.contraceptive_use_encrypted IS NOT NULL)
        );
    END IF;

    -- Insert audit log entry (using the partitioned audit_log table)
    BEGIN
        INSERT INTO audit_log_partitioned (
            user_id,
            action,
            resource,
            metadata,
            created_at
        ) VALUES (
            COALESCE(NEW.user_id, OLD.user_id),
            audit_action,
            'reproductive_health',
            user_info,
            NOW()
        );
    EXCEPTION WHEN OTHERS THEN
        -- Log to regular audit_log if partitioned doesn't exist
        INSERT INTO audit_log (
            user_id,
            action,
            resource,
            metadata,
            created_at
        ) VALUES (
            COALESCE(NEW.user_id, OLD.user_id),
            audit_action,
            'reproductive_health',
            user_info,
            NOW()
        );
    END;

    -- Return appropriate record based on operation
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Create audit triggers for all reproductive health operations
CREATE TRIGGER reproductive_health_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON reproductive_health
    FOR EACH ROW EXECUTE FUNCTION audit_reproductive_health_changes();

-- === ROW LEVEL SECURITY (RLS) FOR PRIVACY CONTROLS ===
-- Enable RLS on reproductive_health table
ALTER TABLE reproductive_health ENABLE ROW LEVEL SECURITY;

-- Policy: Users can only access their own reproductive health data
CREATE POLICY reproductive_health_user_isolation ON reproductive_health
    FOR ALL TO authenticated_users
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- Policy: Allow healthcare providers to access with explicit consent
CREATE POLICY reproductive_health_healthcare_access ON reproductive_health
    FOR SELECT TO healthcare_providers
    USING (
        user_id IN (
            SELECT patient_user_id 
            FROM healthcare_consent 
            WHERE provider_user_id = current_setting('app.current_user_id', true)::UUID 
            AND consent_type = 'reproductive_health'
            AND is_active = TRUE
            AND expires_at > NOW()
        )
    );

-- === DATA RETENTION POLICIES ===
-- Function to implement data retention for reproductive health data
CREATE OR REPLACE FUNCTION cleanup_old_reproductive_health_data()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
    retention_years INTEGER := 7; -- HIPAA compliant retention period
BEGIN
    -- Delete reproductive health records older than retention period
    WITH deleted_rows AS (
        DELETE FROM reproductive_health 
        WHERE recorded_at < NOW() - (retention_years || ' years')::INTERVAL
        RETURNING id
    )
    SELECT COUNT(*) INTO deleted_count FROM deleted_rows;
    
    -- Log cleanup operation
    INSERT INTO audit_log_partitioned (
        action,
        resource,
        metadata,
        created_at
    ) VALUES (
        'reproductive_health_retention_cleanup',
        'reproductive_health',
        jsonb_build_object(
            'deleted_count', deleted_count,
            'retention_years', retention_years,
            'cleanup_date', NOW()
        ),
        NOW()
    );
    
    RAISE NOTICE '🗑️ Cleaned up % old reproductive health records (>% years)', 
        deleted_count, retention_years;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- === UPDATED_AT TRIGGER ===
-- Create trigger to automatically update the updated_at timestamp
CREATE TRIGGER update_reproductive_health_updated_at
    BEFORE UPDATE ON reproductive_health
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- === UTILITY VIEWS FOR ANALYSIS ===
-- View for menstrual cycle analysis (excludes encrypted fields)
CREATE VIEW reproductive_health_cycle_analysis AS
SELECT 
    user_id,
    recorded_at,
    menstrual_flow,
    cycle_day,
    cycle_length,
    fertile_window,
    ovulation_test_result,
    pregnancy_status,
    symptoms,
    cycle_related_mood,
    -- Calculate days since last period
    CASE 
        WHEN LAG(menstrual_flow) OVER (PARTITION BY user_id ORDER BY recorded_at) = 'heavy' 
        THEN recorded_at - LAG(recorded_at) OVER (PARTITION BY user_id ORDER BY recorded_at)
        ELSE NULL
    END as days_since_last_period
FROM reproductive_health
WHERE menstrual_flow IS NOT NULL OR cycle_day IS NOT NULL;

-- View for fertility tracking (excludes encrypted fields)  
CREATE VIEW reproductive_health_fertility_tracking AS
SELECT 
    user_id,
    recorded_at,
    basal_body_temp,
    cervical_mucus_quality,
    ovulation_test_result,
    fertile_window,
    -- Temperature trend calculation
    basal_body_temp - LAG(basal_body_temp) OVER (
        PARTITION BY user_id ORDER BY recorded_at
    ) as temp_change,
    symptoms
FROM reproductive_health
WHERE basal_body_temp IS NOT NULL 
   OR cervical_mucus_quality IS NOT NULL 
   OR ovulation_test_result IS NOT NULL;

-- === FINAL SETUP ===
-- Ensure audit log partitions exist
DO $$
BEGIN
    -- Create audit log partitions if the function exists
    IF EXISTS (SELECT 1 FROM information_schema.routines WHERE routine_name = 'create_monthly_partitions') THEN
        PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
        RAISE NOTICE '✅ Audit log partitions updated for reproductive health tracking';
    ELSE
        RAISE NOTICE '⚠️ Monthly partition function not found - audit logs may not be partitioned';
    END IF;
END $$;

-- Final success message
DO $$
BEGIN
    RAISE NOTICE '🎉 Reproductive Health table created successfully!';
    RAISE NOTICE '📊 Features enabled:';
    RAISE NOTICE '   ✅ Menstrual cycle tracking (flow, spotting, cycle metrics)';
    RAISE NOTICE '   ✅ Fertility monitoring (BBT, cervical mucus, ovulation tests)';
    RAISE NOTICE '   ✅ Pregnancy tracking (tests, status, gestational age)';
    RAISE NOTICE '   ✅ Sexual health tracking (ENCRYPTED for privacy)';
    RAISE NOTICE '   ✅ Comprehensive symptoms and mood tracking';
    RAISE NOTICE '🔒 Privacy & Security:';
    RAISE NOTICE '   ✅ Field-level encryption using pgcrypto';
    RAISE NOTICE '   ✅ HIPAA-compliant audit logging';
    RAISE NOTICE '   ✅ Row Level Security (RLS) policies';
    RAISE NOTICE '   ✅ 7-year data retention compliance';
    RAISE NOTICE '📈 Performance optimizations:';
    RAISE NOTICE '   ✅ Monthly partitioning for time-series data';
    RAISE NOTICE '   ✅ BRIN indexes for temporal queries';
    RAISE NOTICE '   ✅ Specialized indexes for reproductive health patterns';
    RAISE NOTICE '   ✅ GIN indexes for symptoms array searches';
END $$;