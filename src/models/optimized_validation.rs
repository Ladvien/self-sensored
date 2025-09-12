// Optimized validation system for health metrics with lazy evaluation
// Reduces validation overhead from 150+ field validations on every insert
// Author: AGENT-2 Performance Optimizer
// Date: 2025-09-11

use std::collections::HashMap;
use std::sync::{Arc, RwLock, OnceLock};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, anyhow};

/// Validation cache to store pre-computed validation results
/// Reduces repeated validation overhead for similar metric values
static VALIDATION_CACHE: OnceLock<Arc<RwLock<ValidationCache>>> = OnceLock::new();

/// Cached validation results with TTL
#[derive(Debug, Clone)]
struct CachedValidation {
    is_valid: bool,
    error_message: Option<String>,
    computed_at: DateTime<Utc>,
    ttl_seconds: u64,
}

impl CachedValidation {
    fn is_expired(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.computed_at);
        age.num_seconds() as u64 > self.ttl_seconds
    }
}

/// In-memory validation cache with LRU eviction
#[derive(Debug)]
struct ValidationCache {
    cache: HashMap<String, CachedValidation>,
    max_entries: usize,
    access_order: Vec<String>,
    cache_hits: u64,
    cache_misses: u64,
}

impl ValidationCache {
    fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_entries),
            max_entries,
            access_order: Vec::with_capacity(max_entries),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
    
    fn get(&mut self, key: &str) -> Option<&CachedValidation> {
        if let Some(cached) = self.cache.get(key) {
            if !cached.is_expired() {
                // Move to end (most recently used)
                if let Some(pos) = self.access_order.iter().position(|x| x == key) {
                    let key = self.access_order.remove(pos);
                    self.access_order.push(key);
                }
                self.cache_hits += 1;
                return Some(cached);
            } else {
                // Remove expired entry
                self.cache.remove(key);
                if let Some(pos) = self.access_order.iter().position(|x| x == key) {
                    self.access_order.remove(pos);
                }
            }
        }
        self.cache_misses += 1;
        None
    }
    
    fn put(&mut self, key: String, validation: CachedValidation) {
        // Evict oldest entries if at capacity
        while self.cache.len() >= self.max_entries && !self.access_order.is_empty() {
            let oldest = self.access_order.remove(0);
            self.cache.remove(&oldest);
        }
        
        self.cache.insert(key.clone(), validation);
        self.access_order.push(key);
    }
    
    fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

/// Lazy validation configuration
#[derive(Debug, Clone)]
pub struct LazyValidationConfig {
    /// Enable validation caching
    pub enable_caching: bool,
    
    /// Maximum cache entries
    pub max_cache_entries: usize,
    
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
    
    /// Skip validation for certain operations (e.g., migration)
    pub skip_validation_contexts: Vec<ValidationContext>,
    
    /// Batch validation for multiple metrics
    pub enable_batch_validation: bool,
}

impl Default for LazyValidationConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            max_cache_entries: 10000,
            cache_ttl_seconds: 3600, // 1 hour
            skip_validation_contexts: vec![ValidationContext::Migration, ValidationContext::BulkImport],
            enable_batch_validation: true,
        }
    }
}

/// Validation context to determine if validation can be skipped
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationContext {
    UserInput,      // Full validation required
    ApiEndpoint,    // Full validation required  
    Migration,      // Can skip for performance
    BulkImport,     // Can skip for performance
    Testing,        // Can skip for performance
    InternalBatch,  // Minimal validation
}

/// Lazy validator trait for health metrics
pub trait LazyValidation {
    /// Generate cache key for this metric's validation
    fn cache_key(&self) -> String;
    
    /// Perform actual validation (called only when cache miss occurs)
    fn validate_impl(&self, config: &crate::config::ValidationConfig) -> Result<()>;
    
    /// Lazy validation with caching and context awareness
    fn validate_lazy(
        &self, 
        config: &crate::config::ValidationConfig,
        lazy_config: &LazyValidationConfig,
        context: ValidationContext
    ) -> Result<()> {
        // Skip validation for certain contexts
        if lazy_config.skip_validation_contexts.contains(&context) {
            return Ok(());
        }
        
        // Check cache if enabled
        if lazy_config.enable_caching {
            let cache_key = self.cache_key();
            
            // Initialize cache if needed
            let cache_arc = VALIDATION_CACHE.get_or_init(|| {
                Arc::new(RwLock::new(ValidationCache::new(lazy_config.max_cache_entries)))
            });
            
            // Try to read from cache first
            {
                let mut cache = cache_arc.write().unwrap();
                if let Some(cached) = cache.get(&cache_key) {
                    return if cached.is_valid {
                        Ok(())
                    } else {
                        Err(anyhow!(cached.error_message.clone().unwrap_or("Validation failed".to_string())))
                    };
                }
            }
            
            // Cache miss - perform validation
            let validation_result = self.validate_impl(config);
            let is_valid = validation_result.is_ok();
            let error_message = if !is_valid {
                Some(validation_result.as_ref().err().unwrap().to_string())
            } else {
                None
            };
            
            // Store in cache
            {
                let mut cache = cache_arc.write().unwrap();
                cache.put(cache_key, CachedValidation {
                    is_valid,
                    error_message,
                    computed_at: Utc::now(),
                    ttl_seconds: lazy_config.cache_ttl_seconds,
                });
            }
            
            validation_result
        } else {
            // No caching - validate directly
            self.validate_impl(config)
        }
    }
}

/// Batch validation for multiple metrics of the same type
pub struct BatchValidator<T> {
    metrics: Vec<T>,
    config: LazyValidationConfig,
    validation_config: crate::config::ValidationConfig,
}

impl<T> BatchValidator<T> 
where 
    T: LazyValidation + Clone
{
    pub fn new(
        metrics: Vec<T>, 
        config: LazyValidationConfig,
        validation_config: crate::config::ValidationConfig
    ) -> Self {
        Self {
            metrics,
            config,
            validation_config,
        }
    }
    
    /// Validate all metrics in batch with optimizations
    pub fn validate_batch(&self, context: ValidationContext) -> Result<BatchValidationResult> {
        let start_time = std::time::Instant::now();
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        
        // Group metrics by cache key to avoid redundant validations
        let mut cache_key_groups: HashMap<String, Vec<usize>> = HashMap::new();
        
        for (idx, metric) in self.metrics.iter().enumerate() {
            let cache_key = metric.cache_key();
            cache_key_groups.entry(cache_key).or_insert_with(Vec::new).push(idx);
        }
        
        // Validate one metric per cache key group
        for (cache_key, indices) in cache_key_groups {
            let metric = &self.metrics[indices[0]]; // Validate first metric in group
            
            match metric.validate_lazy(&self.validation_config, &self.config, context.clone()) {
                Ok(()) => {
                    successful += indices.len();
                }
                Err(e) => {
                    failed += indices.len();
                    for &idx in &indices {
                        errors.push(BatchValidationError {
                            metric_index: idx,
                            error: e.to_string(),
                            cache_key: cache_key.clone(),
                        });
                    }
                }
            }
        }
        
        let duration = start_time.elapsed();
        
        Ok(BatchValidationResult {
            total_metrics: self.metrics.len(),
            successful_validations: successful,
            failed_validations: failed,
            errors,
            validation_time_ms: duration.as_millis() as u64,
            cache_groups_used: cache_key_groups.len(),
        })
    }
}

/// Result of batch validation
#[derive(Debug)]
pub struct BatchValidationResult {
    pub total_metrics: usize,
    pub successful_validations: usize,
    pub failed_validations: usize,
    pub errors: Vec<BatchValidationError>,
    pub validation_time_ms: u64,
    pub cache_groups_used: usize,
}

#[derive(Debug)]
pub struct BatchValidationError {
    pub metric_index: usize,
    pub error: String,
    pub cache_key: String,
}

/// Performance monitoring for validation system
pub struct ValidationPerformanceMonitor;

impl ValidationPerformanceMonitor {
    /// Get current cache statistics
    pub fn get_cache_stats() -> Option<ValidationCacheStats> {
        VALIDATION_CACHE.get().and_then(|cache_arc| {
            if let Ok(cache) = cache_arc.read() {
                Some(ValidationCacheStats {
                    total_entries: cache.cache.len(),
                    max_entries: cache.max_entries,
                    cache_hits: cache.cache_hits,
                    cache_misses: cache.cache_misses,
                    hit_rate: cache.hit_rate(),
                })
            } else {
                None
            }
        })
    }
    
    /// Clear validation cache (useful for testing)
    pub fn clear_cache() {
        if let Some(cache_arc) = VALIDATION_CACHE.get() {
            if let Ok(mut cache) = cache_arc.write() {
                cache.cache.clear();
                cache.access_order.clear();
                cache.cache_hits = 0;
                cache.cache_misses = 0;
            }
        }
    }
    
    /// Benchmark validation performance
    pub fn benchmark_validation<T>(
        metrics: Vec<T>,
        iterations: usize
    ) -> ValidationBenchmarkResult 
    where 
        T: LazyValidation + Clone
    {
        let validation_config = crate::config::ValidationConfig::from_env();
        let lazy_config = LazyValidationConfig::default();
        
        // Benchmark without caching
        Self::clear_cache();
        let mut no_cache_config = lazy_config.clone();
        no_cache_config.enable_caching = false;
        
        let start_time = std::time::Instant::now();
        for _ in 0..iterations {
            for metric in &metrics {
                let _ = metric.validate_lazy(&validation_config, &no_cache_config, ValidationContext::Testing);
            }
        }
        let no_cache_duration = start_time.elapsed();
        
        // Benchmark with caching
        Self::clear_cache();
        let start_time = std::time::Instant::now();
        for _ in 0..iterations {
            for metric in &metrics {
                let _ = metric.validate_lazy(&validation_config, &lazy_config, ValidationContext::Testing);
            }
        }
        let cached_duration = start_time.elapsed();
        
        let cache_stats = Self::get_cache_stats();
        
        ValidationBenchmarkResult {
            metrics_count: metrics.len(),
            iterations,
            no_cache_duration_ms: no_cache_duration.as_millis() as u64,
            cached_duration_ms: cached_duration.as_millis() as u64,
            performance_improvement: if no_cache_duration.as_millis() > 0 {
                ((no_cache_duration.as_millis() - cached_duration.as_millis()) as f64 
                 / no_cache_duration.as_millis() as f64) * 100.0
            } else {
                0.0
            },
            cache_stats,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ValidationCacheStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
}

#[derive(Debug)]
pub struct ValidationBenchmarkResult {
    pub metrics_count: usize,
    pub iterations: usize,
    pub no_cache_duration_ms: u64,
    pub cached_duration_ms: u64,
    pub performance_improvement: f64,
    pub cache_stats: Option<ValidationCacheStats>,
}

/// Implementation of lazy validation for existing health metrics

impl LazyValidation for crate::models::health_metrics::HeartRateMetric {
    fn cache_key(&self) -> String {
        format!(
            "heartrate_{}_{}",
            self.heart_rate,
            self.resting_heart_rate.unwrap_or(0)
        )
    }
    
    fn validate_impl(&self, config: &crate::config::ValidationConfig) -> Result<()> {
        self.validate_with_config(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_cache() {
        let mut cache = ValidationCache::new(3);
        
        let validation = CachedValidation {
            is_valid: true,
            error_message: None,
            computed_at: Utc::now(),
            ttl_seconds: 3600,
        };
        
        cache.put("test_key".to_string(), validation);
        assert!(cache.get("test_key").is_some());
        assert!(cache.get("nonexistent").is_none());
        
        assert_eq!(cache.cache_hits, 1);
        assert_eq!(cache.cache_misses, 1);
    }
    
    #[test]
    fn test_cache_eviction() {
        let mut cache = ValidationCache::new(2);
        
        let validation = CachedValidation {
            is_valid: true,
            error_message: None,
            computed_at: Utc::now(),
            ttl_seconds: 3600,
        };
        
        cache.put("key1".to_string(), validation.clone());
        cache.put("key2".to_string(), validation.clone());
        cache.put("key3".to_string(), validation); // Should evict key1
        
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }
}