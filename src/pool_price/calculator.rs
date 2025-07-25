/// Pool Price Calculator Module
///
/// This module calculates token prices from decoded pool reserve data.
/// It handles weighted averages, validation, and price confidence scoring.

use super::types::*;
use crate::logger::{ log, LogTag };

use std::time::Instant;

// =============================================================================
// PRICE CALCULATOR
// =============================================================================

pub struct PriceCalculator;

impl PriceCalculator {
    /// Calculate token price from decoded pool data
    pub fn calculate_token_price(
        mint: &str,
        decoded_pools: &[DecodedPoolData]
    ) -> PoolPriceResult<Option<CalculatedPrice>> {
        let valid_pools: Vec<&DecodedPoolData> = decoded_pools
            .iter()
            .filter(|pool| pool.is_valid)
            .collect();

        if valid_pools.is_empty() {
            log(LogTag::Pool, "WARN", &format!("No valid pools found for token {}", mint));
            return Ok(None);
        }

        log(
            LogTag::Pool,
            "CALC",
            &format!("Calculating price for {} using {} valid pools", mint, valid_pools.len())
        );

        // Calculate individual prices from each pool
        let mut pool_prices = Vec::new();
        let mut source_pools = Vec::new();

        for pool in valid_pools {
            if let Some(price) = Self::calculate_pool_price(mint, pool) {
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Pool {} price: {:.12} SOL (liquidity: ${:.2})",
                        pool.address,
                        price,
                        pool.liquidity_usd
                    )
                );

                pool_prices.push((price, pool.liquidity_usd));
                source_pools.push(pool.address.clone());
            }
        }

        if pool_prices.is_empty() {
            log(LogTag::Pool, "WARN", &format!("No calculable prices found for token {}", mint));
            return Ok(None);
        }

        // Calculate weighted average price based on liquidity
        let final_price = if pool_prices.len() == 1 {
            log(
                LogTag::Pool,
                "INFO",
                &format!("Using single pool price for {}: {:.12} SOL", mint, pool_prices[0].0)
            );
            pool_prices[0].0
        } else {
            let weighted_price = Self::calculate_weighted_average(&pool_prices);
            log(
                LogTag::Pool,
                "INFO",
                &format!(
                    "Calculated weighted average price for {} from {} pools: {:.12} SOL",
                    mint,
                    pool_prices.len(),
                    weighted_price
                )
            );
            weighted_price
        };

        // Validate the calculated price
        if !Self::validate_price(final_price) {
            log(
                LogTag::Pool,
                "ERROR",
                &format!("Invalid calculated price for {}: {:.12}", mint, final_price)
            );
            return Err(PoolPriceError::PriceCalculation("Invalid price calculated".to_string()));
        }

        // Calculate confidence score
        let confidence = Self::calculate_confidence(&pool_prices);

        let calculated_price = CalculatedPrice {
            mint: mint.to_string(),
            price_sol: final_price,
            price_usd: None, // TODO: Convert using SOL/USD rate
            source_pools,
            weighted_average: pool_prices.len() > 1,
            confidence,
            timestamp: Instant::now(),
        };

        log(
            LogTag::Pool,
            "SUCCESS",
            &format!(
                "Final price for {}: {:.12} SOL (confidence: {:.2})",
                mint,
                final_price,
                confidence
            )
        );

        Ok(Some(calculated_price))
    }

    /// Calculate price from a single pool's reserve data
    fn calculate_pool_price(mint: &str, pool: &DecodedPoolData) -> Option<f64> {
        // Determine which token is our target and which is the quote
        let (target_reserve, quote_reserve, target_decimals, quote_decimals, quote_mint) = if
            pool.token_a_mint == mint
        {
            // Our token is token A, quote is token B
            (
                pool.token_a_reserve,
                pool.token_b_reserve,
                pool.token_a_decimals,
                pool.token_b_decimals,
                &pool.token_b_mint,
            )
        } else if pool.token_b_mint == mint {
            // Our token is token B, quote is token A
            (
                pool.token_b_reserve,
                pool.token_a_reserve,
                pool.token_b_decimals,
                pool.token_a_decimals,
                &pool.token_a_mint,
            )
        } else {
            log(
                LogTag::Pool,
                "ERROR",
                &format!(
                    "Token {} not found in pool {} (tokens: {}, {})",
                    mint,
                    pool.address,
                    pool.token_a_mint,
                    pool.token_b_mint
                )
            );
            return None;
        };

        // Validate reserves
        if target_reserve == 0 || quote_reserve == 0 {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "Zero reserves in pool {}: target={}, quote={}",
                    pool.address,
                    target_reserve,
                    quote_reserve
                )
            );
            return None;
        }

        // Check if quote token is SOL/WSOL
        if quote_mint != SOL_MINT && quote_mint != WSOL_MINT {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!("Pool {} has non-SOL quote token: {}, skipping", pool.address, quote_mint)
            );
            return None;
        }

        // Calculate price: (quote_reserve / 10^quote_decimals) / (target_reserve / 10^target_decimals)
        let target_ui_amount = (target_reserve as f64) / (10_f64).powi(target_decimals as i32);
        let quote_ui_amount = (quote_reserve as f64) / (10_f64).powi(quote_decimals as i32);

        if target_ui_amount <= 0.0 || quote_ui_amount <= 0.0 {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "Invalid UI amounts for pool {}: target={:.12}, quote={:.12}",
                    pool.address,
                    target_ui_amount,
                    quote_ui_amount
                )
            );
            return None;
        }

        let price = quote_ui_amount / target_ui_amount;

        log(
            LogTag::Pool,
            "DEBUG",
            &format!(
                "Pool {} calculation: {:.2} SOL / {:.2} {} = {:.12} SOL per token",
                pool.address,
                quote_ui_amount,
                target_ui_amount,
                mint,
                price
            )
        );

        Some(price)
    }

    /// Calculate weighted average price based on liquidity
    fn calculate_weighted_average(pool_prices: &[(f64, f64)]) -> f64 {
        let total_liquidity: f64 = pool_prices
            .iter()
            .map(|(_, liquidity)| liquidity)
            .sum();

        if total_liquidity <= 0.0 {
            // Fallback to simple average if no liquidity data
            return (
                pool_prices
                    .iter()
                    .map(|(price, _)| price)
                    .sum::<f64>() / (pool_prices.len() as f64)
            );
        }

        let weighted_sum: f64 = pool_prices
            .iter()
            .map(|(price, liquidity)| price * (liquidity / total_liquidity))
            .sum();

        weighted_sum
    }

    /// Calculate confidence score for the price calculation
    fn calculate_confidence(pool_prices: &[(f64, f64)]) -> f64 {
        if pool_prices.len() == 1 {
            // Single pool confidence based on liquidity
            let liquidity = pool_prices[0].1;
            if liquidity >= 10000.0 {
                0.9
            } else if liquidity >= 5000.0 {
                0.8
            } else if liquidity >= 1000.0 {
                0.7
            } else if liquidity >= 500.0 {
                0.6
            } else {
                0.5
            }
        } else {
            // Multiple pools - check price consistency
            let prices: Vec<f64> = pool_prices
                .iter()
                .map(|(price, _)| *price)
                .collect();
            let avg_price = prices.iter().sum::<f64>() / (prices.len() as f64);

            // Calculate price deviation
            let max_deviation = prices
                .iter()
                .map(|price| (price - avg_price).abs() / avg_price)
                .fold(0.0, f64::max);

            // Higher confidence if prices are consistent
            let consistency_score = if max_deviation < 0.05 {
                0.95
            } else if max_deviation < 0.1 {
                0.85
            } else if max_deviation < 0.2 {
                0.75
            } else if max_deviation < 0.5 {
                0.6
            } else {
                0.4
            };

            // Bonus for multiple pools
            let pool_bonus = ((pool_prices.len() as f64) * 0.05).min(0.1);
            (consistency_score + pool_bonus).min(1.0)
        }
    }

    /// Validate that a calculated price is reasonable
    fn validate_price(price: f64) -> bool {
        // Basic sanity checks
        if price <= 0.0 || !price.is_finite() {
            return false;
        }

        // Check for extremely high or low prices (might indicate errors)
        if price > 1000.0 || price < 0.000000001 {
            return false;
        }

        true
    }

    /// Convert SOL price to USD using current SOL/USD rate
    pub fn convert_sol_to_usd(sol_price: f64, sol_usd_rate: f64) -> Option<f64> {
        if sol_price <= 0.0 || sol_usd_rate <= 0.0 {
            return None;
        }

        Some(sol_price * sol_usd_rate)
    }

    /// Compare calculated price with reference price (for validation)
    pub fn compare_with_reference(calculated: f64, reference: f64) -> (f64, bool) {
        if calculated <= 0.0 || reference <= 0.0 {
            return (0.0, false);
        }

        let deviation_percent = ((calculated - reference).abs() / reference) * 100.0;
        let is_within_tolerance = deviation_percent <= MAX_PRICE_DEVIATION_PERCENT;

        (deviation_percent, is_within_tolerance)
    }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Calculate token price from pool addresses (full pipeline)
pub async fn calculate_token_price_from_pools(
    mint: &str,
    pool_addresses: &[PoolAddressInfo]
) -> PoolPriceResult<Option<CalculatedPrice>> {
    if pool_addresses.is_empty() {
        return Ok(None);
    }

    // Fetch and decode pool data
    let decoded_pools = super::decoder::fetch_and_decode_pools(pool_addresses).await?;

    // Calculate price from decoded data
    PriceCalculator::calculate_token_price(mint, &decoded_pools)
}

/// Calculate price with reference comparison
pub async fn calculate_and_validate_price(
    mint: &str,
    pool_addresses: &[PoolAddressInfo],
    reference_price: Option<f64>
) -> PoolPriceResult<Option<CalculatedPrice>> {
    let calculated_price = calculate_token_price_from_pools(mint, pool_addresses).await?;

    if let (Some(calc_price), Some(ref_price)) = (&calculated_price, reference_price) {
        let (deviation, is_valid) = PriceCalculator::compare_with_reference(
            calc_price.price_sol,
            ref_price
        );

        log(
            LogTag::Pool,
            "VALIDATION",
            &format!(
                "Price comparison for {}: calculated={:.12}, reference={:.12}, deviation={:.2}%, valid={}",
                mint,
                calc_price.price_sol,
                ref_price,
                deviation,
                is_valid
            )
        );

        if !is_valid {
            log(
                LogTag::Pool,
                "WARN",
                &format!(
                    "Price deviation too high for {}: {:.2}% (max allowed: {:.1}%)",
                    mint,
                    deviation,
                    MAX_PRICE_DEVIATION_PERCENT
                )
            );
        }
    }

    Ok(calculated_price)
}
