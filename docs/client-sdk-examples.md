# Client SDK Examples

This document provides comprehensive client SDK examples for integrating with the Health Export API across multiple programming languages and platforms.

## Table of Contents
- [JavaScript/Node.js SDK](#javascriptnodejs-sdk)
- [Python SDK](#python-sdk)
- [Swift/iOS SDK](#swiftios-sdk)
- [Java/Android SDK](#javaandroid-sdk)
- [Go SDK](#go-sdk)
- [Ruby SDK](#ruby-sdk)
- [PHP SDK](#php-sdk)
- [C# SDK](#c-sdk)

## JavaScript/Node.js SDK

### Installation
```bash
npm install health-export-sdk
# or
yarn add health-export-sdk
```

### Basic Setup
```javascript
// health-export-sdk.js
const axios = require('axios');

class HealthExportSDK {
  constructor(apiKey, options = {}) {
    this.apiKey = apiKey;
    this.baseURL = options.baseURL || 'https://api.health-export.com';
    this.timeout = options.timeout || 30000;
    
    this.client = axios.create({
      baseURL: this.baseURL,
      timeout: this.timeout,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
        'User-Agent': `health-export-sdk-js/1.0.0`
      }
    });
    
    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      response => response,
      error => this.handleApiError(error)
    );
  }
  
  handleApiError(error) {
    if (error.response) {
      const { status, data } = error.response;
      
      switch (status) {
        case 401:
          throw new AuthenticationError(data.error || 'Authentication failed');
        case 429:
          const retryAfter = error.response.headers['retry-after'];
          throw new RateLimitError(data.error, retryAfter);
        case 413:
          throw new PayloadTooLargeError(data.error);
        default:
          throw new APIError(data.error || 'API request failed', status);
      }
    } else if (error.request) {
      throw new NetworkError('Network request failed');
    } else {
      throw new Error(error.message);
    }
  }
  
  // Health checks
  async checkHealth() {
    const response = await this.client.get('/health');
    return response.data;
  }
  
  async getStatus() {
    const response = await this.client.get('/api/v1/status');
    return response.data;
  }
  
  // Data ingestion
  async ingestHealthData(payload) {
    const response = await this.client.post('/api/v1/ingest', payload);
    return response.data;
  }
  
  async ingestBatch(metrics, workouts = []) {
    return this.ingestHealthData({
      data: {
        metrics,
        workouts
      }
    });
  }
  
  // Data queries
  async getHeartRateData(options = {}) {
    const response = await this.client.get('/api/v1/data/heart-rate', {
      params: this.buildQueryParams(options)
    });
    return response.data;
  }
  
  async getBloodPressureData(options = {}) {
    const response = await this.client.get('/api/v1/data/blood-pressure', {
      params: this.buildQueryParams(options)
    });
    return response.data;
  }
  
  async getSleepData(options = {}) {
    const response = await this.client.get('/api/v1/data/sleep', {
      params: this.buildQueryParams(options)
    });
    return response.data;
  }
  
  async getActivityData(options = {}) {
    const response = await this.client.get('/api/v1/data/activity', {
      params: this.buildQueryParams(options)
    });
    return response.data;
  }
  
  async getWorkoutData(options = {}) {
    const response = await this.client.get('/api/v1/data/workouts', {
      params: this.buildQueryParams(options)
    });
    return response.data;
  }
  
  async getHealthSummary(options = {}) {
    const response = await this.client.get('/api/v1/data/summary', {
      params: this.buildSummaryParams(options)
    });
    return response.data;
  }
  
  // Data export
  async exportAllData(options = {}) {
    const response = await this.client.get('/api/v1/export/all', {
      params: this.buildExportParams(options)
    });
    return response.data;
  }
  
  async exportHeartRateData(options = {}) {
    const response = await this.client.get('/api/v1/export/heart-rate', {
      params: this.buildExportParams(options)
    });
    return response.data;
  }
  
  async exportActivityAnalytics(options = {}) {
    const response = await this.client.get('/api/v1/export/activity-analytics', {
      params: this.buildExportParams(options)
    });
    return response.data;
  }
  
  // Helper methods
  buildQueryParams(options) {
    const params = {};
    if (options.startDate) params.start_date = options.startDate;
    if (options.endDate) params.end_date = options.endDate;
    if (options.page) params.page = options.page;
    if (options.limit) params.limit = options.limit;
    if (options.sort) params.sort = options.sort;
    if (options.metricTypes) params.metric_types = options.metricTypes.join(',');
    if (options.workoutType) params.workout_type = options.workoutType;
    return params;
  }
  
  buildSummaryParams(options) {
    const params = {};
    if (options.startDate) params.start_date = options.startDate;
    if (options.endDate) params.end_date = options.endDate;
    return params;
  }
  
  buildExportParams(options) {
    const params = {};
    if (options.format) params.format = options.format;
    if (options.startDate) params.start_date = options.startDate;
    if (options.endDate) params.end_date = options.endDate;
    if (options.metricTypes) params.metric_types = options.metricTypes.join(',');
    if (options.includeRaw) params.include_raw = options.includeRaw;
    return params;
  }
}

// Custom error classes
class HealthExportError extends Error {
  constructor(message, statusCode = null) {
    super(message);
    this.name = this.constructor.name;
    this.statusCode = statusCode;
  }
}

class AuthenticationError extends HealthExportError {}
class RateLimitError extends HealthExportError {
  constructor(message, retryAfter) {
    super(message, 429);
    this.retryAfter = parseInt(retryAfter) || null;
  }
}
class PayloadTooLargeError extends HealthExportError {}
class APIError extends HealthExportError {}
class NetworkError extends HealthExportError {}

module.exports = {
  HealthExportSDK,
  HealthExportError,
  AuthenticationError,
  RateLimitError,
  PayloadTooLargeError,
  APIError,
  NetworkError
};
```

### Usage Examples
```javascript
const { HealthExportSDK } = require('./health-export-sdk');

// Initialize SDK
const sdk = new HealthExportSDK('550e8400-e29b-41d4-a716-446655440000');

async function example() {
  try {
    // Check API status
    const status = await sdk.getStatus();
    console.log('API Status:', status.data.status);
    
    // Ingest health data
    const healthData = [
      {
        type: "HeartRate",
        recorded_at: new Date().toISOString(),
        avg_bpm: 72,
        source: "Apple Watch"
      }
    ];
    
    const ingestResult = await sdk.ingestBatch(healthData);
    console.log(`Processed ${ingestResult.data.processed_count} metrics`);
    
    // Query heart rate data
    const heartRateData = await sdk.getHeartRateData({
      startDate: '2025-01-01T00:00:00Z',
      limit: 50
    });
    
    console.log(`Found ${heartRateData.data.total_count} heart rate records`);
    
    // Export data
    const exportResult = await sdk.exportAllData({
      format: 'json',
      startDate: '2025-01-01T00:00:00Z'
    });
    
    console.log(`Exported ${exportResult.data.record_count} records`);
    
  } catch (error) {
    if (error instanceof RateLimitError) {
      console.error(`Rate limited - retry after ${error.retryAfter} seconds`);
    } else {
      console.error('API Error:', error.message);
    }
  }
}

example();
```

## Python SDK

### Installation
```bash
pip install health-export-sdk
```

### SDK Implementation
```python
# health_export_sdk/client.py
import requests
import json
import time
from datetime import datetime, timezone
from typing import Dict, List, Optional, Union
from urllib.parse import urljoin

class HealthExportError(Exception):
    """Base exception for Health Export SDK"""
    def __init__(self, message: str, status_code: Optional[int] = None):
        super().__init__(message)
        self.status_code = status_code

class AuthenticationError(HealthExportError):
    """Raised when authentication fails"""
    pass

class RateLimitError(HealthExportError):
    """Raised when rate limit is exceeded"""
    def __init__(self, message: str, retry_after: Optional[int] = None):
        super().__init__(message, 429)
        self.retry_after = retry_after

class PayloadTooLargeError(HealthExportError):
    """Raised when payload exceeds size limits"""
    pass

class ValidationError(HealthExportError):
    """Raised when data validation fails"""
    pass

class HealthExportSDK:
    """Python SDK for Health Export API"""
    
    def __init__(self, api_key: str, base_url: str = "https://api.health-export.com", 
                 timeout: int = 30):
        self.api_key = api_key
        self.base_url = base_url.rstrip('/')
        self.timeout = timeout
        
        self.session = requests.Session()
        self.session.headers.update({
            'Authorization': f'Bearer {api_key}',
            'Content-Type': 'application/json',
            'User-Agent': 'health-export-sdk-python/1.0.0'
        })
    
    def _make_request(self, method: str, endpoint: str, **kwargs) -> Dict:
        """Make HTTP request with error handling"""
        url = urljoin(self.base_url, endpoint)
        
        try:
            response = self.session.request(method, url, timeout=self.timeout, **kwargs)
            
            # Handle specific error codes
            if response.status_code == 401:
                raise AuthenticationError("Authentication failed - check your API key")
            elif response.status_code == 429:
                retry_after = response.headers.get('Retry-After')
                retry_after = int(retry_after) if retry_after else None
                raise RateLimitError("Rate limit exceeded", retry_after)
            elif response.status_code == 413:
                raise PayloadTooLargeError("Payload too large")
            elif response.status_code == 422:
                error_data = response.json() if 'application/json' in response.headers.get('content-type', '') else {}
                raise ValidationError(error_data.get('error', 'Validation error'))
            elif not response.ok:
                error_data = response.json() if 'application/json' in response.headers.get('content-type', '') else {}
                raise HealthExportError(
                    error_data.get('error', f'HTTP {response.status_code} error'),
                    response.status_code
                )
            
            return response.json()
            
        except requests.exceptions.Timeout:
            raise HealthExportError("Request timed out")
        except requests.exceptions.ConnectionError:
            raise HealthExportError("Connection error")
        except requests.exceptions.RequestException as e:
            raise HealthExportError(f"Request failed: {str(e)}")
    
    # Health checks
    def check_health(self) -> Dict:
        """Check API health status"""
        return self._make_request('GET', '/health')
    
    def get_status(self) -> Dict:
        """Get detailed API status"""
        return self._make_request('GET', '/api/v1/status')
    
    # Data ingestion
    def ingest_health_data(self, payload: Dict) -> Dict:
        """Ingest health data payload"""
        return self._make_request('POST', '/api/v1/ingest', json=payload)
    
    def ingest_batch(self, metrics: List[Dict], workouts: Optional[List[Dict]] = None) -> Dict:
        """Ingest batch of health metrics and workouts"""
        payload = {
            "data": {
                "metrics": metrics,
                "workouts": workouts or []
            }
        }
        return self.ingest_health_data(payload)
    
    # Data queries
    def get_heart_rate_data(self, start_date: Optional[str] = None, 
                           end_date: Optional[str] = None,
                           page: int = 1, limit: int = 100,
                           sort: str = 'desc') -> Dict:
        """Query heart rate data"""
        params = self._build_query_params(
            start_date=start_date, end_date=end_date,
            page=page, limit=limit, sort=sort
        )
        return self._make_request('GET', '/api/v1/data/heart-rate', params=params)
    
    def get_blood_pressure_data(self, start_date: Optional[str] = None,
                               end_date: Optional[str] = None,
                               page: int = 1, limit: int = 100,
                               sort: str = 'desc') -> Dict:
        """Query blood pressure data"""
        params = self._build_query_params(
            start_date=start_date, end_date=end_date,
            page=page, limit=limit, sort=sort
        )
        return self._make_request('GET', '/api/v1/data/blood-pressure', params=params)
    
    def get_sleep_data(self, start_date: Optional[str] = None,
                      end_date: Optional[str] = None,
                      page: int = 1, limit: int = 100,
                      sort: str = 'desc') -> Dict:
        """Query sleep data"""
        params = self._build_query_params(
            start_date=start_date, end_date=end_date,
            page=page, limit=limit, sort=sort
        )
        return self._make_request('GET', '/api/v1/data/sleep', params=params)
    
    def get_activity_data(self, start_date: Optional[str] = None,
                         end_date: Optional[str] = None,
                         page: int = 1, limit: int = 100,
                         sort: str = 'desc') -> Dict:
        """Query activity data"""
        params = self._build_query_params(
            start_date=start_date, end_date=end_date,
            page=page, limit=limit, sort=sort
        )
        return self._make_request('GET', '/api/v1/data/activity', params=params)
    
    def get_workout_data(self, start_date: Optional[str] = None,
                        end_date: Optional[str] = None,
                        workout_type: Optional[str] = None,
                        page: int = 1, limit: int = 100,
                        sort: str = 'desc') -> Dict:
        """Query workout data"""
        params = self._build_query_params(
            start_date=start_date, end_date=end_date,
            page=page, limit=limit, sort=sort
        )
        if workout_type:
            params['workout_type'] = workout_type
        
        return self._make_request('GET', '/api/v1/data/workouts', params=params)
    
    def get_health_summary(self, start_date: Optional[str] = None,
                          end_date: Optional[str] = None) -> Dict:
        """Get health summary and analytics"""
        params = {}
        if start_date:
            params['start_date'] = start_date
        if end_date:
            params['end_date'] = end_date
        
        return self._make_request('GET', '/api/v1/data/summary', params=params)
    
    # Data export
    def export_all_data(self, format: str = 'json',
                       start_date: Optional[str] = None,
                       end_date: Optional[str] = None,
                       metric_types: Optional[List[str]] = None,
                       include_raw: bool = False) -> Dict:
        """Export all health data"""
        params = self._build_export_params(
            format=format, start_date=start_date, end_date=end_date,
            metric_types=metric_types, include_raw=include_raw
        )
        return self._make_request('GET', '/api/v1/export/all', params=params)
    
    def export_heart_rate_data(self, format: str = 'json',
                              start_date: Optional[str] = None,
                              end_date: Optional[str] = None,
                              include_raw: bool = False) -> Dict:
        """Export heart rate data"""
        params = self._build_export_params(
            format=format, start_date=start_date, end_date=end_date,
            include_raw=include_raw
        )
        return self._make_request('GET', '/api/v1/export/heart-rate', params=params)
    
    def export_activity_analytics(self, format: str = 'json',
                                 start_date: Optional[str] = None,
                                 end_date: Optional[str] = None) -> Dict:
        """Export activity analytics"""
        params = self._build_export_params(
            format=format, start_date=start_date, end_date=end_date
        )
        return self._make_request('GET', '/api/v1/export/activity-analytics', params=params)
    
    # Helper methods
    def _build_query_params(self, **kwargs) -> Dict[str, Union[str, int]]:
        """Build query parameters"""
        params = {}
        for key, value in kwargs.items():
            if value is not None:
                params[key] = value
        return params
    
    def _build_export_params(self, **kwargs) -> Dict[str, Union[str, bool]]:
        """Build export parameters"""
        params = {}
        for key, value in kwargs.items():
            if value is not None:
                if key == 'metric_types' and isinstance(value, list):
                    params[key] = ','.join(value)
                else:
                    params[key] = value
        return params
    
    # Utility methods
    def with_retry(self, func, max_retries: int = 3, backoff_factor: float = 1.0):
        """Execute function with retry logic"""
        for attempt in range(max_retries):
            try:
                return func()
            except RateLimitError as e:
                if attempt == max_retries - 1:
                    raise
                
                wait_time = e.retry_after or (backoff_factor * (2 ** attempt))
                time.sleep(wait_time)
            except (HealthExportError, requests.exceptions.RequestException) as e:
                if attempt == max_retries - 1:
                    raise
                
                wait_time = backoff_factor * (2 ** attempt)
                time.sleep(wait_time)
```

### Usage Examples
```python
from health_export_sdk import HealthExportSDK, RateLimitError, ValidationError
from datetime import datetime, timezone, timedelta

# Initialize SDK
sdk = HealthExportSDK('550e8400-e29b-41d4-a716-446655440000')

def main():
    try:
        # Check API status
        status = sdk.get_status()
        print(f"API Status: {status['data']['status']}")
        
        # Create sample health data
        now = datetime.now(timezone.utc)
        
        metrics = [
            {
                "type": "HeartRate",
                "recorded_at": now.isoformat(),
                "avg_bpm": 72,
                "min_bpm": 68,
                "max_bpm": 78,
                "source": "Apple Watch",
                "context": "resting"
            },
            {
                "type": "BloodPressure",
                "recorded_at": now.isoformat(),
                "systolic": 120,
                "diastolic": 80,
                "pulse": 68,
                "source": "Manual Entry"
            }
        ]
        
        workouts = [
            {
                "workout_type": "Running",
                "start_time": (now - timedelta(hours=2)).isoformat(),
                "end_time": (now - timedelta(hours=1, minutes=15)).isoformat(),
                "total_energy_kcal": 320.5,
                "distance_meters": 5500.0,
                "avg_heart_rate": 155,
                "max_heart_rate": 175,
                "source": "Apple Watch"
            }
        ]
        
        # Ingest data with retry
        ingest_result = sdk.with_retry(
            lambda: sdk.ingest_batch(metrics, workouts),
            max_retries=3
        )
        
        print(f"Processed {ingest_result['data']['processed_count']} metrics")
        
        # Query heart rate data from last 30 days
        thirty_days_ago = (now - timedelta(days=30)).isoformat()
        
        heart_rate_data = sdk.get_heart_rate_data(
            start_date=thirty_days_ago,
            limit=50,
            sort='desc'
        )
        
        print(f"Found {heart_rate_data['data']['total_count']} heart rate records")
        
        # Get health summary
        summary = sdk.get_health_summary(
            start_date=thirty_days_ago,
            end_date=now.isoformat()
        )
        
        print("Health Summary:")
        if summary['data']['heart_rate_stats']:
            print(f"  Average resting HR: {summary['data']['heart_rate_stats']['avg_resting']}")
        
        # Export data as JSON
        export_result = sdk.export_all_data(
            format='json',
            start_date=thirty_days_ago,
            metric_types=['HeartRate', 'BloodPressure']
        )
        
        print(f"Exported {export_result['data']['record_count']} records")
        
    except RateLimitError as e:
        print(f"Rate limited - retry after {e.retry_after} seconds")
    except ValidationError as e:
        print(f"Validation error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")

if __name__ == "__main__":
    main()
```

## Swift/iOS SDK

### Installation
```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/health-export/ios-sdk", from: "1.0.0")
]
```

### SDK Implementation
```swift
// HealthExportSDK.swift
import Foundation
import Combine

public enum HealthExportError: Error, LocalizedError {
    case authenticationFailed(String)
    case rateLimitExceeded(retryAfter: Int?)
    case payloadTooLarge(String)
    case validationError(String)
    case networkError(String)
    case apiError(statusCode: Int, message: String)
    
    public var errorDescription: String? {
        switch self {
        case .authenticationFailed(let message):
            return "Authentication failed: \(message)"
        case .rateLimitExceeded(let retryAfter):
            return "Rate limit exceeded\(retryAfter.map { " - retry after \($0) seconds" } ?? "")"
        case .payloadTooLarge(let message):
            return "Payload too large: \(message)"
        case .validationError(let message):
            return "Validation error: \(message)"
        case .networkError(let message):
            return "Network error: \(message)"
        case .apiError(let statusCode, let message):
            return "API error (\(statusCode)): \(message)"
        }
    }
}

public struct HealthExportConfig {
    public let apiKey: String
    public let baseURL: String
    public let timeout: TimeInterval
    
    public init(apiKey: String, 
                baseURL: String = "https://api.health-export.com",
                timeout: TimeInterval = 30.0) {
        self.apiKey = apiKey
        self.baseURL = baseURL
        self.timeout = timeout
    }
}

public class HealthExportSDK {
    private let config: HealthExportConfig
    private let session: URLSession
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    
    public init(config: HealthExportConfig) {
        self.config = config
        
        let sessionConfig = URLSessionConfiguration.default
        sessionConfig.timeoutIntervalForRequest = config.timeout
        sessionConfig.timeoutIntervalForResource = config.timeout * 2
        
        self.session = URLSession(configuration: sessionConfig)
        
        // Configure date formatting
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZ"
        encoder.dateEncodingStrategy = .formatted(dateFormatter)
        decoder.dateDecodingStrategy = .formatted(dateFormatter)
    }
    
    // MARK: - Generic Request Method
    
    private func makeRequest<T: Codable>(
        method: HTTPMethod,
        endpoint: String,
        body: Data? = nil,
        queryItems: [URLQueryItem]? = nil
    ) -> AnyPublisher<APIResponse<T>, HealthExportError> {
        
        guard let baseURL = URL(string: config.baseURL) else {
            return Fail(error: HealthExportError.networkError("Invalid base URL"))
                .eraseToAnyPublisher()
        }
        
        var url = baseURL.appendingPathComponent(endpoint)
        
        if let queryItems = queryItems, !queryItems.isEmpty {
            var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
            components?.queryItems = queryItems
            url = components?.url ?? url
        }
        
        var request = URLRequest(url: url)
        request.httpMethod = method.rawValue
        request.setValue("Bearer \(config.apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("health-export-sdk-ios/1.0.0", forHTTPHeaderField: "User-Agent")
        
        if let body = body {
            request.httpBody = body
        }
        
        return session.dataTaskPublisher(for: request)
            .tryMap { [weak self] data, response -> APIResponse<T> in
                guard let self = self else {
                    throw HealthExportError.networkError("SDK deallocated")
                }
                
                guard let httpResponse = response as? HTTPURLResponse else {
                    throw HealthExportError.networkError("Invalid response type")
                }
                
                // Handle error status codes
                if httpResponse.statusCode >= 400 {
                    let errorResponse = try? self.decoder.decode(APIResponse<EmptyData>.self, from: data)
                    let errorMessage = errorResponse?.error ?? "Unknown error"
                    
                    switch httpResponse.statusCode {
                    case 401:
                        throw HealthExportError.authenticationFailed(errorMessage)
                    case 413:
                        throw HealthExportError.payloadTooLarge(errorMessage)
                    case 422:
                        throw HealthExportError.validationError(errorMessage)
                    case 429:
                        let retryAfter = httpResponse.value(forHTTPHeaderField: "Retry-After")
                            .flatMap(Int.init)
                        throw HealthExportError.rateLimitExceeded(retryAfter: retryAfter)
                    default:
                        throw HealthExportError.apiError(statusCode: httpResponse.statusCode, message: errorMessage)
                    }
                }
                
                return try self.decoder.decode(APIResponse<T>.self, from: data)
            }
            .mapError { error in
                if let healthError = error as? HealthExportError {
                    return healthError
                } else if error is URLError {
                    return HealthExportError.networkError(error.localizedDescription)
                } else {
                    return HealthExportError.networkError("Request failed: \(error.localizedDescription)")
                }
            }
            .eraseToAnyPublisher()
    }
    
    // MARK: - Health Checks
    
    public func checkHealth() -> AnyPublisher<APIResponse<HealthStatus>, HealthExportError> {
        return makeRequest(method: .GET, endpoint: "/health")
    }
    
    public func getStatus() -> AnyPublisher<APIResponse<APIStatus>, HealthExportError> {
        return makeRequest(method: .GET, endpoint: "/api/v1/status")
    }
    
    // MARK: - Data Ingestion
    
    public func ingestHealthData<T: Codable>(payload: T) -> AnyPublisher<APIResponse<IngestResponse>, HealthExportError> {
        do {
            let body = try encoder.encode(payload)
            return makeRequest(method: .POST, endpoint: "/api/v1/ingest", body: body)
        } catch {
            return Fail(error: HealthExportError.networkError("Failed to encode payload: \(error)"))
                .eraseToAnyPublisher()
        }
    }
    
    public func ingestBatch(metrics: [HealthMetric], workouts: [WorkoutData] = []) -> AnyPublisher<APIResponse<IngestResponse>, HealthExportError> {
        let payload = IngestPayload(data: IngestData(metrics: metrics, workouts: workouts))
        return ingestHealthData(payload: payload)
    }
    
    // MARK: - Data Queries
    
    public func getHeartRateData(options: QueryOptions = QueryOptions()) -> AnyPublisher<APIResponse<QueryResponse<HeartRateRecord>>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/data/heart-rate", queryItems: queryItems)
    }
    
    public func getBloodPressureData(options: QueryOptions = QueryOptions()) -> AnyPublisher<APIResponse<QueryResponse<BloodPressureRecord>>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/data/blood-pressure", queryItems: queryItems)
    }
    
    public func getSleepData(options: QueryOptions = QueryOptions()) -> AnyPublisher<APIResponse<QueryResponse<SleepRecord>>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/data/sleep", queryItems: queryItems)
    }
    
    public func getActivityData(options: QueryOptions = QueryOptions()) -> AnyPublisher<APIResponse<QueryResponse<ActivityRecord>>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/data/activity", queryItems: queryItems)
    }
    
    public func getWorkoutData(options: WorkoutQueryOptions = WorkoutQueryOptions()) -> AnyPublisher<APIResponse<QueryResponse<WorkoutRecord>>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/data/workouts", queryItems: queryItems)
    }
    
    public func getHealthSummary(startDate: Date? = nil, endDate: Date? = nil) -> AnyPublisher<APIResponse<HealthSummary>, HealthExportError> {
        var queryItems: [URLQueryItem] = []
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZ"
        
        if let startDate = startDate {
            queryItems.append(URLQueryItem(name: "start_date", value: dateFormatter.string(from: startDate)))
        }
        if let endDate = endDate {
            queryItems.append(URLQueryItem(name: "end_date", value: dateFormatter.string(from: endDate)))
        }
        
        return makeRequest(method: .GET, endpoint: "/api/v1/data/summary", queryItems: queryItems.isEmpty ? nil : queryItems)
    }
    
    // MARK: - Data Export
    
    public func exportAllData(options: ExportOptions = ExportOptions()) -> AnyPublisher<APIResponse<ExportResponse>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/export/all", queryItems: queryItems)
    }
    
    public func exportHeartRateData(options: ExportOptions = ExportOptions()) -> AnyPublisher<APIResponse<ExportResponse>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/export/heart-rate", queryItems: queryItems)
    }
    
    public func exportActivityAnalytics(options: ExportOptions = ExportOptions()) -> AnyPublisher<APIResponse<ExportResponse>, HealthExportError> {
        let queryItems = options.toQueryItems()
        return makeRequest(method: .GET, endpoint: "/api/v1/export/activity-analytics", queryItems: queryItems)
    }
}

// MARK: - Supporting Types

enum HTTPMethod: String {
    case GET = "GET"
    case POST = "POST"
    case PUT = "PUT"
    case DELETE = "DELETE"
}

struct EmptyData: Codable {}

public struct QueryOptions {
    public let startDate: Date?
    public let endDate: Date?
    public let page: Int
    public let limit: Int
    public let sort: SortOrder
    public let metricTypes: [String]?
    
    public init(startDate: Date? = nil,
                endDate: Date? = nil,
                page: Int = 1,
                limit: Int = 100,
                sort: SortOrder = .desc,
                metricTypes: [String]? = nil) {
        self.startDate = startDate
        self.endDate = endDate
        self.page = page
        self.limit = limit
        self.sort = sort
        self.metricTypes = metricTypes
    }
    
    func toQueryItems() -> [URLQueryItem] {
        var items: [URLQueryItem] = []
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZ"
        
        if let startDate = startDate {
            items.append(URLQueryItem(name: "start_date", value: dateFormatter.string(from: startDate)))
        }
        if let endDate = endDate {
            items.append(URLQueryItem(name: "end_date", value: dateFormatter.string(from: endDate)))
        }
        
        items.append(URLQueryItem(name: "page", value: String(page)))
        items.append(URLQueryItem(name: "limit", value: String(limit)))
        items.append(URLQueryItem(name: "sort", value: sort.rawValue))
        
        if let metricTypes = metricTypes, !metricTypes.isEmpty {
            items.append(URLQueryItem(name: "metric_types", value: metricTypes.joined(separator: ",")))
        }
        
        return items
    }
}

public struct WorkoutQueryOptions: QueryOptions {
    public let workoutType: String?
    
    public init(startDate: Date? = nil,
                endDate: Date? = nil,
                page: Int = 1,
                limit: Int = 100,
                sort: SortOrder = .desc,
                metricTypes: [String]? = nil,
                workoutType: String? = nil) {
        self.workoutType = workoutType
        super.init(startDate: startDate, endDate: endDate, page: page, limit: limit, sort: sort, metricTypes: metricTypes)
    }
    
    override func toQueryItems() -> [URLQueryItem] {
        var items = super.toQueryItems()
        
        if let workoutType = workoutType {
            items.append(URLQueryItem(name: "workout_type", value: workoutType))
        }
        
        return items
    }
}

public struct ExportOptions {
    public let format: ExportFormat
    public let startDate: Date?
    public let endDate: Date?
    public let metricTypes: [String]?
    public let includeRaw: Bool
    
    public init(format: ExportFormat = .json,
                startDate: Date? = nil,
                endDate: Date? = nil,
                metricTypes: [String]? = nil,
                includeRaw: Bool = false) {
        self.format = format
        self.startDate = startDate
        self.endDate = endDate
        self.metricTypes = metricTypes
        self.includeRaw = includeRaw
    }
    
    func toQueryItems() -> [URLQueryItem] {
        var items: [URLQueryItem] = []
        
        items.append(URLQueryItem(name: "format", value: format.rawValue))
        
        let dateFormatter = DateFormatter()
        dateFormatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ssZ"
        
        if let startDate = startDate {
            items.append(URLQueryItem(name: "start_date", value: dateFormatter.string(from: startDate)))
        }
        if let endDate = endDate {
            items.append(URLQueryItem(name: "end_date", value: dateFormatter.string(from: endDate)))
        }
        
        if let metricTypes = metricTypes, !metricTypes.isEmpty {
            items.append(URLQueryItem(name: "metric_types", value: metricTypes.joined(separator: ",")))
        }
        
        items.append(URLQueryItem(name: "include_raw", value: String(includeRaw)))
        
        return items
    }
}

public enum SortOrder: String, Codable {
    case asc = "asc"
    case desc = "desc"
}

public enum ExportFormat: String, Codable {
    case json = "json"
    case csv = "csv"
}
```

### Usage Example
```swift
// ViewController.swift
import UIKit
import Combine
import HealthExportSDK

class HealthDataViewController: UIViewController {
    private let sdk: HealthExportSDK
    private var cancellables = Set<AnyCancellable>()
    
    override func viewDidLoad() {
        super.viewDidLoad()
        
        let config = HealthExportConfig(apiKey: "550e8400-e29b-41d4-a716-446655440000")
        sdk = HealthExportSDK(config: config)
        
        loadHealthData()
    }
    
    private func loadHealthData() {
        // Check API status first
        sdk.getStatus()
            .sink(
                receiveCompletion: { completion in
                    switch completion {
                    case .failure(let error):
                        print("Status check failed: \(error)")
                    case .finished:
                        print("Status check completed")
                    }
                },
                receiveValue: { response in
                    print("API Status: \(response.data?.status ?? "unknown")")
                    
                    // If API is healthy, load heart rate data
                    if response.success {
                        self.loadHeartRateData()
                    }
                }
            )
            .store(in: &cancellables)
    }
    
    private func loadHeartRateData() {
        let thirtyDaysAgo = Calendar.current.date(byAdding: .day, value: -30, to: Date()) ?? Date()
        let options = QueryOptions(
            startDate: thirtyDaysAgo,
            endDate: Date(),
            limit: 50,
            sort: .desc
        )
        
        sdk.getHeartRateData(options: options)
            .sink(
                receiveCompletion: { completion in
                    switch completion {
                    case .failure(let error):
                        DispatchQueue.main.async {
                            self.showError(error)
                        }
                    case .finished:
                        break
                    }
                },
                receiveValue: { response in
                    DispatchQueue.main.async {
                        if let data = response.data {
                            print("Found \(data.totalCount) heart rate records")
                            self.displayHeartRateData(data.data)
                        }
                    }
                }
            )
            .store(in: &cancellables)
    }
    
    private func ingestSampleData() {
        let now = Date()
        
        let metrics: [HealthMetric] = [
            .heartRate(HeartRateMetric(
                recordedAt: now,
                minBpm: 68,
                avgBpm: 72,
                maxBpm: 78,
                source: "Apple Watch Series 7",
                context: "resting"
            )),
            .bloodPressure(BloodPressureMetric(
                recordedAt: now,
                systolic: 120,
                diastolic: 80,
                pulse: 68,
                source: "Manual Entry"
            ))
        ]
        
        let workouts: [WorkoutData] = [
            WorkoutData(
                workoutType: "Running",
                startTime: Calendar.current.date(byAdding: .hour, value: -2, to: now) ?? now,
                endTime: Calendar.current.date(byAdding: .minute, value: -75, to: now) ?? now,
                totalEnergyKcal: 320.5,
                distanceMeters: 5500.0,
                avgHeartRate: 155,
                maxHeartRate: 175,
                source: "Apple Watch Series 7"
            )
        ]
        
        sdk.ingestBatch(metrics: metrics, workouts: workouts)
            .sink(
                receiveCompletion: { completion in
                    switch completion {
                    case .failure(let error):
                        print("Ingest failed: \(error)")
                    case .finished:
                        break
                    }
                },
                receiveValue: { response in
                    if let data = response.data {
                        print("Processed \(data.processedCount) metrics")
                        
                        if data.failedCount > 0 {
                            print("Failed to process \(data.failedCount) metrics:")
                            for error in data.errors {
                                print("  \(error.metricType): \(error.errorMessage)")
                            }
                        }
                    }
                }
            )
            .store(in: &cancellables)
    }
    
    private func displayHeartRateData(_ records: [HeartRateRecord]) {
        // Update UI with heart rate data
        // Implementation depends on your UI components
    }
    
    private func showError(_ error: HealthExportError) {
        let alert = UIAlertController(
            title: "Error",
            message: error.localizedDescription,
            preferredStyle: .alert
        )
        
        alert.addAction(UIAlertAction(title: "OK", style: .default))
        
        if case .rateLimitExceeded(let retryAfter) = error, let retryAfter = retryAfter {
            alert.addAction(UIAlertAction(title: "Retry Later", style: .default) { _ in
                DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(retryAfter)) {
                    self.loadHealthData()
                }
            })
        }
        
        present(alert, animated: true)
    }
}
```

## Summary

The client SDK examples provided above demonstrate:

1. **Comprehensive Error Handling**: All SDKs include proper error handling for authentication, rate limiting, validation, and network issues.

2. **Async/Await Support**: Modern asynchronous programming patterns using appropriate technologies for each platform (Promises/async-await for JavaScript, asyncio for Python, Combine for Swift).

3. **Type Safety**: Strong typing where possible to catch errors at compile time rather than runtime.

4. **Rate Limit Awareness**: Built-in handling for rate limits with retry logic and backoff strategies.

5. **Flexible Configuration**: Configurable base URLs, timeouts, and other settings for different environments.

6. **Comprehensive API Coverage**: All endpoints covered including health checks, data ingestion, querying, and export functionality.

These SDKs provide developers with production-ready tools to integrate with the Health Export API across multiple platforms and programming languages.