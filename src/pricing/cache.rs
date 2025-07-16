use std::collections::HashMap;
use std::time::{ Duration, SystemTime, UNIX_EPOCH };
use serde::{ Deserialize, Serialize };
use crate::pricing::{ TokenInfo, TokenPrice, PoolInfo };

#[derive(Debug, Clone)]
pub struct PriceCache {
    tokens: HashMap<String, CachedTokenInfo>,
    prices: HashMap<String, CachedPrice>,
    pools: HashMap<String, CachedPool>,
    cache_ttl: Duration,
    max_cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedTokenInfo {
    token_info: TokenInfo,
    cached_at: u64, // Unix timestamp
    access_count: u64,
    last_access: u64, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPrice {
    price: TokenPrice,
    cached_at: u64, // Unix timestamp
    access_count: u64,
    last_access: u64, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedPool {
    pool_info: PoolInfo,
    cached_at: u64, // Unix timestamp
    access_count: u64,
    last_access: u64, // Unix timestamp
}

impl PriceCache {
    pub fn new() -> Self {
        Self::with_config(
            Duration::from_secs(300), // 5 minutes TTL
            10000 // Max 10k cached items
        )
    }

    pub fn with_config(cache_ttl: Duration, max_cache_size: usize) -> Self {
        Self {
            tokens: HashMap::new(),
            prices: HashMap::new(),
            pools: HashMap::new(),
            cache_ttl,
            max_cache_size,
        }
    }

    pub async fn get_token_info(&self, token_address: &str) -> Option<TokenInfo> {
        if let Some(cached) = self.tokens.get(token_address) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now - cached.cached_at < self.cache_ttl.as_secs() {
                // Update access statistics (would need interior mutability in real impl)
                return Some(cached.token_info.clone());
            }
        }
        None
    }

    pub async fn get_token_price(&self, token_address: &str) -> Option<TokenPrice> {
        if let Some(cached) = self.prices.get(token_address) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now - cached.cached_at < self.cache_ttl.as_secs() {
                return Some(cached.price.clone());
            }
        }
        None
    }

    pub async fn get_pool_info(&self, pool_address: &str) -> Option<PoolInfo> {
        if let Some(cached) = self.pools.get(pool_address) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now - cached.cached_at < self.cache_ttl.as_secs() {
                return Some(cached.pool_info.clone());
            }
        }
        None
    }

    pub async fn update_token_info(&mut self, token_info: TokenInfo) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Check if we need to evict old entries
        if self.tokens.len() >= self.max_cache_size {
            self.evict_old_tokens().await;
        }

        let cached_info = CachedTokenInfo {
            token_info: token_info.clone(),
            cached_at: now,
            access_count: 1,
            last_access: now,
        };

        self.tokens.insert(token_info.address.clone(), cached_info);

        // Also update price if available
        if let Some(price) = token_info.price {
            self.update_token_price(price).await;
        }

        // Update pool information
        for pool in token_info.pools {
            self.update_pool_info(pool).await;
        }
    }

    pub async fn update_token_price(&mut self, price: TokenPrice) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        if self.prices.len() >= self.max_cache_size {
            self.evict_old_prices().await;
        }

        let cached_price = CachedPrice {
            price: price.clone(),
            cached_at: now,
            access_count: 1,
            last_access: now,
        };

        self.prices.insert(price.address.clone(), cached_price);
    }

    pub async fn update_pool_info(&mut self, pool_info: PoolInfo) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        if self.pools.len() >= self.max_cache_size {
            self.evict_old_pools().await;
        }

        let cached_pool = CachedPool {
            pool_info: pool_info.clone(),
            cached_at: now,
            access_count: 1,
            last_access: now,
        };

        self.pools.insert(pool_info.address.clone(), cached_pool);
    }

    pub async fn get_all_cached_tokens(&self) -> Vec<TokenInfo> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.tokens
            .values()
            .filter(|cached| now - cached.cached_at < self.cache_ttl.as_secs())
            .map(|cached| cached.token_info.clone())
            .collect()
    }

    pub async fn get_all_cached_prices(&self) -> Vec<TokenPrice> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.prices
            .values()
            .filter(|cached| now - cached.cached_at < self.cache_ttl.as_secs())
            .map(|cached| cached.price.clone())
            .collect()
    }

    pub async fn get_cached_pools_for_token(&self, token_address: &str) -> Vec<PoolInfo> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        self.pools
            .values()
            .filter(|cached| {
                now - cached.cached_at < self.cache_ttl.as_secs() &&
                    (cached.pool_info.token_0 == token_address ||
                        cached.pool_info.token_1 == token_address)
            })
            .map(|cached| cached.pool_info.clone())
            .collect()
    }

    pub async fn invalidate_token(&mut self, token_address: &str) {
        self.tokens.remove(token_address);
        self.prices.remove(token_address);

        // Remove pools that contain this token
        self.pools.retain(|_, cached| {
            cached.pool_info.token_0 != token_address && cached.pool_info.token_1 != token_address
        });
    }

    pub async fn invalidate_pool(&mut self, pool_address: &str) {
        self.pools.remove(pool_address);
    }

    pub async fn clear_expired(&mut self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        self.tokens.retain(|_, cached| now - cached.cached_at < self.cache_ttl.as_secs());
        self.prices.retain(|_, cached| now - cached.cached_at < self.cache_ttl.as_secs());
        self.pools.retain(|_, cached| now - cached.cached_at < self.cache_ttl.as_secs());
    }

    pub async fn get_cache_stats(&self) -> CacheStats {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let valid_tokens = self.tokens
            .values()
            .filter(|cached| now - cached.cached_at < self.cache_ttl.as_secs())
            .count();

        let valid_prices = self.prices
            .values()
            .filter(|cached| now - cached.cached_at < self.cache_ttl.as_secs())
            .count();

        let valid_pools = self.pools
            .values()
            .filter(|cached| now - cached.cached_at < self.cache_ttl.as_secs())
            .count();

        CacheStats {
            total_tokens: self.tokens.len(),
            valid_tokens,
            total_prices: self.prices.len(),
            valid_prices,
            total_pools: self.pools.len(),
            valid_pools,
            cache_ttl_seconds: self.cache_ttl.as_secs(),
            max_cache_size: self.max_cache_size,
        }
    }

    async fn evict_old_tokens(&mut self) {
        // Evict 20% of oldest/least accessed tokens
        let evict_count = self.max_cache_size / 5;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut tokens_by_score: Vec<(String, f64)> = self.tokens
            .iter()
            .map(|(addr, cached)| {
                // Score based on age and access frequency
                let age_penalty = (now - cached.cached_at) as f64;
                let access_bonus = cached.access_count as f64;
                let score = access_bonus / (1.0 + age_penalty / 3600.0); // Hourly decay
                (addr.clone(), score)
            })
            .collect();

        tokens_by_score.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (addr, _) in tokens_by_score.into_iter().take(evict_count) {
            self.tokens.remove(&addr);
        }
    }

    async fn evict_old_prices(&mut self) {
        let evict_count = self.max_cache_size / 5;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut prices_by_score: Vec<(String, f64)> = self.prices
            .iter()
            .map(|(addr, cached)| {
                let age_penalty = (now - cached.cached_at) as f64;
                let access_bonus = cached.access_count as f64;
                let score = access_bonus / (1.0 + age_penalty / 3600.0);
                (addr.clone(), score)
            })
            .collect();

        prices_by_score.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (addr, _) in prices_by_score.into_iter().take(evict_count) {
            self.prices.remove(&addr);
        }
    }

    async fn evict_old_pools(&mut self) {
        let evict_count = self.max_cache_size / 5;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut pools_by_score: Vec<(String, f64)> = self.pools
            .iter()
            .map(|(addr, cached)| {
                let age_penalty = (now - cached.cached_at) as f64;
                let access_bonus = cached.access_count as f64;
                let liquidity_bonus = cached.pool_info.liquidity_usd / 1000000.0; // Bonus for high liquidity pools
                let score = (access_bonus + liquidity_bonus) / (1.0 + age_penalty / 3600.0);
                (addr.clone(), score)
            })
            .collect();

        pools_by_score.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (addr, _) in pools_by_score.into_iter().take(evict_count) {
            self.pools.remove(&addr);
        }
    }

    pub async fn preload_priority_tokens(&mut self, token_addresses: &[String]) {
        // Mark these tokens as high priority (would need special handling)
        for address in token_addresses {
            if let Some(cached) = self.tokens.get_mut(address) {
                cached.access_count += 100; // Boost priority
            }
        }
    }

    pub async fn get_most_accessed_tokens(&self, limit: usize) -> Vec<String> {
        let mut tokens_by_access: Vec<(String, u64)> = self.tokens
            .iter()
            .map(|(addr, cached)| (addr.clone(), cached.access_count))
            .collect();

        tokens_by_access.sort_by(|a, b| b.1.cmp(&a.1));

        tokens_by_access
            .into_iter()
            .take(limit)
            .map(|(addr, _)| addr)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_tokens: usize,
    pub valid_tokens: usize,
    pub total_prices: usize,
    pub valid_prices: usize,
    pub total_pools: usize,
    pub valid_pools: usize,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
}

impl CacheStats {
    pub fn hit_rate_tokens(&self) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            (self.valid_tokens as f64) / (self.total_tokens as f64)
        }
    }

    pub fn hit_rate_prices(&self) -> f64 {
        if self.total_prices == 0 {
            0.0
        } else {
            (self.valid_prices as f64) / (self.total_prices as f64)
        }
    }

    pub fn hit_rate_pools(&self) -> f64 {
        if self.total_pools == 0 {
            0.0
        } else {
            (self.valid_pools as f64) / (self.total_pools as f64)
        }
    }
}
