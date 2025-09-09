/// Log aggregation queries and examples for the structured logging system
///
/// These queries are designed for log aggregation systems like:
/// - Datadog
/// - CloudWatch Insights
/// - Elasticsearch/OpenSearch
/// - Splunk
/// - Grafana Loki
use serde_json::json;

/// Common log query patterns for monitoring and debugging
pub struct LogQueries;

impl LogQueries {
    /// CloudWatch Insights queries for different monitoring scenarios
    pub fn cloudwatch_queries() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Error Rate by Endpoint",
                r#"
                fields @timestamp, path, status, error
                | filter event = "request_completed" or event = "request_failed"
                | stats count() as total, sum(status >= 400) as errors by path
                | eval error_rate = errors / total * 100
                | sort error_rate desc
                "#,
            ),
            (
                "Request Duration P95 by Endpoint",
                r#"
                fields @timestamp, path, duration_ms
                | filter event = "request_completed"
                | stats percentile(duration_ms, 95) as p95_duration by path
                | sort p95_duration desc
                "#,
            ),
            (
                "Failed Requests with Context",
                r#"
                fields @timestamp, request_id, path, status, error, message
                | filter event = "request_failed" or status >= 400
                | sort @timestamp desc
                | limit 100
                "#,
            ),
            (
                "Authentication Failures",
                r#"
                fields @timestamp, request_id, client_ip, user_agent
                | filter event = "authentication_failed"
                | stats count() as failures by client_ip
                | sort failures desc
                "#,
            ),
            (
                "Performance Issues (>1s)",
                r#"
                fields @timestamp, request_id, path, duration_ms, method
                | filter event = "request_completed" and duration_ms > 1000
                | sort duration_ms desc
                | limit 50
                "#,
            ),
            (
                "API Key Usage by User",
                r#"
                fields @timestamp, user_id, api_key_id, path
                | filter ispresent(user_id) and ispresent(api_key_id)
                | stats count() as requests by user_id, api_key_id
                | sort requests desc
                "#,
            ),
        ]
    }

    /// Datadog log queries for monitoring
    pub fn datadog_queries() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Error Rate by Service",
                r#"
                service:health-export-api event:(request_failed OR request_completed)
                | group by path
                | eval error_rate = count(event:request_failed) / count() * 100
                "#,
            ),
            (
                "High Duration Requests",
                r#"
                service:health-export-api event:request_completed duration_ms:>500
                | sort duration_ms desc
                "#,
            ),
            (
                "Failed Authentication by IP",
                r#"
                service:health-export-api event:authentication_failed
                | group by client_ip
                | sort count desc
                "#,
            ),
            (
                "Database Errors",
                r#"
                service:health-export-api event:error_occurred context:*database*
                | sort @timestamp desc
                "#,
            ),
        ]
    }

    /// Elasticsearch/OpenSearch queries
    pub fn elasticsearch_queries() -> Vec<(&'static str, serde_json::Value)> {
        vec![
            (
                "Error Rate Aggregation",
                json!({
                    "query": {
                        "bool": {
                            "filter": [
                                {"term": {"service_name": "health-export-api"}},
                                {"terms": {"event": ["request_completed", "request_failed"]}},
                                {"range": {"@timestamp": {"gte": "now-1h"}}}
                            ]
                        }
                    },
                    "aggs": {
                        "by_endpoint": {
                            "terms": {"field": "path"},
                            "aggs": {
                                "total_requests": {"value_count": {"field": "event"}},
                                "error_requests": {
                                    "filter": {"term": {"event": "request_failed"}},
                                    "aggs": {
                                        "count": {"value_count": {"field": "event"}}
                                    }
                                },
                                "error_rate": {
                                    "bucket_script": {
                                        "buckets_path": {
                                            "errors": "error_requests>count",
                                            "total": "total_requests"
                                        },
                                        "script": "params.errors / params.total * 100"
                                    }
                                }
                            }
                        }
                    }
                }),
            ),
            (
                "Performance Percentiles",
                json!({
                    "query": {
                        "bool": {
                            "filter": [
                                {"term": {"event": "request_completed"}},
                                {"range": {"@timestamp": {"gte": "now-1h"}}}
                            ]
                        }
                    },
                    "aggs": {
                        "by_endpoint": {
                            "terms": {"field": "path"},
                            "aggs": {
                                "duration_stats": {
                                    "percentiles": {
                                        "field": "duration_ms",
                                        "percents": [50, 90, 95, 99]
                                    }
                                }
                            }
                        }
                    }
                }),
            ),
            (
                "Recent Errors with Full Context",
                json!({
                    "query": {
                        "bool": {
                            "filter": [
                                {"term": {"event": "error_occurred"}},
                                {"range": {"@timestamp": {"gte": "now-1h"}}}
                            ]
                        }
                    },
                    "sort": [{"@timestamp": {"order": "desc"}}],
                    "size": 100,
                    "_source": [
                        "@timestamp", "request_id", "context", "error",
                        "error_chain", "user_id", "path"
                    ]
                }),
            ),
        ]
    }

    /// Grafana Loki LogQL queries
    pub fn loki_queries() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Error Rate by Endpoint",
                r#"
                {service_name="health-export-api"} 
                | json 
                | __error__ = "" 
                | event =~ "request_(completed|failed)"
                | label_format endpoint="{{.path}}"
                | rate(5m) by (endpoint)
                "#,
            ),
            (
                "P95 Response Time",
                r#"
                {service_name="health-export-api"} 
                | json 
                | event = "request_completed"
                | quantile_over_time(0.95, duration_ms[5m]) by (path)
                "#,
            ),
            (
                "Authentication Failures",
                r#"
                {service_name="health-export-api"} 
                | json 
                | event = "authentication_failed"
                | rate(1m) by (client_ip)
                "#,
            ),
            (
                "Database Connection Issues",
                r#"
                {service_name="health-export-api"} 
                | json 
                | context =~ ".*database.*"
                | event = "error_occurred"
                "#,
            ),
        ]
    }

    /// Splunk search queries
    pub fn splunk_queries() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Error Rate Trend",
                r#"
                index=app service_name="health-export-api" event IN ("request_completed", "request_failed")
                | bucket _time span=5m
                | eval is_error=if(event="request_failed", 1, 0)
                | stats count as total, sum(is_error) as errors by _time, path
                | eval error_rate = round(errors/total*100, 2)
                | timechart span=5m avg(error_rate) by path
                "#,
            ),
            (
                "Performance Analysis",
                r#"
                index=app service_name="health-export-api" event="request_completed"
                | stats perc50(duration_ms) as p50, perc95(duration_ms) as p95, 
                        perc99(duration_ms) as p99, avg(duration_ms) as avg by path
                | sort -p95
                "#,
            ),
            (
                "Security Events",
                r#"
                index=app service_name="health-export-api" 
                (event="authentication_failed" OR context="*auth*")
                | stats count by client_ip, user_agent, event
                | where count > 5
                | sort -count
                "#,
            ),
        ]
    }
}

/// Log analysis helpers for common patterns
pub struct LogAnalysis;

impl LogAnalysis {
    /// Generate alert conditions based on log patterns
    pub fn alert_conditions() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (
                "High Error Rate",
                "Error rate > 5% for any endpoint over 5 minutes",
                "sum(rate({service_name=\"health-export-api\"} |~ \"request_failed\")[5m:]) / sum(rate({service_name=\"health-export-api\"} |~ \"request_\")[5m:]) > 0.05"
            ),
            (
                "High Response Time", 
                "P95 response time > 1000ms for any endpoint",
                "quantile_over_time(0.95, {service_name=\"health-export-api\"} | json | duration_ms[5m:]) > 1000"
            ),
            (
                "Authentication Failures",
                "More than 10 auth failures per minute from single IP",
                "sum(rate({service_name=\"health-export-api\"} | json | event=\"authentication_failed\"[1m:])) by (client_ip) > 10"
            ),
            (
                "Database Errors",
                "Any database connection or query errors", 
                "count_over_time({service_name=\"health-export-api\"} | json | context =~ \".*database.*\" | event=\"error_occurred\"[1m:]) > 0"
            ),
            (
                "Memory Usage",
                "High memory usage during request processing",
                "avg_over_time({service_name=\"health-export-api\"} | json | event=\"performance_measurement\" | memory_mb[5m:]) > 400"
            )
        ]
    }

    /// Common debugging queries for development
    pub fn debugging_queries() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "Trace Request Flow",
                r#"
                {service_name="health-export-api"} | json | request_id="<REQUEST_ID>"
                "#,
            ),
            (
                "User Activity Timeline",
                r#"
                {service_name="health-export-api"} | json | user_id="<USER_ID>" 
                | line_format "{{.timestamp}} - {{.event}}: {{.message}}"
                "#,
            ),
            (
                "API Key Usage Pattern",
                r#"
                {service_name="health-export-api"} | json | api_key_id="<API_KEY_ID>"
                | stats count by path, status
                "#,
            ),
            (
                "Performance Bottlenecks",
                r#"
                {service_name="health-export-api"} | json 
                | event="performance_measurement" 
                | duration_ms > 100
                | sort by timestamp desc
                "#,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_generation() {
        let cloudwatch = LogQueries::cloudwatch_queries();
        assert!(!cloudwatch.is_empty());

        let datadog = LogQueries::datadog_queries();
        assert!(!datadog.is_empty());

        let elasticsearch = LogQueries::elasticsearch_queries();
        assert!(!elasticsearch.is_empty());

        let loki = LogQueries::loki_queries();
        assert!(!loki.is_empty());

        let splunk = LogQueries::splunk_queries();
        assert!(!splunk.is_empty());
    }

    #[test]
    fn test_alert_conditions() {
        let alerts = LogAnalysis::alert_conditions();
        assert!(!alerts.is_empty());
        assert_eq!(alerts.len(), 5);
    }

    #[test]
    fn test_debugging_queries() {
        let debug = LogAnalysis::debugging_queries();
        assert!(!debug.is_empty());
        assert_eq!(debug.len(), 4);
    }
}
