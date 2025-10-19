/// Cache configuration per entity type
/// 
/// TTLs and capacities tuned for different use cases:
/// - Token metadata: Long TTL (changes rarely)
/// - Market data: Medium TTL (DexScreener 30s, GeckoTerminal 1min)
/// - Security scores: Long TTL (expensive to compute)

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live for cached entries
    pub ttl: Duration,
    
    /// Maximum number of entries (LRU eviction when exceeded)
    pub capacity: usize,
}

impl CacheConfig {
    /// Token metadata cache (changes rarely)
    pub fn token_metadata() -> Self {
        Self {
            ttl: Duration::from_secs(3600), // 1 hour
            capacity: 5000,
        }
    }
    
    /// DexScreener market data (updates every 30s via API)
    pub fn market_dexscreener() -> Self {
        Self {
            ttl: Duration::from_secs(30),
            capacity: 2000,
        }
    }
    
    /// GeckoTerminal market data (updates every 1min via API)
    pub fn market_geckoterminal() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            capacity: 2000,
        }
    }
    
    /// Rugcheck security scores (expensive to fetch, stable data)
    pub fn security_rugcheck() -> Self {
        Self {
            ttl: Duration::from_secs(1800), // 30 minutes
            capacity: 3000,
        }
    }
    
    /// Blacklist (infrequent changes)
    pub fn blacklist() -> Self {
        Self {
            ttl: Duration::from_secs(600), // 10 minutes
            capacity: 1000,
        }
    }
    
    /// Custom configuration
    pub fn custom(ttl_secs: u64, capacity: usize) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            capacity,
        }
    }
}
