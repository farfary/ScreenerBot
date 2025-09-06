/// PumpFun AMM pool decoder
///
/// This module handles decoding PumpFun AMM pools.
/// PumpFun uses bonding curves and has a specific pool structure with token and SOL vaults.
/// Based on the proven logic from the old pool system.

use super::{ PoolDecoder, AccountData };
use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT };
use crate::tokens::decimals::{ get_cached_decimals, SOL_DECIMALS, DEFAULT_TOKEN_DECIMALS };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

/// PumpFun AMM decoder implementation
pub struct PumpFunAmmDecoder;

impl PoolDecoder for PumpFunAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::PumpFun]
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
                &format!("Decoding PumpFun AMM pool for {}/{}", base_mint, quote_mint)
            );
        }

        // Find the pool account (length heuristic: pool state > 200 bytes, token accounts ~165)
        let pool_account = match accounts.values().find(|a| a.data.len() >= 200) {
            Some(a) => a,
            None => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        &format!(
                            "No suitable PumpFun pool account found (accounts: {})",
                            accounts.len()
                        )
                    );
                }
                return None;
            }
        };

        // Parse pool state from account data using the proven method
        let pool_info = Self::decode_pump_fun_amm_pool(&pool_account.data)?;

        // Calculate price using the working logic from old system
        Self::calculate_pump_fun_amm_price(&pool_info, accounts, base_mint, quote_mint)
    }
}

impl PumpFunAmmDecoder {
    /// Decode PumpFun AMM pool data from account bytes (from old working system)
    fn decode_pump_fun_amm_pool(data: &[u8]) -> Option<PumpFunAmmPoolInfo> {
        if data.len() < 200 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!("Invalid PumpFun AMM pool account data length: {}", data.len())
                );
            }
            return None;
        }

        let mut offset = 8; // Skip discriminator

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!("PumpFun pool data length: {} bytes, decoding structure...", data.len())
            );
        }

        // Decode PUMP.FUN AMM pool structure based on schema:
        // pool_bump (u8), index (u16), creator (pubkey), base_mint (pubkey), quote_mint (pubkey),
        // lp_mint (pubkey), pool_base_token_account (pubkey), pool_quote_token_account (pubkey),
        // lp_supply (u64), coin_creator (pubkey)

        let _pool_bump = data[offset]; // u8
        offset += 1;

        let _index = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?); // u16
        offset += 2;

        let _creator = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // creator pubkey
        let base_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // base_mint (our token)
        let quote_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // quote_mint (SOL)
        let _lp_mint = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // lp_mint
        let pool_base_token_account = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // base token vault
        let pool_quote_token_account = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // quote token vault (SOL)

        let lp_supply = if data.len() >= offset + 8 {
            u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?)
        } else {
            0
        };
        offset += 8;

        let _coin_creator = Self::read_pubkey_at_offset(data, &mut offset).ok()?; // coin_creator

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "Extracted PumpFun pool structure:\n\
                    - Base mint (token): {}\n\
                    - Quote mint (SOL): {}\n\
                    - Base token vault: {}\n\
                    - Quote token vault: {}\n\
                    - LP supply: {}",
                    base_mint,
                    quote_mint,
                    pool_base_token_account,
                    pool_quote_token_account,
                    lp_supply
                )
            );
        }

        Some(PumpFunAmmPoolInfo {
            base_mint,
            quote_mint,
            pool_base_token_account,
            pool_quote_token_account,
            lp_supply,
        })
    }

    /// Calculate price for PumpFun AMM pool (from old working system)
    fn calculate_pump_fun_amm_price(
        pool_info: &PumpFunAmmPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "Calculating PumpFun price for token {} with quote {}",
                    base_mint,
                    quote_mint
                )
            );
        }

        // For PUMP.FUN, SOL is always the quote token
        let sol_mint_str = SOL_MINT;
        if pool_info.quote_mint != sol_mint_str {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!(
                        "PumpFun pool does not contain SOL as quote. Quote: {}",
                        pool_info.quote_mint
                    )
                );
            }
            return None;
        }

        // Determine target token - should be the base mint in PumpFun
        let target_mint = if pool_info.base_mint == base_mint {
            base_mint.to_string()
        } else if pool_info.base_mint == quote_mint {
            quote_mint.to_string()
        } else {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!(
                        "PumpFun pool base mint {} does not match requested tokens {}/{}",
                        pool_info.base_mint,
                        base_mint,
                        quote_mint
                    )
                );
            }
            return None;
        };

        // Get vault balances from fetched account data
        let token_reserve = Self::get_vault_balance_from_accounts(
            accounts,
            &pool_info.pool_base_token_account
        )?;
        let sol_reserve = Self::get_vault_balance_from_accounts(
            accounts,
            &pool_info.pool_quote_token_account
        )?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "DEBUG",
                &format!(
                    "Successfully fetched PumpFun vault balances:\n\
                    - Token vault {} balance: {}\n\
                    - SOL vault {} balance: {}",
                    pool_info.pool_base_token_account,
                    token_reserve,
                    pool_info.pool_quote_token_account,
                    sol_reserve
                )
            );
        }

        // Get token decimals - use cached decimals for target token
        let target_token_decimals = get_cached_decimals(&target_mint).unwrap_or(
            DEFAULT_TOKEN_DECIMALS
        );
        let sol_decimals = SOL_DECIMALS;

        // Validate reserves - for pump.fun, we might have placeholder values
        // If reserves are the placeholders we set (1000000 and 1000), or zero, skip calculation
        if
            (sol_reserve == 1000 && token_reserve == 1_000_000) ||
            sol_reserve == 0 ||
            token_reserve == 0
        {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!(
                        "PumpFun pool has placeholder/zero reserves (SOL: {}, Token: {}), skipping calculation",
                        sol_reserve,
                        token_reserve
                    )
                );
            }
            return None;
        }

        // Calculate price in SOL: price = (SOL reserves / 10^SOL_decimals) / (token reserves / 10^token_decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(target_token_decimals as i32);

        if token_adjusted <= 0.0 {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "ERROR", "Token adjusted amount is zero or negative");
            }
            return None;
        }

        let price_sol = sol_adjusted / token_adjusted;

        // Validate price is reasonable
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!("Invalid price calculated: {:.12} SOL", price_sol)
                );
            }
            return None;
        }

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "SUCCESS",
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
                    target_token_decimals,
                    token_adjusted,
                    price_sol,
                    target_mint
                )
            );
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
        let account_data = match accounts.get(vault_address) {
            Some(a) => a,
            None => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        &format!(
                            "Vault account {} not present in accounts map ({} keys)",
                            vault_address,
                            accounts.len()
                        )
                    );
                }
                return None;
            }
        };
        match Self::decode_token_account_amount(&account_data.data) {
            Ok(v) => Some(v),
            Err(e) => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        &format!(
                            "Failed to decode token account {}: {} (len={})",
                            vault_address,
                            e,
                            account_data.data.len()
                        )
                    );
                }
                None
            }
        }
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
}

/// PumpFun AMM pool information extracted from account data
#[derive(Debug, Clone)]
pub struct PumpFunAmmPoolInfo {
    pub base_mint: String,
    pub quote_mint: String,
    pub pool_base_token_account: String,
    pub pool_quote_token_account: String,
    pub lp_supply: u64,
}
