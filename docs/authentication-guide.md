# Health Export API Authentication Guide

## Overview

The Health Export API uses Bearer token authentication with support for two distinct API key formats to accommodate different use cases:

1. **UUID Format**: Designed for the Auto Health Export iOS application
2. **Hashed Format**: Used for internal applications and custom integrations

All API endpoints (except health checks) require authentication.

## Authentication Methods

### Bearer Token Authentication

All authenticated requests must include an `Authorization` header with a Bearer token:

```http
Authorization: Bearer <your-api-key>
```

### Supported Token Formats

#### 1. UUID Format (Auto Health Export Compatible)

**Format**: Standard UUID v4 format  
**Use Case**: iOS Auto Health Export application and mobile clients  
**Example**: `550e8400-e29b-41d4-a716-446655440000`

**Request Example**:
```http
POST /api/v1/ingest HTTP/1.1
Host: api.health-export.com
Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000
Content-Type: application/json

{
  "data": {
    "metrics": [...],
    "workouts": [...]
  }
}
```

#### 2. Hashed Format (Internal Use)

**Format**: Argon2-hashed string  
**Use Case**: Internal applications, server-to-server communication  
**Example**: `$argon2id$v=19$m=4096,t=3,p=1$...` (truncated for readability)

**Request Example**:
```http
GET /api/v1/data/heart-rate HTTP/1.1
Host: api.health-export.com
Authorization: Bearer $argon2id$v=19$m=4096,t=3,p=1$c2FsdA$hash...
```

## Getting Your API Key

### For iOS Auto Health Export Users

1. Open the Auto Health Export iOS application
2. Navigate to Settings → API Configuration  
3. Your UUID-format API key will be displayed
4. Copy the key for use in API requests

### For Custom Integrations

1. Contact the Health Export team at team@example.com
2. Provide your use case and expected request volume
3. Receive your hashed-format API key via secure channel
4. Test the key using the `/api/v1/status` endpoint

## Authentication Examples

### cURL Examples

**iOS Format Authentication**:
```bash
# Health data ingestion
curl -X POST "https://api.health-export.com/api/v1/ingest" \
  -H "Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "metrics": [
        {
          "type": "HeartRate",
          "recorded_at": "2025-01-15T08:30:00Z",
          "avg_bpm": 72,
          "source": "Apple Watch"
        }
      ],
      "workouts": []
    }
  }'

# Query heart rate data
curl -X GET "https://api.health-export.com/api/v1/data/heart-rate?start_date=2025-01-01T00:00:00Z" \
  -H "Authorization: Bearer 550e8400-e29b-41d4-a716-446655440000"
```

**Internal Format Authentication**:
```bash
# Check API status
curl -X GET "https://api.health-export.com/api/v1/status" \
  -H "Authorization: Bearer \$argon2id\$v=19\$m=4096,t=3,p=1\$c2FsdA\$hash..."

# Export all data
curl -X GET "https://api.health-export.com/api/v1/export/all?format=json" \
  -H "Authorization: Bearer \$argon2id\$v=19\$m=4096,t=3,p=1\$c2FsdA\$hash..."
```

### JavaScript Examples

**Node.js with UUID Token**:
```javascript
const axios = require('axios');

const apiKey = '550e8400-e29b-41d4-a716-446655440000';
const baseURL = 'https://api.health-export.com';

// Configure axios instance
const healthApi = axios.create({
  baseURL,
  headers: {
    'Authorization': `Bearer ${apiKey}`,
    'Content-Type': 'application/json'
  }
});

// Ingest health data
async function ingestHealthData(healthData) {
  try {
    const response = await healthApi.post('/api/v1/ingest', {
      data: healthData
    });
    
    console.log('Ingest successful:', response.data);
    return response.data;
  } catch (error) {
    if (error.response?.status === 401) {
      console.error('Authentication failed - check your API key');
    } else if (error.response?.status === 429) {
      console.error('Rate limit exceeded:', error.response.headers['retry-after']);
    } else {
      console.error('API error:', error.response?.data || error.message);
    }
    throw error;
  }
}

// Query heart rate data
async function getHeartRateData(startDate, endDate) {
  try {
    const response = await healthApi.get('/api/v1/data/heart-rate', {
      params: {
        start_date: startDate,
        end_date: endDate,
        limit: 100
      }
    });
    
    return response.data.data;
  } catch (error) {
    console.error('Query failed:', error.response?.data || error.message);
    throw error;
  }
}

// Usage
(async () => {
  try {
    // Example health data
    const healthData = {
      metrics: [
        {
          type: "HeartRate",
          recorded_at: "2025-01-15T08:30:00Z",
          avg_bpm: 72,
          source: "Apple Watch"
        }
      ],
      workouts: []
    };
    
    await ingestHealthData(healthData);
    
    const heartRateData = await getHeartRateData(
      '2025-01-01T00:00:00Z',
      '2025-01-15T23:59:59Z'
    );
    
    console.log('Heart rate data:', heartRateData);
    
  } catch (error) {
    console.error('Operation failed:', error.message);
  }
})();
```

**Browser JavaScript (Fetch API)**:
```javascript
const API_KEY = '550e8400-e29b-41d4-a716-446655440000';
const BASE_URL = 'https://api.health-export.com';

// Helper function for authenticated requests
async function healthApiRequest(endpoint, options = {}) {
  const url = `${BASE_URL}${endpoint}`;
  const config = {
    ...options,
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
      'Content-Type': 'application/json',
      ...options.headers
    }
  };
  
  const response = await fetch(url, config);
  
  // Handle authentication errors
  if (response.status === 401) {
    throw new Error('Authentication failed - invalid API key');
  }
  
  // Handle rate limiting
  if (response.status === 429) {
    const retryAfter = response.headers.get('Retry-After');
    throw new Error(`Rate limit exceeded - retry after ${retryAfter} seconds`);
  }
  
  const data = await response.json();
  
  if (!response.ok) {
    throw new Error(data.error || 'API request failed');
  }
  
  return data;
}

// Check API status
async function checkApiStatus() {
  const status = await healthApiRequest('/api/v1/status');
  console.log('API Status:', status);
  return status;
}

// Export health data
async function exportHealthData(format = 'json', startDate = null, endDate = null) {
  const params = new URLSearchParams({ format });
  if (startDate) params.append('start_date', startDate);
  if (endDate) params.append('end_date', endDate);
  
  const endpoint = `/api/v1/export/all?${params.toString()}`;
  const exportData = await healthApiRequest(endpoint);
  
  return exportData;
}
```

### Python Examples

**Python with UUID Token**:
```python
import requests
import json
from datetime import datetime, timezone
from typing import Dict, List, Optional

class HealthExportClient:
    def __init__(self, api_key: str, base_url: str = "https://api.health-export.com"):
        self.api_key = api_key
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json'
        })
    
    def _handle_response(self, response: requests.Response) -> Dict:
        """Handle API response and errors"""
        if response.status_code == 401:
            raise Exception("Authentication failed - check your API key")
        elif response.status_code == 429:
            retry_after = response.headers.get('Retry-After', 'unknown')
            raise Exception(f"Rate limit exceeded - retry after {retry_after} seconds")
        elif not response.ok:
            error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
            raise Exception(f"API error ({response.status_code}): {error_data.get('error', 'Unknown error')}")
        
        return response.json()
    
    def check_status(self) -> Dict:
        """Check API and database status"""
        response = self.session.get(f"{self.base_url}/api/v1/status")
        return self._handle_response(response)
    
    def ingest_health_data(self, metrics: List[Dict], workouts: List[Dict] = None) -> Dict:
        """Ingest health data"""
        payload = {
            "data": {
                "metrics": metrics,
                "workouts": workouts or []
            }
        }
        
        response = self.session.post(
            f"{self.base_url}/api/v1/ingest",
            json=payload
        )
        
        return self._handle_response(response)
    
    def get_heart_rate_data(self, start_date: Optional[str] = None, 
                           end_date: Optional[str] = None, 
                           limit: int = 100) -> Dict:
        """Query heart rate data"""
        params = {'limit': limit}
        if start_date:
            params['start_date'] = start_date
        if end_date:
            params['end_date'] = end_date
        
        response = self.session.get(
            f"{self.base_url}/api/v1/data/heart-rate",
            params=params
        )
        
        return self._handle_response(response)
    
    def export_all_data(self, format: str = 'json', 
                       start_date: Optional[str] = None,
                       end_date: Optional[str] = None) -> Dict:
        """Export all health data"""
        params = {'format': format}
        if start_date:
            params['start_date'] = start_date
        if end_date:
            params['end_date'] = end_date
        
        response = self.session.get(
            f"{self.base_url}/api/v1/export/all",
            params=params
        )
        
        return self._handle_response(response)

# Usage example
def main():
    # Initialize client with UUID API key
    client = HealthExportClient('550e8400-e29b-41d4-a716-446655440000')
    
    try:
        # Check API status
        status = client.check_status()
        print(f"API Status: {status['data']['status']}")
        print(f"Database: {status['data']['database']['status']}")
        
        # Example health metrics
        metrics = [
            {
                "type": "HeartRate",
                "recorded_at": datetime.now(timezone.utc).isoformat(),
                "avg_bpm": 72,
                "min_bpm": 68,
                "max_bpm": 78,
                "source": "Apple Watch",
                "context": "resting"
            },
            {
                "type": "BloodPressure", 
                "recorded_at": datetime.now(timezone.utc).isoformat(),
                "systolic": 120,
                "diastolic": 80,
                "pulse": 68,
                "source": "Manual Entry"
            }
        ]
        
        # Ingest data
        result = client.ingest_health_data(metrics)
        print(f"Ingested {result['data']['processed_count']} metrics successfully")
        
        # Query heart rate data from last 30 days
        start_date = (datetime.now(timezone.utc) - datetime.timedelta(days=30)).isoformat()
        heart_rate_data = client.get_heart_rate_data(start_date=start_date)
        print(f"Retrieved {len(heart_rate_data['data']['data'])} heart rate records")
        
        # Export all data as JSON
        export_result = client.export_all_data(format='json', start_date=start_date)
        print(f"Exported {export_result['data']['record_count']} records")
        
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
```

## Rate Limiting

The API implements rate limiting to ensure fair usage and system stability:

- **Request Limit**: 100 requests per hour per API key (configurable)
- **Payload Size**: 100MB maximum per request
- **Metrics per Request**: 10,000 maximum health metrics per request

### Rate Limit Headers

All responses include rate limit information in headers:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 85
X-RateLimit-Reset: 1642248600
```

### Handling Rate Limits

When you exceed the rate limit, the API returns HTTP 429:

```json
{
  "success": false,
  "data": null,
  "error": "Rate limit exceeded. Try again in 3600 seconds.",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

**Best Practices**:
- Monitor the `X-RateLimit-Remaining` header
- Implement exponential backoff when receiving 429 responses
- Cache data locally when possible to reduce API calls
- Use batch operations to maximize data per request

## Error Handling

### Authentication Errors

**HTTP 401 - Unauthorized**:
```json
{
  "success": false,
  "data": null,
  "error": "Invalid or missing authentication token",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

Common causes:
- Missing `Authorization` header
- Invalid token format
- Expired or revoked API key
- Incorrect Bearer token syntax

### Example Error Handling

```javascript
async function handleApiRequest(apiCall) {
  try {
    const response = await apiCall();
    return response;
  } catch (error) {
    if (error.response?.status === 401) {
      // Authentication error - refresh token or notify user
      console.error('Authentication failed:', error.response.data.error);
      // Redirect to login or refresh token
    } else if (error.response?.status === 429) {
      // Rate limit - implement retry with backoff
      const retryAfter = error.response.headers['retry-after'];
      console.log(`Rate limited - retrying after ${retryAfter} seconds`);
      setTimeout(() => handleApiRequest(apiCall), retryAfter * 1000);
    } else {
      // Other errors
      console.error('API error:', error.response?.data?.error || error.message);
    }
    throw error;
  }
}
```

## Security Best Practices

### API Key Management

1. **Store Securely**: Never hardcode API keys in source code
2. **Environment Variables**: Use environment variables or secure configuration
3. **Rotation**: Regularly rotate API keys, especially for internal applications
4. **Scope Limitation**: Request only the minimum required permissions
5. **Monitoring**: Monitor API key usage for unusual patterns

### HTTPS Only

- All API communication must use HTTPS
- The API automatically redirects HTTP requests to HTTPS
- Certificate pinning is recommended for mobile applications

### Request Validation

- Validate all input data before sending to the API
- Use proper data types and ranges as specified in the OpenAPI specification
- Implement client-side validation to reduce unnecessary API calls

## Testing Authentication

### Test Your API Key

Use the status endpoint to verify your authentication is working:

```bash
curl -X GET "https://api.health-export.com/api/v1/status" \
  -H "Authorization: Bearer YOUR_API_KEY_HERE"
```

Expected response for valid authentication:
```json
{
  "success": true,
  "data": {
    "status": "operational",
    "timestamp": "2025-01-15T10:30:00Z",
    "service": "self-sensored-api",
    "version": "0.1.0",
    "database": {
      "status": "connected"
    },
    "environment": "production"
  },
  "error": null,
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### Common Test Scenarios

1. **Valid Authentication**: Should return 200 with status data
2. **Missing Token**: Should return 401 with authentication error
3. **Invalid Format**: Should return 401 with format error
4. **Expired Token**: Should return 401 with expiration error

## Troubleshooting

### Authentication Issues

| Issue | Status Code | Solution |
|-------|-------------|----------|
| Missing Authorization header | 401 | Add `Authorization: Bearer <token>` header |
| Invalid token format | 401 | Verify UUID format or hash format is correct |
| Expired/revoked token | 401 | Obtain new API key from team |
| Wrong authentication type | 401 | Use "Bearer" not "Basic" or other types |

### Rate Limiting Issues

| Issue | Status Code | Solution |
|-------|-------------|----------|
| Too many requests | 429 | Wait for reset time, implement backoff |
| Payload too large | 413 | Reduce batch size below 100MB |
| Too many metrics | 400 | Limit to 10,000 metrics per request |

## Support

For authentication issues or API key requests:

- **Email**: team@example.com
- **Documentation**: [API Documentation](https://docs.health-export.com)
- **Status Page**: [Status Page](https://status.health-export.com)

Include the following information in support requests:
- API key ID (not the full key for security)
- Timestamp of failed requests
- HTTP status codes and error messages
- Request/response examples (with sensitive data redacted)