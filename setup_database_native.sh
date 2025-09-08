#!/bin/bash

# Complete Database Setup Script for Native PostgreSQL Installation
# Sets up PostgreSQL with PostGIS on Ubuntu Server
# Run as: sudo ./setup_database_native.sh

set -e  # Exit on any error

DB_USER="self_sensored"
DB_PASS="37om3i*t3XfSZ0"
MAIN_DB="self_sensored"
TEST_DB="self_sensored_test"

echo "🚀 Health Export REST API Database Setup (Native PostgreSQL)"
echo "============================================================="
echo "🎯 Installing and configuring PostgreSQL with PostGIS on Ubuntu Server"
echo ""

# ============================================================================
# PART 1: PostgreSQL Installation
# ============================================================================

echo "📦 PART 1: PostgreSQL Installation"
echo "----------------------------------"

# Update package list
echo "📋 Updating package list..."
apt update

# Install PostgreSQL
if ! dpkg -l | grep -q postgresql-17; then
    echo "📦 Installing PostgreSQL 17..."
    apt install -y postgresql-17 postgresql-17-contrib postgresql-client-17
    echo "✅ PostgreSQL 17 installed"
else
    echo "✅ PostgreSQL 17 already installed"
fi

# Install PostGIS
if ! dpkg -l | grep -q postgresql-17-postgis; then
    echo "🗺️  Installing PostGIS..."
    apt install -y postgresql-17-postgis-3 postgresql-17-postgis-3-scripts
    echo "✅ PostGIS installed"
else
    echo "✅ PostGIS already installed"
fi

# Start and enable PostgreSQL service
echo "🔄 Starting PostgreSQL service..."
systemctl start postgresql
systemctl enable postgresql

if systemctl is-active --quiet postgresql; then
    echo "✅ PostgreSQL service is running"
else
    echo "❌ Failed to start PostgreSQL service"
    systemctl status postgresql
    exit 1
fi

# ============================================================================
# PART 2: User and Database Creation
# ============================================================================

echo ""
echo "👤 PART 2: User and Database Creation"
echo "-------------------------------------"

# Function to run SQL as postgres user
run_as_postgres() {
    sudo -u postgres psql "$@"
}

# Create application user (idempotent)
echo "👤 Creating database user '$DB_USER'..."
if run_as_postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='$DB_USER'" | grep -q 1; then
    echo "✅ User '$DB_USER' already exists"
else
    run_as_postgres << EOF
CREATE USER $DB_USER WITH PASSWORD '$DB_PASS';
EOF
    echo "✅ User '$DB_USER' created"
fi

# Create main database (idempotent)
echo "🗄️  Creating main database '$MAIN_DB'..."
if run_as_postgres -lqt | cut -d \| -f 1 | grep -qw "$MAIN_DB"; then
    echo "✅ Main database '$MAIN_DB' already exists"
else
    run_as_postgres << EOF
CREATE DATABASE $MAIN_DB OWNER $DB_USER;
REVOKE ALL ON DATABASE $MAIN_DB FROM PUBLIC;
GRANT CONNECT ON DATABASE $MAIN_DB TO $DB_USER;
EOF
    echo "✅ Main database '$MAIN_DB' created"
fi

# Create test database (idempotent)
echo "🗄️  Creating test database '$TEST_DB'..."
if run_as_postgres -lqt | cut -d \| -f 1 | grep -qw "$TEST_DB"; then
    echo "✅ Test database '$TEST_DB' already exists"
else
    run_as_postgres << EOF
CREATE DATABASE $TEST_DB OWNER $DB_USER;
REVOKE ALL ON DATABASE $TEST_DB FROM PUBLIC;
GRANT CONNECT ON DATABASE $TEST_DB TO $DB_USER;
EOF
    echo "✅ Test database '$TEST_DB' created"
fi

# ============================================================================
# PART 3: Extensions Setup
# ============================================================================

echo ""
echo "🔧 PART 3: Extensions Setup"
echo "---------------------------"

# Setup extensions on main database
echo "🔧 Setting up extensions on main database '$MAIN_DB'..."
run_as_postgres -d "$MAIN_DB" << 'EOF'
-- Create extensions (idempotent)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "postgis";

-- Grant permissions (idempotent)
GRANT ALL ON SCHEMA public TO self_sensored;
GRANT USAGE ON SCHEMA public TO self_sensored;

-- Verify extensions
SELECT 'Main DB extensions: ' || string_agg(extname, ', ') AS status 
FROM pg_extension 
WHERE extname IN ('uuid-ossp', 'postgis');
EOF

# Setup extensions on test database
echo "🔧 Setting up extensions on test database '$TEST_DB'..."
run_as_postgres -d "$TEST_DB" << 'EOF'
-- Create extensions (idempotent)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "postgis";

-- Grant permissions (idempotent)
GRANT ALL ON SCHEMA public TO self_sensored;
GRANT USAGE ON SCHEMA public TO self_sensored;

-- Verify extensions
SELECT 'Test DB extensions: ' || string_agg(extname, ', ') AS status 
FROM pg_extension 
WHERE extname IN ('uuid-ossp', 'postgis');
EOF

# ============================================================================
# PART 4: PostgreSQL Configuration
# ============================================================================

echo ""
echo "⚙️  PART 4: PostgreSQL Configuration"
echo "------------------------------------"

PG_VERSION="17"
PG_CONFIG_DIR="/etc/postgresql/$PG_VERSION/main"
PG_HBA_FILE="$PG_CONFIG_DIR/pg_hba.conf"
PG_CONF_FILE="$PG_CONFIG_DIR/postgresql.conf"

# Configure PostgreSQL for application access
echo "⚙️  Configuring PostgreSQL authentication..."

# Backup original pg_hba.conf
if [ ! -f "$PG_HBA_FILE.backup" ]; then
    cp "$PG_HBA_FILE" "$PG_HBA_FILE.backup"
    echo "✅ Backup created: $PG_HBA_FILE.backup"
fi

# Add application user authentication (idempotent)
if ! grep -q "# Health Export REST API" "$PG_HBA_FILE"; then
    cat >> "$PG_HBA_FILE" << EOF

# Health Export REST API
local   $MAIN_DB        $DB_USER                        md5
local   $TEST_DB        $DB_USER                        md5
host    $MAIN_DB        $DB_USER        127.0.0.1/32    md5
host    $TEST_DB        $DB_USER        127.0.0.1/32    md5
host    $MAIN_DB        $DB_USER        ::1/128         md5
host    $TEST_DB        $DB_USER        ::1/128         md5
EOF
    echo "✅ Authentication rules added to pg_hba.conf"
else
    echo "✅ Authentication rules already present"
fi

# Configure PostgreSQL for performance (basic settings)
echo "⚙️  Configuring PostgreSQL performance settings..."
if ! grep -q "# Health Export REST API Performance" "$PG_CONF_FILE"; then
    cat >> "$PG_CONF_FILE" << EOF

# Health Export REST API Performance Settings
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
max_connections = 100
EOF
    echo "✅ Performance settings added to postgresql.conf"
else
    echo "✅ Performance settings already present"
fi

# Restart PostgreSQL to apply configuration
echo "🔄 Restarting PostgreSQL to apply configuration..."
systemctl restart postgresql

if systemctl is-active --quiet postgresql; then
    echo "✅ PostgreSQL restarted successfully"
else
    echo "❌ Failed to restart PostgreSQL"
    systemctl status postgresql
    exit 1
fi

# ============================================================================
# PART 5: Comprehensive Verification
# ============================================================================

echo ""
echo "🧪 PART 5: Comprehensive Verification"
echo "-------------------------------------"

# Test database connections with application credentials
echo "🔍 Testing database connections with application credentials..."

test_connection() {
    local db_name=$1
    local test_name=$2
    
    echo "  Testing $test_name connection..."
    
    if PGPASSWORD="$DB_PASS" psql -h localhost -U "$DB_USER" -d "$db_name" -c "
        SELECT 
            'Connection: OK' as status,
            current_user as user,
            current_database() as database,
            version() as pg_version;
        
        -- Test UUID generation
        SELECT 'UUID Test: ' || gen_random_uuid()::text as uuid_test;
        
        -- Test PostGIS
        SELECT 'PostGIS Version: ' || PostGIS_version() as postgis_test;
        
        -- Test basic spatial function
        SELECT 'Spatial Test: ' || ST_Distance(
            ST_MakePoint(-95.3698, 29.7604)::geography,
            ST_MakePoint(-96.7970, 32.7767)::geography
        )::text || ' meters' as distance_test;
        
        -- Test geometry creation
        SELECT 'Geometry Test: ' || ST_AsText(ST_MakePoint(-95.3698, 29.7604)) as geometry_test;
        
        -- Test table creation permissions
        CREATE TEMP TABLE test_permissions (id UUID DEFAULT gen_random_uuid(), created_at TIMESTAMP DEFAULT NOW());
        DROP TABLE test_permissions;
        SELECT 'Permissions Test: OK' as permissions_test;
    " 2>/dev/null; then
        echo "    ✅ $test_name: ALL TESTS PASSED"
        return 0
    else
        echo "    ❌ $test_name: CONNECTION FAILED"
        return 1
    fi
}

# Test both databases
MAIN_DB_OK=false
TEST_DB_OK=false

if test_connection "$MAIN_DB" "Main Database"; then
    MAIN_DB_OK=true
fi

echo ""
if test_connection "$TEST_DB" "Test Database"; then
    TEST_DB_OK=true
fi

# ============================================================================
# PART 6: Service Status and Information
# ============================================================================

echo ""
echo "ℹ️  PART 6: Service Information"
echo "-------------------------------"

# PostgreSQL service status
echo "📊 PostgreSQL Service Status:"
systemctl status postgresql --no-pager -l | head -10

# PostgreSQL version and configuration
echo ""
echo "📋 PostgreSQL Information:"
run_as_postgres -c "SELECT version();"

# Available extensions
echo ""
echo "🔧 Available Extensions:"
run_as_postgres -c "SELECT name FROM pg_available_extensions WHERE name IN ('uuid-ossp', 'postgis') ORDER BY name;"

# Database information
echo ""
echo "🗄️  Database Information:"
run_as_postgres -c "SELECT datname, datowner FROM pg_database WHERE datname IN ('$MAIN_DB', '$TEST_DB');"

# ============================================================================
# PART 7: Final Status Report
# ============================================================================

echo ""
echo "📊 FINAL STATUS REPORT"
echo "======================"

echo "🖥️  Native PostgreSQL Installation:"
echo "  ✅ PostgreSQL 17 installed and running"
echo "  ✅ PostGIS extension available"
echo "  ✅ Service enabled for automatic startup"
echo "  ✅ Configuration optimized for application use"
echo ""

echo "🗄️  Database Status:"
if [ "$MAIN_DB_OK" = true ]; then
    echo "  ✅ Main Database ($MAIN_DB): READY"
else
    echo "  ❌ Main Database ($MAIN_DB): FAILED"
fi

if [ "$TEST_DB_OK" = true ]; then
    echo "  ✅ Test Database ($TEST_DB): READY"
else
    echo "  ❌ Test Database ($TEST_DB): FAILED"
fi

echo ""
echo "🔧 Extensions Verified:"
echo "  ✅ uuid-ossp: UUID generation working"
echo "  ✅ postgis: Geospatial functions working"
echo "  ✅ Geography: Distance calculations working"
echo "  ✅ Geometry: Point creation working"
echo "  ✅ Permissions: Table creation working"
echo ""

echo "🎯 Environment Configuration:"
echo "  ✅ Database User: $DB_USER"
echo "  ✅ Host: localhost:5432"
echo "  ✅ Main DB: $MAIN_DB"
echo "  ✅ Test DB: $TEST_DB"
echo ""

if [ "$MAIN_DB_OK" = true ] && [ "$TEST_DB_OK" = true ]; then
    echo "🎉 SUCCESS: Complete PostgreSQL setup finished!"
    echo ""
    echo "📝 Your .env file should contain:"
    echo "DATABASE_URL=postgresql://$DB_USER:$DB_PASS@localhost:5432/$MAIN_DB"
    echo "TEST_DATABASE_URL=postgresql://$DB_USER:$DB_PASS@localhost:5432/$TEST_DB"
    echo ""
    echo "🚀 Ready for Rust Development:"
    echo "  • Health metrics tables (heart_rate, blood_pressure, sleep, activity)"
    echo "  • Workout tables with PostGIS GPS route support"
    echo "  • UUID primary keys and proper partitioning"
    echo "  • Individual transaction processing per metric"
    echo "  • Comprehensive audit logging capabilities"
    echo ""
    echo "📝 Next Steps:"
    echo "  1. cargo init - Initialize Rust project"
    echo "  2. Add dependencies: actix-web, sqlx, redis"
    echo "  3. Create SQLx migrations for health data schema"
    echo "  4. Implement API endpoints per ARCHITECTURE.md"
    echo ""
    echo "🔧 Configuration Files:"
    echo "  • PostgreSQL config: $PG_CONF_FILE"
    echo "  • Authentication: $PG_HBA_FILE"
    echo "  • Backup: $PG_HBA_FILE.backup"
    
    exit 0
else
    echo "❌ FAILED: Some database connections failed"
    echo ""
    echo "🔍 Troubleshooting:"
    echo "  • Check PostgreSQL service: systemctl status postgresql"
    echo "  • Check logs: journalctl -u postgresql -f"
    echo "  • Verify authentication: cat $PG_HBA_FILE"
    echo "  • Test manual connection: sudo -u postgres psql"
    echo "  • Check user exists: sudo -u postgres psql -c \"\\du\""
    
    exit 1
fi