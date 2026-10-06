// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Price cache and history management
//!
//! One latest-price map and one price-history map per chain. Every function
//! takes the chain its caller already holds; this module never selects one.
//! The maps are concurrent hashmaps, so the hot path takes only a per-shard
//! lock.

use super::db;
use super::types::{
    price_cache_ttl_seconds, CacheStats, PriceHistory, PriceResult, PRICE_HISTORY_MAX_ENTRIES,
    PRICE_HISTORY_RECORD_INTERVAL_SECS,
};

use crate::chains::{ChainId, PerChain};
use crate::logger::{self, LogTag};

use dashmap::DashMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Latest pool price per token, one map per chain.
static PRICES: PerChain<DashMap<String, PriceResult>> = PerChain::new(new_price_map);

/// Recorded price history per token, one map per chain.
static HISTORIES: PerChain<DashMap<String, PriceHistory>> = PerChain::new(new_history_map);

/// A token whose latest recorded price is older than this leaves the history
/// map unless an open position protects it.
const HISTORY_EVICTION_SECS: u64 = 7200;

fn new_price_map(_chain: ChainId) -> DashMap<String, PriceResult> {
    DashMap::new()
}

fn new_history_map(_chain: ChainId) -> DashMap<String, PriceHistory> {
    DashMap::new()
}

/// The token's cached price on `chain`, but only while it is still fresh
/// (within the configured TTL). Stale entries return `None` so callers never
/// act on an outdated pool price. This matches the TTL filter of
/// [`available_tokens`] (the Pool Service list), so a token that is no longer
/// being actively priced stops reporting a pool price instead of surfacing its
/// last, now-stale, cached value.
pub fn get_fresh_price(chain: ChainId, mint: &str) -> Option<PriceResult> {
    let ttl = price_cache_ttl_seconds();
    PRICES.get(chain).get(mint).and_then(|entry| {
        let price = entry.value();
        if price.timestamp.elapsed().as_secs() < ttl {
            Some(price.clone())
        } else {
            None
        }
    })
}

/// Age of the token's cached price on `chain` regardless of TTL; `None` when
/// no price is cached.
pub fn cached_price_age(chain: ChainId, mint: &str) -> Option<Duration> {
    PRICES
        .get(chain)
        .get(mint)
        .map(|entry| entry.value().timestamp.elapsed())
}

/// Whether a new price is recorded in history (memory and database) given the
/// age of the token's latest recorded entry. The cache always takes the new
/// price; history keeps at most one entry per `PRICE_HISTORY_RECORD_INTERVAL_SECS`.
fn should_record_history(last_recorded_age: Option<Duration>) -> bool {
    last_recorded_age
        .is_none_or(|age| age >= Duration::from_secs(PRICE_HISTORY_RECORD_INTERVAL_SECS))
}

/// Record a token's price on `chain`: the cache always takes it, and history
/// (memory and database) takes it at most once per record interval.
pub fn update_price(chain: ChainId, price: PriceResult) {
    let mint = price.mint.clone();
    let prices = PRICES.get(chain);
    let histories = HISTORIES.get(chain);

    // Update cache first - this is intentionally separate from history update below.
    // The cache and the history can briefly be out of sync, which is acceptable
    // because the cache serves latest-price queries while history serves trends.
    prices.insert(mint.clone(), price.clone());

    let last_recorded_age = histories.get(&mint).and_then(|history| {
        history
            .get_latest()
            .map(|latest| latest.timestamp.elapsed())
    });
    if !should_record_history(last_recorded_age) {
        return;
    }

    // Queue for database storage: a non-blocking channel send, so prices reach
    // the writer in the order they were recorded.
    if let Err(e) = db::queue_price_for_storage(chain, price.clone()) {
        logger::error(
            LogTag::PoolCache,
            &format!("Failed to queue price for storage on {chain}: {e}"),
        );
    }

    // Update history with gap detection.
    // Safety: get_mut() holds a per-shard lock for this key's entry, ensuring atomicity
    // of the cleanup + add_price sequence. No other thread can modify this entry concurrently.
    if let Some(mut history) = histories.get_mut(&mint) {
        let removed_count = history.cleanup_gapped_data();
        if removed_count > 0 {
            logger::info(
                LogTag::PoolCache,
                &format!(
                    "Removed {} gapped entries from memory for token {} on {}",
                    removed_count, mint, chain
                ),
            );
        }

        history.add_price(price);

        logger::debug(
            LogTag::PoolCache,
            &format!("Updated price for token: {mint}"),
        );

        // Trigger database gap cleanup if gaps were detected in memory
        if removed_count > 0 {
            let mint_for_cleanup = mint.clone();
            tokio::spawn(async move {
                if let Err(e) = db::cleanup_gapped_data_for_token(chain, &mint_for_cleanup).await {
                    logger::error(
                        LogTag::PoolCache,
                        &format!(
                            "Failed to cleanup gapped data in database for {} on {}: {}",
                            mint_for_cleanup, chain, e
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
    histories.insert(mint.clone(), new_history);

    logger::debug(
        LogTag::PoolCache,
        &format!("Created new price history for token: {mint}"),
    );
}

/// Tokens on `chain` whose cached price is still fresh.
pub fn available_tokens(chain: ChainId) -> Vec<String> {
    let now = Instant::now();
    let ttl = price_cache_ttl_seconds();
    PRICES
        .get(chain)
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

/// Cache statistics for one chain.
pub fn stats(chain: ChainId) -> CacheStats {
    CacheStats {
        total_prices: PRICES.get(chain).len(),
        fresh_prices: available_tokens(chain).len(),
        history_entries: HISTORIES.get(chain).len(),
    }
}

/// Drop `chain`'s cached prices older than twice the TTL, and its histories
/// whose latest price is older than [`HISTORY_EVICTION_SECS`] unless the token
/// is in `protected` (open positions). Bounds both maps' memory.
pub fn evict_stale(chain: ChainId, protected: &HashSet<String>) {
    evict_stale_as_of(chain, protected, Instant::now());
}

/// [`evict_stale`] judged against an explicit `now`, so tests can age entries
/// forward without backdating a timestamp past the monotonic clock's origin.
fn evict_stale_as_of(chain: ChainId, protected: &HashSet<String>, now: Instant) {
    let ttl = price_cache_ttl_seconds();
    let mut removed_count = 0;

    PRICES.get(chain).retain(|_key, price| {
        let is_fresh = now.duration_since(price.timestamp).as_secs() < ttl * 2;
        if !is_fresh {
            removed_count += 1;
        }
        is_fresh
    });

    if removed_count > 0 {
        logger::debug(
            LogTag::PoolCache,
            &format!("Cleaned {removed_count} stale price entries on {chain}"),
        );
    }

    evict_stale_histories(chain, protected, now);
}

/// Evict `chain`'s inactive histories that no open position protects.
fn evict_stale_histories(chain: ChainId, protected: &HashSet<String>, now: Instant) {
    let histories = HISTORIES.get(chain);

    // Collect tokens to evict: those whose latest price is older than threshold
    let mut candidates: Vec<String> = Vec::new();
    for entry in histories.iter() {
        let history = entry.value();
        if let Some(latest) = history.get_latest() {
            if now.duration_since(latest.timestamp).as_secs() > HISTORY_EVICTION_SECS {
                candidates.push(entry.key().clone());
            }
        } else {
            // Empty history — evict
            candidates.push(entry.key().clone());
        }
    }

    if candidates.is_empty() {
        return;
    }

    let mut evicted = 0;
    for mint in &candidates {
        if !protected.contains(mint) {
            histories.remove(mint);
            evicted += 1;
        }
    }

    if evicted > 0 {
        logger::info(
            LogTag::PoolCache,
            &format!(
                "Evicted {} stale tokens from price history on {} ({} candidates, {} protected by open positions, {} remaining)",
                evicted,
                chain,
                candidates.len(),
                candidates.len() - evicted,
                histories.len()
            ),
        );
    }
}

/// Load `chain`'s recorded price history for `mints` (the open positions'
/// tokens) from its database into memory, caching each token's latest price.
pub async fn load_history(chain: ChainId, mints: &[String]) {
    logger::info(
        LogTag::PoolCache,
        &format!("Loading historical data from database into cache for open positions on {chain}"),
    );

    if mints.is_empty() {
        logger::debug(
            LogTag::PoolCache,
            &format!("No open positions found on {chain} - skipping historical data load"),
        );
        return;
    }

    logger::info(
        LogTag::PoolCache,
        &format!(
            "Loading historical price data for {} tokens with open positions on {}",
            mints.len(),
            chain
        ),
    );

    let prices = PRICES.get(chain);
    let histories = HISTORIES.get(chain);
    let mut loaded_count = 0;
    let mut failed_count = 0;

    for mint in mints {
        match db::load_historical_data_for_token(chain, mint).await {
            Ok(historical_prices) => {
                if !historical_prices.is_empty() {
                    let mut new_history =
                        PriceHistory::new(mint.clone(), PRICE_HISTORY_MAX_ENTRIES);
                    let prices_count = historical_prices.len();

                    // Add all historical prices and cache the latest
                    let mut latest_price = None;
                    for price in historical_prices {
                        new_history.add_price(price.clone());
                        latest_price = Some(price);
                    }

                    if let Some(price) = latest_price {
                        prices.insert(mint.clone(), price);
                    }

                    histories.insert(mint.clone(), new_history);
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
            "Historical data loading completed on {}: {} tokens loaded, {} failed",
            chain, loaded_count, failed_count
        ),
    );
}

/// Remove gapped data from every in-memory history on `chain`. Returns
/// `(entries removed, tokens cleaned)`.
pub fn cleanup_memory_gaps(chain: ChainId) -> (usize, usize) {
    let histories = HISTORIES.get(chain);
    let mut total_removed = 0;
    let mut tokens_cleaned = 0;

    // Collect all tokens first to avoid holding locks during iteration
    let tokens: Vec<String> = histories.iter().map(|entry| entry.key().clone()).collect();

    for token in tokens {
        if let Some(mut history) = histories.get_mut(&token) {
            let removed = history.cleanup_gapped_data();
            if removed > 0 {
                total_removed += removed;
                tokens_cleaned += 1;
            }
        }
    }

    (total_removed, tokens_cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainScope;
    use std::sync::Mutex;

    /// Serializes the tests that write the shared per-chain maps, so one
    /// test's inserts never land between another's two whole-map reads.
    static SHARED_MAPS: Mutex<()> = Mutex::new(());

    fn lock_maps() -> std::sync::MutexGuard<'static, ()> {
        ensure_config();
        SHARED_MAPS.lock().unwrap_or_else(|e| e.into_inner())
    }

    const CHAIN: ChainId = ChainId::Solana;

    fn ensure_config() {
        use crate::config::schemas::Config;
        let _ =
            crate::config::utils::CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
    }

    fn price(mint: &str, age: Duration) -> PriceResult {
        PriceResult {
            mint: mint.to_owned(),
            price_usd: 0.0,
            price_native: 0.000_123_4,
            confidence: 0.9,
            source_pool: Some(format!("{mint}-source")),
            pool_address: format!("{mint}-pool"),
            slot: 42,
            timestamp: Instant::now() - age,
            native_reserves: 10.0,
            token_reserves: 1_000.0,
        }
    }

    fn history_len(mint: &str) -> usize {
        HISTORIES
            .get(CHAIN)
            .get(mint)
            .map(|history| history.prices.len())
            .unwrap_or(0)
    }

    #[test]
    fn history_records_the_first_price_and_then_at_most_once_per_interval() {
        let interval = Duration::from_secs(PRICE_HISTORY_RECORD_INTERVAL_SECS);
        assert!(should_record_history(None));
        assert!(!should_record_history(Some(Duration::from_secs(5))));
        assert!(!should_record_history(Some(
            interval - Duration::from_millis(1)
        )));
        assert!(should_record_history(Some(interval)));
    }

    #[test]
    fn a_recorded_price_reads_back_unchanged_from_its_chain() {
        let _maps = lock_maps();
        let recorded = price("cache-roundtrip-mint", Duration::ZERO);
        update_price(CHAIN, recorded.clone());

        let read = get_fresh_price(CHAIN, "cache-roundtrip-mint").expect("fresh price");
        assert_eq!(read.mint, recorded.mint);
        assert_eq!(read.price_native.to_bits(), recorded.price_native.to_bits());
        assert_eq!(read.price_usd.to_bits(), recorded.price_usd.to_bits());
        assert_eq!(read.confidence.to_bits(), recorded.confidence.to_bits());
        assert_eq!(read.source_pool, recorded.source_pool);
        assert_eq!(read.pool_address, recorded.pool_address);
        assert_eq!(read.slot, recorded.slot);
        assert_eq!(read.timestamp, recorded.timestamp);
        assert_eq!(
            read.native_reserves.to_bits(),
            recorded.native_reserves.to_bits()
        );
        assert_eq!(
            read.token_reserves.to_bits(),
            recorded.token_reserves.to_bits()
        );
        assert!(get_fresh_price(CHAIN, "cache-roundtrip-unknown-mint").is_none());
    }

    #[test]
    fn available_tokens_keep_fresh_prices_and_drop_stale_ones() {
        let _maps = lock_maps();
        let ttl = Duration::from_secs(price_cache_ttl_seconds());
        update_price(CHAIN, price("cache-ttl-fresh-mint", Duration::ZERO));
        update_price(
            CHAIN,
            price("cache-ttl-stale-mint", ttl + Duration::from_secs(1)),
        );

        let available = available_tokens(CHAIN);
        assert!(available.contains(&"cache-ttl-fresh-mint".to_owned()));
        assert!(!available.contains(&"cache-ttl-stale-mint".to_owned()));
        assert!(get_fresh_price(CHAIN, "cache-ttl-stale-mint").is_none());
        assert!(cached_price_age(CHAIN, "cache-ttl-stale-mint").is_some_and(|age| age > ttl));
    }

    #[test]
    fn stats_count_one_chain_and_the_scope_sums_its_chains() {
        let _maps = lock_maps();
        let before = stats(CHAIN);
        update_price(CHAIN, price("cache-stats-mint", Duration::ZERO));
        let after = stats(CHAIN);
        assert_eq!(after.total_prices, before.total_prices + 1);
        assert_eq!(after.fresh_prices, before.fresh_prices + 1);
        assert_eq!(after.history_entries, before.history_entries + 1);

        let scoped = super::super::api::get_cache_stats(ChainScope::All);
        let summed = ChainScope::All.chains().into_iter().map(stats).fold(
            (0, 0, 0),
            |(total, fresh, history), chain_stats| {
                (
                    total + chain_stats.total_prices,
                    fresh + chain_stats.fresh_prices,
                    history + chain_stats.history_entries,
                )
            },
        );
        assert_eq!(
            (
                scoped.total_prices,
                scoped.fresh_prices,
                scoped.history_entries
            ),
            summed
        );
    }

    #[test]
    fn eviction_keeps_a_protected_history_and_drops_an_unprotected_one() {
        let _maps = lock_maps();
        let later = Instant::now() + Duration::from_secs(HISTORY_EVICTION_SECS + 60);
        update_price(CHAIN, price("cache-evict-protected-mint", Duration::ZERO));
        update_price(CHAIN, price("cache-evict-unprotected-mint", Duration::ZERO));

        let protected: HashSet<String> = ["cache-evict-protected-mint".to_owned()].into();
        evict_stale_as_of(CHAIN, &protected, later);

        assert_eq!(history_len("cache-evict-protected-mint"), 1);
        assert_eq!(history_len("cache-evict-unprotected-mint"), 0);
        assert!(cached_price_age(CHAIN, "cache-evict-protected-mint").is_none());
    }

    #[test]
    fn history_takes_at_most_one_price_per_interval_while_the_cache_takes_every_one() {
        let _maps = lock_maps();
        update_price(CHAIN, price("cache-throttle-mint", Duration::ZERO));
        let mut second = price("cache-throttle-mint", Duration::ZERO);
        second.price_native = 0.000_200_0;
        update_price(CHAIN, second);

        assert_eq!(history_len("cache-throttle-mint"), 1);
        let latest = get_fresh_price(CHAIN, "cache-throttle-mint").expect("fresh price");
        assert_eq!(latest.price_native.to_bits(), 0.000_200_0_f64.to_bits());
    }
}
