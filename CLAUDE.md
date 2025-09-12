# CLAUDE.md

This file provides context and guidelines for Claude Code when working with the Health Export REST API codebase.

## 🚨 CRITICAL RULES

- **ALWAYS** run tests before committing any changes
- **ALWAYS** validate API payloads against the Auto Health Export schema
- **NEVER** store sensitive data (API keys, passwords) in code - use environment variables and .env file
- **NEVER** commit .env files to version control - use .env.example for template and keep .env local only
- **ALWAYS** use transactions for database operations that modify multiple tables
- **NEVER** bypass rate limiting or authentication checks
- **ALWAYS** handle errors gracefully with proper error types and logging
- **NEVER** use unwrap() in production code - use proper error handling with ? operator
- **NEVER** use `panic()` or `unsafe`

## 🎯 PROJECT CONTEXT

**Purpose**: Production-ready REST API to receive and store health data from the Auto Health Export iOS application.

**Key Requirements**:
- High data integrity and reliability
- Scalable architecture supporting 10,000+ users
- Comprehensive observability and monitoring
- Open-source friendly deployment options
- HIPAA-compliant data handling practices

**Architecture Principles**:
- Fail gracefully, log comprehensively
- Individual transaction per metric for data integrity
- Store raw payloads for data recovery
- Cache aggressively but invalidate properly
- Monitor everything, alert on anomalies

## 🔧 TECH STACK

- **Language**: Rust (latest stable)
- **Web Framework**: Actix-web 4.x
- **Database**: PostgreSQL 15+ with PostGIS extension
- **ORM**: SQLx (compile-time checked queries)
- **Cache**: Redis 7+
- **Container**: Docker & Kubernetes
- **Monitoring**: Prometheus + Grafana
- **CI/CD**: GitHub Actions

## 📁 PROJECT STRUCTURE

```
self-sensored/
├── src/
│   ├── handlers/        # API endpoints and handlers
│   ├── middleware/      # Auth, request logging, rate limiting
│   ├── models/          # Database models and types (health_metrics, ios_models)
│   ├── services/        # Business logic layer (auth, batch processing)
│   ├── db/             # Database operations and connection pooling
│   └── main.rs         # Application entry point
├── schema.sql           # Database schema definition
├── tests/              # Integration and unit tests
├── CLAUDE.md           # Project documentation and guidelines
├── BACKLOG.md          # Project tasks and features
├── DONE.md             # Completed tasks
├── ARCHITECTURE.md     # System architecture documentation
└── Cargo.toml          # Rust dependencies
```

## 🛠 COMMANDS

### Development
```bash
# Start local development environment
docker-compose up -d

# Run the API server
cargo run

# Watch mode for development
cargo watch -x run

# Run with specific environment
RUST_ENV=development cargo run
```

### Database
```bash
# Initialize database with schema
psql -d health_export_dev < schema.sql

# Prepare offline query data for CI
cargo sqlx prepare
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_ingest_endpoint

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Check test coverage
cargo tarpaulin --out Html
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy linter
cargo clippy -- -D warnings

# Check for security vulnerabilities
cargo audit

# Update dependencies
cargo update
```

### Building & Deployment
```bash
# Build release version
cargo build --release

# Build Docker image
docker build -t health-export-api:latest .

# Run production checks
./scripts/pre-deploy-checks.sh

# Deploy to Kubernetes
kubectl apply -f k8s/
```

## 💾 DATABASE PATTERNS

### Query Patterns
```rust
// Always use parameterized queries with SQLx
let user = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;

// Use transactions for multi-table operations
let mut tx = pool.begin().await?;
// ... operations
tx.commit().await?;

// Check for duplicates before inserting
let exists = sqlx::query!(
    "SELECT EXISTS(SELECT 1 FROM heart_rate_metrics WHERE user_id = $1 AND recorded_at = $2)",
    user_id, recorded_at
)
.fetch_one(&pool)
.await?
.exists.unwrap_or(false);
```

### Partitioning
- Raw ingestions: Monthly partitions
- Health metrics: Monthly partitions by recorded_at
- Automatically create partitions 3 months ahead
- Use BRIN indexes for time-series data

### Batch Processing Configuration
PostgreSQL has a parameter limit of 65,535 per query. Our batch processor implements configurable chunking to stay under this limit.

**Environment Variable Configuration:**
```bash
# Batch Processing Settings
BATCH_MAX_RETRIES=3
BATCH_INITIAL_BACKOFF_MS=100
BATCH_MAX_BACKOFF_MS=5000
BATCH_ENABLE_PARALLEL=true
BATCH_MEMORY_LIMIT_MB=500.0

# Metric-Specific Chunk Sizes (Environment Configurable)
BATCH_HEART_RATE_CHUNK_SIZE=8000      # 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,000
BATCH_BLOOD_PRESSURE_CHUNK_SIZE=8000  # 6 params: 65,535 ÷ 6 × 0.8 ≈ 8,000  
BATCH_SLEEP_CHUNK_SIZE=6000           # 10 params: 65,535 ÷ 10 × 0.8 ≈ 6,000
BATCH_ACTIVITY_CHUNK_SIZE=6500        # 8 params: 65,535 ÷ 8 × 0.8 ≈ 6,500
BATCH_WORKOUT_CHUNK_SIZE=5000         # 10 params: 65,535 ÷ 10 × 0.8 ≈ 5,000

BATCH_ENABLE_PROGRESS_TRACKING=true   # Track progress for large batches
BATCH_ENABLE_DEDUPLICATION=true       # Enable intra-batch deduplication
```

**Code Usage:**
```rust
use crate::config::BatchConfig;

// Load configuration from environment variables
let config = BatchConfig::from_env();
config.validate()?; // Validates chunk sizes against PostgreSQL limits

let batch_processor = BatchProcessor::with_config(pool, config);
```

### Health Metric Validation Configuration
All validation thresholds are configurable via environment variables:

**Heart Rate Validation:**
```bash
VALIDATION_HEART_RATE_MIN=15        # Minimum BPM (physiological lower bound)
VALIDATION_HEART_RATE_MAX=300       # Maximum BPM (extreme athletic scenarios)
```

**Blood Pressure Validation:**
```bash
VALIDATION_SYSTOLIC_MIN=50          # Minimum systolic pressure (mmHg)
VALIDATION_SYSTOLIC_MAX=250         # Maximum systolic pressure (mmHg)
VALIDATION_DIASTOLIC_MIN=30         # Minimum diastolic pressure (mmHg)
VALIDATION_DIASTOLIC_MAX=150        # Maximum diastolic pressure (mmHg)
```

**Sleep Validation:**
```bash
VALIDATION_SLEEP_EFFICIENCY_MIN=0.0        # Minimum sleep efficiency (%)
VALIDATION_SLEEP_EFFICIENCY_MAX=100.0      # Maximum sleep efficiency (%)
VALIDATION_SLEEP_DURATION_TOLERANCE_MINUTES=60  # Sleep time calculation tolerance
```

**Activity Validation (Simplified Schema):**
```bash
VALIDATION_STEP_COUNT_MIN=0         # Minimum daily step count
VALIDATION_STEP_COUNT_MAX=200000    # Maximum daily step count (extreme scenarios)
VALIDATION_DISTANCE_MAX_KM=500.0    # Maximum daily distance (km)
VALIDATION_CALORIES_MAX=20000.0     # Maximum daily calories (extreme athletic events)
```

**GPS & Workout Validation:**
```bash
VALIDATION_LATITUDE_MIN=-90.0       # GPS latitude bounds
VALIDATION_LATITUDE_MAX=90.0
VALIDATION_LONGITUDE_MIN=-180.0     # GPS longitude bounds
VALIDATION_LONGITUDE_MAX=180.0
VALIDATION_WORKOUT_HEART_RATE_MIN=15    # Workout heart rate bounds
VALIDATION_WORKOUT_HEART_RATE_MAX=300
VALIDATION_WORKOUT_MAX_DURATION_HOURS=24  # Maximum workout duration (ultra events)
```

**Code Usage:**
```rust
use crate::config::ValidationConfig;

// Load validation configuration from environment
let validation_config = ValidationConfig::from_env();
validation_config.validate()?; // Ensures configuration consistency

// Use with health metrics
let metric = HeartRateMetric { /* ... */ };
metric.validate_with_config(&validation_config)?;
```

**Parameter Count per Metric Type (Simplified Schema):**
- **Heart Rate**: 7 params (user_id, recorded_at, heart_rate, resting_heart_rate, heart_rate_variability, context, source_device)
- **Blood Pressure**: 6 params (user_id, recorded_at, systolic, diastolic, pulse, source_device)
- **Sleep**: 9 params (user_id, sleep_start, sleep_end, duration_minutes, deep_sleep_minutes, rem_sleep_minutes, light_sleep_minutes, awake_minutes, efficiency, source_device)
- **Activity**: 7 params (user_id, recorded_at, step_count, distance_meters, flights_climbed, active_energy_burned_kcal, basal_energy_burned_kcal, source_device)
- **Workout**: 10 params (user_id, workout_type, started_at, ended_at, total_energy_kcal, active_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source_device)

**Chunking Benefits:**
- Prevents PostgreSQL parameter limit errors
- Maintains transaction integrity within each chunk
- Provides progress tracking for large batches
- Parallel processing of chunks when enabled
- Comprehensive logging of chunk processing

## 🌐 API GUIDELINES

### Endpoint Design
- Version all endpoints: `/v1/...`
- Use proper HTTP status codes
- Return detailed error responses
- Include processing metadata in responses
- Support batch operations with partial success

### Request Validation
```rust
// Validate with custom validators
#[derive(Deserialize, Validate)]
pub struct HeartRateMetric {
    #[validate(range(min = 20, max = 300))]
    pub bpm: u16,
    // ...
}
```

### Error Responses
```json
{
  "error": "validation_error",
  "message": "Heart rate value out of range",
  "details": {
    "field": "bpm",
    "value": 350,
    "valid_range": [20, 300]
  }
}
```

## 🧪 TESTING PRACTICES

### Test Organization
- Unit tests: In same file as code (`#[cfg(test)]`)
- Integration tests: In `tests/` directory
- Use test fixtures for consistent data
- Clean up test data after each test

### Test Database
```rust
// Use TEST_DATABASE_URL for integration tests
let test_db = std::env::var("TEST_DATABASE_URL")
    .expect("TEST_DATABASE_URL must be set");

// Always clean up test data
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();
}
```

## 🔒 SECURITY PATTERNS

### API Key Handling
```rust
// Hash API keys with Argon2
use argon2::{Argon2, PasswordHash, PasswordHasher};

// Never log raw API keys
tracing::info!(
    api_key_id = %api_key.id,
    "API key validated"
);
```

### Rate Limiting
- Request limit: 100/hour per API key
- Bandwidth limit: 10MB/hour per API key
- Use Redis for distributed rate limiting
- Return clear rate limit headers

## 📊 MONITORING & DEBUGGING

### Metrics to Track
- `health_export_ingest_total` - Total ingest requests
- `health_export_ingest_errors_total` - Failed requests
- `health_export_ingest_duration_seconds` - Request duration
- `health_export_metrics_processed_total` - Metrics processed
- `health_export_active_users` - Active users (24h)

### Logging Patterns
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(payload))]
pub async fn handle_ingest(
    user: AuthenticatedUser,
    payload: web::Json<IngestPayload>,
) -> Result<impl Responder> {
    info!(
        user_id = %user.id,
        metric_count = payload.data.metrics.len(),
        "Processing ingest request"
    );
    // ...
}
```

## 🎨 CODE STYLE

### Rust Conventions
- Use `snake_case` for functions and variables
- Use `PascalCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefer `&str` over `String` for function parameters
- Use `Result<T, E>` for fallible operations
- Document public APIs with `///` comments

### Error Handling
```rust
// Define specific error types
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
}

// Use ? operator for propagation
let user = get_user(user_id).await?;
```

## 🔄 COMMON WORKFLOWS

### Core Health Metric Types (Simplified Schema)
The API supports 5 core health metric types only:
1. **Heart Rate Metrics**: BPM, resting heart rate, heart rate variability, activity context
2. **Blood Pressure Metrics**: Systolic, diastolic, pulse measurements
3. **Sleep Metrics**: Sleep stages, duration, efficiency tracking
4. **Activity Metrics**: Steps, distance, energy burned, flights climbed
5. **Workout Metrics**: Exercise type, duration, heart rate, energy expenditure

### Adding a New Health Metric Type
1. Update schema.sql for new table
2. Add model in `src/models/`
3. Add validation rules
4. Update ingest handler
5. Add processing logic in services
6. Write integration tests
7. Update API documentation

### Debugging Failed Ingestion (Simplified Schema)
1. Check `raw_ingestions` table for payload using `payload_hash` and `raw_payload` fields
2. Review `processing_errors` JSONB field for detailed error information
3. Check `processing_status` field for current processing state
4. Check application logs for user_id
5. Verify rate limits aren't exceeded
6. Validate payload against simplified 5-table schema

### Performance Optimization
1. Run `EXPLAIN ANALYZE` on slow queries
2. Check for missing indexes
3. Review cache hit rates
4. Monitor connection pool usage
5. Profile with `cargo flamegraph`

## 📝 DEVELOPMENT TIPS

### Local Development Setup
```bash
# Copy environment template
cp .env.example .env

# Start PostgreSQL and Redis locally
# Using systemctl:
sudo systemctl start postgresql redis

# Or using homebrew on macOS:
brew services start postgresql
brew services start redis

# Create database
createdb health_export_dev

# Apply schema
psql -d health_export_dev < schema.sql

# Start development server
cargo run
```

### Quick Checks Before PR
1. `cargo fmt` - Format code
2. `cargo clippy -- -D warnings` - Lint
3. `cargo test` - Run tests
4. `cargo sqlx prepare --check` - Verify queries
5. Update CHANGELOG.md if needed

### Useful Aliases
```bash
alias cr='cargo run'
alias ct='cargo test'
alias cf='cargo fmt'
alias cc='cargo clippy -- -D warnings'
alias cw='cargo watch -x run'
```

## 🚀 DEPLOYMENT CHECKLIST

- [ ] All tests passing
- [ ] No clippy warnings
- [ ] Schema changes reviewed and tested
- [ ] Environment variables documented
- [ ] Kubernetes manifests updated
- [ ] Monitoring alerts configured
- [ ] API documentation updated
- [ ] CHANGELOG.md updated
- [ ] Performance benchmarks run
- [ ] Security scan completed

## 💡 REMEMBER

- This is a health data API - data integrity is paramount
- Every piece of data should be traceable and auditable
- Performance matters but correctness matters more
- When in doubt, log it and monitor it
- Keep the API simple and predictable for iOS app integration
- Always consider HIPAA compliance in design decisions