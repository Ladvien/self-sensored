# Health Export REST API

A robust REST API for receiving and storing comprehensive health data exported from the iOS Auto Health Export application. This system provides secure data ingestion, processing, and storage for personal health metrics with support for 15+ health data types.

## 🏗️ Project Status

**Current Phase**: Technology Migration & Implementation (Path A - Python Enhancement)

This project recently underwent a strategic technology evaluation, choosing to enhance the existing Python/FastAPI implementation rather than rewrite in Rust. See [MIGRATION.md](MIGRATION.md) for the complete technology evolution story.

## 🎯 Project Overview

The Health Export REST API is designed to:

- **Receive health data** from iOS Auto Health Export app via secure REST endpoints
- **Process and validate** 15+ health metric types with comprehensive data validation  
- **Store time-series data** in PostgreSQL with optimized partitioning and indexing
- **Provide secure access** through API key authentication with Argon2 hashing
- **Scale efficiently** with Redis caching and connection pooling
- **Monitor operations** with Prometheus metrics and structured logging

### Supported Health Data Types

- Heart Rate (min/avg/max values)
- Blood Pressure (systolic/diastolic)
- Sleep Analysis (duration, timing, stages)
- Activity Metrics (steps, distance, flights climbed)
- Workout Data with GPS routes (PostGIS support)
- Blood Glucose, ECG, and 10+ additional metric types

## 🛠️ Technology Stack

### Core Technologies
- **Language**: Python 3.13+
- **Web Framework**: FastAPI 0.115.12+
- **Database**: PostgreSQL 15+ with PostGIS extension
- **ORM**: SQLAlchemy 2.0+ with asyncpg
- **Cache**: Redis 7.0+
- **Authentication**: API keys with Argon2 hashing

### Development & Operations
- **Container**: Docker & Docker Compose
- **Process Manager**: Uvicorn with systemd integration
- **Monitoring**: Prometheus metrics, structured JSON logging
- **Database Migrations**: Alembic migrations
- **Testing**: Pytest with asyncio support

## 🚀 Quick Start

### Prerequisites

- Python 3.13+
- PostgreSQL 15+ with PostGIS extension
- Redis 7.0+
- Poetry (Python dependency management)

### Environment Setup

1. **Clone the repository**
```bash
git clone <repository-url>
cd self-sensored
```

2. **Install dependencies**
```bash
# Install Poetry (if not already installed)
curl -sSL https://install.python-poetry.org | python3 -

# Install project dependencies
poetry install
```

3. **Database Setup**
```bash
# Install PostgreSQL and PostGIS (Ubuntu/Debian)
sudo apt-get install postgresql-15 postgresql-15-postgis-3

# Create database and user
sudo -u postgres psql
CREATE DATABASE health_export;
CREATE USER health_user WITH PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE health_export TO health_user;

# Enable PostGIS extension
\c health_export
CREATE EXTENSION IF NOT EXISTS postgis;
\q
```

4. **Redis Setup**
```bash
# Install Redis (Ubuntu/Debian)
sudo apt-get install redis-server

# Start Redis service
sudo systemctl start redis-server
sudo systemctl enable redis-server
```

5. **Environment Configuration**
```bash
# Copy environment template
cp .env.example .env

# Edit configuration (see Configuration section below)
nano .env
```

6. **Database Migrations**
```bash
# Run database migrations
poetry run alembic upgrade head
```

7. **Start Development Server**
```bash
# Run the application
poetry run uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
```

The API will be available at `http://localhost:8000`

### Docker Development Setup

For a complete development environment with all services:

```bash
# Start all services (PostgreSQL, Redis, API)
docker-compose up -d

# View logs
docker-compose logs -f api

# Stop services
docker-compose down
```

## ⚙️ Configuration

The application uses environment-based configuration. Key settings in `.env`:

```bash
# Database Configuration
DATABASE_URL=postgresql+asyncpg://health_user:password@localhost:5432/health_export

# Redis Configuration
REDIS_URL=redis://localhost:6379/0

# Security Configuration
API_SECRET_KEY=your-super-secret-key-here
ARGON2_MEMORY_COST=19456  # 19MB memory for Argon2 hashing
ARGON2_TIME_COST=2        # 2 iterations
ARGON2_PARALLELISM=1      # Single thread

# Performance Configuration
DB_POOL_SIZE=20           # Base connection pool size
DB_MAX_OVERFLOW=30        # Maximum overflow connections
BATCH_SIZE=1000           # Records per batch for bulk processing

# Rate Limiting
RATE_LIMIT_REQUESTS=100   # Requests per hour per API key
RATE_LIMIT_BANDWIDTH=10485760  # 10MB per hour per API key

# Monitoring
LOG_LEVEL=INFO
STRUCTURED_LOGGING=true
PROMETHEUS_ENABLED=true
PROMETHEUS_PORT=9090
```

## 📖 API Documentation

### Core Endpoints

#### Health Check
```bash
GET /health
# Returns: {"status": "healthy", "database": "connected", "cache": "connected"}
```

#### Data Ingestion
```bash
POST /api/v1/sync
Headers: 
  Authorization: Bearer <your-api-key>
  Content-Type: application/json

# Payload: Complete health data export from iOS Auto Health Export
# Response: Processing summary with success/error counts per metric type
```

#### System Metrics
```bash
GET /api/v1/stats
# Returns: Database statistics, processing metrics, system health

GET /api/v1/metrics  
# Returns: Available metric types and their schemas

GET /metrics
# Returns: Prometheus metrics (monitoring endpoint)
```

### Authentication

All API endpoints (except health checks) require authentication:

```bash
# Request header format
Authorization: Bearer your-api-key-here
```

API keys are managed through the admin interface or direct database operations. Keys are hashed using Argon2 with the following parameters:
- Memory cost: 19MB
- Time cost: 2 iterations
- Parallelism: 1 thread

### Rate Limiting

Default limits per API key:
- **Requests**: 100 per hour
- **Bandwidth**: 10MB per hour

Exceeded limits return HTTP 429 with retry information.

## 🗄️ Database Schema

### Core Tables

- **users** - User account management
- **api_keys** - API key storage with Argon2 hashes
- **raw_ingestions** - Complete backup of incoming payloads (partitioned by month)
- **audit_log** - Action tracking and compliance (partitioned by month)

### Health Metrics Tables (All Partitioned)

- **heart_rate_metrics** - Heart rate min/avg/max data
- **blood_pressure_metrics** - Systolic/diastolic readings
- **sleep_metrics** - Sleep duration, timing, and stage data
- **activity_metrics** - Steps, distance, flights climbed
- **blood_glucose_metrics** - Blood sugar measurements
- **workout_data** - Exercise sessions with duration and energy
- **workout_routes** - GPS tracking data (PostGIS GEOGRAPHY type)

### Performance Features

- **Monthly Partitioning**: Time-series tables partitioned by month for optimal query performance
- **BRIN Indexes**: Block range indexes on timestamp columns for large table efficiency  
- **UUID Primary Keys**: All tables use UUID primary keys with `gen_random_uuid()`
- **Foreign Key Cascades**: Proper relationship management with cascade deletes
- **Unique Constraints**: Deduplication through composite unique constraints

## 🧪 Testing

### Running Tests

```bash
# Run all tests
poetry run pytest

# Run with coverage report
poetry run pytest --cov=app --cov-report=html

# Run specific test file
poetry run pytest tests/test_api.py

# Run integration tests (requires test database)
poetry run pytest tests/integration/
```

### Test Database Setup

Tests use a separate test database:

```bash
# Create test database
sudo -u postgres createdb health_export_test
sudo -u postgres psql health_export_test -c "CREATE EXTENSION IF NOT EXISTS postgis;"

# Set test environment variable
export TEST_DATABASE_URL=postgresql+asyncpg://health_user:password@localhost:5432/health_export_test
```

## 🚀 Deployment

### Production Requirements

- PostgreSQL 15+ with PostGIS extension
- Redis 7.0+ 
- Python 3.13+ runtime environment
- Reverse proxy (nginx recommended)
- SSL/TLS certificates
- Process manager (systemd service included)

### Docker Deployment

```bash
# Build production image
docker build -t health-export-api:latest .

# Run with docker-compose
docker-compose -f docker-compose.prod.yml up -d
```

### Systemd Service

A systemd service file is provided for production deployment:

```bash
# Copy service file
sudo cp example_daemon.service /etc/systemd/system/health-export-api.service

# Edit paths and configuration
sudo nano /etc/systemd/system/health-export-api.service

# Enable and start service
sudo systemctl enable health-export-api
sudo systemctl start health-export-api

# View logs
sudo journalctl -u health-export-api -f
```

### Database Performance Tuning

For production workloads, consider these PostgreSQL optimizations:

```sql
-- Connection settings
max_connections = 100
shared_buffers = 256MB
work_mem = 4MB

-- PostGIS and time-series optimizations  
random_page_cost = 1.1
effective_cache_size = 1GB
maintenance_work_mem = 64MB

-- Partitioning maintenance
pg_partman.bgw_enabled = on
```

## 📊 Monitoring

### Prometheus Metrics

The application exports comprehensive metrics on port 9090:

- Request counts, durations, and error rates
- Database connection pool statistics
- Processing metrics per health data type
- Cache hit rates and performance
- System resource utilization

### Structured Logging

All logs are output in JSON format with:

```json
{
  "timestamp": "2024-09-08T12:00:00Z",
  "level": "INFO", 
  "correlation_id": "uuid",
  "user_id": "uuid",
  "endpoint": "/api/v1/sync",
  "duration_ms": 1234,
  "records_processed": 150,
  "message": "Health data sync completed successfully"
}
```

### Health Checks

- **GET /health** - Application health with dependency checks
- **GET /ready** - Kubernetes readiness probe
- Database connectivity verification
- Redis cache connectivity verification

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines on:

- Development workflow and branch management
- Code style and formatting standards (Black, isort, flake8)
- Testing requirements and coverage expectations
- Pull request process and review criteria
- Documentation standards

### Quick Contribution Setup

```bash
# Fork and clone the repository
git clone https://github.com/your-username/self-sensored.git
cd self-sensored

# Create development branch
git checkout -b feature/your-feature-name

# Install development dependencies
poetry install --with dev

# Install pre-commit hooks
poetry run pre-commit install

# Make changes and run tests
poetry run pytest

# Submit pull request
```

## 🏗️ Architecture

This system follows a layered architecture pattern optimized for health data processing:

### Component Overview

1. **API Gateway Layer**: Rate limiting, SSL termination, request routing
2. **FastAPI Application**: REST endpoints, request validation, authentication
3. **Business Logic Layer**: Health data processing, validation, transformation
4. **Data Access Layer**: SQLAlchemy ORM, database operations, caching
5. **Storage Layer**: PostgreSQL with partitioning, Redis cache

### Key Design Patterns

- **Individual Transaction Processing**: Each health metric processed in isolation
- **Comprehensive Error Handling**: Item-level error reporting and recovery
- **Time-series Optimization**: Monthly partitioning with BRIN indexing
- **Cache-First Strategy**: Redis caching for frequently accessed data
- **Audit Trail**: Complete action logging for compliance and debugging

For detailed architectural information, see [ARCHITECTURE.md](ARCHITECTURE.md).

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: Check [ARCHITECTURE.md](ARCHITECTURE.md) for detailed system design
- **Issues**: Report bugs and feature requests via GitHub Issues
- **Migration Guide**: See [MIGRATION.md](MIGRATION.md) for technology evolution context
- **Contributing**: Read [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines

## 📈 Project Roadmap

### Current Sprint (Path A Enhancement)
- ✅ Strategic technology decision (Python enhancement chosen)
- ✅ Essential documentation creation
- 🔄 Python codebase restoration from git history
- 📋 API key authentication implementation
- 📋 Rate limiting and caching layer

### Upcoming Features
- Redis caching optimization
- Prometheus monitoring integration
- Kubernetes deployment configurations
- Performance optimization and scaling
- Advanced health data analytics

For the complete development backlog, see [BACKLOG.md](BACKLOG.md).

---

**Last Updated**: September 8, 2024
**Version**: 1.0.0 (Path A Enhancement)
**Status**: Active Development