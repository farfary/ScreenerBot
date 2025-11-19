/// Pool discovery module
///
/// This module orchestrates pool discovery for watched tokens by:
/// 1. Building token list (filtered + position tokens)
/// 2. Fetching pool snapshots from tokens module (which handles all caching, deduplication, selection)
/// 3. Converting canonical pools to PoolDescriptor format
/// 4. Sending to analyzer for classification
///
/// All pool data fetching, caching, deduplication, and canonical selection is handled by tokens/pools module.
use super::types::{max_watched_tokens, PoolDescriptor, ProgramKind};
use super::utils::is_stablecoin_mint;

use crate::config::with_config;
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};
use crate::pools::service::{
    get_debug_token_override, get_pool_analyzer, is_single_pool_mode_enabled,
};
use crate::pools::utils::is_sol_mint;
use crate::tokens::{get_token_pools_snapshot, prefetch_token_pools};

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
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
    operations: Arc<std::sync::atomic::AtomicU64>,
    errors: Arc<std::sync::atomic::AtomicU64>,
    pools_discovered: Arc<std::sync::atomic::AtomicU64>,
}

impl PoolDiscovery {
    /// Create new pool discovery instance
    pub fn new() -> Self {
        Self {
            known_pools: HashMap::new(),
            watched_tokens: Vec::new(),
            operations: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            errors: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            pools_discovered: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get metrics for this discovery instance
    pub fn get_metrics(&self) -> (u64, u64, u64) {
        (
            self.operations.load(std::sync::atomic::Ordering::Relaxed),
            self.errors.load(std::sync::atomic::Ordering::Relaxed),
            self.pools_discovered
                .load(std::sync::atomic::Ordering::Relaxed),
        )
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

        Self::log_source_config();

        let interval_seed = DISCOVERY_TICK_INTERVAL_SECS;

        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);
        let pools_discovered = Arc::clone(&self.pools_discovered);

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
                        match Self::batched_discovery_tick_with_metrics(&operations, &errors, &pools_discovered).await {
                            Ok(discovered) => {
                                operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                pools_discovered.fetch_add(discovered as u64, std::sync::atomic::Ordering::Relaxed);
                            }
                            Err(_) => {
                                errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }

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

    /// Execute one batched discovery tick: fetch canonical pools from tokens module and stream to analyzer
    async fn batched_discovery_tick_with_metrics(
        _operations: &Arc<std::sync::atomic::AtomicU64>,
        _errors: &Arc<std::sync::atomic::AtomicU64>,
        _pools_discovered: &Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<usize, String> {
        let tick_start = Instant::now();

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
            return Ok(0);
        }

        // Build token list (respect debug override and global filtering)
        let mut tokens: Vec<String> = if let Some(override_tokens) = get_debug_token_override() {
            override_tokens
        } else {
            crate::tokens::get_passed_tokens()
        };

        // Always include tokens with open positions for price monitoring
        let open_position_mints: Vec<String> = crate::positions::get_open_mints().await;
        let initial_count = tokens.len();

        let mut token_set: std::collections::HashSet<String> = tokens.iter().cloned().collect();

        for mint in open_position_mints.iter() {
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
            return Ok(0);
        }

        // Early stablecoin filtering
        tokens.retain(|m| !is_stablecoin_mint(m));

        // Cap to max_watched, prioritizing position tokens
        if tokens.len() > max_watched {
            let open_position_mints_set: std::collections::HashSet<String> =
                open_position_mints.iter().cloned().collect();

            let (mut position_tokens, mut other_tokens): (Vec<String>, Vec<String>) = tokens
                .into_iter()
                .partition(|mint| open_position_mints_set.contains(mint));

            let remaining_slots = max_watched.saturating_sub(position_tokens.len());
            other_tokens.truncate(remaining_slots);

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

        // Prefetch pool snapshots (triggers tokens module caching)
        prefetch_token_pools(&tokens).await;

        // Convert canonical pools to descriptors for analyzer
        let mut sent_count = 0;
        let mut tokens_with_pools = 0;
        let mut blacklist_filtered = 0;

        if let Some(analyzer) = get_pool_analyzer() {
            let sender = analyzer.get_sender();

            for mint in tokens.iter() {
                // Get snapshot from tokens module (already cached, deduplicated, canonical selected)
                let snapshot = match get_token_pools_snapshot(mint).await {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        logger::debug(
                            LogTag::PoolDiscovery,
                            &format!("No pool snapshot for mint={}", mint),
                        );
                        continue;
                    }
                    Err(e) => {
                        logger::debug(
                            LogTag::PoolDiscovery,
                            &format!("Failed to get snapshot for mint={}: {}", mint, e),
                        );
                        continue;
                    }
                };

                // Use canonical pool address (already selected by tokens/pools module)
                let canonical_address = match snapshot.canonical_pool_address {
                    Some(addr) => addr,
                    None => {
                        logger::debug(
                            LogTag::PoolDiscovery,
                            &format!("No canonical pool for mint={}", mint),
                        );
                        continue;
                    }
                };

                tokens_with_pools += 1;

                // Find the canonical pool in the snapshot
                let canonical_pool = snapshot
                    .pools
                    .iter()
                    .find(|p| p.pool_address == canonical_address);

                let canonical_pool = match canonical_pool {
                    Some(p) => p,
                    None => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Canonical pool {} not found in snapshot for mint={}",
                                canonical_address, mint
                            ),
                        );
                        continue;
                    }
                };

                // Check blacklists
                match super::db::is_pool_blacklisted(&canonical_address).await {
                    Ok(true) => {
                        blacklist_filtered += 1;
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!(
                                "Failed to check pool blacklist for {}: {} - skipping",
                                canonical_address, e
                            ),
                        );
                        blacklist_filtered += 1;
                        continue;
                    }
                }

                // Check token blacklist
                let token_mint = if is_sol_mint(&canonical_pool.base_mint) {
                    &canonical_pool.quote_mint
                } else {
                    &canonical_pool.base_mint
                };

                if let Some(db) = crate::tokens::database::get_global_database() {
                    if let Ok(is_blacklisted) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async { db.is_blacklisted(token_mint) })
                    }) {
                        if is_blacklisted {
                            logger::debug(
                                LogTag::PoolDiscovery,
                                &format!("Skipping pool for blacklisted token: {}", token_mint),
                            );
                            blacklist_filtered += 1;
                            continue;
                        }
                    }
                }

                // Parse addresses
                let pool_id = match Pubkey::from_str(&canonical_address) {
                    Ok(pk) => pk,
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!("Invalid pool address {}: {}", canonical_address, e),
                        );
                        continue;
                    }
                };

                let base_mint = match Pubkey::from_str(&canonical_pool.base_mint) {
                    Ok(pk) => pk,
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!("Invalid base mint {}: {}", canonical_pool.base_mint, e),
                        );
                        continue;
                    }
                };

                let quote_mint = match Pubkey::from_str(&canonical_pool.quote_mint) {
                    Ok(pk) => pk,
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolDiscovery,
                            &format!("Invalid quote mint {}: {}", canonical_pool.quote_mint, e),
                        );
                        continue;
                    }
                };

                // Send to analyzer
                let _ = sender.send(crate::pools::analyzer::AnalyzerMessage::AnalyzePool {
                    pool_id,
                    program_id: Pubkey::default(),
                    base_mint,
                    quote_mint,
                    liquidity_usd: canonical_pool.liquidity_usd.unwrap_or(0.0),
                    volume_h24_usd: canonical_pool.volume_h24.unwrap_or(0.0),
                });
                sent_count += 1;
            }

            record_safe(Event::info(
                EventCategory::Pool,
                Some("discovery_tick_completed".to_string()),
                None,
                None,
                serde_json::json!({
                    "tokens_with_pools": tokens_with_pools,
                    "pools_sent_to_analyzer": sent_count,
                    "pools_filtered_blacklist": blacklist_filtered,
                    "token_count": tokens.len(),
                    "duration_ms": tick_start.elapsed().as_millis(),
                    "result": "success"
                }),
            ))
            .await;

            Ok(sent_count)
        } else {
            logger::warning(
                LogTag::PoolDiscovery,
                "Analyzer not initialized; cannot stream discovered pools",
            );
            Err("Analyzer not initialized".to_string())
        }
    }

    /// Discover pools for a specific token (uses tokens module snapshot directly)
    pub async fn discover_pools_for_token(&self, mint: &str) -> Vec<PoolDescriptor> {
        logger::debug(
            LogTag::PoolDiscovery,
            &format!("Starting pool discovery for token {}", mint),
        );

        if is_stablecoin_mint(mint) {
            logger::warning(
                LogTag::PoolDiscovery,
                &format!(
                    "Token {} is a stablecoin - skipping pool discovery",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Vec::new();
        }

        // Get snapshot from tokens module
        let snapshot = match get_token_pools_snapshot(mint).await {
            Ok(Some(s)) => s,
            Ok(None) => {
                logger::debug(
                    LogTag::PoolDiscovery,
                    &format!("No pool snapshot available for mint={}", mint),
                );
                return Vec::new();
            }
            Err(e) => {
                logger::warning(
                    LogTag::PoolDiscovery,
                    &format!("Failed to get pool snapshot for mint={}: {}", mint, e),
                );
                return Vec::new();
            }
        };

        // Convert pools to descriptors
        let mut descriptors = Vec::new();
        for pool in snapshot.pools.iter() {
            if !pool.is_sol_pair {
                continue;
            }

            let pool_id = match Pubkey::from_str(&pool.pool_address) {
                Ok(pk) => pk,
                Err(_) => continue,
            };

            let base_mint = match Pubkey::from_str(&pool.base_mint) {
                Ok(pk) => pk,
                Err(_) => continue,
            };

            let quote_mint = match Pubkey::from_str(&pool.quote_mint) {
                Ok(pk) => pk,
                Err(_) => continue,
            };

            descriptors.push(PoolDescriptor {
                pool_id,
                program_kind: ProgramKind::Unknown,
                base_mint,
                quote_mint,
                reserve_accounts: Vec::new(),
                liquidity_usd: pool.liquidity_usd.unwrap_or(0.0),
                volume_h24_usd: pool.volume_h24.unwrap_or(0.0),
                last_updated: Instant::now(),
            });
        }

        logger::debug(
            LogTag::PoolDiscovery,
            &format!("Discovered {} pools for token {}", descriptors.len(), mint),
        );

        descriptors
    }
}
