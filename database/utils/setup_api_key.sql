-- Setup the hashed API key for the Auto Export App
-- Token: hea_981e5a34ffd84382ba44f6e97e804a39

BEGIN;

-- First, ensure the test user exists
INSERT INTO users (id, email, apple_health_id, metadata)
VALUES (
    '11111111-1111-1111-1111-111111111111'::uuid,
    'auto-export-app@test.com',
    'auto_export_app_test',
    '{"name": "Auto Export App Test", "app_version": "1.0"}'::jsonb
)
ON CONFLICT (id) DO UPDATE
SET email = EXCLUDED.email,
    apple_health_id = EXCLUDED.apple_health_id,
    metadata = EXCLUDED.metadata,
    is_active = true;

-- Delete any existing API key with the same ID to avoid conflicts
DELETE FROM api_keys WHERE id = '22222222-2222-2222-2222-222222222222'::uuid;

-- Insert the new API key with the correct Argon2 hash
INSERT INTO api_keys (id, user_id, name, key_hash, permissions, rate_limit_per_hour, is_active)
VALUES (
    '22222222-2222-2222-2222-222222222222'::uuid,
    '11111111-1111-1111-1111-111111111111'::uuid,
    'Auto Export App Key',
    '$argon2id$v=19$m=19456,t=2,p=1$fIq99ZGywCK2I0ueAX/K0w$D8YBSeCTdj89Vslpk40YyGCjtpoFhqxOYRJEbplmHLk',
    '["read", "write"]'::jsonb,
    1000,
    true
);

COMMIT;

SELECT 'User and API key configured successfully' as message
UNION ALL
SELECT 'Token: hea_981e5a34ffd84382ba44f6e97e804a39' as message;