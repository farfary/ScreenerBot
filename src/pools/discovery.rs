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
use crate::tokens::dexscreener::{ get_token_pools_from_dexscreener, TokenPair };
use crate::tokens::geckoterminal::{ get_token_pools_from_geckoterminal, GeckoTerminalPool };
use crate::tokens::raydium::{ get_token_pools_from_raydium, RaydiumPool };
use crate::filtering::{ MIN_SOL_RESERVES, MAX_SOL_RESERVES };
use super::types::{ PoolDescriptor, ProgramKind, SOL_MINT };
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
                        // TODO: Implement actual pool discovery for watched tokens
                        // For now, just log that we're running
                        if is_debug_pool_discovery_enabled() {
                            log(LogTag::PoolDiscovery, "DEBUG", "Pool discovery tick - discovery implementation pending");
                        }
                    }
                }
            }
        });
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

        // Apply SOL reserves filtering as final safety check
        let filtered_pools = self.filter_pools_by_sol_reserves(deduplicated_pools);

        filtered_pools
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

    /// Filter pools by SOL reserves criteria (safety check)
    fn filter_pools_by_sol_reserves(&self, pools: Vec<PoolDescriptor>) -> Vec<PoolDescriptor> {
        let mut filtered_pools = Vec::new();
        let mut filtered_out_count = 0;

        for pool in pools {
            // For now, we'll use liquidity_usd as a proxy for SOL reserves
            // This is not perfect but provides some filtering
            // In the future, we could enhance this by fetching actual reserves
            
            // Convert USD liquidity to approximate SOL reserves (assuming SOL is ~$20-200)
            // This is a rough estimate - actual SOL reserves would require pool account data
            let estimated_sol_reserves = pool.liquidity_usd / 100.0; // Rough estimate

            if estimated_sol_reserves >= MIN_SOL_RESERVES && estimated_sol_reserves <= MAX_SOL_RESERVES {
                filtered_pools.push(pool);
            } else {
                filtered_out_count += 1;
                if is_debug_pool_discovery_enabled() {
                    log(
                        LogTag::PoolDiscovery,
                        "DEBUG",
                        &format!(
                            "Filtered out pool {} - estimated SOL reserves {:.2} outside bounds {:.1}-{:.0}",
                            pool.pool_id,
                            estimated_sol_reserves,
                            MIN_SOL_RESERVES,
                            MAX_SOL_RESERVES
                        )
                    );
                }
            }
        }

        if is_debug_pool_discovery_enabled() && filtered_out_count > 0 {
            log(
                LogTag::PoolDiscovery,
                "INFO",
                &format!(
                    "SOL reserves filter: {} pools passed, {} pools filtered out",
                    filtered_pools.len(),
                    filtered_out_count
                )
            );
        }

        filtered_pools
    }
}
