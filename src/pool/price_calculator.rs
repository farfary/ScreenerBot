use crate::pool::types::*;
use anyhow::{ Context, Result };

/// Price calculator for different pool types
pub struct PriceCalculator;

impl PriceCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate token price from pool reserves
    pub async fn calculate_price(
        &self,
        pool_type: &PoolType,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        match pool_type {
            PoolType::Raydium =>
                self.calculate_raydium_price(reserves, target_token, base_token, quote_token).await,
            PoolType::Orca =>
                self.calculate_orca_price(reserves, target_token, base_token, quote_token).await,
            PoolType::Meteora =>
                self.calculate_meteora_price(reserves, target_token, base_token, quote_token).await,
            PoolType::PumpFun =>
                self.calculate_pumpfun_price(reserves, target_token, base_token, quote_token).await,
            PoolType::Serum =>
                self.calculate_serum_price(reserves, target_token, base_token, quote_token).await,
            PoolType::Jupiter =>
                Err(anyhow::anyhow!("Jupiter is an aggregator, not a pool protocol")),
            PoolType::Unknown => Err(anyhow::anyhow!("Unknown pool type")),
        }
    }

    /// Calculate price for Raydium AMM pools
    async fn calculate_raydium_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Standard AMM formula: price = quote_reserve / base_reserve
        if reserves.base_token_amount == 0 {
            return Err(anyhow::anyhow!("Base token reserve is zero"));
        }

        let price = if target_token == base_token {
            // Price of base token in terms of quote token
            (reserves.quote_token_amount as f64) / (reserves.base_token_amount as f64)
        } else if target_token == quote_token {
            // Price of quote token in terms of base token
            (reserves.base_token_amount as f64) / (reserves.quote_token_amount as f64)
        } else {
            return Err(anyhow::anyhow!("Target token not found in pool"));
        };

        Ok(price)
    }

    /// Calculate price for Orca whirlpool
    async fn calculate_orca_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Orca uses concentrated liquidity, but for simplicity we'll use the same formula
        // In reality, you'd need to calculate based on current tick and liquidity
        self.calculate_raydium_price(reserves, target_token, base_token, quote_token).await
    }

    /// Calculate price for Meteora DLMM
    async fn calculate_meteora_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Meteora uses dynamic liquidity market maker
        // This is a simplified calculation
        self.calculate_raydium_price(reserves, target_token, base_token, quote_token).await
    }

    /// Calculate price for Pump.fun bonding curve
    async fn calculate_pumpfun_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Pump.fun uses a bonding curve formula
        // Price increases as more tokens are bought
        if reserves.base_token_amount == 0 {
            return Err(anyhow::anyhow!("Token reserve is zero"));
        }

        // Simplified bonding curve calculation
        let virtual_sol_reserves = 30_000_000_000u64; // 30 SOL virtual reserves
        let virtual_token_reserves = 1_073_000_000_000_000u64; // 1.073M token virtual reserves

        let total_sol_reserves = virtual_sol_reserves + reserves.quote_token_amount;
        let total_token_reserves = virtual_token_reserves - reserves.base_token_amount;

        if total_token_reserves == 0 {
            return Err(anyhow::anyhow!("Total token reserves is zero"));
        }

        let price = (total_sol_reserves as f64) / (total_token_reserves as f64);

        // Adjust for decimals (SOL: 9, Token: 6)
        let adjusted_price = price * 1000.0; // 10^(9-6)

        Ok(adjusted_price)
    }

    /// Calculate price for Serum orderbook
    async fn calculate_serum_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Serum is an orderbook, not an AMM
        // Price would come from the best bid/ask
        // This is a placeholder implementation
        self.calculate_raydium_price(reserves, target_token, base_token, quote_token).await
    }

    /// Calculate price with slippage consideration
    pub async fn calculate_price_with_slippage(
        &self,
        pool_type: &PoolType,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        match pool_type {
            PoolType::Raydium => {
                self.calculate_raydium_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::PumpFun => {
                self.calculate_pumpfun_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            _ => {
                // For other pool types, use basic calculation
                self.calculate_price(
                    pool_type,
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await
            }
        }
    }

    /// Calculate Raydium price with slippage
    async fn calculate_raydium_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        // AMM formula with slippage: new_price = (quote_reserve - quote_out) / (base_reserve + base_in)
        let k = (reserves.base_token_amount as f64) * (reserves.quote_token_amount as f64);

        if target_token == base_token {
            // Buying base token with quote token
            let new_base_reserve = (reserves.base_token_amount as f64) - (trade_amount as f64);
            let new_quote_reserve = k / new_base_reserve;
            let quote_needed = new_quote_reserve - (reserves.quote_token_amount as f64);

            Ok(quote_needed / (trade_amount as f64))
        } else if target_token == quote_token {
            // Buying quote token with base token
            let new_quote_reserve = (reserves.quote_token_amount as f64) - (trade_amount as f64);
            let new_base_reserve = k / new_quote_reserve;
            let base_needed = new_base_reserve - (reserves.base_token_amount as f64);

            Ok(base_needed / (trade_amount as f64))
        } else {
            Err(anyhow::anyhow!("Target token not found in pool"))
        }
    }

    /// Calculate Pump.fun price with slippage
    async fn calculate_pumpfun_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        // Bonding curve with slippage
        // This is a simplified implementation
        let base_price = self.calculate_pumpfun_price(
            reserves,
            target_token,
            base_token,
            quote_token
        ).await?;

        // Add slippage based on trade size
        let slippage_factor =
            1.0 + ((trade_amount as f64) / (reserves.base_token_amount as f64)) * 0.01;

        Ok(base_price * slippage_factor)
    }

    /// Get USD price for a token (requires SOL price)
    pub async fn get_usd_price(&self, token_price_in_sol: f64, sol_price_usd: f64) -> f64 {
        token_price_in_sol * sol_price_usd
    }

    /// Calculate market impact for a trade
    pub async fn calculate_market_impact(
        &self,
        pool_type: &PoolType,
        reserves: &PoolReserve,
        trade_amount: u64,
        token_decimals: u8
    ) -> Result<f64> {
        let trade_amount_adjusted = (trade_amount as f64) / (10_f64).powi(token_decimals as i32);
        let total_liquidity = (reserves.base_token_amount + reserves.quote_token_amount) as f64;

        // Market impact as percentage of total liquidity
        let impact = (trade_amount_adjusted / total_liquidity) * 100.0;

        Ok(impact)
    }
}

impl Default for PriceCalculator {
    fn default() -> Self {
        Self::new()
    }
}
