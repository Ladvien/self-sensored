# Health Export REST API - Development Backlog

## Executive Summary

This backlog contains comprehensive Jira stories derived from multi-agent analysis of the Health Export REST API project. The project currently exists as complete architectural documentation (`ARCHITECTURE.md`) but has zero implementation - all previous Python FastAPI code was deleted, leaving only documentation and planning files.

## Strategic Decision Required

**CRITICAL**: Before starting any development work, a strategic decision is required between two paths:

### Path A: Python Enhancement (60% Less Effort)
Restore the deleted Python FastAPI implementation and enhance it with missing components.

### Path B: Rust Rewrite (Architecture Compliant)  
Complete greenfield Rust implementation following ARCHITECTURE.md specification exactly.

**All stories below are organized by these two paths with clear effort estimates and dependencies.**

---

## Epic Structure

### Strategic Epics
- **EPIC-001**: Technology Stack Decision & Project Foundation
- **EPIC-002**: [Path A] Python Implementation Enhancement  
- **EPIC-003**: [Path B] Rust Implementation from Scratch
- **EPIC-004**: Security & Authentication Layer
- **EPIC-005**: Infrastructure & Deployment
- **EPIC-006**: Monitoring & Observability
- **EPIC-007**: Documentation & Migration

### Cross-Cutting Epics
- **EPIC-008**: Database Schema & Optimization
- **EPIC-009**: Testing & Quality Assurance
- **EPIC-010**: Performance & Scaling

---

## P0 Stories (Critical - Must Do First)

### STORY-001: Strategic Technology Decision
**Epic**: EPIC-001  
**Priority**: P0  
**Story Points**: 1  
**Type**: Decision Story  

**Description:**  
As a project stakeholder, I need to make a strategic decision between continuing with Python enhancement vs complete Rust rewrite so that development can proceed with clear direction and resource allocation.

**Background:**  
- Previous Python FastAPI implementation was completely deleted (confirmed by git history analysis)
- ARCHITECTURE.md specifies Rust/Actix-web/SQLx stack (36KB comprehensive spec)
- Python implementation had working health data models for 15+ metric types
- Missing critical security components in previous implementation
- Technology stack mismatch creates fundamental architectural conflict

**Acceptance Criteria:**  
- [ ] Review cost/benefit analysis for both paths
- [ ] Evaluate resource availability and timeline constraints  
- [ ] Consider architectural compliance requirements
- [ ] Document decision rationale in DECISION.md
- [ ] Update CLAUDE.md to reflect chosen path
- [ ] Assign path-specific stories to development team

**Definition of Done:**  
- Strategic decision documented and communicated
- Development path chosen (Path A or Path B)
- Relevant stories activated for chosen path
- Project timeline and resource allocation confirmed

**Dependencies:** None (blocking for all other development)  
**Effort Estimates:**
- Path A Total: ~150-200 story points
- Path B Total: ~400-500 story points

---

### STORY-002: Documentation Accuracy Update
**Epic**: EPIC-007  
**Priority**: P0  
**Story Points**: 2  
**Type**: Technical Debt  

**Description:**  
As a developer onboarding to the project, I need accurate documentation that reflects the actual project state so that I understand this is a migration/rebuild scenario, not a planning phase.

**Background:**  
- CLAUDE.md Line 7 incorrectly claims "no implementation yet" 
- CLAUDE.md Line 125 misleadingly states "Documentation and Architecture Planning" phase
- Git history shows comprehensive Python implementation was deleted
- Project is actually in technology migration/rebuild phase

**Acceptance Criteria:**  
- [ ] Update CLAUDE.md Line 7 to reflect previous implementation existed
- [ ] Change Line 125 to specify "Technology Migration/Rebuild" phase  
- [ ] Add section documenting Python→Rust transition reasoning
- [ ] Create MIGRATION.md explaining technology stack change
- [ ] Clean .gitignore to remove duplicate Python patterns
- [ ] Update project status to reflect current reality

**Definition of Done:**  
- All documentation accurately reflects project state
- New developers understand migration context
- Technology transition is properly documented
- CLAUDE.md aligns with actual project phase

**Dependencies:** STORY-001 (tech decision)  
**Related Issues:** Documentation Analyst findings - misleading project status

---

### STORY-003: Essential Project Documentation
**Epic**: EPIC-007  
**Priority**: P0  
**Story Points**: 3  
**Type**: Documentation Story  

**Description:**  
As a developer or stakeholder, I need basic project documentation (README, LICENSE) so that I can understand the project, set up development environment, and comply with licensing requirements.

**Background:**  
- README.md was deleted (confirmed by git status)
- LICENSE file was deleted
- No setup instructions exist for either Python or Rust path
- Project lacks basic open-source documentation standards

**Acceptance Criteria:**  
- [ ] Create comprehensive README.md with project overview
- [ ] Add LICENSE file (confirm licensing approach with stakeholder)
- [ ] Document setup instructions for both potential paths
- [ ] Include API documentation links and examples
- [ ] Add contribution guidelines
- [ ] Document architecture overview and key decisions
- [ ] Include deployment instructions and requirements

**Definition of Done:**  
- README.md provides clear project overview and setup
- LICENSE file exists with appropriate license
- New contributors can understand project quickly
- Setup instructions are complete and tested

**Dependencies:** STORY-001 (tech decision affects setup instructions)  
**Related Issues:** Documentation Analyst findings - missing essential files

---

## P1 Stories (High Priority - Path A: Python Enhancement)

### STORY-004A: Restore Python Implementation from Git History
**Epic**: EPIC-002  
**Priority**: P1 (Path A Only)  
**Story Points**: 5  
**Type**: Implementation Story  

**Description:**  
As a developer continuing with Python, I need to restore the deleted Python FastAPI implementation from git history so that I can build upon the existing working health data models and database schema.

**Background:**  
- Complete Python implementation exists in git history (commit 306dc0d)
- Includes working FastAPI app with 15+ health metric types
- Has production-ready database schema with partitioning and optimization
- Comprehensive error handling and data validation implemented
- Only missing security, caching, and monitoring components

**Acceptance Criteria:**  
- [ ] Restore all Python files from git history (app/, pyproject.toml, etc.)
- [ ] Verify restoration completeness against git history analysis
- [ ] Test that restored application starts without errors
- [ ] Validate database schema against PostgreSQL instance  
- [ ] Confirm all health metric models are functional
- [ ] Run existing tests to verify restoration integrity
- [ ] Update dependencies to latest compatible versions

**Definition of Done:**  
- Complete Python application restored and functional
- All tests pass
- Database schema can be created successfully
- Health data models validate correctly
- Application starts and responds to health check

**Dependencies:** STORY-001 (tech decision), STORY-003 (documentation)  
**Related Issues:** Code Analyst findings - comprehensive working implementation existed

---

### STORY-005A: Implement API Key Authentication System
**Epic**: EPIC-004  
**Priority**: P1 (Path A Only)  
**Story Points**: 8  
**Type**: Feature Story  

**Description:**  
As a health data API, I need secure API key authentication with Argon2 hashing so that only authorized applications can submit health data and user data is protected.

**Background:**  
- Previous Python implementation had zero authentication
- ARCHITECTURE.md specifies Argon2 hashing (19MB memory, 2 iterations)  
- Redis caching required for validated keys (5-minute TTL)
- Bearer token authentication in Authorization header
- API key last-used tracking required

**Acceptance Criteria:**  
- [ ] Create users table with UUID primary keys
- [ ] Create api_keys table with Argon2 hash storage  
- [ ] Implement API key generation with secure random values
- [ ] Build Argon2 hashing with specified parameters (19MB memory, 2 iterations)
- [ ] Create authentication middleware for Bearer token validation
- [ ] Implement Redis caching for validated keys (5-minute TTL)
- [ ] Add last_used_at timestamp updating
- [ ] Build API key management endpoints (create, revoke, list)
- [ ] Add comprehensive error handling for auth failures

**Definition of Done:**  
- API key authentication fully functional
- Redis caching working for performance
- All endpoints protected with authentication
- API key management interface available
- Security testing passes

**Dependencies:** STORY-004A (Python restoration), Redis infrastructure  
**Related Issues:** Architecture Reviewer - missing security components

---

### STORY-006A: Implement Rate Limiting System  
**Epic**: EPIC-004  
**Priority**: P1 (Path A Only)  
**Story Points**: 5  
**Type**: Feature Story  

**Description:**  
As the API, I need dual-strategy rate limiting (request count + bandwidth) so that individual API keys cannot overwhelm the system and fair usage is enforced.

**Background:**  
- ARCHITECTURE.md specifies 100 requests/hour + 10MB bandwidth/hour per API key
- Redis-backed sliding window implementation required
- Previous Python implementation had configuration but no implementation
- Proper error responses with retry-after headers needed

**Acceptance Criteria:**  
- [ ] Implement Redis-backed request counting (100/hour per API key)
- [ ] Add bandwidth limiting (10MB/hour per API key)  
- [ ] Use sliding window approach for accurate rate limiting
- [ ] Return proper HTTP 429 responses when limits exceeded
- [ ] Include retry-after headers in rate limit responses
- [ ] Add rate limit status headers for client visibility
- [ ] Create rate limit middleware for all endpoints
- [ ] Implement configurable limits for different API key types

**Definition of Done:**  
- Rate limiting enforces 100 requests/hour limit
- Bandwidth limiting enforces 10MB/hour limit
- Proper HTTP responses with retry information
- Rate limiting can be monitored and configured
- Integration tests pass for various scenarios

**Dependencies:** STORY-005A (authentication), Redis infrastructure  
**Related Issues:** Architecture Reviewer - comprehensive rate limiting specification

---

### STORY-007A: Implement Redis Caching Layer
**Epic**: EPIC-002  
**Priority**: P1 (Path A Only)  
**Story Points**: 8  
**Type**: Feature Story  

**Description:**  
As the API, I need a Redis caching layer for API keys, user summaries, and recent metrics so that database load is reduced and response times are improved.

**Background:**  
- Previous Python implementation had no caching (direct database hits)
- ARCHITECTURE.md specifies comprehensive Redis caching strategy
- API key caching (5-minute TTL), user summaries, recent metrics needed
- Cache invalidation patterns required for data consistency

**Acceptance Criteria:**  
- [ ] Set up Redis connection pool with Python redis client
- [ ] Implement API key caching (5-minute TTL) for authentication
- [ ] Cache user health summaries (5-minute TTL)  
- [ ] Cache recent metrics by user and type (10-minute TTL)
- [ ] Build cache invalidation patterns (user-based, metric-type-based)
- [ ] Add cache warming for frequently accessed data
- [ ] Implement cache-aside pattern with proper fallback
- [ ] Add cache hit/miss metrics for monitoring
- [ ] Create cache management utilities for debugging

**Definition of Done:**  
- Redis caching improves authentication performance  
- User data queries use cache when available
- Cache invalidation maintains data consistency
- Cache hit rates are monitored and optimized
- Fallback to database works when cache unavailable

**Dependencies:** STORY-004A (Python restoration), Redis infrastructure  
**Related Issues:** Code Analyst - missing caching strategy

---

## P1 Stories (High Priority - Path B: Rust Rewrite)

### STORY-004B: Initialize Rust Project Structure
**Epic**: EPIC-003  
**Priority**: P1 (Path B Only)  
**Story Points**: 3  
**Type**: Foundation Story  

**Description:**  
As a Rust developer, I need a complete project structure with all dependencies so that I can begin implementing the health export API according to ARCHITECTURE.md specifications.

**Background:**  
- Zero Rust files exist (no Cargo.toml, src/, etc.)
- ARCHITECTURE.md specifies exact technology stack
- Need Actix-web 4.x, SQLx, PostgreSQL, Redis dependencies
- Project structure must support comprehensive architecture

**Acceptance Criteria:**  
- [ ] Create Cargo.toml with all required dependencies:
  - Actix-web 4.x (web framework)
  - SQLx (async SQL toolkit, NOT an ORM)
  - Tokio (async runtime)
  - Serde (JSON serialization)
  - UUID (unique identifiers)
  - Thiserror (error handling)
  - Validator (input validation)
  - Argon2 (password/API key hashing)
  - Redis client
  - Prometheus client
  - Tracing (structured logging)
- [ ] Create src/ directory structure (main.rs, lib.rs, modules)
- [ ] Set up migrations/ directory for SQLx migrations
- [ ] Create tests/ directory for integration tests
- [ ] Add .env.example with all required environment variables
- [ ] Configure Rust toolchain and formatting (rustfmt.toml)
- [ ] Set up Clippy configuration for linting standards

**Definition of Done:**  
- Cargo.toml compiles without errors
- Project structure follows Rust conventions
- All architectural dependencies included
- Development environment can be set up from README
- Basic application starts successfully

**Dependencies:** STORY-001 (tech decision), STORY-003 (documentation)  
**Related Issues:** File Explorer - complete absence of Rust project structure

---

### STORY-005B: Implement Core Database Schema with SQLx
**Epic**: EPIC-008  
**Priority**: P1 (Path B Only)  
**Story Points**: 13  
**Type**: Implementation Story  

**Description:**  
As the Rust API, I need a complete PostgreSQL database schema with SQLx migrations so that all health data types can be stored with proper partitioning, indexing, and PostGIS support.

**Background:**  
- ARCHITECTURE.md specifies comprehensive schema (users, api_keys, health metrics, workouts)
- Monthly partitioning required for time-series data
- PostGIS extension needed for workout GPS routes
- BRIN indexes for time-based queries on large tables
- Previous Python schema exists but needs conversion to SQLx format

**Acceptance Criteria:**  
- [ ] Create SQLx migration 001_initial_schema.sql:
  - users table with UUID primary keys
  - api_keys table with Argon2 hash storage
  - raw_ingestions table (partitioned by month)
  - audit_log table (partitioned by month)
- [ ] Create migration 002_health_metrics.sql:
  - heart_rate_metrics (partitioned)
  - blood_pressure_metrics (partitioned)  
  - sleep_metrics (partitioned)
  - activity_metrics (partitioned)
- [ ] Create migration 003_workouts.sql:
  - workouts table with energy/HR data
  - workout_routes with PostGIS GEOGRAPHY type
- [ ] Create migration 004_indexes.sql:
  - BRIN indexes for time-series queries
  - GiST indexes for geospatial data
  - Partial indexes for recent data
- [ ] Create migration 005_partitioning.sql:
  - Monthly partition creation function
  - Initial partitions for current + future 12 months
- [ ] Build Rust structs for all database models
- [ ] Test migrations against PostgreSQL with PostGIS

**Definition of Done:**  
- All SQLx migrations run successfully
- Database schema matches ARCHITECTURE.md exactly
- PostGIS extension enabled and functional
- Partitioning works for time-series data
- All indexes created and optimized
- Rust models compile and validate

**Dependencies:** STORY-004B (Rust project), PostgreSQL with PostGIS  
**Related Issues:** Architecture Reviewer - comprehensive database requirements

---

### STORY-006B: Build Core Actix-web Application Structure
**Epic**: EPIC-003  
**Priority**: P1 (Path B Only)  
**Story Points**: 8  
**Type**: Implementation Story  

**Description:**  
As a Rust API, I need the core Actix-web application with routing, middleware, and basic structure so that I can build the health data ingestion endpoint according to architectural specifications.

**Background:**  
- ARCHITECTURE.md specifies Actix-web 4.x framework
- Main endpoint: POST /v1/ingest with Bearer token auth
- Health endpoints: GET /health, GET /ready
- Prometheus metrics endpoint: GET /metrics (port 9090)
- Structured JSON logging with correlation IDs

**Acceptance Criteria:**  
- [ ] Create main.rs with Actix-web application setup
- [ ] Configure database connection pool (5-20 connections)
- [ ] Set up Redis connection pool  
- [ ] Create routing structure:
  - POST /v1/ingest (main data endpoint)
  - GET /health (health check with DB test)
  - GET /ready (readiness probe)
  - GET /metrics (Prometheus metrics on port 9090)
- [ ] Build middleware stack:
  - Request correlation IDs
  - Structured JSON logging (tracing)
  - Request timing and metrics collection
  - CORS handling if needed
- [ ] Create configuration management (TOML-based)
- [ ] Add graceful shutdown handling
- [ ] Implement basic error handling and responses

**Definition of Done:**  
- Actix-web application starts and responds
- All endpoints return appropriate responses
- Database and Redis connections established
- Logging produces structured JSON output
- Configuration can be loaded from files/environment

**Dependencies:** STORY-004B (project structure), STORY-005B (database)  
**Related Issues:** Architecture Reviewer - comprehensive API endpoint specification

---

## P1 Stories (Cross-Cutting - Both Paths)

### STORY-008: Database Infrastructure Setup
**Epic**: EPIC-005  
**Priority**: P1 (Both Paths)  
**Story Points**: 5  
**Type**: Infrastructure Story  

**Description:**  
As the development team, I need PostgreSQL with PostGIS and Redis infrastructure so that the application can store health data and implement caching regardless of technology stack choice.

**Background:**  
- Both paths require PostgreSQL 15+ with PostGIS extension  
- Redis needed for caching and rate limiting
- Development environment needs Docker Compose setup
- Production environment needs Kubernetes configuration

**Acceptance Criteria:**  
- [ ] Create docker-compose.yml for development:
  - PostgreSQL 15 with PostGIS extension
  - Redis 7.x with persistence  
  - Proper volume mounts and networking
  - Environment variable configuration
- [ ] Test database connectivity and PostGIS functionality
- [ ] Verify Redis connectivity and basic operations
- [ ] Create Kubernetes manifests for production:
  - PostgreSQL StatefulSet with persistent volumes
  - Redis Deployment with ConfigMap
  - Services and networking configuration  
- [ ] Document connection strings and configuration
- [ ] Add health checks and monitoring for both databases

**Definition of Done:**  
- Development database accessible via Docker Compose
- PostGIS extension functional for geospatial queries
- Redis available for caching operations
- Production Kubernetes manifests ready for deployment
- Database connectivity tested from application

**Dependencies:** STORY-001 (tech decision affects connection details)  
**Related Issues:** Architecture Reviewer - PostgreSQL + Redis requirements

---

### STORY-009: Monitoring Infrastructure Foundation
**Epic**: EPIC-006  
**Priority**: P1 (Both Paths)  
**Story Points**: 8  
**Type**: Infrastructure Story  

**Description:**  
As an operations team member, I need Prometheus metrics collection and structured logging so that I can monitor application health, performance, and debug issues in production.

**Background:**  
- ARCHITECTURE.md specifies Prometheus + Grafana monitoring
- Structured JSON logging required for observability
- Metrics needed: request counts, durations, errors, active users
- Both Python and Rust paths need monitoring capabilities

**Acceptance Criteria:**  
- [ ] Set up Prometheus metrics collection:
  - Request counters by endpoint and status
  - Request duration histograms  
  - Error counters by type
  - Active user gauges
  - Database query performance metrics
- [ ] Implement structured JSON logging:
  - Correlation IDs for request tracing
  - Log levels (info, warn, error, debug)
  - Contextual metadata (user_id, request_id, etc.)
  - Performance timing information
- [ ] Create Prometheus configuration:
  - Scraping configuration for application
  - AlertManager rules for critical issues
  - Recording rules for common queries
- [ ] Build basic Grafana dashboards:
  - Application overview dashboard
  - Performance monitoring dashboard  
  - Error tracking dashboard
- [ ] Add health metrics for infrastructure:
  - Database connection pool status
  - Redis connectivity and performance
  - Memory and CPU utilization

**Definition of Done:**  
- Prometheus successfully collects application metrics
- JSON logs are structured and searchable
- Grafana dashboards show application health
- Alerting rules trigger for critical issues
- Monitoring works for both local and production

**Dependencies:** Application implementation (different per path)  
**Related Issues:** Architecture Reviewer - comprehensive observability requirements

---

## P2 Stories (Medium Priority - Path A Enhancements)

### STORY-010A: Enhance Database Schema Alignment
**Epic**: EPIC-008  
**Priority**: P2 (Path A Only)  
**Story Points**: 8  
**Type**: Technical Debt Story  

**Description:**  
As a Python developer, I need the database schema aligned with ARCHITECTURE.md specifications so that the table structure, partitioning strategy, and indexing match the architectural requirements.

**Background:**  
- Previous Python implementation uses yearly partitions (2012-2028)
- ARCHITECTURE.md specifies monthly partitioning
- Table names and structure differ from specification
- BRIN indexes and PostGIS integration needs verification

**Acceptance Criteria:**  
- [ ] Analyze differences between current schema and ARCHITECTURE.md
- [ ] Create migration scripts to align table structures:
  - Convert yearly partitions to monthly partitions
  - Rename tables to match specification if needed
  - Add missing columns or constraints
- [ ] Verify BRIN index implementation for time-series queries
- [ ] Test PostGIS integration for workout routes
- [ ] Update SQLAlchemy models to match new schema
- [ ] Migrate existing data to new partition structure
- [ ] Update all queries and endpoints for schema changes

**Definition of Done:**  
- Database schema exactly matches ARCHITECTURE.md
- Monthly partitioning implemented and functional
- All existing data migrated successfully  
- Python models and queries work with new schema
- Performance remains optimal with new structure

**Dependencies:** STORY-004A (Python restoration), STORY-008 (database infrastructure)  
**Related Issues:** Code Analyst - schema differences identified

---

### STORY-011A: Add Prometheus Metrics to Python App
**Epic**: EPIC-006  
**Priority**: P2 (Path A Only)  
**Story Points**: 5  
**Type**: Feature Story  

**Description:**  
As an operations team member, I need Prometheus metrics from the Python FastAPI application so that I can monitor performance, errors, and usage patterns.

**Background:**  
- Previous Python implementation had basic logging only
- ARCHITECTURE.md specifies comprehensive Prometheus metrics
- Need integration with Python prometheus_client library
- Metrics endpoint should be available on port 9090

**Acceptance Criteria:**  
- [ ] Install prometheus_client Python library
- [ ] Implement core metrics:
  - ingest_requests_total (counter)
  - ingest_errors_total (counter)  
  - ingest_duration_seconds (histogram)
  - metrics_processed_total (counter)
  - active_users (gauge)
- [ ] Create metrics middleware for automatic collection
- [ ] Add custom metrics for health data processing
- [ ] Expose /metrics endpoint on port 9090
- [ ] Add metrics for database query performance
- [ ] Include Redis cache hit/miss metrics
- [ ] Test metrics collection and Prometheus scraping

**Definition of Done:**  
- Prometheus metrics endpoint functional
- All specified metrics being collected
- Metrics accurately reflect application behavior
- Integration with monitoring infrastructure complete
- Performance impact of metrics collection minimal

**Dependencies:** STORY-004A (Python restoration), STORY-009 (monitoring infrastructure)  
**Related Issues:** Architecture Reviewer - missing observability features

---

### STORY-012A: Implement Audit Logging System
**Epic**: EPIC-004  
**Priority**: P2 (Path A Only)  
**Story Points**: 5  
**Type**: Feature Story  

**Description:**  
As a compliance officer, I need comprehensive audit logging of all API actions so that user data access is tracked and security incidents can be investigated.

**Background:**  
- Previous Python implementation had no audit logging
- ARCHITECTURE.md specifies comprehensive audit trail
- Need IP address, user agent, and action metadata tracking
- Audit data should be partitioned by month for performance

**Acceptance Criteria:**  
- [ ] Create audit_log table (if not exists from schema alignment)
- [ ] Implement audit logging middleware:
  - Capture all API requests with user context
  - Record IP addresses and user agent strings
  - Log action types (data_ingest, api_key_create, etc.)
  - Store metadata as JSONB for flexibility
- [ ] Add audit entries for critical actions:
  - Data ingestion (with processing results)
  - API key creation/revocation
  - Authentication failures
  - Rate limiting violations
- [ ] Implement asynchronous audit writing to avoid performance impact
- [ ] Add audit log retention policies
- [ ] Create audit log query utilities for investigations

**Definition of Done:**  
- All API actions are audited appropriately
- Audit logs include sufficient detail for investigations  
- Audit logging doesn't impact API performance
- Retention policies prevent unbounded growth
- Audit queries work efficiently

**Dependencies:** STORY-005A (authentication), database schema  
**Related Issues:** Architecture Reviewer - comprehensive audit requirements

---

## P2 Stories (Medium Priority - Path B Rust Features)

### STORY-010B: Implement Health Data Validation Layer
**Epic**: EPIC-003  
**Priority**: P2 (Path B Only)  
**Story Points**: 13  
**Type**: Implementation Story  

**Description:**  
As the Rust API, I need comprehensive health data validation with Serde and Validator so that incoming health metrics are properly validated, typed, and processed according to the 15+ metric types specification.

**Background:**  
- ARCHITECTURE.md shows comprehensive validation examples
- Previous Python implementation had working Pydantic models for 15+ types
- Need Rust structs with serde and validator derives
- Timezone-aware datetime handling required

**Acceptance Criteria:**  
- [ ] Create Rust structs for all health metric types:
  - HeartRateMetric (min/avg/max with range validation 20-300)
  - BloodPressureMetric (systolic/diastolic validation)
  - SleepMetric (duration and timing validation)
  - ActivityMetric (steps, distance, flights_climbed)
  - WorkoutMetric (with GPS route support)
  - 10+ additional specialized metrics
- [ ] Implement serde serialization/deserialization
- [ ] Add validator constraints for all numeric ranges
- [ ] Create timezone-aware DateTime handling
- [ ] Build validation error handling and reporting
- [ ] Implement metric type routing and processing
- [ ] Add duplicate detection logic
- [ ] Test validation with real health export data

**Definition of Done:**  
- All health metric types validate correctly
- Validation errors provide helpful feedback
- DateTime handling works across timezones
- Duplicate detection prevents data conflicts
- Performance is adequate for batch processing

**Dependencies:** STORY-006B (Actix-web structure), health data samples  
**Related Issues:** Code Analyst - comprehensive health data models needed

---

### STORY-011B: Build Individual Transaction Processing
**Epic**: EPIC-003  
**Priority**: P2 (Path B Only)  
**Story Points**: 8  
**Type**: Implementation Story  

**Description:**  
As the Rust API, I need individual transaction processing per health metric so that partial failures don't affect other data and comprehensive error reporting is provided.

**Background:**  
- ARCHITECTURE.md specifies individual transactions per metric
- Previous Python implementation had this working
- Need SQLx transaction management with proper error handling
- Item-level error classification and reporting required

**Acceptance Criteria:**  
- [ ] Implement transaction isolation per metric:
  - Each health metric processed in separate transaction
  - Failure of one metric doesn't affect others
  - Database rollback on individual metric errors
- [ ] Build comprehensive error classification:
  - DuplicateEntry errors
  - ValidationError with details
  - DatabaseError handling
  - ProcessingError types
- [ ] Create item-level error reporting:
  - Track item_type, item_index for failed metrics
  - Provide detailed error messages and context
  - Include suggestions for fixing errors
- [ ] Implement async processing with proper error collection
- [ ] Add processing metrics and timing information
- [ ] Test with various failure scenarios

**Definition of Done:**  
- Individual metrics process in isolation
- Partial failures are handled gracefully
- Error reporting provides actionable information
- Processing performance remains acceptable
- Database consistency maintained under all conditions

**Dependencies:** STORY-010B (validation), STORY-005B (database)  
**Related Issues:** Architecture Reviewer - individual transaction pattern

---

### STORY-012B: Implement API Key Authentication in Rust
**Epic**: EPIC-004  
**Priority**: P2 (Path B Only)  
**Story Points**: 8  
**Type**: Feature Story  

**Description:**  
As the Rust API, I need secure API key authentication with Argon2 hashing and Redis caching so that only authorized clients can access the health data ingestion endpoint.

**Background:**  
- ARCHITECTURE.md provides detailed Rust authentication code examples
- Need Bearer token middleware with SQLx database lookups
- Argon2 hashing with specific parameters (19MB memory, 2 iterations)
- Redis caching for performance optimization

**Acceptance Criteria:**  
- [ ] Create Rust authentication structs (ApiKey, AuthenticatedUser)
- [ ] Implement Argon2 hashing with specified parameters:
  - 19MB memory usage
  - 2 iterations
  - Parallelism level 1
- [ ] Build Bearer token middleware:
  - Extract Authorization header
  - Hash incoming API key
  - Check Redis cache first (5-minute TTL)
  - Fallback to database lookup with SQLx
  - Update last_used_at asynchronously
- [ ] Create API key management:
  - Generate secure random API keys
  - Hash and store in database
  - Revoke API keys functionality
- [ ] Add comprehensive error handling for auth failures
- [ ] Test authentication performance under load

**Definition of Done:**  
- Bearer token authentication works correctly
- Argon2 hashing matches specification
- Redis caching improves performance significantly
- API key management is functional
- Security testing passes

**Dependencies:** STORY-005B (database), STORY-006B (Actix-web), Redis  
**Related Issues:** Architecture Reviewer - detailed authentication specification

---

## P3 Stories (Low Priority - Nice to Have)

### STORY-013: Performance Optimization and Caching
**Epic**: EPIC-010  
**Priority**: P3 (Both Paths)  
**Story Points**: 8  
**Type**: Performance Story  

**Description:**  
As the API, I need optimized database queries and comprehensive caching so that response times remain fast under high load and database resources are used efficiently.

**Background:**  
- ARCHITECTURE.md specifies various performance optimizations
- Need connection pooling, statement caching, query optimization
- Cache invalidation patterns for data consistency
- Performance monitoring and alerting

**Acceptance Criteria:**  
- [ ] Optimize database connection pooling:
  - Configure optimal pool sizes (5-20 connections)
  - Statement caching (100 statements)
  - Connection health checks and recycling
- [ ] Implement advanced caching patterns:
  - User summary caching (5-minute TTL)
  - Recent metrics caching (10-minute TTL)
  - Query result caching for common patterns
- [ ] Add cache warming for frequently accessed data
- [ ] Build cache invalidation patterns:
  - User-based invalidation
  - Metric-type-based invalidation  
  - Time-based cache expiry
- [ ] Optimize database queries:
  - Use BRIN indexes for time-series data
  - Implement partial indexes for recent data
  - Query plan analysis and optimization
- [ ] Add performance monitoring and alerting

**Definition of Done:**  
- Database queries are optimized and fast
- Caching reduces database load significantly
- Cache hit rates are high (>80%)
- Performance monitoring alerts on degradation
- System handles expected load comfortably

**Dependencies:** Core implementation (either path)  
**Related Issues:** Architecture Reviewer - comprehensive performance requirements

---

### STORY-014: Advanced Monitoring and Data Quality
**Epic**: EPIC-006  
**Priority**: P3 (Both Paths)  
**Story Points**: 13  
**Type**: Feature Story  

**Description:**  
As an operations team member, I need advanced monitoring including data quality checks and anomaly detection so that data issues are detected early and system health is maintained.

**Background:**  
- ARCHITECTURE.md includes comprehensive data quality monitoring code
- Need background tasks for monitoring missing syncs and anomalous values
- Alert manager integration for critical issues
- Data freshness and consistency checks

**Acceptance Criteria:**  
- [ ] Implement data quality monitoring:
  - Check for missing expected syncs (users with no recent data)
  - Detect anomalous health values (heart rate spikes, etc.)
  - Monitor data freshness across metric types
  - Track data consistency across partitions
- [ ] Build alert management system:
  - Integration with external alerting (email, Slack, PagerDuty)
  - Alert severity levels (critical, warning, info)
  - Alert deduplication and grouping
  - Alert acknowledgment and resolution tracking
- [ ] Create monitoring dashboards:
  - System health overview
  - Data quality metrics
  - User activity patterns
  - Performance trends
- [ ] Implement background monitoring tasks:
  - Scheduled data quality checks
  - Automatic partition maintenance
  - Cleanup of old audit logs and raw data
- [ ] Add monitoring for external dependencies:
  - Database health and performance
  - Redis availability and memory usage
  - API gateway and load balancer status

**Definition of Done:**  
- Data quality issues are detected automatically
- Alerts are sent for critical issues
- Monitoring dashboards provide clear system overview
- Background tasks maintain system health
- External dependency monitoring is comprehensive

**Dependencies:** Core implementation, monitoring infrastructure  
**Related Issues:** Architecture Reviewer - data quality monitoring specification

---

### STORY-015: Deployment Automation and CI/CD
**Epic**: EPIC-005  
**Priority**: P3 (Both Paths)  
**Story Points**: 13  
**Type**: Infrastructure Story  

**Description:**  
As a DevOps engineer, I need complete CI/CD pipeline with automated testing and deployment so that code changes are reliably tested and deployed to production environments.

**Background:**  
- ARCHITECTURE.md includes comprehensive GitHub Actions workflow  
- Need automated testing with PostgreSQL and Redis services
- Docker image building and registry pushing
- Kubernetes deployment automation

**Acceptance Criteria:**  
- [ ] Create GitHub Actions CI/CD pipeline:
  - Automated testing on pull requests
  - PostgreSQL and Redis services for integration tests
  - Code quality checks (linting, formatting)
  - Security scanning and dependency checks
- [ ] Build Docker containerization:
  - Multi-stage builds for minimal image size
  - Health checks and proper signal handling
  - Security scanning of container images
  - Registry pushing with proper tagging
- [ ] Implement Kubernetes deployment automation:
  - Automated deployment to staging environment
  - Production deployment with manual approval
  - Rolling updates with proper health checks
  - Rollback capabilities for failed deployments
- [ ] Add deployment monitoring:
  - Deployment success/failure tracking
  - Performance monitoring post-deployment
  - Automated smoke tests after deployment
  - Integration with monitoring and alerting
- [ ] Create environment management:
  - Staging environment that mirrors production
  - Development environment setup automation
  - Configuration management across environments

**Definition of Done:**  
- CI/CD pipeline runs automatically on code changes
- All tests pass before deployment
- Production deployments are safe and monitored
- Rollback process works when needed
- Multiple environments are properly managed

**Dependencies:** Core implementation, Kubernetes infrastructure  
**Related Issues:** Architecture Reviewer - comprehensive CI/CD specification

---

## Technical Debt Stories

### STORY-016: Clean Up Git History and Project Structure
**Epic**: EPIC-001  
**Priority**: P3  
**Story Points**: 3  
**Type**: Technical Debt  

**Description:**  
As a developer, I need a clean git history and properly organized project structure so that the codebase is maintainable and follows best practices.

**Background:**  
- .gitignore contains both Python and Rust patterns with duplicates
- Git history shows technology transition that wasn't properly documented
- Project structure needs organization based on chosen technology path

**Acceptance Criteria:**  
- [ ] Clean up .gitignore file:
  - Remove duplicate Python patterns (lines 182+)
  - Keep only patterns relevant to chosen technology stack
  - Add any missing patterns for development tools
- [ ] Organize project structure:
  - Remove unnecessary files and directories
  - Ensure consistent naming conventions
  - Add proper directory structure for chosen path
- [ ] Document git history decisions:
  - Add CHANGELOG.md documenting major changes
  - Document technology transition reasoning
  - Tag important commits for reference
- [ ] Set up development standards:
  - Code formatting configuration
  - Linting rules and pre-commit hooks
  - Contributing guidelines for new developers

**Definition of Done:**  
- .gitignore is clean and relevant
- Project structure follows conventions
- Development standards are documented and enforced
- Git history is properly documented

**Dependencies:** STORY-001 (tech decision affects cleanup scope)  
**Related Issues:** File Explorer - .gitignore confusion identified

---

## Epic Sizing Summary

| Epic | Path A Stories | Path B Stories | Cross-Cutting |
|------|----------------|----------------|---------------|
| **EPIC-001: Foundation** | 10 points | 10 points | 10 points |
| **EPIC-002: Python Enhancement** | 45 points | 0 points | 0 points |  
| **EPIC-003: Rust Implementation** | 0 points | 85 points | 0 points |
| **EPIC-004: Security** | 18 points | 16 points | 0 points |
| **EPIC-005: Infrastructure** | 5 points | 5 points | 31 points |
| **EPIC-006: Monitoring** | 13 points | 0 points | 29 points |
| **EPIC-007: Documentation** | 5 points | 0 points | 5 points |
| **EPIC-008: Database** | 8 points | 13 points | 5 points |
| **EPIC-009: Testing** | TBD | TBD | TBD |
| **EPIC-010: Performance** | 0 points | 0 points | 8 points |

**Path A Total**: ~104 story points + cross-cutting (~93) = **~197 points**  
**Path B Total**: ~129 story points + cross-cutting (~93) = **~222 points**

*Note: Testing stories (EPIC-009) need to be created based on chosen path*

---

## Dependencies Map

### Critical Path (Must Complete First)
1. **STORY-001**: Technology Decision → Enables all other work
2. **STORY-002**: Documentation Update → Corrects misleading information  
3. **STORY-003**: Essential Documentation → Basic project setup

### Path A Dependencies
- STORY-001 → STORY-004A (restoration) → STORY-005A (auth) → STORY-006A (rate limiting)
- STORY-004A → STORY-007A (caching) → STORY-011A (metrics)
- STORY-008 (database) → STORY-010A (schema alignment)

### Path B Dependencies  
- STORY-001 → STORY-004B (project) → STORY-005B (schema) → STORY-006B (Actix-web)
- STORY-006B → STORY-010B (validation) → STORY-011B (processing)  
- STORY-005B + STORY-006B → STORY-012B (authentication)

### Cross-Cutting Dependencies
- STORY-008 (database infrastructure) blocks most implementation work
- STORY-009 (monitoring) can be developed in parallel with core features
- Performance and deployment stories depend on core implementation completion

---

## Effort Estimation Notes

**Story Point Scale**: Fibonacci (1, 2, 3, 5, 8, 13, 21)  
- 1-2 points: Simple tasks, documentation updates, configuration  
- 3-5 points: Standard features, moderate complexity implementation
- 8-13 points: Complex features requiring multiple components
- 21+ points: Epic-level work that should be broken down further

**Velocity Assumptions**: 
- Single developer: ~10-15 points per sprint (2 weeks)
- Team of 2-3 developers: ~25-40 points per sprint
- Includes time for code review, testing, documentation

**Risk Factors**:
- Path A: Lower risk, building on known working implementation
- Path B: Higher risk, greenfield development with new technology stack
- Cross-cutting: Infrastructure setup can have unexpected complexity

---

## Next Steps

1. **Immediate**: Complete STORY-001 (technology decision)
2. **Phase 1**: Execute P0 stories (foundation and documentation)  
3. **Phase 2**: Execute P1 stories based on chosen path
4. **Phase 3**: P2 and P3 stories for feature completion and optimization

**Success Metrics**:
- All P0 and P1 stories completed successfully
- API functional with authentication, rate limiting, and basic monitoring
- Database schema optimized and performing well
- Deployment pipeline functional and reliable

This backlog provides a comprehensive roadmap for implementing the Health Export REST API according to architectural specifications while maintaining flexibility between technology stack options.