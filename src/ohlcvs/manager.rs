// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV manager — coordinates candle fetching, caching, and priority management.

use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::{OhlcvDatabase, SeriesPoolReset};
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig, PoolMetadata};
use crate::tokens::pools;
use crate::tokens::types::{TokenPoolInfo, TokenPoolsSnapshot};
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

        // A default that just went unhealthy hands the series to the healthy pool the
        // discovery rule would pick, through the same write that resets the token's rows and
        // backfill flags. With no healthy pool left the default stays and the token resolves
        // no series pool until discovery restores one.
        let pools = self.db.get_pools(mint)?;
        let default_failed = pools
            .iter()
            .any(|p| p.is_default && p.address == pool_address && !p.is_healthy());
        if default_failed {
            let healthy: Vec<PoolConfig> =
                pools.iter().filter(|p| p.is_healthy()).cloned().collect();
            if let Some(next) = select_series_default(None, None, &healthy) {
                let write = self.db.write_series_pools(mint, &pools, &next)?;
                if let Some(reset) = write.reset {
                    self.series_moved(mint, &next, &reset, false).await?;
                }
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
    /// liquidity. The pools, the single default and, when the series pool changes,
    /// the reset of the other pools' rows and the token's backfill flags are one
    /// write (`OhlcvDatabase::write_series_pools`).
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

        self.register_snapshot_pools(mint, &snapshot).await
    }

    /// Register the pools of `snapshot` for `mint` (see [`Self::discover_pools`]): the
    /// planned pools, their single default and, when the series moves, the reset of the
    /// token's rows and flags are one write; the hot cache is invalidated after it commits.
    async fn register_snapshot_pools(
        &self,
        mint: &str,
        snapshot: &TokenPoolsSnapshot,
    ) -> OhlcvResult<Vec<PoolConfig>> {
        let server_series = snapshot.series_pool_address.as_deref().filter(|address| {
            snapshot
                .pools
                .iter()
                .any(|pool| pool.pool_address == *address)
        });

        let existing_pools = self.db.get_pools(mint)?;
        let previous_default = existing_pools
            .iter()
            .find(|p| p.is_default)
            .map(|p| p.address.clone());
        let mut existing_map: HashMap<String, PoolConfig> = existing_pools
            .into_iter()
            .map(|cfg| (cfg.address.clone(), cfg))
            .collect();
        let merged: Vec<PoolConfig> = snapshot
            .pools
            .iter()
            .map(|pool| Self::merge_pool_info(pool, existing_map.remove(&pool.pool_address)))
            .collect();

        let Some(plan) = plan_series_pools(merged, server_series, previous_default.as_deref())
        else {
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
        };

        if !plan.pools.iter().any(|p| p.is_native_pair) {
            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "No SOL pool for mint={}; registering {} USD pools, \
                     OHLCV from the data server only",
                    mint,
                    plan.pools.len()
                ),
            );
        }

        let write = self
            .db
            .write_series_pools(mint, &plan.pools, &plan.series)?;

        if !write.removed_pools.is_empty() {
            let preview: Vec<&str> = write
                .removed_pools
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let suffix = if write.removed_pools.len() > 3 {
                format!(" (+{} more)", write.removed_pools.len() - 3)
            } else {
                String::new()
            };

            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "Removed {} stale pool entries with their candles and gaps for mint={} [{}]{}",
                    write.removed_pools.len(),
                    mint,
                    preview.join(", "),
                    suffix
                ),
            );
        }

        if let Some(reset) = &write.reset {
            self.series_moved(mint, &plan.series, reset, server_series.is_some())
                .await?;
        }

        record_ohlcv_event(
            "pool_discovery_complete",
            Severity::Info,
            Some(mint),
            None,
            json!({
                "mint": mint,
                "pools_found": plan.pools.len(),
                "sol_pool": plan.pools.iter().any(|c| c.is_native_pair),
                "removed_pools": write.removed_pools.len(),
                "series_pool": plan.series,
                "server_series_pool": server_series,
            }),
        )
        .await;

        Ok(plan.pools)
    }

    /// Follow up a committed series move onto `current`: invalidate the hot candle cache, so
    /// no reader keeps serving the previous pool's series, and record the move.
    async fn series_moved(
        &self,
        mint: &str,
        current: &str,
        reset: &SeriesPoolReset,
        from_server: bool,
    ) -> OhlcvResult<()> {
        self.cache.invalidate(mint, None, None)?;

        let previous = reset.previous_pool.as_deref().unwrap_or("none");
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
                "previous_pool": reset.previous_pool,
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

/// The pools a snapshot registers for a token and its series pool among them.
#[derive(Debug)]
struct SeriesPoolPlan {
    /// Every pool to keep registered, exactly one of them `is_default`.
    pools: Vec<PoolConfig>,
    series: String,
}

/// Plan a token's registered pools from its snapshot pools (merged with their stored rows).
///
/// A token with a native pool registers its native pools, the data server's series pool and
/// the current default while the snapshot still lists it; every other pool is dropped, so no
/// unregistered pool can keep a stale default. A token without a native pool registers all
/// of its pools (the data server serves them in SOL). `None` when the snapshot has no pool.
fn plan_series_pools(
    snapshot_pools: Vec<PoolConfig>,
    server_series: Option<&str>,
    previous_default: Option<&str>,
) -> Option<SeriesPoolPlan> {
    let has_native = snapshot_pools.iter().any(|p| p.is_native_pair);
    let mut pools: Vec<PoolConfig> = snapshot_pools
        .into_iter()
        .filter(|p| {
            !has_native
                || p.is_native_pair
                || server_series == Some(p.address.as_str())
                || previous_default == Some(p.address.as_str())
        })
        .collect();
    let series = select_series_default(server_series, previous_default, &pools)?;
    for pool in &mut pools {
        pool.is_default = pool.address == series;
    }
    Some(SeriesPoolPlan { pools, series })
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
    use crate::chains::ChainId;
    use crate::ohlcvs::types::{Candle, Priority, Timeframe, TokenOhlcvConfig};

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

    fn plan(
        pools: Vec<PoolConfig>,
        server: Option<&str>,
        previous: Option<&str>,
    ) -> (Vec<String>, String) {
        let plan = plan_series_pools(pools, server, previous).expect("a plan");
        assert_eq!(plan.pools.iter().filter(|p| p.is_default).count(), 1);
        assert!(plan
            .pools
            .iter()
            .any(|p| p.is_default && p.address == plan.series));
        let mut addresses: Vec<String> = plan.pools.into_iter().map(|p| p.address).collect();
        addresses.sort();
        (addresses, plan.series)
    }

    fn names(addresses: &[&str]) -> Vec<String> {
        addresses.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn a_token_with_a_native_pool_keeps_native_server_and_current_default_pools_only() {
        let snapshot = || {
            vec![
                pool("sol", 300_000.0, true),
                pool("usdc", 200_000.0, false),
                pool("usdt", 50_000.0, false),
            ]
        };
        // The data server names a USD pool: it is registered and is the series.
        assert_eq!(
            plan(snapshot(), Some("usdc"), Some("sol")),
            (names(&["sol", "usdc"]), "usdc".to_string())
        );
        // A server-less refresh keeps the USD default while it is listed: sticky, no flip.
        assert_eq!(
            plan(snapshot(), None, Some("usdc")),
            (names(&["sol", "usdc"]), "usdc".to_string())
        );
        // A default the snapshot no longer lists is dropped with every other non-native pool.
        assert_eq!(
            plan(snapshot(), None, Some("gone")),
            (names(&["sol"]), "sol".to_string())
        );
    }

    #[test]
    fn a_token_without_a_native_pool_registers_every_pool() {
        let snapshot = || vec![pool("usdc", 900.0, false), pool("usdt", 100.0, false)];
        // A legacy USD-only token whose server pool is another USD pool moves to it.
        assert_eq!(
            plan(snapshot(), Some("usdt"), Some("usdc")),
            (names(&["usdc", "usdt"]), "usdt".to_string())
        );
        assert_eq!(
            plan(snapshot(), None, None),
            (names(&["usdc", "usdt"]), "usdc".to_string())
        );
        // A new SOL pool below the switch factor keeps the USD default and drops the rest.
        let mut with_sol = snapshot();
        with_sol.push(pool("sol", 1_000.0, true));
        assert_eq!(
            plan(with_sol, None, Some("usdc")),
            (names(&["sol", "usdc"]), "usdc".to_string())
        );
        assert!(plan_series_pools(Vec::new(), None, Some("usdc")).is_none());
    }

    struct Harness {
        manager: PoolManager,
        db: Arc<OhlcvDatabase>,
        path: std::path::PathBuf,
    }

    impl Harness {
        fn open(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "screenerbot-ohlcv-manager-{label}-{}.db",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            let db = Arc::new(OhlcvDatabase::new(&path, ChainId::Solana).unwrap());
            let cache = Arc::new(OhlcvCache::new(ChainId::Solana));
            let manager = PoolManager::new(Arc::clone(&db), cache);
            Self { manager, db, path }
        }

        async fn discover(&self, server: Option<&str>, pools: &[(&str, f64, bool)]) -> String {
            let snapshot = TokenPoolsSnapshot {
                mint: "mint".to_string(),
                pools: pools
                    .iter()
                    .map(|(address, liquidity, native)| TokenPoolInfo {
                        pool_address: address.to_string(),
                        dex: Some("dex".to_string()),
                        is_native_pair: *native,
                        liquidity_native: Some(*liquidity),
                        ..TokenPoolInfo::default()
                    })
                    .collect(),
                series_pool_address: server.map(str::to_string),
                ..TokenPoolsSnapshot::default()
            };
            let returned = self
                .manager
                .register_snapshot_pools("mint", &snapshot)
                .await
                .unwrap();
            let stored = self.db.get_pools("mint").unwrap();
            assert_eq!(stored.iter().filter(|p| p.is_default).count(), 1);
            let series = PoolConfig::series_pool(&stored).unwrap().address.clone();
            assert_eq!(
                PoolConfig::series_pool(&returned).map(|p| p.address.as_str()),
                Some(series.as_str())
            );
            series
        }

        fn seed(&self, pool: &str) {
            self.db
                .insert_candles_batch(
                    "mint",
                    pool,
                    Timeframe::Hour1,
                    &[Candle::new(3_600, 1.0, 1.0, 1.0, 1.0, 1.0)],
                    OhlcvDatabase::NATIVE_SOURCE,
                )
                .unwrap();
        }

        fn rows(&self, pool: &str) -> bool {
            self.db
                .get_time_bounds("mint", pool, Timeframe::Hour1)
                .unwrap()
                .is_some()
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[tokio::test]
    async fn a_server_less_discovery_keeps_a_usd_series_pool_and_one_default() {
        let h = Harness::open("usd-sticky");
        h.db.upsert_monitor_config(&TokenOhlcvConfig::new("mint".to_string(), Priority::High))
            .unwrap();
        let pools = [("sol", 300.0, true), ("usdc", 200.0, false)];
        assert_eq!(h.discover(Some("usdc"), &pools).await, "usdc");
        h.seed("usdc");
        h.db.mark_all_backfills_complete("mint", "usdc").unwrap();

        // The server does not answer: the USD default stays the only default and keeps its rows.
        assert_eq!(h.discover(None, &pools).await, "usdc");
        assert!(h.rows("usdc"));
        assert!(h.db.is_backfill_complete("mint", Timeframe::Hour1).unwrap());

        // The server moves the series: one write moves the default and resets the token.
        assert_eq!(h.discover(Some("sol"), &pools).await, "sol");
        assert!(!h.rows("usdc"));
        assert!(!h.db.is_backfill_complete("mint", Timeframe::Hour1).unwrap());
    }

    #[tokio::test]
    async fn a_failing_default_hands_the_series_over_through_the_reset() {
        let h = Harness::open("failure-handover");
        h.db.upsert_monitor_config(&TokenOhlcvConfig::new("mint".to_string(), Priority::High))
            .unwrap();
        let pools = [("deep", 300.0, true), ("shallow", 100.0, true)];
        assert_eq!(h.discover(None, &pools).await, "deep");
        h.seed("deep");
        h.db.mark_all_backfills_complete("mint", "deep").unwrap();

        for _ in 0..5 {
            h.manager.mark_failure("mint", "deep").await.unwrap();
        }

        let stored = h.db.get_pools("mint").unwrap();
        assert_eq!(stored.iter().filter(|p| p.is_default).count(), 1);
        assert_eq!(
            PoolConfig::series_pool(&stored).map(|p| p.address.as_str()),
            Some("shallow")
        );
        assert!(!h.rows("deep"));
        for tf in Timeframe::all() {
            assert!(!h.db.is_backfill_complete("mint", tf).unwrap(), "{tf:?}");
        }
    }
}
