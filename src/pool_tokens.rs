use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_tokens_enabled;
use crate::tokens::cache::TokenDatabase;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Token list update interval in seconds
const TOKEN_LIST_UPDATE_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Minimum liquidity threshold for tokens (in USD)
const MIN_LIQUIDITY_THRESHOLD_USD: f64 = 1000.0; // $1,000 minimum liquidity

/// Maximum number of tokens to track
const MAX_TRACKED_TOKENS: usize = 10000;

/// Token cache TTL in seconds
const TOKEN_CACHE_TTL_SECS: i64 = 300; // 5 minutes

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Token information for pool service
#[derive(Debug, Clone)]
pub struct PoolTokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub liquidity_usd: f64,
    pub volume_24h: Option<f64>,
    pub price_usd: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub is_active: bool,
}

/// Pool tokens service statistics
#[derive(Debug, Clone)]
pub struct PoolTokensStats {
    pub total_tokens_loaded: u64,
    pub active_tokens: u64,
    pub tokens_with_liquidity: u64,
    pub last_database_query: Option<DateTime<Utc>>,
    pub last_cache_update: Option<DateTime<Utc>>,
    pub average_liquidity_usd: f64,
    pub database_query_time_ms: f64,
}

impl Default for PoolTokensStats {
    fn default() -> Self {
        Self {
            total_tokens_loaded: 0,
            active_tokens: 0,
            tokens_with_liquidity: 0,
            last_database_query: None,
            last_cache_update: None,
            average_liquidity_usd: 0.0,
            database_query_time_ms: 0.0,
        }
    }
}

/// Pool tokens service
pub struct PoolTokensService {
    /// Token cache: mint -> PoolTokenInfo
    token_cache: Arc<RwLock<HashMap<String, PoolTokenInfo>>>,
    /// Statistics
    stats: Arc<RwLock<PoolTokensStats>>,
    /// Database instance
    database: Arc<TokenDatabase>,
    /// Debug mode
    debug_enabled: bool,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolTokensService {
    /// Create new pool tokens service
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let debug_enabled = is_debug_pool_tokens_enabled();
        let database = Arc::new(TokenDatabase::new()?);

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool tokens service debug mode enabled");
        }

        Ok(Self {
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolTokensStats::default())),
            database,
            debug_enabled,
        })
    }

    /// Load tokens from database and update cache
    pub async fn load_tokens_from_database(&self) -> Result<usize, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(LogTag::Pool, "TOKEN_LOAD_START", "🔄 Loading tokens from database...");
        }

        // Load tokens from database
        let tokens = match self.database.get_all_tokens().await {
            Ok(tokens) => tokens,
            Err(e) => {
                log(LogTag::Pool, "TOKEN_LOAD_ERROR", &format!("Failed to load tokens from database: {}", e));
                return Err(e);
            }
        };

        let query_time = start_time.elapsed().as_millis() as f64;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TOKEN_LOAD_QUERY",
                &format!("Loaded {} tokens from database in {:.1}ms", tokens.len(), query_time)
            );
        }

        // Filter and process tokens
        let mut processed_tokens = HashMap::new();
        let mut total_liquidity = 0.0;
        let mut tokens_with_liquidity = 0;

        for token in tokens {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            // Filter tokens by liquidity threshold
            if liquidity_usd >= MIN_LIQUIDITY_THRESHOLD_USD {
                let pool_token = PoolTokenInfo {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: token.name.clone(),
                    liquidity_usd,
                    volume_24h: token.volume.as_ref().and_then(|v| v.h24),
                    price_usd: Some(token.price_usd),
                    last_updated: token.last_updated,
                    is_active: true,
                };

                processed_tokens.insert(token.mint, pool_token);
                total_liquidity += liquidity_usd;
                tokens_with_liquidity += 1;
            }
        }

        // Limit to maximum number of tokens (sort by liquidity)
        if processed_tokens.len() > MAX_TRACKED_TOKENS {
            let mut token_vec: Vec<(String, PoolTokenInfo)> = processed_tokens.into_iter().collect();
            token_vec.sort_by(|a, b| b.1.liquidity_usd.partial_cmp(&a.1.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));
            
            processed_tokens = token_vec
                .into_iter()
                .take(MAX_TRACKED_TOKENS)
                .collect();

            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "TOKEN_LIMIT_APPLIED",
                    &format!("Limited to top {} tokens by liquidity", MAX_TRACKED_TOKENS)
                );
            }
        }

        // Update cache
        {
            let mut cache = self.token_cache.write().await;
            *cache = processed_tokens.clone();
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_tokens_loaded += processed_tokens.len() as u64;
            stats.active_tokens = processed_tokens.len() as u64;
            stats.tokens_with_liquidity = tokens_with_liquidity as u64;
            stats.last_database_query = Some(Utc::now());
            stats.last_cache_update = Some(Utc::now());
            stats.database_query_time_ms = query_time;
            stats.average_liquidity_usd = if tokens_with_liquidity > 0 {
                total_liquidity / tokens_with_liquidity as f64
            } else {
                0.0
            };
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TOKEN_LOAD_COMPLETE",
                &format!(
                    "✅ Token loading completed: {} active tokens ({} with liquidity), avg liquidity: ${:.0}",
                    processed_tokens.len(),
                    tokens_with_liquidity,
                    total_liquidity / tokens_with_liquidity.max(1) as f64
                )
            );
        }

        Ok(processed_tokens.len())
    }

    /// Get all tracked tokens (mint addresses)
    pub async fn get_tracked_tokens(&self) -> Vec<String> {
        let cache = self.token_cache.read().await;
        cache.keys().cloned().collect()
    }

    /// Get token information by mint
    pub async fn get_token_info(&self, mint: &str) -> Option<PoolTokenInfo> {
        let cache = self.token_cache.read().await;
        cache.get(mint).cloned()
    }

    /// Get all token information
    pub async fn get_all_token_info(&self) -> HashMap<String, PoolTokenInfo> {
        let cache = self.token_cache.read().await;
        cache.clone()
    }

    /// Update tracked tokens in shared state
    pub async fn update_tracked_tokens_in_state(
        &self,
        tracked_tokens: &mut HashMap<String, DateTime<Utc>>
    ) -> Result<usize, String> {
        let token_info = self.get_all_token_info().await;
        let now = Utc::now();
        let mut updated_count = 0;

        // Clear existing tracked tokens
        tracked_tokens.clear();

        // Add all active tokens to tracked tokens
        for (mint, token_info) in &token_info {
            if token_info.is_active {
                tracked_tokens.insert(mint.clone(), now);
                updated_count += 1;
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TRACKED_TOKENS_UPDATE",
                &format!("Updated tracked tokens: {} tokens", updated_count)
            );
        }

        Ok(updated_count)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolTokensStats {
        self.stats.read().await.clone()
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        {
            let mut cache = self.token_cache.write().await;
            cache.clear();
        }

        log(LogTag::Pool, "CACHE_CLEAR", "Cleared pool tokens cache");
    }

    /// Check if cache is stale and needs refresh
    pub async fn is_cache_stale(&self) -> bool {
        let stats = self.stats.read().await;
        if let Some(last_update) = stats.last_cache_update {
            let age = Utc::now().signed_duration_since(last_update);
            age.num_seconds() > TOKEN_CACHE_TTL_SECS
        } else {
            true // No cache, consider it stale
        }
    }

    /// Get tokens that need updating based on time criteria
    pub async fn get_tokens_needing_update(&self, min_hours_since_update: i64) -> Result<Vec<String>, String> {
        match self.database.get_tokens_needing_update(min_hours_since_update).await {
            Ok(tokens) => {
                let mints: Vec<String> = tokens.into_iter().map(|(mint, _, _, _)| mint).collect();
                Ok(mints)
            }
            Err(e) => Err(format!("Failed to get tokens needing update: {}", e))
        }
    }

    /// Refresh token data for specific tokens
    pub async fn refresh_tokens(&self, token_mints: &[String]) -> Result<usize, String> {
        if token_mints.is_empty() {
            return Ok(0);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TOKEN_REFRESH_START",
                &format!("Refreshing {} tokens", token_mints.len())
            );
        }

        // Get tokens from database
        let tokens = match self.database.get_tokens_by_mints(token_mints).await {
            Ok(tokens) => tokens,
            Err(e) => {
                log(LogTag::Pool, "TOKEN_REFRESH_ERROR", &format!("Failed to get tokens: {}", e));
                return Err(e.to_string());
            }
        };

        let mut updated_count = 0;
        let mut cache = self.token_cache.write().await;

        for token in tokens {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            let pool_token = PoolTokenInfo {
                mint: token.mint.clone(),
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                liquidity_usd,
                volume_24h: token.volume.as_ref().and_then(|v| v.h24),
                price_usd: Some(token.price_usd),
                last_updated: token.last_updated,
                is_active: liquidity_usd >= MIN_LIQUIDITY_THRESHOLD_USD,
            };

            cache.insert(token.mint, pool_token);
            updated_count += 1;
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "TOKEN_REFRESH_COMPLETE",
                &format!("Refreshed {} tokens", updated_count)
            );
        }

        Ok(updated_count)
    }
}

// =============================================================================
// GLOBAL INSTANCE MANAGEMENT
// =============================================================================

static GLOBAL_POOL_TOKENS: std::sync::OnceLock<PoolTokensService> = std::sync::OnceLock::new();

/// Initialize the global pool tokens service
pub fn init_pool_tokens() -> Result<&'static PoolTokensService, Box<dyn std::error::Error>> {
    GLOBAL_POOL_TOKENS.get_or_init(|| {
        log(LogTag::Pool, "INIT", "Initializing global pool tokens service");
        PoolTokensService::new().expect("Failed to create pool tokens service")
    });
    Ok(GLOBAL_POOL_TOKENS.get().unwrap())
}

/// Get the global pool tokens service
pub fn get_pool_tokens() -> &'static PoolTokensService {
    GLOBAL_POOL_TOKENS.get().expect("Pool tokens service not initialized")
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Load tokens from database (convenience function)
pub async fn load_tokens_from_database() -> Result<usize, String> {
    get_pool_tokens().load_tokens_from_database().await
}

/// Get tracked tokens (convenience function)
pub async fn get_tracked_tokens() -> Vec<String> {
    get_pool_tokens().get_tracked_tokens().await
}

/// Update tracked tokens in state (convenience function)
pub async fn update_tracked_tokens_in_state(
    tracked_tokens: &mut HashMap<String, DateTime<Utc>>
) -> Result<usize, String> {
    get_pool_tokens().update_tracked_tokens_in_state(tracked_tokens).await
}

/// Get pool tokens statistics (convenience function)
pub async fn get_pool_tokens_stats() -> PoolTokensStats {
    get_pool_tokens().get_stats().await
}

/// Check if cache is stale (convenience function)
pub async fn is_tokens_cache_stale() -> bool {
    get_pool_tokens().is_cache_stale().await
}

/// Refresh specific tokens (convenience function)
pub async fn refresh_tokens(token_mints: &[String]) -> Result<usize, String> {
    get_pool_tokens().refresh_tokens(token_mints).await
}
