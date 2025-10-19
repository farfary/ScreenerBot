/// Generic cache system - reusable for tokens, pools, ohlcvs, any data
/// 
/// Features:
/// - TTL-based expiration
/// - LRU eviction when capacity reached
/// - Thread-safe access (Arc<RwLock<...>>)
/// - Metrics tracking (hits, misses, evictions)

pub mod config;
pub mod manager;

pub use config::CacheConfig;
pub use manager::CacheManager;
