/// Centralized Token Management System - Thread-Safe Edition
///
/// This module provides thread-safe access to token data and prices
/// using a centralized price service instead of direct database access.

use crate::logger::{ log, LogTag };
use std::sync::Arc;
use tokio::sync::Notify;

pub mod api;
pub mod pool;
pub mod discovery;
pub mod monitor;
pub mod cache;
pub mod types;
pub mod blacklist;
pub mod price_service;
pub mod decimals;

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
pub use decimals::{
    get_token_decimals_from_chain,
    batch_fetch_token_decimals,
    get_cached_decimals,
};
pub use blacklist::{
    TokenBlacklist,
    is_token_blacklisted,
    check_and_track_liquidity,
    get_blacklist_stats,
};
pub use price_service::{
    initialize_price_service,
    get_token_price_safe,
    update_open_positions_safe,
    get_priority_tokens_safe,
    update_tokens_prices_safe,
    get_price_cache_stats,
    cleanup_price_service,
    TokenPriceService,
    PriceCacheEntry,
};

// Pool pricing is disabled - use pool module only if explicitly needed
#[allow(unused_imports)]
pub use pool::{ PoolPriceCalculator, get_token_price_from_pools };

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Enable/disable pool price calculations globally (DISABLED)
pub const ENABLE_POOL_PRICES: bool = false;

/// Primary price source configuration
pub const USE_DEXSCREENER_PRIMARY: bool = true;

/// Rate limits for DexScreener API
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: usize = 250; // 300 requests per minute for tokens
pub const DEXSCREENER_DISCOVERY_RATE_LIMIT: usize = 50; // 60 requests per minute for discovery

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
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
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
    pub async fn get_system_stats(&self) -> Result<TokensSystemStats, String> {
        let db_stats = self.database
            .get_stats()
            .map_err(|e| format!("Failed to get database stats: {}", e))?;
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
// INITIALIZATION FUNCTIONS
// =============================================================================

/// Initialize the tokens system with price service
pub async fn initialize_tokens_system() -> Result<TokensSystem, Box<dyn std::error::Error>> {
    log(LogTag::System, "INIT", "Initializing complete tokens system...");

    // Initialize price service first
    initialize_price_service().await?;

    // Create tokens system
    let system = TokensSystem::new()?;

    log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");
    Ok(system)
}

// =============================================================================
// SAFE TOKEN ACCESS FUNCTIONS
// =============================================================================

/// Get token decimals from blockchain with caching
pub async fn get_token_decimals(mint: &str) -> u8 {
    match get_token_decimals_from_chain(mint).await {
        Ok(decimals) => decimals,
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!("Failed to get decimals for {}: {}, using default (9)", mint, e)
            );
            9 // Fallback to default
        }
    }
}

/// Get token decimals synchronously (cache-only, no RPC calls)
/// Returns cached decimals if available, otherwise returns default (9)
pub fn get_token_decimals_sync(mint: &str) -> u8 {
    get_cached_decimals(mint).unwrap_or(9)
}

/// Get current token price using thread-safe price service
pub async fn get_current_token_price(mint: &str) -> Option<f64> {
    get_token_price_safe(mint).await
}

/// Get all tokens by liquidity using database directly (for compatibility)
pub async fn get_all_tokens_by_liquidity() -> Result<Vec<ApiToken>, String> {
    let db = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;

    match db.get_all_tokens().await {
        Ok(tokens) => {
            let mut sorted_tokens = tokens;
            sorted_tokens.sort_by(|a, b| {
                let a_liq = a.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                let b_liq = b.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
            });
            Ok(sorted_tokens)
        }
        Err(e) => Err(format!("Database error: {}", e)),
    }
}

// =============================================================================
// ENHANCED MONITORING WITH PRICE SERVICE
// =============================================================================

/// Start enhanced monitoring that prioritizes open positions and high liquidity tokens
pub async fn start_enhanced_monitoring(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::System, "START", "Starting enhanced token monitoring with 5-second price updates");

    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5)); // Every 5 seconds

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Enhanced monitoring stopping");
                    break;
                }
                
                _ = interval.tick() => {
                    if let Err(e) = enhanced_monitoring_cycle().await {
                        log(LogTag::System, "ERROR", 
                            &format!("Enhanced monitoring cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::System, "STOP", "Enhanced monitoring stopped");
    });

    Ok(handle)
}

/// Execute one enhanced monitoring cycle
async fn enhanced_monitoring_cycle() -> Result<(), String> {
    // Get priority tokens from price service
    let priority_mints = get_priority_tokens_safe().await;

    if priority_mints.is_empty() {
        log(LogTag::System, "MONITOR", "No priority tokens for monitoring");
        return Ok(());
    }

    log(
        LogTag::System,
        "MONITOR",
        &format!("Starting enhanced monitoring for {} priority tokens", priority_mints.len())
    );

    // Update database with fresh token data
    let db = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;
    let mut api = DexScreenerApi::new();

    // Process tokens in batches (smaller batch size for 5-second intervals)
    let batch_size = 10;
    let mut total_updated = 0;

    for chunk in priority_mints.chunks(batch_size) {
        match api.get_tokens_info(chunk).await {
            Ok(updated_tokens) => {
                // Update database
                if let Err(e) = db.update_tokens(&updated_tokens).await {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to update tokens in database: {}", e)
                    );
                } else {
                    // Update price service cache
                    let updated_count = update_tokens_prices_safe(chunk).await;
                    total_updated += updated_count;

                    log(
                        LogTag::Monitor,
                        "UPDATE",
                        &format!("Updated {} tokens in priority batch", updated_count)
                    );
                }
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("Failed to get token info for priority batch: {}", e)
                );
            }
        }

        // Rate limiting between batches (reduced for faster updates)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    log(
        LogTag::Monitor,
        "MONITOR",
        &format!("Enhanced monitoring cycle complete: {} tokens updated", total_updated)
    );

    Ok(())
}

// =============================================================================
// BACKGROUND TASK COORDINATION
// =============================================================================

/// Start all background pricing tasks
pub async fn start_pricing_background_tasks(
    shutdown: Arc<Notify>
) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
    log(LogTag::System, "INFO", "Starting enhanced pricing background tasks...");

    let mut handles = Vec::new();

    // Start enhanced monitoring
    let enhanced_monitor_handle = start_enhanced_monitoring(shutdown.clone()).await?;
    handles.push(enhanced_monitor_handle);

    // Start cache cleanup task
    let cleanup_handle = start_cache_cleanup_task(shutdown.clone()).await?;
    handles.push(cleanup_handle);

    log(LogTag::System, "SUCCESS", "Enhanced pricing background tasks started");
    Ok(handles)
}

/// Start cache cleanup background task
async fn start_cache_cleanup_task(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Every 5 minutes

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Cache cleanup stopping");
                    break;
                }
                
                _ = interval.tick() => {
                    let removed_count = price_service::cleanup_price_cache().await;
                    if removed_count > 0 {
                        log(LogTag::System, "CLEANUP", 
                            &format!("Cleaned up {} expired cache entries", removed_count));
                    }
                }
            }
        }
    });

    Ok(handle)
}

/// Get pricing system statistics
pub async fn get_pricing_stats() -> String {
    get_price_cache_stats().await
}

/// Get token from database using safe system (compatibility function)
pub async fn get_token_from_db(mint: &str) -> Option<Token> {
    let db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return None;
        }
    };

    match db.get_token_by_mint(mint) {
        Ok(Some(api_token)) => Some(api_token.into()),
        _ => None,
    }
}
