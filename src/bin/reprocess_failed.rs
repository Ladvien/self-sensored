use sqlx::PgPool;
use std::env;
use tracing::{error, info, warn};

use self_sensored::models::IngestPayload;
use self_sensored::services::batch_processor::BatchProcessor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;

    info!("Connected to database, checking for failed raw_ingestions...");

    // Get all failed raw_ingestions that had PostgreSQL parameter limit errors
    let failed_records = sqlx::query!(
        r#"
        SELECT id, user_id, raw_payload, processing_errors
        FROM raw_ingestions
        WHERE processing_status = 'error'
          AND (
            processing_errors::text ILIKE '%123500 parameters%' OR
            processing_errors::text ILIKE '%too many arguments for query%' OR
            processing_errors::text ILIKE '%exceeding safe limit%'
          )
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&pool)
    .await?;

    info!("Found {} failed records to reprocess", failed_records.len());

    if failed_records.is_empty() {
        info!("No failed records found to reprocess");
        return Ok(());
    }

    // Initialize batch processor with corrected configuration
    let batch_processor = BatchProcessor::new(pool.clone());

    let mut processed_count = 0;
    let mut failed_count = 0;

    for record in failed_records {
        info!("Reprocessing raw_ingestion: {}", record.id);

        // Parse the raw payload back to IngestPayload
        let payload: IngestPayload = match serde_json::from_value(record.raw_payload) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to parse raw payload for {}: {}", record.id, e);
                failed_count += 1;
                continue;
            }
        };

        // Process the payload with the corrected batch configuration
        let result = batch_processor.process_batch(record.user_id, payload).await;

        if result.errors.is_empty() {
            // Update status to processed
            sqlx::query!(
                "UPDATE raw_ingestions SET processing_status = 'processed', processing_errors = NULL, processed_at = CURRENT_TIMESTAMP WHERE id = $1",
                record.id
            )
            .execute(&pool)
            .await?;

            info!("✅ Successfully reprocessed {}: {} metrics processed",
                  record.id, result.processed_count);
            processed_count += 1;
        } else {
            // Update with new errors
            let errors_json = serde_json::to_value(&result.errors)?;
            sqlx::query!(
                "UPDATE raw_ingestions SET processing_errors = $1, processed_at = CURRENT_TIMESTAMP WHERE id = $2",
                errors_json as sqlx::types::JsonValue,
                record.id
            )
            .execute(&pool)
            .await?;

            warn!("⚠️ Reprocessed {} with {} errors: {} processed, {} failed",
                  record.id, result.errors.len(), result.processed_count, result.failed_count);
            failed_count += 1;
        }
    }

    info!("🎯 Reprocessing complete: {} successfully processed, {} failed",
          processed_count, failed_count);

    // Show final statistics
    let stats = sqlx::query!(
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
    .fetch_all(&pool)
    .await?;

    info!("📊 Final statistics:");
    for stat in stats {
        let avg_size = stat.avg_size_mb
            .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);
        info!("  {}: {} records (avg {:.2} MB)",
              stat.processing_status.unwrap_or("unknown".to_string()),
              stat.count.unwrap_or(0),
              avg_size);
    }

    Ok(())
}