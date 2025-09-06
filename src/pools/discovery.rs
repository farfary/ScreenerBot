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
use super::types::{ PoolDescriptor, ProgramKind };
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
        self.deduplicate_pools(discovered_pools)
    }

    /// Discover pools from DexScreener API
    async fn discover_from_dexscreener(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let token_pairs = get_token_pools_from_dexscreener(mint).await?;

        let mut pools = Vec::new();
        for pair in token_pairs {
            if let Ok(pool_descriptor) = self.convert_dexscreener_pair_to_descriptor(&pair, mint) {
                pools.push(pool_descriptor);
            }
        }

        Ok(pools)
    }

    /// Discover pools from GeckoTerminal API
    async fn discover_from_geckoterminal(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let gecko_pools = get_token_pools_from_geckoterminal(mint).await?;

        let mut pools = Vec::new();
        for pool in gecko_pools {
            if let Ok(pool_descriptor) = self.convert_geckoterminal_pool_to_descriptor(&pool, mint) {
                pools.push(pool_descriptor);
            }
        }

        Ok(pools)
    }

    /// Discover pools from Raydium API
    async fn discover_from_raydium(&self, mint: &str) -> Result<Vec<PoolDescriptor>, String> {
        let raydium_pools = get_token_pools_from_raydium(mint).await?;

        let mut pools = Vec::new();
        for pool in raydium_pools {
            if let Ok(pool_descriptor) = self.convert_raydium_pool_to_descriptor(&pool, mint) {
                pools.push(pool_descriptor);
            }
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
}
