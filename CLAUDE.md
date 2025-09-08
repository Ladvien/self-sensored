# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Health Export REST API** system designed to receive and store health data from the Auto Health Export iOS application. The project underwent strategic technology decision-making and is now in the **implementation phase** following **Path A: Python Enhancement** strategy. A working Python implementation previously existed but was deleted, and is being restored and enhanced with missing architectural components.

## Technology Stack (Chosen Path A)

- **Language**: Python 3.13
- **Web Framework**: FastAPI  
- **Database**: PostgreSQL 15+ with PostGIS extension
- **ORM/Query Builder**: SQLAlchemy + Asyncpg
- **Cache**: Redis
- **Container Orchestration**: Kubernetes
- **Monitoring**: Prometheus + Grafana
- **CI/CD**: GitHub Actions

## Commands

Python-based commands for development and deployment:

```bash
# Install dependencies
poetry install

# Run the development server
uvicorn app.main:app --reload

# Run tests
pytest

# Run database migrations (when implemented)
alembic upgrade head

# Run linting
ruff check .

# Format code
black .

# Run with docker-compose for development
docker-compose up
```

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

## Project Status

**Current Phase**: Strategic Decision Complete - Path A Implementation
**Decision**: Python Enhancement chosen over Rust rewrite (see DECISION.md)
**Next Steps**: 
1. Restore Python implementation (STORY-004A)
2. Setup database infrastructure (STORY-008)
3. Implement API key authentication (STORY-005A)
4. Add rate limiting and caching (STORY-006A, STORY-007A)

**Key Files**:
- `DECISION.md` - Strategic technology decision rationale
- `BACKLOG.md` - Complete project backlog with stories
- `ARCHITECTURE.md` - Original architecture specification
- `team_chat.md` - Multi-agent analysis findings