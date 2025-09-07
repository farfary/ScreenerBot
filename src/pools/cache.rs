/// Price cache and history management
///
/// This module provides thread-safe caching for pool prices and maintains
/// price history for tokens. It uses efficient concurrent data structures
/// to minimize lock contention on the hot path.

use crate::global::is_debug_pool_service_enabled;
use crate::arguments::is_debug_pool_cache_enabled;
use crate::logger::{ log, LogTag };
use super::types::{ PriceResult, PriceHistory, PRICE_CACHE_TTL_SECONDS, PRICE_HISTORY_MAX_ENTRIES };
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

    // Update history
    if let Ok(history_map) = PRICE_HISTORY.read() {
        if let Some(mut history) = history_map.get_mut(&mint) {
            history.add_price(price);
            if is_debug_pool_cache_enabled() {
                log(LogTag::PoolCache, "DEBUG", &format!("Updated price for token: {}", mint));
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
        log(
            LogTag::PoolCache,
            "DEBUG",
            &format!("Cleaned {} stale price entries", removed_count)
        );
    }
}
