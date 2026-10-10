// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV manager — coordinates candle fetching, caching, and priority management.

use crate::config::with_config;
use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::{OhlcvDatabase, SeriesPoolPlan, SeriesPoolReset};
use crate::ohlcvs::monitor::data_server_usable;
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig, PoolMetadata};
use crate::tokens::pools;
use crate::tokens::types::{TokenPoolInfo, TokenPoolsSnapshot};
use crate::tokens::{fetch_token_pools_immediate, get_token_pools_snapshot_allow_stale};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// While the Data Server is unusable, a native pool replaces the current series pool only at
/// this multiple of its liquidity. The Data Server applies the same factor before it moves a
/// stored series, so both sides switch on one rule.
const SERIES_POOL_SWITCH_FACTOR: f64 = 2.0;

/// How long a pool the series was handed away from (see `plan_failure_handover`) stays out of
/// the series choice, so discovery does not move the series back onto it on the evidence its
/// failures already outweighed. The cooldown lives in memory: an app start begins without it.
const SERIES_HANDOVER_COOLDOWN: Duration = Duration::from_secs(6 * 60 * 60);

/// A pool of a mint: `(mint, pool address)`.
type PoolKey = (String, String);

fn pool_key(mint: &str, pool_address: &str) -> PoolKey {
    (mint.to_string(), pool_address.to_string())
}

pub struct PoolManager {
    db: Arc<OhlcvDatabase>,
    cache: Arc<OhlcvCache>,
    /// When each unhealthy pool may be fetched again in this run (see `fetch_due`).
    retry_at: Mutex<HashMap<PoolKey, Instant>>,
    /// Until when each pool the series was handed away from stays out of the series choice
    /// (see `SERIES_HANDOVER_COOLDOWN`).
    cooling_until: Mutex<HashMap<PoolKey, Instant>>,
}

impl PoolManager {
    pub fn new(db: Arc<OhlcvDatabase>, cache: Arc<OhlcvCache>) -> Self {
        Self {
            db,
            cache,
            retry_at: Mutex::new(HashMap::new()),
            cooling_until: Mutex::new(HashMap::new()),
        }
    }

    /// Get all pools for a token
    pub async fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        self.db.get_pools(mint)
    }

    /// The token's series pool from its stored pool rows (see [`PoolConfig::series_pool`]).
    pub async fn series_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;
        Ok(PoolConfig::series_pool(&pools).cloned())
    }

    /// Whether `pool` of `mint` may be fetched now: a healthy pool always, an unhealthy one once
    /// its [`PoolConfig::fetch_retry_delay`] has passed since its last failure in this run. A
    /// success resets the pool's count, so the bounded retry is what heals an unhealthy pool.
    pub fn fetch_due(&self, mint: &str, pool: &PoolConfig) -> bool {
        pool.is_healthy()
            || self
                .retry_at
                .lock()
                .ok()
                .and_then(|retry_at| retry_at.get(&pool_key(mint, &pool.address)).copied())
                .is_none_or(|at| Instant::now() >= at)
    }

    /// Count a failed fetch of `pool_address`. The count and the handover decision are one
    /// write transaction: only a default that just went unhealthy and never produced a candle
    /// hands the series over (`plan_failure_handover`); a default that holds candles keeps the
    /// series and is retried on its backoff (`fetch_due`). While the Data Server is usable it
    /// alone moves the series, so no failure hands it over.
    pub async fn mark_failure(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.count_failure(mint, pool_address, data_server_usable())
            .await
    }

    /// [`Self::mark_failure`] with the Data Server's usability given.
    async fn count_failure(
        &self,
        mint: &str,
        pool_address: &str,
        server_usable: bool,
    ) -> OhlcvResult<()> {
        let failover = !server_usable && with_config(|cfg| cfg.ohlcv.pool_failover_enabled);
        let write = self
            .db
            .mark_pool_failure(mint, pool_address, |pools, holds_candles| {
                let now = Instant::now();
                let delay = pools
                    .iter()
                    .find(|p| p.address == pool_address)
                    .and_then(PoolConfig::fetch_retry_delay);
                if let (Some(delay), Ok(mut retry_at)) = (delay, self.retry_at.lock()) {
                    retry_at.insert(pool_key(mint, pool_address), now + delay);
                }
                let plan = failover
                    .then(|| plan_failure_handover(pools, pool_address, holds_candles))
                    .flatten()?;
                // Recorded inside the transaction, so a discovery that runs right after the
                // commit already sees the cooldown.
                if let Ok(mut cooling_until) = self.cooling_until.lock() {
                    cooling_until
                        .insert(pool_key(mint, pool_address), now + SERIES_HANDOVER_COOLDOWN);
                }
                Some(plan)
            })?;

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

        if let Some(write) = write {
            if let Some(reset) = &write.reset {
                self.series_moved(mint, &write.plan.series, reset, false)
                    .await?;
            }
        }

        Ok(())
    }

    /// Mark a pool as successful
    pub async fn mark_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        if let Ok(mut retry_at) = self.retry_at.lock() {
            retry_at.remove(&pool_key(mint, pool_address));
        }
        self.db.mark_pool_success(mint, pool_address)
    }

    /// The pools of `mint` still in their handover cooldown. Expired entries of every mint are
    /// dropped on the way.
    fn cooling_pools(&self, mint: &str) -> HashSet<String> {
        let Ok(mut cooling_until) = self.cooling_until.lock() else {
            return HashSet::new();
        };
        let now = Instant::now();
        cooling_until.retain(|_, until| now < *until);
        cooling_until
            .keys()
            .filter(|(key_mint, _)| key_mint == mint)
            .map(|(_, address)| address.clone())
            .collect()
    }

    /// Discover and register pools for a token using centralized token snapshots.
    /// Uses immediate fetch (bypasses background queue) for fast response.
    ///
    /// The default (series) pool is the data server's canonical pool whenever the
    /// snapshot names one, whatever its quote. Otherwise the current default is
    /// kept: while the Data Server is usable it alone moves the series, and while it is
    /// unusable only a native pool with `SERIES_POOL_SWITCH_FACTOR` times its
    /// liquidity replaces it. The pools, the single default and, when the series pool changes,
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

        self.register_snapshot_pools(mint, &snapshot, data_server_usable())
            .await
    }

    /// Register the pools of `snapshot` for `mint` (see [`Self::discover_pools`]), with
    /// `server_usable` saying whether the Data Server is usable. The plan is made from the pool
    /// rows read inside the write transaction, so a handover, an adoption or a count committed
    /// meanwhile is never overwritten; the hot cache is invalidated after the commit.
    async fn register_snapshot_pools(
        &self,
        mint: &str,
        snapshot: &TokenPoolsSnapshot,
        server_usable: bool,
    ) -> OhlcvResult<Vec<PoolConfig>> {
        let server_series = snapshot.series_pool_address.as_deref().filter(|address| {
            snapshot
                .pools
                .iter()
                .any(|pool| pool.pool_address == *address)
        });

        let write = self.db.write_series_pools(mint, |registered| {
            let cooling = self.cooling_pools(mint);
            let stored: HashMap<&str, &PoolConfig> = registered
                .iter()
                .map(|cfg| (cfg.address.as_str(), cfg))
                .collect();
            let merged: Vec<PoolConfig> = snapshot
                .pools
                .iter()
                .map(|pool| {
                    Self::merge_pool_info(
                        pool,
                        stored
                            .get(pool.pool_address.as_str())
                            .map(|&cfg| cfg.clone()),
                        cooling.contains(&pool.pool_address),
                    )
                })
                .collect();
            let previous_default = registered.iter().find(|p| p.is_default);
            plan_series_pools(
                merged,
                server_series,
                previous_default,
                &cooling,
                server_usable,
            )
        })?;

        let Some(write) = write else {
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
        let plan = &write.plan;

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

        Ok(write.plan.pools)
    }

    /// Make `server_pool`, the series pool the Data Server reported while serving a candle
    /// request, the token's default through the one series write
    /// (`OhlcvDatabase::write_series_pools`). The server's report is its own series, so it
    /// overrides the switch factor and the handover cooldown; when it already is the one
    /// default nothing is written, so repeated reports never reset the series again. Returns
    /// whether the default moved.
    pub async fn adopt_server_series(&self, mint: &str, server_pool: &str) -> OhlcvResult<bool> {
        let write = self.db.write_series_pools(mint, |registered| {
            plan_server_series(registered, server_pool)
        })?;
        let Some(reset) = write.and_then(|write| write.reset) else {
            return Ok(false);
        };
        self.series_moved(mint, server_pool, &reset, true).await?;
        Ok(true)
    }

    /// The pool a fetched page belongs to: `pool_address`, or the Data Server's series pool
    /// when the page names another one, which first becomes the token's default
    /// (`Self::adopt_server_series`). Every writer of a Data Server page resolves its pool
    /// here before it stores anything.
    pub async fn page_series_pool<'a>(
        &self,
        mint: &str,
        pool_address: &'a str,
        series_pool: Option<&'a str>,
    ) -> OhlcvResult<&'a str> {
        match series_pool {
            Some(server_pool) if server_pool != pool_address => {
                self.adopt_server_series(mint, server_pool).await?;
                Ok(server_pool)
            }
            _ => Ok(pool_address),
        }
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

    /// Merge a snapshot pool into its stored row. A pool in its handover cooldown (`cooling`)
    /// keeps its failure count.
    fn merge_pool_info(
        pool: &TokenPoolInfo,
        existing: Option<PoolConfig>,
        cooling: bool,
    ) -> PoolConfig {
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
        // A pool the series was just handed away from keeps its count until its
        // cooldown ends, so discovery does not move the series straight back.
        if was_existing && !cooling && !config.is_healthy() {
            config.failure_count = 0;
        }

        config
    }

    /// Get pool metadata for API responses
    pub async fn get_pool_metadata(&self, mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
        let pools = self.get_pools(mint).await?;
        Ok(pools.iter().map(PoolMetadata::from).collect())
    }
}

/// Plan a token's registered pools from its snapshot pools (merged with their stored rows).
///
/// A token with a native pool registers its native pools, the data server's series pool and
/// the current default while the snapshot still lists it; every other pool is dropped, so no
/// unregistered pool can keep a stale default. A token without a native pool registers all
/// of its pools (the data server serves them in SOL). Without a data server series pool, a
/// stored default the snapshot omits stays registered and stays the default: a provider-only
/// list is no evidence that the pool is gone. Pools in `cooling` are not chosen as the series
/// unless one is the current default. With `server_usable`, only the data server's pool moves a
/// current default (see `select_series_default`). `None` when the snapshot has no pool.
fn plan_series_pools(
    snapshot_pools: Vec<PoolConfig>,
    server_series: Option<&str>,
    previous_default: Option<&PoolConfig>,
    cooling: &HashSet<String>,
    server_usable: bool,
) -> Option<SeriesPoolPlan> {
    if snapshot_pools.is_empty() {
        return None;
    }
    let previous = previous_default.map(|p| p.address.as_str());
    let has_native = snapshot_pools.iter().any(|p| p.is_native_pair);
    let mut pools: Vec<PoolConfig> = snapshot_pools
        .into_iter()
        .filter(|p| {
            !has_native
                || p.is_native_pair
                || server_series == Some(p.address.as_str())
                || previous == Some(p.address.as_str())
        })
        .collect();
    if let (None, Some(stored)) = (server_series, previous_default) {
        if !pools.iter().any(|p| p.address == stored.address) {
            pools.push(stored.clone());
        }
    }
    let eligible: Vec<PoolConfig> = pools
        .iter()
        .filter(|p| previous == Some(p.address.as_str()) || !cooling.contains(&p.address))
        .cloned()
        .collect();
    let series = select_series_default(server_series, previous, &eligible, server_usable)
        .or_else(|| select_series_default(server_series, previous, &pools, server_usable))?;
    for pool in &mut pools {
        pool.is_default = pool.address == series;
    }
    Some(SeriesPoolPlan { pools, series })
}

/// The plan that makes `server_pool` the series pool over the `registered` rows, or `None`
/// when it already is the one default. A pool the token has not registered is added with an
/// unknown quote, so only the Data Server serves it. The current default and the native pools
/// stay registered under `plan_series_pools`; no cooldown applies to the server's own pool, and
/// a pool registered with no known liquidity is the series all the same.
fn plan_server_series(registered: &[PoolConfig], server_pool: &str) -> Option<SeriesPoolPlan> {
    let defaults: Vec<&str> = registered
        .iter()
        .filter(|p| p.is_default)
        .map(|p| p.address.as_str())
        .collect();
    if defaults.as_slice() == [server_pool] {
        return None;
    }
    let mut pools = registered.to_vec();
    if !pools.iter().any(|p| p.address == server_pool) {
        let mut unknown = PoolConfig::new(server_pool.to_string(), "unknown".to_string(), 0.0);
        unknown.is_native_pair = false;
        pools.push(unknown);
    }
    plan_series_pools(
        pools,
        Some(server_pool),
        registered.iter().find(|p| p.is_default),
        &HashSet::new(),
        true,
    )
}

/// The series handover after a failed fetch of `failed`: a default that is now unhealthy and
/// never produced a candle (no successful fetch, no stored candle) hands the series to the
/// healthy pool the discovery rule picks. A default that holds candles never moves on fetch
/// failures, since a move deletes its rows; it keeps the series and is retried on its backoff.
/// `None` when nothing moves.
fn plan_failure_handover(
    pools: &[PoolConfig],
    failed: &str,
    holds_candles: bool,
) -> Option<SeriesPoolPlan> {
    let default = pools.iter().find(|p| p.is_default && p.address == failed)?;
    if default.is_healthy() || holds_candles || default.last_successful_fetch.is_some() {
        return None;
    }
    let healthy: Vec<PoolConfig> = pools
        .iter()
        .filter(|p| p.address != failed && p.is_healthy())
        .cloned()
        .collect();
    let series = select_series_default(None, None, &healthy, false)?;
    let pools = pools
        .iter()
        .map(|p| PoolConfig {
            is_default: p.address == series,
            ..p.clone()
        })
        .collect();
    Some(SeriesPoolPlan { pools, series })
}

/// The default (series) pool among the registered `candidates`: the data server's
/// pool when it is registered; else the current default, which only the data server moves
/// while it is usable (`server_usable`) and otherwise only a native pool with
/// `SERIES_POOL_SWITCH_FACTOR` times its liquidity replaces; else the deepest native pool,
/// else the deepest pool.
fn select_series_default(
    server_series: Option<&str>,
    current_default: Option<&str>,
    candidates: &[PoolConfig],
    server_usable: bool,
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
        (Some(current), _) if server_usable => current,
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
            select_series_default(Some("usdc"), Some("sol"), &candidates, false).as_deref(),
            Some("usdc")
        );
    }

    #[test]
    fn native_pools_within_the_switch_factor_keep_the_current_default() {
        // The pools swapped depth order, but neither holds twice the other's liquidity.
        let candidates = [pool("a", 150_000.0, true), pool("b", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, Some("b"), &candidates, false).as_deref(),
            Some("b")
        );
        // A server pool that is not registered is no server pool.
        assert_eq!(
            select_series_default(Some("gone"), Some("b"), &candidates, false).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn a_native_pool_at_the_switch_factor_replaces_the_current_default() {
        let candidates = [pool("a", 200_000.0, true), pool("b", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, Some("b"), &candidates, false).as_deref(),
            Some("a")
        );
        // A USD default yields only to a native pool at the factor.
        let candidates = [pool("sol", 150_000.0, true), pool("usdc", 100_000.0, false)];
        assert_eq!(
            select_series_default(None, Some("usdc"), &candidates, false).as_deref(),
            Some("usdc")
        );
    }

    #[test]
    fn without_a_default_the_deepest_native_pool_wins_then_the_deepest_pool() {
        let candidates = [pool("usdc", 900_000.0, false), pool("sol", 100_000.0, true)];
        assert_eq!(
            select_series_default(None, None, &candidates, false).as_deref(),
            Some("sol")
        );
        let candidates = [
            pool("usdc", 900_000.0, false),
            pool("usdt", 100_000.0, false),
        ];
        assert_eq!(
            select_series_default(None, Some("gone"), &candidates, false).as_deref(),
            Some("usdc")
        );
        assert_eq!(select_series_default(None, None, &[], false), None);
    }

    /// Plan `pools` with `previous` as the stored default: the snapshot's row of that address,
    /// or a USD row the snapshot omits.
    fn plan_cooling(
        pools: Vec<PoolConfig>,
        server: Option<&str>,
        previous: Option<&str>,
        cooling: &[&str],
    ) -> (Vec<String>, String) {
        let previous = previous.map(|address| PoolConfig {
            is_default: true,
            ..pools
                .iter()
                .find(|p| p.address == address)
                .cloned()
                .unwrap_or_else(|| pool(address, 200_000.0, false))
        });
        let cooling: HashSet<String> = cooling.iter().map(|a| a.to_string()).collect();
        let plan =
            plan_series_pools(pools, server, previous.as_ref(), &cooling, false).expect("a plan");
        assert_eq!(plan.pools.iter().filter(|p| p.is_default).count(), 1);
        assert!(plan
            .pools
            .iter()
            .any(|p| p.is_default && p.address == plan.series));
        let mut addresses: Vec<String> = plan.pools.into_iter().map(|p| p.address).collect();
        addresses.sort();
        (addresses, plan.series)
    }

    fn plan(
        pools: Vec<PoolConfig>,
        server: Option<&str>,
        previous: Option<&str>,
    ) -> (Vec<String>, String) {
        plan_cooling(pools, server, previous, &[])
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
        // A server-less refresh that omits the stored default keeps it registered and the
        // default; every other non-native pool is dropped.
        assert_eq!(
            plan(snapshot(), None, Some("gone")),
            (names(&["gone", "sol"]), "gone".to_string())
        );
        // A server-backed snapshot that omits the stored default drops it.
        assert_eq!(
            plan(snapshot(), Some("sol"), Some("gone")),
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
        let stored = pool("usdc", 900.0, false);
        assert!(
            plan_series_pools(Vec::new(), None, Some(&stored), &HashSet::new(), false).is_none()
        );
    }

    #[test]
    fn a_cooling_pool_is_never_chosen_unless_it_is_the_current_default() {
        let snapshot = || vec![pool("deep", 300.0, true), pool("shallow", 100.0, true)];
        // Neither the server's choice nor the switch factor moves the series onto it.
        for server in [Some("deep"), None] {
            assert_eq!(
                plan_cooling(snapshot(), server, Some("shallow"), &["deep"]),
                (names(&["deep", "shallow"]), "shallow".to_string())
            );
        }
        // A current default is never excluded, and a token with no other pool still gets one.
        assert_eq!(
            plan_cooling(snapshot(), None, Some("deep"), &["deep"]).1,
            "deep"
        );
        assert_eq!(
            plan_cooling(vec![pool("deep", 300.0, true)], None, None, &["deep"]).1,
            "deep"
        );
    }

    #[test]
    fn only_an_unhealthy_default_that_never_produced_a_candle_hands_the_series_over() {
        crate::config::utils::install_default_config();
        let limit = with_config(|cfg| cfg.ohlcv.max_pool_failures);
        let pools = |failures: u32, succeeded: bool| {
            let mut default = pool("deep", 300.0, true);
            default.is_default = true;
            default.failure_count = failures;
            default.last_successful_fetch = succeeded.then(chrono::Utc::now);
            vec![
                default,
                pool("shallow", 100.0, true),
                pool("usdc", 900.0, false),
            ]
        };
        let handover = |pools: &[PoolConfig], failed: &str, holds_candles: bool| {
            plan_failure_handover(pools, failed, holds_candles).map(|plan| plan.series)
        };

        assert_eq!(
            handover(&pools(limit, false), "deep", false).as_deref(),
            Some("shallow")
        );
        assert_eq!(handover(&pools(limit - 1, false), "deep", false), None);
        assert_eq!(handover(&pools(limit, false), "deep", true), None);
        assert_eq!(handover(&pools(limit, true), "deep", false), None);
        assert_eq!(handover(&pools(limit, false), "shallow", false), None);
        let plan = plan_failure_handover(&pools(limit, false), "deep", false).unwrap();
        assert_eq!(plan.pools.len(), 3);
        assert_eq!(plan.pools.iter().filter(|p| p.is_default).count(), 1);
    }

    struct Harness {
        manager: PoolManager,
        db: Arc<OhlcvDatabase>,
        path: std::path::PathBuf,
        /// Series resets observed so far (see `Harness::count_reset`).
        resets: usize,
    }

    impl Harness {
        fn open(label: &str) -> Self {
            crate::config::utils::install_default_config();
            let path = std::env::temp_dir().join(format!(
                "screenerbot-ohlcv-manager-{label}-{}.db",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            let db = Arc::new(OhlcvDatabase::new(&path, ChainId::Solana).unwrap());
            db.upsert_monitor_config(&TokenOhlcvConfig::new("mint".to_string(), Priority::High))
                .unwrap();
            let cache = Arc::new(OhlcvCache::new(ChainId::Solana));
            let manager = PoolManager::new(Arc::clone(&db), cache);
            Self {
                manager,
                db,
                path,
                resets: 0,
            }
        }

        /// Discovery while the Data Server is unusable.
        async fn discover(&self, server: Option<&str>, pools: &[(&str, f64, bool)]) -> String {
            self.discover_with(server, pools, false).await
        }

        async fn discover_with(
            &self,
            server: Option<&str>,
            pools: &[(&str, f64, bool)],
            server_usable: bool,
        ) -> String {
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
                .register_snapshot_pools("mint", &snapshot, server_usable)
                .await
                .unwrap();
            let series = self.series();
            assert_eq!(
                PoolConfig::series_pool(&returned).map(|p| p.address.as_str()),
                Some(series.as_str())
            );
            series
        }

        /// The one stored default.
        fn series(&self) -> String {
            let stored = self.db.get_pools("mint").unwrap();
            assert_eq!(stored.iter().filter(|p| p.is_default).count(), 1);
            PoolConfig::series_pool(&stored).unwrap().address.clone()
        }

        async fn fail(&self, pool: &str, times: u32) {
            for _ in 0..times {
                self.manager.mark_failure("mint", pool).await.unwrap();
            }
        }

        /// Count a reset since the last call: a reset clears the backfill flags, which are
        /// marked complete again on the current series.
        fn count_reset(&mut self) {
            if !self
                .db
                .is_backfill_complete("mint", Timeframe::Hour1)
                .unwrap()
            {
                self.resets += 1;
            }
            self.db
                .mark_all_backfills_complete("mint", &self.series())
                .unwrap();
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

        fn stored(&self, pool: &str) -> PoolConfig {
            self.db
                .get_pools("mint")
                .unwrap()
                .into_iter()
                .find(|p| p.address == pool)
                .expect("a registered pool")
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    const POOLS: [(&str, f64, bool); 2] = [("deep", 300.0, true), ("shallow", 100.0, true)];

    #[tokio::test]
    async fn a_server_less_discovery_keeps_a_usd_series_pool_and_one_default() {
        let mut h = Harness::open("usd-sticky");
        let pools = [("sol", 300.0, true), ("usdc", 200.0, false)];
        assert_eq!(h.discover(Some("usdc"), &pools).await, "usdc");
        h.seed("usdc");
        h.count_reset();
        h.resets = 0;

        // The server does not answer: the USD default stays the only default and keeps its rows.
        assert_eq!(h.discover(None, &pools).await, "usdc");
        assert!(h.rows("usdc"));
        h.count_reset();
        // A provider-only snapshot that omits the USD pool keeps it as the series.
        assert_eq!(h.discover(None, &[("sol", 300.0, true)]).await, "usdc");
        assert!(h.rows("usdc"));
        h.count_reset();
        assert_eq!(h.resets, 0);

        // The server moves the series: one write moves the default and resets the token.
        assert_eq!(h.discover(Some("sol"), &pools).await, "sol");
        assert!(!h.rows("usdc"));
        h.count_reset();
        assert_eq!(h.resets, 1);
    }

    /// Failures and rediscoveries in sequence, with and without a data server pool: a default
    /// that holds candles never moves and keeps its rows.
    #[tokio::test]
    async fn fetch_failures_and_rediscovery_never_move_a_default_that_holds_candles() {
        for server in [None, Some("deep")] {
            let mut h = Harness::open(&format!("failures-seeded-{}", server.is_some()));
            let limit = with_config(|cfg| cfg.ohlcv.max_pool_failures);
            assert_eq!(h.discover(server, &POOLS).await, "deep");
            h.seed("deep");
            h.count_reset();
            h.resets = 0;

            for _ in 0..2 {
                h.fail("deep", limit).await;
                // Unhealthy, still the series, its stored candles still read.
                assert!(!h.stored("deep").is_healthy());
                assert_eq!(h.series(), "deep");
                assert!(h.rows("deep"));
                h.count_reset();
                assert_eq!(h.discover(server, &POOLS).await, "deep");
                assert!(h.rows("deep"));
                h.count_reset();
            }
            assert_eq!(h.resets, 0, "server={server:?}");
        }
    }

    /// A default that never produced a candle hands the series over once; rediscovery does not
    /// move it back while the handed-away pool cools down, and does once the cooldown ends.
    #[tokio::test]
    async fn a_handover_and_rediscovery_reset_the_series_at_most_once_per_cooldown() {
        for server in [None, Some("deep")] {
            let mut h = Harness::open(&format!("failures-handover-{}", server.is_some()));
            let limit = with_config(|cfg| cfg.ohlcv.max_pool_failures);
            assert_eq!(h.discover(server, &POOLS).await, "deep");
            h.count_reset();
            h.resets = 0;

            h.fail("deep", limit).await;
            assert_eq!(h.series(), "shallow");
            h.count_reset();
            assert_eq!(h.resets, 1, "server={server:?}");
            h.seed("shallow");

            for _ in 0..2 {
                assert_eq!(h.discover(server, &POOLS).await, "shallow");
                assert!(h.rows("shallow"));
                assert!(!h.stored("deep").is_healthy());
                h.count_reset();
                h.fail("deep", limit).await;
                assert_eq!(h.series(), "shallow");
                h.count_reset();
            }
            assert_eq!(h.resets, 1, "server={server:?}");

            // The cooldown ends: the pool is eligible again with a fresh count.
            h.manager
                .cooling_until
                .lock()
                .unwrap()
                .insert(pool_key("mint", "deep"), Instant::now());
            assert_eq!(h.discover(server, &POOLS).await, "deep");
            assert!(h.stored("deep").is_healthy());
        }
    }

    #[tokio::test]
    async fn an_unhealthy_default_is_fetched_again_after_its_backoff_and_heals_on_success() {
        let h = Harness::open("failure-backoff");
        let limit = with_config(|cfg| cfg.ohlcv.max_pool_failures);
        assert_eq!(h.discover(None, &POOLS).await, "deep");
        h.seed("deep");

        h.fail("deep", limit - 1).await;
        assert!(h.manager.fetch_due("mint", &h.stored("deep")));
        h.fail("deep", 1).await;
        let deep = h.stored("deep");
        assert!(!h.manager.fetch_due("mint", &deep));

        let retry_at = h.manager.retry_at.lock().unwrap()[&pool_key("mint", "deep")];
        let delay = deep
            .fetch_retry_delay()
            .expect("an unhealthy pool backs off");
        assert!(retry_at > Instant::now() && retry_at <= Instant::now() + delay);
        h.manager
            .retry_at
            .lock()
            .unwrap()
            .insert(pool_key("mint", "deep"), Instant::now());
        assert!(h.manager.fetch_due("mint", &deep));

        h.manager.mark_success("mint", "deep").await.unwrap();
        assert!(h.stored("deep").is_healthy());
        assert_eq!(h.series(), "deep");
    }

    /// The pool the Data Server reports overrides the switch factor and the cooldown, resets the
    /// series once, and a repeated report of the current default writes nothing.
    #[tokio::test]
    async fn the_data_server_series_pool_is_adopted_once_whatever_the_local_rules_say() {
        let mut h = Harness::open("server-series");
        assert_eq!(h.discover(None, &POOLS).await, "deep");
        h.seed("deep");
        h.count_reset();
        h.resets = 0;
        // Cooling and below the switch factor: local discovery would never choose it.
        h.manager.cooling_until.lock().unwrap().insert(
            pool_key("mint", "shallow"),
            Instant::now() + SERIES_HANDOVER_COOLDOWN,
        );

        assert!(h
            .manager
            .adopt_server_series("mint", "shallow")
            .await
            .unwrap());
        assert_eq!(h.series(), "shallow");
        assert!(!h.rows("deep"));
        h.count_reset();
        assert_eq!(h.resets, 1);

        h.seed("shallow");
        assert!(!h
            .manager
            .adopt_server_series("mint", "shallow")
            .await
            .unwrap());
        assert!(h.rows("shallow"));
        h.count_reset();
        assert_eq!(h.resets, 1);

        // An unregistered server pool is registered with an unknown quote.
        assert!(h
            .manager
            .adopt_server_series("mint", "unlisted")
            .await
            .unwrap());
        assert_eq!(h.series(), "unlisted");
        assert!(!h.stored("unlisted").is_native_pair);
        assert!(h.stored("deep").is_native_pair);
        h.count_reset();
        assert_eq!(h.resets, 2);
    }

    /// While the Data Server is usable it alone moves the series. A pool it named, registered
    /// with no known liquidity and without a candle yet, survives discoveries without a server
    /// pool next to a native pool far past the switch factor, failures past the handover limit
    /// and repeated reports of itself, with no reset at all.
    #[tokio::test]
    async fn while_the_data_server_is_usable_only_it_moves_the_series_pool() {
        let mut h = Harness::open("server-authority");
        let limit = with_config(|cfg| cfg.ohlcv.max_pool_failures);
        assert_eq!(h.discover_with(None, &POOLS, true).await, "deep");
        assert!(h
            .manager
            .adopt_server_series("mint", "server")
            .await
            .unwrap());
        assert_eq!(h.stored("server").liquidity, 0.0);
        h.count_reset();
        h.resets = 0;

        for liquidity in [600.0, 3_000.0, 1e9] {
            let pools = [("native", liquidity, true), ("deep", 300.0, true)];
            assert_eq!(h.discover_with(None, &pools, true).await, "server");
            h.count_reset();
            for _ in 0..limit {
                h.manager
                    .count_failure("mint", "server", true)
                    .await
                    .unwrap();
            }
            assert_eq!(h.series(), "server");
            h.count_reset();
            assert!(!h
                .manager
                .adopt_server_series("mint", "server")
                .await
                .unwrap());
            h.count_reset();
        }
        assert_eq!(h.resets, 0);

        // Once the Data Server is unusable, the local rule applies again.
        assert_eq!(h.discover(None, &[("native", 600.0, true)]).await, "native");
    }
}
