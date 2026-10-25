// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! PumpFun AMM pool decoder — parses on-chain PumpFun pool accounts.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::{SOL_DECIMALS, SOL_MINT};
use crate::chains::solana::pools::layouts::pumpfun_amm::PumpAmmPoolState;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;
use std::time::Instant;

// Import centralized utilities
use crate::chains::solana::pools::decode_utils::read_pubkey_at_offset;
use crate::chains::solana::pools::decode_utils::validate_sol_pool;
use crate::chains::solana::pools::types::PoolMintVaultInfo;

/// PumpFun AMM pool decoder and calculator
pub struct PumpFunAmmDecoder;

impl PoolDecoder for PumpFunAmmDecoder {
    /// Get the program kinds this decoder supports
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::PumpFunAmm]
    }

    /// Decode pool data and calculate price using centralized utilities
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "PumpFun AMM: Processing for {} vs {}",
                base_mint, quote_mint
            ),
        );

        // Find the pool account by looking for the PumpFun program account
        // Pool account is the only one that should be decoded as pool data
        for (pool_account, pool_data) in accounts.iter() {
            // Skip non-pool accounts (vaults, mints) by checking size and owner
            if pool_data.data.len() < 200 {
                continue; // Too small to be a pool account
            }

            // Check if this is the actual pool account by looking for PumpFun AMM program ownership
            if pool_data.owner.to_string() != "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" {
                continue; // Not owned by PumpFun AMM program
            }

            if let Some(pool_info) = Self::decode_pump_fun_amm_pool(&pool_data.data) {
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Successfully decoded PumpFun pool: {} -> {}",
                        pool_info.base_mint, pool_info.quote_mint
                    ),
                );

                // Calculate price using the extracted pool info
                if let Some(price_result) = Self::calculate_pump_fun_amm_price(
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

impl PumpFunAmmDecoder {
    /// Extract mints and vaults from PumpFun AMM pool data
    fn extract_pumpfun_mints_and_vaults(data: &[u8]) -> Option<PoolMintVaultInfo> {
        if data.len() < 200 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("PumpFun pool data too short: {} bytes", data.len()),
            );
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!("Extracting PumpFun pool data ({} bytes)", data.len()),
        );

        // PumpFun AMM structure (confirmed via structure analysis):
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
                "Extracted PumpFun: mint1={}, mint2={}, vault1={}, vault2={}",
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

    /// Decode PumpFun AMM pool data from account bytes using centralized utilities
    fn decode_pump_fun_amm_pool(data: &[u8]) -> Option<PumpFunAmmPoolInfo> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Decoding PumpFun pool data ({} bytes)", data.len()),
        );

        // Extract mints and vaults using local extraction method
        let pool_info = Self::extract_pumpfun_mints_and_vaults(data)?;

        // Validate this is a SOL-based pool and get normalized token pair info
        let pair_info = match validate_sol_pool(pool_info) {
            Ok(info) => info,
            Err(e) => {
                logger::warning(
                    LogTag::PoolDecoder,
                    &format!("PumpFun pool validation failed: {e}"),
                );
                return None;
            }
        };

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Valid PumpFun SOL pool: token={}, sol_is_first={}, token_vault={}, sol_vault={}",
                &pair_info.token_mint[..8],
                pair_info.sol_is_first,
                &pair_info.token_vault[..8],
                &pair_info.sol_vault[..8]
            ),
        );

        // Extract LP supply from the pool data
        let lp_supply = extract_lp_supply(data).unwrap_or_default();

        // The programme prices off the quote vault plus this signed virtual reserve;
        // a pool account too short to carry it cannot be priced the way it trades.
        let Some(state) = PumpAmmPoolState::decode(Pubkey::default(), data) else {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun pool account ({} bytes) does not carry the virtual quote reserve",
                    data.len()
                ),
            );
            return None;
        };

        Some(PumpFunAmmPoolInfo {
            base_mint: pair_info.token_mint,
            quote_mint: pair_info.sol_mint,
            pool_base_token_account: pair_info.token_vault,
            pool_quote_token_account: pair_info.sol_vault,
            lp_supply,
            virtual_quote_reserves: state.virtual_quote_reserves,
            sol_is_pool_quote: !pair_info.sol_is_first,
        })
    }

    #[cfg(test)]
    fn build_pool_bytes(
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        vault1: &Pubkey,
        vault2: &Pubkey,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 301];
        let mut offset = 43usize; // discriminator(8) + bump(1) + index(2) + creator(32)
        data[offset..offset + 32].copy_from_slice(base_mint.as_ref());
        offset += 32;
        data[offset..offset + 32].copy_from_slice(quote_mint.as_ref());
        offset += 32;
        offset += 32; // lp_mint, skipped
        data[offset..offset + 32].copy_from_slice(vault1.as_ref());
        offset += 32;
        data[offset..offset + 32].copy_from_slice(vault2.as_ref());
        data
    }

    /// Calculate price for PumpFun AMM pool
    fn calculate_pump_fun_amm_price(
        pool_info: &PumpFunAmmPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
        pool_account: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Calculating PumpFun price for token {} with quote {}",
                base_mint, quote_mint
            ),
        );

        // For PUMP.FUN, SOL should be the quote token (we've already ensured this in decode_pump_fun_amm_pool)
        let sol_mint_str = SOL_MINT;
        if pool_info.quote_mint != sol_mint_str {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun pool does not contain SOL as quote. Quote: {}",
                    pool_info.quote_mint
                ),
            );
            return None;
        }

        // Use the base mint (token) from the pool - this is the token we'll calculate price for
        let target_mint = pool_info.base_mint.clone();

        logger::debug(
            LogTag::PoolDecoder,
            &format!("Using target mint: {target_mint}"),
        );

        // Get token reserves from vault accounts
        let token_reserve =
            Self::get_vault_balance_from_accounts(accounts, &pool_info.pool_base_token_account)?;
        let sol_reserve =
            Self::get_vault_balance_from_accounts(accounts, &pool_info.pool_quote_token_account)?;

        logger::verbose(
            LogTag::PoolDecoder,
            &format!(
                "Raw reserves - Token: {}, SOL: {}",
                token_reserve, sol_reserve
            ),
        );

        // Reserve validation
        if token_reserve == 0 || sol_reserve == 0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Zero reserves detected - Token: {}, SOL: {}",
                    token_reserve, sol_reserve
                ),
            );
            return None;
        }

        // Get token decimals - CRITICAL: must be available, no assumptions
        let token_decimals = match get_cached_decimals(crate::chains::ChainId::Solana, &target_mint)
        {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "No decimals found for PumpFun token: {}, skipping pool calculation",
                        target_mint
                    ),
                );
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Adjust for decimals
        if token_decimals > 18 || sol_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun AMM: Decimals too large: sol={}, token={}",
                    sol_decimals, token_decimals
                ),
            );
            return None;
        }
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);

        let Some((pricing_token, pricing_sol)) = pricing_reserves(
            token_reserve,
            sol_reserve,
            pool_info.virtual_quote_reserves,
            pool_info.sol_is_pool_quote,
        ) else {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun pool {}: virtual quote reserve {} leaves no positive reserve",
                    pool_account, pool_info.virtual_quote_reserves
                ),
            );
            return None;
        };

        let price_sol = (pricing_sol as f64 / (10_f64).powi(sol_decimals as i32))
            / (pricing_token as f64 / (10_f64).powi(token_decimals as i32));

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
                "PumpFun price calculation:\n\
            - SOL Reserve: {} (decimals: {}, adjusted: {:.12})\n\
            - Token Reserve: {} (decimals: {}, adjusted: {:.12})\n\
            - Price SOL: {:.12}\n\
            - Target Token: {}",
                sol_reserve,
                sol_decimals,
                sol_adjusted,
                token_reserve,
                token_decimals,
                token_adjusted,
                price_sol,
                target_mint
            ),
        );

        Some(PriceResult {
            mint: target_mint,
            price_usd: 0.0, // We don't calculate USD price here
            price_native: price_sol,
            confidence: 0.9, // High confidence for PumpFun pools
            source_pool: Some("PumpFun".to_owned()),
            pool_address: pool_account.to_string(),
            slot: 0, // Would need to be passed from the calling context
            timestamp: Instant::now(),
            native_reserves: sol_adjusted,
            token_reserves: token_adjusted,
        })
    }

    /// Get token balance from vault account
    fn get_vault_balance_from_accounts(
        accounts: &HashMap<String, AccountData>,
        vault_account: &str,
    ) -> Option<u64> {
        let vault_data = accounts.get(vault_account)?;

        if vault_data.data.len() < 72 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Vault account {} has insufficient data: {} bytes",
                    &vault_account[..8],
                    vault_data.data.len()
                ),
            );
            return None;
        }

        match Self::decode_token_account_amount(&vault_data.data) {
            Some(amount) => {
                logger::verbose(
                    LogTag::PoolDecoder,
                    &format!("Vault {} balance: {}", &vault_account[..8], amount),
                );
                Some(amount)
            }
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!("Failed to decode vault balance for {}", &vault_account[..8]),
                );
                None
            }
        }
    }

    /// Decode token account amount from account data
    fn decode_token_account_amount(data: &[u8]) -> Option<u64> {
        if data.len() < 72 {
            return None;
        }

        // Token account layout: mint(32) + owner(32) + amount(8) + ...
        let amount_offset = 64;
        let amount_bytes = &data[amount_offset..amount_offset + 8];
        Some(u64::from_le_bytes(amount_bytes.try_into().ok()?))
    }
}

/// Extract LP supply from pool data (helper function)
fn extract_lp_supply(data: &[u8]) -> Option<u64> {
    // LP supply is typically at a fixed offset after all the pubkeys
    // For PumpFun: discriminator(8) + pool_bump(1) + index(2) + creator(32) + creator(32) +
    // base_mint(32) + quote_mint(32) + lp_mint(32) + vault1(32) + vault2(32) = 235 bytes
    let lp_supply_offset = 8 + 1 + 2 + 32 + 32 + 32 + 32 + 32 + 32 + 32; // 235

    (if data.len() >= lp_supply_offset + 8 {
        let lp_supply_bytes = &data[lp_supply_offset..lp_supply_offset + 8];
        u64::from_le_bytes(lp_supply_bytes.try_into().ok()?)
    } else {
        0
    })
    .into()
}

/// PumpFun AMM pool information extracted from account data
#[derive(Debug, Clone)]
pub struct PumpFunAmmPoolInfo {
    pub base_mint: String,
    pub quote_mint: String,
    pub pool_base_token_account: String,
    pub pool_quote_token_account: String,
    pub lp_supply: u64,
    /// Signed virtual reserve the programme adds to the pool's quote side.
    pub virtual_quote_reserves: i128,
    /// Whether SOL is the pool's quote side, the side the virtual reserve joins.
    pub sol_is_pool_quote: bool,
}

/// The raw `(token, sol)` reserves the programme prices against: the vault
/// balances with the signed virtual reserve added to the pool's quote side.
/// `None` when either side is not positive.
fn pricing_reserves(
    token_reserve: u64,
    sol_reserve: u64,
    virtual_quote_reserves: i128,
    sol_is_pool_quote: bool,
) -> Option<(u128, u128)> {
    let (mut token, mut sol) = (token_reserve as i128, sol_reserve as i128);
    if sol_is_pool_quote {
        sol = sol.checked_add(virtual_quote_reserves)?;
    } else {
        token = token.checked_add(virtual_quote_reserves)?;
    }
    if token <= 0 || sol <= 0 {
        return None;
    }
    Some((token as u128, sol as u128))
}

impl PumpFunAmmDecoder {
    /// Extract reserve account addresses from PumpFun AMM pool data
    pub fn extract_reserve_accounts(data: &[u8]) -> Option<Vec<String>> {
        // Use the local extraction method for consistent SOL detection
        let pool_info = Self::extract_pumpfun_mints_and_vaults(data)?;

        // Get vaults in the correct order for the decoder
        let vault_addresses =
            crate::chains::solana::pools::decode_utils::get_analyzer_vault_order(pool_info);

        if vault_addresses.is_empty() {
            return None;
        }

        Some(vault_addresses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn short_data_is_rejected_without_panicking() {
        assert!(PumpFunAmmDecoder::extract_pumpfun_mints_and_vaults(&[]).is_none());
        assert!(PumpFunAmmDecoder::extract_pumpfun_mints_and_vaults(&vec![0u8; 199]).is_none());
        assert!(PumpFunAmmDecoder::decode_pump_fun_amm_pool(&vec![0u8; 199]).is_none());
    }

    #[test]
    fn valid_pool_normalizes_sol_as_quote_and_extracts_vaults() {
        let base_mint = Pubkey::new_unique();
        let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();
        let vault1 = Pubkey::new_unique();
        let vault2 = Pubkey::new_unique();
        let data = PumpFunAmmDecoder::build_pool_bytes(&base_mint, &sol_mint, &vault1, &vault2);

        let info = PumpFunAmmDecoder::decode_pump_fun_amm_pool(&data)
            .expect("well-formed PumpFun AMM pool data must decode");

        assert_eq!(info.base_mint, base_mint.to_string());
        assert_eq!(info.quote_mint, SOL_MINT);
        assert_eq!(info.pool_base_token_account, vault1.to_string());
        assert_eq!(info.pool_quote_token_account, vault2.to_string());
    }

    #[test]
    fn the_virtual_reserve_joins_the_pools_quote_side_only() {
        // SOL quoted: a boost adds to the SOL side, an unboosted pool's negative
        // value removes the unclaimed fees from it.
        assert_eq!(
            pricing_reserves(1_000, 2_000, 500, true),
            Some((1_000, 2_500))
        );
        assert_eq!(
            pricing_reserves(1_000, 2_000, -500, true),
            Some((1_000, 1_500))
        );
        // SOL as the pool's base: the token is the quote side.
        assert_eq!(
            pricing_reserves(1_000, 2_000, 500, false),
            Some((1_500, 2_000))
        );
        // A virtual reserve that drains the quote side leaves nothing to price.
        assert_eq!(pricing_reserves(1_000, 2_000, -2_000, true), None);
    }

    #[test]
    fn the_virtual_quote_reserve_is_read_from_the_pool_account() {
        let base_mint = Pubkey::new_unique();
        let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();
        let mut data = PumpFunAmmDecoder::build_pool_bytes(
            &base_mint,
            &sol_mint,
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
        );
        data[245..261].copy_from_slice(&(-42i128).to_le_bytes());
        let info = PumpFunAmmDecoder::decode_pump_fun_amm_pool(&data).expect("decodes");
        assert_eq!(info.virtual_quote_reserves, -42);
        assert!(info.sol_is_pool_quote);

        assert!(
            PumpFunAmmDecoder::decode_pump_fun_amm_pool(&data[..250]).is_none(),
            "an account without the virtual reserve is not priced"
        );
    }

    #[test]
    fn pool_with_no_sol_leg_is_rejected() {
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        let vault1 = Pubkey::new_unique();
        let vault2 = Pubkey::new_unique();
        let data = PumpFunAmmDecoder::build_pool_bytes(&mint_a, &mint_b, &vault1, &vault2);

        assert!(PumpFunAmmDecoder::decode_pump_fun_amm_pool(&data).is_none());
    }
}
