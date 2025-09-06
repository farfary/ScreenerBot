/// Meteora DLMM decoder
///
/// This decoder handles Meteora Dynamic Liquidity Market Maker (DLMM) pools.
/// DLMM uses a different account structure from CPMM with token reserves stored
/// in separate vault accounts.

use super::{ PoolDecoder, AccountData };
use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::decimals::get_cached_decimals;
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT, METEORA_DLMM_PROGRAM_ID };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct MeteoraDlmmDecoder;

impl PoolDecoder for MeteoraDlmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDlmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "START",
                &format!("Meteora DLMM decoder: base={} quote={}", base_mint, quote_mint)
            );
        }

        // Find the pool account
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Meteora DLMM program as owner
            acc.owner.to_string() == METEORA_DLMM_PROGRAM_ID
        })?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "Found DLMM pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse DLMM pool structure
        let dlmm_info = Self::parse_dlmm_pool(&pool_account.data)?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "DLMM parsed: token_x={} token_y={} reserve_x={} reserve_y={}",
                    dlmm_info.token_x_mint,
                    dlmm_info.token_y_mint,
                    dlmm_info.reserve_x,
                    dlmm_info.reserve_y
                )
            );
        }

        // Determine which token is SOL and which is the base token
        let (token_mint, sol_vault, token_vault) = if dlmm_info.token_y_mint == SOL_MINT {
            // token_x is the base token, token_y is SOL
            (
                dlmm_info.token_x_mint.clone(),
                dlmm_info.reserve_y.clone(),
                dlmm_info.reserve_x.clone(),
            )
        } else if dlmm_info.token_x_mint == SOL_MINT {
            // token_x is SOL, token_y is the base token
            (
                dlmm_info.token_y_mint.clone(),
                dlmm_info.reserve_x.clone(),
                dlmm_info.reserve_y.clone(),
            )
        } else {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "ERROR", "Pool doesn't contain SOL");
            }
            return None;
        };

        // Verify this matches the requested base mint
        if token_mint != base_mint {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!("Token mint mismatch: expected {} got {}", base_mint, token_mint)
                );
            }
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance = Self::decode_token_account_amount(&token_account.data).ok()?;

        // Verify vault mints to ensure correct assignment
        if is_debug_pool_calculator_enabled() {
            let sol_vault_mint = Self::decode_token_account_mint(&sol_account.data);
            let token_vault_mint = Self::decode_token_account_mint(&token_account.data);

            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Sol vault {} mint: {:?}", sol_vault, sol_vault_mint)
            );
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Token vault {} mint: {:?}", token_vault, token_vault_mint)
            );

            // Verify mints match expected
            if let (Ok(sol_mint), Ok(token_mint_check)) = (sol_vault_mint, token_vault_mint) {
                let sol_mint_correct = sol_mint == SOL_MINT;
                let token_mint_correct = token_mint_check == token_mint;

                log(
                    LogTag::PoolCalculator,
                    if sol_mint_correct && token_mint_correct {
                        "INFO"
                    } else {
                        "ERROR"
                    },
                    &format!(
                        "Vault verification: SOL vault mint correct={}, Token vault mint correct={}",
                        sol_mint_correct,
                        token_mint_correct
                    )
                );

                if !sol_mint_correct || !token_mint_correct {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        "VAULT ASSIGNMENT ERROR - swapping vaults"
                    );
                    // Maybe we need to swap the assignments?
                    return None;
                }
            }
        }

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("SOL vault {} balance: {}", sol_vault, sol_balance)
            );
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Token vault {} balance: {}", token_vault, token_balance)
            );
        }

        if token_balance == 0 {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "ERROR", "Token reserve is zero");
            }
            return None;
        }

        // Get token decimals
        let token_decimals = get_cached_decimals(&token_mint).unwrap_or(9);
        let sol_decimals = 9u8;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("Token {} has {} decimals (cached/fallback)", token_mint, token_decimals)
            );
        }

        // Calculate price: SOL per token using vault balances (more accurate for current price)
        let price_sol =
            (sol_balance as f64) /
            (10_f64).powi(sol_decimals as i32) /
            ((token_balance as f64) / (10_f64).powi(token_decimals as i32));

        // Convert reserves to human-readable format for display
        let sol_reserves_display = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
        let token_reserves_display = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "CALC",
                &format!(
                    "DLMM price from vault balances: {:.12} SOL per token (sol_bal={} token_bal={} decimals={})",
                    price_sol,
                    sol_balance,
                    token_balance,
                    token_decimals
                )
            );
            let theoretical_price = Self::calculate_dlmm_price(&dlmm_info, &token_mint).unwrap_or(
                0.0
            );
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "DLMM theoretical price from active_id: {:.12} (active_id={} bin_step={})",
                    theoretical_price,
                    dlmm_info.active_id,
                    dlmm_info.bin_step
                )
            );
        }

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // USD conversion not implemented yet
            price_sol,
            confidence: 0.9, // DLMM pools are generally reliable
            source_pool: Some("METEORA DLMM".to_string()),
            pool_address: pool_account.pubkey.to_string(),
            slot: pool_account.slot,
            timestamp: Instant::now(),
            sol_reserves: sol_reserves_display,
            token_reserves: token_reserves_display,
        })
    }
}

impl MeteoraDlmmDecoder {
    /// Parse DLMM pool account data to extract token mints and reserve addresses
    fn parse_dlmm_pool(data: &[u8]) -> Option<DlmmPoolInfo> {
        if data.len() < 216 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!("DLMM pool data too short: {} bytes", data.len())
                );
            }
            return None;
        }

        // Extract pubkeys at known offsets (from original working implementation)
        let token_x_mint = Self::extract_pubkey_at_offset(data, 88)?;
        let token_y_mint = Self::extract_pubkey_at_offset(data, 120)?;
        let reserve_x = Self::extract_pubkey_at_offset(data, 152)?;
        let reserve_y = Self::extract_pubkey_at_offset(data, 184)?;

        // Extract DLMM-specific fields
        // Let me scan for the expected values first
        if is_debug_pool_calculator_enabled() {
            Self::debug_scan_for_dlmm_values(data);
        }

        // Use the correct offsets found by scanning
        let active_id = Self::extract_i32_at_offset(data, 76)?; // Found at offset 76: -485
        let bin_step = Self::extract_u16_at_offset(data, 80)?; // Found at offset 80: 30

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("DLMM offsets: token_x@88={} token_y@120={}", token_x_mint, token_y_mint)
            );
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("DLMM offsets: reserve_x@152={} reserve_y@184={}", reserve_x, reserve_y)
            );
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("DLMM pricing: active_id@76={} bin_step@80={}", active_id, bin_step)
            );
        }

        Some(DlmmPoolInfo {
            token_x_mint: token_x_mint.to_string(),
            token_y_mint: token_y_mint.to_string(),
            reserve_x: reserve_x.to_string(),
            reserve_y: reserve_y.to_string(),
            active_id,
            bin_step,
        })
    }

    /// Extract a pubkey from raw data at a specific offset
    fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Option<Pubkey> {
        if data.len() < offset + 32 {
            return None;
        }

        let pubkey_bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
        Some(Pubkey::new_from_array(pubkey_bytes))
    }

    /// Extract an i32 from raw data at a specific offset (little-endian)
    fn extract_i32_at_offset(data: &[u8], offset: usize) -> Option<i32> {
        if data.len() < offset + 4 {
            return None;
        }

        let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }

    /// Extract a u16 from raw data at a specific offset (little-endian)
    fn extract_u16_at_offset(data: &[u8], offset: usize) -> Option<u16> {
        if data.len() < offset + 2 {
            return None;
        }

        let bytes: [u8; 2] = data[offset..offset + 2].try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }

    /// Calculate DLMM price using active_id and bin_step
    fn calculate_dlmm_price(dlmm_info: &DlmmPoolInfo, token_mint: &str) -> Option<f64> {
        // DLMM price formula: price = (1 + bin_step / 10000) ^ active_id
        // This gives the price of token_y in terms of token_x

        let bin_step_factor = 1.0 + (dlmm_info.bin_step as f64) / 10000.0;
        let raw_price = bin_step_factor.powf(dlmm_info.active_id as f64);

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "DLMM calc: bin_step_factor={:.8}, active_id={}, raw_price={:.12}",
                    bin_step_factor,
                    dlmm_info.active_id,
                    raw_price
                )
            );
        }

        // Determine if we need to invert the price based on token order
        let price_sol = if dlmm_info.token_x_mint == token_mint {
            // token_x is our target token, token_y is SOL
            // raw_price gives SOL per token (what we want)
            raw_price
        } else if dlmm_info.token_y_mint == token_mint {
            // token_y is our target token, token_x is SOL
            // raw_price gives token per SOL, so we need to invert
            if raw_price != 0.0 {
                1.0 / raw_price
            } else {
                return None;
            }
        } else {
            // Neither token matches (shouldn't happen)
            return None;
        };

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "DLMM price direction: token_x={} token_y={} target={}, final_price={:.12}",
                    dlmm_info.token_x_mint,
                    dlmm_info.token_y_mint,
                    token_mint,
                    price_sol
                )
            );
        }

        Some(price_sol)
    }

    /// Debug function to scan for expected DLMM values in pool data
    fn debug_scan_for_dlmm_values(data: &[u8]) {
        // Scan for active_id = -485
        for offset in (0..data.len().saturating_sub(4)).step_by(4) {
            if let Some(value) = Self::extract_i32_at_offset(data, offset) {
                if value == -485 {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("Found active_id=-485 at offset {}", offset)
                    );
                }
            }
        }

        // Scan for bin_step = 30
        for offset in (0..data.len().saturating_sub(2)).step_by(2) {
            if let Some(value) = Self::extract_u16_at_offset(data, offset) {
                if value == 30 {
                    log(
                        LogTag::PoolCalculator,
                        "DEBUG",
                        &format!("Found bin_step=30 at offset {}", offset)
                    );
                }
            }
        }
    }

    /// Decode token account amount from token account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Token account data too short".to_string());
        }

        // Amount is stored at offset 64 as little-endian u64
        let amount_bytes: [u8; 8] = data[64..72]
            .try_into()
            .map_err(|_| "Failed to extract amount bytes".to_string())?;

        Ok(u64::from_le_bytes(amount_bytes))
    }

    /// Decode token account mint from token account data
    fn decode_token_account_mint(data: &[u8]) -> Result<String, String> {
        if data.len() < 32 {
            return Err("Token account data too short for mint".to_string());
        }

        // Mint is stored at offset 0 as 32-byte pubkey
        let mint_bytes: [u8; 32] = data[0..32]
            .try_into()
            .map_err(|_| "Failed to extract mint bytes".to_string())?;

        let mint = Pubkey::new_from_array(mint_bytes);
        Ok(mint.to_string())
    }
}

/// Meteora DLMM pool information structure
#[derive(Debug, Clone)]
struct DlmmPoolInfo {
    pub token_x_mint: String,
    pub token_y_mint: String,
    pub reserve_x: String,
    pub reserve_y: String,
    pub active_id: i32,
    pub bin_step: u16,
}
