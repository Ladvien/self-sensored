use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Monitoring metrics for data processing integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMonitorMetrics {
    pub timestamp: DateTime<Utc>,
    pub total_raw_ingestions: i64,
    pub processed_successfully: i64,
    pub processing_errors: i64,
    pub partial_success: i64,
    pub pending_processing: i64,
    pub data_loss_detected: i64,
    pub average_processing_time_mins: f64,
    pub largest_payload_mb: f64,
    pub smallest_payload_mb: f64,
    pub user_impact_stats: HashMap<Uuid, UserImpactStats>,
    pub error_pattern_analysis: HashMap<String, i64>,
    pub recovery_recommendations: Vec<String>,
}

/// Per-user impact statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserImpactStats {
    pub total_payloads: i64,
    pub failed_payloads: i64,
    pub data_loss_percentage: f64,
    pub largest_failed_payload_mb: f64,
    pub days_since_last_success: i64,
    pub recommended_action: String,
}

/// Processing monitor for detecting data integrity issues
pub struct ProcessingMonitor {
    pool: PgPool,
    monitoring_window_hours: i64,
    alert_thresholds: AlertThresholds,
}

/// Configurable thresholds for alerts
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub data_loss_percentage_critical: f64,
    pub data_loss_percentage_warning: f64,
    pub processing_failure_rate_critical: f64,
    pub processing_failure_rate_warning: f64,
    pub max_acceptable_processing_delay_hours: i64,
    pub minimum_user_success_rate: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            data_loss_percentage_critical: 10.0, // >10% data loss is critical
            data_loss_percentage_warning: 5.0,   // >5% data loss is warning
            processing_failure_rate_critical: 20.0, // >20% processing failures critical
            processing_failure_rate_warning: 10.0, // >10% processing failures warning
            max_acceptable_processing_delay_hours: 24, // Payloads pending >24h need attention
            minimum_user_success_rate: 80.0,     // Users with <80% success rate need help
        }
    }
}

impl ProcessingMonitor {
    pub fn new(pool: PgPool, monitoring_window_hours: i64) -> Self {
        Self {
            pool,
            monitoring_window_hours,
            alert_thresholds: AlertThresholds::default(),
        }
    }

    /// Run comprehensive monitoring analysis
    #[instrument(skip(self))]
    pub async fn run_monitoring_analysis(
        &self,
    ) -> Result<ProcessingMonitorMetrics, Box<dyn std::error::Error>> {
        info!("🔍 Starting processing integrity monitoring analysis");

        let since_timestamp = Utc::now() - Duration::hours(self.monitoring_window_hours);

        // Gather core metrics
        let raw_stats = self.gather_raw_ingestion_stats(since_timestamp).await?;
        let user_impact = self.analyze_user_impact(since_timestamp).await?;
        let error_patterns = self.analyze_error_patterns(since_timestamp).await?;
        let recovery_recommendations = self
            .generate_recovery_recommendations(&raw_stats, &user_impact, &error_patterns)
            .await;

        let metrics = ProcessingMonitorMetrics {
            timestamp: Utc::now(),
            total_raw_ingestions: raw_stats.total,
            processed_successfully: raw_stats.successful,
            processing_errors: raw_stats.errors,
            partial_success: raw_stats.partial_success,
            pending_processing: raw_stats.pending,
            data_loss_detected: raw_stats.data_loss_detected,
            average_processing_time_mins: raw_stats.avg_processing_time_mins,
            largest_payload_mb: raw_stats.largest_payload_mb,
            smallest_payload_mb: raw_stats.smallest_payload_mb,
            user_impact_stats: user_impact,
            error_pattern_analysis: error_patterns,
            recovery_recommendations,
        };

        // Evaluate alerts
        self.evaluate_and_report_alerts(&metrics).await;

        // Save metrics for trending
        self.save_monitoring_metrics(&metrics).await?;

        Ok(metrics)
    }

    /// Gather raw ingestion statistics
    #[instrument(skip(self))]
    async fn gather_raw_ingestion_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<RawIngestionStats, sqlx::Error> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE processing_status = 'processed') as successful,
                COUNT(*) FILTER (WHERE processing_status = 'error') as errors,
                COUNT(*) FILTER (WHERE processing_status = 'partial_success') as partial_success,
                COUNT(*) FILTER (WHERE processing_status = 'pending') as pending,
                COUNT(*) FILTER (WHERE processing_status = 'recovered') as recovered,
                COUNT(*) FILTER (WHERE processing_status = 'recovery_failed') as recovery_failed,
                COALESCE(AVG(EXTRACT(EPOCH FROM (processed_at - created_at))/60.0), 0) as avg_processing_time_mins,
                COALESCE(MAX(payload_size_bytes::numeric/1024/1024), 0) as max_payload_mb,
                COALESCE(MIN(payload_size_bytes::numeric/1024/1024), 0) as min_payload_mb
            FROM raw_ingestions
            WHERE created_at >= $1
            "#,
            since
        )
        .fetch_one(&self.pool)
        .await?;

        // Check for potential data loss indicators
        let data_loss_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM raw_ingestions
            WHERE created_at >= $1
              AND (
                processing_errors::text ILIKE '%parameter%' OR
                processing_errors::text ILIKE '%too many arguments%' OR
                processing_errors::text ILIKE '%data loss%' OR
                processing_errors::text ILIKE '%chunk size%' OR
                processing_status = 'recovery_failed'
              )
            "#,
            since
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(RawIngestionStats {
            total: stats.total.unwrap_or(0),
            successful: stats.successful.unwrap_or(0),
            errors: stats.errors.unwrap_or(0),
            partial_success: stats.partial_success.unwrap_or(0),
            pending: stats.pending.unwrap_or(0),
            recovered: stats.recovered.unwrap_or(0),
            recovery_failed: stats.recovery_failed.unwrap_or(0),
            data_loss_detected: data_loss_count,
            avg_processing_time_mins: stats
                .avg_processing_time_mins
                .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0),
            largest_payload_mb: stats
                .max_payload_mb
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            smallest_payload_mb: stats
                .min_payload_mb
                .map(|d| d.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
        })
    }

    /// Analyze per-user impact
    #[instrument(skip(self))]
    async fn analyze_user_impact(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<Uuid, UserImpactStats>, sqlx::Error> {
        let user_stats = sqlx::query!(
            r#"
            SELECT
                user_id,
                COUNT(*) as total_payloads,
                COUNT(*) FILTER (WHERE processing_status IN ('error', 'recovery_failed')) as failed_payloads,
                MAX(CASE WHEN processing_status IN ('error', 'recovery_failed')
                    THEN payload_size_bytes::numeric/1024/1024
                    ELSE 0 END) as largest_failed_mb,
                COALESCE(EXTRACT(DAYS FROM (NOW() - MAX(CASE WHEN processing_status = 'processed'
                    THEN processed_at END))), 999) as days_since_success
            FROM raw_ingestions
            WHERE created_at >= $1
            GROUP BY user_id
            "#,
            since
        )
        .fetch_all(&self.pool)
        .await?;

        let mut impact_map = HashMap::new();

        for stat in user_stats {
            let user_id = stat.user_id;
            let total = stat.total_payloads.unwrap_or(0) as f64;
            let failed = stat.failed_payloads.unwrap_or(0) as f64;
            let data_loss_percentage = if total > 0.0 {
                (failed / total) * 100.0
            } else {
                0.0
            };

            let days_since_success = stat
                .days_since_success
                .as_ref()
                .map(|bd| bd.to_string().parse::<i64>().unwrap_or(999))
                .unwrap_or(999);

            let recommended_action =
                self.recommend_user_action(data_loss_percentage, days_since_success);

            let impact = UserImpactStats {
                total_payloads: total as i64,
                failed_payloads: failed as i64,
                data_loss_percentage,
                largest_failed_payload_mb: stat
                    .largest_failed_mb
                    .map(|d| d.to_string().parse().unwrap_or(0.0))
                    .unwrap_or(0.0),
                days_since_last_success: days_since_success,
                recommended_action,
            };

            impact_map.insert(user_id, impact);
        }

        Ok(impact_map)
    }

    /// Analyze error patterns to identify systemic issues
    #[instrument(skip(self))]
    async fn analyze_error_patterns(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<String, i64>, sqlx::Error> {
        let error_records = sqlx::query!(
            r#"
            SELECT processing_errors
            FROM raw_ingestions
            WHERE created_at >= $1
              AND processing_errors IS NOT NULL
            "#,
            since
        )
        .fetch_all(&self.pool)
        .await?;

        let mut pattern_counts = HashMap::new();

        for record in error_records {
            if let Some(errors) = record.processing_errors {
                // Parse and categorize errors
                if let Ok(errors_array) = serde_json::from_value::<Vec<serde_json::Value>>(errors) {
                    for error in errors_array {
                        let error_text = error.to_string().to_lowercase();

                        let category = if error_text.contains("parameter")
                            || error_text.contains("argument")
                        {
                            "PostgreSQL Parameter Limit"
                        } else if error_text.contains("validation") {
                            "Data Validation Error"
                        } else if error_text.contains("duplicate") {
                            "Duplicate Key Violation"
                        } else if error_text.contains("connection") || error_text.contains("pool") {
                            "Database Connection Issues"
                        } else if error_text.contains("timeout") {
                            "Processing Timeout"
                        } else if error_text.contains("memory") {
                            "Memory Limit Exceeded"
                        } else if error_text.contains("json") || error_text.contains("parse") {
                            "Payload Parsing Error"
                        } else {
                            "Other Processing Error"
                        };

                        *pattern_counts.entry(category.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok(pattern_counts)
    }

    /// Generate recovery recommendations based on analysis
    async fn generate_recovery_recommendations(
        &self,
        raw_stats: &RawIngestionStats,
        user_impact: &HashMap<Uuid, UserImpactStats>,
        error_patterns: &HashMap<String, i64>,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check overall data loss
        let total_failures = raw_stats.errors + raw_stats.recovery_failed;
        let failure_rate = if raw_stats.total > 0 {
            (total_failures as f64 / raw_stats.total as f64) * 100.0
        } else {
            0.0
        };

        if failure_rate > self.alert_thresholds.processing_failure_rate_critical {
            recommendations.push(format!(
                "CRITICAL: {:.1}% processing failure rate detected. Immediate investigation required.",
                failure_rate
            ));
        }

        if raw_stats.data_loss_detected > 0 {
            recommendations.push(format!(
                "Data loss detected in {} payloads. Run data recovery utility immediately.",
                raw_stats.data_loss_detected
            ));
        }

        // Check for PostgreSQL parameter limit issues
        if let Some(param_errors) = error_patterns.get("PostgreSQL Parameter Limit") {
            if *param_errors > 10 {
                recommendations.push(format!(
                    "PostgreSQL parameter limit violations detected ({} cases). Review batch chunk sizes.",
                    param_errors
                ));
            }
        }

        // Check for users with high data loss
        let high_impact_users = user_impact
            .iter()
            .filter(|(_, stats)| {
                stats.data_loss_percentage > self.alert_thresholds.minimum_user_success_rate
            })
            .count();

        if high_impact_users > 0 {
            recommendations.push(format!(
                "{} users experiencing high data loss rates. Consider individual user recovery.",
                high_impact_users
            ));
        }

        // Check for pending processing backlog
        if raw_stats.pending > 100 {
            recommendations.push(format!(
                "{} payloads pending processing. Consider background job scaling.",
                raw_stats.pending
            ));
        }

        // Check processing time trends
        if raw_stats.avg_processing_time_mins > 10.0 {
            recommendations.push(format!(
                "Average processing time {:.1} minutes is high. Review performance optimizations.",
                raw_stats.avg_processing_time_mins
            ));
        }

        if recommendations.is_empty() {
            recommendations
                .push("No critical issues detected. System processing normally.".to_string());
        }

        recommendations
    }

    /// Recommend action for a specific user
    fn recommend_user_action(&self, data_loss_percentage: f64, days_since_success: i64) -> String {
        if data_loss_percentage > 50.0 || days_since_success > 7 {
            "URGENT: Contact user support immediately"
        } else if data_loss_percentage > 20.0 || days_since_success > 3 {
            "HIGH: Schedule user recovery within 24 hours"
        } else if data_loss_percentage > 10.0 || days_since_success > 1 {
            "MEDIUM: Monitor and recover if trend continues"
        } else {
            "LOW: Normal processing"
        }
        .to_string()
    }

    /// Evaluate thresholds and generate alerts
    #[instrument(skip(self, metrics))]
    async fn evaluate_and_report_alerts(&self, metrics: &ProcessingMonitorMetrics) {
        let total_processed =
            metrics.processed_successfully + metrics.processing_errors + metrics.partial_success;
        let failure_rate = if total_processed > 0 {
            ((metrics.processing_errors + metrics.data_loss_detected) as f64
                / total_processed as f64)
                * 100.0
        } else {
            0.0
        };

        // Critical alerts
        if failure_rate > self.alert_thresholds.processing_failure_rate_critical {
            error!("🚨 CRITICAL ALERT: Processing failure rate {:.1}% exceeds critical threshold {:.1}%",
                  failure_rate, self.alert_thresholds.processing_failure_rate_critical);
        }

        if metrics.data_loss_detected > 0 {
            error!(
                "🚨 CRITICAL ALERT: Data loss detected in {} payloads",
                metrics.data_loss_detected
            );
        }

        if metrics.pending_processing > (total_processed / 10).max(100) {
            error!(
                "🚨 CRITICAL ALERT: {} payloads pending processing (backlog)",
                metrics.pending_processing
            );
        }

        // Warning alerts
        if failure_rate > self.alert_thresholds.processing_failure_rate_warning {
            warn!(
                "⚠️ WARNING: Processing failure rate {:.1}% exceeds warning threshold {:.1}%",
                failure_rate, self.alert_thresholds.processing_failure_rate_warning
            );
        }

        // User-specific alerts
        let high_impact_users = metrics
            .user_impact_stats
            .iter()
            .filter(|(_, stats)| {
                stats.data_loss_percentage > self.alert_thresholds.minimum_user_success_rate
            })
            .count();

        if high_impact_users > 0 {
            warn!(
                "⚠️ WARNING: {} users experiencing high data loss rates",
                high_impact_users
            );
        }

        // Success metrics
        if failure_rate < self.alert_thresholds.processing_failure_rate_warning {
            info!(
                "✅ System processing normally: {:.1}% failure rate",
                failure_rate
            );
        }
    }

    /// Save monitoring metrics for historical analysis
    #[instrument(skip(self, metrics))]
    async fn save_monitoring_metrics(
        &self,
        metrics: &ProcessingMonitorMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let metrics_json = serde_json::to_value(metrics)?;

        // Try to save to monitoring_metrics table if it exists
        let save_result = sqlx::query(
            r#"
            INSERT INTO monitoring_metrics (
                timestamp,
                metric_type,
                metrics_data,
                created_at
            ) VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(metrics.timestamp)
        .bind("processing_monitor")
        .bind(metrics_json)
        .execute(&self.pool)
        .await;

        match save_result {
            Ok(_) => {
                info!("📊 Monitoring metrics saved to database");
            }
            Err(e) => {
                // If table doesn't exist, just log and continue
                warn!(
                    "Could not save monitoring metrics to database (table may not exist): {}",
                    e
                );
                info!(
                    "💡 To enable historical monitoring, create monitoring_metrics table in schema"
                );
            }
        }

        Ok(())
    }

    /// Generate detailed report for operations team
    #[instrument(skip(self, metrics))]
    pub async fn generate_operations_report(&self, metrics: &ProcessingMonitorMetrics) -> String {
        let mut report = format!(
            "
=== PROCESSING MONITORING REPORT ===
Timestamp: {}
Monitoring Window: {} hours

📊 OVERALL STATISTICS:
  • Total Ingestions: {}
  • Successfully Processed: {} ({:.1}%)
  • Processing Errors: {} ({:.1}%)
  • Partial Success: {} ({:.1}%)
  • Pending Processing: {}
  • Data Loss Detected: {}
  • Recovered Records: Available in raw_ingestions

⏱️ PERFORMANCE METRICS:
  • Average Processing Time: {:.1} minutes
  • Largest Payload: {:.1} MB
  • Smallest Payload: {:.1} MB

🚨 ERROR ANALYSIS:
",
            metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.monitoring_window_hours,
            metrics.total_raw_ingestions,
            metrics.processed_successfully,
            (metrics.processed_successfully as f64 / metrics.total_raw_ingestions as f64) * 100.0,
            metrics.processing_errors,
            (metrics.processing_errors as f64 / metrics.total_raw_ingestions as f64) * 100.0,
            metrics.partial_success,
            (metrics.partial_success as f64 / metrics.total_raw_ingestions as f64) * 100.0,
            metrics.pending_processing,
            metrics.data_loss_detected,
            metrics.average_processing_time_mins,
            metrics.largest_payload_mb,
            metrics.smallest_payload_mb
        );

        for (error_type, count) in &metrics.error_pattern_analysis {
            report.push_str(&format!("  • {}: {} occurrences\n", error_type, count));
        }

        report.push_str("\n👥 USER IMPACT ANALYSIS:\n");
        let high_impact_users: Vec<_> = metrics
            .user_impact_stats
            .iter()
            .filter(|(_, stats)| stats.data_loss_percentage > 10.0)
            .collect();

        if high_impact_users.is_empty() {
            report.push_str("  • No users experiencing significant data loss\n");
        } else {
            for (user_id, stats) in high_impact_users {
                report.push_str(&format!(
                    "  • User {}: {:.1}% data loss, {} days since success, Action: {}\n",
                    user_id,
                    stats.data_loss_percentage,
                    stats.days_since_last_success,
                    stats.recommended_action
                ));
            }
        }

        report.push_str("\n🔧 RECOVERY RECOMMENDATIONS:\n");
        for recommendation in &metrics.recovery_recommendations {
            report.push_str(&format!("  • {}\n", recommendation));
        }

        report.push_str("\n=== END REPORT ===\n");

        report
    }
}

/// Raw ingestion statistics structure
#[derive(Debug, Clone)]
struct RawIngestionStats {
    pub total: i64,
    pub successful: i64,
    pub errors: i64,
    pub partial_success: i64,
    pub pending: i64,
    pub recovered: i64,
    pub recovery_failed: i64,
    pub data_loss_detected: i64,
    pub avg_processing_time_mins: f64,
    pub largest_payload_mb: f64,
    pub smallest_payload_mb: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,self_sensored=debug")
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let monitoring_window_hours = if args.len() > 1 {
        args[1].parse().unwrap_or(24)
    } else {
        24
    };

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;

    info!("🔗 Connected to database");

    // Initialize monitoring service
    let monitor = ProcessingMonitor::new(pool, monitoring_window_hours);

    // Run monitoring analysis
    let metrics = monitor.run_monitoring_analysis().await?;

    // Generate and display operations report
    let report = monitor.generate_operations_report(&metrics).await;
    println!("{}", report);

    // Save report to file
    let report_filename = format!(
        "processing_monitor_report_{}.txt",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    tokio::fs::write(&report_filename, &report).await?;

    info!("📄 Report saved to: {}", report_filename);

    // Exit with appropriate code based on critical issues
    if metrics.data_loss_detected > 0
        || metrics.processing_errors > (metrics.total_raw_ingestions / 10).max(10)
    {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}
