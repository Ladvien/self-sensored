use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::env;
use std::time::{Duration, Instant};
use tracing::{error, info, warn, instrument};
use uuid::Uuid;

use self_sensored::config::BatchConfig;
use self_sensored::models::{HealthMetric, IngestPayload};
use self_sensored::services::batch_processor::BatchProcessor;

/// Recovery statistics for tracking progress and results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryStats {
    pub total_records: usize,
    pub processed_records: usize,
    pub failed_records: usize,
    pub skipped_records: usize,
    pub total_metrics_recovered: usize,
    pub total_metrics_failed: usize,
    pub processing_time_seconds: u64,
    pub verification_checksums: HashMap<String, String>,
    pub error_breakdown: HashMap<String, usize>,
    pub user_recovery_stats: HashMap<Uuid, UserRecoveryStats>,
}

/// Per-user recovery statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserRecoveryStats {
    pub records_processed: usize,
    pub metrics_recovered: usize,
    pub metrics_failed: usize,
    pub data_loss_percentage: f64,
    pub largest_payload_mb: f64,
    pub processing_errors: Vec<String>,
}

/// Recovery configuration options
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub batch_size: usize,
    pub max_concurrent_jobs: usize,
    pub verification_enabled: bool,
    pub dry_run: bool,
    pub target_status: Vec<String>,
    pub since_date: Option<DateTime<Utc>>,
    pub specific_user_id: Option<Uuid>,
    pub progress_report_interval: usize,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_jobs: 5,
            verification_enabled: true,
            dry_run: false,
            target_status: vec!["error".to_string(), "partial_success".to_string()],
            since_date: None,
            specific_user_id: None,
            progress_report_interval: 50,
        }
    }
}

/// Data recovery and reprocessing utility
pub struct DataRecoveryService {
    pool: PgPool,
    batch_processor: BatchProcessor,
    config: RecoveryConfig,
    stats: RecoveryStats,
}

impl DataRecoveryService {
    pub fn new(pool: PgPool, config: RecoveryConfig) -> Self {
        // Use environment-configured batch processor with safe chunk sizes
        let batch_config = BatchConfig::from_env();
        if let Err(validation_error) = batch_config.validate() {
            panic!("Invalid batch configuration: {}", validation_error);
        }

        let batch_processor = BatchProcessor::with_config(pool.clone(), batch_config);

        Self {
            pool,
            batch_processor,
            config,
            stats: RecoveryStats::default(),
        }
    }

    /// Main recovery orchestration method
    #[instrument(skip(self))]
    pub async fn run_recovery(&mut self) -> Result<RecoveryStats, Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        info!("🚀 Starting data recovery process with configuration: {:?}", self.config);

        // Phase 1: Discover failed records
        let failed_records = self.discover_failed_records().await?;
        self.stats.total_records = failed_records.len();

        if failed_records.is_empty() {
            info!("✅ No failed records found to reprocess");
            return Ok(self.stats.clone());
        }

        info!("📊 Discovered {} failed records for recovery", failed_records.len());

        // Phase 2: Pre-recovery verification
        if self.config.verification_enabled {
            self.verify_pre_recovery_state(&failed_records).await?;
        }

        // Phase 3: Process records in batches
        self.process_recovery_batches(failed_records).await?;

        // Phase 4: Post-recovery verification and checksums
        if self.config.verification_enabled {
            self.verify_post_recovery_state().await?;
        }

        // Phase 5: Generate final report
        self.stats.processing_time_seconds = start_time.elapsed().as_secs();
        self.generate_recovery_report().await?;

        info!("🎯 Data recovery completed successfully");
        Ok(self.stats.clone())
    }

    /// Discover records that need reprocessing
    #[instrument(skip(self))]
    async fn discover_failed_records(&self) -> Result<Vec<FailedRecord>, sqlx::Error> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, user_id, raw_payload, processing_errors, processing_status,
             payload_size_bytes, created_at FROM raw_ingestions WHERE "
        );

        // Build dynamic WHERE clause based on config
        let mut conditions = Vec::new();

        // Filter by processing status
        if !self.config.target_status.is_empty() {
            let status_list = self.config.target_status
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ");
            conditions.push(format!("processing_status IN ({})", status_list));
        }

        // Filter by date if specified
        if let Some(since_date) = self.config.since_date {
            conditions.push(format!("created_at >= '{}'", since_date));
        }

        // Filter by user if specified
        if let Some(user_id) = self.config.specific_user_id {
            conditions.push(format!("user_id = '{}'", user_id));
        }

        // Add PostgreSQL parameter limit error detection
        conditions.push(
            "(processing_errors::text ILIKE '%parameter%' OR
              processing_errors::text ILIKE '%too many arguments%' OR
              processing_errors::text ILIKE '%exceeding safe limit%' OR
              processing_errors::text ILIKE '%chunk size%')"
                .to_string()
        );

        let where_clause = conditions.join(" AND ");
        query_builder.push(&where_clause);
        query_builder.push(" ORDER BY created_at DESC");

        if self.config.batch_size > 0 {
            query_builder.push(&format!(" LIMIT {}", self.config.batch_size * 10)); // Allow for multiple batches
        }

        let query = query_builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let mut failed_records = Vec::new();
        for row in rows {
            let record = FailedRecord {
                id: row.get("id"),
                user_id: row.get("user_id"),
                raw_payload: row.get("raw_payload"),
                processing_errors: row.try_get("processing_errors").ok(),
                processing_status: row.get("processing_status"),
                payload_size_bytes: row.get("payload_size_bytes"),
                created_at: row.get("created_at"),
            };
            failed_records.push(record);
        }

        Ok(failed_records)
    }

    /// Verify the state before recovery starts
    #[instrument(skip(self, records))]
    async fn verify_pre_recovery_state(&mut self, records: &[FailedRecord]) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔍 Performing pre-recovery verification...");

        // Count existing metrics for each user before recovery
        for record in records {
            let existing_counts = self.count_existing_metrics(record.user_id).await?;
            let checksum = self.calculate_payload_checksum(&record.raw_payload)?;

            self.stats.verification_checksums.insert(
                format!("pre_recovery_{}", record.id),
                checksum,
            );

            info!("✅ Pre-recovery verification for record {}: {} existing metrics",
                  record.id, existing_counts.total());
        }

        Ok(())
    }

    /// Process failed records in manageable batches
    #[instrument(skip(self, records))]
    async fn process_recovery_batches(&mut self, records: Vec<FailedRecord>) -> Result<(), Box<dyn std::error::Error>> {
        let total_batches = (records.len() + self.config.batch_size - 1) / self.config.batch_size;
        info!("📦 Processing {} records in {} batches of {} records each",
              records.len(), total_batches, self.config.batch_size);

        for (batch_idx, batch) in records.chunks(self.config.batch_size).enumerate() {
            info!("🔄 Processing batch {}/{} ({} records)", batch_idx + 1, total_batches, batch.len());

            for (record_idx, record) in batch.iter().enumerate() {
                if (self.stats.processed_records + self.stats.failed_records + self.stats.skipped_records)
                    % self.config.progress_report_interval == 0 {
                    self.report_progress().await;
                }

                match self.process_single_record(record).await {
                    Ok(recovery_result) => {
                        self.update_stats_from_result(record, recovery_result);
                        self.stats.processed_records += 1;
                    }
                    Err(e) => {
                        error!("❌ Failed to process record {}: {}", record.id, e);
                        self.stats.failed_records += 1;
                        self.add_error_to_breakdown(e.to_string());
                    }
                }

                // Small delay to prevent overwhelming the database
                if record_idx % 10 == 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            info!("✅ Completed batch {}/{}", batch_idx + 1, total_batches);
        }

        Ok(())
    }

    /// Process a single failed record
    #[instrument(skip(self, record))]
    async fn process_single_record(&self, record: &FailedRecord) -> Result<RecoveryResult, Box<dyn std::error::Error>> {
        info!("🔄 Processing record {} for user {}", record.id, record.user_id);

        // Parse the raw payload back to IngestPayload
        let payload: IngestPayload = serde_json::from_value(record.raw_payload.clone())
            .map_err(|e| format!("Failed to parse raw payload: {}", e))?;

        let expected_metrics = payload.data.metrics.len() + payload.data.workouts.len();

        if self.config.dry_run {
            info!("🧪 DRY RUN: Would reprocess {} metrics for record {}", expected_metrics, record.id);
            return Ok(RecoveryResult {
                metrics_processed: expected_metrics,
                metrics_failed: 0,
                processing_successful: true,
                errors: Vec::new(),
            });
        }

        // Count metrics before processing
        let before_counts = self.count_existing_metrics(record.user_id).await?;

        // Process the payload with the corrected batch configuration
        let batch_result = self.batch_processor.process_batch(record.user_id, payload).await;

        // Count metrics after processing
        let after_counts = self.count_existing_metrics(record.user_id).await?;

        let metrics_recovered = after_counts.total() - before_counts.total();
        let processing_successful = batch_result.errors.is_empty();

        // Update the raw_ingestions record status
        if processing_successful {
            sqlx::query!(
                "UPDATE raw_ingestions SET
                 processing_status = 'recovered',
                 processing_errors = NULL,
                 processed_at = CURRENT_TIMESTAMP
                 WHERE id = $1",
                record.id
            )
            .execute(&self.pool)
            .await?;

            info!("✅ Successfully recovered record {}: {} metrics processed, {} metrics recovered",
                  record.id, batch_result.processed_count, metrics_recovered);
        } else {
            // Update with new errors but mark as attempted recovery
            let errors_json = serde_json::to_value(&batch_result.errors)?;
            sqlx::query!(
                "UPDATE raw_ingestions SET
                 processing_status = 'recovery_failed',
                 processing_errors = $1,
                 processed_at = CURRENT_TIMESTAMP
                 WHERE id = $2",
                errors_json as sqlx::types::JsonValue,
                record.id
            )
            .execute(&self.pool)
            .await?;

            warn!("⚠️ Recovery attempt for record {} completed with {} errors: {} metrics processed",
                  record.id, batch_result.errors.len(), batch_result.processed_count);
        }

        Ok(RecoveryResult {
            metrics_processed: batch_result.processed_count,
            metrics_failed: batch_result.failed_count,
            processing_successful,
            errors: batch_result.errors.iter().map(|e| e.message.clone()).collect(),
        })
    }

    /// Count existing metrics for a user across all tables
    #[instrument(skip(self))]
    async fn count_existing_metrics(&self, user_id: Uuid) -> Result<MetricCounts, sqlx::Error> {
        let heart_rate = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1",
            user_id
        ).fetch_one(&self.pool).await?.unwrap_or(0) as usize;

        let blood_pressure = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM blood_pressure_metrics WHERE user_id = $1",
            user_id
        ).fetch_one(&self.pool).await?.unwrap_or(0) as usize;

        let sleep = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sleep_metrics WHERE user_id = $1",
            user_id
        ).fetch_one(&self.pool).await?.unwrap_or(0) as usize;

        let activity = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM activity_metrics WHERE user_id = $1",
            user_id
        ).fetch_one(&self.pool).await?.unwrap_or(0) as usize;

        let workouts = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM workout_metrics WHERE user_id = $1",
            user_id
        ).fetch_one(&self.pool).await?.unwrap_or(0) as usize;

        Ok(MetricCounts {
            heart_rate,
            blood_pressure,
            sleep,
            activity,
            workouts,
        })
    }

    /// Calculate SHA256 checksum of payload for verification
    fn calculate_payload_checksum(&self, payload: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
        let payload_bytes = serde_json::to_vec(payload)?;
        let mut hasher = Sha256::new();
        hasher.update(&payload_bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Verify the state after recovery completes
    #[instrument(skip(self))]
    async fn verify_post_recovery_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔍 Performing post-recovery verification...");

        // Get overall recovery statistics
        let recovery_stats = sqlx::query!(
            r#"
            SELECT
                processing_status,
                COUNT(*) as count,
                ROUND(AVG(payload_size_bytes::numeric)/1024/1024, 2) as avg_size_mb
            FROM raw_ingestions
            GROUP BY processing_status
            ORDER BY processing_status
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        info!("📊 Post-recovery processing status breakdown:");
        for stat in recovery_stats {
            let avg_size = stat.avg_size_mb
                .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0);
            info!("  {}: {} records (avg {:.2} MB)",
                  stat.processing_status.unwrap_or("unknown".to_string()),
                  stat.count.unwrap_or(0),
                  avg_size);
        }

        // Verify no data loss occurred during recovery
        let potential_data_loss = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM raw_ingestions
             WHERE processing_status IN ('recovery_failed', 'error')
             AND processing_errors::text ILIKE '%data loss%'"
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        if potential_data_loss > 0 {
            warn!("⚠️ {} records still show potential data loss after recovery", potential_data_loss);
        } else {
            info!("✅ No data loss detected in recovered records");
        }

        Ok(())
    }

    /// Update statistics from a recovery result
    fn update_stats_from_result(&mut self, record: &FailedRecord, result: RecoveryResult) {
        self.stats.total_metrics_recovered += result.metrics_processed;
        self.stats.total_metrics_failed += result.metrics_failed;

        // Update per-user stats
        let user_stats = self.stats.user_recovery_stats
            .entry(record.user_id)
            .or_insert_with(UserRecoveryStats::default);

        user_stats.records_processed += 1;
        user_stats.metrics_recovered += result.metrics_processed;
        user_stats.metrics_failed += result.metrics_failed;
        user_stats.largest_payload_mb = user_stats.largest_payload_mb
            .max(record.payload_size_bytes as f64 / (1024.0 * 1024.0));

        for error in result.errors {
            user_stats.processing_errors.push(error.clone());
            self.add_error_to_breakdown(error);
        }

        // Calculate data loss percentage
        let total_attempted = user_stats.metrics_recovered + user_stats.metrics_failed;
        if total_attempted > 0 {
            user_stats.data_loss_percentage =
                (user_stats.metrics_failed as f64 / total_attempted as f64) * 100.0;
        }
    }

    /// Add error to breakdown statistics
    fn add_error_to_breakdown(&mut self, error: String) {
        // Categorize errors by type
        let error_category = if error.contains("parameter") || error.contains("argument") {
            "PostgreSQL Parameter Limit"
        } else if error.contains("validation") {
            "Data Validation"
        } else if error.contains("duplicate") {
            "Duplicate Key"
        } else if error.contains("connection") {
            "Database Connection"
        } else {
            "Other"
        };

        *self.stats.error_breakdown.entry(error_category.to_string()).or_insert(0) += 1;
    }

    /// Report current progress
    async fn report_progress(&self) {
        let total_processed = self.stats.processed_records + self.stats.failed_records + self.stats.skipped_records;
        let progress_percentage = if self.stats.total_records > 0 {
            (total_processed as f64 / self.stats.total_records as f64) * 100.0
        } else {
            0.0
        };

        info!("📈 Progress: {}/{} records ({:.1}%) - {} recovered, {} failed, {} skipped",
              total_processed, self.stats.total_records, progress_percentage,
              self.stats.processed_records, self.stats.failed_records, self.stats.skipped_records);

        info!("📊 Metrics: {} recovered, {} failed",
              self.stats.total_metrics_recovered, self.stats.total_metrics_failed);
    }

    /// Generate comprehensive recovery report
    #[instrument(skip(self))]
    async fn generate_recovery_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("📋 Generating comprehensive recovery report...");

        info!("🎯 === DATA RECOVERY COMPLETION REPORT ===");
        info!("📊 Overall Statistics:");
        info!("  • Total records processed: {}", self.stats.total_records);
        info!("  • Successfully recovered: {}", self.stats.processed_records);
        info!("  • Failed recovery: {}", self.stats.failed_records);
        info!("  • Skipped records: {}", self.stats.skipped_records);
        info!("  • Total metrics recovered: {}", self.stats.total_metrics_recovered);
        info!("  • Total metrics failed: {}", self.stats.total_metrics_failed);
        info!("  • Processing time: {} seconds", self.stats.processing_time_seconds);

        let success_rate = if self.stats.total_records > 0 {
            (self.stats.processed_records as f64 / self.stats.total_records as f64) * 100.0
        } else {
            0.0
        };
        info!("  • Success rate: {:.2}%", success_rate);

        info!("🔍 Error Breakdown:");
        for (error_type, count) in &self.stats.error_breakdown {
            info!("  • {}: {} occurrences", error_type, count);
        }

        info!("👥 Per-User Recovery Summary:");
        for (user_id, user_stats) in &self.stats.user_recovery_stats {
            info!("  • User {}: {} records, {} metrics recovered, {:.1}% data loss",
                  user_id, user_stats.records_processed, user_stats.metrics_recovered,
                  user_stats.data_loss_percentage);
        }

        // Generate JSON report for automated monitoring
        let report_json = serde_json::to_string_pretty(&self.stats)?;

        // Save report to file for auditing
        let report_filename = format!("data_recovery_report_{}.json",
                                     chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        tokio::fs::write(&report_filename, report_json).await?;

        info!("📄 Detailed report saved to: {}", report_filename);
        info!("✅ === RECOVERY REPORT COMPLETE ===");

        Ok(())
    }
}

/// Represents a failed record to be reprocessed
#[derive(Debug, Clone)]
struct FailedRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub raw_payload: serde_json::Value,
    pub processing_errors: Option<serde_json::Value>,
    pub processing_status: String,
    pub payload_size_bytes: i32,
    pub created_at: DateTime<Utc>,
}

/// Result of processing a single recovery record
#[derive(Debug, Clone)]
struct RecoveryResult {
    pub metrics_processed: usize,
    pub metrics_failed: usize,
    pub processing_successful: bool,
    pub errors: Vec<String>,
}

/// Counts of metrics by type for verification
#[derive(Debug, Clone, Default)]
struct MetricCounts {
    pub heart_rate: usize,
    pub blood_pressure: usize,
    pub sleep: usize,
    pub activity: usize,
    pub workouts: usize,
}

impl MetricCounts {
    fn total(&self) -> usize {
        self.heart_rate + self.blood_pressure + self.sleep + self.activity + self.workouts
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with structured output
    tracing_subscriber::fmt()
        .with_env_filter("info,self_sensored=debug")
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let mut config = RecoveryConfig::default();

    // Simple argument parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => config.dry_run = true,
            "--batch-size" => {
                if i + 1 < args.len() {
                    config.batch_size = args[i + 1].parse().unwrap_or(100);
                    i += 1;
                }
            }
            "--user-id" => {
                if i + 1 < args.len() {
                    config.specific_user_id = Some(Uuid::parse_str(&args[i + 1])?);
                    i += 1;
                }
            }
            "--no-verify" => config.verification_enabled = false,
            "--status" => {
                if i + 1 < args.len() {
                    config.target_status = vec![args[i + 1].clone()];
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;

    info!("🔗 Connected to database");

    // Initialize and run recovery service
    let mut recovery_service = DataRecoveryService::new(pool, config);
    let final_stats = recovery_service.run_recovery().await?;

    // Exit with appropriate code based on results
    if final_stats.failed_records > 0 {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}