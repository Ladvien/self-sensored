# iOS Metric Type Coverage Monitoring

## Overview

This monitoring system provides comprehensive visibility into iOS metric type processing, conversion rates, and data loss prevention. It tracks the full pipeline from iOS metric ingestion to internal metric conversion.

## Metrics Collected

### 1. iOS Metric Type Distribution
**Metric**: `health_export_ios_metric_type_distribution_total`
**Type**: Counter
**Labels**:
- `ios_metric_type`: The iOS metric name (e.g., "HKQuantityTypeIdentifierHeartRate")
- `processing_result`: "encountered", "converted", "dropped", "error"

**Purpose**: Track how many of each iOS metric type are processed and their outcomes.

### 2. iOS Metric Conversion Success Rate
**Metric**: `health_export_ios_metric_conversion_success_rate`
**Type**: Gauge (0.0 to 1.0)
**Labels**:
- `ios_metric_type`: The iOS metric name
- `internal_metric_type`: The internal metric type (e.g., "heart_rate", "activity")

**Purpose**: Monitor conversion success rates for each iOS metric type.

### 3. Unknown iOS Metric Types
**Metric**: `health_export_ios_unknown_metric_types_total`
**Type**: Counter
**Labels**:
- `ios_metric_type`: The unknown metric name
- `criticality_level`: "critical", "high", "medium", "low"

**Purpose**: Track unknown iOS metric types by severity for prioritization.

### 4. iOS Fallback Cases
**Metric**: `health_export_ios_fallback_cases_total`
**Type**: Counter
**Labels**:
- `fallback_type`: "wildcard_match", "default_case", "partial_match"
- `ios_metric_pattern`: Pattern that matched the fallback

**Purpose**: Monitor metrics hitting fallback processing patterns.

### 5. iOS Metric Type Coverage Ratio
**Metric**: `health_export_ios_metric_type_coverage_ratio`
**Type**: Gauge (0.0 to 1.0)

**Purpose**: Overall coverage quality metric (supported types / total encountered).

### 6. iOS Metric Data Loss
**Metric**: `health_export_ios_metric_data_loss_total`
**Type**: Counter
**Labels**:
- `loss_reason`: "unsupported_type", "conversion_error", "validation_failed"
- `ios_metric_type`: The metric type that was lost
- `severity`: "critical", "high", "medium", "low"

**Purpose**: Track data loss events with detailed categorization.

### 7. HealthKit Identifier Usage
**Metric**: `health_export_ios_healthkit_identifier_usage_total`
**Type**: Counter
**Labels**:
- `identifier_type`: "healthkit_identifier", "simplified_name", "legacy_format"
- `metric_category`: "heart_rate", "activity", "sleep", etc.

**Purpose**: Monitor usage patterns of HealthKit identifiers vs simplified names.

## Alerting Rules

### Critical Alerts

#### New Critical Unknown Metric Types
- **Alert**: `iOSUnknownMetricTypeCritical`
- **Condition**: Any critical unknown metric type detected
- **Response Time**: Immediate (0 minutes)
- **Action**: Implement mapping immediately

#### Critical Data Loss
- **Alert**: `iOSMetricDataLossCritical`
- **Condition**: Any critical severity data loss
- **Response Time**: Immediate (0 minutes)
- **Action**: Investigate and fix root cause

### Warning Alerts

#### High-Priority Unknown Metrics
- **Alert**: `iOSUnknownMetricTypeHigh`
- **Condition**: High-priority unknown metrics for >2 minutes
- **Response Time**: 15 minutes
- **Action**: Add to implementation backlog

#### Low Coverage Ratio
- **Alert**: `iOSMetricCoverageLow`
- **Condition**: Coverage ratio < 80% for >5 minutes
- **Response Time**: 30 minutes
- **Action**: Review and implement missing mappings

#### Low Conversion Success Rate
- **Alert**: `iOSConversionSuccessRateLow`
- **Condition**: Average conversion rate < 90% for >10 minutes
- **Response Time**: 30 minutes
- **Action**: Investigate conversion failures

### Monitoring Alerts

#### Spike in Unknown Types
- **Alert**: `iOSUnknownMetricTypeSpike`
- **Condition**: >50 unknown metrics in 10 minutes
- **Response Time**: 5 minutes
- **Action**: Check for new iOS app version

#### High Fallback Usage
- **Alert**: `iOSFallbackCasesHigh`
- **Condition**: >100 fallback cases in 30 minutes
- **Response Time**: 15 minutes
- **Action**: Review fallback patterns

## Dashboard Configuration

### Overview Panel
- iOS Metric Type Coverage Ratio (stat panel with thresholds)
- Unknown iOS Metric Types in last hour (stat panel)
- Data Loss Events in last hour (stat panel)

### Distribution Analysis
- iOS Metric Type Distribution (pie chart showing processing results)
- Top Unknown iOS Metric Types (table)

### Conversion Monitoring
- iOS Metric Conversion Success Rate by Type (time series)
- HealthKit Identifier vs Simplified Name Usage (bar gauge)

### Data Loss Analysis
- Fallback Cases by Pattern (time series)
- Data Loss by Reason and Severity (heatmap)

## Implementation Details

### Monitoring Integration Points

1. **iOS Metric Processing** (`src/models/ios_models.rs`)
   - Track every iOS metric type encountered
   - Record conversion outcomes
   - Classify unknown types by criticality

2. **Conversion Logic**
   - Monitor successful conversions
   - Track fallback cases
   - Record data loss events

3. **Aggregation**
   - Calculate coverage statistics
   - Update conversion success rates
   - Generate summary metrics

### Data Collection Strategy

- **Real-time tracking**: Every metric processed generates monitoring data
- **Batch aggregation**: Summary statistics calculated per processing session
- **Historical analysis**: Trends tracked over time for pattern detection

### Performance Considerations

- Minimal overhead: <1ms per metric for monitoring calls
- Async processing: Monitoring doesn't block main processing pipeline
- Efficient storage: Cardinality control through label limiting

## Operational Playbooks

### High Data Loss Alert Response

1. **Immediate Assessment** (0-5 minutes)
   - Check Grafana dashboard for data loss patterns
   - Identify affected metric types and severity
   - Check if it's a new unknown type or conversion failure

2. **Investigation** (5-15 minutes)
   - Review recent iOS app deployments or changes
   - Check logs for detailed error information
   - Analyze payload samples with unknown types

3. **Resolution** (15+ minutes)
   - For unknown types: Implement mapping if critical/high priority
   - For conversion errors: Fix validation or processing logic
   - For new HealthKit identifiers: Add to supported mappings

### Coverage Degradation Response

1. **Trend Analysis** (0-10 minutes)
   - Check if coverage drop is gradual or sudden
   - Identify which metric types are causing the drop
   - Review new unknown metric types discovered

2. **Prioritization** (10-20 minutes)
   - Classify new types by medical importance
   - Estimate implementation effort
   - Plan implementation schedule

3. **Implementation** (20+ minutes)
   - Add mappings for high-priority types
   - Update validation rules
   - Test with sample payloads

## Configuration Files

- **Grafana Dashboard**: `/monitoring/grafana-dashboards/ios-metric-type-coverage.json`
- **Prometheus Alerts**: `/monitoring/prometheus/ios-metric-alerts.yml`
- **Monitoring Code**: `/src/middleware/metrics.rs`

## Testing

### Monitoring Tests

1. **Unit Tests**: Verify metric recording functions
2. **Integration Tests**: Test full monitoring pipeline
3. **Alert Tests**: Verify alert thresholds and conditions
4. **Dashboard Tests**: Validate dashboard queries and visualizations

### Test Data

Use sample iOS payloads with:
- Known supported metric types
- Known unsupported metric types
- New unknown metric types
- Edge cases and error conditions

## Maintenance

### Regular Tasks

- **Weekly**: Review unknown metric types and prioritize implementations
- **Monthly**: Analyze coverage trends and conversion rates
- **Quarterly**: Update alert thresholds based on production patterns

### Dashboard Updates

- Add new metric types to monitoring as they're implemented
- Update alert thresholds based on production experience
- Enhance visualizations based on operational needs

## Troubleshooting

### Common Issues

1. **Missing Metrics**: Check if monitoring calls are in the right code paths
2. **High Cardinality**: Limit label values to prevent metric explosion
3. **Alert Fatigue**: Tune thresholds based on production patterns
4. **Dashboard Performance**: Optimize queries for large time ranges

### Debug Information

- Enable debug logging for detailed metric type analysis
- Use structured logging with correlation IDs for tracing
- Monitor metric collection performance impact