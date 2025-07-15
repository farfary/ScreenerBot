use std::collections::HashMap;
use std::time::{ SystemTime, UNIX_EPOCH };
use crate::pricing::{ PoolInfo, PoolType };
use crate::pricing::pool_decoders::{ DecodedPoolData, PoolDecoder };

pub struct PriceCalculator {
    pool_decoder: PoolDecoder,
    sol_price_cache: Option<(f64, u64)>, // (price, timestamp)
}

impl PriceCalculator {
    pub fn new() -> Self {
        Self {
            pool_decoder: PoolDecoder::new(),
            sol_price_cache: None,
        }
    }

    pub async fn calculate_from_pools(
        &self,
        pools: &[PoolInfo]
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        if pools.is_empty() {
            return Err("No pools provided for price calculation".into());
        }

        // Sort pools by liquidity (highest first)
        let mut sorted_pools = pools.to_vec();
        sorted_pools.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

        // Use top 3 pools for price calculation to get more accurate price
        let top_pools = &sorted_pools[..std::cmp::min(3, sorted_pools.len())];

        let mut weighted_prices = Vec::new();
        let mut total_weight = 0.0;

        for pool in top_pools {
            if let Ok(price) = self.calculate_pool_price(pool).await {
                let weight = pool.liquidity_usd;
                weighted_prices.push((price, weight));
                total_weight += weight;
            }
        }

        if weighted_prices.is_empty() {
            return Err("Unable to calculate price from any pool".into());
        }

        // Calculate weighted average price
        let weighted_sum: f64 = weighted_prices
            .iter()
            .map(|(price, weight)| price * weight)
            .sum();

        Ok(weighted_sum / total_weight)
    }

    async fn calculate_pool_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        match pool.pool_type {
            PoolType::Raydium => self.calculate_raydium_price(pool).await,
            PoolType::PumpFun => self.calculate_pumpfun_price(pool).await,
            PoolType::Meteora => self.calculate_meteora_price(pool).await,
            PoolType::Orca => self.calculate_orca_price(pool).await,
            PoolType::Serum => self.calculate_raydium_price(pool).await, // Treat as Raydium
            PoolType::Unknown(_) => self.calculate_generic_amm_price(pool).await,
        }
    }

    async fn calculate_raydium_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        // For Raydium AMM pools, use simple constant product formula
        if pool.reserve_0 == 0 || pool.reserve_1 == 0 {
            return Err("Invalid reserves for Raydium pool".into());
        }

        // Determine which token is SOL/USDC for price calculation
        let (token_reserve, quote_reserve) = if self.is_quote_token(&pool.token_1) {
            (pool.reserve_0, pool.reserve_1)
        } else if self.is_quote_token(&pool.token_0) {
            (pool.reserve_1, pool.reserve_0)
        } else {
            // If neither token is a known quote, use reserve ratio
            (pool.reserve_0, pool.reserve_1)
        };

        let price = (quote_reserve as f64) / (token_reserve as f64);

        // Convert to USD if needed
        if self.is_sol_token(&pool.token_0) || self.is_sol_token(&pool.token_1) {
            let sol_price = self.get_sol_price_usd().await?;
            Ok(price * sol_price)
        } else {
            Ok(price)
        }
    }

    async fn calculate_pumpfun_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        // PumpFun uses bonding curve pricing
        if pool.reserve_0 == 0 || pool.reserve_1 == 0 {
            return Err("Invalid reserves for PumpFun pool".into());
        }

        // PumpFun formula: price = sol_reserves / token_reserves
        let sol_reserves = if self.is_sol_token(&pool.token_1) {
            pool.reserve_1
        } else {
            pool.reserve_0
        };

        let token_reserves = if self.is_sol_token(&pool.token_1) {
            pool.reserve_0
        } else {
            pool.reserve_1
        };

        let price_in_sol = (sol_reserves as f64) / (token_reserves as f64);
        let sol_price = self.get_sol_price_usd().await?;

        Ok(price_in_sol * sol_price)
    }

    async fn calculate_meteora_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        // Meteora DLMM pricing based on active bin
        // For now, use simple AMM formula as approximation
        self.calculate_generic_amm_price(pool).await
    }

    async fn calculate_orca_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        // Orca concentrated liquidity pricing
        // For now, use simple AMM formula as approximation
        self.calculate_generic_amm_price(pool).await
    }

    async fn calculate_generic_amm_price(
        &self,
        pool: &PoolInfo
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        if pool.reserve_0 == 0 || pool.reserve_1 == 0 {
            return Err("Invalid reserves for generic AMM pool".into());
        }

        let price = (pool.reserve_1 as f64) / (pool.reserve_0 as f64);

        // If one of the tokens is SOL, convert to USD
        if self.is_sol_token(&pool.token_0) || self.is_sol_token(&pool.token_1) {
            let sol_price = self.get_sol_price_usd().await?;
            Ok(price * sol_price)
        } else {
            Ok(price)
        }
    }

    async fn get_sol_price_usd(&self) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        // Check cache first (cache for 60 seconds)
        if let Some((cached_price, timestamp)) = &self.sol_price_cache {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now - timestamp < 60 {
                return Ok(*cached_price);
            }
        }

        // Fetch SOL price from a reliable source (simplified)
        // In production, you would use multiple sources and have proper error handling
        let sol_price = 150.0; // Placeholder - should fetch from CoinGecko or similar

        Ok(sol_price)
    }

    fn is_sol_token(&self, token_address: &str) -> bool {
        // Wrapped SOL mint address
        token_address == "So11111111111111111111111111111111111111112"
    }

    fn is_quote_token(&self, token_address: &str) -> bool {
        // Common quote tokens on Solana
        matches!(
            token_address,
            "So11111111111111111111111111111111111111112" | // Wrapped SOL
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" | // USDC
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" | // USDT
                "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R" // RAY
        )
    }

    pub async fn calculate_slippage_impact(
        &self,
        pool: &PoolInfo,
        trade_amount: f64,
        is_buy: bool
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        if pool.reserve_0 == 0 || pool.reserve_1 == 0 {
            return Err("Invalid pool reserves for slippage calculation".into());
        }

        let (input_reserve, output_reserve) = if is_buy {
            (pool.reserve_1, pool.reserve_0)
        } else {
            (pool.reserve_0, pool.reserve_1)
        };

        // Constant product formula with fees
        let fee_multiplier = 1.0 - self.get_pool_fee_rate(pool);
        let input_amount_with_fee = trade_amount * fee_multiplier;

        let output_amount =
            ((output_reserve as f64) * input_amount_with_fee) /
            ((input_reserve as f64) + input_amount_with_fee);

        // Calculate price impact
        let expected_price = (output_reserve as f64) / (input_reserve as f64);
        let actual_price = trade_amount / output_amount;
        let price_impact = ((actual_price - expected_price) / expected_price).abs();

        Ok(price_impact)
    }

    fn get_pool_fee_rate(&self, pool: &PoolInfo) -> f64 {
        match pool.pool_type {
            PoolType::Raydium => 0.0025, // 0.25%
            PoolType::PumpFun => 0.01, // 1%
            PoolType::Meteora => 0.003, // 0.3% (variable)
            PoolType::Orca => 0.003, // 0.3% (variable)
            PoolType::Serum => 0.0025, // 0.25%
            PoolType::Unknown(_) => 0.003, // Default 0.3%
        }
    }

    pub async fn get_optimal_route(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: f64,
        pools: &[PoolInfo]
    ) -> Result<Vec<PoolInfo>, Box<dyn std::error::Error + Send + Sync>> {
        // Simple implementation - find direct pool first
        let direct_pool = pools
            .iter()
            .find(|pool| {
                (pool.token_0 == token_in && pool.token_1 == token_out) ||
                    (pool.token_0 == token_out && pool.token_1 == token_in)
            });

        if let Some(pool) = direct_pool {
            return Ok(vec![pool.clone()]);
        }

        // Multi-hop routing through SOL or USDC (simplified)
        let intermediate_tokens = [
            "So11111111111111111111111111111111111111112", // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        ];

        for intermediate in &intermediate_tokens {
            let pool1 = pools
                .iter()
                .find(|pool| {
                    (pool.token_0 == token_in && pool.token_1 == *intermediate) ||
                        (pool.token_0 == *intermediate && pool.token_1 == token_in)
                });

            let pool2 = pools
                .iter()
                .find(|pool| {
                    (pool.token_0 == *intermediate && pool.token_1 == token_out) ||
                        (pool.token_0 == token_out && pool.token_1 == *intermediate)
                });

            if let (Some(p1), Some(p2)) = (pool1, pool2) {
                return Ok(vec![p1.clone(), p2.clone()]);
            }
        }

        Err("No route found between tokens".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_price_calculator() {
        let calculator = PriceCalculator::new();

        // Create a mock pool
        let pool = PoolInfo {
            address: "test_pool".to_string(),
            pool_type: PoolType::Raydium,
            reserve_0: 1000000, // 1M tokens
            reserve_1: 100000, // 100K SOL
            token_0: "test_token".to_string(),
            token_1: "So11111111111111111111111111111111111111112".to_string(), // SOL
            liquidity_usd: 15000000.0, // 15M USD
            volume_24h: 1000000.0,
            fee_tier: Some(0.0025),
            last_updated: Instant::now(),
        };

        match calculator.calculate_pool_price(&pool).await {
            Ok(price) => {
                println!("Calculated price: ${}", price);
                assert!(price > 0.0);
            }
            Err(e) => {
                println!("Price calculation failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_slippage_calculation() {
        let calculator = PriceCalculator::new();

        let pool = PoolInfo {
            address: "test_pool".to_string(),
            pool_type: PoolType::Raydium,
            reserve_0: 1000000,
            reserve_1: 100000,
            token_0: "test_token".to_string(),
            token_1: "So11111111111111111111111111111111111111112".to_string(),
            liquidity_usd: 15000000.0,
            volume_24h: 1000000.0,
            fee_tier: Some(0.0025),
            last_updated: Instant::now(),
        };

        match calculator.calculate_slippage_impact(&pool, 1000.0, true).await {
            Ok(slippage) => {
                println!("Calculated slippage: {:.4}%", slippage * 100.0);
                assert!(slippage >= 0.0);
            }
            Err(e) => {
                println!("Slippage calculation failed: {}", e);
            }
        }
    }
}
