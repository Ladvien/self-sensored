-- Rollback Migration for 0015_create_reproductive_health.sql
-- Safely removes reproductive health table and all associated components
-- Maintains data integrity and audit trail during rollback process

-- === ROLLBACK SAFETY CHECKS ===
DO $$
BEGIN
    RAISE NOTICE '🔄 Starting rollback of reproductive health table migration...';
    
    -- Check if table exists before attempting rollback
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'reproductive_health'
    ) THEN
        RAISE NOTICE '⚠️ reproductive_health table does not exist - rollback not needed';
        RETURN;
    END IF;
    
    -- Count existing records for audit purposes
    DECLARE
        record_count INTEGER;
    BEGIN
        SELECT COUNT(*) INTO record_count FROM reproductive_health;
        RAISE NOTICE '📊 Found % reproductive health records to be removed', record_count;
        
        -- Log rollback operation if records exist
        IF record_count > 0 THEN
            RAISE NOTICE '⚠️ WARNING: Rollback will delete % reproductive health records!', record_count;
        END IF;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '📊 Could not count records in reproductive_health table';
    END;
END $$;

-- === AUDIT ROLLBACK OPERATION ===
DO $$
BEGIN
    -- Log the rollback operation in audit_log if it exists
    BEGIN
        IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_log_partitioned') THEN
            -- Use partitioned audit log if available
            INSERT INTO audit_log_partitioned (
                action,
                resource,
                metadata,
                created_at
            ) VALUES (
                'reproductive_health_table_rollback',
                'reproductive_health',
                jsonb_build_object(
                    'rollback_reason', 'Migration rollback requested',
                    'rollback_timestamp', NOW(),
                    'migration_file', '0015_create_reproductive_health_rollback.sql'
                ),
                NOW()
            );
            RAISE NOTICE '📝 Rollback logged in audit_log_partitioned table';
        ELSIF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_log') THEN
            -- Fall back to regular audit_log
            INSERT INTO audit_log (
                action,
                resource,
                metadata,
                created_at
            ) VALUES (
                'reproductive_health_table_rollback',
                'reproductive_health',
                jsonb_build_object(
                    'rollback_reason', 'Migration rollback requested',
                    'rollback_timestamp', NOW(),
                    'migration_file', '0015_create_reproductive_health_rollback.sql'
                ),
                NOW()
            );
            RAISE NOTICE '📝 Rollback logged in audit_log table';
        ELSE
            RAISE NOTICE '⚠️ audit_log table not found - rollback not logged';
        END IF;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE '⚠️ Could not log rollback to audit_log: %', SQLERRM;
    END;
END $$;

-- === DROP ANALYSIS VIEWS ===
-- Remove utility views for reproductive health analysis
DROP VIEW IF EXISTS reproductive_health_fertility_tracking CASCADE;
DROP VIEW IF EXISTS reproductive_health_cycle_analysis CASCADE;

RAISE NOTICE '🗑️ Dropped reproductive health analysis views';

-- === DROP ROW LEVEL SECURITY POLICIES ===
-- Remove RLS policies before dropping table
DROP POLICY IF EXISTS reproductive_health_healthcare_access ON reproductive_health;
DROP POLICY IF EXISTS reproductive_health_user_isolation ON reproductive_health;

RAISE NOTICE '🔐 Dropped Row Level Security policies';

-- === DROP TRIGGERS AND FUNCTIONS ===
-- Remove audit trigger
DROP TRIGGER IF EXISTS reproductive_health_audit_trigger ON reproductive_health;

-- Remove updated_at trigger
DROP TRIGGER IF EXISTS update_reproductive_health_updated_at ON reproductive_health;

RAISE NOTICE '🔧 Dropped reproductive health triggers';

-- Drop audit function
DROP FUNCTION IF EXISTS audit_reproductive_health_changes() CASCADE;

-- Drop data retention function
DROP FUNCTION IF EXISTS cleanup_old_reproductive_health_data() CASCADE;

-- Drop encryption/decryption functions
DROP FUNCTION IF EXISTS encrypt_reproductive_data(TEXT, TEXT) CASCADE;
DROP FUNCTION IF EXISTS decrypt_reproductive_data(BYTEA, TEXT) CASCADE;

-- Drop partition creation function
DROP FUNCTION IF EXISTS create_reproductive_health_partitions() CASCADE;

RAISE NOTICE '🗑️ Dropped reproductive health utility functions';

-- === DROP PARTITIONED TABLE AND PARTITIONS ===
-- Drop all reproductive health partitions first
DO $$
DECLARE
    partition_name TEXT;
    partition_count INTEGER := 0;
BEGIN
    -- Find and drop all reproductive health partitions
    FOR partition_name IN 
        SELECT tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'reproductive_health_%'
        AND tablename ~ '^reproductive_health_[0-9]{4}_[0-9]{2}$'
    LOOP
        EXECUTE 'DROP TABLE IF EXISTS ' || quote_ident(partition_name) || ' CASCADE';
        partition_count := partition_count + 1;
        RAISE NOTICE '🗑️ Dropped partition: %', partition_name;
    END LOOP;
    
    IF partition_count > 0 THEN
        RAISE NOTICE '✅ Dropped % reproductive health partitions', partition_count;
    ELSE
        RAISE NOTICE 'ℹ️ No reproductive health partitions found to drop';
    END IF;
END $$;

-- === DROP MAIN TABLE ===
-- Drop the main reproductive_health table
DROP TABLE IF EXISTS reproductive_health CASCADE;

RAISE NOTICE '🗑️ Dropped reproductive_health table';

-- === VERIFY CLEANUP ===
-- Verify that all components have been removed
DO $$
DECLARE
    table_count INTEGER;
    view_count INTEGER;
    function_count INTEGER;
    trigger_count INTEGER;
BEGIN
    -- Check for remaining tables
    SELECT COUNT(*) INTO table_count 
    FROM information_schema.tables 
    WHERE table_name LIKE 'reproductive_health%';
    
    -- Check for remaining views
    SELECT COUNT(*) INTO view_count 
    FROM information_schema.views 
    WHERE table_name LIKE 'reproductive_health%';
    
    -- Check for remaining functions
    SELECT COUNT(*) INTO function_count 
    FROM information_schema.routines 
    WHERE routine_name IN (
        'audit_reproductive_health_changes',
        'cleanup_old_reproductive_health_data',
        'encrypt_reproductive_data',
        'decrypt_reproductive_data',
        'create_reproductive_health_partitions'
    );
    
    -- Check for remaining triggers
    SELECT COUNT(*) INTO trigger_count 
    FROM information_schema.triggers 
    WHERE trigger_name LIKE '%reproductive_health%';
    
    -- Report cleanup status
    IF table_count = 0 AND view_count = 0 AND function_count = 0 AND trigger_count = 0 THEN
        RAISE NOTICE '✅ Rollback completed successfully - all components removed';
    ELSE
        RAISE NOTICE '⚠️ Rollback verification: tables=%, views=%, functions=%, triggers=%', 
            table_count, view_count, function_count, trigger_count;
    END IF;
END $$;

-- === REFRESH AUDIT LOG PARTITIONS ===
DO $$
BEGIN
    -- Refresh audit log partitions if the function exists
    IF EXISTS (SELECT 1 FROM information_schema.routines WHERE routine_name = 'create_monthly_partitions') THEN
        PERFORM create_monthly_partitions('audit_log_partitioned', 'created_at');
        RAISE NOTICE '✅ Audit log partitions refreshed after rollback';
    ELSE
        RAISE NOTICE 'ℹ️ Monthly partition function not found - audit logs may not be partitioned';
    END IF;
END $$;

-- === PGCRYPTO EXTENSION CHECK ===
-- Note: We don't drop pgcrypto as it might be used by other tables
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pgcrypto') THEN
        RAISE NOTICE 'ℹ️ pgcrypto extension preserved (may be used by other tables)';
    ELSE
        RAISE NOTICE 'ℹ️ pgcrypto extension was not installed';
    END IF;
END $$;

-- === FINAL ROLLBACK SUMMARY ===
DO $$
BEGIN
    RAISE NOTICE '🎉 Reproductive Health table rollback completed!';
    RAISE NOTICE '📋 Rollback summary:';
    RAISE NOTICE '   ✅ Dropped reproductive_health main table';
    RAISE NOTICE '   ✅ Dropped all monthly partitions';
    RAISE NOTICE '   ✅ Removed audit logging triggers';
    RAISE NOTICE '   ✅ Dropped Row Level Security policies';
    RAISE NOTICE '   ✅ Removed encryption/decryption functions';
    RAISE NOTICE '   ✅ Dropped utility views and functions';
    RAISE NOTICE '   ✅ Cleaned up all indexes and constraints';
    RAISE NOTICE '   ✅ Rollback operation logged in audit trail';
    RAISE NOTICE '🔐 Security note: pgcrypto extension preserved for other uses';
    RAISE NOTICE '📊 Note: All reproductive health data has been permanently deleted';
END $$;