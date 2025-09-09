/// Raydium CPMM pool decoder
///
/// This module handles decoding Raydium Constant Product Market Maker pools.
/// It extracts reserve data and calculates token prices using the proven logic
/// from the old pool system.

use super::{ PoolDecoder, AccountData };
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT, RAYDIUM_CPMM_PROGRAM_ID };
use crate::tokens::{ get_token_decimals_sync, decimals::{ SOL_DECIMALS, raw_to_ui_amount } };
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
        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("Decoding Raydium CPMM pool for {}/{}", base_mint, quote_mint)
            );
        }

        // Find the pool account by checking owner program ID (like CLMM decoder)
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Raydium CPMM program as owner
            let owner_str = acc.owner.to_string();
            let matches = owner_str == RAYDIUM_CPMM_PROGRAM_ID;

            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "DEBUG",
                    &format!(
                        "Checking account owner: {} vs expected: {}, matches: {}",
                        owner_str,
                        RAYDIUM_CPMM_PROGRAM_ID,
                        matches
                    )
                );
            }

            matches
        })?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!(
                    "Found CPMM pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse pool state from account data using the enhanced method
        let pool_info = Self::decode_raydium_cpmm_pool(
            &pool_account.data,
            &pool_account.pubkey.to_string()
        )?;

        // Calculate price using the working logic from old system
        Self::calculate_raydium_cpmm_price(&pool_info, accounts, base_mint, quote_mint)
    }
}

impl RaydiumCpmmDecoder {
    /// Decode Raydium CPMM pool data from account bytes (enhanced for swap operations)
    /// Made public for use by the direct swap system
    pub fn decode_raydium_cpmm_pool(data: &[u8], pool_id: &str) -> Option<RaydiumCpmmPoolInfo> {
        if data.len() < 8 + 32 * 10 + 8 * 10 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("Invalid Raydium CPMM pool account data length: {}", data.len())
                );
            }
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Decode pool data according to Raydium CPMM layout (enhanced version)
        let amm_config = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let pool_creator = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_vault = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_vault = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let lp_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_program = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_program = Self::read_pubkey_at_offset(data, &mut offset).ok()?;
        let observation_key = Self::read_pubkey_at_offset(data, &mut offset).ok()?;

        let auth_bump = Self::read_u8_at_offset(data, &mut offset).ok()?;
        let status = Self::read_u8_at_offset(data, &mut offset).ok()?;
        let lp_mint_decimals = Self::read_u8_at_offset(data, &mut offset).ok()?;
        let pool_mint_0_decimals = Self::read_u8_at_offset(data, &mut offset).ok()?;
        let pool_mint_1_decimals = Self::read_u8_at_offset(data, &mut offset).ok()?;

        // Get token decimals - CRITICAL: must be available, no fallback to pool defaults
        let mint_0_decimals = match get_token_decimals_sync(&token_0_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!(
                            "No decimals found for CPMM token_0: {}, skipping pool calculation",
                            token_0_mint.chars().take(8).collect::<String>()
                        )
                    );
                }
                return None;
            }
        };

        let mint_1_decimals = match get_token_decimals_sync(&token_1_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!(
                            "No decimals found for CPMM token_1: {}, skipping pool calculation",
                            token_1_mint.chars().take(8).collect::<String>()
                        )
                    );
                }
                return None;
            }
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DECIMALS",
                &format!(
                    "Decimal Analysis:
  
                     Token0 {} decimals: {} (cached) vs {} (pool)
  
                     Token1 {} decimals: {} (cached) vs {} (pool)",
                    token_0_mint.chars().take(8).collect::<String>(),
                    mint_0_decimals,
                    pool_mint_0_decimals,
                    token_1_mint.chars().take(8).collect::<String>(),
                    mint_1_decimals,
                    pool_mint_1_decimals
                )
            );

            // Warning if cached and pool decimals don't match
            if mint_0_decimals != pool_mint_0_decimals {
                log(
                    LogTag::PoolDecoder,
                    "DECIMAL_MISMATCH",
                    &format!(
                        "DECIMAL MISMATCH Token0 {}: cache={}, pool={}",
                        token_0_mint,
                        mint_0_decimals,
                        pool_mint_0_decimals
                    )
                );
            }
            if mint_1_decimals != pool_mint_1_decimals {
                log(
                    LogTag::PoolDecoder,
                    "DECIMAL_MISMATCH",
                    &format!(
                        "DECIMAL MISMATCH Token1 {}: cache={}, pool={}",
                        token_1_mint,
                        mint_1_decimals,
                        pool_mint_1_decimals
                    )
                );
            }
        }

        Some(RaydiumCpmmPoolInfo {
            // Basic token information
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            token_0_decimals: mint_0_decimals,
            token_1_decimals: mint_1_decimals,

            // Additional fields for swap operations
            pool_id: pool_id.to_string(),
            amm_config,
            pool_creator,
            lp_mint,
            token_0_program,
            token_1_program,
            observation_key,
            auth_bump,
            status,
            lp_mint_decimals,
        })
    }

    /// Calculate price for Raydium CPMM pool (from old working system)
    fn calculate_raydium_cpmm_price(
        pool_info: &RaydiumCpmmPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        // Determine which token is SOL and which is the target token
        let sol_mint_str = SOL_MINT;
        let (target_mint, sol_reserve, token_reserve, sol_decimals, token_decimals) = if
            pool_info.token_0_mint == sol_mint_str &&
            pool_info.token_1_mint == base_mint
        {
            // Token 0 is SOL, Token 1 is the target
            let vault_0_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_0_vault
            )?;
            let vault_1_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_1_vault
            )?;
            (
                base_mint.to_string(),
                vault_0_balance,
                vault_1_balance,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
            )
        } else if pool_info.token_1_mint == sol_mint_str && pool_info.token_0_mint == base_mint {
            // Token 1 is SOL, Token 0 is the target
            let vault_0_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_0_vault
            )?;
            let vault_1_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_1_vault
            )?;
            (
                base_mint.to_string(),
                vault_1_balance,
                vault_0_balance,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
            )
        } else if pool_info.token_0_mint == sol_mint_str && pool_info.token_1_mint == quote_mint {
            // Token 0 is SOL, Token 1 is the target
            let vault_0_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_0_vault
            )?;
            let vault_1_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_1_vault
            )?;
            (
                quote_mint.to_string(),
                vault_0_balance,
                vault_1_balance,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
            )
        } else if pool_info.token_1_mint == sol_mint_str && pool_info.token_0_mint == quote_mint {
            // Token 1 is SOL, Token 0 is the target
            let vault_0_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_0_vault
            )?;
            let vault_1_balance = Self::get_vault_balance_from_accounts(
                accounts,
                &pool_info.token_1_vault
            )?;
            (
                quote_mint.to_string(),
                vault_1_balance,
                vault_0_balance,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
            )
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "Pool does not contain SOL or target tokens {}/{}",
                        base_mint,
                        quote_mint
                    )
                );
            }
            return None;
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            if is_debug_pool_decoders_enabled() {
                log(LogTag::PoolDecoder, "ERROR", "Pool has zero reserves");
            }
            return None;
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        // Validate price is reasonable
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("Invalid price calculated: {:.12} SOL", price_sol)
                );
            }
            return None;
        }

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "SUCCESS",
                &format!(
                    "Raydium CPMM Price Calculation for {}:
  
                     SOL Reserve: {} ({:.9} adjusted, {} decimals)
  
                     Token Reserve: {} ({:.9} adjusted, {} decimals)
  
                     Price: {:.9} SOL",
                    target_mint,
                    sol_reserve,
                    sol_adjusted,
                    sol_decimals,
                    token_reserve,
                    token_adjusted,
                    token_decimals,
                    price_sol
                )
            );

            // Additional validation checks
            if sol_adjusted <= 0.0 || token_adjusted <= 0.0 {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    &format!(
                        "WARNING: Zero or negative adjusted values detected! 
                         SOL_adj: {:.9}, Token_adj: {:.9}",
                        sol_adjusted,
                        token_adjusted
                    )
                );
            }

            // Check for extremely small or large prices that might indicate decimal issues
            if price_sol < 0.000000001 || price_sol > 1000.0 {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    &format!(
                        "WARNING: Unusual price detected: {:.12} SOL. 
                         Check if decimals are correct (SOL: {}, Token: {})",
                        price_sol,
                        sol_decimals,
                        token_decimals
                    )
                );
            }
        }

        Some(
            PriceResult::new(
                target_mint,
                0.0, // No USD calculation
                price_sol,
                sol_adjusted,
                token_adjusted,
                String::new() // Pool address will be set by calculator
            )
        )
    }

    /// Extract vault balance from token account data (from old system)
    fn get_vault_balance_from_accounts(
        accounts: &HashMap<String, AccountData>,
        vault_address: &str
    ) -> Option<u64> {
        let account_data = accounts.get(vault_address)?;
        Self::decode_token_account_amount(&account_data.data).ok()
    }

    /// Decode token account amount from account data (from old working system)
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Invalid token account data length".to_string());
        }

        // Token account amount is at offset 64 (8 bytes)
        let amount_bytes = &data[64..72];
        let amount = u64::from_le_bytes(
            amount_bytes.try_into().map_err(|_| "Failed to parse token account amount".to_string())?
        );

        Ok(amount)
    }

    // Helper functions for reading pool data (from old working system)
    fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
        if *offset >= data.len() {
            return Err("Insufficient data for u8".to_string());
        }

        let value = data[*offset];
        *offset += 1;
        Ok(value)
    }
}

/// Raydium CPMM pool information extracted from account data
/// Enhanced version with all fields needed for direct swap operations
#[derive(Debug, Clone)]
pub struct RaydiumCpmmPoolInfo {
    // Basic token information
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: String,
    pub token_1_vault: String,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,

    // Additional fields required for swap operations
    pub pool_id: String, // Pool's public key
    pub amm_config: String, // AMM configuration account
    pub pool_creator: String, // Pool creator account
    pub lp_mint: String, // LP token mint
    pub token_0_program: String, // Token 0 program ID
    pub token_1_program: String, // Token 1 program ID
    pub observation_key: String, // Observation state account
    pub auth_bump: u8, // Authority bump seed
    pub status: u8, // Pool status
    pub lp_mint_decimals: u8, // LP token decimals
}
