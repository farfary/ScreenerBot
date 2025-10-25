/// PumpFun Legacy decoder for the original PumpFun program
///
/// Handles pool decoding and price calculation for the legacy PumpFun program.
/// Parses pool account data to extract mint and vault information.
use super::super::utils::{
    analyze_token_pair, is_sol_mint, read_pubkey_at_offset, PoolMintVaultInfo,
};
use super::{AccountData, PoolDecoder};

use crate::constants::{PUMP_FUN_LEGACY_PROGRAM_ID, SOL_DECIMALS, SOL_MINT};
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind};
use crate::tokens::get_cached_decimals;

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

/// PumpFun Legacy pool decoder and calculator
pub struct PumpFunLegacyDecoder;

impl PoolDecoder for PumpFunLegacyDecoder {
    /// Get the program kinds this decoder supports
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::PumpFunLegacy]
    }

    /// Decode pool data and calculate price for legacy PumpFun
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "PumpFun Legacy: Processing for {} vs {}",
                base_mint, quote_mint
            ),
        );

        // Find the pool account by looking for the legacy PumpFun program account
        for (pool_account, pool_data) in accounts.iter() {
            // Skip non-pool accounts (vaults, mints) by checking size and owner
            if pool_data.data.len() < 100 {
                continue; // Too small to be a pool account
            }

            // Check if this is the actual pool account by looking for legacy PumpFun program ownership
            if pool_data.owner.to_string() != PUMP_FUN_LEGACY_PROGRAM_ID {
                continue; // Not owned by legacy PumpFun program
            }

            if let Some(pool_info) = Self::decode_pump_fun_legacy_pool(&pool_data.data) {
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Successfully decoded PumpFun Legacy pool: {} -> {}",
                        pool_info.base_mint, pool_info.quote_mint
                    ),
                );

                // Calculate price using the extracted pool info
                if let Some(price_result) = Self::calculate_pump_fun_legacy_price(
                    &pool_info,
                    accounts,
                    base_mint,
                    quote_mint,
                    pool_account,
                ) {
                    return Some(price_result);
                }
            }
        }

        None
    }
}

impl PumpFunLegacyDecoder {
    /// Extract mints and vaults from PumpFun Legacy pool data
    fn extract_pumpfun_mints_and_vaults(data: &[u8]) -> Option<PoolMintVaultInfo> {
        if data.len() < 200 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("PumpFun Legacy pool data too short: {} bytes", data.len()),
            );
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!("Extracting PumpFun Legacy pool data ({} bytes)", data.len()),
        );

        // PumpFun Legacy structure (same as AMM for extraction purposes):
        // discriminator(8) + pool_bump(1) + index(2) + creator(32) + base_mint(32) + quote_mint(32) + lp_mint(32) + vault1(32) + vault2(32) + ...
        let mut offset = 8 + 1 + 2 + 32; // Skip discriminator, bump, index, and creator

        // Read base mint and quote mint
        let mint1 = read_pubkey_at_offset(data, &mut offset).ok()?; // base_mint
        let mint2 = read_pubkey_at_offset(data, &mut offset).ok()?; // quote_mint

        // Skip lp_mint
        offset += 32;

        // Read vault addresses
        let vault1 = read_pubkey_at_offset(data, &mut offset).ok()?;
        let vault2 = read_pubkey_at_offset(data, &mut offset).ok()?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Extracted PumpFun Legacy: mint1={}, mint2={}, vault1={}, vault2={}",
                &mint1[..8],
                &mint2[..8],
                &vault1[..8],
                &vault2[..8]
            ),
        );

        Some(PoolMintVaultInfo {
            mint1,
            mint2,
            vault1,
            vault2,
        })
    }

    /// Decode PumpFun Legacy pool data from account bytes
    fn decode_pump_fun_legacy_pool(data: &[u8]) -> Option<PumpFunLegacyPoolInfo> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Decoding PumpFun Legacy pool data ({} bytes)", data.len()),
        );

        // Legacy PumpFun has different structure than AMM version
        // This is a simplified structure - adjust offsets based on actual data layout
        if data.len() < 200 {
            return None;
        }

        // Extract data from legacy pool structure
        // Offsets derived from legacy pool account structure
        let base_mint_offset = 8; // Skip discriminator
        let quote_mint_offset = base_mint_offset + 32;
        let vault1_offset = quote_mint_offset + 32;
        let vault2_offset = vault1_offset + 32;

        if data.len() < vault2_offset + 32 {
            return None;
        }

        let base_mint = Pubkey::try_from(&data[base_mint_offset..base_mint_offset + 32]).ok()?;
        let quote_mint = Pubkey::try_from(&data[quote_mint_offset..quote_mint_offset + 32]).ok()?;
        let vault1 = Pubkey::try_from(&data[vault1_offset..vault1_offset + 32]).ok()?;
        let vault2 = Pubkey::try_from(&data[vault2_offset..vault2_offset + 32]).ok()?;

        // Determine which is SOL and which is the token
        let pool_info = PoolMintVaultInfo {
            mint1: base_mint.to_string(),
            mint2: quote_mint.to_string(),
            vault1: vault1.to_string(),
            vault2: vault2.to_string(),
        };

        let pair_info = analyze_token_pair(pool_info);

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Valid PumpFun Legacy SOL pool: token={}, sol_is_first={}, token_vault={}, sol_vault={}",
                &pair_info.token_mint[..8],
                pair_info.sol_is_first,
                &pair_info.token_vault[..8],
                &pair_info.sol_vault[..8]
            ),
        );

        Some(PumpFunLegacyPoolInfo {
            base_mint: pair_info.token_mint,
            quote_mint: pair_info.sol_mint,
            pool_base_token_account: pair_info.token_vault,
            pool_quote_token_account: pair_info.sol_vault,
        })
    }

    /// Calculate price for PumpFun Legacy pool
    fn calculate_pump_fun_legacy_price(
        pool_info: &PumpFunLegacyPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
        pool_account: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Calculating PumpFun Legacy price for token {} with quote {}",
                base_mint, quote_mint
            ),
        );

        // Determine which mint is our target
        let target_mint = if is_sol_mint(base_mint) {
            quote_mint
        } else {
            base_mint
        };

        logger::debug(
            LogTag::PoolDecoder,
            &format!("Using target mint: {}", target_mint),
        );

        // Get vault balances
        let token_vault_balance =
            accounts
                .get(&pool_info.pool_base_token_account)
                .and_then(|acc| {
                    if acc.data.len() >= 72 {
                        let balance_bytes = &acc.data[64..72];
                        Some(u64::from_le_bytes(balance_bytes.try_into().ok()?))
                    } else {
                        None
                    }
                })?;

        let sol_vault_balance =
            accounts
                .get(&pool_info.pool_quote_token_account)
                .and_then(|acc| {
                    if acc.data.len() >= 72 {
                        let balance_bytes = &acc.data[64..72];
                        Some(u64::from_le_bytes(balance_bytes.try_into().ok()?))
                    } else {
                        None
                    }
                })?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Vault {} balance: {}",
                &pool_info.pool_base_token_account[..8],
                token_vault_balance
            ),
        );
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Vault {} balance: {}",
                &pool_info.pool_quote_token_account[..8],
                sol_vault_balance
            ),
        );

        // Get decimals for the target token
        let target_decimals = get_cached_decimals(target_mint)? as u32;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Raw reserves - Token: {}, SOL: {}",
                token_vault_balance, sol_vault_balance
            ),
        );

        // Calculate price in SOL
        let sol_reserve_f64 = (sol_vault_balance as f64) / (10_f64).powi(SOL_DECIMALS as i32);
        let token_reserve_f64 =
            (token_vault_balance as f64) / (10_f64).powi(target_decimals as i32);

        if token_reserve_f64 <= 0.0 || sol_reserve_f64 <= 0.0 {
            return None;
        }

        let price_sol = sol_reserve_f64 / token_reserve_f64;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "PumpFun Legacy price calculation:\n                                              - SOL Reserve: {} (decimals: {}, adjusted: {:.8})\n                                              - Token Reserve: {} (decimals: {}, adjusted: {:.8})\n                                              - Price SOL: {:.12}\n                                              - Target Token: {}",
                sol_vault_balance,
                SOL_DECIMALS,
                sol_reserve_f64,
                token_vault_balance,
                target_decimals,
                token_reserve_f64,
                price_sol,
                target_mint
            ),
        );

        Some(PriceResult {
            mint: target_mint.to_string(),
            price_usd: 0.0, // USD price not calculated here
            price_sol,
            confidence: 1.0,
            source_pool: Some("PumpFun Legacy".to_string()),
            pool_address: pool_account.to_string(),
            slot: 0,
            timestamp: Instant::now(),
            sol_reserves: sol_reserve_f64,
            token_reserves: token_reserve_f64,
        })
    }
}

/// PumpFun Legacy pool information extracted from account data
#[derive(Debug, Clone)]
pub struct PumpFunLegacyPoolInfo {
    pub base_mint: String,
    pub quote_mint: String,
    pub pool_base_token_account: String,
    pub pool_quote_token_account: String,
}

impl PumpFunLegacyDecoder {
    /// Extract reserve account addresses from PumpFun Legacy pool data
    pub fn extract_reserve_accounts(data: &[u8]) -> Option<Vec<String>> {
        // For legacy PumpFun, extract pool info first
        let pool_info = Self::extract_pumpfun_mints_and_vaults(data)?;

        // Analyze the pool data to find SOL and token vault accounts
        let pair_info = super::super::utils::analyze_token_pair(pool_info);

        // Return vaults in standard order (token, sol)
        Some(vec![pair_info.token_vault, pair_info.sol_vault])
    }
}
