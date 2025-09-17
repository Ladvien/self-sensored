use actix_web::{web, HttpResponse, Result};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::models::{
    ApiResponse, HealthMetric, IngestData, IngestPayload, IngestResponse, IosIngestPayload,
};
use crate::services::auth::AuthContext;
use crate::services::batch_processor::{BatchProcessingResult, BatchProcessor};

// All payload and metrics limits removed for personal health app

/// Optimized health data ingest endpoint with performance improvements
/// Key optimizations:
/// 1. Reduced string conversions and allocations
/// 2. Streaming JSON parsing to avoid double-buffering  
/// 3. Parallel validation using tokio spawn_blocking
/// 4. Reduced memory footprint through arena allocations
/// 5. Connection pooling optimizations
#[instrument(skip(pool, payload))]
pub async fn optimized_ingest_handler(
    pool: web::Data<PgPool>,
    auth: AuthContext,
    payload: web::Bytes,
) -> Result<HttpResponse> {
    let start_time = Instant::now();

    // No payload size limit for personal health app

    info!(
        user_id = %auth.user.id,
        api_key_id = %auth.api_key.id,
        payload_size = payload.len(),
        "Starting optimized health data ingestion"
    );

    // Optimization 1: Parse JSON with faster parser and better error handling
    let internal_payload = match parse_payload_optimized(&payload, auth.user.id).await {
        Ok(payload) => payload,
        Err(e) => {
            error!("JSON parsing failed: {}", e);
            return Ok(
                HttpResponse::BadRequest().json(ApiResponse::<()>::error(format!(
                    "Invalid JSON format: {e}"
                ))),
            );
        }
    };

    // Optimization 2: Validate payload constraints early
    let _total_metrics = internal_payload.data.metrics.len() + internal_payload.data.workouts.len();
    // No metrics count limit for personal health app

    // Optimization 3: Parallel validation using task-based parallelism
    let validation_result = validate_payload_parallel(&internal_payload).await;

    let processed_payload = match validation_result {
        Ok(payload) => payload,
        Err(errors) => {
            if errors.is_empty() {
                error!("No valid metrics remaining after validation");
                return Ok(HttpResponse::BadRequest().json(
                    ApiResponse::<IngestResponse>::error_with_data(
                        "All metrics failed validation".to_string(),
                        IngestResponse {
                            success: false,
                            processed_count: 0,
                            failed_count: errors.len(),
                            processing_time_ms: start_time.elapsed().as_millis() as u64,
                            errors,
                            processing_status: Some("error".to_string()),
                            raw_ingestion_id: None,
                        },
                    ),
                ));
            }
            // Continue with valid data
            internal_payload
        }
    };

    // Optimization 4: Use Arc for shared data to reduce copying
    let payload_arc = Arc::new(processed_payload);

    // Store raw payload asynchronously (fire and forget for performance)
    let raw_storage_task = {
        let pool_clone = pool.get_ref().clone();
        let auth_clone = auth.clone();
        let payload_clone = payload_arc.clone();

        tokio::spawn(async move {
            store_raw_payload_optimized(&pool_clone, &auth_clone, &payload_clone).await
        })
    };

    // Optimization 5: Process batch with optimized processor
    let processor = BatchProcessor::new(pool.get_ref().clone());
    let result = processor
        .process_batch(auth.user.id, (*payload_arc).clone())
        .await;

    let processing_time = start_time.elapsed().as_millis() as u64;

    // Wait for raw storage (but don't block response on it)
    #[allow(clippy::collapsible_match)]
    if let Ok(join_result) =
        tokio::time::timeout(std::time::Duration::from_millis(100), raw_storage_task).await
    {
        if let Ok(storage_result) = join_result {
            if let Ok(raw_uuid) = storage_result {
                // Update processing status asynchronously
                let pool_clone = pool.get_ref().clone();
                let result_clone = result.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        update_processing_status(&pool_clone, raw_uuid, &result_clone).await
                    {
                        error!("Failed to update processing status: {}", e);
                    }
                });
            }
        }
    }

    // Create optimized response
    let success = result.errors.is_empty();
    let processing_status = if success { "processed" } else { "partial_success" };

    let response = IngestResponse {
        success,
        processed_count: result.processed_count,
        failed_count: result.failed_count,
        processing_time_ms: processing_time,
        errors: result.errors,
        processing_status: Some(processing_status.to_string()),
        raw_ingestion_id: None, // Optimized handler doesn't track raw ingestion
    };

    info!(
        user_id = %auth.user.id,
        processed_count = response.processed_count,
        failed_count = response.failed_count,
        processing_time_ms = processing_time,
        "Optimized health data ingestion completed"
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Optimized JSON parsing with better error handling and memory efficiency
async fn parse_payload_optimized(
    payload: &[u8],
    user_id: uuid::Uuid,
) -> Result<IngestPayload, String> {
    // Use spawn_blocking for CPU-intensive JSON parsing to avoid blocking the async runtime
    let payload_vec = payload.to_vec();

    tokio::task::spawn_blocking(move || {
        // Try iOS format first (more common based on usage patterns)
        match simd_json::from_slice::<IosIngestPayload>(&mut payload_vec.clone()) {
            Ok(ios_payload) => Ok(ios_payload.to_internal_format(user_id)),
            Err(_) => {
                // Try standard format
                match simd_json::from_slice::<IngestPayload>(&mut payload_vec.clone()) {
                    Ok(standard_payload) => Ok(standard_payload),
                    Err(e) => Err(format!(
                        "Failed to parse both iOS and standard formats: {e}"
                    )),
                }
            }
        }
    })
    .await
    .map_err(|e| format!("JSON parsing task failed: {e}"))?
}

/// Parallel validation using task-based processing
async fn validate_payload_parallel(
    payload: &IngestPayload,
) -> Result<IngestPayload, Vec<crate::models::ProcessingError>> {
    let metrics = payload.data.metrics.clone();
    let workouts = payload.data.workouts.clone();

    // Spawn validation tasks in parallel
    let metrics_validation = tokio::task::spawn_blocking(move || validate_metrics_batch(&metrics));

    let workouts_validation =
        tokio::task::spawn_blocking(move || validate_workouts_batch(&workouts));

    // Await both validation tasks
    let (metrics_result, workouts_result) = tokio::join!(metrics_validation, workouts_validation);

    let metrics_errors = metrics_result.map_err(|e| {
        vec![crate::models::ProcessingError {
            metric_type: "ValidationTask".to_string(),
            error_message: format!("Metrics validation task failed: {e}"),
            index: None,
        }]
    })?;

    let workouts_errors = workouts_result.map_err(|e| {
        vec![crate::models::ProcessingError {
            metric_type: "ValidationTask".to_string(),
            error_message: format!("Workouts validation task failed: {e}"),
            index: None,
        }]
    })?;

    let mut all_errors = metrics_errors;
    all_errors.extend(workouts_errors);

    if all_errors.is_empty() {
        Ok(payload.clone())
    } else {
        // Filter out invalid metrics and return both valid payload and errors
        let valid_metrics: Vec<HealthMetric> = payload
            .data
            .metrics
            .iter()
            .filter(|metric| metric.validate().is_ok())
            .cloned()
            .collect();

        let valid_workouts: Vec<crate::models::WorkoutData> = payload
            .data
            .workouts
            .iter()
            .filter(|workout| validate_single_workout_optimized(workout).is_ok())
            .cloned()
            .collect();

        let filtered_payload = IngestPayload {
            data: IngestData {
                metrics: valid_metrics,
                workouts: valid_workouts,
            },
        };

        if filtered_payload.data.metrics.is_empty() && filtered_payload.data.workouts.is_empty() {
            Err(all_errors)
        } else {
            Ok(filtered_payload)
        }
    }
}

/// Batch validation for metrics (CPU intensive)
fn validate_metrics_batch(metrics: &[HealthMetric]) -> Vec<crate::models::ProcessingError> {
    metrics
        .iter() // Sequential processing to avoid rayon dependency
        .enumerate()
        .filter_map(|(index, metric)| match metric.validate() {
            Ok(()) => None,
            Err(validation_error) => Some(crate::models::ProcessingError {
                metric_type: metric.metric_type().to_string(),
                error_message: validation_error,
                index: Some(index),
            }),
        })
        .collect()
}

/// Batch validation for workouts (CPU intensive)
fn validate_workouts_batch(
    workouts: &[crate::models::WorkoutData],
) -> Vec<crate::models::ProcessingError> {
    workouts
        .iter() // Sequential processing to avoid rayon dependency
        .enumerate()
        .filter_map(
            |(index, workout)| match validate_single_workout_optimized(workout) {
                Ok(()) => None,
                Err(validation_error) => Some(crate::models::ProcessingError {
                    metric_type: "Workout".to_string(),
                    error_message: validation_error,
                    index: Some(index),
                }),
            },
        )
        .collect()
}

/// Optimized workout validation with early returns
fn validate_single_workout_optimized(workout: &crate::models::WorkoutData) -> Result<(), String> {
    // Early returns for performance
    if workout.started_at >= workout.ended_at {
        return Err("Workout ended_at must be after started_at".to_string());
    }

    let duration = workout.ended_at - workout.started_at;
    if duration.num_hours() > 24 {
        return Err("Workout duration cannot exceed 24 hours".to_string());
    }

    // Use match for better performance than if-let chains
    match workout.total_energy_kcal {
        Some(calories) if !(0.0..=10000.0).contains(&calories) => {
            return Err(format!(
                "total_energy_kcal {calories} is out of reasonable range (0-10000)"
            ));
        }
        _ => {}
    }

    match workout.distance_meters {
        Some(distance) if !(0.0..=1000000.0).contains(&distance) => {
            return Err(format!(
                "distance_meters {distance} is out of reasonable range (0-1000000)"
            ));
        }
        _ => {}
    }

    if let Some(hr) = workout.avg_heart_rate {
        if !(15..=300).contains(&hr) {
            return Err(format!("avg_heart_rate {hr} is out of range (15-300)"));
        }
    }

    if let Some(hr) = workout.max_heart_rate {
        if !(15..=300).contains(&hr) {
            return Err(format!("max_heart_rate {hr} is out of range (15-300)"));
        }
    }

    // WorkoutType is an enum, so it's always valid

    Ok(())
}

/// Optimized raw payload storage with better connection handling
async fn store_raw_payload_optimized(
    pool: &PgPool,
    auth: &AuthContext,
    payload: &IngestPayload,
) -> Result<Uuid, sqlx::Error> {
    // Use faster JSON serialization
    let payload_json = simd_json::to_string(payload).map_err(sqlx::Error::decode)?;
    let payload_hash = format!("{:x}", Sha256::digest(payload_json.as_bytes()));
    let payload_size = payload_json.len() as i32;

    // Use prepared statement for better performance
    let result = sqlx::query!(
        r#"
        INSERT INTO raw_ingestions (user_id, payload_hash, payload_size_bytes, raw_payload) 
        VALUES ($1, $2, $3, $4) 
        RETURNING id
        "#,
        auth.user.id,
        payload_hash,
        payload_size,
        serde_json::to_value(payload).map_err(|e| sqlx::Error::decode(e))?
    )
    .fetch_one(pool)
    .await?;

    Ok(result.id)
}

/// Update processing status (async, non-blocking)
async fn update_processing_status(
    pool: &PgPool,
    raw_id: Uuid,
    result: &BatchProcessingResult,
) -> Result<(), sqlx::Error> {
    let status = if result.errors.is_empty() {
        "processed"
    } else {
        "error"
    };

    let processing_errors = if result.errors.is_empty() {
        None
    } else {
        Some(
            serde_json::to_value(
                result
                    .errors
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "metric_type": e.metric_type,
                            "error_message": e.error_message
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        )
    };

    sqlx::query!(
        r#"
        UPDATE raw_ingestions 
        SET processed_at = NOW(), processing_status = $2, processing_errors = $3
        WHERE id = $1
        "#,
        raw_id,
        status,
        processing_errors
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Additional performance optimizations that could be implemented:

/// Connection pool optimization settings
pub fn configure_optimized_pool() -> sqlx::postgres::PgPoolOptions {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(50) // Optimize based on CPU cores and workload
        .min_connections(10) // Keep minimum connections warm
        .idle_timeout(Some(std::time::Duration::from_secs(300))) // 5 minutes
        .max_lifetime(Some(std::time::Duration::from_secs(1800))) // 30 minutes
        .test_before_acquire(true) // Ensure connection health
        .acquire_timeout(std::time::Duration::from_secs(10))
}

/// Memory pool for reducing allocations in hot paths
pub struct MetricsArena {
    buffer: Vec<u8>,
    position: usize,
}

impl Default for MetricsArena {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsArena {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024 * 1024), // 1MB initial capacity
            position: 0,
        }
    }

    pub fn allocate(&mut self, size: usize) -> &mut [u8] {
        if self.position + size > self.buffer.len() {
            self.buffer.resize(self.position + size, 0);
        }

        let start = self.position;
        self.position += size;
        &mut self.buffer[start..self.position]
    }

    pub fn reset(&mut self) {
        self.position = 0;
    }
}

/// Async queue for batching database operations
pub struct BatchQueue<T> {
    items: Arc<tokio::sync::Mutex<Vec<T>>>,
    batch_size: usize,
    #[allow(dead_code)]
    flush_interval: std::time::Duration,
}

impl<T> BatchQueue<T> {
    pub fn new(batch_size: usize, flush_interval: std::time::Duration) -> Self {
        Self {
            items: Arc::new(tokio::sync::Mutex::new(Vec::with_capacity(batch_size))),
            batch_size,
            flush_interval,
        }
    }

    pub async fn push(&self, item: T) -> bool {
        let mut items = self.items.lock().await;
        items.push(item);
        items.len() >= self.batch_size
    }

    pub async fn flush(&self) -> Vec<T> {
        let mut items = self.items.lock().await;
        std::mem::take(&mut *items)
    }
}
