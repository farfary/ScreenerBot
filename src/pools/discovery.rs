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

/// Enable DexScreener API pool discovery
/// When true: Include DexScreener pools in discovery
/// When false: Skip DexScreener API entirely
pub const ENABLE_DEXSCREENER_DISCOVERY: bool = true;

/// Enable GeckoTerminal API pool discovery
/// When true: Include GeckoTerminal pools in discovery
/// When false: Skip GeckoTerminal API entirely
pub const ENABLE_GECKOTERMINAL_DISCOVERY: bool = false;

/// Enable Raydium API pool discovery
/// When true: Include Raydium pools in discovery
/// When false: Skip Raydium API entirely
pub const ENABLE_RAYDIUM_DISCOVERY: bool = false;

use crate::global::is_debug_pool_discovery_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::dexscreener::{
    get_token_pools_from_dexscreener,
    get_batch_token_pools_from_dexscreener,
    TokenPair,
};
use crate::tokens::geckoterminal::{
    get_token_pools_from_geckoterminal,
    get_batch_token_pools_from_geckoterminal,
    GeckoTerminalPool,
};
use crate::tokens::raydium::{
    get_token_pools_from_raydium,
    get_batch_token_pools_from_raydium,
    RaydiumPool,
};
use super::types::{ PoolDescriptor, ProgramKind, SOL_MINT, MAX_WATCHED_TOKENS };
use crate::pools::service::{
    get_pool_analyzer,
    is_single_pool_mode_enabled,
    get_debug_token_override,
};
use crate::filtering;
use super::utils::{ is_stablecoin_mint };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Notify;

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
        (ENABLE_DEXSCREENER_DISCOVERY, ENABLE_GECKOTERMINAL_DISCOVERY, ENABLE_RAYDIUM_DISCOVERY)
    }

    /// Log the current discovery source configuration
    pub fn log_source_config() {
        let enabled_sources: Vec<&str> = [
            if ENABLE_DEXSCREENER_DISCOVERY { Some("DexScreener") } else { None },
            if ENABLE_GECKOTERMINAL_DISCOVERY { Some("GeckoTerminal") } else { None },
            if ENABLE_RAYDIUM_DISCOVERY { Some("Raydium") } else { None },
        ]
            .iter()
            .filter_map(|&s| s)
            .collect();

        if enabled_sources.is_empty() {
            log(LogTag::PoolDiscovery, "WARN", "⚠️ No pool discovery sources enabled!");
        } else {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!("🔍 Pool discovery sources enabled: {}", enabled_sources.join(", "))
            );
        }

        // Log disabled sources for clarity
        let disabled_sources: Vec<&str> = [
            if !ENABLE_DEXSCREENER_DISCOVERY { Some("DexScreener") } else { None },
            if !ENABLE_GECKOTERMINAL_DISCOVERY { Some("GeckoTerminal") } else { None },
            if !ENABLE_RAYDIUM_DISCOVERY { Some("Raydium") } else { None },
        ]
            .iter()
            .filter_map(|&s| s)
            .collect();

        if !disabled_sources.is_empty() && is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("🚫 Pool discovery sources disabled: {}", disabled_sources.join(", "))
            );
        }
    }

    /// Start discovery background task
    pub async fn start_discovery_task(&self, shutdown: Arc<Notify>) {
        if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "INFO", "Starting pool discovery task");
        }

        // Log the current source configuration
        Self::log_source_config();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

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
                    }
                }
            }
        });
    }

    /// Execute one batched discovery tick: fetch pools for all tokens via batch APIs and stream to analyzer
    async fn batched_discovery_tick() {
        // Check if any sources are enabled
        if
            !ENABLE_DEXSCREENER_DISCOVERY &&
            !ENABLE_GECKOTERMINAL_DISCOVERY &&
            !ENABLE_RAYDIUM_DISCOVERY
        {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    "All pool discovery sources disabled - skipping tick"
                );
            }
            return;
        }

        if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "DEBUG", "🔄 Starting batched discovery tick");
        }

        // Build token list (respect debug override and global filtering)
        let mut tokens: Vec<String> = if let Some(override_tokens) = get_debug_token_override() {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("📋 Using debug token override: {} tokens", override_tokens.len())
                );
            }
            override_tokens
        } else {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "DEBUG", "📋 Loading filtered tokens from database");
            }
            match filtering::get_filtered_tokens().await {
                Ok(v) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("📋 Loaded {} tokens from database before filtering", v.len())
                        );
                    }
                    v
                },
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("Failed to load filtered tokens: {}", e)
                        );
                    }
                    Vec::new()
                }
            }
        };

        if tokens.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "DEBUG", "❌ No tokens to discover this tick - token list is empty");
            }
            return;
        }

        // Early stablecoin filtering and cap to MAX_WATCHED_TOKENS
        let original_count = tokens.len();
        tokens.retain(|m| !is_stablecoin_mint(m));
        let after_stablecoin_filter = tokens.len();
        
        if tokens.len() > MAX_WATCHED_TOKENS {
            tokens.truncate(MAX_WATCHED_TOKENS);
        }

        if is_debug_pool_discovery_enabled() {
            if original_count != after_stablecoin_filter {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("🚫 Filtered out {} stablecoin tokens ({} -> {})", 
                        original_count - after_stablecoin_filter, 
                        original_count, 
                        after_stablecoin_filter)
                );
            }
            if tokens.len() > MAX_WATCHED_TOKENS {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("✂️ Truncated to {} tokens (from {})", MAX_WATCHED_TOKENS, after_stablecoin_filter)
                );
            }
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("🎯 Discovery tick: {} tokens queued for processing", tokens.len())
            );
            
            // Log first few token addresses for debugging
            if tokens.len() > 0 {
                let sample_tokens: Vec<String> = tokens.iter()
                    .take(3)
                    .map(|t| format!("{}...", &t[..8]))
                    .collect();
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("📋 Sample tokens: [{}]", sample_tokens.join(", "))
                );
            }
        }

        // Run batch fetches for all sources concurrently (each handles rate limiting internally)
        // Using tokio::join! to minimize total tick latency vs sequential awaits
        // Only fetch from enabled sources
        if is_debug_pool_discovery_enabled() {
            let enabled_sources: Vec<&str> = [
                if ENABLE_DEXSCREENER_DISCOVERY { Some("DexScreener") } else { None },
                if ENABLE_GECKOTERMINAL_DISCOVERY { Some("GeckoTerminal") } else { None },
                if ENABLE_RAYDIUM_DISCOVERY { Some("Raydium") } else { None },
            ]
                .iter()
                .filter_map(|&s| s)
                .collect();
            
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("🚀 Starting batch fetch from {} sources: [{}]", 
                    enabled_sources.len(), 
                    enabled_sources.join(", "))
            );
        }

        let (dexs_batch, gecko_batch, raydium_batch) = tokio::join!(
            async {
                if ENABLE_DEXSCREENER_DISCOVERY {
                    get_batch_token_pools_from_dexscreener(&tokens).await
                } else {
                    crate::tokens::dexscreener::DexScreenerBatchResult {
                        pools: std::collections::HashMap::new(),
                        errors: std::collections::HashMap::new(),
                        successful_tokens: 0,
                        failed_tokens: 0,
                    }
                }
            },
            async {
                if ENABLE_GECKOTERMINAL_DISCOVERY {
                    get_batch_token_pools_from_geckoterminal(&tokens).await
                } else {
                    crate::tokens::geckoterminal::GeckoTerminalBatchResult {
                        pools: std::collections::HashMap::new(),
                        errors: std::collections::HashMap::new(),
                        successful_tokens: 0,
                        failed_tokens: 0,
                    }
                }
            },
            async {
                if ENABLE_RAYDIUM_DISCOVERY {
                    get_batch_token_pools_from_raydium(&tokens).await
                } else {
                    crate::tokens::raydium::RaydiumBatchResult {
                        pools: std::collections::HashMap::new(),
                        errors: std::collections::HashMap::new(),
                        successful_tokens: 0,
                        failed_tokens: 0,
                    }
                }
            }
        );

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("📊 Batch fetch results - DexScreener: {}/{} tokens successful, GeckoTerminal: {}/{} tokens successful, Raydium: {}/{} tokens successful",
                    dexs_batch.successful_tokens, dexs_batch.successful_tokens + dexs_batch.failed_tokens,
                    gecko_batch.successful_tokens, gecko_batch.successful_tokens + gecko_batch.failed_tokens,
                    raydium_batch.successful_tokens, raydium_batch.successful_tokens + raydium_batch.failed_tokens)
            );
        }

        // Convert to PoolDescriptor list
        let mut descriptors: Vec<PoolDescriptor> = Vec::new();

        // Process DexScreener results only if enabled
        if ENABLE_DEXSCREENER_DISCOVERY {
            let mut dexs_pools_added = 0;
            for (mint, pairs) in dexs_batch.pools.into_iter() {
                for pair in pairs {
                    if let Ok(desc) = Self::convert_dexscreener_pair_to_descriptor_static(&pair) {
                        descriptors.push(desc);
                        dexs_pools_added += 1;
                    }
                }
            }
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("✅ DexScreener: Added {} pool descriptors", dexs_pools_added)
                );
            }
        } else if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "DEBUG", "🚫 DexScreener discovery disabled");
        }

        // Process GeckoTerminal results only if enabled
        if ENABLE_GECKOTERMINAL_DISCOVERY {
            let mut gecko_pools_added = 0;
            for (mint, pools) in gecko_batch.pools.into_iter() {
                for pool in pools {
                    if let Ok(desc) = Self::convert_gecko_pool_to_descriptor_static(&pool) {
                        descriptors.push(desc);
                        gecko_pools_added += 1;
                    }
                }
            }
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("✅ GeckoTerminal: Added {} pool descriptors", gecko_pools_added)
                );
            }
        } else if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "DEBUG", "🚫 GeckoTerminal discovery disabled");
        }

        // Process Raydium results only if enabled
        if ENABLE_RAYDIUM_DISCOVERY {
            let mut raydium_pools_added = 0;
            for (mint, pools) in raydium_batch.pools.into_iter() {
                for pool in pools {
                    if let Ok(desc) = Self::convert_raydium_pool_to_descriptor_static(&pool) {
                        descriptors.push(desc);
                        raydium_pools_added += 1;
                    }
                }
            }
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("✅ Raydium: Added {} pool descriptors", raydium_pools_added)
                );
            }
        } else if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "DEBUG", "🚫 Raydium discovery disabled");
        }

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("📊 Total pools discovered: {} descriptors before deduplication", descriptors.len())
            );
        }

        if descriptors.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "WARN", "⚠️ No pools discovered in this tick - check token filtering and API responses");
            }
            return;
        }

        // Deduplicate by pool_id and sort by liquidity desc
        let mut deduped = Self::deduplicate_discovered(descriptors);
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("🔄 After deduplication: {} unique pools", deduped.len())
            );
        }

        // If single pool mode, keep only highest-liquidity pool per token mint
        if is_single_pool_mode_enabled() {
            let before_single_pool = deduped.len();
            deduped = Self::select_highest_liquidity_per_token(deduped);
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("🎯 Single pool mode: {} -> {} pools (highest liquidity per token)", 
                        before_single_pool, deduped.len())
                );
            }
        }

        // Stream to analyzer immediately
        if let Some(analyzer) = get_pool_analyzer() {
            let sender = analyzer.get_sender();
            let mut sent_count = 0;
            for pool in deduped.into_iter() {
                // Let analyzer determine actual program id
                if let Err(e) = sender.send(crate::pools::analyzer::AnalyzerMessage::AnalyzePool {
                    pool_id: pool.pool_id,
                    program_id: Pubkey::default(),
                    base_mint: pool.base_mint,
                    quote_mint: pool.quote_mint,
                    liquidity_usd: pool.liquidity_usd,
                    volume_h24_usd: pool.volume_h24_usd,
                }) {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("Failed to send pool to analyzer: {}", e)
                        );
                    }
                } else {
                    sent_count += 1;
                }
            }
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "DEBUG",
                    &format!("✅ Successfully sent {} pools to analyzer for processing", sent_count)
                );
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "WARN",
                "❌ Analyzer not initialized; cannot stream discovered pools"
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
        v.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );
        v
    }

    fn select_highest_liquidity_per_token(pools: Vec<PoolDescriptor>) -> Vec<PoolDescriptor> {
        // Group by non-SOL token
        let sol = Pubkey::from_str(SOL_MINT).unwrap();
        let mut best_by_token: HashMap<Pubkey, PoolDescriptor> = HashMap::new();
        for p in pools.into_iter() {
            let token = if p.base_mint == sol { p.quote_mint } else { p.base_mint };
            match best_by_token.get(&token) {
                Some(existing) => {
                    // Smart pool selection: prioritize volume when liquidity is misleading
                    let should_replace = if existing.liquidity_usd <= 0.0 && p.liquidity_usd <= 0.0 {
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
        best_by_token.into_values().collect()
    }

    fn convert_dexscreener_pair_to_descriptor_static(
        pair: &TokenPair
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pair.pair_address).map_err(|_| "Invalid pool address")?;
        let base_mint = Pubkey::from_str(&pair.base_token.address).map_err(
            |_| "Invalid base token address"
        )?;
        let quote_mint = Pubkey::from_str(&pair.quote_token.address).map_err(
            |_| "Invalid quote token address"
        )?;

        // Ensure SOL on one side
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).map_err(|_| "Invalid SOL mint")?;
        if base_mint != sol_mint_pubkey && quote_mint != sol_mint_pubkey {
            return Err("Pool does not contain SOL - skipping".to_string());
        }

        let liquidity_usd = pair.liquidity
            .as_ref()
            .map(|l| l.usd)
            .unwrap_or(0.0);
        let volume_h24_usd = pair.volume.h24.unwrap_or(0.0);
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
        pool: &GeckoTerminalPool
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;
        let base_mint = Pubkey::from_str(&pool.base_token).map_err(
            |_| "Invalid base token address"
        )?;
        let quote_mint = Pubkey::from_str(&pool.quote_token).map_err(
            |_| "Invalid quote token address"
        )?;
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
            liquidity_usd: pool.liquidity_usd,
            volume_h24_usd: pool.volume_24h,
            last_updated: std::time::Instant::now(),
        })
    }

    fn convert_raydium_pool_to_descriptor_static(
        pool: &RaydiumPool
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;
        let base_mint = Pubkey::from_str(&pool.base_token).map_err(
            |_| "Invalid base token address"
        )?;
        let quote_mint = Pubkey::from_str(&pool.quote_token).map_err(
            |_| "Invalid quote token address"
        )?;
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
            liquidity_usd: pool.liquidity_usd,
            volume_h24_usd: 0.0,
            last_updated: std::time::Instant::now(),
        })
    }

    /// Discover pools for a specific token
    pub async fn discover_pools_for_token(&self, mint: &str) -> Vec<PoolDescriptor> {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!("Starting pool discovery for token {}", &mint[..8])
            );
        }

        // Early stablecoin filtering - reject stablecoin tokens immediately
        if is_stablecoin_mint(mint) {
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::PoolDiscovery,
                    "WARN",
                    &format!("Token {} is a stablecoin - skipping pool discovery", &mint[..8])
                );
            }
            return Vec::new();
        }

        let mut discovered_pools = Vec::new();

        // Discover from DexScreener API only if enabled
        if ENABLE_DEXSCREENER_DISCOVERY {
            match get_token_pools_from_dexscreener(mint).await {
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
                                    "DexScreener: Filtered out {} non-SOL pools for {}",
                                    filtered_count,
                                    &mint[..8]
                                )
                            );
                        }
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("DexScreener found {} pools for {}", pools.len(), &mint[..8])
                        );
                    }
                    discovered_pools.extend(pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("DexScreener discovery failed for {}: {}", &mint[..8], e)
                        );
                    }
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("DexScreener discovery disabled for {}", &mint[..8])
            );
        }

        // Discover from GeckoTerminal API only if enabled
        if ENABLE_GECKOTERMINAL_DISCOVERY {
            match self.discover_from_geckoterminal(mint).await {
                Ok(mut pools) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("GeckoTerminal found {} pools for {}", pools.len(), &mint[..8])
                        );
                    }
                    discovered_pools.append(&mut pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("GeckoTerminal discovery failed for {}: {}", &mint[..8], e)
                        );
                    }
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("GeckoTerminal discovery disabled for {}", &mint[..8])
            );
        }

        // Discover from Raydium API only if enabled
        if ENABLE_RAYDIUM_DISCOVERY {
            match self.discover_from_raydium(mint).await {
                Ok(mut pools) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "DEBUG",
                            &format!("Raydium found {} pools for {}", pools.len(), &mint[..8])
                        );
                    }
                    discovered_pools.append(&mut pools);
                }
                Err(e) => {
                    if is_debug_pool_discovery_enabled() {
                        log(
                            LogTag::PoolDiscovery,
                            "WARN",
                            &format!("Raydium discovery failed for {}: {}", &mint[..8], e)
                        );
                    }
                }
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("Raydium discovery disabled for {}", &mint[..8])
            );
        }

        // Deduplicate pools by pool address
        let deduplicated_pools = self.deduplicate_pools(discovered_pools);

        // Return all deduplicated pools - always use biggest pool by liquidity for accurate pricing
        deduplicated_pools
    }

    /// Discover pools from GeckoTerminal API
    async fn discover_from_geckoterminal(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let gecko_pools = get_token_pools_from_geckoterminal(mint).await?;

        let mut pools = Vec::new();
        let mut filtered_count = 0;

        for pool in gecko_pools {
            match self.convert_geckoterminal_pool_to_descriptor(&pool, mint) {
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
                    "GeckoTerminal: Filtered out {} non-SOL pools for {}",
                    filtered_count,
                    &mint[..8]
                )
            );
        }

        Ok(pools)
    }

    /// Discover pools from Raydium API
    async fn discover_from_raydium(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let raydium_pools = get_token_pools_from_raydium(mint).await?;

        let mut pools = Vec::new();
        let mut filtered_count = 0;

        for pool in raydium_pools {
            match self.convert_raydium_pool_to_descriptor(&pool, mint) {
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
                    "Raydium: Filtered out {} non-SOL pools for {}",
                    filtered_count,
                    &mint[..8]
                )
            );
        }

        Ok(pools)
    }

    /// Convert GeckoTerminal pool to PoolDescriptor
    fn convert_geckoterminal_pool_to_descriptor(
        &self,
        pool: &GeckoTerminalPool,
        target_mint: &str
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;

        let base_mint = Pubkey::from_str(&pool.base_token).map_err(
            |_| "Invalid base token address"
        )?;

        let quote_mint = Pubkey::from_str(&pool.quote_token).map_err(
            |_| "Invalid quote token address"
        )?;

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
            liquidity_usd: pool.liquidity_usd,
            volume_h24_usd: 0.0,
            last_updated: std::time::Instant::now(),
        })
    }

    /// Convert Raydium pool to PoolDescriptor
    fn convert_raydium_pool_to_descriptor(
        &self,
        pool: &RaydiumPool,
        target_mint: &str
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pool.pool_address).map_err(|_| "Invalid pool address")?;

        let base_mint = Pubkey::from_str(&pool.base_token).map_err(
            |_| "Invalid base token address"
        )?;

        let quote_mint = Pubkey::from_str(&pool.quote_token).map_err(
            |_| "Invalid quote token address"
        )?;

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
            liquidity_usd: pool.liquidity_usd,
            volume_h24_usd: 0.0,
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
        result.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!("Deduplicated to {} unique pools", result.len())
            );
        }

        result
    }
}
