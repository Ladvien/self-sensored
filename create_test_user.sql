-- Create a test user for Auto Export app
DO $$
DECLARE
    test_user_id UUID;
    test_api_key_hash VARCHAR(255);
BEGIN
    -- Clean up any existing test user
    DELETE FROM users WHERE email = 'auto-export-test@example.com';
    
    -- Create new test user
    test_user_id := gen_random_uuid();
    INSERT INTO users (id, email, apple_health_id, metadata, created_at, updated_at)
    VALUES (
        test_user_id,
        'auto-export-test@example.com',
        'auto_export_test_user',
        '{"name": "Auto Export Test User", "created_by": "setup_script"}'::jsonb,
        NOW(),
        NOW()
    );
    
    -- Create API key with the hash for "test_auto_export_key_2024"
    -- This is a pre-computed Argon2 hash for testing
    test_api_key_hash := '$argon2id$v=19$m=19456,t=2,p=1$cGFzc3dvcmQxMjM$qKJ1ug6h7+lBU5V5wkFEeH0UYb3kCCehFxGvLSXK6T8';
    
    INSERT INTO api_keys (id, user_id, name, key_hash, permissions, created_at)
    VALUES (
        gen_random_uuid(),
        test_user_id,
        'Auto Export Test Key',
        test_api_key_hash,
        '["write", "read"]'::jsonb,
        NOW()
    );
    
    RAISE NOTICE 'Test user created successfully!';
    RAISE NOTICE 'Email: auto-export-test@example.com';
    RAISE NOTICE 'API Key: test_auto_export_key_2024';
    RAISE NOTICE 'User ID: %', test_user_id;
END $$;
