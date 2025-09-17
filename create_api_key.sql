-- Create API key for test@lolzlab.com user
-- API Key (to use in iOS app): test_auto_export_key_2024
-- User ID: b479a3b9-9ef1-4a82-9771-adff57432e18

INSERT INTO api_keys (
    user_id,
    name,
    key_hash,
    is_active,
    expires_at,
    rate_limit_per_hour
) VALUES (
    'b479a3b9-9ef1-4a82-9771-adff57432e18',
    'iOS Auto Export App - Primary',
    '$argon2id$v=19$m=19456,t=2,p=1$S39tdsZHn9W/qe9B2qtVEg$8p9rhB4iRzGDrb9W3hdqV63RbzgBN+98Qqke6g70gKM',
    true,
    CURRENT_TIMESTAMP + INTERVAL '1 year',
    1000
);

-- We'll only create one API key for now
-- The iOS app will use the key: test_auto_export_key_2024