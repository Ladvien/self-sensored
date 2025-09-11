# Rate Limiting and Error Handling Guide

## Rate Limiting Overview

The Health Export API implements comprehensive rate limiting to ensure fair usage, prevent abuse, and maintain system stability for all users.

### Rate Limit Policies

#### Request-Based Limits
- **Default Limit**: 100 requests per hour per API key (IP-based by default)
- **Window**: Sliding 1-hour window (3600 seconds)
- **Granularity**: Per API key, tracked individually (configurable per-user)
- **Reset**: Sliding window that resets continuously

#### Data-Based Limits
- **Payload Size**: Maximum 100MB per request
- **Metrics per Request**: Maximum 10,000 health metrics per request
- **Workouts per Request**: Maximum 1,000 workout records per request
- **Concurrent Requests**: Maximum 5 simultaneous requests per API key

#### Specialized Limits by Endpoint

| Endpoint | Specific Limits | Notes |
|----------|----------------|-------|
| `/api/v1/ingest` | 50 requests/hour | More restrictive due to write operations |
| `/api/v1/data/*` | 100 requests/hour | Standard query limit |
| `/api/v1/export/*` | 10 requests/hour | Limited due to resource intensity |
| `/health`, `/api/v1/status` | Unlimited | Health checks exempt from limits |

### Rate Limit Headers

All API responses include rate limiting information in HTTP headers:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 85
X-RateLimit-Reset: 1642248600
X-RateLimit-Window: 3600
```

#### Header Descriptions

- `X-RateLimit-Limit`: Maximum requests allowed in the current window
- `X-RateLimit-Remaining`: Number of requests remaining in current window
- `X-RateLimit-Reset`: Unix timestamp when the current window resets
- `X-RateLimit-Window`: Length of the rate limit window in seconds

### Rate Limit Exceeded Response

When rate limits are exceeded, the API returns HTTP 429:

```json
{
  "success": false,
  "data": null,
  "error": "Rate limit exceeded. Try again in 3600 seconds.",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

Additional headers are included:
```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1642252200
Retry-After: 3600
Content-Type: application/json
```

## Error Codes and Responses

### Standard Error Response Format

All errors follow a consistent JSON structure:

```json
{
  "success": false,
  "data": null,
  "error": "Error description",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

For some endpoints that support partial success, errors may include data:

```json
{
  "success": false,
  "data": {
    "processed_count": 23,
    "failed_count": 2,
    "errors": [...]
  },
  "error": "2 metrics failed validation",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### HTTP Status Codes

#### 2xx Success Codes

| Code | Description | Usage |
|------|-------------|-------|
| 200 | OK | Successful GET, POST, PUT requests |
| 201 | Created | Successful resource creation |
| 204 | No Content | Successful DELETE operations |

#### 4xx Client Error Codes

| Code | Description | Common Causes | Example Response |
|------|-------------|---------------|------------------|
| 400 | Bad Request | Invalid JSON, validation errors, constraint violations | `{"success": false, "error": "Invalid JSON format"}` |
| 401 | Unauthorized | Missing/invalid authentication token | `{"success": false, "error": "Invalid authentication token"}` |
| 403 | Forbidden | Valid auth but insufficient permissions | `{"success": false, "error": "Insufficient permissions"}` |
| 404 | Not Found | Endpoint doesn't exist or resource not found | `{"success": false, "error": "Endpoint not found"}` |
| 405 | Method Not Allowed | Wrong HTTP method for endpoint | `{"success": false, "error": "Method POST not allowed"}` |
| 409 | Conflict | Data conflicts or constraint violations | `{"success": false, "error": "Duplicate data detected"}` |
| 413 | Payload Too Large | Request body exceeds 100MB limit | `{"success": false, "error": "Payload exceeds 100MB limit"}` |
| 422 | Unprocessable Entity | Valid JSON but semantic validation errors | `{"success": false, "error": "Heart rate 350 BPM exceeds maximum"}` |
| 429 | Too Many Requests | Rate limit exceeded | `{"success": false, "error": "Rate limit exceeded"}` |

#### 5xx Server Error Codes

| Code | Description | When It Occurs | Action |
|------|-------------|----------------|---------|
| 500 | Internal Server Error | Unexpected server issues | Retry after delay, contact support |
| 502 | Bad Gateway | Upstream service issues | Retry after delay |
| 503 | Service Unavailable | Planned maintenance or overload | Check status page, retry later |
| 504 | Gateway Timeout | Request timeout (>30 seconds) | Reduce payload size, retry |

### Detailed Error Scenarios

#### Authentication Errors (401)

**Missing Authorization Header**:
```json
{
  "success": false,
  "data": null,
  "error": "Missing Authorization header",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Invalid Token Format**:
```json
{
  "success": false,
  "data": null,
  "error": "Invalid token format. Expected: Bearer <token>",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Expired/Revoked Token**:
```json
{
  "success": false,
  "data": null,
  "error": "Token expired or revoked",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

#### Validation Errors (400/422)

**JSON Parsing Error**:
```json
{
  "success": false,
  "data": null,
  "error": "Invalid JSON format: expected value at line 5 column 12",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Constraint Violation**:
```json
{
  "success": false,
  "data": null,
  "error": "Too many metrics: 15000 exceeds limit of 10000",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Field Validation Error**:
```json
{
  "success": false,
  "data": {
    "processed_count": 23,
    "failed_count": 2,
    "processing_time_ms": 1180,
    "errors": [
      {
        "metric_type": "HeartRate",
        "error_message": "Heart rate 350 BPM exceeds maximum of 300",
        "index": 5
      },
      {
        "metric_type": "BloodPressure",
        "error_message": "Systolic pressure must be between 60-250 mmHg",
        "index": 12
      }
    ]
  },
  "error": "2 metrics failed validation",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

#### Payload Errors (413)

**Payload Too Large**:
```json
{
  "success": false,
  "data": null,
  "error": "Payload size 120MB exceeds maximum of 100MB",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

#### Server Errors (500)

**Database Connection Error**:
```json
{
  "success": false,
  "data": null,
  "error": "Database temporarily unavailable. Please try again later.",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Processing Timeout**:
```json
{
  "success": false,
  "data": null,
  "error": "Request timeout. Please reduce payload size and try again.",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

## Rate Limiting Implementation Details

### Redis-Based Distributed Rate Limiting

The API uses Redis for distributed rate limiting with in-memory fallback:

```
Key Pattern: ratelimit:{api_key_id}:{endpoint}:{window}
Value: {count: 45, reset_time: 1642248600}
TTL: Window duration (3600 seconds)
```

### Sliding Window Algorithm

The API implements a sliding window rate limiting algorithm:

1. **Window Creation**: Each request creates or updates a sliding window
2. **Request Counting**: Increments counter for the current window
3. **Limit Check**: Compares current count to limit
4. **Window Sliding**: Old requests automatically expire from the window

### Bypass Conditions

Certain conditions bypass rate limiting:
- Health check endpoints (`/health`, `/api/v1/status`)
- Internal service requests (marked with special headers)
- Emergency override mode (admin-only)

## Client Implementation Best Practices

### Handling Rate Limits

#### 1. Monitor Headers
Always check rate limit headers before making requests:

```javascript
function checkRateLimit(response) {
  const remaining = parseInt(response.headers['x-ratelimit-remaining']);
  const resetTime = parseInt(response.headers['x-ratelimit-reset']);
  const currentTime = Math.floor(Date.now() / 1000);
  
  if (remaining < 5) {
    const waitTime = resetTime - currentTime;
    console.warn(`Rate limit low: ${remaining} remaining, resets in ${waitTime}s`);
  }
}
```

#### 2. Implement Exponential Backoff
When receiving 429 responses, implement exponential backoff:

```python
import time
import random

def api_request_with_backoff(request_func, max_retries=5):
    for attempt in range(max_retries):
        try:
            response = request_func()
            return response
        except RateLimitError as e:
            if attempt == max_retries - 1:
                raise
            
            # Exponential backoff with jitter
            wait_time = (2 ** attempt) + random.uniform(0, 1)
            print(f"Rate limited, waiting {wait_time:.2f} seconds...")
            time.sleep(wait_time)
```

#### 3. Batch Operations Efficiently
Maximize data per request to reduce API calls:

```python
def batch_health_data(metrics, batch_size=5000):
    """Split metrics into optimal batches for API ingestion"""
    for i in range(0, len(metrics), batch_size):
        batch = metrics[i:i + batch_size]
        yield {
            "data": {
                "metrics": batch,
                "workouts": []
            }
        }

# Usage
metrics = load_health_metrics()
for batch_payload in batch_health_data(metrics):
    result = ingest_health_data(batch_payload)
    print(f"Processed {result['processed_count']} metrics")
    
    # Respect rate limits between batches
    time.sleep(1)
```

#### 4. Cache Responses
Cache API responses to reduce redundant requests:

```javascript
class HealthApiClient {
  constructor(apiKey) {
    this.apiKey = apiKey;
    this.cache = new Map();
  }
  
  async getHeartRateData(startDate, endDate, useCache = true) {
    const cacheKey = `heart-rate-${startDate}-${endDate}`;
    
    if (useCache && this.cache.has(cacheKey)) {
      const cached = this.cache.get(cacheKey);
      const age = Date.now() - cached.timestamp;
      
      // Use cache if less than 5 minutes old
      if (age < 5 * 60 * 1000) {
        return cached.data;
      }
    }
    
    const response = await this.apiRequest('/api/v1/data/heart-rate', {
      params: { start_date: startDate, end_date: endDate }
    });
    
    // Cache successful responses
    this.cache.set(cacheKey, {
      data: response,
      timestamp: Date.now()
    });
    
    return response;
  }
}
```

### Error Recovery Strategies

#### 1. Categorize Errors
Implement different strategies based on error types:

```python
class ApiErrorHandler:
    @staticmethod
    def handle_error(error):
        status_code = error.response.status_code
        
        if status_code == 401:
            # Authentication error - refresh token
            return "refresh_auth"
        elif status_code == 429:
            # Rate limit - retry with backoff
            return "retry_with_backoff"
        elif status_code in [500, 502, 503, 504]:
            # Server error - retry with exponential backoff
            return "retry_server_error"
        elif status_code in [400, 422]:
            # Client error - fix data and retry
            return "fix_data"
        else:
            # Other errors - fail fast
            return "fail_fast"
```

#### 2. Circuit Breaker Pattern
Implement circuit breaker for handling sustained failures:

```javascript
class CircuitBreaker {
  constructor(failureThreshold = 5, timeoutDuration = 60000) {
    this.failureThreshold = failureThreshold;
    this.timeoutDuration = timeoutDuration;
    this.failureCount = 0;
    this.lastFailureTime = null;
    this.state = 'CLOSED'; // CLOSED, OPEN, HALF_OPEN
  }
  
  async execute(operation) {
    if (this.state === 'OPEN') {
      if (Date.now() - this.lastFailureTime < this.timeoutDuration) {
        throw new Error('Circuit breaker is OPEN');
      } else {
        this.state = 'HALF_OPEN';
      }
    }
    
    try {
      const result = await operation();
      this.reset();
      return result;
    } catch (error) {
      this.recordFailure();
      throw error;
    }
  }
  
  recordFailure() {
    this.failureCount++;
    this.lastFailureTime = Date.now();
    
    if (this.failureCount >= this.failureThreshold) {
      this.state = 'OPEN';
    }
  }
  
  reset() {
    this.failureCount = 0;
    this.state = 'CLOSED';
  }
}
```

## Rate Limit Monitoring and Alerting

### Monitoring Metrics
Track these metrics for rate limit management:
- Request rate per API key
- Rate limit hit frequency
- Error rate by status code
- Average response time
- Payload size distribution

### Alert Conditions
Set up alerts for:
- API key approaching rate limit (>90% of limit)
- High rate limit hit rate (>5% of requests)
- Sustained high error rates
- Large payload requests (>50MB)

### Dashboard Queries

**Prometheus/Grafana Examples**:
```promql
# Rate limit utilization
rate_limit_utilization = (requests_per_hour / rate_limit) * 100

# Error rate by status code
error_rate = rate(http_requests_total{status=~"4..|5.."}[5m]) / rate(http_requests_total[5m]) * 100

# P95 response time
http_request_duration_p95 = histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

## Troubleshooting Guide

### Common Rate Limiting Issues

| Problem | Symptoms | Solution |
|---------|----------|----------|
| Frequent 429 errors | High rate limit hit rate | Implement request batching, caching |
| Slow API responses | High P95 latency | Reduce payload sizes, optimize queries |
| Authentication failures | 401 errors after success | Check token expiration, implement refresh |
| Payload too large | 413 errors | Split large batches, compress data |
| Request timeouts | 504 errors | Reduce complexity, implement pagination |

### Debugging Rate Limits

**Check Current Rate Limit Status**:
```bash
curl -I "https://api.health-export.com/api/v1/status" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  | grep -E "X-RateLimit-"
```

**Monitor Rate Limit Consumption**:
```python
def monitor_rate_limits(api_client, duration_minutes=60):
    start_time = time.time()
    end_time = start_time + (duration_minutes * 60)
    
    while time.time() < end_time:
        try:
            response = api_client.check_status()
            headers = response.headers
            
            print(f"Time: {time.strftime('%H:%M:%S')}")
            print(f"Remaining: {headers.get('X-RateLimit-Remaining')}")
            print(f"Reset: {headers.get('X-RateLimit-Reset')}")
            print("---")
            
            time.sleep(60)  # Check every minute
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(60)
```

### Support and Escalation

For persistent rate limiting or error issues:

1. **Gather Information**:
   - API key ID (not the full key)
   - Timestamps of failed requests
   - Request/response examples
   - Error patterns and frequency

2. **Contact Support**:
   - Email: team@example.com
   - Include rate limit headers from recent requests
   - Specify current usage patterns and requirements

3. **Escalation Path**:
   - Level 1: Standard support (24-48 hours)
   - Level 2: Engineering team (urgent issues)
   - Level 3: Rate limit adjustment requests

## Best Practices Summary

1. **Monitor Rate Limits**: Always check headers and plan requests accordingly
2. **Implement Backoff**: Use exponential backoff for 429 responses
3. **Batch Efficiently**: Maximize data per request while staying under limits
4. **Cache Responses**: Avoid redundant API calls through intelligent caching
5. **Handle Errors Gracefully**: Implement appropriate retry logic for different error types
6. **Plan for Scale**: Design your application to handle rate limits from the start
7. **Monitor and Alert**: Set up monitoring to detect and respond to issues proactively