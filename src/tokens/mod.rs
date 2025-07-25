/// Centralized Token Management System
///
/// This module handles all token operations including:
/// - Token discovery from multiple sources (DexScreener, RugCheck, etc.)
/// - Periodic price monitoring with rate limiting
/// - SQLite database caching for persistent storage
/// - Blacklist management for problematic tokens
/// - Comprehensive token data types and structures
/// - Rate limiting and API management

pub mod api;
pub mod pool;
pub mod discovery;
pub mod monitor;
pub mod cache;
pub mod types;
pub mod blacklist;
pub mod tests;
pub mod compatibility;

// Re-export main types and functions
pub use types::*;
pub use api::{ DexScreenerApi, get_token_prices_from_api };
pub use discovery::{ TokenDiscovery, start_token_discovery, discover_tokens_once };
pub use monitor::{
    TokenMonitor,
    start_token_monitoring,
    monitor_tokens_once,
    get_monitoring_stats,
};
pub use cache::{ PriceCache, PriceCacheStats, TokenDatabase, DatabaseStats };
pub use blacklist::{
    TokenBlacklist,
    is_token_blacklisted,
    check_and_track_liquidity,
    get_blacklist_stats,
};

// Re-export decimal caching functions
pub use cache::{ get_token_decimals_cached, fetch_or_cache_decimals };
pub use tests::{
    run_token_system_tests,
    test_discovery_manual,
    test_monitoring_manual,
    test_tokens_integration,
};

// Pool pricing is disabled - use pool module only if explicitly needed
#[allow(unused_imports)]
pub use pool::{ PoolPriceCalculator, get_token_price_from_pools };

use crate::logger::{ log, LogTag };
use std::sync::{ Arc, Mutex };
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::Notify;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Enable/disable pool price calculations globally (DISABLED)
pub const ENABLE_POOL_PRICES: bool = false;

/// Primary price source configuration
pub const USE_DEXSCREENER_PRIMARY: bool = true;

/// Rate limits for DexScreener API
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: usize = 300; // 300 requests per minute for tokens
pub const DEXSCREENER_DISCOVERY_RATE_LIMIT: usize = 60; // 60 requests per minute for discovery

/// Batch size for API calls
pub const MAX_TOKENS_PER_BATCH: usize = 30; // DexScreener supports up to 30 tokens per call

/// Price validation thresholds
pub const MAX_PRICE_DEVIATION_PERCENT: f64 = 50.0; // Maximum deviation between sources

// =============================================================================
// TOKENS SYSTEM MANAGER
// =============================================================================

/// Complete tokens system manager
pub struct TokensSystem {
    discovery: TokenDiscovery,
    monitor: TokenMonitor,
    database: TokenDatabase,
}

impl TokensSystem {
    /// Create new tokens system instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let discovery = TokenDiscovery::new()?;
        let monitor = TokenMonitor::new()?;
        let database = TokenDatabase::new()?;

        log(LogTag::System, "INIT", "Tokens system initialized");

        Ok(Self {
            discovery,
            monitor,
            database,
        })
    }

    /// Start all background tasks
    pub async fn start_background_tasks(
        &mut self,
        shutdown: Arc<Notify>
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
        let mut handles = Vec::new();

        // Start discovery task
        log(LogTag::System, "START", "Starting token discovery task...");
        let discovery_handle = start_token_discovery(shutdown.clone()).await?;
        handles.push(discovery_handle);

        // Start monitoring task
        log(LogTag::System, "START", "Starting token monitoring task...");
        let monitor_handle = start_token_monitoring(shutdown.clone()).await?;
        handles.push(monitor_handle);

        log(LogTag::System, "SUCCESS", "All tokens system background tasks started");

        Ok(handles)
    }

    /// Get system statistics
    pub async fn get_system_stats(&self) -> Result<TokensSystemStats, Box<dyn std::error::Error>> {
        let db_stats = self.database.get_stats()?;
        let monitor_stats = get_monitoring_stats().await?;
        let blacklist_stats = get_blacklist_stats();

        Ok(TokensSystemStats {
            total_tokens: db_stats.total_tokens,
            tokens_with_liquidity: db_stats.tokens_with_liquidity,
            active_tokens: monitor_stats.active_tokens,
            blacklisted_tokens: blacklist_stats.map(|s| s.total_blacklisted).unwrap_or(0),
            last_discovery_cycle: monitor_stats.last_cycle,
            last_monitoring_cycle: monitor_stats.last_cycle,
        })
    }
}

/// Tokens system statistics
#[derive(Debug, Clone)]
pub struct TokensSystemStats {
    pub total_tokens: usize,
    pub tokens_with_liquidity: usize,
    pub active_tokens: usize,
    pub blacklisted_tokens: usize,
    pub last_discovery_cycle: chrono::DateTime<chrono::Utc>,
    pub last_monitoring_cycle: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Initialize complete tokens system
pub async fn initialize_tokens_system() -> Result<TokensSystem, Box<dyn std::error::Error>> {
    log(LogTag::System, "INIT", "Initializing complete tokens system...");

    let system = TokensSystem::new()?;

    log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");

    Ok(system)
}

/// Get all tokens from database (synchronous version)
pub fn get_all_tokens_sync() -> Vec<Token> {
    if let Ok(db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *db_guard {
            // This needs to be handled differently since the database method is async
            // For now, return empty vector - this will need to be addressed later
            // TODO: Implement proper sync/async bridge or change database to support sync operations
            return Vec::new();
        }
    }
    Vec::new()
}

/// Get token decimals by mint address from database
pub async fn get_token_decimals(mint: &str) -> Option<u8> {
    // First try the cached version
    if let Some(decimals) = cache::get_token_decimals_cached(mint) {
        return Some(decimals);
    }

    // If not cached, try to get from token database
    if let Some(token) = get_token_from_db(mint).await {
        Some(token.decimals)
    } else {
        None
    }
}

/// Get token decimals synchronously (fallback to default 9 if not found)
pub fn get_token_decimals_or_default(mint: &str) -> u8 {
    cache::get_token_decimals_cached(mint).unwrap_or(9)
}

/// Get token from database by mint address
pub async fn get_token_from_db(mint: &str) -> Option<Token> {
    // Create a temporary database instance to avoid threading issues
    if let Ok(db) = TokenDatabase::new() {
        if let Ok(api_tokens) = db.get_all_tokens().await {
            for api_token in api_tokens {
                if api_token.mint == mint {
                    return Some(api_token.into());
                }
            }
        }
    }
    None
}

/// Initialize the global token database
pub fn initialize_token_database() -> Result<(), Box<dyn std::error::Error>> {
    let db = TokenDatabase::new()?;
    if let Ok(mut token_db) = TOKEN_DB.lock() {
        *token_db = Some(db);
        log(LogTag::System, "SUCCESS", "Token database initialized successfully");
    }
    Ok(())
}

/// Get current token price from cached data
pub async fn get_current_token_price(mint: &str) -> Option<f64> {
    // Create a temporary database instance to avoid threading issues
    if let Ok(db) = TokenDatabase::new() {
        if let Ok(Some(token)) = db.get_token_by_mint(mint) {
            // Return the most recent price available
            return Some(token.price_usd);
        }
    }
    None
}

/// Get token by mint address
pub async fn get_token_by_mint(mint: &str) -> Result<Option<ApiToken>, Box<dyn std::error::Error>> {
    // Create a temporary database instance to avoid threading issues
    let db = TokenDatabase::new()?;
    db.get_token_by_mint(mint)
}

/// Get all tokens sorted by liquidity
pub async fn get_all_tokens_by_liquidity() -> Result<Vec<ApiToken>, Box<dyn std::error::Error>> {
    // Create a temporary database instance to avoid threading issues
    let db = TokenDatabase::new()?;
    db.get_all_tokens().await
}

/// Thread-safe version for use in tokio::spawn contexts
/// Creates its own database connection to avoid Send/Sync issues
pub async fn get_all_tokens_by_liquidity_threadsafe() -> Result<
    Vec<ApiToken>,
    Box<dyn std::error::Error>
> {
    // Create a new database connection for this call to avoid threading issues
    let db = TokenDatabase::new()?;
    let tokens = db.get_all_tokens().await?;

    // Sort by liquidity descending
    let mut sorted_tokens = tokens;
    sorted_tokens.sort_by(|a, b| {
        let liquidity_a = a.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);
        let liquidity_b = b.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);
        liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(sorted_tokens)
}

// =============================================================================
// GLOBAL INSTANCES
// =============================================================================

/// Global token database instance
pub static TOKEN_DB: Lazy<Mutex<Option<TokenDatabase>>> = Lazy::new(|| Mutex::new(None));

/// Global price cache instance
pub static PRICE_CACHE: Lazy<Arc<Mutex<PriceCache>>> = Lazy::new(|| {
    Arc::new(Mutex::new(PriceCache::new()))
});

/// Global DexScreener API instance
pub static DEXSCREENER_API: Lazy<Arc<Mutex<DexScreenerApi>>> = Lazy::new(|| {
    Arc::new(Mutex::new(DexScreenerApi::new()))
});

/// Global pool price calculator (optional)
pub static POOL_CALCULATOR: Lazy<Arc<Mutex<Option<PoolPriceCalculator>>>> = Lazy::new(|| {
    if ENABLE_POOL_PRICES {
        Arc::new(Mutex::new(Some(PoolPriceCalculator::new())))
    } else {
        Arc::new(Mutex::new(None))
    }
});

// =============================================================================
// MAIN INTERFACE FUNCTIONS
// =============================================================================

/// Get current token price using primary API source with optional pool fallback
/// This is the main entry point for all price requests
pub async fn get_token_price(mint: &str) -> Option<f64> {
    log(LogTag::Trader, "PRICE", &format!("Getting price for token: {}", mint));

    // 1. Try primary API source (DexScreener)
    if USE_DEXSCREENER_PRIMARY {
        if let Some(api_price) = get_api_price(mint).await {
            log(
                LogTag::Trader,
                "PRICE",
                &format!("Got API price for {}: {:.12} SOL", mint, api_price)
            );
            return Some(api_price);
        }
    }

    // 2. Try pool price as fallback (if enabled)
    if ENABLE_POOL_PRICES {
        if let Some(pool_price) = get_pool_price(mint).await {
            log(
                LogTag::Trader,
                "PRICE",
                &format!("Got pool price for {}: {:.12} SOL", mint, pool_price)
            );
            return Some(pool_price);
        }
    }

    log(LogTag::Trader, "WARN", &format!("No price available for token: {}", mint));
    None
}

/// Get token price from API sources only
pub async fn get_api_price(mint: &str) -> Option<f64> {
    // Check cache first
    if let Ok(mut cache) = PRICE_CACHE.lock() {
        if let Some(cached_price) = cache.get_price(mint) {
            return Some(cached_price);
        }
    }

    // For now, call the API directly without caching to avoid mutex issues
    // TODO: Implement proper async-safe caching
    let mut temp_api = crate::tokens::api::DexScreenerApi::new();
    if let Some(price) = temp_api.get_token_price(mint).await {
        // Cache the result
        if let Ok(mut cache) = PRICE_CACHE.lock() {
            cache.set_price(mint, price);
        }
        return Some(price);
    }

    None
}

/// Get token price from pool calculations (if enabled)
pub async fn get_pool_price(mint: &str) -> Option<f64> {
    if !ENABLE_POOL_PRICES {
        return None;
    }

    if let Ok(calculator_guard) = POOL_CALCULATOR.lock() {
        if let Some(ref calculator) = *calculator_guard {
            if let Some(price) = calculator.get_token_price(mint).await {
                return Some(price);
            }
        }
    }

    None
}

/// Update token prices in bulk using API
pub async fn update_token_prices(mints: Vec<String>) -> HashMap<String, f64> {
    let mut prices = HashMap::new();

    if let Ok(mut api) = DEXSCREENER_API.lock() {
        let api_prices = api.get_multiple_token_prices(&mints).await;
        prices.extend(api_prices);
    }

    // Update cache
    if let Ok(mut cache) = PRICE_CACHE.lock() {
        for (mint, price) in &prices {
            cache.set_price(mint, *price);
        }
    }

    prices
}

/// Initialize the pricing system
pub async fn initialize_pricing_system() -> Result<(), String> {
    log(LogTag::System, "INFO", "Initializing centralized pricing system...");

    // Initialize DexScreener API
    if let Ok(mut api) = DEXSCREENER_API.lock() {
        api.initialize().await?;
    }

    // Initialize pool calculator if enabled
    if ENABLE_POOL_PRICES {
        if let Ok(mut calculator_guard) = POOL_CALCULATOR.lock() {
            if let Some(ref mut calculator) = *calculator_guard {
                let _ = calculator.initialize().await;
            }
        }
        log(LogTag::System, "INFO", "Pool price calculations enabled as fallback");
    } else {
        log(LogTag::System, "INFO", "Pool price calculations disabled - API only mode");
    }

    log(LogTag::System, "SUCCESS", "Pricing system initialized successfully");
    Ok(())
}

/// Start background price monitoring
pub async fn start_pricing_background_tasks(shutdown: Arc<Notify>) {
    log(LogTag::System, "INFO", "Starting pricing background tasks...");

    // Note: Background tasks are temporarily disabled due to Send/Sync issues with SQLite
    // This needs to be fixed by making the database thread-safe or using a different approach

    log(
        LogTag::System,
        "WARN",
        "Background tasks disabled - database needs Send/Sync implementation"
    );

    // Wait for shutdown
    shutdown.notified().await;

    log(LogTag::System, "INFO", "Pricing background tasks stopped");
}

/// Get pricing system statistics
pub fn get_pricing_stats() -> String {
    let mut stats = String::new();

    if let Ok(cache) = PRICE_CACHE.lock() {
        let cache_stats = cache.get_stats();
        stats.push_str(&format!("Cache: {}\n", cache_stats));
    }

    if let Ok(api) = DEXSCREENER_API.lock() {
        let api_stats = api.get_stats();
        stats.push_str(&format!("API: {}\n", api_stats));
    }

    if ENABLE_POOL_PRICES {
        if let Ok(calculator_guard) = POOL_CALCULATOR.lock() {
            if let Some(ref calculator) = *calculator_guard {
                let pool_stats = calculator.get_stats();
                stats.push_str(&format!("Pool: {:?}\n", pool_stats));
            }
        }
    }

    stats
}
