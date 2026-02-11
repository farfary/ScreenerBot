/// Raydium CPMM pool decoder
///
/// This module handles decoding Raydium Constant Product Market Maker pools.
/// Extracts reserve data and calculates token prices.
use super::{AccountData, PoolDecoder};

use crate::constants::{RAYDIUM_CPMM_PROGRAM_ID, SOL_DECIMALS, SOL_MINT};
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind};
use crate::pools::utils::{
    read_bool_at_offset, read_pubkey_at_offset, read_u64_at_offset, read_u8_at_offset,
};
use crate::tokens::get_cached_decimals;

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
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Decoding Raydium CPMM pool for {}/{}",
                base_mint, quote_mint
            ),
        );

        // Find the pool account by checking owner program ID (like CLMM decoder)
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Raydium CPMM program as owner
            let owner_str = acc.owner.to_string();
            let matches = owner_str == RAYDIUM_CPMM_PROGRAM_ID;
            logger::debug(
                LogTag::PoolDecoder,
                &format!(
                    "Checking account owner: {} vs expected: {}, matches: {}",
                    owner_str, RAYDIUM_CPMM_PROGRAM_ID, matches
                ),
            );

            matches
        })?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Found CPMM pool account {} with {} bytes",
                pool_account.pubkey,
                pool_account.data.len()
            ),
        );

        let pool_info =
            Self::decode_raydium_cpmm_pool(&pool_account.data, &pool_account.pubkey.to_string())?;

        Self::calculate_raydium_cpmm_price(&pool_info, accounts, base_mint, quote_mint)
    }
}

impl RaydiumCpmmDecoder {
    /// Extract reserve account addresses from CPMM pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [token_0_vault, token_1_vault]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 8 + 32 * 4 {
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Extract vault addresses (same logic as decode_raydium_cpmm_pool)
        let _amm_config = read_pubkey_at_offset(pool_data, &mut offset).ok()?;
        let _pool_creator = read_pubkey_at_offset(pool_data, &mut offset).ok()?;
        let token_0_vault = read_pubkey_at_offset(pool_data, &mut offset).ok()?;
        let token_1_vault = read_pubkey_at_offset(pool_data, &mut offset).ok()?;

        Some(vec![token_0_vault, token_1_vault])
    }

    /// Decode Raydium CPMM pool data from account bytes
    pub fn decode_raydium_cpmm_pool(data: &[u8], pool_id: &str) -> Option<RaydiumCpmmPoolInfo> {
        if data.len() < 8 + 32 * 10 + 8 * 10 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Invalid Raydium CPMM pool account data length: {}",
                    data.len()
                ),
            );
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Decode pool data according to Raydium CPMM layout (enhanced version)
        let amm_config = read_pubkey_at_offset(data, &mut offset).ok()?;
        let pool_creator = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_vault = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_vault = read_pubkey_at_offset(data, &mut offset).ok()?;
        let lp_mint = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_mint = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_mint = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_0_program = read_pubkey_at_offset(data, &mut offset).ok()?;
        let token_1_program = read_pubkey_at_offset(data, &mut offset).ok()?;
        let observation_key = read_pubkey_at_offset(data, &mut offset).ok()?;

        let auth_bump = read_u8_at_offset(data, &mut offset).ok()?;
        let status = read_u8_at_offset(data, &mut offset).ok()?;
        let lp_mint_decimals = read_u8_at_offset(data, &mut offset).ok()?;
        let pool_mint_0_decimals = read_u8_at_offset(data, &mut offset).ok()?;
        let pool_mint_1_decimals = read_u8_at_offset(data, &mut offset).ok()?;

        // Skip padding to reach LP supply field at offset 333
        offset = 333;
        let lp_supply = read_u64_at_offset(data, &mut offset).ok()?;
        let protocol_fees_token_0 = read_u64_at_offset(data, &mut offset).ok()?;
        let protocol_fees_token_1 = read_u64_at_offset(data, &mut offset).ok()?;
        let fund_fees_token_0 = read_u64_at_offset(data, &mut offset).ok()?;
        let fund_fees_token_1 = read_u64_at_offset(data, &mut offset).ok()?;
        let open_time = read_u64_at_offset(data, &mut offset).ok()?;
        let recent_epoch = read_u64_at_offset(data, &mut offset).ok()?;

        // Skip padding to reach creator fee fields
        offset = 389; // After recent_epoch
        let creator_fee_on = read_u8_at_offset(data, &mut offset).ok()?;
        let enable_creator_fee = read_bool_at_offset(data, &mut offset).ok()?;

        // Skip padding1[6] bytes
        offset += 6;
        let creator_fees_token_0 = read_u64_at_offset(data, &mut offset).ok()?;
        let creator_fees_token_1 = read_u64_at_offset(data, &mut offset).ok()?;

        // Get token decimals - CRITICAL: must be available, no fallback to pool defaults
        let mint_0_decimals = match get_cached_decimals(&token_0_mint) {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "No decimals found for CPMM token_0: {}, skipping pool calculation",
                        token_0_mint
                    ),
                );
                return None;
            }
        };

        let mint_1_decimals = match get_cached_decimals(&token_1_mint) {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "No decimals found for CPMM token_1: {}, skipping pool calculation",
                        token_1_mint
                    ),
                );
                return None;
            }
        };

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Decimal Analysis:\n\n                     Token0 {} decimals: {} (cached) vs {} (pool)\n\n                     Token1 {} decimals: {} (cached) vs {} (pool)",
                token_0_mint,
                mint_0_decimals,
                pool_mint_0_decimals,
                token_1_mint,
                mint_1_decimals,
                pool_mint_1_decimals
            ),
        );

        // Warning if cached and pool decimals don't match
        if mint_0_decimals != pool_mint_0_decimals {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "DECIMAL MISMATCH Token0 {}: cache={}, pool={}",
                    token_0_mint, mint_0_decimals, pool_mint_0_decimals
                ),
            );
        }
        if mint_1_decimals != pool_mint_1_decimals {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "DECIMAL MISMATCH Token1 {}: cache={}, pool={}",
                    token_1_mint, mint_1_decimals, pool_mint_1_decimals
                ),
            );
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

            // Complete CPMM pool state fields
            lp_supply,
            protocol_fees_token_0,
            protocol_fees_token_1,
            fund_fees_token_0,
            fund_fees_token_1,
            open_time,
            recent_epoch,
            creator_fee_on,
            enable_creator_fee,
            creator_fees_token_0,
            creator_fees_token_1,
        })
    }

    /// Calculate price for Raydium CPMM pool
    fn calculate_raydium_cpmm_price(
        pool_info: &RaydiumCpmmPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        // Determine which token is SOL and which is the target token
        let sol_mint_str = SOL_MINT;
        let (target_mint, sol_reserve, token_reserve, sol_decimals, token_decimals) = if pool_info
            .token_0_mint
            == sol_mint_str
            && pool_info.token_1_mint == base_mint
        {
            // Token 0 is SOL, Token 1 is the target
            let vault_0_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_0_vault)?;
            let vault_1_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_1_vault)?;
            (
                base_mint.to_string(),
                vault_0_balance,
                vault_1_balance,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
            )
        } else if pool_info.token_1_mint == sol_mint_str && pool_info.token_0_mint == base_mint {
            // Token 1 is SOL, Token 0 is the target
            let vault_0_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_0_vault)?;
            let vault_1_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_1_vault)?;
            (
                base_mint.to_string(),
                vault_1_balance,
                vault_0_balance,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
            )
        } else if pool_info.token_0_mint == sol_mint_str && pool_info.token_1_mint == quote_mint {
            // Token 0 is SOL, Token 1 is the target
            let vault_0_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_0_vault)?;
            let vault_1_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_1_vault)?;
            (
                quote_mint.to_string(),
                vault_0_balance,
                vault_1_balance,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
            )
        } else if pool_info.token_1_mint == sol_mint_str && pool_info.token_0_mint == quote_mint {
            // Token 1 is SOL, Token 0 is the target
            let vault_0_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_0_vault)?;
            let vault_1_balance =
                Self::get_vault_balance_from_accounts(accounts, &pool_info.token_1_vault)?;
            (
                quote_mint.to_string(),
                vault_1_balance,
                vault_0_balance,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
            )
        } else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Pool does not contain SOL or target tokens {}/{}",
                    base_mint, quote_mint
                ),
            );
            return None;
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            logger::error(LogTag::PoolDecoder, "Pool has zero reserves");
            return None;
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        if sol_decimals > 18 || token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Raydium CPMM: Decimals too large: sol={}, token={}", sol_decimals, token_decimals),
            );
            return None;
        }
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        // Validate price is reasonable
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Invalid price calculated: {:.12} SOL", price_sol),
            );
            return None;
        }

        logger::verbose(
            LogTag::PoolDecoder,
            &format!(
                "Raydium CPMM Price Calculation for {}:\n\n                     SOL Reserve: {} ({:.9} adjusted, {} decimals)\n\n                     Token Reserve: {} ({:.9} adjusted, {} decimals)\n\n                     Price: {:.9} SOL",
                target_mint,
                sol_reserve,
                sol_adjusted,
                sol_decimals,
                token_reserve,
                token_adjusted,
                token_decimals,
                price_sol
            ),
        );

        // Additional validation checks
        if sol_adjusted <= 0.0 || token_adjusted <= 0.0 {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "WARNING: Zero or negative adjusted values detected! SOL_adj: {:.9}, Token_adj: {:.9}",
                    sol_adjusted, token_adjusted
                ),
            );
        }

        // Check for extremely small or large prices that might indicate decimal issues
        if price_sol < 0.000000001 || price_sol > 1000.0 {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "WARNING: Unusual price detected: {:.12} SOL. Check if decimals are correct (SOL: {}, Token: {})",
                    price_sol, sol_decimals, token_decimals
                ),
            );
        }

        Some(PriceResult::new(
            target_mint,
            0.0, // No USD calculation
            price_sol,
            sol_adjusted,
            token_adjusted,
            String::new(), // Pool address will be set by calculator
        ))
    }

    /// Extract vault balance from token account data
    fn get_vault_balance_from_accounts(
        accounts: &HashMap<String, AccountData>,
        vault_address: &str,
    ) -> Option<u64> {
        let account_data = accounts.get(vault_address)?;
        Self::decode_token_account_amount(&account_data.data).ok()
    }

    /// Decode token account amount from account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Invalid token account data length".to_string());
        }

        // Token account amount is at offset 64 (8 bytes)
        let amount_bytes = &data[64..72];
        let amount = u64::from_le_bytes(
            amount_bytes
                .try_into()
                .map_err(|_| "Failed to parse token account amount".to_string())?,
        );

        Ok(amount)
    }
}

/// Raydium CPMM pool information extracted from account data
/// Enhanced version with all fields needed for direct swap operations and complete pool state
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
    pub pool_id: String,         // Pool's public key
    pub amm_config: String,      // AMM configuration account
    pub pool_creator: String,    // Pool creator account
    pub lp_mint: String,         // LP token mint
    pub token_0_program: String, // Token 0 program ID
    pub token_1_program: String, // Token 1 program ID
    pub observation_key: String, // Observation state account
    pub auth_bump: u8,           // Authority bump seed
    pub status: u8,              // Pool status
    pub lp_mint_decimals: u8,    // LP token decimals

    // Complete CPMM pool state fields
    pub lp_supply: u64,             // LP token supply
    pub protocol_fees_token_0: u64, // Protocol fees for token 0
    pub protocol_fees_token_1: u64, // Protocol fees for token 1
    pub fund_fees_token_0: u64,     // Fund fees for token 0
    pub fund_fees_token_1: u64,     // Fund fees for token 1
    pub open_time: u64,             // Pool open timestamp
    pub recent_epoch: u64,          // Recent epoch number
    pub creator_fee_on: u8,         // Creator fee status
    pub enable_creator_fee: bool,   // Creator fee enabled flag
    pub creator_fees_token_0: u64,  // Creator fees for token 0
    pub creator_fees_token_1: u64,  // Creator fees for token 1
}
