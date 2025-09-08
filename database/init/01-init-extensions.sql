-- Health Export API Database Initialization
-- Enable required PostgreSQL extensions

-- Enable PostGIS for geospatial data (workout routes)
CREATE EXTENSION IF NOT EXISTS postgis;

-- Enable UUID generation for primary keys
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable pgcrypto for additional cryptographic functions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Enable pg_stat_statements for query performance monitoring
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Verify extensions are installed
SELECT 
    e.extname AS "Extension Name",
    e.extversion AS "Version",
    n.nspname AS "Schema"
FROM pg_extension e 
LEFT JOIN pg_namespace n ON n.oid = e.extnamespace
WHERE e.extname IN ('postgis', 'uuid-ossp', 'pgcrypto', 'pg_stat_statements')
ORDER BY e.extname;

-- Create the apple_health schema for data organization
CREATE SCHEMA IF NOT EXISTS apple_health;

-- Set default permissions
GRANT USAGE ON SCHEMA apple_health TO health_user;
GRANT CREATE ON SCHEMA apple_health TO health_user;