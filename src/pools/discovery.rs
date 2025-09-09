/// Pool discovery module
///
/// This module handles discovering pools for watched tokens from various sources:
/// - DexScreener API pool discovery
/// - GeckoTerminal API pool discovery
/// - Raydium API pool discovery
/// - Database cache of known pools
///
/// The discovery module feeds raw pool information to the analyzer for classification and program kind detection.

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
use std::time::{ Instant, Duration };

// =============================================================================
// DISCOVERY TUNING CONSTANTS
// =============================================================================
// Maximum number of tokens to process per tick (rotation shard size)
const TOKENS_PER_TICK: usize = 25; // keep small to reduce latency
// Minimum freshness interval before re-discovering same token
const MIN_DISCOVERY_INTERVAL: Duration = Duration::from_secs(90);
// Timeout per upstream source in a batched tick
const SOURCE_TIMEOUT: Duration = Duration::from_secs(8);
// Liquidity threshold to short-circuit further sources in single pool mode
const SHORT_CIRCUIT_LIQUIDITY_USD: f64 = 25_000.0;

// Global lightweight state for rotation & freshness (no locking complexity; best-effort)
struct TokenDiscoveryState {
    rotation_index: usize,
    last_discovery: HashMap<String, Instant>,
}

static mut TOKEN_DISCOVERY_STATE: Option<TokenDiscoveryState> = None;

fn get_state_mut() -> &'static mut TokenDiscoveryState {
    unsafe {
        TOKEN_DISCOVERY_STATE.get_or_insert(TokenDiscoveryState {
            rotation_index: 0,
            last_discovery: HashMap::new(),
        })
    }
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

    /// Start discovery background task
    pub async fn start_discovery_task(&self, shutdown: Arc<Notify>) {
        if is_debug_pool_discovery_enabled() {
            log(LogTag::PoolDiscovery, "INFO", "Starting pool discovery task");
        }

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
        let tick_start = Instant::now();
        // Build token list (respect debug override and global filtering)
        let mut all_tokens: Vec<String> = if let Some(override_tokens) = get_debug_token_override() {
            override_tokens
        } else {
            match filtering::get_filtered_tokens().await {
                Ok(v) => v,
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

        if all_tokens.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "DEBUG", "No tokens to discover this tick");
            }
            return;
        }

        // Early stablecoin filtering & cap
        all_tokens.retain(|m| !is_stablecoin_mint(m));
        if all_tokens.len() > MAX_WATCHED_TOKENS {
            all_tokens.truncate(MAX_WATCHED_TOKENS);
        }

        // Rotation slice selection
        let state = get_state_mut();
        if state.rotation_index >= all_tokens.len() {
            state.rotation_index = 0;
        }
        let start_index = state.rotation_index;
        let end = (state.rotation_index + TOKENS_PER_TICK).min(all_tokens.len());
        let mut tokens: Vec<String> = all_tokens[state.rotation_index..end].to_vec();
        let original_slice_size = tokens.len();
        state.rotation_index = end; // advance

        // Freshness gating: skip tokens discovered recently
        let now = Instant::now();
        tokens.retain(|t| {
            match state.last_discovery.get(t) {
                Some(ts) => now.duration_since(*ts) >= MIN_DISCOVERY_INTERVAL,
                None => true,
            }
        });

        if tokens.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "DEBUG", "Rotation slice empty or all fresh");
            }
            return;
        }

        if is_debug_pool_discovery_enabled() {
            let fresh_skipped = original_slice_size.saturating_sub(tokens.len());
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!(
                    "Discovery tick: slice_size={} queued_tokens={} rotation_index={} total_tokens={} fresh_skipped={} ",
                    original_slice_size,
                    tokens.len(),
                    state.rotation_index,
                    all_tokens.len(),
                    fresh_skipped
                )
            );
        }

        // Fetch DexScreener first (fastest) then concurrently fetch others with timeout
        let dexs_start = Instant::now();
        let dexs_batch = match
            tokio::time::timeout(
                SOURCE_TIMEOUT,
                get_batch_token_pools_from_dexscreener(&tokens)
            ).await
        {
            Ok(res) => res,
            Err(_) => {
                if is_debug_pool_discovery_enabled() {
                    log(LogTag::PoolDiscovery, "WARN", "DexScreener batch timeout");
                }
                // empty fallback
                get_batch_token_pools_from_dexscreener(&Vec::new()).await
            }
        };
        let dexs_ms = dexs_start.elapsed().as_millis();

        // Early process DexScreener pools and optionally short-circuit
        let mut descriptors: Vec<PoolDescriptor> = Vec::with_capacity(tokens.len() * 6); // heuristic
        let mut best_liquidity_by_token: HashMap<Pubkey, f64> = HashMap::new();
        let sol_pk = Pubkey::from_str(SOL_MINT).unwrap();

        for (mint, pairs) in dexs_batch.pools.iter() {
            for pair in pairs {
                if let Ok(desc) = Self::convert_dexscreener_pair_to_descriptor_static(pair) {
                    let token = if desc.base_mint == sol_pk {
                        desc.quote_mint
                    } else {
                        desc.base_mint
                    };
                    let entry = best_liquidity_by_token.entry(token).or_insert(0.0);
                    if desc.liquidity_usd > *entry {
                        *entry = desc.liquidity_usd;
                    }
                    descriptors.push(desc);
                }
            }
        }

        let mut short_circuit = false;
        if is_single_pool_mode_enabled() {
            // If every token in slice already has liquidity above threshold, skip slower sources
            let mut covered = 0usize;
            for (mint, _pairs) in dexs_batch.pools.iter() {
                if let Ok(pk) = Pubkey::from_str(mint) {
                    if
                        best_liquidity_by_token
                            .get(&pk)
                            .map(|v| *v >= SHORT_CIRCUIT_LIQUIDITY_USD)
                            .unwrap_or(false)
                    {
                        covered += 1;
                    }
                }
            }
            if covered == dexs_batch.pools.len() && !dexs_batch.pools.is_empty() {
                short_circuit = true;
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        "Short-circuit: sufficient liquidity from DexScreener only"
                    );
                }
            }
        }

        // Concurrently fetch Gecko & Raydium unless short-circuited
        let (gecko_batch_opt, raydium_batch_opt, gecko_ms, raydium_ms) = if short_circuit {
            (None, None, 0u128, 0u128)
        } else {
            let gecko_start = Instant::now();
            let ray_start = Instant::now();
            let (gecko_res, ray_res) = tokio::join!(
                tokio::time::timeout(SOURCE_TIMEOUT, async {
                    get_batch_token_pools_from_geckoterminal(&tokens).await
                }),
                tokio::time::timeout(SOURCE_TIMEOUT, async {
                    get_batch_token_pools_from_raydium(&tokens).await
                })
            );
            let gecko_batch = match gecko_res {
                Ok(v) => v,
                Err(_) => get_batch_token_pools_from_geckoterminal(&Vec::new()).await,
            };
            let raydium_batch = match ray_res {
                Ok(v) => v,
                Err(_) => get_batch_token_pools_from_raydium(&Vec::new()).await,
            };
            (
                Some(gecko_batch),
                Some(raydium_batch),
                gecko_start.elapsed().as_millis(),
                ray_start.elapsed().as_millis(),
            )
        };

        if let Some(gecko_batch) = gecko_batch_opt.as_ref() {
            for (mint, pools) in gecko_batch.pools.iter() {
                for pool in pools {
                    if let Ok(desc) = Self::convert_gecko_pool_to_descriptor_static(pool) {
                        descriptors.push(desc);
                    }
                }
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!("Gecko pools appended for {}", &mint[..8])
                    );
                }
            }
        }
        if let Some(raydium_batch) = raydium_batch_opt.as_ref() {
            for (mint, pools) in raydium_batch.pools.iter() {
                for pool in pools {
                    if let Ok(desc) = Self::convert_raydium_pool_to_descriptor_static(pool) {
                        descriptors.push(desc);
                    }
                }
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!("Raydium pools appended for {}", &mint[..8])
                    );
                }
            }
        }

        if descriptors.is_empty() {
            if is_debug_pool_discovery_enabled() {
                log(LogTag::PoolDiscovery, "DEBUG", "No pools discovered in this tick");
            }
            return;
        }

        // Deduplicate by pool_id and sort by liquidity desc
        let mut deduped = Self::deduplicate_discovered(descriptors);

        // If single pool mode, keep only highest-liquidity pool per token mint
        if is_single_pool_mode_enabled() {
            deduped = Self::select_highest_liquidity_per_token(deduped);
        }

        // Update state freshness timestamps
        let state = get_state_mut();
        let now = Instant::now();
        for t in tokens.iter() {
            state.last_discovery.insert(t.clone(), now);
        }

        // Stream to analyzer immediately
        if let Some(analyzer) = get_pool_analyzer() {
            let sender = analyzer.get_sender();
            for pool in deduped.into_iter() {
                let _ = sender.send(crate::pools::analyzer::AnalyzerMessage::AnalyzePool {
                    pool_id: pool.pool_id,
                    program_id: Pubkey::default(),
                    base_mint: pool.base_mint,
                    quote_mint: pool.quote_mint,
                    liquidity_usd: pool.liquidity_usd,
                    volume_h24_usd: pool.volume_h24_usd,
                });
            }
        } else if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolDiscovery,
                "WARN",
                "Analyzer not initialized; cannot stream discovered pools"
            );
        }

        if is_debug_pool_discovery_enabled() {
            let total_ms = tick_start.elapsed().as_millis();
            log(
                LogTag::PoolDiscovery,
                "DEBUG",
                &format!(
                    "TickComplete ms_total={} ms_dexs={} ms_gecko={} ms_raydium={} pools_sent={} tokens_slice={} short_circuit={} ",
                    total_ms,
                    dexs_ms,
                    gecko_ms,
                    raydium_ms,
                    state.last_discovery.len(),
                    tokens.len(),
                    short_circuit
                )
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

        // Discover from DexScreener API
        match self.discover_from_dexscreener(mint).await {
            Ok(mut pools) => {
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!("DexScreener found {} pools for {}", pools.len(), &mint[..8])
                    );
                }
                discovered_pools.append(&mut pools);
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

        // Discover from GeckoTerminal API
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

        // Discover from Raydium API
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

        // Deduplicate pools by pool address
        let deduplicated_pools = self.deduplicate_pools(discovered_pools);

        // Return all deduplicated pools - always use biggest pool by liquidity for accurate pricing
        deduplicated_pools
    }

    /// Discover pools from DexScreener API
    async fn discover_from_dexscreener(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let token_pairs = get_token_pools_from_dexscreener(mint).await?;

        let mut pools = Vec::new();
        let mut filtered_count = 0;

        for pair in token_pairs {
            match self.convert_dexscreener_pair_to_descriptor(&pair, mint) {
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
                    "DexScreener: Filtered out {} non-SOL pools for {}",
                    filtered_count,
                    &mint[..8]
                )
            );
        }

        Ok(pools)
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

    /// Convert DexScreener TokenPair to PoolDescriptor
    fn convert_dexscreener_pair_to_descriptor(
        &self,
        pair: &TokenPair,
        target_mint: &str
    ) -> Result<PoolDescriptor, String> {
        let pool_id = Pubkey::from_str(&pair.pair_address).map_err(|_| "Invalid pool address")?;

        let base_mint = Pubkey::from_str(&pair.base_token.address).map_err(
            |_| "Invalid base token address"
        )?;

        let quote_mint = Pubkey::from_str(&pair.quote_token.address).map_err(
            |_| "Invalid quote token address"
        )?;

        // Check if pool contains SOL - reject if neither side is SOL
        let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).map_err(|_| "Invalid SOL mint")?;
        if base_mint != sol_mint_pubkey && quote_mint != sol_mint_pubkey {
            return Err("Pool does not contain SOL - skipping".to_string());
        }

        let liquidity_usd = pair.liquidity
            .as_ref()
            .map(|l| l.usd)
            .unwrap_or(0.0);

        Ok(PoolDescriptor {
            pool_id,
            program_kind: ProgramKind::Unknown,
            base_mint,
            quote_mint,
            reserve_accounts: Vec::new(),
            liquidity_usd,
            volume_h24_usd: 0.0,
            last_updated: std::time::Instant::now(),
        })
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
