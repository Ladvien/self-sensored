use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::OnceCell;
use tracing::{error, info, warn};

/// Lazy-loaded data mapping for Apple HealthKit types
/// Replaces the large static DATA.md file loaded at compile time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMappingEntry {
    pub healthkit_identifier: String,
    pub description: String,
    pub support_level: SupportLevel,
    pub category: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SupportLevel {
    FullySupported,
    Partial,
    Planned,
    PartialUncertain,
    NotSupported,
}

impl SupportLevel {
    pub fn from_symbol(symbol: &str) -> Self {
        match symbol {
            "✅" => SupportLevel::FullySupported,
            "⚠️" => SupportLevel::PartialUncertain,
            "❌" => SupportLevel::NotSupported,
            _ => SupportLevel::PartialUncertain,
        }
    }

    pub fn to_symbol(&self) -> &'static str {
        match self {
            SupportLevel::FullySupported => "✅",
            SupportLevel::Partial => "⚠️",
            SupportLevel::Planned => "📋",
            SupportLevel::PartialUncertain => "⚠️",
            SupportLevel::NotSupported => "❌",
        }
    }
}

/// Configuration for data loader
#[derive(Debug, Clone)]
pub struct DataLoaderConfig {
    /// Whether to preload all data on first access
    pub preload_on_startup: bool,
    /// Cache timeout in seconds (0 = never expire)
    pub cache_timeout_secs: u64,
    /// Load data from database instead of static file
    pub use_database_source: bool,
    /// Database table name for health data mappings
    pub database_table: String,
}

impl Default for DataLoaderConfig {
    fn default() -> Self {
        Self {
            preload_on_startup: false,
            cache_timeout_secs: 3600, // 1 hour cache
            use_database_source: false,
            database_table: "health_data_mappings".to_string(),
        }
    }
}

/// Lazy data loader for health data mappings
pub struct LazyDataLoader {
    config: DataLoaderConfig,
    cache: Arc<RwLock<Option<HashMap<String, DataMappingEntry>>>>,
    last_loaded: Arc<RwLock<Option<std::time::Instant>>>,
    pool: Option<sqlx::PgPool>,
    redis_client: Option<Arc<crate::services::cache::CacheService>>,
}

impl LazyDataLoader {
    pub fn new(config: DataLoaderConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(None)),
            last_loaded: Arc::new(RwLock::new(None)),
            pool: None,
            redis_client: None,
        }
    }

    pub fn with_database(mut self, pool: sqlx::PgPool) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn with_redis(mut self, redis_client: Arc<crate::services::cache::CacheService>) -> Self {
        self.redis_client = Some(redis_client);
        self
    }

    pub fn with_default_config() -> Self {
        Self::new(DataLoaderConfig::default())
    }

    /// Get health data mapping for a specific identifier
    pub async fn get_mapping(&self, identifier: &str) -> Option<DataMappingEntry> {
        // Check if we need to load data
        if self.should_reload().await {
            if let Err(e) = self.load_data().await {
                error!("Failed to load health data mappings: {}", e);
                return None;
            }
        }

        // Get from cache
        let cache = match self.cache.read() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire read lock on cache: {}", e);
                return None::<DataMappingEntry>;
            }
        };
        cache.as_ref()?.get(identifier).cloned()
    }

    /// Get all mappings for a specific category
    pub async fn get_category_mappings(&self, category: &str) -> Vec<DataMappingEntry> {
        if self.should_reload().await {
            if let Err(e) = self.load_data().await {
                error!("Failed to load health data mappings: {}", e);
                return Vec::new();
            }
        }

        match self.cache.read() {
            Ok(cache) => {
                if let Some(data) = cache.as_ref() {
                    data.values()
                        .filter(|entry| entry.category.eq_ignore_ascii_case(category))
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Err(e) => {
                error!("Failed to acquire read lock on cache: {}", e);
                Vec::new()
            }
        }
    }

    /// Get all supported identifiers
    pub async fn get_supported_identifiers(&self) -> Vec<String> {
        if self.should_reload().await {
            if let Err(e) = self.load_data().await {
                error!("Failed to load health data mappings: {}", e);
                return Vec::new();
            }
        }

        match self.cache.read() {
            Ok(cache) => {
                if let Some(data) = cache.as_ref() {
                    data.values()
                        .filter(|entry| entry.support_level == SupportLevel::FullySupported)
                        .map(|entry| entry.healthkit_identifier.clone())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Err(e) => {
                error!("Failed to acquire read lock on cache: {}", e);
                Vec::new()
            }
        }
    }

    /// Check support level for a specific identifier
    pub async fn get_support_level(&self, identifier: &str) -> Option<SupportLevel> {
        self.get_mapping(identifier)
            .await
            .map(|entry| entry.support_level)
    }

    /// Get statistics about data mappings
    pub async fn get_statistics(&self) -> DataMappingStats {
        if self.should_reload().await {
            if let Err(e) = self.load_data().await {
                error!("Failed to load health data mappings: {}", e);
                return DataMappingStats::default();
            }
        }

        match self.cache.read() {
            Ok(cache) => {
                if let Some(data) = cache.as_ref() {
                    let mut stats = DataMappingStats::default();

                    for entry in data.values() {
                        stats.total_count += 1;
                        match entry.support_level {
                            SupportLevel::FullySupported => stats.fully_supported += 1,
                            SupportLevel::Partial | SupportLevel::PartialUncertain => {
                                stats.partial_uncertain += 1
                            }
                            SupportLevel::Planned => stats.partial_uncertain += 1, // Count planned as partial for stats
                            SupportLevel::NotSupported => stats.not_supported += 1,
                        }

                        *stats
                            .category_counts
                            .entry(entry.category.clone())
                            .or_insert(0) += 1;
                    }

                    stats
                } else {
                    DataMappingStats::default()
                }
            }
            Err(e) => {
                error!("Failed to acquire read lock on cache: {}", e);
                DataMappingStats::default()
            }
        }
    }

    /// Force reload of data mappings
    pub async fn reload(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.load_data().await
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        match self.cache.write() {
            Ok(mut cache) => *cache = None,
            Err(e) => error!("Failed to acquire write lock on cache: {}", e),
        }
        match self.last_loaded.write() {
            Ok(mut last_loaded) => *last_loaded = None,
            Err(e) => error!("Failed to acquire write lock on last_loaded: {}", e),
        }
        info!("Health data mappings cache cleared");
    }

    /// Check if data should be reloaded
    async fn should_reload(&self) -> bool {
        let last_loaded = match self.last_loaded.read() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire read lock on last_loaded: {}", e);
                return true; // Assume reload needed on error
            }
        };

        match *last_loaded {
            None => true, // Never loaded
            Some(loaded_time) => {
                if self.config.cache_timeout_secs == 0 {
                    false // Never expire
                } else {
                    loaded_time.elapsed().as_secs() > self.config.cache_timeout_secs
                }
            }
        }
    }

    /// Load data from source (database or static data)
    async fn load_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Loading health data mappings...");

        let data = if self.config.use_database_source {
            self.load_from_database().await?
        } else {
            self.load_static_data().await?
        };

        // Update cache
        match self.cache.write() {
            Ok(mut cache) => *cache = Some(data),
            Err(e) => {
                error!("Failed to acquire write lock on cache: {}", e);
                return Err(format!("Failed to update cache: {e}").into());
            }
        }

        // Update last loaded time
        match self.last_loaded.write() {
            Ok(mut last_loaded) => *last_loaded = Some(std::time::Instant::now()),
            Err(e) => {
                error!("Failed to acquire write lock on last_loaded: {}", e);
                return Err(format!("Failed to update last loaded time: {e}").into());
            }
        }

        info!("Health data mappings loaded successfully");
        Ok(())
    }

    /// Load data from database with Redis caching
    async fn load_from_database(
        &self,
    ) -> Result<HashMap<String, DataMappingEntry>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if database is available
        let pool = match &self.pool {
            Some(p) => p,
            None => {
                warn!("No database connection available, falling back to static data");
                return self.load_static_data().await;
            }
        };

        // Check Redis cache first (simplified - skip for now as cache requires CacheKey)
        // TODO: Implement proper cache key structure for data mappings

        // Load from database using dynamic query (table may not exist at compile time)
        let mappings_result =
            sqlx::query_as::<_, (String, String, String, String, Option<String>)>(
                r#"
            SELECT
                healthkit_identifier,
                description,
                support_level,
                category,
                notes
            FROM data_mappings
            WHERE is_active = true
            ORDER BY healthkit_identifier
            "#,
            )
            .fetch_all(pool)
            .await;

        match mappings_result {
            Ok(rows) => {
                let mut mappings = HashMap::new();

                for (healthkit_identifier, description, support_level_str, category, notes) in rows
                {
                    let support_level = match support_level_str.as_str() {
                        "fully_supported" => SupportLevel::FullySupported,
                        "partial" => SupportLevel::Partial,
                        "planned" => SupportLevel::Planned,
                        "not_supported" => SupportLevel::NotSupported,
                        _ => SupportLevel::NotSupported,
                    };

                    let entry = DataMappingEntry {
                        healthkit_identifier: healthkit_identifier.clone(),
                        description,
                        support_level,
                        category,
                        notes,
                    };

                    mappings.insert(healthkit_identifier, entry);
                }

                info!("Loaded {} data mappings from database", mappings.len());

                // Cache in Redis for 1 hour (simplified - skip for now as cache requires CacheKey)
                // TODO: Implement proper cache key structure for data mappings

                Ok(mappings)
            }
            Err(e) => {
                warn!(
                    "Failed to load from database: {}, falling back to static data",
                    e
                );
                self.load_static_data().await
            }
        }
    }

    /// Load essential static data (reduced set instead of full DATA.md)
    async fn load_static_data(
        &self,
    ) -> Result<HashMap<String, DataMappingEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let mut mappings = HashMap::new();

        // Essential core mappings only (instead of loading the full 313-line DATA.md file)
        let core_mappings = [
            (
                "HKQuantityTypeIdentifierStepCount",
                "Step count",
                SupportLevel::FullySupported,
                "ACTIVITY",
            ),
            (
                "HKQuantityTypeIdentifierHeartRate",
                "Heart rate",
                SupportLevel::FullySupported,
                "HEART",
            ),
            (
                "HKQuantityTypeIdentifierActiveEnergyBurned",
                "Active calories",
                SupportLevel::FullySupported,
                "ENERGY",
            ),
            (
                "HKQuantityTypeIdentifierBodyMass",
                "Body weight",
                SupportLevel::FullySupported,
                "BODY",
            ),
            (
                "HKCategoryTypeIdentifierSleepAnalysis",
                "Sleep stages",
                SupportLevel::FullySupported,
                "SLEEP",
            ),
            (
                "HKQuantityTypeIdentifierBloodPressureSystolic",
                "Systolic BP",
                SupportLevel::FullySupported,
                "BLOOD_PRESSURE",
            ),
            (
                "HKQuantityTypeIdentifierBloodPressureDiastolic",
                "Diastolic BP",
                SupportLevel::FullySupported,
                "BLOOD_PRESSURE",
            ),
            (
                "HKWorkoutType",
                "All workout types",
                SupportLevel::FullySupported,
                "WORKOUTS",
            ),
            // Add more core mappings as needed, but keep it minimal
        ];

        for (identifier, description, support_level, category) in &core_mappings {
            mappings.insert(
                identifier.to_string(),
                DataMappingEntry {
                    healthkit_identifier: identifier.to_string(),
                    description: description.to_string(),
                    support_level: support_level.clone(),
                    category: category.to_string(),
                    notes: None,
                },
            );
        }

        info!("Loaded {} core health data mappings", mappings.len());
        Ok(mappings)
    }
}

/// Statistics about data mappings
#[derive(Debug, Default)]
pub struct DataMappingStats {
    pub total_count: usize,
    pub fully_supported: usize,
    pub partial_uncertain: usize,
    pub not_supported: usize,
    pub category_counts: HashMap<String, usize>,
}

impl DataMappingStats {
    pub fn get_support_percentage(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.fully_supported as f64 / self.total_count as f64) * 100.0
        }
    }
}

/// Global lazy data loader instance
static GLOBAL_DATA_LOADER: OnceCell<LazyDataLoader> = OnceCell::const_new();

/// Initialize the global data loader
pub async fn initialize_data_loader(config: DataLoaderConfig) -> &'static LazyDataLoader {
    GLOBAL_DATA_LOADER
        .get_or_init(|| async {
            let loader = LazyDataLoader::new(config);
            if loader.config.preload_on_startup {
                if let Err(e) = loader.load_data().await {
                    error!("Failed to preload health data mappings: {}", e);
                }
            }
            loader
        })
        .await
}

/// Get the global data loader instance
pub async fn get_data_loader() -> &'static LazyDataLoader {
    GLOBAL_DATA_LOADER
        .get_or_init(|| async { LazyDataLoader::with_default_config() })
        .await
}

/// Migration recommendations for moving from static file to database
pub struct DataLoaderMigrationPath {
    pub current_approach: &'static str,
    pub recommended_approach: &'static str,
    pub migration_steps: Vec<&'static str>,
    pub benefits: Vec<&'static str>,
    pub estimated_effort_days: u32,
}

impl Default for DataLoaderMigrationPath {
    fn default() -> Self {
        Self {
            current_approach: "Large static DATA.md file (313 lines) loaded at compile time",
            recommended_approach: "Lazy-loaded database-driven mappings with caching",
            migration_steps: vec![
                "1. Create health_data_mappings table in database",
                "2. Import current DATA.md content into database table",
                "3. Update DataLoader to use database source",
                "4. Implement cache invalidation strategies",
                "5. Add admin interface for managing mappings",
                "6. Remove static DATA.md file dependency",
                "7. Add monitoring for mapping cache performance",
            ],
            benefits: vec![
                "Reduced memory usage at application startup",
                "Dynamic updates without application restart",
                "Better cache control and invalidation",
                "Easier maintenance of health data mappings",
                "Improved application startup time",
                "Better observability of data usage patterns",
            ],
            estimated_effort_days: 5, // 1 week for complete migration
        }
    }
}
