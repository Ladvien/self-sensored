# Self-Sensored Health Export API

A production-ready REST API built in Rust for receiving and storing health data from the Auto Health Export iOS application. Designed for high data integrity, scalability, and comprehensive health metrics processing.

## 🚀 Features

- **Multi-Format Support**: Handles both standard and iOS Auto Health Export JSON formats seamlessly
- **Comprehensive Health Metrics**: Heart rate, blood pressure, sleep, activity, workouts, and 50+ HealthKit metric types
- **Robust Authentication**: API key-based authentication with Argon2 hashing and audit logging
- **High Performance**: Built with Rust and Actix-web for optimal performance and memory safety
- **Spatial Data Support**: PostGIS integration for workout route tracking and location data
- **Rate Limiting**: Configurable rate limiting to prevent abuse
- **Observability**: Structured logging with tracing, metrics, and monitoring
- **Data Integrity**: Individual transactions per metric with comprehensive validation
- **Raw Data Storage**: Preserves original payloads for data recovery and analysis

## 🏗 Architecture

### Tech Stack
- **Language**: Rust 2021 edition
- **Web Framework**: Actix-web 4.4
- **Database**: PostgreSQL 15+ with PostGIS extension
- **ORM**: SQLx with compile-time query verification
- **Cache**: Redis 7+ for session management and rate limiting
- **Authentication**: Custom API key middleware with Argon2 hashing
- **Logging**: Tracing with structured JSON output
- **Monitoring**: Prometheus metrics integration

### Key Components

```
src/
├── handlers/           # HTTP request handlers
│   ├── health.rs      # Health check endpoints
│   └── ingest.rs      # Main data ingestion logic
├── middleware/         # Request processing middleware
│   ├── auth.rs        # API key authentication
│   ├── rate_limit.rs  # Rate limiting
│   └── request_logger.rs # Request logging
├── models/            # Data structures and validation
│   ├── health_metrics.rs # Standard health metric models
│   ├── ios_models.rs  # iOS Auto Health Export compatibility
│   └── db.rs          # Database model definitions
├── services/          # Business logic layer
│   ├── auth.rs        # Authentication service
│   ├── health.rs      # Health metric processing
│   └── batch_processor.rs # Batch data processing
└── db/                # Database operations
    └── database.rs    # Connection pool management
```

## 📊 Supported Health Metrics

The API supports comprehensive health data from HealthKit and other sources:

### Core Metrics
- **Heart Rate**: Min/avg/max BPM with context (resting, exercise, recovery)
- **Blood Pressure**: Systolic/diastolic readings with pulse
- **Sleep**: Duration, efficiency, sleep stages (deep, REM, awake)
- **Activity**: Steps, distance, calories, active minutes, flights climbed
- **Workouts**: Type, duration, energy, heart rate, GPS routes

### Extended iOS HealthKit Metrics (50+ types)
- Blood oxygen saturation, respiratory rate, body temperature
- Heart rate variability, ECG readings
- Blood glucose, insulin delivery
- Hearing levels, environmental audio exposure
- Fall detection, walking steadiness
- And many more...

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ with Cargo
- PostgreSQL 15+ with PostGIS extension
- Redis 7+ (optional, for caching)

### Installation

1. **Clone the repository**
```bash
git clone https://github.com/yourusername/self-sensored.git
cd self-sensored
```

2. **Set up environment variables**
```bash
cp .env.example .env
# Edit .env with your database credentials and configuration
```

3. **Set up the database**
```bash
# Create database
createdb health_export_dev

# Install PostGIS extension
psql health_export_dev -c "CREATE EXTENSION IF NOT EXISTS postgis;"

# Run migrations
cargo install sqlx-cli
sqlx migrate run
```

4. **Build and run**
```bash
# Development mode
cargo run

# Production build
cargo build --release
./target/release/self-sensored
```

The API will be available at `http://localhost:8080`

### Environment Configuration

Key environment variables in `.env`:

```bash
# Database
DATABASE_URL=postgresql://username:password@localhost/health_export_dev
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
WORKERS=4

# Optional: Redis for caching
REDIS_URL=redis://localhost:6379

# Optional: Rate limiting
RATE_LIMIT_REQUESTS_PER_HOUR=100
RATE_LIMIT_BANDWIDTH_MB_PER_HOUR=10
```

## 📡 API Endpoints

### Health Check
```http
GET /health
```
Basic health check endpoint.

```http
GET /api/v1/status
```
Detailed API status with database connectivity.

### Data Ingestion
```http
POST /api/v1/ingest
Authorization: Bearer <your-api-key>
Content-Type: application/json
```

Accepts health data in two formats:

**Standard Format:**
```json
{
  "data": {
    "metrics": [
      {
        "type": "HeartRate",
        "recorded_at": "2024-01-15T10:30:00Z",
        "avg_bpm": 72,
        "source": "Apple Watch"
      }
    ],
    "workouts": []
  }
}
```

**iOS Auto Health Export Format:**
```json
{
  "data": {
    "metrics": [
      {
        "name": "heart_rate",
        "units": "count/min",
        "data": [
          {
            "date": "2024-01-15 10:30:00 +0000",
            "qty": 72,
            "source": "Apple Watch"
          }
        ]
      }
    ]
  }
}
```

### Authentication

Generate API keys using the admin interface or database:

```sql
INSERT INTO users (email, full_name) VALUES ('user@example.com', 'User Name');
INSERT INTO api_keys (user_id, name, key_hash) 
VALUES (
  (SELECT id FROM users WHERE email = 'user@example.com'),
  'iOS App',
  -- Hash of your API key using Argon2
  '$argon2id$v=19$m=4096,t=3,p=1$...'
);
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test test_connections    # Database connectivity
cargo test schema_test        # Database schema validation
cargo test auth_service_test  # Authentication tests

# Run tests with output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'
```

### Test Database Setup
```bash
# Create test database
createdb health_export_test
export TEST_DATABASE_URL=postgresql://username:password@localhost/health_export_test

# Run schema tests
cargo test schema_test
```

## 🐳 Docker Deployment

```bash
# Build image
docker build -t self-sensored:latest .

# Run with docker-compose
docker-compose up -d
```

Sample `docker-compose.yml`:
```yaml
version: '3.8'
services:
  api:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/health_export
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis
  
  db:
    image: postgis/postgis:15-3.3
    environment:
      POSTGRES_DB: health_export
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
  
  redis:
    image: redis:7-alpine
    
volumes:
  postgres_data:
```

## 📈 Monitoring & Observability

### Logging
The API uses structured logging with the `tracing` crate:

```bash
# View logs in development
RUST_LOG=debug cargo run

# Production logging to JSON
RUST_LOG=info cargo run 2>&1 | jq
```

### Metrics
Prometheus metrics are exposed at `/metrics` (when enabled):

- `health_export_ingest_total` - Total ingestion requests
- `health_export_ingest_errors_total` - Failed requests  
- `health_export_ingest_duration_seconds` - Request duration
- `health_export_metrics_processed_total` - Processed metrics count
- `health_export_active_users` - Active users (24h window)

### Health Checks
- `/health` - Basic liveness check
- `/api/v1/status` - Readiness check with database connectivity

## 🔒 Security

- **API Key Authentication**: Secure Argon2 hashed API keys
- **Rate Limiting**: Configurable per-key request and bandwidth limits  
- **Audit Logging**: All API access logged with IP, user agent, and metadata
- **Input Validation**: Comprehensive validation of all health metric data
- **SQL Injection Protection**: SQLx compile-time query verification
- **No Raw SQL**: All database queries use parameterized statements

## 🔧 Development

### Code Quality
```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Security audit
cargo audit

# Update dependencies
cargo update
```

### Database Migrations
```bash
# Create new migration
sqlx migrate add add_new_metric_type

# Run migrations
sqlx migrate run

# Revert last migration  
sqlx migrate revert
```

### Performance Testing
```bash
# Load testing with curl
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/v1/ingest \
    -H "Authorization: Bearer your-api-key" \
    -H "Content-Type: application/json" \
    -d @test_payload.json &
done
```

## 📝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make changes and add tests
4. Ensure all tests pass (`cargo test`)
5. Run code quality checks (`cargo fmt && cargo clippy`)
6. Commit changes (`git commit -m 'Add amazing feature'`)
7. Push to branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Support

- **Issues**: Report bugs and request features via [GitHub Issues](https://github.com/yourusername/self-sensored/issues)
- **Documentation**: Additional docs available in the `docs/` directory
- **Community**: Join discussions in GitHub Discussions

## 🙏 Acknowledgments

- Built for integration with [Auto Health Export](https://apps.apple.com/us/app/auto-health-export/id1115567069) iOS app
- Inspired by the need for open-source health data aggregation
- Thanks to the Rust community for excellent crates and tooling

---

**Note**: This API is designed for personal health data collection and analysis. Ensure compliance with relevant privacy regulations (HIPAA, GDPR, etc.) when deploying in production environments.