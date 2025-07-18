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
            PoolType::MeteoraDynamic =>
                self.calculate_meteora_dynamic_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::PumpFunAmm =>
                self.calculate_pumpfun_amm_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::RaydiumAmmV4 =>
                self.calculate_raydium_amm_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::RaydiumAmmV5 =>
                self.calculate_raydium_amm_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::RaydiumClmm =>
                self.calculate_raydium_clmm_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::RaydiumCpmm =>
                self.calculate_raydium_cpmm_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::RaydiumStableSwap =>
                self.calculate_raydium_stable_swap_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::OrcaWhirlpool =>
                self.calculate_orca_whirlpool_price(
                    reserves,
                    target_token,
                    base_token,
                    quote_token
                ).await,
            PoolType::Unknown => Err(anyhow::anyhow!("Unknown pool type")),
        }
    }

    /// Calculate price for Meteora Dynamic pools
    async fn calculate_meteora_dynamic_price(
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

    /// Calculate price for Pump.fun AMM pools
    async fn calculate_pumpfun_amm_price(
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

    /// Calculate price for Orca Whirlpool pools
    async fn calculate_orca_whirlpool_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Orca Whirlpool uses concentrated liquidity like Raydium CLMM
        // For now, use basic AMM formula as fallback
        let base_amount = reserves.base_token_amount as f64;
        let quote_amount = reserves.quote_token_amount as f64;

        if base_amount == 0.0 || quote_amount == 0.0 {
            return Err(anyhow::anyhow!("Pool has no liquidity"));
        }

        // Simple price calculation - in production, this would need proper concentrated liquidity math
        let price = if target_token == base_token {
            quote_amount / base_amount
        } else {
            base_amount / quote_amount
        };

        Ok(price)
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
            PoolType::MeteoraDynamic => {
                self.calculate_meteora_dynamic_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::PumpFunAmm => {
                self.calculate_pumpfun_amm_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::RaydiumAmmV4 => {
                // Use basic AMM formula for Raydium pools
                self.calculate_raydium_amm_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::RaydiumAmmV5 => {
                // Use basic AMM formula for Raydium pools
                self.calculate_raydium_amm_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::RaydiumClmm => {
                // Use concentrated liquidity formula for CLMM
                self.calculate_raydium_clmm_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::RaydiumCpmm => {
                // Use constant product formula for CPMM
                self.calculate_raydium_cpmm_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::RaydiumStableSwap => {
                // Use stable swap formula
                self.calculate_raydium_stable_swap_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::OrcaWhirlpool => {
                // Use Orca Whirlpool formula
                self.calculate_orca_whirlpool_price_with_slippage(
                    reserves,
                    target_token,
                    base_token,
                    quote_token,
                    trade_amount
                ).await
            }
            PoolType::Unknown => Err(anyhow::anyhow!("Unknown pool type")),
        }
    }

    /// Calculate Meteora Dynamic price with slippage
    async fn calculate_meteora_dynamic_price_with_slippage(
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

    /// Calculate Pump.fun AMM price with slippage
    async fn calculate_pumpfun_amm_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        // AMM formula with slippage
        let base_price = self.calculate_pumpfun_amm_price(
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

    /// Calculate Raydium AMM price (V4/V5)
    async fn calculate_raydium_amm_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Standard AMM formula: price = quote_reserve / base_reserve
        if target_token == base_token {
            Ok((reserves.quote_token_amount as f64) / (reserves.base_token_amount as f64))
        } else if target_token == quote_token {
            Ok((reserves.base_token_amount as f64) / (reserves.quote_token_amount as f64))
        } else {
            Err(anyhow::anyhow!("Target token not found in pool"))
        }
    }

    /// Calculate Raydium CLMM price
    async fn calculate_raydium_clmm_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // For CLMM, we use the reserve ratio as an approximation
        if target_token == base_token {
            Ok((reserves.quote_token_amount as f64) / (reserves.base_token_amount as f64))
        } else if target_token == quote_token {
            Ok((reserves.base_token_amount as f64) / (reserves.quote_token_amount as f64))
        } else {
            Err(anyhow::anyhow!("Target token not found in pool"))
        }
    }

    /// Calculate Raydium CPMM price
    async fn calculate_raydium_cpmm_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Constant product formula: price = quote_reserve / base_reserve
        if target_token == base_token {
            Ok((reserves.quote_token_amount as f64) / (reserves.base_token_amount as f64))
        } else if target_token == quote_token {
            Ok((reserves.base_token_amount as f64) / (reserves.quote_token_amount as f64))
        } else {
            Err(anyhow::anyhow!("Target token not found in pool"))
        }
    }

    /// Calculate Raydium Stable Swap price
    async fn calculate_raydium_stable_swap_price(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str
    ) -> Result<f64> {
        // Stable swap pools should have price close to 1:1
        if target_token == base_token {
            Ok((reserves.quote_token_amount as f64) / (reserves.base_token_amount as f64))
        } else if target_token == quote_token {
            Ok((reserves.base_token_amount as f64) / (reserves.quote_token_amount as f64))
        } else {
            Err(anyhow::anyhow!("Target token not found in pool"))
        }
    }

    /// Calculate Raydium AMM price with slippage
    async fn calculate_raydium_amm_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        let base_price = self.calculate_raydium_amm_price(
            reserves,
            target_token,
            base_token,
            quote_token
        ).await?;
        let slippage_factor =
            1.0 + ((trade_amount as f64) / (reserves.base_token_amount as f64)) * 0.01;
        Ok(base_price * slippage_factor)
    }

    /// Calculate Raydium CLMM price with slippage
    async fn calculate_raydium_clmm_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        let base_price = self.calculate_raydium_clmm_price(
            reserves,
            target_token,
            base_token,
            quote_token
        ).await?;
        let slippage_factor =
            1.0 + ((trade_amount as f64) / (reserves.base_token_amount as f64)) * 0.005; // Lower slippage for CLMM
        Ok(base_price * slippage_factor)
    }

    /// Calculate Raydium CPMM price with slippage
    async fn calculate_raydium_cpmm_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        let base_price = self.calculate_raydium_cpmm_price(
            reserves,
            target_token,
            base_token,
            quote_token
        ).await?;
        let slippage_factor =
            1.0 + ((trade_amount as f64) / (reserves.base_token_amount as f64)) * 0.01;
        Ok(base_price * slippage_factor)
    }

    /// Calculate Raydium Stable Swap price with slippage
    async fn calculate_raydium_stable_swap_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        let base_price = self.calculate_raydium_stable_swap_price(
            reserves,
            target_token,
            base_token,
            quote_token
        ).await?;
        let slippage_factor =
            1.0 + ((trade_amount as f64) / (reserves.base_token_amount as f64)) * 0.001; // Very low slippage for stable swaps
        Ok(base_price * slippage_factor)
    }

    /// Calculate Orca Whirlpool price with slippage
    async fn calculate_orca_whirlpool_price_with_slippage(
        &self,
        reserves: &PoolReserve,
        target_token: &str,
        base_token: &str,
        quote_token: &str,
        trade_amount: u64
    ) -> Result<f64> {
        // For now, use basic AMM formula with slippage
        let base_amount = reserves.base_token_amount as f64;
        let quote_amount = reserves.quote_token_amount as f64;

        if base_amount == 0.0 || quote_amount == 0.0 {
            return Err(anyhow::anyhow!("Pool has no liquidity"));
        }

        let trade_amount_f = trade_amount as f64;

        // Simple constant product formula with slippage
        let price_with_slippage = if target_token == base_token {
            let new_base_amount = base_amount + trade_amount_f;
            let new_quote_amount = (base_amount * quote_amount) / new_base_amount;
            let quote_out = quote_amount - new_quote_amount;
            quote_out / trade_amount_f
        } else {
            let new_quote_amount = quote_amount + trade_amount_f;
            let new_base_amount = (base_amount * quote_amount) / new_quote_amount;
            let base_out = base_amount - new_base_amount;
            base_out / trade_amount_f
        };

        Ok(price_with_slippage)
    }
}

impl Default for PriceCalculator {
    fn default() -> Self {
        Self::new()
    }
}
