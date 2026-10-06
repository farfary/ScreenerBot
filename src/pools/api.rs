// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Public API for the pools module
//!
//! This module provides the clean public interface for the pools system.
//! Only these functions should be used by other modules - all internal
//! implementation details are hidden.

use super::cache;
use super::driver::PricingStatus;
use super::service;
use super::types::{CacheStats, PoolDescriptor, PriceResult};
use crate::chains::{ChainId, ChainScope};

/// Get the current pool price for a token on `chain`
///
/// Returns the most recent price calculation for the specified token.
/// The price includes both USD and SOL values along with confidence metrics.
///
/// # Arguments
/// * `chain` - The chain the token lives on
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `Some(PriceResult)` - Current price data if available and fresh
/// * `None` - No price available or price is stale
pub fn get_pool_price(chain: ChainId, mint: &str) -> Option<PriceResult> {
    if !service::is_pool_service_running() {
        return None;
    }

    // Only return a price that is still within the configured TTL. A stale cached
    // value means the token is no longer being actively priced on-chain, so it
    // must NOT be reported as a live pool price (that produced the "header shows
    // Price Pool but the Pool Service list omits the token" mismatch, and would
    // feed stale prices to trading/P&L). Matches `get_available_tokens`.
    cache::get_fresh_price(chain, mint)
}

/// Tokens on `chain` with available prices
///
/// Returns every token of that chain whose price is newer than the configured
/// TTL. There is no cross-chain list: an address without its chain is
/// ambiguous, so a caller covering several chains asks each one.
pub fn get_available_tokens(chain: ChainId) -> Vec<String> {
    if !service::is_pool_service_running() {
        return Vec::new();
    }

    cache::available_tokens(chain)
}

/// Cache statistics for monitoring, summed over the chains of `scope`
pub fn get_cache_stats(scope: ChainScope) -> CacheStats {
    scope.chains().into_iter().map(cache::stats).fold(
        CacheStats {
            total_prices: 0,
            fresh_prices: 0,
            history_entries: 0,
        },
        |sum, chain_stats| CacheStats {
            total_prices: sum.total_prices + chain_stats.total_prices,
            fresh_prices: sum.fresh_prices + chain_stats.fresh_prices,
            history_entries: sum.history_entries + chain_stats.history_entries,
        },
    )
}

/// The pools `chain`'s pricing driver knows for `mint`, its selected pricing
/// pool first. Empty while the pool service is not running.
pub fn token_pools(chain: ChainId, mint: &str) -> Vec<PoolDescriptor> {
    if !service::is_pool_service_running() {
        return Vec::new();
    }
    service::pricing_driver(chain)
        .map(|driver| driver.token_pools(mint))
        .unwrap_or_default()
}

/// The stable protocol slug of `pool`, a known pool of `mint` on `chain`.
pub fn pool_protocol(chain: ChainId, mint: &str, pool: &str) -> Option<&'static str> {
    if !service::is_pool_service_running() {
        return None;
    }
    service::pricing_driver(chain)
        .ok()?
        .pool_protocol(mint, pool)
}

/// The pricing pipelines' state, merged over the chains of `scope`.
pub fn pricing_status(scope: ChainScope) -> PricingStatus {
    scope
        .chains()
        .into_iter()
        .filter_map(|chain| service::pricing_driver(chain).ok())
        .map(|driver| driver.status())
        .fold(PricingStatus::default(), PricingStatus::merge)
}
