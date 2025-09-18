# Data Recovery and Reprocessing Guide

This guide covers the comprehensive data recovery utilities for the Health Export REST API, designed to recover missing metrics from the `raw_ingestions` table after batch processing failures.

## Overview

The data recovery system consists of three main components:
1. **Data Recovery Utility** (`data_recovery`) - Primary recovery tool
2. **Processing Monitor** (`processing_monitor`) - Monitoring and alerting
3. **Legacy Reprocessor** (`reprocess_failed`) - Simple reprocessing tool

## Background

Due to PostgreSQL parameter limit violations (65,535 parameter maximum), some large payloads experienced silent data loss where raw payloads were stored but metrics failed to be processed into their respective tables. The recovery utilities address this by:

- Reprocessing failed payloads with corrected batch configurations
- Verifying data integrity with payload checksums
- Providing comprehensive progress tracking and reporting
- Implementing monitoring to prevent future data loss

## Data Recovery Utility

### Features

- **Comprehensive Recovery**: Processes all failed raw ingestions with PostgreSQL parameter limit errors
- **Progress Tracking**: Real-time progress reporting with batch processing
- **Verification**: SHA256 checksums for payload integrity verification
- **Monitoring Integration**: Detailed recovery statistics and user impact analysis
- **Flexible Configuration**: Supports dry-run mode, specific users, custom batch sizes
- **Error Analysis**: Categorizes and reports error patterns for operational insights

### Usage

```bash
# Basic recovery run (processes all failed records)
cargo run --bin data_recovery

# Dry run to preview what would be recovered
cargo run --bin data_recovery -- --dry-run

# Recover for specific user
cargo run --bin data_recovery -- --user-id "550e8400-e29b-41d4-a716-446655440000"

# Custom batch size and disable verification
cargo run --bin data_recovery -- --batch-size 50 --no-verify

# Recover only specific status types
cargo run --bin data_recovery -- --status "error"
```

### Command Line Options

- `--dry-run`: Preview recovery without making changes
- `--batch-size N`: Process N records per batch (default: 100)
- `--user-id UUID`: Recover only for specific user
- `--no-verify`: Skip payload verification checksums
- `--status STATUS`: Target specific processing status (default: error, partial_success)

### Recovery Process

1. **Discovery Phase**: Identifies failed records with specific error patterns
2. **Pre-Recovery Verification**: Calculates payload checksums and existing metric counts
3. **Batch Processing**: Reprocesses payloads using corrected batch configuration
4. **Post-Recovery Verification**: Validates recovered data and generates statistics
5. **Reporting**: Creates comprehensive recovery report with JSON export

### Output

The utility generates:
- Real-time progress updates in console
- Comprehensive recovery report with statistics
- JSON export file: `data_recovery_report_YYYYMMDD_HHMMSS.json`
- Updated `raw_ingestions` table with recovery status

### Recovery Status Values

- `recovered`: Successfully reprocessed with no errors
- `recovery_failed`: Attempted recovery but still has errors
- `pending`: Not yet processed for recovery

## Processing Monitor

### Features

- **Real-time Monitoring**: Analyzes processing health within configurable time windows
- **Alert System**: Configurable thresholds for critical and warning alerts
- **User Impact Analysis**: Per-user data loss statistics and recommended actions
- **Error Pattern Analysis**: Categorizes errors to identify systemic issues
- **Trend Analysis**: Historical metrics for operational intelligence

### Usage

```bash
# Monitor last 24 hours (default)
cargo run --bin processing_monitor

# Monitor last 6 hours
cargo run --bin processing_monitor 6

# Monitor last week
cargo run --bin processing_monitor 168
```

### Alert Thresholds (Default)

- **Critical Data Loss**: >10% data loss rate
- **Warning Data Loss**: >5% data loss rate
- **Critical Processing Failure**: >20% processing failure rate
- **Warning Processing Failure**: >10% processing failure rate
- **User Success Rate**: <80% requires action

### Monitoring Output

```
=== PROCESSING MONITORING REPORT ===
📊 OVERALL STATISTICS:
  • Total Ingestions: 1,234
  • Successfully Processed: 1,100 (89.1%)
  • Processing Errors: 89 (7.2%)
  • Data Loss Detected: 45

👥 USER IMPACT ANALYSIS:
  • User abc123: 15.2% data loss, 2 days since success, Action: HIGH: Schedule user recovery

🔧 RECOVERY RECOMMENDATIONS:
  • Data loss detected in 45 payloads. Run data recovery utility immediately.
  • PostgreSQL parameter limit violations detected (23 cases). Review batch chunk sizes.
```

## Legacy Reprocessor

The original `reprocess_failed` utility provides basic reprocessing functionality:

```bash
cargo run --bin reprocess_failed
```

This tool is simpler but less comprehensive than the new data recovery utility.

## Environment Configuration

The recovery utilities use the corrected batch configuration to prevent future parameter limit violations:

```bash
# PostgreSQL parameter limit compliance
BATCH_HEART_RATE_CHUNK_SIZE=5242      # 11 params: safe limit
BATCH_BLOOD_PRESSURE_CHUNK_SIZE=8738  # 6 params: optimized
BATCH_SLEEP_CHUNK_SIZE=5242           # 10 params: safe limit
BATCH_ACTIVITY_CHUNK_SIZE=1450        # 36 params: safe with all extensions
BATCH_WORKOUT_CHUNK_SIZE=5000         # 10 params: safe limit

# Recovery utility configuration
RECOVERY_BATCH_SIZE=100               # Records per recovery batch
RECOVERY_MAX_CONCURRENT_JOBS=5        # Parallel recovery jobs
RECOVERY_VERIFICATION_ENABLED=true    # Enable checksum verification
```

## Database Impact

### Raw Ingestions Table

The utilities update `raw_ingestions.processing_status`:

- `recovered`: Successfully reprocessed
- `recovery_failed`: Recovery attempted but failed
- `error`: Original failure status (untouched if not processed)

### Metrics Tables

Recovered metrics are inserted into their respective tables:
- `heart_rate_metrics`
- `blood_pressure_metrics`
- `sleep_metrics`
- `activity_metrics`
- `workout_metrics`
- Additional metric types as supported

## Monitoring Integration

### Prometheus Metrics

The utilities can integrate with monitoring systems:

```
health_export_recovery_processed_total{status="success"} 1234
health_export_recovery_failed_total{status="error"} 56
health_export_data_loss_detected_total 45
```

### Alerting Rules

Example Prometheus alerting rules:

```yaml
- alert: HealthDataLossDetected
  expr: health_export_data_loss_detected_total > 0
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Health data loss detected"
    description: "{{ $value }} payloads with data loss detected"

- alert: HighRecoveryFailureRate
  expr: rate(health_export_recovery_failed_total[1h]) > 0.1
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High recovery failure rate"
```

## Best Practices

### Pre-Recovery

1. **Backup**: Ensure database backups are current
2. **Verification**: Run monitor to assess scope of data loss
3. **Testing**: Use dry-run mode to preview recovery
4. **Resources**: Ensure adequate database connection pool size

### During Recovery

1. **Monitoring**: Watch for memory usage and database load
2. **Progress**: Monitor logs for processing statistics
3. **Errors**: Review any recovery failures immediately
4. **Performance**: Adjust batch size if needed for performance

### Post-Recovery

1. **Verification**: Run monitor again to confirm recovery success
2. **User Communication**: Notify affected users of data restoration
3. **Root Cause**: Review error patterns to prevent recurrence
4. **Documentation**: Update operational runbooks

## Troubleshooting

### Common Issues

**Recovery Fails with "Invalid batch configuration"**
- Solution: Ensure environment variables are set correctly
- Check: `BatchConfig::from_env()` validation passes

**High Memory Usage During Recovery**
- Solution: Reduce batch size with `--batch-size` parameter
- Check: Database connection pool limits

**Verification Checksum Mismatches**
- Solution: Checksums may differ due to JSON serialization order
- Check: Verify payload can still be parsed and processed

**No Records Found for Recovery**
- Solution: Check processing status filters in discovery query
- Check: Verify time window includes failed records

### Error Categories

1. **PostgreSQL Parameter Limit**: Original cause of data loss
2. **Data Validation**: Payload validation failures
3. **Duplicate Key**: Metrics already exist (safe to ignore)
4. **Database Connection**: Connection pool exhaustion
5. **Processing Timeout**: Payloads too large for processing window

## Performance Considerations

### Batch Sizing

- Small batches (50-100): Lower memory usage, slower overall processing
- Large batches (500+): Higher memory usage, faster processing
- Default (100): Balanced approach for most scenarios

### Concurrent Processing

- Limited by database connection pool (default: 10 connections)
- Monitor database load during recovery
- Adjust `max_concurrent_jobs` if needed

### Memory Usage

- Each batch loads full payloads into memory
- Large payloads (>100MB) may require reduced batch sizes
- Monitor system memory during recovery

## Security Considerations

### Data Access

- Recovery utilities require full database access
- Payloads contain sensitive health information
- Ensure proper access controls and audit logging

### Checksum Verification

- SHA256 checksums verify payload integrity
- Checksums stored in recovery statistics for audit
- Verification can be disabled for performance if needed

### Error Logging

- Recovery logs may contain sensitive error details
- Ensure log rotation and retention policies
- Consider log sanitization for production environments

## Integration Testing

The recovery utilities include comprehensive integration tests:

```bash
# Run recovery tests (requires test database)
TEST_DATABASE_URL=postgresql://localhost/health_export_test cargo test data_recovery_tests

# Test specific recovery scenarios
cargo test test_recovery_postgresql_parameter_limit
cargo test test_recovery_verification_checksums
cargo test test_data_loss_monitoring
```

## Operational Runbook

### Daily Monitoring

1. Run processing monitor for last 24 hours
2. Review alert thresholds and recommendations
3. Check for users with high data loss rates
4. Monitor processing failure trends

### Incident Response

1. **Data Loss Alert**: Run recovery utility immediately
2. **High Failure Rate**: Investigate error patterns
3. **User Complaints**: Check individual user impact statistics
4. **System Performance**: Review batch configuration and database load

### Weekly Review

1. Analyze recovery statistics trends
2. Review error pattern analysis
3. Update alert thresholds if needed
4. Plan capacity and performance improvements

This comprehensive data recovery system ensures minimal data loss and provides operational visibility into processing health, enabling proactive identification and resolution of data integrity issues.