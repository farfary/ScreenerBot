/// Pool discovery module
///
/// This module handles discovering pools for watched tokens from various sources:
/// - DexScreener API pool discovery
/// - GeckoTerminal API pool discovery
/// - Raydium API pool discovery
/// - Database cache of known pools
///
/// The discovery module feeds raw pool information to the analyzer for classification and program kind detection.
use super::types::{max_watched_tokens, PoolDescriptor, ProgramKind};
use super::utils::is_stablecoin_mint;

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::filtering;
use crate::logger::{self, LogTag};
use crate::pools::service::{
    get_debug_token_override, get_pool_analyzer, is_single_pool_mode_enabled,
};
use crate::pools::utils::is_sol_mint;
use crate::tokens::{
    get_token_pools_snapshot, get_token_pools_snapshot_allow_stale, prefetch_token_pools,
};
use crate::tokens::types::TokenPoolInfo;

use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

// Timing constants
const DISCOVERY_TICK_INTERVAL_SECS: u64 = 5;

/// Returns whether DexScreener discovery is enabled via configuration
pub fn is_dexscreener_discovery_enabled() -> bool {
    with_config(|cfg| cfg.pools.enable_dexscreener_discovery)
}

/// Returns whether GeckoTerminal discovery is enabled via configuration
pub fn is_geckoterminal_discovery_enabled() -> bool {
    with_config(|cfg| cfg.pools.enable_geckoterminal_discovery)
}

/// Returns whether Raydium discovery is enabled via configuration
pub fn is_raydium_discovery_enabled() -> bool {
    with_config(|cfg| cfg.pools.enable_raydium_discovery)
}

/// Pool discovery service state
pub struct PoolDiscovery {
    known_pools: HashMap<Pubkey, PoolDescriptor>,
    watched_tokens: Vec<String>,
}

impl PoolDiscovery {
    /// Create new pool discovery instance
    pub fn new() -> Self {
        Self {
            known_pools: HashMap::new(),
            watched_tokens: Vec::new(),
        }
    }

    /// Get current discovery source configuration
    pub fn get_source_config() -> (bool, bool, bool) {
        (
            is_dexscreener_discovery_enabled(),
            is_geckoterminal_discovery_enabled(),
            is_raydium_discovery_enabled(),
        )
    }

    /// Log the current discovery source configuration
    pub fn log_source_config() {
        let (dex_enabled, gecko_enabled, raydium_enabled) = Self::get_source_config();
        let enabled_sources: Vec<&str> = [
            if dex_enabled {
                Some("DexScreener")
            } else {
                None
            },
            if gecko_enabled {
                Some("GeckoTerminal")
            } else {
                None
            },
            if raydium_enabled {
                Some("Raydium")
            } else {
                None
            },
        ]
        .iter()
        .filter_map(|&s| s)
        .collect();

        if enabled_sources.is_empty() {
            logger::warning(
                LogTag::PoolDiscovery,
                "⚠️ No pool discovery sources enabled!",
            );
        } else {
            logger::info(
                LogTag::PoolDiscovery,
                &format!(
                    "🔍 Pool discovery sources enabled: {}",
                    enabled_sources.join(", ")
                ),
            );
        }

        // Log disabled sources for clarity
        let disabled_sources: Vec<&str> = [
            if !dex_enabled {
                Some("DexScreener")
            } else {
                None
            },
            if !gecko_enabled {
                Some("GeckoTerminal")
            } else {
                None
            },
            if !raydium_enabled {
                Some("Raydium")
            } else {
                None
            },
        ]
        .iter()
        .filter_map(|&s| s)
        .collect();

        if !disabled_sources.is_empty() {
            logger::debug(
                LogTag::PoolDiscovery,
                &format!(
                    "🚫 Pool discovery sources disabled: {}",
                    disabled_sources.join(", ")
                ),
            );
        }
    }

    /// Start discovery background task
    pub async fn start_discovery_task(&self, shutdown: Arc<Notify>) {
        logger::info(LogTag::PoolDiscovery, "Starting pool discovery task");

        // Log the current source configuration
        Self::log_source_config();

        let interval_seed = DISCOVERY_TICK_INTERVAL_SECS;

        tokio::spawn(async move {
            let mut current_interval = interval_seed;
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(current_interval));

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        logger::info(LogTag::PoolDiscovery, "Pool discovery task shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        Self::batched_discovery_tick().await;

                        let updated_interval = DISCOVERY_TICK_INTERVAL_SECS;
                        if updated_interval != current_interval {
                            current_interval = updated_interval;
                            interval = tokio::time::interval(tokio::time::Duration::from_secs(current_interval));
                        }
                    }
                }
            }
        });
    }

    /// Execute one batched discovery tick: fetch pools for all tokens via batch APIs and stream to analyzer
    async fn batched_discovery_tick() {
        let tick_start = Instant::now();

        // Check if any sources are enabled (hot-reload aware)
        let (dex_enabled, gecko_enabled, raydium_enabled) = with_config(|cfg| {
            (
                cfg.pools.enable_dexscreener_discovery,
                cfg.pools.enable_geckoterminal_discovery,
                cfg.pools.enable_raydium_discovery,
            )
        });
        let max_watched = max_watched_tokens();

        record_safe(Event::info(
            EventCategory::Pool,
            Some("discovery_tick_started".to_string()),
            None,
            None,
            serde_json::json!({
                "dexscreener_enabled": dex_enabled,
                "geckoterminal_enabled": gecko_enabled,
                "raydium_enabled": raydium_enabled
            }),
        ))
        .await;

        if !dex_enabled && !gecko_enabled && !raydium_enabled {
            logger::warning(
                LogTag::PoolDiscovery,
                "All pool discovery sources disabled - skipping tick",
            );

            record_safe(Event::warn(
                EventCategory::Pool,
                Some("discovery_sources_disabled".to_string()),
                None,
                None,
                serde_json::json!({
                    "warning": "All discovery sources disabled",
                    "action": "skipping_tick"
                }),
            ))
            .await;

            return;
        }

        // Build token list (respect debug override and global filtering)
        let mut tokens: Vec<String> = if let Some(override_tokens) = get_debug_token_override() {
            override_tokens
        } else {
            // Get passed tokens from tokens module (not filtering module)
            let passed_tokens = crate::tokens::get_passed_tokens();

            if passed_tokens.is_empty() {
                logger::info(
                    LogTag::PoolDiscovery,
                    "No tokens passed filtering - pool service has nothing to price",
                );
            }

            passed_tokens
        };

        // CRITICAL FIX: Always include tokens with open positions for price monitoring
        // Position tokens must be monitored regardless of whether they still meet filtering criteria
        let open_position_mints: Vec<String> = crate::positions::get_open_mints().await;
        let initial_count = tokens.len();

        // Use HashSet for efficient duplicate checking
        let mut token_set: std::collections::HashSet<String> = tokens.iter().cloned().collect();

        for mint in open_position_mints.iter() {
            // Skip stablecoins and tokens already in the set
            if !is_stablecoin_mint(mint) && !token_set.contains(mint) {
                token_set.insert(mint.clone());
                tokens.push(mint.clone());
            }
        }

        let added_count = tokens.len() - initial_count;
        if added_count > 0 {
            logger::info(
                LogTag::PoolDiscovery,
                &format!(
                    "Added {} open position tokens to monitoring set",
                    added_count
                ),
            );
        }

        if tokens.is_empty() {
            logger::debug(LogTag::PoolDiscovery, "No tokens to discover this tick");

            record_safe(Event::info(
                EventCategory::Pool,
                Some("discovery_tick_empty".to_string()),
                None,
                None,
                serde_json::json!({
                    "reason": "no_tokens_to_discover",
                    "duration_ms": tick_start.elapsed().as_millis()
                }),
            ))
            .await;

            return;
        }

        // Early stablecoin filtering - position tokens already filtered above
        tokens.retain(|m| !is_stablecoin_mint(m));

        // Cap to configured max_watched but prioritize position tokens
        if tokens.len() > max_watched {
            // Ensure position tokens are preserved when truncating — reuse the set we already fetched
            let open_position_mints_set: std::collections::HashSet<String> =
                open_position_mints.iter().cloned().collect();

            // Separate position tokens from others
            let (mut position_tokens, mut other_tokens): (Vec<String>, Vec<String>) = tokens
                .into_iter()
                .partition(|mint| open_position_mints_set.contains(mint));

            // Always include all position tokens, then fill remaining slots with other tokens
            let remaining_slots = max_watched.saturating_sub(position_tokens.len());
            other_tokens.truncate(remaining_slots);

            // Combine back: position tokens first, then others
            position_tokens.extend(other_tokens);
            tokens = position_tokens;

            logger::info(
                LogTag::PoolDiscovery,
                &format!(
                    "Truncated to {} tokens (prioritized {} position tokens)",
                    tokens.len(),
                    open_position_mints_set.len()
                ),
            );
        }

        logger::debug(
            LogTag::PoolDiscovery,
            &format!("Discovery tick: {} tokens queued", tokens.len()),
        );

        record_safe(Event::info(
            EventCategory::Pool,
            Some("discovery_tokens_prepared".to_string()),
            None,
            None,
            serde_json::json!({
                "token_count": tokens.len(),
                "position_tokens": open_position_mints.len(),
                "initial_filtered": initial_count,
                "max_watched": max_watched
            }),
        ))
        .await;

        // Prefetch pool snapshots through the tokens module to reuse cached data
        prefetch_token_pools(&tokens).await;

        // Convert to PoolDescriptor list sourced exclusively from token snapshots
        let mut descriptors: Vec<PoolDescriptor> = Vec::new();
        let mut tokens_with_pools = 0usize;

        for mint in tokens.iter() {
            let before = descriptors.len();
            let mut token_descriptors =
                Self::load_descriptors_from_snapshot(mint, dex_enabled, gecko_enabled).await;

            if token_descriptors.is_empty() {
                logger::debug(
                    LogTag::PoolDiscovery,
                    &format!("No eligible pool descriptors found for mint={}", mint),
                );
                continue;
            }

            tokens_with_pools += 1;
            descriptors.append(&mut token_descriptors);

            let added = descriptors.len().saturating_sub(before);
            logger::debug(
                LogTag::PoolDiscovery,
                &format!("Discovered {} pool descriptors for mint={}", added, mint),
            );
        }

        if raydium_enabled {
            logger::debug(
                LogTag::PoolDiscovery,
                "Raydium discovery configured but handled by analyzer using existing snapshots",
            );
        }

        logger::debug(
            LogTag::PoolDiscovery,
            &format!(
                "Discovery tick processed {} tokens, {} yielded pool snapshots",
                tokens.len(),
                tokens_with_pools
            ),
        );

        if descriptors.is_empty() {
            logger::debug(LogTag::PoolDiscovery, "No pools discovered in this tick");

            record_safe(Event::info(
                EventCategory::Pool,
                Some("discovery_tick_completed".to_string()),
                None,
                None,
                serde_json::json!({
                    "pools_discovered": 0,
                    "token_count": tokens.len(),
                    "duration_ms": tick_start.elapsed().as_millis(),
                    "result": "no_pools_found"
                }),
            ))
            .await;

            return;
        }

        // Deduplicate by pool_id and sort by liquidity desc
        let descriptors_count = descriptors.len();
        let mut deduped = Self::deduplicate_discovered(descriptors);
        let deduped_count = deduped.len();

        // If single pool mode, keep only highest-liquidity pool per token mint
        if is_single_pool_mode_enabled() {
            deduped = Self::select_highest_liquidity_per_token(deduped);
        }

        let final_pool_count = deduped.len();

        // Stream to analyzer immediately
        if let Some(analyzer) = get_pool_analyzer() {
            let sender = analyzer.get_sender();
            let mut sent_count = 0;

            for pool in deduped.into_iter() {
                // Check if pool is blacklisted
                match super::db::is_pool_blacklisted(&pool.pool_id.to_string()).await {
                    Ok(true) => {
                        continue; // Skip blacklisted pool silently
                    }
                    Ok(false) => {
                        // Not blacklisted, proceed to check token
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Failed to check pool blacklist for {}: {} - skipping as precaution",
                                pool.pool_id, e
                            ),
                        );
                        // FAIL-CLOSED: Skip pool if blacklist check fails
                        continue;
                    }
                }

                // Check if token is blacklisted
                let token_mint = if is_sol_mint(&pool.base_mint.to_string()) {
                    pool.quote_mint.to_string()
                } else {
                    pool.base_mint.to_string()
                };

                if let Some(db) = crate::tokens::database::get_global_database() {
                    if let Ok(is_blacklisted) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async { db.is_blacklisted(&token_mint) })
                    }) {
                        if is_blacklisted {
                            logger::debug(
                                LogTag::PoolDiscovery,
                                &format!("Skipping pool for blacklisted token: {}", token_mint),
                            );
                            continue;
                        }
                    }
                }

                // Send to analyzer
                let _ = sender.send(crate::pools::analyzer::AnalyzerMessage::AnalyzePool {
                    pool_id: pool.pool_id,
                    program_id: Pubkey::default(),
                    base_mint: pool.base_mint,
                    quote_mint: pool.quote_mint,
                    liquidity_usd: pool.liquidity_usd,
                    volume_h24_usd: pool.volume_h24_usd,
                });
                sent_count += 1;
            }

            record_safe(Event::info(
                EventCategory::Pool,
                Some("discovery_tick_completed".to_string()),
                None,
                None,
                serde_json::json!({
                    "pools_discovered": descriptors_count,
                    "pools_deduped": deduped_count,
                    "pools_final": final_pool_count,
                    "pools_sent_to_analyzer": sent_count,
                    "pools_filtered_blacklist": final_pool_count - sent_count,
                    "token_count": tokens.len(),
                    "duration_ms": tick_start.elapsed().as_millis(),
                    "single_pool_mode": is_single_pool_mode_enabled(),
                    "result": "success"
                }),
            ))
            .await;
        } else {
            record_safe(Event::error(
                EventCategory::Pool,
                Some("discovery_analyzer_unavailable".to_string()),
                None,
                None,
                serde_json::json!({
                    "error": "Analyzer not initialized",
                    "pools_discovered": final_pool_count,
                    "token_count": tokens.len(),
                    "duration_ms": tick_start.elapsed().as_millis()
                }),
            ))
            .await;

            logger::warning(
                LogTag::PoolDiscovery,
                "Analyzer not initialized; cannot stream discovered pools",
            );
        }
    }

    fn deduplicate_discovered(pools: Vec<PoolDescriptor>) -> Vec<PoolDescriptor> {
        let mut map: HashMap<Pubkey, PoolDescriptor> = HashMap::new();
        for p in pools.into_iter() {
            match map.get(&p.pool_id) {
                Some(existing) => {
                    if p.liquidity_usd > existing.liquidity_usd {
                        map.insert(p.pool_id, p);
                    }
                }
                None => {
                    map.insert(p.pool_id, p);
                }
            }
        }
        let mut v: Vec<PoolDescriptor> = map.into_values().collect();
        v.sort_by(|a, b| {
            b.liquidity_usd
                .partial_cmp(&a.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v
    }

    pub(crate) fn select_highest_liquidity_per_token(
        pools: Vec<PoolDescriptor>,
    ) -> Vec<PoolDescriptor> {
        // Group by non-SOL token
        let sol = match Pubkey::from_str(SOL_MINT) {
            Ok(v) => v,
            Err(e) => {
                // Respect project rule: no unwrap/panic; log and return input unchanged
                logger::warning(
                    LogTag::PoolDiscovery,
                    &format!(
                        "Failed to parse SOL_MINT '{}': {} — returning pools unchanged",
                        SOL_MINT, e
                    ),
                );
                return pools;
            }
        };
        let mut best_by_token: HashMap<Pubkey, PoolDescriptor> = HashMap::new();
        for p in pools.into_iter() {
            let token = if p.base_mint == sol {
                p.quote_mint
            } else {
                p.base_mint
            };
            match best_by_token.get(&token) {
                Some(existing) => {
                    // Smart pool selection: prioritize volume when liquidity is misleading
                    let should_replace = if existing.liquidity_usd <= 0.0 && p.liquidity_usd <= 0.0
                    {
                        // Both have no/low liquidity, choose based on volume
                        p.volume_h24_usd > existing.volume_h24_usd
                    } else if existing.liquidity_usd <= 0.0 {
                        // Current has no liquidity, new has some - prefer new
                        true
                    } else if p.liquidity_usd <= 0.0 {
                        // New has no liquidity, current has some - keep current unless volume is massively higher
                        p.volume_h24_usd > existing.volume_h24_usd * 100.0 // 100x volume threshold
                    } else {
                        // Both have liquidity, use traditional liquidity comparison
                        p.liquidity_usd > existing.liquidity_usd
                    };

                    if should_replace {
                        best_by_token.insert(token, p);
                    }
                }
                None => {
                    best_by_token.insert(token, p);
                }
            }
        }
        // Return sorted for determinism (highest liquidity first)
        let mut out: Vec<PoolDescriptor> = best_by_token.into_values().collect();
        out.sort_by(|a, b| {
            b.liquidity_usd
                .partial_cmp(&a.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }

    fn pool_sources_allowed(
        pool: &TokenPoolInfo,
        dex_enabled: bool,
        gecko_enabled: bool,
    ) -> bool {
        let has_dex = pool.sources.dexscreener.is_some();
        let has_gecko = pool.sources.geckoterminal.is_some();

        let mut allowed = false;
        if has_dex {
            allowed |= dex_enabled;
        }
        if has_gecko {
            allowed |= gecko_enabled;
        }

        if !has_dex && !has_gecko {
            // Persisted entries or future sources – allow by default
            allowed = true;
        }

        allowed
    }

    fn convert_token_pool_to_descriptor(pool: &TokenPoolInfo) -> Result<PoolDescriptor, String> {
        if pool.pool_address.trim().is_empty() {
            return Err("Missing pool address".to_string());
        }

        if !pool.is_sol_pair {
            return Err("Pool does not contain SOL".to_string());
        }

        let pool_id = Pubkey::from_str(&pool.pool_address)
            .map_err(|_| format!("Invalid pool address: {}", pool.pool_address))?;
        let base_mint = Pubkey::from_str(&pool.base_mint)
            .map_err(|_| format!("Invalid base mint: {}", pool.base_mint))?;
        let quote_mint = Pubkey::from_str(&pool.quote_mint)
            .map_err(|_| format!("Invalid quote mint: {}", pool.quote_mint))?;

        let liquidity_usd = pool.liquidity_usd.unwrap_or(0.0);
        let volume_h24_usd = pool.volume_h24.unwrap_or(0.0);

        Ok(PoolDescriptor {
            pool_id,
            program_kind: ProgramKind::Unknown,
            base_mint,
            quote_mint,
            reserve_accounts: Vec::new(),
            liquidity_usd,
            volume_h24_usd,
            last_updated: std::time::Instant::now(),
        })
    }

    async fn load_descriptors_from_snapshot(
        mint: &str,
        dex_enabled: bool,
        gecko_enabled: bool,
    ) -> Vec<PoolDescriptor> {
        let primary = get_token_pools_snapshot(mint).await;

        let snapshot = match primary {
            Ok(Some(snapshot)) => Some(snapshot),
            Ok(None) => {
                logger::debug(
                    LogTag::PoolDiscovery,
                    &format!(
                        "No pool snapshot available for mint={} (primary fetch)",
                        mint
                    ),
                );
                match get_token_pools_snapshot_allow_stale(mint).await {
                    Ok(Some(snapshot)) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Using stale pool snapshot for mint={} (no fresh data)",
                                mint
                            ),
                        );
                        Some(snapshot)
                    }
                    Ok(None) => None,
                    Err(err) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Failed to load stale pool snapshot for mint={} error={}",
                                mint, err
                            ),
                        );
                        None
                    }
                }
            }
            Err(err) => {
                logger::warning(
                    LogTag::PoolDiscovery,
                    &format!(
                        "Pool snapshot fetch failed for mint={} error={}",
                        mint, err
                    ),
                );
                match get_token_pools_snapshot_allow_stale(mint).await {
                    Ok(Some(snapshot)) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Using stale pool snapshot for mint={} after error",
                                mint
                            ),
                        );
                        Some(snapshot)
                    }
                    Ok(None) => None,
                    Err(fallback_err) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Failed to load fallback pool snapshot for mint={} error={}",
                                mint, fallback_err
                            ),
                        );
                        None
                    }
                }
            }
        };

        let snapshot = match snapshot {
            Some(snapshot) => snapshot,
            None => {
                logger::debug(
                    LogTag::PoolDiscovery,
                    &format!("No pool snapshot found for mint={}", mint),
                );
                return Vec::new();
            }
        };

        let mut descriptors = Vec::new();
        let mut skipped_non_sol = 0usize;
        let mut skipped_disabled = 0usize;
        let mut parse_errors = 0usize;

        for pool in snapshot.pools.iter() {
            if !Self::pool_sources_allowed(pool, dex_enabled, gecko_enabled) {
                skipped_disabled += 1;
                continue;
            }

            if !pool.is_sol_pair {
                skipped_non_sol += 1;
                continue;
            }

            match Self::convert_token_pool_to_descriptor(pool) {
                Ok(desc) => descriptors.push(desc),
                Err(err) => {
                    parse_errors += 1;
                    logger::debug(
                        LogTag::PoolDiscovery,
                        &format!(
                            "Failed to convert pool for mint={} address={} reason={}",
                            mint, pool.pool_address, err
                        ),
                    );
                }
            }
        }

        if skipped_disabled > 0 {
            logger::debug(
                LogTag::PoolDiscovery,
                &format!(
                    "Filtered {} pools for mint={} due to disabled sources",
                    skipped_disabled, mint
                ),
            );
        }

        if skipped_non_sol > 0 {
            logger::debug(
                LogTag::PoolDiscovery,
                &format!(
                    "Filtered {} non-SOL pools for mint={}",
                    skipped_non_sol, mint
                ),
            );
        }

        if parse_errors > 0 {
            logger::debug(
                LogTag::PoolDiscovery,
                &format!(
                    "Encountered {} pool conversion errors for mint={}",
                    parse_errors, mint
                ),
            );
        }

        descriptors
    }

    /// Discover pools for a specific token
    pub async fn discover_pools_for_token(&self, mint: &str) -> Vec<PoolDescriptor> {
        logger::debug(
            LogTag::PoolDiscovery,
            &format!("Starting pool discovery for token {mint}"),
        );

        // Early stablecoin filtering - reject stablecoin tokens immediately
        if is_stablecoin_mint(mint) {
            logger::warning(
                LogTag::PoolDiscovery,
                &format!(
                    "Token {} is a stablecoin - skipping pool discovery",
                    &mint[..8]
                ),
            );
            return Vec::new();
        }

        let single = vec![mint.to_string()];
        prefetch_token_pools(&single).await;

        let dex_enabled = is_dexscreener_discovery_enabled();
        let gecko_enabled = is_geckoterminal_discovery_enabled();

        let descriptors = Self::load_descriptors_from_snapshot(mint, dex_enabled, gecko_enabled).await;

        if descriptors.is_empty() {
            logger::debug(
                LogTag::PoolDiscovery,
                &format!(
                    "No pool descriptors available for token {} via centralized snapshot",
                    mint
                ),
            );
            return Vec::new();
        }

        // Deduplicate pools by pool address
        let deduplicated_pools = self.deduplicate_pools(descriptors);

        // Return all deduplicated pools - always use biggest pool by liquidity for accurate pricing
        deduplicated_pools
    }

    /// Deduplicate pools by pool address, keeping the one with highest liquidity
    fn deduplicate_pools(&self, pools: Vec<PoolDescriptor>) -> Vec<PoolDescriptor> {
        let mut unique_pools: HashMap<Pubkey, PoolDescriptor> = HashMap::new();

        for pool in pools {
            match unique_pools.get(&pool.pool_id) {
                Some(existing) => {
                    // Keep the pool with higher liquidity
                    if pool.liquidity_usd > existing.liquidity_usd {
                        unique_pools.insert(pool.pool_id, pool);
                    }
                }
                None => {
                    unique_pools.insert(pool.pool_id, pool);
                }
            }
        }

        let mut result: Vec<PoolDescriptor> = unique_pools.into_values().collect();

        // Sort by liquidity (highest first)
        result.sort_by(|a, b| {
            b.liquidity_usd
                .partial_cmp(&a.liquidity_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        logger::debug(
            LogTag::PoolDiscovery,
            &format!("Deduplicated to {} unique pools", result.len()),
        );

        result
    }
}

// =============================================================================
// Lightweight, in-memory discovery cache (canonical pool per mint)
// =============================================================================

// TTL for discovered pools used by non-monitoring components (e.g., OHLCV)
const DISCOVERY_CACHE_TTL_SECS: u64 = 6 * 60 * 60; // 6 hours

// Cache: mint -> (canonical PoolDescriptor, cached_at)
static DISCOVERY_CACHE: OnceLock<Arc<DashMap<Pubkey, (PoolDescriptor, Instant)>>> = OnceLock::new();

// In-flight guard: mint -> Notify; ensures single-flight discovery per mint
static INFLIGHT_GUARD: OnceLock<Arc<DashMap<Pubkey, Arc<Notify>>>> = OnceLock::new();

fn discovery_cache() -> &'static Arc<DashMap<Pubkey, (PoolDescriptor, Instant)>> {
    DISCOVERY_CACHE.get_or_init(|| Arc::new(DashMap::new()))
}

fn inflight_guard() -> &'static Arc<DashMap<Pubkey, Arc<Notify>>> {
    INFLIGHT_GUARD.get_or_init(|| Arc::new(DashMap::new()))
}

fn is_cache_fresh(cached_at: Instant) -> bool {
    cached_at.elapsed() < Duration::from_secs(DISCOVERY_CACHE_TTL_SECS)
}

/// Public accessor: get canonical pool address for a mint (cache-first, single-flight discovery on miss)
///
/// Contract:
/// - Does NOT touch price APIs or price cache
/// - Uses existing discovery + selection logic (no duplication)
/// - Returns Some(<pool_pubkey_string>) on success, None on failure/invalid mint
pub async fn get_canonical_pool_address(mint: &str) -> Option<String> {
    // Parse mint pubkey
    let mint_pk = match Pubkey::from_str(mint) {
        Ok(pk) => pk,
        Err(e) => {
            logger::warning(
                LogTag::PoolDiscovery,
                &format!(
                    "Invalid mint provided to get_canonical_pool_address: {} ({})",
                    mint, e
                ),
            );
            return None;
        }
    };

    // Fast path: check cache
    if let Some(entry) = discovery_cache().get(&mint_pk) {
        let (desc, cached_at) = entry.value();
        if is_cache_fresh(*cached_at) {
            return Some(desc.pool_id.to_string());
        }
    }

    // Single-flight guard: only one discovery per mint at a time
    let notify = {
        let guards = inflight_guard();
        match guards.entry(mint_pk) {
            dashmap::mapref::entry::Entry::Vacant(v) => {
                let n = Arc::new(Notify::new());
                v.insert(n.clone());
                // Leader path: perform discovery
                drop(guards);
                // Re-check cache just before doing network work in case another thread populated it
                if let Some(entry) = discovery_cache().get(&mint_pk) {
                    let (desc, cached_at) = entry.value();
                    if is_cache_fresh(*cached_at) {
                        inflight_guard().remove(&mint_pk);
                        n.notify_waiters();
                        return Some(desc.pool_id.to_string());
                    }
                }

                // Perform discovery using existing logic
                let discovery = PoolDiscovery::new();
                let pools = discovery.discover_pools_for_token(mint).await;

                // Deduplicate and select canonical pool using existing helpers
                let deduped = PoolDiscovery::deduplicate_discovered(pools);
                let selected = PoolDiscovery::select_highest_liquidity_per_token(deduped);

                // Keep only the first selected pool (canonical)
                let result = selected.first().cloned();

                // Update cache and notify waiters
                if let Some(desc) = result.as_ref() {
                    discovery_cache().insert(mint_pk, (desc.clone(), Instant::now()));
                    logger::debug(
                        LogTag::PoolDiscovery,
                        &format!(
                            "Cached canonical pool for mint {} -> {} (program: {:?})",
                            mint, desc.pool_id, desc.program_kind
                        ),
                    );
                } else {
                    logger::warning(
                        LogTag::PoolDiscovery,
                        &format!("No pools discovered for mint {}", mint),
                    );
                }

                // Prepare return value without moving `result`
                let pool_id_str = result.as_ref().map(|d| d.pool_id.to_string());

                inflight_guard().remove(&mint_pk);
                n.notify_waiters();

                return pool_id_str;
            }
            dashmap::mapref::entry::Entry::Occupied(o) => {
                // Follower path: wait for leader to finish
                o.get().clone()
            }
        }
    };

    // Wait for leader with short polling to avoid lost-notify race
    let start = Instant::now();
    let max_wait = Duration::from_secs(5);
    loop {
        // If cache is now fresh, return immediately
        if let Some(entry) = discovery_cache().get(&mint_pk) {
            let (desc, cached_at) = entry.value();
            if is_cache_fresh(*cached_at) {
                return Some(desc.pool_id.to_string());
            }
        }

        // If the guard is gone, leader finished; break to final read
        if !inflight_guard().contains_key(&mint_pk) {
            break;
        }

        // Wait briefly, then re-check
        let _ = tokio::time::timeout(Duration::from_millis(150), notify.notified()).await;
        if start.elapsed() >= max_wait {
            break;
        }
    }

    // Final cache read
    discovery_cache().get(&mint_pk).and_then(|e| {
        let (desc, cached_at) = e.value();
        if is_cache_fresh(*cached_at) {
            Some(desc.pool_id.to_string())
        } else {
            None
        }
    })
}
