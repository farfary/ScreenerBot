use crate::pairs::{ PoolDataFetcher, PoolInfo, PriceInfo, program_ids, PoolType };
use crate::rpc::RpcManager;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use log::{ info, error };

/// Example usage of the pool decoders and fetcher
pub struct PoolAnalyzer {
    pool_fetcher: PoolDataFetcher,
}

impl PoolAnalyzer {
    /// Create a new pool analyzer
    pub fn new(rpc_manager: Arc<RpcManager>) -> Self {
        let pool_fetcher = PoolDataFetcher::new(rpc_manager);

        Self {
            pool_fetcher,
        }
    }

    /// Analyze a specific pool by address
    pub async fn analyze_pool(&self, pool_address_str: &str) -> Result<PoolAnalysis> {
        let pool_address = Pubkey::from_str(pool_address_str)?;

        info!("🔍 Analyzing pool: {}", pool_address);

        // Fetch pool data
        let pool_info = self.pool_fetcher.fetch_pool_data(&pool_address).await?;

        // Calculate price
        let price_info = self.pool_fetcher.get_price_info(&pool_info)?;

        // Calculate additional metrics
        let tvl = calculate_pool_tvl(&pool_info);
        let utilization = calculate_liquidity_utilization(&pool_info);
        let health_score = calculate_health_score(&pool_info);

        let analysis = PoolAnalysis {
            pool_info,
            price_info,
            tvl,
            liquidity_utilization: utilization,
            health_score,
        };

        info!("✅ Pool analysis complete");
        info!("   Type: {:?}", analysis.pool_info.pool_type);
        info!("   Price: {:.6}", analysis.price_info.price);
        info!("   TVL: {:.2}", analysis.tvl);
        info!("   Health Score: {:.2}", analysis.health_score);

        Ok(analysis)
    }

    /// Analyze multiple pools and compare them
    pub async fn compare_pools(&self, pool_addresses: Vec<&str>) -> Result<Vec<PoolComparison>> {
        let mut comparisons = Vec::new();

        for pool_address_str in pool_addresses {
            match self.analyze_pool(pool_address_str).await {
                Ok(analysis) => {
                    let comparison = PoolComparison {
                        address: pool_address_str.to_string(),
                        pool_type: analysis.pool_info.pool_type,
                        price: analysis.price_info.price,
                        tvl: analysis.tvl,
                        health_score: analysis.health_score,
                        liquidity: analysis.pool_info.liquidity,
                    };
                    comparisons.push(comparison);
                }
                Err(e) => {
                    error!("Failed to analyze pool {}: {}", pool_address_str, e);
                }
            }
        }

        // Sort by TVL descending
        comparisons.sort_by(|a, b| b.tvl.partial_cmp(&a.tvl).unwrap_or(std::cmp::Ordering::Equal));

        Ok(comparisons)
    }

    /// Find the best pools for a token pair
    pub async fn find_best_pools_for_tokens(
        &self,
        token_0: &str,
        token_1: &str
    ) -> Result<Vec<PoolComparison>> {
        // This would require implementing pool scanning/indexing
        // For now, return empty vec as placeholder
        info!("🔍 Searching for pools with tokens {} and {}", token_0, token_1);
        Ok(Vec::new())
    }

    /// Get supported pool types
    pub fn get_supported_pool_types(&self) -> Vec<String> {
        self.pool_fetcher
            .get_supported_programs()
            .iter()
            .map(|program_id| {
                if *program_id == program_ids::raydium_clmm() {
                    "Raydium CLMM".to_string()
                } else if *program_id == program_ids::meteora_dlmm() {
                    "Meteora DLMM".to_string()
                } else if *program_id == program_ids::whirlpool() {
                    "Whirlpool".to_string()
                } else if *program_id == program_ids::pump_fun_amm() {
                    "Pump.fun AMM".to_string()
                } else {
                    format!("Unknown ({})", program_id)
                }
            })
            .collect()
    }
}

/// Complete analysis of a pool
#[derive(Debug, Clone)]
pub struct PoolAnalysis {
    pub pool_info: PoolInfo,
    pub price_info: PriceInfo,
    pub tvl: f64,
    pub liquidity_utilization: f64,
    pub health_score: f64,
}

/// Pool comparison data
#[derive(Debug, Clone)]
pub struct PoolComparison {
    pub address: String,
    pub pool_type: PoolType,
    pub price: f64,
    pub tvl: f64,
    pub health_score: f64,
    pub liquidity: Option<u128>,
}

/// Calculate total value locked in the pool
fn calculate_pool_tvl(pool_info: &PoolInfo) -> f64 {
    let reserve_0_adjusted =
        (pool_info.reserve_0 as f64) / (10_f64).powi(pool_info.decimals_0 as i32);
    let reserve_1_adjusted =
        (pool_info.reserve_1 as f64) / (10_f64).powi(pool_info.decimals_1 as i32);

    // Simple TVL calculation (would need USD prices for accurate TVL)
    match pool_info.calculate_price() {
        Ok(price) => reserve_1_adjusted + reserve_0_adjusted * price,
        Err(_) => reserve_1_adjusted + reserve_0_adjusted, // Fallback
    }
}

/// Calculate liquidity utilization (active liquidity vs total)
fn calculate_liquidity_utilization(pool_info: &PoolInfo) -> f64 {
    match pool_info.pool_type {
        PoolType::RaydiumClmm | PoolType::Whirlpool => {
            // For concentrated liquidity pools, this would require
            // calculating active liquidity in the current price range
            if let Some(liquidity) = pool_info.liquidity {
                if liquidity > 0 {
                    0.7
                } else {
                    0.0
                } // Placeholder
            } else {
                0.0
            }
        }
        PoolType::MeteoraDlmm => {
            // For DLMM, utilization depends on active bins
            0.8 // Placeholder
        }
        PoolType::PumpFunAmm => {
            // Pump.fun AMM is constant product, full utilization
            1.0
        }
        _ => 1.0, // Full utilization for constant product pools
    }
}

/// Calculate a health score for the pool
fn calculate_health_score(pool_info: &PoolInfo) -> f64 {
    let mut score = 50.0; // Base score

    // Add points for liquidity
    if let Some(liquidity) = pool_info.liquidity {
        if liquidity > 1_000_000 {
            score += 20.0;
        } else if liquidity > 100_000 {
            score += 10.0;
        }
    }

    // Add points for reserves
    if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
        score += 15.0;
    }

    // Add points for active status
    match pool_info.status {
        crate::pairs::PoolStatus::Active => {
            score += 15.0;
        }
        crate::pairs::PoolStatus::Paused => {
            score -= 10.0;
        }
        crate::pairs::PoolStatus::Inactive => {
            score -= 20.0;
        }
        crate::pairs::PoolStatus::Unknown => {
            score -= 5.0;
        }
    }

    // Cap score at 100
    (score as f64).min(100.0)
}

/// Example usage function
pub async fn example_usage() -> Result<()> {
    use crate::config::RpcConfig;

    // Initialize RPC manager (this would typically be done at startup)
    let primary_url = "https://api.mainnet-beta.solana.com".to_string();
    let fallback_urls = vec!["https://solana-api.projectserum.com".to_string()];
    let rpc_config = RpcConfig::default();

    let rpc_manager = Arc::new(RpcManager::new(primary_url, fallback_urls, rpc_config)?);

    // Create pool analyzer
    let analyzer = PoolAnalyzer::new(rpc_manager);

    // Example pool addresses (these should be real pool addresses)
    let pool_addresses = vec![
        "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj", // Example Raydium CLMM pool
        "2QdhepnKRTLjjSqPL1PtKNwqrUkoLee5Gqs8bvZhRdMv" // Example Meteora DLMM pool
    ];

    info!("🚀 Starting pool analysis example");
    info!("Supported pool types: {:?}", analyzer.get_supported_pool_types());

    // Analyze individual pool
    if let Some(pool_address) = pool_addresses.first() {
        match analyzer.analyze_pool(pool_address).await {
            Ok(_analysis) => {
                info!("Individual pool analysis successful");
            }
            Err(e) => {
                error!("Individual pool analysis failed: {}", e);
            }
        }
    }

    // Compare multiple pools
    match analyzer.compare_pools(pool_addresses).await {
        Ok(comparisons) => {
            info!("Pool comparison completed with {} pools", comparisons.len());
            for comparison in comparisons {
                info!(
                    "  {} - Type: {:?}, TVL: {:.2}, Score: {:.1}",
                    comparison.address,
                    comparison.pool_type,
                    comparison.tvl,
                    comparison.health_score
                );
            }
        }
        Err(e) => {
            error!("Pool comparison failed: {}", e);
        }
    }

    Ok(())
}
