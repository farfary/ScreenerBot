/// Raydium CPMM pool decoder
///
/// This module handles decoding Raydium Constant Product Market Maker pools.
/// It extracts reserve data and calculates token prices.

use super::{ PoolDecoder, AccountData };
use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT };
use crate::tokens::decimals::get_cached_decimals;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

/// Raydium CPMM decoder implementation
pub struct RaydiumCpmmDecoder;

impl PoolDecoder for RaydiumCpmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumCpmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Decoding Raydium CPMM pool for {}/{}", base_mint, quote_mint)
            );
        }

        // Find the pool account (typically the first/main account)
        let pool_account = accounts.values().next()?;
        
        // Parse pool state from account data
        let pool_state = RaydiumCpmmPoolState::from_bytes(&pool_account.data)?;

        // Get token decimals
        let base_decimals = get_cached_decimals(base_mint).unwrap_or(9);
        let quote_decimals = get_cached_decimals(quote_mint).unwrap_or(9);

        // Calculate normalized reserves
        let base_reserve_normalized = pool_state.base_reserve as f64 / 10_f64.powi(base_decimals as i32);
        let quote_reserve_normalized = pool_state.quote_reserve as f64 / 10_f64.powi(quote_decimals as i32);

        // Determine which token is SOL and calculate SOL price
        let sol_mint_str = SOL_MINT;
        let (target_mint, price_sol, sol_reserves, token_reserves) = if base_mint == sol_mint_str {
            // Base is SOL, quote is the token we're pricing
            let token_price_in_sol = base_reserve_normalized / quote_reserve_normalized;
            (quote_mint.to_string(), token_price_in_sol, base_reserve_normalized, quote_reserve_normalized)
        } else if quote_mint == sol_mint_str {
            // Quote is SOL, base is the token we're pricing
            let token_price_in_sol = quote_reserve_normalized / base_reserve_normalized;
            (base_mint.to_string(), token_price_in_sol, quote_reserve_normalized, base_reserve_normalized)
        } else {
            // Neither is SOL - cannot calculate SOL price directly
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!("Pool {}/{} does not contain SOL - cannot calculate SOL price", base_mint, quote_mint)
                );
            }
            return None;
        };

        // Validate reserves are positive
        if sol_reserves <= 0.0 || token_reserves <= 0.0 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!("Invalid reserves for pool {}/{}: SOL={}, Token={}", base_mint, quote_mint, sol_reserves, token_reserves)
                );
            }
            return None;
        }

        // Validate price is reasonable (not zero or extremely high)
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!("Unreasonable price for {}: {} SOL", target_mint, price_sol)
                );
            }
            return None;
        }

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "SUCCESS",
                &format!(
                    "Decoded CPMM pool: {} = {} SOL (reserves: {} SOL, {} tokens)",
                    target_mint, price_sol, sol_reserves, token_reserves
                )
            );
        }

        Some(PriceResult::new(
            target_mint,
            0.0, // No USD calculation
            price_sol,
            sol_reserves,
            token_reserves,
            pool_account.pubkey.to_string(),
        ))
    }
}

/// Raydium CPMM pool state structure (simplified)
/// This is a basic representation - the actual structure may vary
#[repr(C)]
#[derive(Debug)]
pub struct RaydiumCpmmPoolState {
    pub discriminator: [u8; 8],
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    // Additional fields would be here in the real implementation
}

impl RaydiumCpmmPoolState {
    /// Parse pool state from raw account data
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }

        // This is a simplified parsing - real implementation would use proper deserialization
        // For now, we'll try to extract reserves from common positions in the account data
        
        // Try to parse as little-endian u64 values at different offsets
        // This is a heuristic approach until we have the exact layout
        
        let base_reserve = Self::try_parse_u64_at_offset(data, 8)?;
        let quote_reserve = Self::try_parse_u64_at_offset(data, 16)?;
        
        // Basic validation
        if base_reserve == 0 || quote_reserve == 0 {
            return None;
        }

        Some(RaydiumCpmmPoolState {
            discriminator: [0u8; 8],
            base_reserve,
            quote_reserve,
            base_decimals: 9, // Default SOL decimals
            quote_decimals: 9, // Default assumption
        })
    }

    /// Try to parse a u64 from the given offset
    fn try_parse_u64_at_offset(data: &[u8], offset: usize) -> Option<u64> {
        if data.len() < offset + 8 {
            return None;
        }
        
        let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }

    /// Calculate price based on reserves (legacy method)
    pub fn calculate_price(&self, base_decimals: u8, quote_decimals: u8) -> Option<f64> {
        if self.base_reserve == 0 || self.quote_reserve == 0 {
            return None;
        }

        let base_normalized = self.base_reserve as f64 / 10_f64.powi(base_decimals as i32);
        let quote_normalized = self.quote_reserve as f64 / 10_f64.powi(quote_decimals as i32);

        if base_normalized <= 0.0 {
            return None;
        }

        Some(quote_normalized / base_normalized)
    }
}
