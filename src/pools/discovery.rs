/// Pool discovery module
///
/// This module handles discovering pools for watched tokens from various sources:
/// - DexScreener API pool discovery
/// - GeckoTerminal API pool discovery
/// - Raydium API pool discovery
/// - Database cache of known pools
///
/// The discovery module feeds raw pool information to the analyzer for classification and program kind detection.
// =============================================================================
// POOL DISCOVERY SOURCE CONFIGURATION
// =============================================================================
use super::types::{max_watched_tokens, PoolDescriptor, ProgramKind, SOL_MINT};
use super::utils::is_stablecoin_mint;
use crate::config::with_config;
use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::filtering;
use crate::global::is_debug_pool_discovery_enabled;
use crate::logger::{log, LogTag};
use crate::pools::service::{
    get_debug_token_override, get_pool_analyzer, is_single_pool_mode_enabled,
};
use crate::tokens::api::{
    dexscreener::DexScreenerClient,
    dexscreener_types::DexScreenerPool,
    geckoterminal::GeckoTerminalClient,
    geckoterminal_types::GeckoTerminalPool,
};
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

// Timing constants
const DISCOVERY_TICK_INTERVAL_SECS: u64 = 5;

// Lightweight batch result structures for this module only
struct DexsBatchResult {
    pools: HashMap<String, Vec<DexScreenerPool>>, // mint -> pools
}

impl DexsBatchResult {
    fn empty() -> Self {
        Self { pools: HashMap::new() }
    }
}

struct GeckoBatchResult {
    pools: HashMap<String, Vec<GeckoTerminalPool>>, // mint -> pools
}

impl GeckoBatchResult {
    fn empty() -> Self {
        Self { pools: HashMap::new() }
    }
}

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

    async fn fetch_dexscreener_batch(tokens: &[String]) -> DexsBatchResult {
        // DexScreener has a batch endpoint returning one best pair per token.
        // We'll chunk inputs by 30 and fetch sequentially to keep it simple.
        let client = DexScreenerClient::new(
            crate::tokens::api::dexscreener::RATE_LIMIT_PER_MINUTE,
            crate::tokens::api::dexscreener::TIMEOUT_SECS,
        );

        let mut out: HashMap<String, Vec<DexScreenerPool>> = HashMap::new();
        let mut i = 0;
        while i < tokens.len() {
            let end = (i + 30).min(tokens.len());
            let batch = &tokens[i..end];
            match client.fetch_token_batch(batch, Some("solana")).await {
                Ok(pairs) => {
                    for pool in pairs {
                        // Determine which input mint this pair corresponds to
                        // Prefer matching base, else quote
                        let mut assigned = false;
                        for mint in batch {
                            if &pool.base_token_address == mint || &pool.quote_token_address == mint {
                                out.entry(mint.clone()).or_default().push(pool.clone());
                                assigned = true;
                                break;
                            }
                        }
                        if !assigned {
                            // Fallback: group under base token
                            out.entry(pool.base_token_address.clone()).or_default().push(pool.clone());
                        }
                    }
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("DexScreener batch failed ({}..{}): {}", i, end, e),
                        );
                    }
                }
            }
            i = end;
        }
        DexsBatchResult { pools: out }
    }

    async fn fetch_geckoterminal_batch(tokens: &[String]) -> GeckoBatchResult {
        let client = GeckoTerminalClient::new(
            crate::tokens::api::geckoterminal::RATE_LIMIT_PER_MINUTE,
            crate::tokens::api::geckoterminal::TIMEOUT_SECS,
        );
        let mut out: HashMap<String, Vec<GeckoTerminalPool>> = HashMap::new();
        for mint in tokens {
            match client.fetch_pools(mint).await {
                Ok(mut pools) => {
                    out.entry(mint.clone()).or_default().append(&mut pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("GeckoTerminal fetch_pools failed for {}: {}", mint, e),
                        );
                    }
                }
            }
        }
        GeckoBatchResult { pools: out }
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
            log(
                LogTag::PoolDiscovery,
                "WARN",
                "⚠️ No pool discovery sources enabled!",
            );
        } else {
            log(
                LogTag::PoolDiscovery,
                "INFO",
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

        if !disabled_sources.is_empty() && is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!(
                    "🚫 Pool discovery sources disabled: {}",
                    disabled_sources.join(", ")
                ),
            );
        }
    }

    /// Start discovery background task
    pub async fn start_discovery_task(&self, shutdown: Arc<Notify>) {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                "Starting pool discovery task",
            );
        }

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
                        if is_debug_pool_discovery_enabled() {
                            log(LogTag::PoolDiscovery, "INFO", "Pool discovery task shutting down");
                        }
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
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    "All pool discovery sources disabled - skipping tick",
                );
            }

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
            match filtering::get_filtered_token_mints().await {
                Ok(v) => v,
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("Failed to load filtered tokens: {}", e),
                        );
                    }
                    Vec::new()
                }
            }
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
        if added_count > 0 && is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "POSITIONS_ADDED",
                &format!(
                    "Added {} open position tokens to monitoring set",
                    added_count
                ),
            );
        }

        if tokens.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    "No tokens to discover this tick",
                );
            }

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

            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "TRUNCATED",
                    &format!(
                        "Truncated to {} tokens (prioritized {} position tokens)",
                        tokens.len(),
                        open_position_mints_set.len()
                    ),
                );
            }
        }

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("Discovery tick: {} tokens queued", tokens.len()),
            );
        }

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

        // Run batch fetches for all sources concurrently (each handles rate limiting internally)
        // Using tokio::join! to minimize total tick latency vs sequential awaits
        // Only fetch from enabled sources
        let (dexs_batch, gecko_batch) = tokio::join!(
            async {
                if dex_enabled {
                    Self::fetch_dexscreener_batch(&tokens).await
                } else {
                    DexsBatchResult::empty()
                }
            },
            async {
                if gecko_enabled {
                    Self::fetch_geckoterminal_batch(&tokens).await
                } else {
                    GeckoBatchResult::empty()
                }
            }
        );

        // Convert to PoolDescriptor list
        let mut descriptors: Vec<PoolDescriptor> = Vec::new();

        // Process DexScreener results only if enabled
        if is_dexscreener_discovery_enabled() {
            for (mint, pairs) in dexs_batch.pools.into_iter() {
                let before = descriptors.len();
                for pair in pairs {
                    if let Ok(desc) = Self::convert_dexscreener_pair_to_descriptor_static(&pair) {
                        descriptors.push(desc);
                    }
                }
                let added = descriptors.len().saturating_sub(before);
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!("DexScreener batched pools for {mint}: added {added}"),
                    );
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                "DexScreener discovery disabled",
            );
        }

        // Process GeckoTerminal results only if enabled
        if is_geckoterminal_discovery_enabled() {
            for (mint, pools) in gecko_batch.pools.into_iter() {
                let before = descriptors.len();
                for pool in pools {
                    if let Ok(desc) = Self::convert_gecko_pool_to_descriptor_static(&pool) {
                        descriptors.push(desc);
                    }
                }
                let added = descriptors.len().saturating_sub(before);
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!("GeckoTerminal batched pools for {mint}: added {added}"),
                    );
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                "GeckoTerminal discovery disabled",
            );
        }

        // Process Raydium results only if enabled
        // Raydium discovery not implemented via direct API client in this module
        if is_raydium_discovery_enabled() && is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                "Raydium discovery is configured but not implemented in this path",
            );
        }

        if descriptors.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    "No pools discovered in this tick",
                );
            }

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
            for pool in deduped.into_iter() {
                // Let analyzer determine actual program id
                let _ = sender.send(crate::pools::analyzer::AnalyzerMessage::AnalyzePool {
                    pool_id: pool.pool_id,
                    program_id: Pubkey::default(),
                    base_mint: pool.base_mint,
                    quote_mint: pool.quote_mint,
                    liquidity_usd: pool.liquidity_usd,
                    volume_h24_usd: pool.volume_h24_usd,
                });
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

            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    "Analyzer not initialized; cannot stream discovered pools",
                );
            }
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
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "WARN",
                        &format!(
                            "Failed to parse SOL_MINT '{}': {} — returning pools unchanged",
                            SOL_MINT, e
                        ),
                    );
                }
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

    fn convert_dexscreener_pair_to_descriptor_static(
        pair: &DexScreenerPool,
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pair.pair_address).map_err(|_| "Invalid pool address")?;
        let base_mint =
            Pubkey::from_str(&pair.base_token_address).map_err(|_| "Invalid base token address")?;
        let quote_mint = Pubkey::from_str(&pair.quote_token_address)
            .map_err(|_| "Invalid quote token address")?;

        // Ensure SOL on one side
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).map_err(|_| "Invalid SOL mint")?;
        if base_mint != sol_mint_pubkey && quote_mint != sol_mint_pubkey {
            return Err("Pool does not contain SOL - skipping".to_string());
        }

        let liquidity_usd = pair.liquidity_usd.unwrap_or(0.0);
        let volume_h24_usd = pair.volume_h24.unwrap_or(0.0);
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

    fn convert_gecko_pool_to_descriptor_static(
        pool: &GeckoTerminalPool,
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;
        let base_mint =
            Pubkey::from_str(&pool.base_token_id).map_err(|_| "Invalid base token address")?;
        let quote_mint =
            Pubkey::from_str(&pool.quote_token_id).map_err(|_| "Invalid quote token address")?;
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).map_err(|_| "Invalid SOL mint")?;
        if base_mint != sol_mint_pubkey && quote_mint != sol_mint_pubkey {
            return Err("Pool does not contain SOL - skipping".to_string());
        }
        Ok(PoolDescriptor {
            pool_id,
            program_kind: ProgramKind::Unknown,
            base_mint,
            quote_mint,
            reserve_accounts: Vec::new(),
            liquidity_usd: pool.reserve_usd.unwrap_or(0.0),
            volume_h24_usd: pool.volume_h24.unwrap_or(0.0),
            last_updated: std::time::Instant::now(),
        })
    }

    /// Discover pools for a specific token
    pub async fn discover_pools_for_token(&self, mint: &str) -> Vec<PoolDescriptor> {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!("Starting pool discovery for token {mint}"),
            );
        }

        // Early stablecoin filtering - reject stablecoin tokens immediately
        if is_stablecoin_mint(mint) {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    &format!(
                        "Token {} is a stablecoin - skipping pool discovery",
                        &mint[..8]
                    ),
                );
            }
            return Vec::new();
        }

        let mut discovered_pools = Vec::new();

        // Discover from DexScreener API only if enabled
        if is_dexscreener_discovery_enabled() {
            // Create DexScreener client for pool discovery
            let client = DexScreenerClient::new(
                crate::tokens::api::dexscreener::RATE_LIMIT_PER_MINUTE,
                crate::tokens::api::dexscreener::TIMEOUT_SECS,
            );
            
            match client.fetch_token_pools(mint, Some("solana")).await {
                Ok(token_pairs) => {
                    let mut pools = Vec::new();
                    let mut filtered_count = 0;

                    for pair in token_pairs {
                        match Self::convert_dexscreener_pair_to_descriptor_static(&pair) {
                            Ok(pool_descriptor) => {
                                pools.push(pool_descriptor);
                            }
                            Err(e) => {
                                if e.contains("does not contain SOL") {
                                    filtered_count += 1;
                                }
                                // Skip logging individual errors to avoid spam
                            }
                        }
                    }

                    if is_debug_pool_discovery_enabled() {
                        if filtered_count > 0 {
                            log(
                                LogTag::PoolDiscovery,
                                "DEBUG",
                                &format!(
                                    "DexScreener: Filtered out {} non-SOL pools for {mint}",
                                    filtered_count
                                ),
                            );
                        }
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("DexScreener found {} pools for {mint}", pools.len()),
                        );
                    }
                    discovered_pools.extend(pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("DexScreener discovery failed for {mint}: {}", e),
                        );
                    }
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("DexScreener discovery disabled for {mint}"),
            );
        }

        // Discover from GeckoTerminal API only if enabled
        if is_geckoterminal_discovery_enabled() {
            match self.discover_from_geckoterminal(mint).await {
                Ok(mut pools) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("GeckoTerminal found {} pools for {mint}", pools.len()),
                        );
                    }
                    discovered_pools.append(&mut pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("GeckoTerminal discovery failed for {mint}: {}", e),
                        );
                    }
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("GeckoTerminal discovery disabled for {mint}"),
            );
        }

        // Raydium discovery path removed here (not implemented)

        // Deduplicate pools by pool address
        let deduplicated_pools = self.deduplicate_pools(discovered_pools);

        // Return all deduplicated pools - always use biggest pool by liquidity for accurate pricing
        deduplicated_pools
    }

    /// Discover pools from GeckoTerminal API
    async fn discover_from_geckoterminal(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let client = GeckoTerminalClient::new(
            crate::tokens::api::geckoterminal::RATE_LIMIT_PER_MINUTE,
            crate::tokens::api::geckoterminal::TIMEOUT_SECS,
        );
        let gecko_pools = client.fetch_pools(mint).await?;

        let mut pools = Vec::new();
        let mut filtered_count = 0;

        for pool in gecko_pools {
            match self.convert_geckoterminal_pool_to_descriptor(&pool) {
                Ok(pool_descriptor) => {
                    pools.push(pool_descriptor);
                }
                Err(e) => {
                    if e.contains("does not contain SOL") {
                        filtered_count += 1;
                    }
                    // Skip logging individual errors to avoid spam
                }
            }
        }

        if is_debug_pool_discovery_enabled() && filtered_count > 0 {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!(
                    "GeckoTerminal: Filtered out {} non-SOL pools for {mint}",
                    filtered_count
                ),
            );
        }

        Ok(pools)
    }

    // Raydium path removed

    /// Convert GeckoTerminal pool to PoolDescriptor
    fn convert_geckoterminal_pool_to_descriptor(
        &self,
        pool: &GeckoTerminalPool,
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;

        let base_mint =
            Pubkey::from_str(&pool.base_token_id).map_err(|_| "Invalid base token address")?;

        let quote_mint =
            Pubkey::from_str(&pool.quote_token_id).map_err(|_| "Invalid quote token address")?;

        // Check if pool contains SOL - reject if neither side is SOL
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).map_err(|_| "Invalid SOL mint")?;
        if base_mint != sol_mint_pubkey && quote_mint != sol_mint_pubkey {
            return Err("Pool does not contain SOL - skipping".to_string());
        }

        Ok(PoolDescriptor {
            pool_id,
            program_kind: ProgramKind::Unknown,
            base_mint,
            quote_mint,
            reserve_accounts: Vec::new(),
            liquidity_usd: pool.reserve_usd.unwrap_or(0.0),
            volume_h24_usd: pool.volume_h24.unwrap_or(0.0),
            last_updated: std::time::Instant::now(),
        })
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

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("Deduplicated to {} unique pools", result.len()),
            );
        }

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
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    &format!(
                        "Invalid mint provided to get_canonical_pool_address: {} ({})",
                        mint, e
                    ),
                );
            }
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
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "INFO",
                            &format!(
                                "Cached canonical pool for mint {} -> {} (program: {:?})",
                                mint, desc.pool_id, desc.program_kind
                            ),
                        );
                    }
                } else if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "WARN",
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
