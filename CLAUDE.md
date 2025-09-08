# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Health Export REST API** system designed to receive and store health data from the Auto Health Export iOS application. The project is built with Rust, Actix-web, SQLx, and PostgreSQL following modern architectural patterns for production deployment.

## Technology Stack

- **Language**: Rust
- **Web Framework**: Actix-web 4.x
- **Database**: PostgreSQL 17+ with PostGIS extension
- **ORM/Query Builder**: SQLx
- **Cache**: Redis
- **Container Orchestration**: Kubernetes
- **Monitoring**: Prometheus + Grafana
- **CI/CD**: GitHub Actions

## Commands

Rust-based commands for development and deployment:

```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run the development server
cargo run

# Run tests
cargo test

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_tests

# Check for compile errors without building
cargo check

# Format code
cargo fmt

# Run linting
cargo clippy

# Run linting with strict rules
cargo clippy -- -D warnings

# Install SQLx CLI for migrations
cargo install sqlx-cli --features postgres

# Run database migrations
sqlx migrate run

# Create new migration
sqlx migrate add migration_name

# Revert last migration
sqlx migrate revert
```

## Database Testing Rules - MANDATORY

**⚠️ CRITICAL: ALL database connections and testing MUST use the remote PostgreSQL server at 192.168.1.104**

### Database Connection Requirements:
- **Production Database**: `postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored`
- **Test Database**: `postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored_test`
- **NO localhost connections**: Never use localhost, 127.0.0.1, or local database connections
- **NO containers**: Do not use any containerized database solutions
- **Remote only**: All database operations must connect to 192.168.1.104

### Testing Database Configuration:
```env
# .env file MUST contain these exact values:
DATABASE_URL=postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored
TEST_DATABASE_URL=postgresql://self_sensored:37om3i*t3XfSZ0@192.168.1.104:5432/self_sensored_test
```

### Database Testing Commands:
```bash
# Test main database connection
PGPASSWORD='37om3i*t3XfSZ0' psql -h 192.168.1.104 -U self_sensored -d self_sensored -c "SELECT version();"

# Test test database connection  
PGPASSWORD='37om3i*t3XfSZ0' psql -h 192.168.1.104 -U self_sensored -d self_sensored_test -c "SELECT version();"

# Verify PostGIS functionality
PGPASSWORD='37om3i*t3XfSZ0' psql -h 192.168.1.104 -U self_sensored -d self_sensored -c "SELECT PostGIS_version();"

# Test UUID generation
PGPASSWORD='37om3i*t3XfSZ0' psql -h 192.168.1.104 -U self_sensored -d self_sensored -c "SELECT gen_random_uuid();"
```

### Rust Application Configuration:
- SQLx connection pools MUST point to 192.168.1.104
- All migrations MUST run against 192.168.1.104
- Integration tests MUST use the test database at 192.168.1.104
- No mocking or local database substitutes allowed

**This is a hard requirement - any code that attempts to connect to localhost or containers will be rejected.**

## Architecture Overview

The system follows a layered architecture pattern:

### Core Components
1. **API Gateway** - Rate limiting and SSL termination
2. **FastAPI Service** - Main REST API with /v1/ingest endpoint
3. **Validation Layer** - Pydantic models for input validation and sanitization
4. **Business Logic** - Core processing logic with async support
5. **Data Storage** - PostgreSQL for persistence, Redis for caching

### Database Design
- **User Management**: users, api_keys tables
- **Raw Data Backup**: raw_ingestions table (partitioned by month)
- **Health Metrics**: Separate tables for different metric types
  - heart_rate_metrics
  - blood_pressure_metrics  
  - sleep_metrics
  - activity_metrics
- **Workout Data**: workouts table with GPS route support via PostGIS
- **Audit Trail**: audit_log table for tracking actions

### Key Features
- **API Key Authentication**: Argon2 hashed keys with Redis caching
- **Rate Limiting**: Dual strategy (request count + bandwidth)
- **Data Partitioning**: Monthly partitions for time-series data
- **Geospatial Support**: PostGIS for workout route tracking
- **Individual Transaction Processing**: Each metric processed separately
- **Comprehensive Error Handling**: Detailed error responses with classifications

## Development Guidelines

### Implementation Path A Strategy:
1. **PHASE 1**: Restore Python implementation from git history
2. **PHASE 2**: Implement missing security (API key auth, rate limiting, audit)
3. **PHASE 3**: Add Redis caching layer and Prometheus monitoring
4. **PHASE 4**: Align database schema with architectural specification
5. **PHASE 5**: Performance optimization and production deployment
6. **PHASE 6**: CI/CD pipeline and comprehensive testing

### Database Management:
- Use SQLAlchemy with async support for database operations
- Implement monthly partitioning for time-series tables
- Use BRIN indexes for time-based queries on large tables
- Use PostGIS for geospatial workout data
- Leverage existing schema design from previous implementation

### Security Considerations:
- API keys must be Argon2 hashed
- Individual transactions per metric to prevent partial failures
- Comprehensive audit logging for all actions
- Rate limiting at both request and bandwidth levels

### Performance Optimizations:
- Redis caching for frequently accessed data
- Connection pooling for database connections
- BRIN indexes for time-series data
- Individual metric processing to isolate failures

## Testing Strategy

The system should include:
- Unit tests for core business logic
- Integration tests for the full ingest pipeline
- Database migration tests
- Performance tests for high-volume data ingestion
- Security tests for authentication and rate limiting

## Monitoring and Observability

Implement comprehensive monitoring with:
- Prometheus metrics for request counts, durations, errors
- Structured JSON logging with correlation IDs
- Data quality monitoring for anomalous values
- Alert management for system health issues

## Technology Evolution Context

This project underwent a complex technology evolution that is important for developers to understand:

### Migration History:
1. **Original Implementation (2024)**: Working Python/FastAPI implementation with comprehensive health data models, database schema with partitioning, and production-ready features
2. **Architecture Planning Phase**: Detailed 36KB ARCHITECTURE.md specification created targeting Rust/Actix-web stack for performance and modern architecture
3. **Implementation Deletion**: Previous Python implementation was completely removed (commit 306dc0d), leaving only architectural documentation
4. **Strategic Decision (STORY-001)**: Multi-agent analysis revealed technology stack contradiction and evaluated two paths:
   - **Path A (Chosen)**: Restore and enhance Python implementation (197 story points, 6-8 weeks)
   - **Path B**: Complete Rust rewrite per ARCHITECTURE.md (222 story points, 8-10 weeks)

### Decision Rationale:
- **Proven Foundation**: Previous Python implementation demonstrated solid domain understanding with 15+ health metric types
- **Lower Risk**: Building on existing business logic vs greenfield rewrite
- **Resource Efficiency**: 25% less effort with faster time to market
- **Architecture Compliance**: Python stack enhanced to meet all architectural requirements (security, caching, monitoring)

## Project Status

**Current Phase**: Technology Migration/Implementation (Path A Enhancement)
**Decision**: Python Enhancement chosen over Rust rewrite (see MIGRATION.md)
**Next Steps**: 
1. Restore Python implementation (STORY-004A)
2. Setup database infrastructure (STORY-008)
3. Implement API key authentication (STORY-005A)
4. Add rate limiting and caching (STORY-006A, STORY-007A)

**Key Files**:
- `MIGRATION.md` - Technology evolution and strategic decision documentation
- `BACKLOG.md` - Complete project backlog with stories
- `ARCHITECTURE.md` - Original architecture specification (Rust-based)
- `team_chat.md` - Multi-agent analysis findings