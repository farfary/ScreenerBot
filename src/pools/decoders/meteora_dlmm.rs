/// Meteora DLMM decoder
///
/// This decoder handles Meteora Dynamic Liquidity Market Maker (DLMM) pools.
/// DLMM uses a different account structure from CPMM with token reserves stored
/// in separate vault accounts.
use super::super::utils::is_sol_mint;
use super::{AccountData, PoolDecoder};
use crate::constants::{METEORA_DLMM_PROGRAM_ID, SOL_DECIMALS, SOL_MINT};
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind};
use crate::tokens::get_cached_decimals;
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
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Meteora DLMM decoder: base={} quote={}",
                base_mint, quote_mint
            ),
        );

        // Find the pool account
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Meteora DLMM program as owner
            acc.owner.to_string() == METEORA_DLMM_PROGRAM_ID
        })?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Found DLMM pool account {} with {} bytes",
                pool_account.pubkey,
                pool_account.data.len()
            ),
        );

        // Parse DLMM pool structure
        let dlmm_info = Self::parse_dlmm_pool(&pool_account.data)?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM parsed: token_x={} token_y={} reserve_x={} reserve_y={}",
                dlmm_info.token_x_mint,
                dlmm_info.token_y_mint,
                dlmm_info.reserve_x,
                dlmm_info.reserve_y
            ),
        );

        // Determine which token is SOL and which is the base token
        let (token_mint, sol_vault, token_vault) = if is_sol_mint(&dlmm_info.token_y_mint) {
            // token_x is the base token, token_y is SOL
            (
                dlmm_info.token_x_mint.clone(),
                dlmm_info.reserve_y.clone(),
                dlmm_info.reserve_x.clone(),
            )
        } else if is_sol_mint(&dlmm_info.token_x_mint) {
            // token_x is SOL, token_y is the base token
            (
                dlmm_info.token_y_mint.clone(),
                dlmm_info.reserve_x.clone(),
                dlmm_info.reserve_y.clone(),
            )
        } else {
            logger::error(LogTag::PoolDecoder, "Pool doesn't contain SOL");
            return None;
        };

        // Verify the token mint matches one of the requested mints (base or quote)
        // This handles both TOKEN/SOL and SOL/TOKEN orientations
        if token_mint != base_mint && token_mint != quote_mint {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "DLMM token mint {} doesn't match either requested mint: base={}, quote={}",
                    token_mint, base_mint, quote_mint
                ),
            );
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance = Self::decode_token_account_amount(&token_account.data).ok()?;

        // Verify vault mints to ensure correct assignment
        let sol_vault_mint = Self::decode_token_account_mint(&sol_account.data);
        let token_vault_mint = Self::decode_token_account_mint(&token_account.data);
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Sol vault {} mint: {:?}", sol_vault, sol_vault_mint),
        );
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Token vault {} mint: {:?}", token_vault, token_vault_mint),
        );

        // Verify mints match expected
        if let (Ok(sol_mint), Ok(token_mint_check)) = (sol_vault_mint, token_vault_mint) {
            let sol_mint_correct = is_sol_mint(&sol_mint);
            let token_mint_correct = token_mint_check == token_mint;

            logger::debug(
                LogTag::PoolDecoder,
                &format!(
                    "Vault verification: SOL vault mint correct={}, Token vault mint correct={}",
                    sol_mint_correct, token_mint_correct
                ),
            );

            if !sol_mint_correct || !token_mint_correct {
                logger::error(
                    LogTag::PoolDecoder,
                    "VAULT ASSIGNMENT ERROR - swapping vaults",
                );
                return None;
            }
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!("SOL vault {} balance: {}", sol_vault, sol_balance),
        );
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Token vault {} balance: {}", token_vault, token_balance),
        );

        if token_balance == 0 {
            logger::error(LogTag::PoolDecoder, "Token reserve is zero");
            return None;
        }

        // Get token decimals - CRITICAL: must be available, no fallback to defaults
        let token_decimals = match get_cached_decimals(&token_mint) {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "DLMM: Token decimals not found for {}, skipping price calculation",
                        token_mint
                    ),
                );
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Token {} has {} decimals (cached/fallback)",
                token_mint, token_decimals
            ),
        );

        // Calculate price using DLMM theoretical price (more accurate than vault balances)
        let price_sol = Self::calculate_dlmm_price(&dlmm_info, &token_mint)?;

        if sol_decimals > 18 || token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Meteora DLMM: Decimals too large: sol={}, token={}", sol_decimals, token_decimals),
            );
            return None;
        }
        // Convert reserves to human-readable format for display
        let sol_reserves_display = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
        let token_reserves_display = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

        if !price_sol.is_finite() || price_sol <= 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Meteora DLMM: Invalid price calculated: {}", price_sol),
            );
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM theoretical price: {:.12} SOL per token (active_id={} bin_step={})",
                price_sol, dlmm_info.active_id, dlmm_info.bin_step
            ),
        );

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
    /// Extract reserve account addresses from DLMM pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [reserve_x, reserve_y]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 216 {
            return None;
        }

        // Extract reserve pubkeys at known offsets (same logic as parse_dlmm_pool)
        let reserve_x = Self::extract_pubkey_at_offset(pool_data, 152)?;
        let reserve_y = Self::extract_pubkey_at_offset(pool_data, 184)?;

        Some(vec![reserve_x.to_string(), reserve_y.to_string()])
    }

    /// Parse DLMM pool account data to extract token mints and reserve addresses
    fn parse_dlmm_pool(data: &[u8]) -> Option<DlmmPoolInfo> {
        if data.len() < 216 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("DLMM pool data too short: {} bytes", data.len()),
            );
            return None;
        }

        // Extract pubkeys at known offsets (from original working implementation)
        let token_x_mint = Self::extract_pubkey_at_offset(data, 88)?;
        let token_y_mint = Self::extract_pubkey_at_offset(data, 120)?;
        let reserve_x = Self::extract_pubkey_at_offset(data, 152)?;
        let reserve_y = Self::extract_pubkey_at_offset(data, 184)?;

        // Extract DLMM-specific fields
        // Run debug scan for DLMM values (centralized logger will filter output)
        logger::debug(
            LogTag::PoolDecoder,
            "Running DLMM debug scan for offsets and values",
        );
        Self::debug_scan_for_dlmm_values(data);

        // Use the correct offsets found by scanning
        let active_id = Self::extract_i32_at_offset(data, 76)?; // Found at offset 76: -485
        let bin_step = Self::extract_u16_at_offset(data, 80)?; // Found at offset 80: 30

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM offsets: token_x@88={} token_y@120={}",
                token_x_mint, token_y_mint
            ),
        );
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM offsets: reserve_x@152={} reserve_y@184={}",
                reserve_x, reserve_y
            ),
        );
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM pricing: active_id@76={} bin_step@80={}",
                active_id, bin_step
            ),
        );

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

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM calc: bin_step_factor={:.8}, active_id={}, raw_price={:.12}",
                bin_step_factor, dlmm_info.active_id, raw_price
            ),
        );

        // Get decimals for proper price scaling
        let token_x_decimals = get_cached_decimals(&dlmm_info.token_x_mint).unwrap_or(6);
        let token_y_decimals = get_cached_decimals(&dlmm_info.token_y_mint).unwrap_or(9);

        // Adjust for decimals difference: price_in_human_units = raw_price * 10^(token_x_decimals - token_y_decimals)
        let decimals_scale = (10f64).powi((token_x_decimals as i32) - (token_y_decimals as i32));
        let adjusted_price = raw_price * decimals_scale;

        // Determine if we need to invert the price based on token order
        let price_sol = if dlmm_info.token_x_mint == token_mint {
            // token_x is our target token, token_y is SOL
            // adjusted_price gives SOL per token (what we want)
            adjusted_price
        } else if dlmm_info.token_y_mint == token_mint {
            // token_y is our target token, token_x is SOL
            // adjusted_price gives token per SOL, so we need to invert
            if adjusted_price != 0.0 {
                1.0 / adjusted_price
            } else {
                return None;
            }
        } else {
            // Neither token matches (shouldn't happen)
            return None;
        };

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DLMM price direction: token_x={} ({}d) token_y={} ({}d) target={}, decimals_scale={:.6}, final_price={:.12}",
                dlmm_info.token_x_mint,
                token_x_decimals,
                dlmm_info.token_y_mint,
                token_y_decimals,
                token_mint,
                decimals_scale,
                price_sol
            ),
        );

        Some(price_sol)
    }

    /// Debug function to scan for expected DLMM values in pool data
    fn debug_scan_for_dlmm_values(data: &[u8]) {
        // Scan for active_id = -485
        for offset in (0..data.len().saturating_sub(4)).step_by(4) {
            if let Some(value) = Self::extract_i32_at_offset(data, offset) {
                if value == -485 {
                    logger::debug(
                        LogTag::PoolDecoder,
                        &format!("Found active_id=-485 at offset {}", offset),
                    );
                }
            }
        }

        // Scan for bin_step = 30
        for offset in (0..data.len().saturating_sub(2)).step_by(2) {
            if let Some(value) = Self::extract_u16_at_offset(data, offset) {
                if value == 30 {
                    logger::debug(
                        LogTag::PoolDecoder,
                        &format!("Found bin_step=30 at offset {}", offset),
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
