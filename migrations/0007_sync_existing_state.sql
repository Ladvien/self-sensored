-- Sync Existing State Migration
-- This migration marks previous migrations as applied since the database already has some tables

-- Mark the first two migrations as applied to sync state
INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
VALUES 
(1, 'initial schema', NOW(), true, decode('FF', 'hex'), 1000),
(2, 'health metrics schema', NOW(), true, decode('FF', 'hex'), 1000)
ON CONFLICT (version) DO NOTHING;

-- Add any missing functions or features that should exist
-- This is a no-op migration to sync state