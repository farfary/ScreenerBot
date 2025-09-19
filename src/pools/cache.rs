/// Price cache and history management
///
/// This module provides thread-safe caching for pool prices and maintains
/// price history for tokens. It uses efficient concurrent data structures
/// to minimize lock contention on the hot path.

use crate::global::is_debug_pool_service_enabled;
use crate::arguments::is_debug_pool_cache_enabled;
use crate::logger::{ log, LogTag };
use super::types::{ PriceResult, PriceHistory, PRICE_CACHE_TTL_SECONDS, PRICE_HISTORY_MAX_ENTRIES };
use super::db; // Database module for persistence
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tokio::sync::Notify;

/// Global price cache - high-performance concurrent hashmap
static PRICE_CACHE: once_cell::sync::Lazy<
    DashMap<String, PriceResult>
> = once_cell::sync::Lazy::new(|| DashMap::new());

/// Global price history - protected by RwLock for batch operations
static PRICE_HISTORY: once_cell::sync::Lazy<
    RwLock<dashmap::DashMap<String, PriceHistory>>
> = once_cell::sync::Lazy::new(|| RwLock::new(DashMap::new()));

/// Initialize the cache system
pub async fn initialize_cache() {
    if is_debug_pool_cache_enabled() {
        log(LogTag::PoolCache, "DEBUG", "Initializing price cache system");
    }

    // Load historical data from database into cache
    load_historical_data_into_cache().await;

    // Start cleanup task
    start_cache_cleanup_task().await;

    if is_debug_pool_cache_enabled() {
        log(LogTag::PoolCache, "DEBUG", "Price cache system initialized");
    }
}

/// Get current price for a token
pub fn get_price(mint: &str) -> Option<PriceResult> {
    PRICE_CACHE.get(mint).map(|entry| entry.clone())
}

/// Update price for a token
pub fn update_price(price: PriceResult) {
    let mint = price.mint.clone();

    // Update cache
    PRICE_CACHE.insert(mint.clone(), price.clone());

    // Queue for database storage (async, non-blocking)
    let price_for_db = price.clone();
    tokio::spawn(async move {
        if let Err(e) = db::queue_price_for_storage(price_for_db).await {
            if is_debug_pool_cache_enabled() {
                log(
                    LogTag::PoolCache,
                    "ERROR",
                    &format!("Failed to queue price for storage: {}", e)
                );
            }
        }
    });

    // Update history with gap detection
    if let Ok(history_map) = PRICE_HISTORY.read() {
        if let Some(mut history) = history_map.get_mut(&mint) {
            let removed_count = history.cleanup_gapped_data();
            if removed_count > 0 && is_debug_pool_cache_enabled() {
                log(
                    LogTag::PoolCache,
                    "GAP_CLEANUP",
                    &format!(
                        "Removed {} gapped entries from memory for token: {}",
                        removed_count,
                        mint
                    )
                );
            }

            history.add_price(price);

            if is_debug_pool_cache_enabled() {
                log(LogTag::PoolCache, "DEBUG", &format!("Updated price for token: {}", mint));
            }

            // Trigger database gap cleanup if gaps were detected in memory
            if removed_count > 0 {
                let mint_for_cleanup = mint.clone();
                tokio::spawn(async move {
                    if let Err(e) = db::cleanup_gapped_data_for_token(&mint_for_cleanup).await {
                        log(
                            LogTag::PoolCache,
                            "ERROR",
                            &format!(
                                "Failed to cleanup gapped data in database for {}: {}",
                                mint_for_cleanup,
                                e
                            )
                        );
                    }
                });
            }

            return;
        }
    }

    // Create new history entry if it doesn't exist
    if let Ok(mut history_map) = PRICE_HISTORY.write() {
        let mut new_history = PriceHistory::new(mint.clone(), PRICE_HISTORY_MAX_ENTRIES);
        new_history.add_price(price);
        history_map.insert(mint.clone(), new_history);

        if is_debug_pool_cache_enabled() {
            log(
                LogTag::PoolCache,
                "DEBUG",
                &format!("Created new price history for token: {}", mint)
            );
        }
    }
}

/// Get available tokens (tokens with fresh prices)
pub fn get_available_tokens() -> Vec<String> {
    let now = Instant::now();
    PRICE_CACHE.iter()
        .filter_map(|entry| {
            let price = entry.value();
            if now.duration_since(price.timestamp).as_secs() < PRICE_CACHE_TTL_SECONDS {
                Some(price.mint.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Get price history for a token
pub fn get_price_history(mint: &str) -> Vec<PriceResult> {
    if let Ok(history_map) = PRICE_HISTORY.read() {
        if let Some(history) = history_map.get(mint) {
            return history.to_vec();
        }
    }
    Vec::new()
}

/// Get cache statistics
pub fn get_cache_stats() -> CacheStats {
    let price_count = PRICE_CACHE.len();
    let fresh_count = get_available_tokens().len();
    let history_count = if let Ok(history_map) = PRICE_HISTORY.read() {
        history_map.len()
    } else {
        0
    };

    CacheStats {
        total_prices: price_count,
        fresh_prices: fresh_count,
        history_entries: history_count,
    }
}

/// Cache statistics structure
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_prices: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}

/// Start background cache cleanup task
async fn start_cache_cleanup_task() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        loop {
            interval.tick().await;
            cleanup_stale_entries();
        }
    });
}

/// Remove stale price entries from cache
fn cleanup_stale_entries() {
    let now = Instant::now();
    let mut removed_count = 0;

    // Clean price cache
    PRICE_CACHE.retain(|_key, price| {
        let is_fresh = now.duration_since(price.timestamp).as_secs() < PRICE_CACHE_TTL_SECONDS * 2;
        if !is_fresh {
            removed_count += 1;
        }
        is_fresh
    });

    if removed_count > 0 && is_debug_pool_cache_enabled() {
        log(LogTag::PoolCache, "DEBUG", &format!("Cleaned {} stale price entries", removed_count));
    }
}

/// Load historical data from database into in-memory cache
async fn load_historical_data_into_cache() {
    if is_debug_pool_cache_enabled() {
        log(LogTag::PoolCache, "DEBUG", "Loading historical data from database into cache");
    }

    // Get list of all tokens that have available prices in cache (this will be empty on first run)
    // Instead, we'll load based on tokens that exist in the database
    // For now, we'll load on-demand when prices are requested

    if is_debug_pool_cache_enabled() {
        log(LogTag::PoolCache, "DEBUG", "Historical data loading setup completed");
    }
}

/// Load historical data for a specific token from database
pub async fn load_token_history_from_database(mint: &str) -> Result<(), String> {
    match db::load_historical_data_for_token(mint).await {
        Ok(historical_prices) => {
            if !historical_prices.is_empty() {
                // Create or update history entry
                if let Ok(mut history_map) = PRICE_HISTORY.write() {
                    let mut new_history = PriceHistory::new(
                        mint.to_string(),
                        PRICE_HISTORY_MAX_ENTRIES
                    );
                    let prices_count = historical_prices.len();

                    // Add all historical prices
                    for price in historical_prices {
                        new_history.add_price(price);
                    }

                    history_map.insert(mint.to_string(), new_history);

                    if is_debug_pool_cache_enabled() {
                        log(
                            LogTag::PoolCache,
                            "DEBUG",
                            &format!(
                                "Loaded {} historical prices for token: {}",
                                prices_count,
                                mint
                            )
                        );
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            if is_debug_pool_cache_enabled() {
                log(
                    LogTag::PoolCache,
                    "WARN",
                    &format!("Failed to load historical data for {}: {}", mint, e)
                );
            }
            Err(e)
        }
    }
}

/// Cleanup gapped data from all tokens in memory
pub async fn cleanup_all_memory_gaps() -> (usize, usize) {
    if let Ok(mut history_map) = PRICE_HISTORY.write() {
        let mut total_removed = 0;
        let mut tokens_cleaned = 0;

        // Collect all tokens to avoid holding the write lock during iteration
        let tokens: Vec<String> = history_map
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for token in tokens {
            if let Some(mut history) = history_map.get_mut(&token) {
                let removed = history.cleanup_gapped_data();
                if removed > 0 {
                    total_removed += removed;
                    tokens_cleaned += 1;
                }
            }
        }

        (total_removed, tokens_cleaned)
    } else {
        (0, 0)
    }
}
