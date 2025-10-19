/// Generic in-memory cache with TTL and LRU eviction
/// 
/// Thread-safe, generic over key/value types.
/// Tracks metrics for monitoring.

use super::config::CacheConfig;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache entry with TTL tracking
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            inserted_at: now,
            last_accessed: now,
        }
    }
    
    fn is_expired(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() > ttl
    }
    
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

/// Cache metrics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub inserts: u64,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Generic cache manager
pub struct CacheManager<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    config: CacheConfig,
    data: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    access_order: Arc<RwLock<VecDeque<K>>>, // For LRU tracking
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl<K, V> CacheManager<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Create new cache with given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            data: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }
    
    /// Get value from cache (returns None if expired or missing)
    pub fn get(&self, key: &K) -> Option<V> {
        // Read lock for checking
        let mut data = self.data.write().unwrap();
        
        if let Some(entry) = data.get_mut(key) {
            // Check expiration
            if entry.is_expired(self.config.ttl) {
                // Expired - remove and count
                data.remove(key);
                self.remove_from_access_order(key);
                
                let mut metrics = self.metrics.write().unwrap();
                metrics.misses += 1;
                metrics.expirations += 1;
                
                return None;
            }
            
            // Valid entry - touch and return
            entry.touch();
            self.update_access_order(key);
            
            let mut metrics = self.metrics.write().unwrap();
            metrics.hits += 1;
            
            Some(entry.value.clone())
        } else {
            // Cache miss
            let mut metrics = self.metrics.write().unwrap();
            metrics.misses += 1;
            None
        }
    }
    
    /// Insert value into cache (evicts LRU if at capacity)
    pub fn insert(&self, key: K, value: V) {
        let mut data = self.data.write().unwrap();
        
        // Check if we need to evict
        if data.len() >= self.config.capacity && !data.contains_key(&key) {
            self.evict_lru(&mut data);
        }
        
        // Insert new entry
        data.insert(key.clone(), CacheEntry::new(value));
        self.update_access_order(&key);
        
        let mut metrics = self.metrics.write().unwrap();
        metrics.inserts += 1;
    }
    
    /// Remove specific key from cache
    pub fn remove(&self, key: &K) {
        let mut data = self.data.write().unwrap();
        data.remove(key);
        self.remove_from_access_order(key);
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
        
        let mut access_order = self.access_order.write().unwrap();
        access_order.clear();
    }
    
    /// Get current metrics
    pub fn metrics(&self) -> CacheMetrics {
        self.metrics.read().unwrap().clone()
    }
    
    /// Get current cache size
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }
    
    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    // Private: Evict least recently used entry
    fn evict_lru(&self, data: &mut HashMap<K, CacheEntry<V>>) {
        let mut access_order = self.access_order.write().unwrap();
        
        if let Some(lru_key) = access_order.pop_front() {
            data.remove(&lru_key);
            
            let mut metrics = self.metrics.write().unwrap();
            metrics.evictions += 1;
        }
    }
    
    // Private: Update access order for LRU tracking
    fn update_access_order(&self, key: &K) {
        let mut access_order = self.access_order.write().unwrap();
        
        // Remove from current position
        access_order.retain(|k| k != key);
        
        // Add to back (most recently used)
        access_order.push_back(key.clone());
    }
    
    // Private: Remove key from access order
    fn remove_from_access_order(&self, key: &K) {
        let mut access_order = self.access_order.write().unwrap();
        access_order.retain(|k| k != key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_basic_operations() {
        let config = CacheConfig::custom(60, 100);
        let cache = CacheManager::new(config);
        
        // Insert and get
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // Miss
        assert_eq!(cache.get(&"nonexistent".to_string()), None);
        
        // Metrics
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
    }
    
    #[test]
    fn test_ttl_expiration() {
        let config = CacheConfig::custom(1, 100); // 1 second TTL
        let cache = CacheManager::new(config);
        
        cache.insert("key".to_string(), "value".to_string());
        assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));
        
        // Wait for expiration
        thread::sleep(Duration::from_secs(2));
        assert_eq!(cache.get(&"key".to_string()), None);
    }
    
    #[test]
    fn test_lru_eviction() {
        let config = CacheConfig::custom(60, 2); // Capacity of 2
        let cache = CacheManager::new(config);
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string()); // Should evict key1
        
        assert_eq!(cache.get(&"key1".to_string()), None); // Evicted
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), Some("value3".to_string()));
    }
}
