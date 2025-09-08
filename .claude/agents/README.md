# Health Export REST API - Specialized Agents

This directory contains specialized agent definitions for different components of the Rust-based health data ingestion API.

## Agent Overview

Each agent is designed to focus on a specific area of the technology stack, providing deep expertise and focused analysis for backlog generation and implementation guidance.

### 🗄️ Database Architect (`database-architect.md`)
**Specialization**: PostgreSQL, SQLx migrations, PostGIS integration, data modeling
- Health metrics table design and partitioning
- Database performance optimization with BRIN indexes
- PostGIS geospatial features for workout routes
- Connection pooling and async database operations

### 🌐 API Developer (`api-developer.md`) 
**Specialization**: Actix-web REST API, routing, middleware, HTTP handling
- `/v1/ingest` endpoint implementation
- Authentication and rate limiting middleware
- Request/response serialization with comprehensive validation
- OpenAPI documentation and error handling

### 🔒 Auth & Security Specialist (`auth-security-specialist.md`)
**Specialization**: API key authentication, rate limiting, audit logging, security
- Argon2 API key hashing with Redis caching
- Dual-strategy rate limiting (requests + bandwidth)
- Comprehensive audit trail with structured logging
- Security middleware and compliance considerations

### 📊 Data Processor (`data-processor.md`)
**Specialization**: Health data validation, transformation, processing logic
- Individual health metric models (HeartRate, BloodPressure, Sleep, Activity)
- Individual transaction processing with duplicate prevention
- Data validation and anomaly detection
- Error classification and comprehensive reporting

### ⚡ Redis Cache Specialist (`redis-cache-specialist.md`)
**Specialization**: Redis caching, session management, performance optimization
- API key validation caching with TTL management
- Rate limiting implementation with sliding windows
- User data caching with intelligent invalidation
- Redis clustering and high availability patterns

### 📈 Monitoring & Observability (`monitoring-observability.md`)
**Specialization**: Prometheus metrics, structured logging, health monitoring
- Request metrics collection and monitoring
- Structured JSON logging with tracing correlation
- Health check endpoints and data quality monitoring
- Grafana dashboards and alerting rules

### 🚀 DevOps & Deployment (`devops-deployment.md`)
**Specialization**: Kubernetes, Docker, CI/CD, infrastructure management  
- Kubernetes StatefulSet for PostgreSQL, Deployment for API
- Multi-stage Docker builds and security scanning
- GitHub Actions CI/CD pipelines
- Infrastructure as code and scaling strategies

### 🧪 Testing & QA (`testing-qa.md`)
**Specialization**: Testing strategies, quality assurance, test automation
- Unit, integration, and end-to-end test suites
- Performance and load testing with custom Rust tools
- Security testing and vulnerability assessments
- Test data management and coverage analysis

## Usage

### Individual Agent Analysis
Each agent can be used independently for focused analysis of their specialization area:

```bash
# Example: Analyze database requirements
claude-code --context "ARCHITECTURE.md,agents/database-architect.md" \
  --prompt "Analyze the database requirements and generate implementation tasks"
```

### Parallel Backlog Generation
Use the provided script to deploy all agents in parallel:

```bash
./scripts/generate-backlog.sh
```

This will create individual backlogs for each agent and a consolidated project backlog.

## Agent Responsibilities

### Core Principles
- **Specialization Focus**: Each agent only handles tasks within their domain
- **Architecture Alignment**: All agents follow the comprehensive ARCHITECTURE.md specification
- **Cross-Agent Coordination**: Agents identify dependencies and integration points
- **Production Readiness**: Focus on scalable, maintainable, and secure implementations

### Output Standards  
- **Actionable Tasks**: Specific implementation tasks with clear acceptance criteria
- **Story Point Estimation**: 1-21 scale for effort estimation
- **Priority Classification**: High/Medium/Low priority with rationale
- **Dependency Mapping**: Clear identification of cross-team dependencies
- **Risk Assessment**: Technical risks and mitigation strategies

## Implementation Phases

### Phase 1: Foundation (Database + Core API)
- Database Architect: Schema design and migrations
- API Developer: Basic endpoint structure and routing

### Phase 2: Security & Processing (Auth + Data)
- Auth & Security Specialist: Authentication and rate limiting
- Data Processor: Health metric validation and processing

### Phase 3: Performance & Caching (Redis + Monitoring)
- Redis Cache Specialist: Caching layer implementation
- Monitoring & Observability: Metrics and logging

### Phase 4: Production Readiness (DevOps + Testing)
- DevOps & Deployment: Container and orchestration setup
- Testing & QA: Comprehensive test suites and automation

## Cross-Team Dependencies

### Critical Path Dependencies
1. **Database Schema** → API Development → Data Processing
2. **Authentication Middleware** → All Protected Endpoints
3. **Redis Setup** → Caching & Rate Limiting
4. **Monitoring Infrastructure** → Production Deployment

### Integration Points
- **API ↔ Database**: SQLx integration and connection pooling  
- **API ↔ Auth**: Authentication middleware integration
- **Data ↔ Cache**: Cache invalidation on data updates
- **All ↔ Monitoring**: Metrics and logging integration

This agent-based approach ensures comprehensive coverage of all system components while maintaining clear separation of concerns and expertise domains.