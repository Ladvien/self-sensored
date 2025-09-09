# Health Export REST API Architecture Document

## Executive Summary

A production-ready REST API built with Rust, Actix-web, PostgreSQL, and SQLx to receive and store health data from the Auto Health Export iOS application. The system emphasizes data integrity, scalability, and observability while maintaining simplicity for open-source deployment.

## Technology Stack

### Core Components
- **Language**: Rust
- **Web Framework**: Actix-web 4.x
- **Database**: PostgreSQL 15+ with PostGIS extension
- **ORM/Query Builder**: SQLx
- **Cache**: Redis
- **Container Orchestration**: Kubernetes
- **Cloud Platforms**: AWS/GCP (self-hosted option available)

### Supporting Tools
- **Monitoring**: Prometheus + Grafana
- **Logging**: Structured JSON logs (stdout) → Datadog/CloudWatch
- **CI/CD**: GitHub Actions
- **Data Transformation**: dbt (separate project)
- **API Documentation**: OpenAPI 3.0

## System Architecture

### High-Level Design

```
┌─────────────────┐
│  Auto Health    │
│  Export App     │
└────────┬────────┘
         │ HTTPS + API Key
         ▼
┌─────────────────┐
│   API Gateway   │ ← Rate Limiting
│   (Ingress)     │ ← SSL Termination
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│      Actix-web Service          │
│  ┌─────────────────────────┐    │
│  │   /v1/ingest Endpoint   │    │
│  └─────────────────────────┘    │
│  ┌─────────────────────────┐    │
│  │   Validation Layer      │    │
│  └─────────────────────────┘    │
│  ┌─────────────────────────┐    │
│  │   Business Logic        │    │
│  └─────────────────────────┘    │
└────────┬───────────┬────────────┘
         │           │
         ▼           ▼
┌──────────────┐ ┌────────────┐
│  PostgreSQL  │ │   Redis    │
│  + PostGIS   │ │   Cache    │
└──────────────┘ └────────────┘
```

## Database Design

### Schema Structure

```sql
-- Core Tables
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(100),
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    CONSTRAINT active_key CHECK (revoked_at IS NULL OR revoked_at > created_at)
);

-- Raw data backup
CREATE TABLE raw_ingestions (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID NOT NULL REFERENCES api_keys(id),
    payload JSONB NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    processing_errors JSONB
) PARTITION BY RANGE (received_at);

-- Create monthly partitions for raw data
CREATE TABLE raw_ingestions_2025_01 PARTITION OF raw_ingestions 
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

-- Audit trail
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id UUID NOT NULL,
    api_key_id UUID,
    action VARCHAR(50) NOT NULL,
    resource_type VARCHAR(50),
    resource_id VARCHAR(100),
    metadata JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);
```

### Health Metrics Tables (Type-Specific)

```sql
-- Heart Rate Data
CREATE TABLE heart_rate_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    min_bpm SMALLINT,
    avg_bpm SMALLINT,
    max_bpm SMALLINT,
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at),
    UNIQUE (user_id, recorded_at)
) PARTITION BY RANGE (recorded_at);

CREATE INDEX idx_heart_rate_user_time 
    ON heart_rate_metrics USING BRIN (user_id, recorded_at);

-- Blood Pressure
CREATE TABLE blood_pressure_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    systolic SMALLINT NOT NULL,
    diastolic SMALLINT NOT NULL,
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at)
) PARTITION BY RANGE (recorded_at);

-- Sleep Analysis
CREATE TABLE sleep_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    asleep_duration_minutes INTEGER,
    in_bed_duration_minutes INTEGER,
    sleep_start TIMESTAMPTZ,
    sleep_end TIMESTAMPTZ,
    in_bed_start TIMESTAMPTZ,
    in_bed_end TIMESTAMPTZ,
    sleep_source VARCHAR(100),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, date)
) PARTITION BY RANGE (date);

-- Steps and Activity
CREATE TABLE activity_metrics (
    id BIGSERIAL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- 'steps', 'distance', 'flights_climbed', etc.
    value NUMERIC(10,2) NOT NULL,
    unit VARCHAR(20) NOT NULL,
    source VARCHAR(50),
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recorded_at, metric_type)
) PARTITION BY RANGE (recorded_at);

CREATE INDEX idx_activity_user_type_time 
    ON activity_metrics USING BRIN (user_id, metric_type, recorded_at);
```

### Workout Tables with PostGIS

```sql
-- Enable PostGIS
CREATE EXTENSION IF NOT EXISTS postgis;

-- Workouts table
CREATE TABLE workouts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workout_type VARCHAR(50) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    duration_seconds INTEGER GENERATED ALWAYS AS 
        (EXTRACT(EPOCH FROM (end_time - start_time))) STORED,
    total_energy_kcal NUMERIC(8,2),
    active_energy_kcal NUMERIC(8,2),
    distance_meters NUMERIC(10,2),
    avg_heart_rate SMALLINT,
    max_heart_rate SMALLINT,
    step_count INTEGER,
    raw_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT workout_time_check CHECK (end_time > start_time),
    UNIQUE (user_id, start_time)
);

CREATE INDEX idx_workouts_user_time 
    ON workouts USING BRIN (user_id, start_time);

-- Workout GPS routes
CREATE TABLE workout_routes (
    id BIGSERIAL PRIMARY KEY,
    workout_id UUID NOT NULL REFERENCES workouts(id) ON DELETE CASCADE,
    point_order INTEGER NOT NULL,
    location GEOGRAPHY(POINT, 4326) NOT NULL,
    altitude_meters NUMERIC(7,2),
    recorded_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workout_id, point_order)
);

CREATE INDEX idx_workout_routes_geography 
    ON workout_routes USING GIST (location);

CREATE INDEX idx_workout_routes_workout 
    ON workout_routes (workout_id, point_order);
```

## API Design

### Endpoint Structure

```yaml
openapi: 3.0.0
info:
  title: Health Export API
  version: 1.0.0

paths:
  /v1/ingest:
    post:
      summary: Ingest health data from Auto Health Export
      security:
        - ApiKeyAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                data:
                  type: object
                  properties:
                    metrics:
                      type: array
                      maxItems: 10000
                    workouts:
                      type: array
                      maxItems: 10000
      responses:
        '200':
          description: Detailed processing results
          content:
            application/json:
              schema:
                type: object
                properties:
                  success:
                    type: boolean
                  processed:
                    type: object
                    properties:
                      metrics:
                        type: integer
                      workouts:
                        type: integer
                  errors:
                    type: array
                    items:
                      type: object
                      properties:
                        item_type:
                          type: string
                        item_index:
                          type: integer
                        error:
                          type: string
                        details:
                          type: object
                  warnings:
                    type: array
                  processing_time_ms:
                    type: integer
```

### Response Examples

#### Success Response
```json
{
  "success": true,
  "processed": {
    "metrics": 147,
    "workouts": 2
  },
  "errors": [
    {
      "item_type": "metric",
      "item_index": 23,
      "error": "duplicate_entry",
      "details": {
        "metric_type": "heart_rate",
        "timestamp": "2025-01-15T14:30:00Z",
        "message": "Heart rate data for this timestamp already exists"
      }
    }
  ],
  "warnings": [
    {
      "item_type": "metric",
      "item_index": 45,
      "warning": "suspicious_value",
      "details": {
        "metric_type": "heart_rate",
        "value": 250,
        "message": "Heart rate value unusually high but accepted"
      }
    }
  ],
  "processing_time_ms": 342
}
```

## Authentication & Security

### API Key Management

**Design Decision**: The authentication system supports dual API key formats to ensure compatibility with the Auto Health Export iOS application while maintaining security for internally generated keys.

#### Supported API Key Formats

1. **Auto Health Export Format (UUID-based)**
   - The Auto Health Export app sends API keys as UUIDs directly in the Bearer token
   - Example: `Bearer 2d56f485-85bc-4337-839d-9b08a6626baf`
   - These are looked up directly by UUID in the database
   - No hashing is performed for Auto Export keys as they use their own key management

2. **Internal Format (Hashed)**
   - Keys generated by our system use the prefix `hea_` followed by a UUID
   - Example: `hea_550e8400e29b41d4a716446655440000`
   - These keys are hashed using Argon2 before storage for security
   - The hash is verified during authentication

```rust
// API Key structure
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,  // Either UUID for Auto Export or Argon2 hash for internal keys
    pub name: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

// Authentication flow supports both formats
pub async fn authenticate(&self, api_key: &str) -> Result<AuthContext, AuthError> {
    // Check if the API key is a UUID (Auto Export format)
    if let Ok(api_key_uuid) = Uuid::parse_str(api_key) {
        // Direct UUID lookup for Auto Export compatibility
        let api_key_record = sqlx::query!(
            "SELECT * FROM api_keys WHERE id = $1 AND is_active = true",
            api_key_uuid
        )
        .fetch_optional(&db_pool)
        .await?;
        
        if let Some(record) = api_key_record {
            // Validate and return auth context
            return Ok(create_auth_context(record));
        }
    }
    
    // If not a UUID, check against hashed keys (for our generated keys)
    if api_key.starts_with("hea_") {
        // Verify against Argon2 hashed keys in database
        let api_keys = fetch_active_hashed_keys().await?;
        
        for row in api_keys {
            if verify_argon2_hash(api_key, &row.key_hash)? {
                return Ok(create_auth_context(row));
            }
        }
    }
    
    Err(AuthError::InvalidApiKey)
}
```

#### Security Considerations

- **Auto Export Keys**: Trust the iOS app's key generation and management
- **Internal Keys**: Use Argon2 hashing with secure salt generation
- **Rate Limiting**: Applied equally to both key types
- **Audit Logging**: All authentication attempts are logged with key type identified
- **Key Rotation**: Supported through the `expires_at` field
- **Cache Strategy**: Both key types are cached in Redis after successful authentication

## Rate Limiting

### Dual Strategy Implementation

```rust
pub struct RateLimiter {
    request_limit: u32,      // 100 requests per hour
    bandwidth_limit: usize,  // 10MB per hour
    window: Duration,        // 1 hour sliding window
}

// Redis-backed rate limiting
async fn check_rate_limit(
    api_key_id: &Uuid,
    payload_size: usize,
) -> Result<(), RateLimitError> {
    let key_prefix = format!("rate_limit:{}", api_key_id);
    
    // Check request count
    let request_key = format!("{}_requests", key_prefix);
    let request_count: u32 = redis.incr(&request_key).await?;
    if request_count == 1 {
        redis.expire(&request_key, 3600).await?;
    }
    
    // Check bandwidth
    let bandwidth_key = format!("{}_bandwidth", key_prefix);
    let current_bandwidth: usize = redis.get(&bandwidth_key).await?.unwrap_or(0);
    
    if request_count > 100 {
        return Err(RateLimitError::RequestLimitExceeded);
    }
    
    if current_bandwidth + payload_size > 10_485_760 { // 10MB
        return Err(RateLimitError::BandwidthLimitExceeded);
    }
    
    redis.incrby(&bandwidth_key, payload_size).await?;
    redis.expire(&bandwidth_key, 3600).await?;
    
    Ok(())
}
```

## Data Processing Pipeline

### Validation Layer

```rust
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct HeartRateMetric {
    #[validate(range(min = 20, max = 300))]
    pub min: Option<u16>,
    #[validate(range(min = 20, max = 300))]
    pub avg: Option<u16>,
    #[validate(range(min = 20, max = 300))]
    pub max: Option<u16>,
    pub date: DateTime<Utc>,
}

pub async fn process_ingest(
    user_id: Uuid,
    payload: IngestPayload,
) -> ProcessingResult {
    let mut results = ProcessingResult::new();
    
    // Store raw payload first (always succeeds)
    let raw_id = store_raw_payload(&user_id, &payload).await?;
    
    // Process metrics with individual transactions
    for (index, metric) in payload.data.metrics.iter().enumerate() {
        match process_single_metric(&user_id, metric).await {
            Ok(()) => results.increment_processed_metrics(),
            Err(e) => {
                results.add_error(ItemError {
                    item_type: "metric",
                    item_index: index,
                    error: classify_error(&e),
                    details: error_details(&e),
                });
            }
        }
    }
    
    // Process workouts
    for (index, workout) in payload.data.workouts.iter().enumerate() {
        match process_single_workout(&user_id, workout).await {
            Ok(()) => results.increment_processed_workouts(),
            Err(e) => {
                results.add_error(ItemError {
                    item_type: "workout",
                    item_index: index,
                    error: classify_error(&e),
                    details: error_details(&e),
                });
            }
        }
    }
    
    results
}

async fn process_single_metric(
    user_id: &Uuid,
    metric: &HealthMetric,
) -> Result<(), ProcessingError> {
    // Individual transaction per metric
    let mut tx = db_pool.begin().await?;
    
    // Check for duplicates
    let exists = check_duplicate(&mut tx, user_id, metric).await?;
    if exists {
        return Err(ProcessingError::DuplicateEntry);
    }
    
    // Route to appropriate table based on metric type
    match metric {
        HealthMetric::HeartRate(hr) => {
            insert_heart_rate(&mut tx, user_id, hr).await?;
        }
        HealthMetric::BloodPressure(bp) => {
            insert_blood_pressure(&mut tx, user_id, bp).await?;
        }
        HealthMetric::Sleep(sleep) => {
            insert_sleep_metrics(&mut tx, user_id, sleep).await?;
        }
        // ... other metric types
    }
    
    tx.commit().await?;
    
    // Invalidate relevant caches
    invalidate_user_caches(user_id, &metric.metric_type()).await?;
    
    Ok(())
}
```

## Caching Strategy

### Redis Cache Layer

```rust
pub struct CacheManager {
    redis_pool: RedisPool,
    default_ttl: Duration,
}

impl CacheManager {
    // Cache recent summaries
    pub async fn cache_user_summary(
        &self,
        user_id: &Uuid,
        summary: &UserHealthSummary,
    ) -> Result<()> {
        let key = format!("summary:{}", user_id);
        let value = serde_json::to_string(summary)?;
        
        self.redis_pool
            .set_ex(&key, &value, 300) // 5 minute TTL
            .await?;
        
        Ok(())
    }
    
    // Cache recent metrics for quick access
    pub async fn cache_recent_metrics(
        &self,
        user_id: &Uuid,
        metric_type: &str,
        data: &[MetricData],
    ) -> Result<()> {
        let key = format!("recent:{}:{}", user_id, metric_type);
        let value = serde_json::to_string(data)?;
        
        self.redis_pool
            .set_ex(&key, &value, 600) // 10 minute TTL
            .await?;
        
        Ok(())
    }
    
    // Invalidation patterns
    pub async fn invalidate_user_caches(
        &self,
        user_id: &Uuid,
        metric_type: Option<&str>,
    ) -> Result<()> {
        let patterns = match metric_type {
            Some(mt) => vec![
                format!("summary:{}", user_id),
                format!("recent:{}:{}", user_id, mt),
            ],
            None => vec![
                format!("summary:{}", user_id),
                format!("recent:{}:*", user_id),
            ],
        };
        
        for pattern in patterns {
            self.redis_pool.del(&pattern).await?;
        }
        
        Ok(())
    }
}
```

## Monitoring & Observability

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, IntGauge, register_counter, register_histogram};

lazy_static! {
    static ref INGEST_REQUESTS: Counter = register_counter!(
        "health_export_ingest_total",
        "Total number of ingest requests"
    ).unwrap();
    
    static ref INGEST_ERRORS: Counter = register_counter!(
        "health_export_ingest_errors_total",
        "Total number of ingest errors"
    ).unwrap();
    
    static ref INGEST_DURATION: Histogram = register_histogram!(
        "health_export_ingest_duration_seconds",
        "Ingest request duration"
    ).unwrap();
    
    static ref METRICS_PROCESSED: Counter = register_counter!(
        "health_export_metrics_processed_total",
        "Total number of health metrics processed"
    ).unwrap();
    
    static ref ACTIVE_USERS: IntGauge = register_int_gauge!(
        "health_export_active_users",
        "Number of active users in last 24 hours"
    ).unwrap();
}

// Middleware for metrics
pub async fn metrics_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let start = Instant::now();
    
    INGEST_REQUESTS.inc();
    
    let result = next.call(req).await;
    
    let duration = start.elapsed().as_secs_f64();
    INGEST_DURATION.observe(duration);
    
    if result.is_err() {
        INGEST_ERRORS.inc();
    }
    
    result
}
```

### Structured Logging

```rust
use tracing::{info, warn, error, instrument};
use serde_json::json;

#[instrument(skip(payload))]
pub async fn handle_ingest(
    user: AuthenticatedUser,
    payload: web::Json<IngestPayload>,
) -> Result<impl Responder> {
    let payload_size = calculate_payload_size(&payload);
    
    info!(
        user_id = %user.id,
        payload_size = payload_size,
        metric_count = payload.data.metrics.len(),
        workout_count = payload.data.workouts.len(),
        "Processing ingest request"
    );
    
    let result = process_ingest(user.id, payload.into_inner()).await;
    
    if !result.errors.is_empty() {
        warn!(
            user_id = %user.id,
            error_count = result.errors.len(),
            errors = ?result.errors,
            "Ingest completed with errors"
        );
    }
    
    // Audit log
    audit_log(AuditEntry {
        user_id: user.id,
        action: "data_ingest",
        metadata: json!({
            "metrics_processed": result.processed.metrics,
            "workouts_processed": result.processed.workouts,
            "errors": result.errors.len(),
        }),
    }).await;
    
    Ok(HttpResponse::Ok().json(result))
}
```

### Data Quality Monitoring

```rust
pub struct DataQualityMonitor {
    db_pool: PgPool,
    alert_manager: AlertManager,
}

impl DataQualityMonitor {
    // Run as background task
    pub async fn monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        
        loop {
            interval.tick().await;
            
            // Check for missing expected syncs
            self.check_missing_syncs().await;
            
            // Detect anomalous values
            self.check_anomalous_values().await;
            
            // Monitor data freshness
            self.check_data_freshness().await;
        }
    }
    
    async fn check_missing_syncs(&self) {
        let query = r#"
            SELECT u.id, u.email, MAX(r.received_at) as last_sync
            FROM users u
            LEFT JOIN raw_ingestions r ON u.id = r.user_id
            WHERE r.received_at < NOW() - INTERVAL '48 hours'
            GROUP BY u.id, u.email
        "#;
        
        let stale_users = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(query)
            .fetch_all(&self.db_pool)
            .await
            .unwrap();
        
        for (user_id, email, last_sync) in stale_users {
            self.alert_manager.send_alert(Alert {
                severity: AlertSeverity::Warning,
                title: "Missing data sync",
                message: format!("User {} hasn't synced since {}", email, last_sync),
                metadata: json!({
                    "user_id": user_id,
                    "last_sync": last_sync,
                }),
            }).await;
        }
    }
}
```

## Infrastructure & Deployment

### Kubernetes Configuration

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: health-export-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: health-export-api
  template:
    metadata:
      labels:
        app: health-export-api
    spec:
      containers:
      - name: api
        image: health-export-api:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: health-export-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: health-export-secrets
              key: redis-url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: health-export-api-service
spec:
  selector:
    app: health-export-api
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### Environment Configuration

```toml
# config/production.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
max_connections = 20
min_connections = 5
connect_timeout = 10
idle_timeout = 300
max_lifetime = 3600

[redis]
pool_size = 10
timeout = 5

[rate_limiting]
requests_per_hour = 100
bandwidth_per_hour_mb = 10

[monitoring]
metrics_port = 9090
log_level = "info"

[security]
api_key_hash_memory = 19456  # 19 MiB
api_key_hash_iterations = 2
api_key_hash_parallelism = 1
```

## Testing Strategy

### Integration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use sqlx::postgres::PgPoolOptions;
    
    #[actix_rt::test]
    async fn test_ingest_endpoint() {
        // Setup test database
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set");
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("Failed to connect to test database");
        
        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
        
        // Create test app
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .service(routes::ingest)
        ).await;
        
        // Create test user and API key
        let user_id = create_test_user(&pool).await;
        let api_key = create_test_api_key(&pool, user_id).await;
        
        // Test payload
        let payload = json!({
            "data": {
                "metrics": [
                    {
                        "name": "heart_rate",
                        "data": [{
                            "date": "2025-01-15 14:30:00 +0000",
                            "Min": 55,
                            "Avg": 72,
                            "Max": 95
                        }]
                    }
                ],
                "workouts": []
            }
        });
        
        // Make request
        let req = test::TestRequest::post()
            .uri("/v1/ingest")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .set_json(&payload)
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), 200);
        
        // Verify data was stored
        let stored_metrics = sqlx::query!(
            "SELECT * FROM heart_rate_metrics WHERE user_id = $1",
            user_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        
        assert_eq!(stored_metrics.len(), 1);
        assert_eq!(stored_metrics[0].avg_bpm, Some(72));
        
        // Cleanup
        cleanup_test_data(&pool, user_id).await;
    }
}
```

## CI/CD Pipeline

### GitHub Actions Workflow

```yaml
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  SQLX_VERSION: 0.7.3
  SQLX_FEATURES: "runtime-tokio-rustls,postgres"

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgis/postgis:15-3.3
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: health_export_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
      
      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: rustfmt, clippy
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Install sqlx-cli
      run: |
        cargo install sqlx-cli \
          --version=${{ env.SQLX_VERSION }} \
          --features=${{ env.SQLX_FEATURES }} \
          --no-default-features \
          --locked
    
    - name: Run migrations
      run: |
        export DATABASE_URL=postgres://postgres:postgres@localhost/health_export_test
        sqlx migrate run
    
    - name: Run tests
      run: |
        export TEST_DATABASE_URL=postgres://postgres:postgres@localhost/health_export_test
        export REDIS_URL=redis://localhost:6379
        cargo test --all-features
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check

  build:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Build Docker image
      run: |
        docker build -t health-export-api:${{ github.sha }} .
    
    - name: Push to registry
      run: |
        echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
        docker tag health-export-api:${{ github.sha }} ${{ secrets.DOCKER_REGISTRY }}/health-export-api:latest
        docker push ${{ secrets.DOCKER_REGISTRY }}/health-export-api:latest
```

## Migration Strategy

### SQLx Migration Files

```sql
-- migrations/001_initial_schema.sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "postgis";

-- Core tables
CREATE TABLE users (...);
CREATE TABLE api_keys (...);

-- migrations/002_health_metrics.sql
CREATE TABLE heart_rate_metrics (...);
CREATE TABLE blood_pressure_metrics (...);

-- migrations/003_partitioning.sql
-- Create partitioning functions
CREATE OR REPLACE FUNCTION create_monthly_partitions()
RETURNS void AS $$
DECLARE
    start_date date;
    end_date date;
    partition_name text;
BEGIN
    FOR i IN 0..12 LOOP
        start_date := date_trunc('month', CURRENT_DATE) + (i || ' months')::interval;
        end_date := start_date + '1 month'::interval;
        partition_name := 'raw_ingestions_' || to_char(start_date, 'YYYY_MM');
        
        EXECUTE format('
            CREATE TABLE IF NOT EXISTS %I PARTITION OF raw_ingestions
            FOR VALUES FROM (%L) TO (%L)',
            partition_name, start_date, end_date
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Run partition creation
SELECT create_monthly_partitions();
```

## Performance Optimizations

### Database Indexes

```sql
-- BRIN indexes for time-series data
CREATE INDEX idx_heart_rate_time_brin 
    ON heart_rate_metrics USING BRIN (recorded_at);

CREATE INDEX idx_activity_time_brin 
    ON activity_metrics USING BRIN (recorded_at);

-- B-tree for frequent lookups
CREATE INDEX idx_api_keys_hash 
    ON api_keys USING BTREE (key_hash) 
    WHERE revoked_at IS NULL;

-- GiST for spatial queries
CREATE INDEX idx_workout_routes_geo 
    ON workout_routes USING GIST (location);

-- Partial indexes for common queries
CREATE INDEX idx_recent_heart_rate 
    ON heart_rate_metrics (user_id, recorded_at DESC) 
    WHERE recorded_at > NOW() - INTERVAL '7 days';
```

### Connection Pooling

```rust
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};

pub async fn create_db_pool(config: &DatabaseConfig) -> Result<PgPool> {
    let options = PgConnectOptions::from_str(&config.url)?
        .application_name("health-export-api")
        .statement_cache_capacity(100);
    
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout))
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .max_lifetime(Duration::from_secs(config.max_lifetime))
        .connect_with(options)
        .await
}
```

## Error Handling

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Duplicate entry: {resource} at {timestamp}")]
    DuplicateEntry {
        resource: String,
        timestamp: DateTime<Utc>,
    },
    
    #[error("Rate limit exceeded: {0}")]
    RateLimit(#[from] RateLimitError),
    
    #[error("Authentication failed")]
    Authentication,
    
    #[error("Internal server error")]
    Internal,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::Validation(msg) => {
                HttpResponse::BadRequest().json(json!({
                    "error": "validation_error",
                    "message": msg,
                    "details": {
                        "help": "Please check the data format matches the Auto Health Export specification"
                    }
                }))
            }
            ApiError::DuplicateEntry { resource, timestamp } => {
                HttpResponse::Conflict().json(json!({
                    "error": "duplicate_entry",
                    "message": format!("Data for {} at {} already exists", resource, timestamp),
                    "details": {
                        "help": "Use unique timestamps or check your sync settings",
                        "resource": resource,
                        "timestamp": timestamp
                    }
                }))
            }
            ApiError::RateLimit(e) => {
                HttpResponse::TooManyRequests().json(json!({
                    "error": "rate_limit_exceeded",
                    "message": e.to_string(),
                    "retry_after": 3600
                }))
            }
            _ => HttpResponse::InternalServerError().json(json!({
                "error": "internal_error",
                "message": "An unexpected error occurred"
            }))
        }
    }
}
```

## Future Enhancements

### Planned Features

1. **WebSocket Support**: Real-time data streaming for continuous monitoring devices
2. **GraphQL API**: Flexible querying for dashboard applications
3. **Data Export**: Batch export functionality for data portability
4. **Webhook Notifications**: Alert external systems on specific health events
5. **Multi-region Support**: Geographic data replication for compliance
6. **Advanced Analytics**: Built-in trend analysis and anomaly detection
7. **OAuth 2.0**: Support for third-party app integrations
8. **Compression**: Support for gzip/brotli request body compression

### Scaling Considerations

1. **Read Replicas**: PostgreSQL streaming replication for read scaling
2. **Sharding**: User-based sharding for horizontal scaling beyond 10,000 users
3. **Time-series Database**: Consider TimescaleDB or InfluxDB for metrics at scale
4. **CDN Integration**: Static asset caching for documentation and dashboards
5. **Message Queue**: RabbitMQ/Kafka for async processing at high volume

## Appendix

### Sample Docker Compose for Development

```yaml
version: '3.8'

services:
  postgres:
    image: postgis/postgis:15-3.3
    environment:
      POSTGRES_DB: health_export
      POSTGRES_USER: health_export
      POSTGRES_PASSWORD: development
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
  
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"
  
  api:
    build: .
    environment:
      DATABASE_URL: postgres://health_export:development@postgres/health_export
      REDIS_URL: redis://redis:6379
      RUST_LOG: info
    ports:
      - "8080:8080"
    depends_on:
      - postgres
      - redis
    volumes:
      - ./config:/app/config:ro

volumes:
  postgres_data:
  redis_data:
```

### Useful SQL Queries

```sql
-- Daily active users
SELECT DATE(received_at) as date, COUNT(DISTINCT user_id) as active_users
FROM raw_ingestions
WHERE received_at > NOW() - INTERVAL '30 days'
GROUP BY DATE(received_at)
ORDER BY date DESC;

-- Most common error types
SELECT 
    processing_errors->>'error_type' as error_type,
    COUNT(*) as count
FROM raw_ingestions
WHERE processing_errors IS NOT NULL
GROUP BY processing_errors->>'error_type'
ORDER BY count DESC;

-- User health data summary
SELECT 
    u.email,
    COUNT(DISTINCT DATE(hr.recorded_at)) as days_with_heart_data,
    AVG(hr.avg_bpm) as avg_heart_rate,
    COUNT(DISTINCT w.id) as total_workouts
FROM users u
LEFT JOIN heart_rate_metrics hr ON u.id = hr.user_id
LEFT JOIN workouts w ON u.id = w.user_id
WHERE hr.recorded_at > NOW() - INTERVAL '30 days'
GROUP BY u.id, u.email;
```

---

*This architecture document is a living document and should be updated as the system evolves.*