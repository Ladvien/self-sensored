-- Clean up and create test user with working API key
-- This uses a pre-generated hash that matches the key "test123"

BEGIN;

-- Delete existing test user
DELETE FROM users WHERE email = 'auto-export-app@test.com';

-- Create test user
INSERT INTO users (id, email, apple_health_id, metadata)
VALUES (
    '11111111-1111-1111-1111-111111111111'::uuid,
    'auto-export-app@test.com',
    'auto_export_app_test',
    '{"name": "Auto Export App Test", "app_version": "1.0"}'::jsonb
);

-- Create API key with hash for "test123"
-- This hash was generated using the same Argon2 settings as the app
INSERT INTO api_keys (id, user_id, name, key_hash, permissions)
VALUES (
    '22222222-2222-2222-2222-222222222222'::uuid,
    '11111111-1111-1111-1111-111111111111'::uuid,
    'Auto Export App Key',
    '$argon2id$v=19$m=19456,t=2,p=1$MTIzNDU2Nzg5MGFiY2RlZg$rl+LXHQ5kvQkPDYJqpGJiXg7qFpLKPFY6gZbCCPJLmQ',
    '["read", "write"]'::jsonb
);

COMMIT;

SELECT 'Test user created: auto-export-app@test.com' as message
UNION ALL
SELECT 'API Key: test123' as message;