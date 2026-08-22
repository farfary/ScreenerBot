//! OHLCV manager — coordinates candle fetching, caching, and priority management.

use crate::chains::active_chain;
use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig, PoolMetadata};
use crate::tokens::pools;
use crate::tokens::types::TokenPoolInfo;
use crate::tokens::{fetch_token_pools_immediate, get_token_pools_snapshot_allow_stale};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PoolManager {
    db: Arc<OhlcvDatabase>,
}

impl PoolManager {
    pub fn new(db: Arc<OhlcvDatabase>) -> Self {
        Self { db }
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
    pub async fn get_default_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;
        Ok(pools.into_iter().find(|p| p.is_default))
    }

    /// Get the best available pool (highest liquidity, healthy)
    pub async fn get_best_pool(&self, mint: &str) -> OhlcvResult<Option<PoolConfig>> {
        let pools = self.db.get_pools(mint)?;

        // Find highest liquidity pool that's healthy
        let best = pools
            .into_iter()
            .filter(|p| p.is_healthy() && p.liquidity.is_finite())
            .max_by(|a, b| {
                a.liquidity
                    .partial_cmp(&b.liquidity)
                    .unwrap_or(Ordering::Less)
            });

        Ok(best)
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

        // Check if we need to switch default
        let default_pool = self.get_default_pool(mint).await?;
        if let Some(pool) = default_pool {
            if pool.address == pool_address && !pool.is_healthy() {
                // Switch to best alternative
                if let Some(best) = self.get_best_pool(mint).await? {
                    self.set_default_pool(mint, &best.address).await?;
                }
            }
        }

        Ok(())
    }

    /// Mark a pool as successful
    pub async fn mark_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        self.db.mark_pool_success(mint, pool_address)
    }

    /// Select the best pool for fetching
    /// Returns (pool_address, should_set_as_default)
    pub async fn select_pool_for_fetch(&self, mint: &str) -> OhlcvResult<Option<(String, bool)>> {
        // Try default pool first
        if let Some(default) = self.get_default_pool(mint).await? {
            if default.is_healthy() {
                return Ok(Some((default.address, false)));
            }
        }

        // Fall back to best pool
        if let Some(best) = self.get_best_pool(mint).await? {
            return Ok(Some((best.address, true))); // Should set as new default
        }

        Ok(None)
    }

    /// Discover and register pools for a token using centralized token snapshots.
    /// Uses immediate fetch (bypasses background queue) for fast response.
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
        let snapshot = match fetch_token_pools_immediate(active_chain(), mint).await {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => {
                // Try stale fallback
                match get_token_pools_snapshot_allow_stale(active_chain(), mint).await {
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

        let canonical_address = snapshot
            .canonical_pool_address
            .clone()
            .or_else(|| pools::choose_canonical_pool(&snapshot.pools));

        let mut existing_map: HashMap<String, PoolConfig> = self
            .db
            .get_pools(mint)?
            .into_iter()
            .map(|cfg| (cfg.address.clone(), cfg))
            .collect();

        let mut discovered_configs = Vec::new();
        let mut non_sol_configs = Vec::new();

        for pool in snapshot.pools.iter() {
            let existing = existing_map.remove(&pool.pool_address);
            let config = Self::merge_pool_info(pool, canonical_address.as_deref(), existing);
            if pool.is_sol_pair {
                discovered_configs.push(config);
            } else {
                non_sol_configs.push(config);
            }
        }

        // A token whose ONLY pools are USD-quoted (e.g. a pump token that only ever
        // paired with USDC) has no wSOL pool. We used to bail out ("No SOL pools")
        // and never chart it — but the data server (and SolanaTracker) return
        // SOL-denominated candles for ANY pool by forcing SOL on the paid path, so
        // we CAN chart it. Register the best USD pool as a fallback; the fetcher
        // then skips GeckoTerminal for it (is_sol_pair=false) to avoid pulling USD
        // candles that would poison the SOL series, and relies on the SOL-forcing
        // sources instead.
        if discovered_configs.is_empty() {
            if non_sol_configs.is_empty() {
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
                    "No SOL pool for mint={}; registering best USD pool ({} candidates), \
                     OHLCV via SOL-forcing sources (server/SolanaTracker), Gecko skipped",
                    mint,
                    non_sol_configs.len()
                ),
            );
            discovered_configs = non_sol_configs;
        }

        if !discovered_configs.iter().any(|cfg| cfg.is_default) {
            if let Some(best_idx) = Self::best_pool_index(&discovered_configs) {
                discovered_configs[best_idx].is_default = true;
            }
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

        record_ohlcv_event(
            "pool_discovery_complete",
            Severity::Info,
            Some(mint),
            None,
            json!({
                "mint": mint,
                "pools_found": discovered_configs.len(),
                "sol_pool": discovered_configs.iter().any(|c| c.is_sol_pair),
                "removed_pools": removed_addresses.len(),
                "canonical_address": canonical_address,
            }),
        )
        .await;

        Ok(discovered_configs)
    }

    fn merge_pool_info(
        pool: &TokenPoolInfo,
        canonical: Option<&str>,
        existing: Option<PoolConfig>,
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

        // Carry the pool's SOL/USD denomination so the fetcher can avoid running
        // GeckoTerminal (USD) on a USD-quoted pool.
        config.is_sol_pair = pool.is_sol_pair;

        if let Some(canonical_address) = canonical {
            config.is_default = canonical_address == config.address;
        }

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

    fn best_pool_index(configs: &[PoolConfig]) -> Option<usize> {
        configs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                Self::pool_config_liquidity(a)
                    .partial_cmp(&Self::pool_config_liquidity(b))
                    .unwrap_or(Ordering::Equal)
            })
            .map(|(idx, _)| idx)
    }

    fn pool_config_liquidity(config: &PoolConfig) -> f64 {
        if config.liquidity.is_finite() && config.liquidity > 0.0 {
            config.liquidity
        } else {
            0.0
        }
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

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_pools: usize,
    pub healthy_pools: usize,
    pub total_liquidity: f64,
    pub has_default: bool,
}
