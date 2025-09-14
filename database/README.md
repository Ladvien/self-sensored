# Database Directory Structure

This directory contains all SQL scripts and database-related files for the Health Export REST API.

## Directory Structure

```
database/
├── README.md                           # This file
├── schema.sql                          # Main database schema
├── setup/
│   └── setup_production.sql           # Production database setup script
├── migrations/
│   └── 0016_background_job_functions.sql # Database migration files
└── utils/
    ├── create_test_user.sql           # Create test user utility
    ├── create_test_api_key.sql        # Create test API key utility
    ├── add_hashed_api_key.sql         # Add hashed API key utility
    └── setup_api_key.sql              # API key setup utility
```

## Usage

### Production Setup

To set up the production database:

```bash
# Create user and database (run as postgres superuser)
sudo -u postgres psql -f database/setup/setup_production.sql

# Apply schema (will be done automatically by setup script)
```

### Development Setup

For local development:

```bash
# Apply main schema to your dev database
psql -d health_export_dev < database/schema.sql

# Create test user and API keys
psql -d health_export_dev < database/utils/create_test_user.sql
psql -d health_export_dev < database/utils/create_test_api_key.sql
```

### Test Setup

For test database:

```bash
# Use TEST_DATABASE_URL environment variable
# Schema applied automatically in test setup
```

## Database Configuration

- **Production Database**: `self_sensored`
- **Test Database**: `self_sensored_test`

See `.env.example` for database connection configuration.

## Schema Overview

The database uses a simplified schema with 5 core health metric types:

1. **Heart Rate Metrics** - BPM, resting heart rate, HRV
2. **Blood Pressure Metrics** - Systolic, diastolic, pulse
3. **Sleep Metrics** - Sleep stages, duration, efficiency
4. **Activity Metrics** - Steps, distance, energy burned
5. **Workouts** - Exercise type, duration, heart rate data

All tables use:
- UUID primary keys
- Monthly partitioning for time-series data
- BRIN indexes for optimal performance
- Proper foreign key constraints

## Migration System

Database migrations are stored in `migrations/` directory. Apply migrations in numerical order:

```bash
psql -d your_database < database/migrations/0016_background_job_functions.sql
```

## Security Notes

- All API keys should be hashed using Argon2
- Use environment variables for database credentials
- Never commit `.env` files to version control
- Use Row Level Security (RLS) for multi-tenant isolation