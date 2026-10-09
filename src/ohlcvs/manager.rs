// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV manager — coordinates candle fetching, caching, and priority management.

use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig, PoolMetadata};
use crate::tokens::pools;
use crate::tokens::types::TokenPoolInfo;
use crate::tokens::{fetch_token_pools_immediate, get_token_pools_snapshot_allow_stale};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

/// Without a data server series pool, a native pool replaces the current series
/// pool only at this multiple of its liquidity. The data server applies the same
/// factor before it moves a stored series, so both sides switch on one rule.
const SERIES_POOL_SWITCH_FACTOR: f64 = 2.0;

pub struct PoolManager {
    db: Arc<OhlcvDatabase>,
    cache: Arc<OhlcvCache>,
}

impl PoolManager {
    pub fn new(db: Arc<OhlcvDatabase>, cache: Arc<OhlcvCache>) -> Self {
        Self { db, cache }
    }

    /// Register a pool for a token
    pub async fn register_pool(
        &self,
        mint: &str,
        pool_address: &str,
        dex: &str,
        liquidity: f64,
    ) -> OhlcvResult<()> {
        let pool = PoolConfig::new(pool_address.to_string(), dex.to_string(), liquidity);
        self.db.upsert_pool(mint, &pool)?;

        // INFO: Record pool registration
        record_ohlcv_event(
            "pool_registered",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "dex": dex,
                "liquidity": liquidity,
            }),
        )
        .await;

        Ok(())
    }

    /// Get all pools for a token
    pub async fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        self.db.get_pools(mint)
    }

    /// Get the default pool for a token
    pub async fn series_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;
        Ok(PoolConfig::series_pool(&pools).cloned())
    }

    /// Set a pool as default
    pub async fn set_default_pool(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let mut pools = self.db.get_pools(mint)?;

        for pool in &mut pools {
            pool.is_default = pool.address == pool_address;
            self.db.upsert_pool(mint, pool)?;
        }

        // INFO: Record default pool change
        record_ohlcv_event(
            "default_pool_changed",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
            }),
        )
        .await;

        Ok(())
    }

    /// Mark a pool as failed
    pub async fn mark_failure(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_failure(mint, pool_address)?;

        // WARN: Record pool failure
        record_ohlcv_event(
            "pool_failure",
            Severity::Warn,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
            }),
        )
        .await;

        // A default that just went unhealthy hands the default to the pool the series now
        // resolves to, so the choice survives a restart.
        let pools = self.db.get_pools(mint)?;
        let default_failed = pools
            .iter()
            .any(|p| p.is_default && p.address == pool_address && !p.is_healthy());
        if default_failed {
            if let Some(next) = PoolConfig::series_pool(&pools) {
                let next_address = next.address.clone();
                self.set_default_pool(mint, &next_address).await?;
            }
        }

        Ok(())
    }

    /// Mark a pool as successful
    pub async fn mark_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_success(mint, pool_address)
    }

    /// Discover and register pools for a token using centralized token snapshots.
    /// Uses immediate fetch (bypasses background queue) for fast response.
    ///
    /// The default (series) pool is the data server's canonical pool whenever the
    /// snapshot names one, whatever its quote. Otherwise the current default is
    /// kept unless a native pool holds `SERIES_POOL_SWITCH_FACTOR` times its
    /// liquidity. When the series pool changes, the other pools' rows and the
    /// token's backfill flags are reset together (`reset_series_to_pool`).
    pub async fn discover_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        record_ohlcv_event(
            "pool_discovery_start",
            Severity::Debug,
            Some(mint),
            None,
            json!({
                "mint": mint,
            }),
        )
        .await;

        // The snapshot is now the systematic server-first pool source: the token
        // pool_data fetch queries the data server as the PRIMARY source (see
        // tokens/pool_data/server.rs), so the wSOL pools it registers here already
        // include the server's centrally-resolved pools — no OHLCV-specific hook.
        let snapshot = match fetch_token_pools_immediate(self.db.chain(), mint).await {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                // Try stale fallback
                match get_token_pools_snapshot_allow_stale(self.db.chain(), mint).await {
                    Ok(Some(snapshot)) => snapshot,
                    Ok(None) | Err(_) => {
                        record_ohlcv_event(
                            "pool_discovery_error",
                            Severity::Error,
                            Some(mint),
                            None,
                            json!({
                                "mint": mint,
                                "error": "No pool snapshot available",
                            }),
                        )
                        .await;
                        return Err(OhlcvError::NotFound(format!(
                            "No pool snapshot available for mint {}",
                            mint
                        )));
                    }
                }
            }
            Err(err) => {
                record_ohlcv_event(
                    "pool_discovery_error",
                    Severity::Error,
                    Some(mint),
                    None,
                    json!({
                        "mint": mint,
                        "error": err.to_string(),
                    }),
                )
                .await;
                return Err(OhlcvError::ApiError(err.to_string()));
            }
        };

        let server_series = snapshot.series_pool_address.as_deref().filter(|address| {
            snapshot
                .pools
                .iter()
                .any(|pool| pool.pool_address == *address)
        });

        let existing_pools = self.db.get_pools(mint)?;
        let previous_series = PoolConfig::series_pool(&existing_pools)
            .or_else(|| existing_pools.iter().find(|p| p.is_default))
            .map(|p| p.address.clone());
        let previous_default = existing_pools
            .iter()
            .find(|p| p.is_default)
            .map(|p| p.address.clone());
        let mut existing_map: HashMap<String, PoolConfig> = existing_pools
            .into_iter()
            .map(|cfg| (cfg.address.clone(), cfg))
            .collect();

        let mut discovered_configs = Vec::new();
        let mut non_native_configs = Vec::new();

        for pool in snapshot.pools.iter() {
            let existing = existing_map.remove(&pool.pool_address);
            let config = Self::merge_pool_info(pool, existing);
            if pool.is_native_pair || server_series == Some(pool.pool_address.as_str()) {
                discovered_configs.push(config);
            } else {
                non_native_configs.push(config);
            }
        }

        // A token whose ONLY pools are USD-quoted (e.g. a pump token that only ever
        // paired with USDC) has no wSOL pool. The data server returns SOL-denominated
        // candles for any pool, so its USD pools are registered instead; the fetcher
        // serves a non-native pool from the data server only (is_native_pair=false),
        // never from a candle feed or GeckoTerminal, whose USD candles would poison
        // the SOL series.
        if discovered_configs.is_empty() {
            if non_native_configs.is_empty() {
                record_ohlcv_event(
                    "pool_discovery_empty",
                    Severity::Warn,
                    Some(mint),
                    None,
                    json!({ "mint": mint }),
                )
                .await;

                return Err(OhlcvError::NotFound(format!(
                    "No pools available for mint {}",
                    mint
                )));
            }

            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "No SOL pool for mint={}; registering {} USD pools, \
                     OHLCV from the data server only",
                    mint,
                    non_native_configs.len()
                ),
            );
            discovered_configs = non_native_configs;
        }

        let default_address = select_series_default(
            server_series,
            previous_default.as_deref(),
            &discovered_configs,
        );
        for config in &mut discovered_configs {
            config.is_default = default_address.as_deref() == Some(config.address.as_str());
        }

        for config in &discovered_configs {
            self.db.upsert_pool(mint, config)?;
        }

        let mut removed_addresses = Vec::new();
        for leftover in existing_map.into_values() {
            self.db.delete_pool(mint, &leftover.address)?;
            // Also drop the removed pool's candles so a stale pool's price series
            // can never resurface or be combined with the current pool's candles
            // (the chart/status must only ever reflect the single resolved pool).
            if let Ok(removed) = self.db.delete_candles_for_pool(mint, &leftover.address) {
                if removed > 0 {
                    logger::debug(
                        LogTag::Ohlcv,
                        &format!(
                            "Removed {} candles from dropped pool {} for mint={}",
                            removed, leftover.address, mint
                        ),
                    );
                }
            }
            removed_addresses.push(leftover.address);
        }

        if !removed_addresses.is_empty() {
            let preview: Vec<&str> = removed_addresses
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let suffix = if removed_addresses.len() > 3 {
                format!(" (+{} more)", removed_addresses.len() - 3)
            } else {
                String::new()
            };

            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "Removed {} stale pool entries for mint={}{}",
                    removed_addresses.len(),
                    mint,
                    if preview.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]{}", preview.join(", "), suffix)
                    }
                ),
            );
        }

        let series = PoolConfig::series_pool(&discovered_configs).map(|p| p.address.clone());
        if let (Some(previous), Some(current)) = (previous_series.as_deref(), series.as_deref()) {
            if previous != current {
                self.reset_series(mint, previous, current, server_series.is_some())
                    .await?;
            }
        }

        record_ohlcv_event(
            "pool_discovery_complete",
            Severity::Info,
            Some(mint),
            None,
            json!({
                "mint": mint,
                "pools_found": discovered_configs.len(),
                "sol_pool": discovered_configs.iter().any(|c| c.is_native_pair),
                "removed_pools": removed_addresses.len(),
                "series_pool": series,
                "server_series_pool": server_series,
            }),
        )
        .await;

        Ok(discovered_configs)
    }

    /// Move the token's series onto `current`: other pools' rows and the backfill
    /// flags in one transaction, then the hot candle cache, so no reader keeps
    /// serving the previous pool's series.
    async fn reset_series(
        &self,
        mint: &str,
        previous: &str,
        current: &str,
        from_server: bool,
    ) -> OhlcvResult<()> {
        let reset = self.db.reset_series_to_pool(mint, current)?;
        self.cache.invalidate(mint, None, None)?;

        logger::info(
            LogTag::Ohlcv,
            &format!(
                "Series pool for mint={} moved {} -> {} (data server pool: {}); removed {} candles and {} gaps, backfill restarts",
                mint, previous, current, from_server, reset.candles_deleted, reset.gaps_deleted
            ),
        );
        record_ohlcv_event(
            "default_pool_changed",
            Severity::Info,
            Some(mint),
            Some(current),
            json!({
                "mint": mint,
                "previous_pool": previous,
                "pool_address": current,
                "from_data_server": from_server,
                "candles_deleted": reset.candles_deleted,
                "gaps_deleted": reset.gaps_deleted,
            }),
        )
        .await;

        Ok(())
    }

    fn merge_pool_info(pool: &TokenPoolInfo, existing: Option<PoolConfig>) -> PoolConfig {
        let dex_label = pools::extract_dex_label(pool);
        let liquidity = pools::extract_pool_liquidity(pool);

        let was_existing = existing.is_some();
        let mut config = existing.unwrap_or_else(|| {
            PoolConfig::new(pool.pool_address.clone(), dex_label.clone(), liquidity)
        });

        config.address = pool.pool_address.clone();

        let incoming_dex_known = !dex_label.eq_ignore_ascii_case("unknown");
        if incoming_dex_known
            || config.dex.trim().is_empty()
            || config.dex.eq_ignore_ascii_case("unknown")
        {
            config.dex = dex_label;
        }

        if liquidity.is_finite() && liquidity > 0.0 {
            config.liquidity = liquidity;
        }

        // Carry the pool's SOL/USD denomination so the fetcher serves a
        // USD-quoted pool from the data server only.
        config.is_native_pair = pool.is_native_pair;

        // When re-discovering an existing pool, reset its failure_count so it
        // gets a fresh chance. The pool service just confirmed the pool exists
        // and is valid — stale failure counts from a previous session (or from
        // transient API errors) should not permanently block OHLCV fetching.
        // If the API is still failing, the count will build up again naturally.
        if was_existing && !config.is_healthy() {
            config.failure_count = 0;
        }

        config
    }

    /// Get pool metadata for API responses
    pub async fn get_pool_metadata(&self, mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
        let pools = self.get_pools(mint).await?;
        Ok(pools.iter().map(PoolMetadata::from).collect())
    }

    /// Health check all pools for a token
    pub async fn check_pool_health(&self, mint: &str) -> OhlcvResult<Vec<(String, bool)>> {
        let pools = self.get_pools(mint).await?;
        Ok(pools
            .into_iter()
            .map(|p| {
                let address = p.address.clone();
                (address, p.is_healthy())
            })
            .collect())
    }

    /// Reset failure count for a pool (for manual recovery)
    pub async fn reset_pool_failures(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_success(mint, pool_address)
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self, mint: &str) -> OhlcvResult<PoolStats> {
        let pools = self.get_pools(mint).await?;

        let total_pools = pools.len();
        let healthy_pools = pools.iter().filter(|p| p.is_healthy()).count();
        let total_liquidity: f64 = pools.iter().map(|p| p.liquidity).sum();
        let has_default = pools.iter().any(|p| p.is_default);

        Ok(PoolStats {
            total_pools,
            healthy_pools,
            total_liquidity,
            has_default,
        })
    }
}

/// The default (series) pool among the registered `candidates`: the data server's
/// pool when it is registered; else the current default, replaced only by a native
/// pool with `SERIES_POOL_SWITCH_FACTOR` times its liquidity; else the deepest
/// native pool, else the deepest pool.
fn select_series_default(
    server_series: Option<&str>,
    current_default: Option<&str>,
    candidates: &[PoolConfig],
) -> Option<String> {
    if let Some(server) = server_series {
        if candidates.iter().any(|c| c.address == server) {
            return Some(server.to_owned());
        }
    }
    let deepest = |native_only: bool| {
        candidates
            .iter()
            .filter(|c| !native_only || c.is_native_pair)
            .max_by(|a, b| {
                pool_liquidity(a)
                    .partial_cmp(&pool_liquidity(b))
                    .unwrap_or(Ordering::Equal)
            })
    };
    let current =
        current_default.and_then(|address| candidates.iter().find(|c| c.address == address));
    let chosen = match (current, deepest(true)) {
        (Some(current), Some(native))
            if native.address != current.address
                && pool_liquidity(native)
                    >= SERIES_POOL_SWITCH_FACTOR * pool_liquidity(current) =>
        {
            native
        }
        (Some(current), _) => current,
        (None, Some(native)) => native,
        (None, None) => deepest(false)?,
    };
    Some(chosen.address.clone())
}

fn pool_liquidity(config: &PoolConfig) -> f64 {
    if config.liquidity.is_finite() && config.liquidity > 0.0 {
        config.liquidity
    } else {
        0.0
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_pools: usize,
    pub healthy_pools: usize,
    pub total_liquidity: f64,
    pub has_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(address: &str, liquidity: f64, native: bool) -> PoolConfig {
        let mut config = PoolConfig::new(address.to_owned(), "dex".to_owned(), liquidity);
        config.is_native_pair = native;
        config
    }

    #[test]
    fn the_data_server_pool_is_the_default_even_when_usd_quoted() {
        let candidates = [pool("sol", 900_000.0, true), pool("usdc", 100_000.0, false)];
        assert_eq!(
            select_series_default(Some("usdc"), Some("sol"), &candidates).as_deref(),
            Some("usdc")
        );
    }

    #[test]
    fn native_pools_within_the_switch_factor_keep_the_current_default() {
        // The pools swapped depth order, but neither holds twice the other's liquidity.
        let candidates = [pool("a", 150_000.0, true), pool("b", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, Some("b"), &candidates).as_deref(),
            Some("b")
        );
        // A server pool that is not registered is no server pool.
        assert_eq!(
            select_series_default(Some("gone"), Some("b"), &candidates).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn a_native_pool_at_the_switch_factor_replaces_the_current_default() {
        let candidates = [pool("a", 200_000.0, true), pool("b", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, Some("b"), &candidates).as_deref(),
            Some("a")
        );
        // A USD default yields only to a native pool at the factor.
        let candidates = [pool("sol", 150_000.0, true), pool("usdc", 100_000.0, false)];
        assert_eq!(
            select_series_default(None, Some("usdc"), &candidates).as_deref(),
            Some("usdc")
        );
    }

    #[test]
    fn without_a_default_the_deepest_native_pool_wins_then_the_deepest_pool() {
        let candidates = [pool("usdc", 900_000.0, false), pool("sol", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, None, &candidates).as_deref(),
            Some("sol")
        );
        let candidates = [
            pool("usdc", 900_000.0, false),
            pool("usdt", 100_000.0, false),
        ];
        assert_eq!(
            select_series_default(None, Some("gone"), &candidates).as_deref(),
            Some("usdc")
        );
        assert_eq!(select_series_default(None, None, &[]), None);
    }
}
