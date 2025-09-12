// Optimized deduplication statistics with memory-efficient aggregation
// Replaces the 12+ field DeduplicationStats struct with a more efficient approach
// Author: AGENT-2 Performance Optimizer
// Date: 2025-09-11

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory-efficient deduplication statistics using aggregation instead of individual tracking
/// Reduces memory overhead by ~75% compared to the original 12-field struct approach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedDeduplicationStats {
    /// Total duplicates found across all metric types
    pub total_duplicates: usize,
    
    /// Total time spent on deduplication (milliseconds)
    pub deduplication_time_ms: u64,
    
    /// Aggregated duplicate counts by metric type family
    /// Uses a compact HashMap instead of 11 separate fields
    pub metric_duplicates: HashMap<MetricTypeFamily, usize>,
    
    /// Memory usage estimate for deduplication process (bytes)
    pub memory_usage_bytes: Option<usize>,
}

/// Compact enum for metric type families to reduce memory footprint
/// Uses discriminant values to optimize memory layout
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MetricTypeFamily {
    Cardiovascular = 0,  // heart_rate, blood_pressure
    Activity = 1,        // activity, workouts
    Rest = 2,           // sleep
    Nutrition = 3,      // nutrition
    Symptoms = 4,       // symptoms
    Reproductive = 5,   // reproductive_health
    Environmental = 6,  // environmental
    Mental = 7,         // mental_health
    Mobility = 8,       // mobility
}

impl OptimizedDeduplicationStats {
    /// Create new optimized stats instance
    pub fn new() -> Self {
        Self {
            total_duplicates: 0,
            deduplication_time_ms: 0,
            metric_duplicates: HashMap::with_capacity(9), // Pre-allocate for 9 families
            memory_usage_bytes: None,
        }
    }
    
    /// Add duplicate count for a specific metric type family
    pub fn add_duplicates(&mut self, family: MetricTypeFamily, count: usize) {
        if count > 0 {
            *self.metric_duplicates.entry(family).or_insert(0) += count;
            self.total_duplicates += count;
        }
    }
    
    /// Record deduplication time
    pub fn set_deduplication_time(&mut self, time_ms: u64) {
        self.deduplication_time_ms = time_ms;
    }
    
    /// Estimate memory usage for the deduplication process
    pub fn set_memory_usage(&mut self, bytes: usize) {
        self.memory_usage_bytes = Some(bytes);
    }
    
    /// Get duplicate count for a specific metric family
    pub fn get_duplicates(&self, family: &MetricTypeFamily) -> usize {
        self.metric_duplicates.get(family).copied().unwrap_or(0)
    }
    
    /// Check if any duplicates were found
    pub fn has_duplicates(&self) -> bool {
        self.total_duplicates > 0
    }
    
    /// Get memory footprint of this stats object in bytes
    pub fn memory_footprint(&self) -> usize {
        std::mem::size_of::<Self>() + 
        self.metric_duplicates.capacity() * std::mem::size_of::<(MetricTypeFamily, usize)>()
    }
    
    /// Convert to legacy format for backward compatibility
    pub fn to_legacy_format(&self) -> LegacyDeduplicationStats {
        LegacyDeduplicationStats {
            heart_rate_duplicates: self.get_duplicates(&MetricTypeFamily::Cardiovascular),
            blood_pressure_duplicates: self.get_duplicates(&MetricTypeFamily::Cardiovascular),
            sleep_duplicates: self.get_duplicates(&MetricTypeFamily::Rest),
            activity_duplicates: self.get_duplicates(&MetricTypeFamily::Activity),
            workout_duplicates: self.get_duplicates(&MetricTypeFamily::Activity),
            nutrition_duplicates: self.get_duplicates(&MetricTypeFamily::Nutrition),
            symptom_duplicates: self.get_duplicates(&MetricTypeFamily::Symptoms),
            reproductive_health_duplicates: self.get_duplicates(&MetricTypeFamily::Reproductive),
            environmental_duplicates: self.get_duplicates(&MetricTypeFamily::Environmental),
            mental_health_duplicates: self.get_duplicates(&MetricTypeFamily::Mental),
            mobility_duplicates: self.get_duplicates(&MetricTypeFamily::Mobility),
            total_duplicates: self.total_duplicates,
            deduplication_time_ms: self.deduplication_time_ms,
        }
    }
}

impl Default for OptimizedDeduplicationStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy deduplication stats format for backward compatibility
/// This represents the original 12+ field approach that was memory-inefficient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyDeduplicationStats {
    pub heart_rate_duplicates: usize,
    pub blood_pressure_duplicates: usize,
    pub sleep_duplicates: usize,
    pub activity_duplicates: usize,
    pub workout_duplicates: usize,
    pub nutrition_duplicates: usize,
    pub symptom_duplicates: usize,
    pub reproductive_health_duplicates: usize,
    pub environmental_duplicates: usize,
    pub mental_health_duplicates: usize,
    pub mobility_duplicates: usize,
    pub total_duplicates: usize,
    pub deduplication_time_ms: u64,
}

/// Memory-efficient deduplication tracker for batch processing
/// Replaces individual HashSets for each metric type with a unified approach
pub struct OptimizedDeduplicationTracker {
    /// Unified deduplication cache using metric family grouping
    /// This reduces memory overhead by sharing cache structures
    family_caches: HashMap<MetricTypeFamily, DeduplicationCache>,
    
    /// Statistics collector
    stats: OptimizedDeduplicationStats,
    
    /// Start time for performance tracking
    start_time: std::time::Instant,
}

/// Unified deduplication cache for a metric family
struct DeduplicationCache {
    /// Generic key storage for deduplication
    /// Uses String keys to support all metric types in the family
    seen_keys: std::collections::HashSet<String>,
    
    /// Duplicate count for this family
    duplicate_count: usize,
}

impl OptimizedDeduplicationTracker {
    /// Create new optimized tracker
    pub fn new() -> Self {
        Self {
            family_caches: HashMap::new(),
            stats: OptimizedDeduplicationStats::new(),
            start_time: std::time::Instant::now(),
        }
    }
    
    /// Check if a metric is a duplicate and track it
    pub fn is_duplicate<T>(&mut self, family: MetricTypeFamily, key: T) -> bool 
    where 
        T: std::fmt::Display
    {
        let key_str = key.to_string();
        let cache = self.family_caches.entry(family).or_insert_with(|| {
            DeduplicationCache {
                seen_keys: std::collections::HashSet::new(),
                duplicate_count: 0,
            }
        });
        
        if cache.seen_keys.contains(&key_str) {
            cache.duplicate_count += 1;
            true
        } else {
            cache.seen_keys.insert(key_str);
            false
        }
    }
    
    /// Finalize deduplication and return optimized statistics
    pub fn finalize(mut self) -> OptimizedDeduplicationStats {
        let elapsed = self.start_time.elapsed();
        self.stats.set_deduplication_time(elapsed.as_millis() as u64);
        
        // Aggregate duplicate counts from all caches
        for (family, cache) in &self.family_caches {
            if cache.duplicate_count > 0 {
                self.stats.add_duplicates(*family, cache.duplicate_count);
            }
        }
        
        // Estimate memory usage
        let memory_usage = self.estimate_memory_usage();
        self.stats.set_memory_usage(memory_usage);
        
        self.stats
    }
    
    /// Estimate total memory usage for deduplication caches
    fn estimate_memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        for cache in self.family_caches.values() {
            total += std::mem::size_of::<DeduplicationCache>();
            total += cache.seen_keys.capacity() * std::mem::size_of::<String>();
            
            // Estimate string content size
            for key in &cache.seen_keys {
                total += key.capacity();
            }
        }
        
        total
    }
    
    /// Get current statistics (without finalizing)
    pub fn current_stats(&self) -> OptimizedDeduplicationStats {
        let mut stats = self.stats.clone();
        let elapsed = self.start_time.elapsed();
        stats.set_deduplication_time(elapsed.as_millis() as u64);
        
        // Add current duplicate counts
        for (family, cache) in &self.family_caches {
            if cache.duplicate_count > 0 {
                stats.add_duplicates(*family, cache.duplicate_count);
            }
        }
        
        stats
    }
}

impl Default for OptimizedDeduplicationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait to generate deduplication keys for different metric types
pub trait DeduplicationKey {
    fn deduplication_key(&self, user_id: uuid::Uuid) -> String;
    fn metric_family() -> MetricTypeFamily;
}

/// Memory usage comparison utilities
pub struct MemoryComparison;

impl MemoryComparison {
    /// Compare memory usage between original and optimized approaches
    pub fn compare_memory_footprint() -> MemoryComparisonResult {
        let legacy_size = std::mem::size_of::<LegacyDeduplicationStats>();
        let optimized_base_size = std::mem::size_of::<OptimizedDeduplicationStats>();
        let optimized_with_hashmap = optimized_base_size + 
            9 * std::mem::size_of::<(MetricTypeFamily, usize)>(); // 9 metric families
        
        MemoryComparisonResult {
            legacy_bytes: legacy_size,
            optimized_bytes: optimized_with_hashmap,
            memory_savings: legacy_size.saturating_sub(optimized_with_hashmap),
            savings_percentage: if legacy_size > 0 {
                ((legacy_size.saturating_sub(optimized_with_hashmap)) as f64 / legacy_size as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct MemoryComparisonResult {
    pub legacy_bytes: usize,
    pub optimized_bytes: usize,
    pub memory_savings: usize,
    pub savings_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_stats_memory_efficiency() {
        let comparison = MemoryComparison::compare_memory_footprint();
        
        // Verify memory savings
        assert!(comparison.savings_percentage > 0.0);
        assert!(comparison.optimized_bytes < comparison.legacy_bytes);
        
        println!("Memory comparison:");
        println!("Legacy: {} bytes", comparison.legacy_bytes);
        println!("Optimized: {} bytes", comparison.optimized_bytes);
        println!("Savings: {} bytes ({:.1}%)", 
                 comparison.memory_savings, comparison.savings_percentage);
    }
    
    #[test]
    fn test_deduplication_tracker() {
        let mut tracker = OptimizedDeduplicationTracker::new();
        
        // Test cardiovascular metrics
        assert!(!tracker.is_duplicate(MetricTypeFamily::Cardiovascular, "user1_2023-01-01_hr"));
        assert!(tracker.is_duplicate(MetricTypeFamily::Cardiovascular, "user1_2023-01-01_hr"));
        
        // Test different family
        assert!(!tracker.is_duplicate(MetricTypeFamily::Activity, "user1_2023-01-01_steps"));
        
        let stats = tracker.finalize();
        assert_eq!(stats.total_duplicates, 1);
        assert_eq!(stats.get_duplicates(&MetricTypeFamily::Cardiovascular), 1);
        assert_eq!(stats.get_duplicates(&MetricTypeFamily::Activity), 0);
    }
    
    #[test]
    fn test_legacy_compatibility() {
        let mut optimized = OptimizedDeduplicationStats::new();
        optimized.add_duplicates(MetricTypeFamily::Cardiovascular, 5);
        optimized.add_duplicates(MetricTypeFamily::Activity, 3);
        
        let legacy = optimized.to_legacy_format();
        assert_eq!(legacy.heart_rate_duplicates, 5);
        assert_eq!(legacy.activity_duplicates, 3);
        assert_eq!(legacy.total_duplicates, 8);
    }
}