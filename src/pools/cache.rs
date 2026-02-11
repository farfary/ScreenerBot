/// Price cache and history management
///
/// This module provides thread-safe caching for pool prices and maintains
/// price history for tokens. It uses efficient concurrent data structures
/// to minimize lock contention on the hot path.
use super::db;
use super::types::{price_cache_ttl_seconds, PriceHistory, PriceResult, PRICE_HISTORY_MAX_ENTRIES};

use crate::logger::{self, LogTag};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::Notify;

/// Global price cache - high-performance concurrent hashmap
static PRICE_CACHE: Lazy<DashMap<String, PriceResult>> = Lazy::new(DashMap::new);

/// Global price history - DashMap is already thread-safe, no additional RwLock needed
static PRICE_HISTORY: Lazy<DashMap<String, PriceHistory>> = Lazy::new(DashMap::new);

/// Global shutdown handle for cleanup task
static CLEANUP_SHUTDOWN: Lazy<Arc<Notify>> = Lazy::new(|| Arc::new(Notify::new()));

/// Initialize the cache system
pub async fn initialize_cache() {
    logger::debug(LogTag::PoolCache, "Initializing price cache system");

    // Load historical data from database into cache
    load_historical_data_into_cache().await;

    // Start cleanup task with shutdown support
    start_cache_cleanup_task().await;

    logger::debug(LogTag::PoolCache, "Price cache system initialized");
}

/// Get current price for a token
pub fn get_price(mint: &str) -> Option<PriceResult> {
    PRICE_CACHE.get(mint).map(|entry| entry.clone())
}

/// Update price for a token
pub fn update_price(price: PriceResult) {
    let mint = price.mint.clone();

    // Update cache first - this is intentionally separate from history update below.
    // Race condition: PRICE_CACHE and PRICE_HISTORY can briefly be out of sync, but this is
    // acceptable because cache is for latest-price queries while history is for trends.
    PRICE_CACHE.insert(mint.clone(), price.clone());

    // Queue for database storage (async, non-blocking)
    let price_for_db = price.clone();
    tokio::spawn(async move {
        if let Err(e) = db::queue_price_for_storage(price_for_db).await {
            logger::error(
                LogTag::PoolCache,
                &format!("Failed to queue price for storage: {}", e),
            );
        }
    });

    // Update history with gap detection - DashMap is thread-safe
    // Safety: get_mut() holds a per-shard lock for this key's entry, ensuring atomicity
    // of the cleanup + add_price sequence. No other thread can modify this entry concurrently.
    if let Some(mut history) = PRICE_HISTORY.get_mut(&mint) {
        let removed_count = history.cleanup_gapped_data();
        if removed_count > 0 {
            logger::info(
                LogTag::PoolCache,
                &format!(
                    "Removed {} gapped entries from memory for token: {}",
                    removed_count, mint
                ),
            );
        }

        history.add_price(price);

        logger::debug(
            LogTag::PoolCache,
            &format!("Updated price for token: {}", mint),
        );

        // Trigger database gap cleanup if gaps were detected in memory
        if removed_count > 0 {
            let mint_for_cleanup = mint.clone();
            tokio::spawn(async move {
                if let Err(e) = db::cleanup_gapped_data_for_token(&mint_for_cleanup).await {
                    logger::error(
                        LogTag::PoolCache,
                        &format!(
                            "Failed to cleanup gapped data in database for {}: {}",
                            mint_for_cleanup, e
                        ),
                    );
                }
            });
        }

        return;
    }

    // Create new history entry if it doesn't exist
    let mut new_history = PriceHistory::new(mint.clone(), PRICE_HISTORY_MAX_ENTRIES);
    new_history.add_price(price);
    PRICE_HISTORY.insert(mint.clone(), new_history);

    logger::debug(
        LogTag::PoolCache,
        &format!("Created new price history for token: {}", mint),
    );
}

/// Get available tokens (tokens with fresh prices)
pub fn get_available_tokens() -> Vec<String> {
    let now = Instant::now();
    let ttl = price_cache_ttl_seconds();
    PRICE_CACHE
        .iter()
        .filter_map(|entry| {
            let price = entry.value();
            if now.duration_since(price.timestamp).as_secs() < ttl {
                Some(price.mint.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Get price history for a token
pub fn get_price_history(mint: &str) -> Vec<PriceResult> {
    if let Some(history) = PRICE_HISTORY.get(mint) {
        return history.to_vec();
    }
    Vec::new()
}

/// Get cache statistics
pub fn get_cache_stats() -> CacheStats {
    let price_count = PRICE_CACHE.len();
    let fresh_count = get_available_tokens().len();
    let history_count = PRICE_HISTORY.len();

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

/// Start background cache cleanup task with shutdown support
async fn start_cache_cleanup_task() {
    let shutdown = CLEANUP_SHUTDOWN.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::debug(LogTag::PoolCache, "Cache cleanup task shutting down");
                    break;
                }
                _ = interval.tick() => {
                    cleanup_stale_entries();
                }
            }
        }
    });
}

/// Signal the cache cleanup task to shut down
pub fn shutdown_cache() {
    CLEANUP_SHUTDOWN.notify_waiters();
}

/// Remove stale price entries from cache
fn cleanup_stale_entries() {
    let now = Instant::now();
    let ttl = price_cache_ttl_seconds();
    let mut removed_count = 0;

    // Clean price cache
    PRICE_CACHE.retain(|_key, price| {
        let is_fresh = now.duration_since(price.timestamp).as_secs() < ttl * 2;
        if !is_fresh {
            removed_count += 1;
        }
        is_fresh
    });

    if removed_count > 0 {
        logger::debug(
            LogTag::PoolCache,
            &format!("Cleaned {} stale price entries", removed_count),
        );
    }
}

/// Load historical data from database into in-memory cache
async fn load_historical_data_into_cache() {
    logger::info(
        LogTag::PoolCache,
        "Loading historical data from database into cache for open positions",
    );

    // Get mints from open positions - these need their price history loaded
    let open_mints = crate::positions::get_open_mints().await;

    if open_mints.is_empty() {
        logger::debug(
            LogTag::PoolCache,
            "No open positions found - skipping historical data load",
        );
        return;
    }

    logger::info(
        LogTag::PoolCache,
        &format!(
            "Loading historical price data for {} tokens with open positions",
            open_mints.len()
        ),
    );

    let mut loaded_count = 0;
    let mut failed_count = 0;

    for mint in &open_mints {
        match db::load_historical_data_for_token(mint).await {
            Ok(historical_prices) => {
                if !historical_prices.is_empty() {
                    // Create history entry - DashMap is thread-safe
                    let mut new_history =
                        PriceHistory::new(mint.clone(), PRICE_HISTORY_MAX_ENTRIES);
                    let prices_count = historical_prices.len();

                    // Add all historical prices and cache the latest
                    let mut latest_price = None;
                    for price in historical_prices {
                        new_history.add_price(price.clone());
                        latest_price = Some(price); // Track the last one
                    }

                    // Insert the latest price into cache
                    if let Some(price) = latest_price {
                        PRICE_CACHE.insert(mint.clone(), price);
                    }

                    PRICE_HISTORY.insert(mint.clone(), new_history);
                    loaded_count += 1;

                    logger::debug(
                        LogTag::PoolCache,
                        &format!(
                            "Loaded {} historical prices for open position token: {}",
                            prices_count, mint
                        ),
                    );
                }
            }
            Err(e) => {
                failed_count += 1;
                logger::warning(
                    LogTag::PoolCache,
                    &format!(
                        "Failed to load historical data for open position token {}: {}",
                        mint, e
                    ),
                );
            }
        }
    }

    logger::info(
        LogTag::PoolCache,
        &format!(
            "Historical data loading completed: {} tokens loaded, {} failed",
            loaded_count, failed_count
        ),
    );
}

/// Load historical data for a specific token from database
pub async fn load_token_history_from_database(mint: &str) -> Result<(), String> {
    match db::load_historical_data_for_token(mint).await {
        Ok(historical_prices) => {
            if !historical_prices.is_empty() {
                // Create or update history entry - DashMap is thread-safe
                let mut new_history =
                    PriceHistory::new(mint.to_string(), PRICE_HISTORY_MAX_ENTRIES);
                let prices_count = historical_prices.len();

                // Add all historical prices
                for price in historical_prices {
                    new_history.add_price(price);
                }

                PRICE_HISTORY.insert(mint.to_string(), new_history);

                logger::debug(
                    LogTag::PoolCache,
                    &format!(
                        "Loaded {} historical prices for token: {}",
                        prices_count, mint
                    ),
                );
            }
            Ok(())
        }
        Err(e) => {
            logger::warning(
                LogTag::PoolCache,
                &format!("Failed to load historical data for {}: {}", mint, e),
            );
            Err(e)
        }
    }
}

/// Cleanup gapped data from all tokens in memory
pub async fn cleanup_all_memory_gaps() -> (usize, usize) {
    let mut total_removed = 0;
    let mut tokens_cleaned = 0;

    // Collect all tokens first to avoid holding locks during iteration
    let tokens: Vec<String> = PRICE_HISTORY
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    for token in tokens {
        if let Some(mut history) = PRICE_HISTORY.get_mut(&token) {
            let removed = history.cleanup_gapped_data();
            if removed > 0 {
                total_removed += removed;
                tokens_cleaned += 1;
            }
        }
    }

    (total_removed, tokens_cleaned)
}
