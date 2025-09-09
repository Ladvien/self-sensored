Based on your feedback, here are the revised MVP-focused Jira stories with proper environment configuration and database schema details:

# 🎯 Health Export REST API - MVP Jira Stories


---

## 🚀 MVP Epic Structure (Parallel Streams)

**Stream 1: Project Setup & Database** (Backend Agent)  
**Stream 2: Core API & Data Models** (API Agent)  
**Stream 3: Authentication** (Security Agent)  
**Stream 4: Batch Processing** (Data Agent)  
**Stream 5: Testing** (QA Agent)

---

## Stream 1: Project Setup & Database

### STORY-001: Initialize Rust Project with Dependencies
**Assigned Agent**: Backend Agent  
**Story Points**: 3  
**Dependencies**: None  
**Parallel Work**: Yes

**Description:**  
As a developer, I need a Rust project structure with all required dependencies and environment configuration.

**Acceptance Criteria:**
- [ ] Create Cargo.toml with dependencies:
  ```toml
  [package]
  name = "health-export-api"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  actix-web = "4.4"
  sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }
  tokio = { version = "1.35", features = ["full"] }
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  uuid = { version = "1.6", features = ["v4", "serde"] }
  chrono = { version = "0.4", features = ["serde"] }
  argon2 = "0.5"
  redis = { version = "0.24", features = ["tokio-comp"] }
  thiserror = "1.0"
  validator = { version = "0.16", features = ["derive"] }
  dotenv = "0.15"
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["json"] }
  ```
- [ ] Create directory structure:
  ```
  src/
    ├── main.rs
    ├── lib.rs
    ├── models/
    ├── handlers/
    ├── services/
    └── db/
  tests/
  migrations/
  ```
- [ ] Create .env.example:
  ```env
  DATABASE_URL=postgresql://username:password@host:port/database
  TEST_DATABASE_URL=postgresql://username:password@host:port/test_database
  REDIS_URL=redis://host:port
  RUST_LOG=info
  ```
- [ ] Implement environment loading in main.rs:
  ```rust
  use dotenv::dotenv;
  use std::env;

  fn load_config() -> Config {
      dotenv().ok();
      Config {
          database_url: env::var("DATABASE_URL")
              .expect("DATABASE_URL must be set"),
          test_database_url: env::var("TEST_DATABASE_URL")
              .expect("TEST_DATABASE_URL must be set"),
          redis_url: env::var("REDIS_URL")
              .expect("REDIS_URL must be set"),
      }
  }
  ```

**Test Requirements:**
- [ ] Create `tests/project_setup_test.rs`:
  ```rust
  #[test]
  fn test_env_variables_loaded() {
      dotenv::dotenv().ok();
      assert!(std::env::var("DATABASE_URL").is_ok());
      assert!(std::env::var("TEST_DATABASE_URL").is_ok());
  }
  ```

**Definition of Done:**
- ✅ Project compiles with `cargo build`
- ✅ Environment variables load from .env
- ✅ Tests pass with `cargo test`
- ✅ No hardcoded credentials in code

---

### STORY-002: Create Database Schema with Migrations
**Assigned Agent**: Backend Agent  
**Story Points**: 8  
**Dependencies**: STORY-001  
**Parallel Work**: No

**Description:**  
As the system, I need PostgreSQL database schema with all tables for storing health data.

**Acceptance Criteria:**
- [ ] Install sqlx-cli: `cargo install sqlx-cli`
- [ ] Create migration files:
  ```bash
  sqlx migrate add create_users_tables
  sqlx migrate add create_health_metrics_tables
  sqlx migrate add create_partitions
  ```
- [ ] Migration 001: Core tables
  ```sql
  -- Enable extensions
  CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
  CREATE EXTENSION IF NOT EXISTS "postgis";

  -- Users table
  CREATE TABLE users (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      email VARCHAR(255) UNIQUE NOT NULL,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );

  -- API keys with Argon2 hashing
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

  -- Indexes
  CREATE INDEX idx_api_keys_user ON api_keys(user_id);
  CREATE INDEX idx_api_keys_hash ON api_keys(key_hash) WHERE revoked_at IS NULL;
  ```
- [ ] Migration 002: Health metrics tables
  ```sql
  -- Raw ingestions for backup
  CREATE TABLE raw_ingestions (
      id BIGSERIAL PRIMARY KEY,
      user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      api_key_id UUID NOT NULL REFERENCES api_keys(id),
      payload JSONB NOT NULL,
      received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      processed_at TIMESTAMPTZ,
      processing_errors JSONB
  ) PARTITION BY RANGE (received_at);

  -- Heart rate metrics
  CREATE TABLE heart_rate_metrics (
      id BIGSERIAL,
      user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      recorded_at TIMESTAMPTZ NOT NULL,
      min_bpm SMALLINT CHECK (min_bpm BETWEEN 20 AND 300),
      avg_bpm SMALLINT CHECK (avg_bpm BETWEEN 20 AND 300),
      max_bpm SMALLINT CHECK (max_bpm BETWEEN 20 AND 300),
      source VARCHAR(50),
      raw_data JSONB,
      created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
      PRIMARY KEY (user_id, recorded_at)
  ) PARTITION BY RANGE (recorded_at);

  -- Other metrics tables (blood_pressure, sleep, activity, etc.)
  ```
- [ ] Migration 003: Create initial partitions
  ```sql
  -- Create monthly partitions for next 12 months
  CREATE TABLE raw_ingestions_2025_01 PARTITION OF raw_ingestions 
      FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
  -- Continue for 12 months...
  ```
- [ ] Run migrations:
  ```bash
  export DATABASE_URL=$DATABASE_URL
  sqlx migrate run
  ```

**Test Requirements:**
- [ ] Create `tests/db/schema_test.rs`:
  ```rust
  #[sqlx::test]
  async fn test_database_schema(pool: PgPool) {
      // Test UUID generation
      let result = sqlx::query!("SELECT gen_random_uuid() as id")
          .fetch_one(&pool)
          .await;
      assert!(result.is_ok());

      // Test table existence
      let tables = sqlx::query!(
          "SELECT table_name FROM information_schema.tables 
           WHERE table_schema = 'public'"
      ).fetch_all(&pool).await.unwrap();
      
      assert!(tables.iter().any(|t| t.table_name == Some("users".to_string())));
  }
  ```

**Definition of Done:**
- ✅ All migrations run successfully
- ✅ Database schema matches specification
- ✅ Partitioning configured for time-series data
- ✅ Tests verify schema correctness
- ✅ `sqlx migrate info` shows all applied

---

## Stream 2: Core API & Data Models

### STORY-003: Implement Core Actix-web Application
**Assigned Agent**: API Agent  
**Story Points**: 5  
**Dependencies**: STORY-001  
**Parallel Work**: Yes

**Description:**  
As the API, I need the basic Actix-web server with health endpoints and database connection pooling.

**Acceptance Criteria:**
- [ ] Implement `src/main.rs`:
  ```rust
  use actix_web::{web, App, HttpServer, HttpResponse};
  use sqlx::postgres::PgPoolOptions;
  use dotenv::dotenv;
  use std::env;

  #[derive(Clone)]
  struct AppState {
      db_pool: sqlx::PgPool,
  }

  async fn health(state: web::Data<AppState>) -> HttpResponse {
      // Test database connection
      match sqlx::query!("SELECT 1 as healthy")
          .fetch_one(&state.db_pool)
          .await {
          Ok(_) => HttpResponse::Ok().json(serde_json::json!({
              "status": "healthy",
              "database": "connected"
          })),
          Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
              "status": "unhealthy",
              "database": "disconnected"
          }))
      }
  }

  #[actix_web::main]
  async fn main() -> std::io::Result<()> {
      dotenv().ok();
      tracing_subscriber::fmt::init();

      let database_url = env::var("DATABASE_URL")
          .expect("DATABASE_URL must be set in .env");

      let pool = PgPoolOptions::new()
          .max_connections(5)
          .connect(&database_url)
          .await
          .expect("Failed to connect to database");

      let state = AppState { db_pool: pool };

      HttpServer::new(move || {
          App::new()
              .app_data(web::Data::new(state.clone()))
              .route("/health", web::get().to(health))
              .route("/ready", web::get().to(ready))
      })
      .bind("127.0.0.1:8080")?
      .run()
      .await
  }
  ```
- [ ] Create ready endpoint that checks all dependencies
- [ ] Implement graceful shutdown handling
- [ ] Add request logging middleware

**Test Requirements:**
- [ ] Create `tests/api/health_test.rs`:
  ```rust
  use actix_web::test;

  #[actix_rt::test]
  async fn test_health_endpoint() {
      let app = test::init_service(create_app()).await;
      let req = test::TestRequest::get()
          .uri("/health")
          .to_request();
      let resp = test::call_service(&app, req).await;
      assert_eq!(resp.status(), 200);
  }
  ```

**Definition of Done:**
- ✅ Server starts on port 8080
- ✅ Health endpoint returns 200 when healthy
- ✅ Database connection uses .env configuration
- ✅ Graceful shutdown works
- ✅ Tests pass

---

### STORY-004: Create Health Data Models
**Assigned Agent**: API Agent  
**Story Points**: 8  
**Dependencies**: STORY-002  
**Parallel Work**: Yes

**Description:**  
As the API, I need data models for all health metric types with validation.

**Acceptance Criteria:**
- [ ] Create `src/models/health_metrics.rs`:
  ```rust
  use serde::{Deserialize, Serialize};
  use validator::Validate;
  use chrono::{DateTime, Utc};
  use uuid::Uuid;

  #[derive(Debug, Deserialize, Serialize, Validate)]
  pub struct HeartRateMetric {
      pub recorded_at: DateTime<Utc>,
      #[validate(range(min = 20, max = 300))]
      pub min_bpm: Option<i16>,
      #[validate(range(min = 20, max = 300))]
      pub avg_bpm: Option<i16>,
      #[validate(range(min = 20, max = 300))]
      pub max_bpm: Option<i16>,
      pub source: Option<String>,
  }

  #[derive(Debug, Deserialize, Serialize, Validate)]
  pub struct BloodPressureMetric {
      pub recorded_at: DateTime<Utc>,
      #[validate(range(min = 40, max = 250))]
      pub systolic: i16,
      #[validate(range(min = 30, max = 150))]
      pub diastolic: i16,
      pub source: Option<String>,
  }

  #[derive(Debug, Deserialize, Serialize)]
  pub struct WorkoutData {
      pub workout_type: String,
      pub start_time: DateTime<Utc>,
      pub end_time: DateTime<Utc>,
      pub total_energy_kcal: Option<f64>,
      pub distance_meters: Option<f64>,
      pub avg_heart_rate: Option<i16>,
  }

  #[derive(Debug, Deserialize, Serialize)]
  #[serde(tag = "type")]
  pub enum HealthMetric {
      HeartRate(HeartRateMetric),
      BloodPressure(BloodPressureMetric),
      Workout(WorkoutData),
      // Add other types as needed
  }

  #[derive(Debug, Deserialize, Serialize)]
  pub struct IngestPayload {
      pub data: IngestData,
  }

  #[derive(Debug, Deserialize, Serialize)]
  pub struct IngestData {
      pub metrics: Vec<HealthMetric>,
      pub workouts: Vec<WorkoutData>,
  }
  ```
- [ ] Create database models in `src/models/db.rs`
- [ ] Implement conversion between API and DB models
- [ ] Add validation for all numeric ranges

**Test Requirements:**
- [ ] Create `tests/models/health_metrics_test.rs`:
  ```rust
  #[test]
  fn test_heart_rate_validation() {
      let valid = HeartRateMetric {
          recorded_at: Utc::now(),
          min_bpm: Some(60),
          avg_bpm: Some(75),
          max_bpm: Some(90),
          source: Some("Apple Watch".to_string()),
      };
      assert!(valid.validate().is_ok());

      let invalid = HeartRateMetric {
          recorded_at: Utc::now(),
          min_bpm: Some(400), // Invalid
          avg_bpm: Some(75),
          max_bpm: Some(90),
          source: None,
      };
      assert!(invalid.validate().is_err());
  }
  ```

**Definition of Done:**
- ✅ All health metric types modeled
- ✅ Validation rules match specification
- ✅ Serialization/deserialization works
- ✅ Test coverage for all models
- ✅ Models align with database schema

---

## Stream 3: Authentication

### STORY-005: Implement API Key Authentication
**Assigned Agent**: Security Agent  
**Story Points**: 8  
**Dependencies**: STORY-002  
**Parallel Work**: Yes

**Description:**  
As the API, I need Bearer token authentication with Argon2-hashed API keys.

**Acceptance Criteria:**
- [ ] Create `src/services/auth.rs`:
  ```rust
  use argon2::{
      password_hash::{PasswordHasher, PasswordVerifier, SaltString},
      Argon2,
  };
  use uuid::Uuid;

  pub struct AuthService {
      db_pool: sqlx::PgPool,
  }

  impl AuthService {
      pub async fn generate_api_key(
          &self,
          user_id: Uuid,
          name: &str,
      ) -> Result<String, AuthError> {
          // Generate secure random key
          let api_key = Uuid::new_v4().to_string();
          
          // Hash with Argon2
          let salt = SaltString::generate(&mut rand::thread_rng());
          let argon2 = Argon2::default();
          let hash = argon2
              .hash_password(api_key.as_bytes(), &salt)?
              .to_string();

          // Store in database
          sqlx::query!(
              "INSERT INTO api_keys (user_id, key_hash, name) 
               VALUES ($1, $2, $3)",
              user_id,
              hash,
              name
          )
          .execute(&self.db_pool)
          .await?;

          Ok(api_key)
      }

      pub async fn verify_api_key(
          &self,
          api_key: &str,
      ) -> Result<Uuid, AuthError> {
          // Get all active keys (not ideal but MVP)
          let keys = sqlx::query!(
              "SELECT user_id, key_hash FROM api_keys 
               WHERE revoked_at IS NULL"
          )
          .fetch_all(&self.db_pool)
          .await?;

          let argon2 = Argon2::default();
          
          for key_record in keys {
              let hash = PasswordHash::new(&key_record.key_hash)?;
              if argon2.verify_password(api_key.as_bytes(), &hash).is_ok() {
                  // Update last_used_at
                  sqlx::query!(
                      "UPDATE api_keys SET last_used_at = NOW() 
                       WHERE key_hash = $1",
                      key_record.key_hash
                  )
                  .execute(&self.db_pool)
                  .await?;

                  return Ok(key_record.user_id);
              }
          }

          Err(AuthError::InvalidApiKey)
      }
  }
  ```
- [ ] Create authentication middleware:
  ```rust
  use actix_web::{dev::ServiceRequest, Error, FromRequest};

  pub async fn auth_middleware(
      req: ServiceRequest,
      credentials: BearerAuth,
  ) -> Result<ServiceRequest, Error> {
      let auth_service = req.app_data::<AuthService>().unwrap();
      
      match auth_service.verify_api_key(credentials.token()).await {
          Ok(user_id) => {
              req.extensions_mut().insert(user_id);
              Ok(req)
          }
          Err(_) => Err(actix_web::error::ErrorUnauthorized("Invalid API key"))
      }
  }
  ```
- [ ] Add API key management endpoints

**Test Requirements:**
- [ ] Create `tests/services/auth_test.rs`:
  ```rust
  #[sqlx::test]
  async fn test_api_key_generation_and_verification(pool: PgPool) {
      let auth_service = AuthService::new(pool.clone());
      let user_id = create_test_user(&pool).await;
      
      // Generate key
      let api_key = auth_service
          .generate_api_key(user_id, "test")
          .await
          .unwrap();
      
      // Verify valid key
      let verified_user = auth_service
          .verify_api_key(&api_key)
          .await
          .unwrap();
      assert_eq!(verified_user, user_id);
      
      // Verify invalid key
      let result = auth_service
          .verify_api_key("invalid-key")
          .await;
      assert!(result.is_err());
  }
  ```

**Definition of Done:**
- ✅ API keys generated securely
- ✅ Argon2 hashing implemented
- ✅ Bearer token middleware works
- ✅ Last used timestamp updates
- ✅ Tests verify authentication flow

---

## Stream 4: Batch Processing

### STORY-006: Implement Batch Data Ingestion
**Assigned Agent**: Data Agent  
**Story Points**: 13  
**Dependencies**: STORY-003, STORY-004, STORY-005  
**Parallel Work**: No

**Description:**  
As the API, I need to process health data in batches for efficient database operations.

**Acceptance Criteria:**
- [ ] Create `src/handlers/ingest.rs`:
  ```rust
  use actix_web::{web, HttpResponse};
  use sqlx::PgPool;

  pub async fn ingest_handler(
      state: web::Data<AppState>,
      user_id: web::ReqData<Uuid>,
      payload: web::Json<IngestPayload>,
  ) -> HttpResponse {
      let user_id = user_id.into_inner();
      
      // Store raw payload first
      let raw_id = store_raw_payload(
          &state.db_pool,
          user_id,
          &payload
      ).await;

      // Process in batches
      let result = process_batch(
          &state.db_pool,
          user_id,
          payload.into_inner()
      ).await;

      HttpResponse::Ok().json(result)
  }

  async fn store_raw_payload(
      pool: &PgPool,
      user_id: Uuid,
      payload: &IngestPayload,
  ) -> Result<i64, sqlx::Error> {
      let result = sqlx::query!(
          "INSERT INTO raw_ingestions (user_id, api_key_id, payload) 
           VALUES ($1, $2, $3) RETURNING id",
          user_id,
          user_id, // MVP: reuse user_id
          serde_json::to_value(payload).unwrap()
      )
      .fetch_one(pool)
      .await?;

      Ok(result.id)
  }
  ```
- [ ] Create `src/services/batch_processor.rs`:
  ```rust
  use sqlx::{PgPool, QueryBuilder, Postgres};

  pub async fn process_batch(
      pool: &PgPool,
      user_id: Uuid,
      payload: IngestPayload,
  ) -> ProcessingResult {
      let mut result = ProcessingResult::default();
      
      // Group metrics by type
      let mut heart_rates = Vec::new();
      let mut blood_pressures = Vec::new();
      
      for metric in payload.data.metrics {
          match metric {
              HealthMetric::HeartRate(hr) => heart_rates.push(hr),
              HealthMetric::BloodPressure(bp) => blood_pressures.push(bp),
              _ => {}
          }
      }
      
      // Batch insert heart rates
      if !heart_rates.is_empty() {
          let inserted = insert_heart_rates(pool, user_id, heart_rates).await?;
          result.processed_metrics += inserted;
      }
      
      // Batch insert blood pressures
      if !blood_pressures.is_empty() {
          let inserted = insert_blood_pressures(pool, user_id, blood_pressures).await?;
          result.processed_metrics += inserted;
      }
      
      result
  }

  async fn insert_heart_rates(
      pool: &PgPool,
      user_id: Uuid,
      metrics: Vec<HeartRateMetric>,
  ) -> Result<usize, sqlx::Error> {
      let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
          "INSERT INTO heart_rate_metrics 
           (user_id, recorded_at, min_bpm, avg_bpm, max_bpm, source) "
      );
      
      query_builder.push_values(metrics.iter(), |mut b, metric| {
          b.push_bind(user_id)
           .push_bind(metric.recorded_at)
           .push_bind(metric.min_bpm)
           .push_bind(metric.avg_bpm)
           .push_bind(metric.max_bpm)
           .push_bind(&metric.source);
      });
      
      query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");
      
      let result = query_builder.build().execute(pool).await?;
      Ok(result.rows_affected() as usize)
  }
  ```
- [ ] Implement processing for all metric types
- [ ] Add error handling and partial failure support
- [ ] Return detailed processing results

**Test Requirements:**
- [ ] Create `tests/handlers/ingest_test.rs`:
  ```rust
  #[sqlx::test]
  async fn test_batch_ingestion(pool: PgPool) {
      let app = create_test_app(pool.clone()).await;
      let user = create_test_user(&pool).await;
      let api_key = create_test_api_key(&pool, user.id).await;
      
      let payload = json!({
          "data": {
              "metrics": [
                  {
                      "type": "HeartRate",
                      "recorded_at": "2025-01-15T10:00:00Z",
                      "min_bpm": 60,
                      "avg_bpm": 75,
                      "max_bpm": 90
                  }
              ],
              "workouts": []
          }
      });
      
      let req = test::TestRequest::post()
          .uri("/v1/ingest")
          .header("Authorization", format!("Bearer {}", api_key))
          .set_json(&payload)
          .to_request();
      
      let resp = test::call_service(&app, req).await;
      assert_eq!(resp.status(), 200);
      
      // Verify data in database
      let count = sqlx::query_scalar!(
          "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
          user.id
      )
      .fetch_one(&pool)
      .await
      .unwrap();
      
      assert_eq!(count, Some(1i64));
  }
  ```

**Definition of Done:**
- ✅ Batch insert implemented for all metric types
- ✅ Raw payload stored for backup
- ✅ Duplicates handled with ON CONFLICT
- ✅ Detailed response with processing stats
- ✅ Integration tests pass

---

### STORY-007: Add Rate Limiting
**Assigned Agent**: Security Agent  
**Story Points**: 5  
**Dependencies**: STORY-005  
**Parallel Work**: No

**Description:**  
As the API, I need rate limiting to prevent abuse (100 requests/hour per API key).

**Acceptance Criteria:**
- [ ] Create `src/services/rate_limiter.rs`:
  ```rust
  use redis::AsyncCommands;
  use std::env;

  pub struct RateLimiter {
      redis_client: redis::Client,
  }

  impl RateLimiter {
      pub fn new() -> Self {
          let redis_url = env::var("REDIS_URL")
              .expect("REDIS_URL must be set");
          let redis_client = redis::Client::open(redis_url)
              .expect("Failed to connect to Redis");
          
          RateLimiter { redis_client }
      }

      pub async fn check_rate_limit(
          &self,
          api_key_id: &Uuid,
      ) -> Result<bool, RateLimitError> {
          let mut conn = self.redis_client
              .get_async_connection()
              .await?;
          
          let key = format!("rate_limit:{}", api_key_id);
          let count: u32 = conn.incr(&key, 1).await?;
          
          if count == 1 {
              conn.expire(&key, 3600).await?; // 1 hour
          }
          
          Ok(count <= 100)
      }
  }
  ```
- [ ] Add rate limiting middleware
- [ ] Return 429 status when limit exceeded
- [ ] Include X-RateLimit headers in responses

**Test Requirements:**
- [ ] Create `tests/services/rate_limiter_test.rs`:
  ```rust
  #[tokio::test]
  async fn test_rate_limiting() {
      let limiter = RateLimiter::new();
      let api_key = Uuid::new_v4();
      
      // First 100 requests should pass
      for _ in 0..100 {
          assert!(limiter.check_rate_limit(&api_key).await.unwrap());
      }
      
      // 101st request should fail
      assert!(!limiter.check_rate_limit(&api_key).await.unwrap());
  }
  ```

**Definition of Done:**
- ✅ Rate limiting enforces 100 req/hour
- ✅ Redis connection uses .env config
- ✅ 429 responses for exceeded limits
- ✅ Rate limit headers included
- ✅ Tests verify limiting behavior

---

## Stream 5: Testing

### STORY-008: Create Integration Test Suite
**Assigned Agent**: QA Agent  
**Story Points**: 8  
**Dependencies**: All other stories  
**Parallel Work**: No

**Description:**  
As a QA engineer, I need comprehensive integration tests for the entire API flow.

**Acceptance Criteria:**
- [ ] Create `tests/integration/full_flow_test.rs`:
  ```rust
  use sqlx::PgPool;
  use dotenv::dotenv;

  async fn setup_test_db() -> PgPool {
      dotenv().ok();
      let test_db_url = std::env::var("TEST_DATABASE_URL")
          .expect("TEST_DATABASE_URL must be set");
      
      PgPoolOptions::new()
          .connect(&test_db_url)
          .await
          .expect("Failed to connect to test database")
  }

  #[tokio::test]
  async fn test_complete_flow() {
      let pool = setup_test_db().await;
      
      // Clean test data
      cleanup_test_data(&pool).await;
      
      // 1. Create user
      let user = create_test_user(&pool).await;
      
      // 2. Generate API key
      let auth_service = AuthService::new(pool.clone());
      let api_key = auth_service
          .generate_api_key(user.id, "test")
          .await
          .unwrap();
      
      // 3. Start test server
      let app = create_test_app(pool.clone()).await;
      
      // 4. Submit health data
      let payload = create_test_payload();
      let response = submit_ingest(&app, &api_key, payload).await;
      assert_eq!(response.status(), 200);
      
      // 5. Verify data stored
      verify_stored_data(&pool, user.id).await;
      
      // 6. Test duplicate handling
      let response2 = submit_ingest(&app, &api_key, payload).await;
      assert_eq!(response2.status(), 200);
      
      // 7. Test rate limiting
      test_rate_limiting(&app, &api_key).await;
      
      // Cleanup
      cleanup_test_data(&pool).await;
  }
  ```
- [ ] Create test helpers in `tests/common/helpers.rs`
- [ ] Test all endpoints with valid/invalid data
- [ ] Test authentication flows
- [ ] Test error scenarios

**Test Requirements:**
- [ ] All tests use TEST_DATABASE_URL from .env
- [ ] Tests are isolated (cleanup before/after)
- [ ] Test coverage > 70%
- [ ] `cargo test` runs all tests successfully

**Definition of Done:**
- ✅ Integration tests cover full API flow
- ✅ Test database configured via .env
- ✅ All critical paths tested
- ✅ Tests run in CI pipeline
- ✅ No test data pollution

---

## 📋 MVP Delivery Summary

### Parallel Work Streams

| Stream | Stories | Dependencies | Can Start |
|--------|---------|--------------|-----------|
| **Setup & DB** | 001, 002 | None → 001 | ✅ Immediately |
| **Core API** | 003, 004 | 001 | ✅ After 001 |
| **Auth** | 005, 007 | 002 | ✅ After 002 |
| **Batch Process** | 006 | 003, 004, 005 | ⏸️ After streams converge |
| **Testing** | 008 | All | ⏸️ After implementation |

### Critical Path
1. **Day 1-2**: STORY-001 (Setup) + STORY-003 (API Core) in parallel
2. **Day 3-4**: STORY-002 (Database) + STORY-004 (Models) in parallel
3. **Day 5-6**: STORY-005 (Auth) + STORY-007 (Rate Limit)
4. **Day 7-9**: STORY-006 (Batch Processing)
5. **Day 10**: STORY-008 (Integration Tests)

### Definition of MVP Done
- ✅ All environment variables in .env (no hardcoded credentials)
- ✅ Database schema deployed with migrations
- ✅ API accepts batch health data with authentication
- ✅ Rate limiting prevents abuse
- ✅ Duplicates handled gracefully
- ✅ Integration tests pass
- ✅ README documents setup and usage

**Total Estimated Effort**: 52 story points (2-week sprint for 2-3 developers)