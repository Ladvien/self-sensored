use redis::{aio::ConnectionManager, AsyncCommands, Client, RedisError};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Redis cache service for performance optimization
#[derive(Clone)]
pub struct CacheService {
    connection_manager: ConnectionManager,
    default_ttl: Duration,
    enabled: bool,
}

impl std::fmt::Debug for CacheService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheService")
            .field("default_ttl", &self.default_ttl)
            .field("enabled", &self.enabled)
            .field("connection_manager", &"<RedisConnectionManager>")
            .finish()
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub default_ttl_seconds: u64,
    pub summary_ttl_seconds: u64,
    pub user_data_ttl_seconds: u64,
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_seconds: 300,   // 5 minutes
            summary_ttl_seconds: 1800,  // 30 minutes for summaries
            user_data_ttl_seconds: 600, // 10 minutes for user data
            key_prefix: "health_export".to_string(),
        }
    }
}

/// Cache key types for different data categories
#[derive(Debug, Clone)]
pub enum CacheKey {
    HeartRateQuery {
        user_id: Uuid,
        hash: String,
    },
    BloodPressureQuery {
        user_id: Uuid,
        hash: String,
    },
    SleepQuery {
        user_id: Uuid,
        hash: String,
    },
    ActivityQuery {
        user_id: Uuid,
        hash: String,
    },
    WorkoutQuery {
        user_id: Uuid,
        hash: String,
    },
    MindfulnessQuery {
        user_id: Uuid,
        hash: String,
    },
    MentalHealthQuery {
        user_id: Uuid,
        hash: String,
    },
    MindfulnessInsights {
        user_id: Uuid,
        period: String,
    },
    MentalHealthInsights {
        user_id: Uuid,
        period: String,
    },
    MindfulnessTrends {
        user_id: Uuid,
        days: u32,
    },
    HealthSummary {
        user_id: Uuid,
        date_range: String,
    },
    UserMetrics {
        user_id: Uuid,
        metric_type: String,
    },
    // Authentication cache keys
    ApiKeyAuth {
        api_key_hash: String,
    },
    ApiKeyLookup {
        api_key_id: Uuid,
    },
    ApiKeyAuthHash {
        key_prefix: String,
        hash_suffix: String,
    },
}

impl CacheKey {
    pub fn to_redis_key(&self, prefix: &str) -> String {
        match self {
            CacheKey::HeartRateQuery { user_id, hash } => {
                format!("{prefix}:hr_query:{user_id}:{hash}")
            }
            CacheKey::BloodPressureQuery { user_id, hash } => {
                format!("{prefix}:bp_query:{user_id}:{hash}")
            }
            CacheKey::SleepQuery { user_id, hash } => {
                format!("{prefix}:sleep_query:{user_id}:{hash}")
            }
            CacheKey::ActivityQuery { user_id, hash } => {
                format!("{prefix}:activity_query:{user_id}:{hash}")
            }
            CacheKey::WorkoutQuery { user_id, hash } => {
                format!("{prefix}:workout_query:{user_id}:{hash}")
            }
            CacheKey::MindfulnessQuery { user_id, hash } => {
                format!("{prefix}:mindfulness_query:{user_id}:{hash}")
            }
            CacheKey::MentalHealthQuery { user_id, hash } => {
                format!("{prefix}:mental_health_query:{user_id}:{hash}")
            }
            CacheKey::MindfulnessInsights { user_id, period } => {
                format!("{prefix}:mindfulness_insights:{user_id}:{period}")
            }
            CacheKey::MentalHealthInsights { user_id, period } => {
                format!("{prefix}:mental_health_insights:{user_id}:{period}")
            }
            CacheKey::MindfulnessTrends { user_id, days } => {
                format!("{prefix}:mindfulness_trends:{user_id}:{days}d")
            }
            CacheKey::HealthSummary {
                user_id,
                date_range,
            } => format!("{prefix}:summary:{user_id}:{date_range}"),
            CacheKey::UserMetrics {
                user_id,
                metric_type,
            } => format!("{prefix}:metrics:{user_id}:{metric_type}"),
            // Authentication cache keys
            CacheKey::ApiKeyAuth { api_key_hash } => {
                format!("{prefix}:auth:{api_key_hash}")
            }
            CacheKey::ApiKeyLookup { api_key_id } => {
                format!("{prefix}:lookup:{api_key_id}")
            }
            CacheKey::ApiKeyAuthHash {
                key_prefix,
                hash_suffix,
            } => format!("{prefix}:auth_hash:{key_prefix}:{hash_suffix}"),
        }
    }
}

/// Cacheable data with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

impl CacheService {
    /// Create a new cache service with Redis connection
    pub async fn new(redis_url: &str, config: CacheConfig) -> Result<Self, RedisError> {
        if !config.enabled {
            info!("Cache service disabled by configuration");
            // Create a dummy connection for disabled cache
            let client = Client::open("redis://127.0.0.1:6379/")?; // Dummy URL
            let connection_manager = client.get_connection_manager().await?;
            return Ok(Self {
                connection_manager,
                default_ttl: Duration::from_secs(config.default_ttl_seconds),
                enabled: false,
            });
        }

        info!("Initializing Redis cache service with URL: {}", redis_url);

        let client = Client::open(redis_url)?;
        let connection_manager = client.get_connection_manager().await?;

        // Test the connection
        let mut conn = connection_manager.clone();
        let _: redis::RedisResult<()> = redis::cmd("PING").query_async(&mut conn).await;
        info!("✓ Redis cache service initialized successfully");

        Ok(Self {
            connection_manager,
            default_ttl: Duration::from_secs(config.default_ttl_seconds),
            enabled: true,
        })
    }

    /// Get cached data by key
    #[instrument(skip(self))]
    pub async fn get<T>(&self, key: &CacheKey, prefix: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        if !self.enabled {
            return None;
        }

        let redis_key = key.to_redis_key(prefix);
        let mut conn = self.connection_manager.clone();

        match conn.get::<_, String>(&redis_key).await {
            Ok(data) => {
                match serde_json::from_str::<CacheEntry<T>>(&data) {
                    Ok(entry) => {
                        // Check if entry is still valid
                        let age = chrono::Utc::now() - entry.cached_at;
                        if age.num_seconds() < entry.ttl_seconds as i64 {
                            info!(cache_key = %redis_key, "Cache hit");
                            Some(entry.data)
                        } else {
                            info!(cache_key = %redis_key, "Cache expired, removing");
                            let _ = self.delete(key, prefix).await;
                            None
                        }
                    }
                    Err(e) => {
                        warn!(cache_key = %redis_key, error = %e, "Failed to deserialize cache entry");
                        let _ = self.delete(key, prefix).await;
                        None
                    }
                }
            }
            Err(e) => {
                // Log only if it's not a typical cache miss
                if !e.to_string().contains("nil") {
                    error!(cache_key = %redis_key, error = %e, "Redis get error");
                }
                None
            }
        }
    }

    /// Set cached data with TTL
    #[instrument(skip(self, data))]
    pub async fn set<T>(&self, key: &CacheKey, prefix: &str, data: T, ttl: Option<Duration>) -> bool
    where
        T: Serialize,
    {
        if !self.enabled {
            return false;
        }

        let redis_key = key.to_redis_key(prefix);
        let ttl = ttl.unwrap_or(self.default_ttl);

        let entry = CacheEntry {
            data,
            cached_at: chrono::Utc::now(),
            ttl_seconds: ttl.as_secs(),
        };

        let mut conn = self.connection_manager.clone();

        match serde_json::to_string(&entry) {
            Ok(serialized) => {
                match conn
                    .set_ex::<_, _, ()>(&redis_key, serialized, ttl.as_secs())
                    .await
                {
                    Ok(_) => {
                        info!(cache_key = %redis_key, ttl_seconds = ttl.as_secs(), "Cache set");
                        true
                    }
                    Err(e) => {
                        error!(cache_key = %redis_key, error = %e, "Redis set error");
                        false
                    }
                }
            }
            Err(e) => {
                error!(cache_key = %redis_key, error = %e, "Serialization error");
                false
            }
        }
    }

    /// Delete cached data
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &CacheKey, prefix: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let redis_key = key.to_redis_key(prefix);
        let mut conn = self.connection_manager.clone();

        match conn.del::<_, ()>(&redis_key).await {
            Ok(_) => {
                info!(cache_key = %redis_key, "Cache deleted");
                true
            }
            Err(e) => {
                error!(cache_key = %redis_key, error = %e, "Redis delete error");
                false
            }
        }
    }

    /// Invalidate all cache entries for a user
    #[instrument(skip(self))]
    pub async fn invalidate_user_cache(&self, user_id: Uuid, prefix: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let pattern = format!("{prefix}:*:{user_id}:*");
        let mut conn = self.connection_manager.clone();

        match conn.keys::<_, Vec<String>>(&pattern).await {
            Ok(keys) => {
                if keys.is_empty() {
                    return true;
                }

                match conn.del::<_, ()>(keys.clone()).await {
                    Ok(_) => {
                        info!(
                            user_id = %user_id,
                            keys_deleted = keys.len(),
                            "User cache invalidated"
                        );
                        true
                    }
                    Err(e) => {
                        error!(
                            user_id = %user_id,
                            error = %e,
                            "Failed to invalidate user cache"
                        );
                        false
                    }
                }
            }
            Err(e) => {
                error!(
                    user_id = %user_id,
                    error = %e,
                    "Failed to find user cache keys"
                );
                false
            }
        }
    }

    /// Get cache statistics
    #[instrument(skip(self))]
    pub async fn get_stats(&self) -> CacheStats {
        if !self.enabled {
            return CacheStats::disabled();
        }

        let mut conn = self.connection_manager.clone();

        let info: String = match redis::cmd("INFO").arg("stats").query_async(&mut conn).await {
            Ok(info) => info,
            Err(_) => return CacheStats::error(),
        };

        // Parse Redis INFO stats
        let mut stats = CacheStats::default();
        for line in info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                match key {
                    "keyspace_hits" => stats.hits = value.parse().unwrap_or(0),
                    "keyspace_misses" => stats.misses = value.parse().unwrap_or(0),
                    "used_memory" => stats.memory_usage = value.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }

        stats
    }

    /// Warm up cache with frequently accessed data
    #[instrument(skip(self))]
    pub async fn warm_cache(&self, _user_ids: Vec<Uuid>) -> bool {
        if !self.enabled {
            return false;
        }

        // TODO: Implement cache warming strategy
        // This could pre-load recent summaries for active users
        info!("Cache warming not yet implemented");
        true
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub memory_usage: u64,
    pub hit_rate: f64,
    pub enabled: bool,
    pub error: bool,
}

impl CacheStats {
    fn disabled() -> Self {
        Self {
            hits: 0,
            misses: 0,
            memory_usage: 0,
            hit_rate: 0.0,
            enabled: false,
            error: false,
        }
    }

    fn error() -> Self {
        Self {
            hits: 0,
            misses: 0,
            memory_usage: 0,
            hit_rate: 0.0,
            enabled: true,
            error: true,
        }
    }
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            memory_usage: 0,
            hit_rate: 0.0,
            enabled: true,
            error: false,
        }
    }
}

impl CacheStats {
    pub fn calculate_hit_rate(&mut self) {
        let total = self.hits + self.misses;
        self.hit_rate = if total > 0 {
            self.hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
    }
}

/// Generate cache key hash from query parameters
pub fn generate_query_hash(params: &crate::handlers::query::QueryParams) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(format!("{params:?}").as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}
