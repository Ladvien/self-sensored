use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),
    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub requests_remaining: i32,
    pub requests_limit: i32,
    pub reset_time: DateTime<Utc>,
    pub retry_after: Option<i32>, // seconds until reset
}

#[derive(Debug, Clone)]
struct InMemoryRateLimit {
    count: i32,
    window_start: Instant,
    reset_time: DateTime<Utc>,
}

/// Rate limiting service with Redis backend and in-memory fallback
#[derive(Debug)]
pub struct RateLimiter {
    redis_client: Option<RedisClient>,
    fallback_store: Arc<Mutex<HashMap<String, InMemoryRateLimit>>>,
    requests_per_hour: i32,
    window_duration: Duration,
    using_redis: bool,
}

impl RateLimiter {
    /// Create a new rate limiter with Redis backend
    pub async fn new(redis_url: &str) -> Result<Self, RateLimitError> {
        let requests_per_hour = std::env::var("RATE_LIMIT_REQUESTS_PER_HOUR")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<i32>()
            .unwrap_or(100);

        // Try to connect to Redis
        match RedisClient::open(redis_url) {
            Ok(client) => {
                // Test the connection
                if let Ok(mut conn) = client.get_async_connection().await {
                    // Test ping
                    match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                        Ok(_) => {
                            log::info!("Connected to Redis for rate limiting");
                            return Ok(Self {
                                redis_client: Some(client),
                                fallback_store: Arc::new(Mutex::new(HashMap::new())),
                                requests_per_hour,
                                window_duration: Duration::from_secs(3600), // 1 hour
                                using_redis: true,
                            });
                        }
                        Err(e) => {
                            log::warn!("Redis ping failed, falling back to in-memory store: {e}");
                        }
                    }
                } else {
                    log::warn!("Failed to connect to Redis, falling back to in-memory store");
                }
            }
            Err(e) => {
                log::warn!("Failed to create Redis client, falling back to in-memory store: {e}");
            }
        }

        // Fallback to in-memory store
        log::info!("Using in-memory rate limiting store");
        Ok(Self {
            redis_client: None,
            fallback_store: Arc::new(Mutex::new(HashMap::new())),
            requests_per_hour,
            window_duration: Duration::from_secs(3600), // 1 hour
            using_redis: false,
        })
    }

    /// Create a new rate limiter with in-memory store only (for testing)
    pub fn new_in_memory(requests_per_hour: i32) -> Self {
        Self {
            redis_client: None,
            fallback_store: Arc::new(Mutex::new(HashMap::new())),
            requests_per_hour,
            window_duration: Duration::from_secs(3600), // 1 hour
            using_redis: false,
        }
    }

    /// Check if a request is allowed and update rate limit counters
    pub async fn check_rate_limit(
        &self,
        api_key_id: Uuid,
    ) -> Result<RateLimitInfo, RateLimitError> {
        let key = format!("rate_limit:{api_key_id}");

        if self.using_redis && self.redis_client.is_some() {
            self.check_redis_rate_limit(&key).await
        } else {
            self.check_memory_rate_limit(&key).await
        }
    }

    /// Redis-based rate limiting
    async fn check_redis_rate_limit(&self, key: &str) -> Result<RateLimitInfo, RateLimitError> {
        if let Some(client) = &self.redis_client {
            let mut conn = client.get_async_connection().await?;

            // Use Redis sliding window with expiration
            let now = Utc::now().timestamp();
            let window_start = now - 3600; // 1 hour window

            // Remove expired entries and count current requests
            let _: () = redis::cmd("ZREMRANGEBYSCORE")
                .arg(key)
                .arg(0)
                .arg(window_start)
                .query_async(&mut conn)
                .await?;
            let current_count: i32 = conn.zcard(key).await?;

            if current_count >= self.requests_per_hour {
                // Get the oldest entry to determine reset time
                let oldest_entries: Vec<(String, f64)> = conn.zrange_withscores(key, 0, 0).await?;
                let reset_time = if let Some((_, timestamp)) = oldest_entries.first() {
                    DateTime::from_timestamp(*timestamp as i64 + 3600, 0)
                        .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1))
                } else {
                    Utc::now() + chrono::Duration::hours(1)
                };

                return Ok(RateLimitInfo {
                    requests_remaining: 0,
                    requests_limit: self.requests_per_hour,
                    reset_time,
                    retry_after: Some((reset_time - Utc::now()).num_seconds() as i32),
                });
            }

            // Add current request
            let request_id = Uuid::new_v4().to_string();
            let _: () = conn.zadd(key, request_id, now).await?;
            let _: () = conn.expire(key, 3600).await?; // Set expiration

            let reset_time = Utc::now() + chrono::Duration::hours(1);

            Ok(RateLimitInfo {
                requests_remaining: self.requests_per_hour - current_count - 1,
                requests_limit: self.requests_per_hour,
                reset_time,
                retry_after: None,
            })
        } else {
            Err(RateLimitError::InternalError(
                "Redis client not available".to_string(),
            ))
        }
    }

    /// In-memory rate limiting (fallback)
    async fn check_memory_rate_limit(&self, key: &str) -> Result<RateLimitInfo, RateLimitError> {
        let now = Instant::now();
        let mut store = self
            .fallback_store
            .lock()
            .map_err(|e| RateLimitError::InternalError(format!("Lock poisoned: {e}")))?;

        // Clean up expired entries periodically
        let keys_to_remove: Vec<String> = store
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.window_start) > self.window_duration)
            .map(|(k, _)| k.clone())
            .collect();

        for key_to_remove in keys_to_remove {
            store.remove(&key_to_remove);
        }

        // Get or create entry for this key
        let entry = store
            .entry(key.to_string())
            .or_insert_with(|| InMemoryRateLimit {
                count: 0,
                window_start: now,
                reset_time: Utc::now() + chrono::Duration::hours(1),
            });

        // Check if we need to reset the window
        if now.duration_since(entry.window_start) > self.window_duration {
            entry.count = 0;
            entry.window_start = now;
            entry.reset_time = Utc::now() + chrono::Duration::hours(1);
        }

        // Check if limit exceeded
        if entry.count >= self.requests_per_hour {
            return Ok(RateLimitInfo {
                requests_remaining: 0,
                requests_limit: self.requests_per_hour,
                reset_time: entry.reset_time,
                retry_after: Some((entry.reset_time - Utc::now()).num_seconds() as i32),
            });
        }

        // Increment counter
        entry.count += 1;

        Ok(RateLimitInfo {
            requests_remaining: self.requests_per_hour - entry.count,
            requests_limit: self.requests_per_hour,
            reset_time: entry.reset_time,
            retry_after: None,
        })
    }

    /// Get current rate limit status without incrementing
    pub async fn get_rate_limit_status(
        &self,
        api_key_id: Uuid,
    ) -> Result<RateLimitInfo, RateLimitError> {
        let key = format!("rate_limit:{api_key_id}");

        if self.using_redis && self.redis_client.is_some() {
            self.get_redis_rate_limit_status(&key).await
        } else {
            self.get_memory_rate_limit_status(&key).await
        }
    }

    /// Get Redis rate limit status
    async fn get_redis_rate_limit_status(
        &self,
        key: &str,
    ) -> Result<RateLimitInfo, RateLimitError> {
        if let Some(client) = &self.redis_client {
            let mut conn = client.get_async_connection().await?;

            let now = Utc::now().timestamp();
            let window_start = now - 3600;

            // Clean and count
            let _: () = redis::cmd("ZREMRANGEBYSCORE")
                .arg(key)
                .arg(0)
                .arg(window_start)
                .query_async(&mut conn)
                .await?;
            let current_count: i32 = conn.zcard(key).await?;

            let reset_time = Utc::now() + chrono::Duration::hours(1);

            Ok(RateLimitInfo {
                requests_remaining: (self.requests_per_hour - current_count).max(0),
                requests_limit: self.requests_per_hour,
                reset_time,
                retry_after: if current_count >= self.requests_per_hour {
                    Some((reset_time - Utc::now()).num_seconds() as i32)
                } else {
                    None
                },
            })
        } else {
            Err(RateLimitError::InternalError(
                "Redis client not available".to_string(),
            ))
        }
    }

    /// Get in-memory rate limit status
    async fn get_memory_rate_limit_status(
        &self,
        key: &str,
    ) -> Result<RateLimitInfo, RateLimitError> {
        let now = Instant::now();
        let store = self
            .fallback_store
            .lock()
            .map_err(|e| RateLimitError::InternalError(format!("Lock poisoned: {e}")))?;

        if let Some(entry) = store.get(key) {
            // Check if window expired
            if now.duration_since(entry.window_start) > self.window_duration {
                Ok(RateLimitInfo {
                    requests_remaining: self.requests_per_hour,
                    requests_limit: self.requests_per_hour,
                    reset_time: Utc::now() + chrono::Duration::hours(1),
                    retry_after: None,
                })
            } else {
                Ok(RateLimitInfo {
                    requests_remaining: (self.requests_per_hour - entry.count).max(0),
                    requests_limit: self.requests_per_hour,
                    reset_time: entry.reset_time,
                    retry_after: if entry.count >= self.requests_per_hour {
                        Some((entry.reset_time - Utc::now()).num_seconds() as i32)
                    } else {
                        None
                    },
                })
            }
        } else {
            Ok(RateLimitInfo {
                requests_remaining: self.requests_per_hour,
                requests_limit: self.requests_per_hour,
                reset_time: Utc::now() + chrono::Duration::hours(1),
                retry_after: None,
            })
        }
    }

    /// Clear all rate limits (for testing)
    pub async fn clear_all(&self) -> Result<(), RateLimitError> {
        if self.using_redis && self.redis_client.is_some() {
            if let Some(client) = &self.redis_client {
                let mut conn = client.get_async_connection().await?;
                let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await?;
            }
        } else {
            let mut store = self
                .fallback_store
                .lock()
                .map_err(|e| RateLimitError::InternalError(format!("Lock poisoned: {e}")))?;
            store.clear();
        }
        Ok(())
    }

    /// Check if using Redis backend
    pub fn is_using_redis(&self) -> bool {
        self.using_redis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_in_memory_basic() {
        let rate_limiter = RateLimiter::new_in_memory(5); // 5 requests per hour
        let api_key_id = Uuid::new_v4();

        // First 5 requests should succeed
        for i in 0..5 {
            let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
            assert_eq!(result.requests_remaining, 4 - i);
            assert_eq!(result.requests_limit, 5);
        }

        // 6th request should be rate limited
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        assert_eq!(result.requests_remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiter_status_check() {
        let rate_limiter = RateLimiter::new_in_memory(3);
        let api_key_id = Uuid::new_v4();

        // Make 2 requests
        rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        rate_limiter.check_rate_limit(api_key_id).await.unwrap();

        // Check status without incrementing
        let status = rate_limiter
            .get_rate_limit_status(api_key_id)
            .await
            .unwrap();
        assert_eq!(status.requests_remaining, 1);

        // Status should not have changed
        let status2 = rate_limiter
            .get_rate_limit_status(api_key_id)
            .await
            .unwrap();
        assert_eq!(status2.requests_remaining, 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_keys() {
        let rate_limiter = RateLimiter::new_in_memory(2);
        let key1 = Uuid::new_v4();
        let key2 = Uuid::new_v4();

        // Use up limit for key1
        rate_limiter.check_rate_limit(key1).await.unwrap();
        rate_limiter.check_rate_limit(key1).await.unwrap();

        // key1 should be limited
        let result1 = rate_limiter.check_rate_limit(key1).await.unwrap();
        assert_eq!(result1.requests_remaining, 0);

        // key2 should still work
        let result2 = rate_limiter.check_rate_limit(key2).await.unwrap();
        assert_eq!(result2.requests_remaining, 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_clear_all() {
        let rate_limiter = RateLimiter::new_in_memory(1);
        let api_key_id = Uuid::new_v4();

        // Use up the limit
        rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        assert_eq!(result.requests_remaining, 0);

        // Clear all limits
        rate_limiter.clear_all().await.unwrap();

        // Should be able to make requests again
        let result = rate_limiter.check_rate_limit(api_key_id).await.unwrap();
        assert_eq!(result.requests_remaining, 0); // 1 - 1 = 0 remaining after this request
    }
}
