use std::time::Duration;
use serde::{ Deserialize, Serialize };

/// Configuration for pricing updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    pub update_interval_secs: u64,
    pub top_tokens_count: usize,
    pub cache_ttl_secs: u64,
    pub max_cache_size: usize,
    pub enable_dynamic_pricing: bool,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            update_interval_secs: 60,
            top_tokens_count: 1000,
            cache_ttl_secs: 300,
            max_cache_size: 10000,
            enable_dynamic_pricing: false,
        }
    }
}

impl PricingConfig {
    pub fn update_interval(&self) -> Duration {
        Duration::from_secs(self.update_interval_secs)
    }

    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }
}

/// Pricing statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingMetrics {
    pub total_tokens_tracked: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub gecko_requests: u64,
    pub pool_calculations: u64,
    pub last_update: u64,
}

impl Default for PricingMetrics {
    fn default() -> Self {
        Self {
            total_tokens_tracked: 0,
            cache_hits: 0,
            cache_misses: 0,
            gecko_requests: 0,
            pool_calculations: 0,
            last_update: 0,
        }
    }
}

impl PricingMetrics {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64) / (total as f64)
        }
    }
}
