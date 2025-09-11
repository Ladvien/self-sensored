-- Enable required extensions
-- Note: These require superuser privileges. 
-- If they fail, create them manually as postgres user before running migrations.
DO $$ 
BEGIN
    -- Try to create extensions, but continue if they fail (already exist or no privileges)
    BEGIN
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'uuid-ossp extension already exists or insufficient privileges';
    END;
    
    BEGIN  
        CREATE EXTENSION IF NOT EXISTS "postgis";
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'postgis extension already exists or insufficient privileges';
    END;
END $$;

-- Users table for storing user information
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    full_name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN DEFAULT TRUE
);

-- API Keys table for authentication
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL, -- Friendly name for the API key
    key_hash VARCHAR(255) NOT NULL UNIQUE, -- Argon2 hashed API key
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT TRUE,
    scopes TEXT[], -- Array of permission scopes
    
    CONSTRAINT api_keys_user_id_name_unique UNIQUE(user_id, name)
);

-- Create indexes for performance
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_last_used_at ON api_keys(last_used_at);
CREATE INDEX idx_users_email ON users(email);

-- Audit log table for security tracking
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL, -- login, logout, api_call, etc.
    resource VARCHAR(255), -- Resource accessed
    ip_address INET,
    user_agent TEXT,
    metadata JSONB, -- Additional context data
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for audit log performance
CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_api_key_id ON audit_log(api_key_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_audit_log_action ON audit_log(action);

-- Create a trigger to update the updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Insert a default admin user for testing
INSERT INTO users (email, full_name) VALUES 
('admin@healthexport.com', 'Health Export Admin');