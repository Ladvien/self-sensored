# Health Export API - Prometheus Metrics Documentation

## Overview

The Health Export REST API exposes comprehensive Prometheus metrics for monitoring application performance, health, and business KPIs. All metrics are exposed on the `/metrics` endpoint and follow Prometheus naming conventions with the `health_export_` prefix.

## Performance Requirements

- **Metric Collection Overhead**: < 1ms per request (validated through performance tests)
- **Memory Impact**: Minimal - metrics use efficient counter/gauge data structures
- **Cardinality Control**: Endpoint normalization prevents metric explosion

## HTTP Request Metrics

### `health_export_http_requests_total`
**Type**: Counter  
**Description**: Total number of HTTP requests processed by the API  
**Labels**:
- `method`: HTTP method (GET, POST, etc.)
- `endpoint`: Normalized endpoint path (e.g., `/api/v1/ingest`, `/api/v1/data/heart-rate`)
- `status_code`: HTTP response status code (200, 400, 500, etc.)

**Usage**: Track request volume, success rates, and identify problematic endpoints
**Example Queries**:
```promql
# Request rate per second
rate(health_export_http_requests_total[5m])

# Error rate percentage
rate(health_export_http_requests_total{status_code=~"5.."}[5m]) / rate(health_export_http_requests_total[5m]) * 100
```

### `health_export_http_request_duration_seconds`
**Type**: Histogram  
**Description**: HTTP request duration in seconds  
**Labels**:
- `method`: HTTP method
- `endpoint`: Normalized endpoint path
- `status_code`: HTTP response status code

**Buckets**: [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]  
**Usage**: Monitor API response times and SLA compliance  
**Example Queries**:
```promql
# 95th percentile response time
histogram_quantile(0.95, rate(health_export_http_request_duration_seconds_bucket[5m]))

# Average response time
rate(health_export_http_request_duration_seconds_sum[5m]) / rate(health_export_http_request_duration_seconds_count[5m])
```

## Processing Pipeline Metrics

### `health_export_ingest_requests_total`
**Type**: Counter  
**Description**: Total number of ingest requests processed  
**Labels**: None  
**Usage**: Track ingestion volume and detect service interruptions  
**Example Queries**:
```promql
# Ingestion rate
rate(health_export_ingest_requests_total[5m])

# Daily ingestion volume
increase(health_export_ingest_requests_total[24h])
```

### `health_export_ingest_metrics_processed_total`
**Type**: Counter  
**Description**: Total number of health metrics processed during ingestion  
**Labels**:
- `metric_type`: Type of health metric (heart_rate, blood_pressure, sleep, etc.)
- `status`: Processing status (success, failed)

**Usage**: Monitor data processing success rates and identify problematic metric types  
**Example Queries**:
```promql
# Processing success rate
rate(health_export_ingest_metrics_processed_total{status="success"}[5m]) / rate(health_export_ingest_metrics_processed_total[5m]) * 100

# Failed metrics by type
rate(health_export_ingest_metrics_processed_total{status="failed"}[5m])
```

### `health_export_ingest_duration_seconds`
**Type**: Histogram  
**Description**: Duration of complete ingest operations in seconds  
**Labels**:
- `status`: Operation status (success, partial_failure)

**Buckets**: [0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]  
**Usage**: Monitor ingestion performance and identify slow operations  
**Example Queries**:
```promql
# 95th percentile ingestion time
histogram_quantile(0.95, rate(health_export_ingest_duration_seconds_bucket[5m]))
```

### `health_export_batch_processing_duration_seconds`
**Type**: Histogram  
**Description**: Duration of batch processing operations in seconds  
**Labels**:
- `metric_type`: Type of metrics being processed
- `batch_size_bucket`: Size category (small: 0-10, medium: 11-100, large: 101-1000, xlarge: 1000+)

**Buckets**: [0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]  
**Usage**: Monitor batch processing performance by size and type  
**Example Queries**:
```promql
# Processing time by batch size
histogram_quantile(0.95, rate(health_export_batch_processing_duration_seconds_bucket[5m])) by (batch_size_bucket)
```

## Database Connection Pool Metrics

### `health_export_db_connections_active`
**Type**: Gauge  
**Description**: Number of active database connections currently in use  
**Labels**: None  
**Usage**: Monitor database connection utilization  
**Example Queries**:
```promql
# Current active connections
health_export_db_connections_active

# Connection utilization percentage
health_export_db_connections_active / (health_export_db_connections_active + health_export_db_connections_idle) * 100
```

### `health_export_db_connections_idle`
**Type**: Gauge  
**Description**: Number of idle database connections available for use  
**Labels**: None  
**Usage**: Monitor available database capacity  

### `health_export_db_connection_wait_time_seconds`
**Type**: Histogram  
**Description**: Time spent waiting to acquire a database connection in seconds  
**Labels**:
- `operation`: Type of database operation (ingest, query, export)

**Buckets**: [0.000001, 0.00001, 0.0001, 0.001, 0.01, 0.1, 1.0, 10.0]  
**Usage**: Identify database connection pool bottlenecks  
**Example Queries**:
```promql
# 95th percentile wait time
histogram_quantile(0.95, rate(health_export_db_connection_wait_time_seconds_bucket[5m]))
```

## Error Tracking Metrics

### `health_export_errors_total`
**Type**: Counter  
**Description**: Total number of errors by type and endpoint  
**Labels**:
- `error_type`: Category of error (validation, database, json_parse, etc.)
- `endpoint`: Endpoint where error occurred
- `severity`: Error severity level (info, warning, error, critical)

**Usage**: Track error patterns and identify problem areas  
**Example Queries**:
```promql
# Error rate by type
rate(health_export_errors_total[5m]) by (error_type)

# Critical errors only
rate(health_export_errors_total{severity="critical"}[5m])
```

## Custom Business Metrics

### `health_export_active_users_24h`
**Type**: Gauge  
**Description**: Number of unique active users in the last 24 hours  
**Labels**: None  
**Usage**: Monitor user engagement and detect service issues  
**Example Queries**:
```promql
# Current active users
health_export_active_users_24h

# User activity trend
health_export_active_users_24h[24h:1h]
```

### `health_export_data_volume_bytes_total`
**Type**: Counter  
**Description**: Total volume of data processed in bytes  
**Labels**:
- `data_type`: Type of data (ingest, health_data, export)
- `operation`: Operation performed (received, processed, exported)

**Usage**: Monitor data throughput and capacity planning  
**Example Queries**:
```promql
# Data ingestion rate in MB/s
rate(health_export_data_volume_bytes_total{operation="received"}[5m]) / 1048576

# Daily data volume
increase(health_export_data_volume_bytes_total[24h])
```

### `health_export_health_metrics_stored_total`
**Type**: Counter  
**Description**: Total number of health metrics successfully stored by type  
**Labels**:
- `metric_type`: Type of health metric (heart_rate, blood_pressure, sleep, activity, workout)

**Usage**: Track successful data storage by metric type  
**Example Queries**:
```promql
# Metrics stored per second by type
rate(health_export_health_metrics_stored_total[5m]) by (metric_type)
```

## Rate Limiting Metrics

### `health_export_rate_limited_requests_total`
**Type**: Counter  
**Description**: Total number of requests that were rate limited  
**Labels**:
- `endpoint`: Endpoint being rate limited
- `user_id`: Hashed user identifier (for privacy)

**Usage**: Monitor rate limiting effectiveness and identify abuse patterns  
**Example Queries**:
```promql
# Rate limiting activity
rate(health_export_rate_limited_requests_total[5m])

# Most rate-limited endpoints
topk(5, rate(health_export_rate_limited_requests_total[5m]) by (endpoint))
```

## Authentication Metrics

### `health_export_auth_attempts_total`
**Type**: Counter  
**Description**: Total authentication attempts  
**Labels**:
- `result`: Authentication result (success, failure)
- `key_type`: Type of API key used (uuid, hashed)

**Usage**: Monitor authentication patterns and detect potential attacks  
**Example Queries**:
```promql
# Authentication success rate
rate(health_export_auth_attempts_total{result="success"}[5m]) / rate(health_export_auth_attempts_total[5m]) * 100

# Failed authentication rate
rate(health_export_auth_attempts_total{result="failure"}[5m])
```

## Monitoring and Alerting

### Key SLIs (Service Level Indicators)

1. **Availability**: `up{job="health-export-api"} == 1`
2. **Error Rate**: `rate(health_export_http_requests_total{status_code=~"5.."}[5m]) / rate(health_export_http_requests_total[5m])`
3. **Latency P95**: `histogram_quantile(0.95, rate(health_export_http_request_duration_seconds_bucket[5m]))`
4. **Throughput**: `rate(health_export_http_requests_total[5m])`

### Alert Thresholds

- **Critical**: Service down, database pool exhausted, high error rate (>10%)
- **Warning**: High response time (>1s), high connection usage (>80%), slow batch processing (>30s)
- **Info**: Capacity concerns, unusual patterns, low user activity

### Grafana Dashboard

The included Grafana dashboard (`monitoring/grafana-dashboard.json`) provides:
- HTTP request metrics visualization
- Database connection pool monitoring
- Error rate tracking
- Business KPI displays
- Performance trend analysis

### Prometheus Alert Rules

The alert rules (`monitoring/prometheus-alerts.yml`) cover:
- Service health and availability
- Performance degradation
- Error rate thresholds
- Capacity planning warnings
- Business logic anomalies

## Performance Characteristics

### Metric Collection Overhead
- **HTTP Middleware**: < 0.5ms per request (validated through performance tests)
- **Business Metrics**: < 0.1ms per recording operation
- **Database Metrics**: Updated every 10 seconds via background task

### Memory Usage
- **Metric Storage**: ~1KB per unique metric combination
- **Cardinality Control**: Endpoint normalization limits explosion
- **Cleanup**: Prometheus handles metric lifecycle automatically

### Best Practices

1. **Use rate() function** for counter metrics to get per-second values
2. **Use histogram_quantile()** for latency percentiles
3. **Monitor cardinality** to prevent metric explosion
4. **Set appropriate alerting thresholds** based on SLA requirements
5. **Use recording rules** for complex queries used in multiple alerts

## Integration

### With Prometheus
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'health-export-api'
    static_configs:
      - targets: ['api:8080']
    metrics_path: /metrics
    scrape_interval: 15s
```

### With Grafana
Import the dashboard JSON file and configure Prometheus as the data source.

### With Alertmanager
Configure alert routing based on severity labels and component tags for appropriate notification channels.

## Troubleshooting

### Missing Metrics
- Verify `/metrics` endpoint is accessible
- Check that middleware is properly configured in application
- Ensure Prometheus can scrape the endpoint

### High Cardinality
- Review label values for unbounded dimensions (user IDs, timestamps, etc.)
- Use endpoint normalization to reduce path variations
- Monitor Prometheus memory usage

### Performance Impact
- Run performance tests to validate overhead requirements
- Monitor application response times after enabling metrics
- Adjust scrape intervals if needed

## Development

### Adding New Metrics
1. Define metric in `src/middleware/metrics.rs`
2. Add collection points in relevant code
3. Update this documentation
4. Add to Grafana dashboard
5. Consider alert rules if applicable
6. Write tests for new metrics

### Testing
Comprehensive test suite in `tests/middleware/metrics_test.rs` covers:
- Metric collection accuracy
- Performance overhead validation
- Endpoint normalization
- Concurrent access safety
- Business logic integration