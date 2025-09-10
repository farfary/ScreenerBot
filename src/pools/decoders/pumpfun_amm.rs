use std::collections::HashMap;
use std::time::Instant;
use solana_sdk::pubkey::Pubkey;
use super::{ PoolDecoder, AccountData };
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT };
use crate::tokens::{ get_token_decimals_sync, decimals::SOL_DECIMALS };

// Import centralized utilities
use super::super::utils::{ validate_sol_pool, read_pubkey_at_offset, PoolMintVaultInfo };

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
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("PumpFun AMM: Processing for {} vs {}", base_mint, quote_mint)
            );
        }

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
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "DEBUG",
                        &format!(
                            "Successfully decoded PumpFun pool: {} -> {}",
                            pool_info.base_mint,
                            pool_info.quote_mint
                        )
                    );
                }

                // Calculate price using the extracted pool info
                if
                    let Some(price_result) = Self::calculate_pump_fun_amm_price(
                        &pool_info,
                        accounts,
                        base_mint,
                        quote_mint,
                        pool_account
                    )
                {
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
        use crate::arguments::is_debug_pool_service_enabled;

        if data.len() < 200 {
            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("PumpFun pool data too short: {} bytes", data.len())
                );
            }
            return None;
        }

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("Extracting PumpFun pool data ({} bytes)", data.len())
            );
        }

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

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!(
                    "Extracted PumpFun: mint1={}, mint2={}, vault1={}, vault2={}",
                    &mint1[..8],
                    &mint2[..8],
                    &vault1[..8],
                    &vault2[..8]
                )
            );
        }

        Some(PoolMintVaultInfo {
            mint1,
            mint2,
            vault1,
            vault2,
        })
    }

    /// Decode PumpFun AMM pool data from account bytes using centralized utilities
    fn decode_pump_fun_amm_pool(data: &[u8]) -> Option<PumpFunAmmPoolInfo> {
        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("Decoding PumpFun pool data ({} bytes)", data.len())
            );
        }

        // Extract mints and vaults using local extraction method
        let pool_info = Self::extract_pumpfun_mints_and_vaults(data)?;

        // Validate this is a SOL-based pool and get normalized token pair info
        let pair_info = match validate_sol_pool(pool_info) {
            Ok(info) => info,
            Err(e) => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "WARN",
                        &format!("PumpFun pool validation failed: {}", e)
                    );
                }
                return None;
            }
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "SUCCESS",
                &format!(
                    "Valid PumpFun SOL pool: token={}, sol_is_first={}, token_vault={}, sol_vault={}",
                    &pair_info.token_mint[..8],
                    pair_info.sol_is_first,
                    &pair_info.token_vault[..8],
                    &pair_info.sol_vault[..8]
                )
            );
        }

        // Extract LP supply from the pool data
        let lp_supply = extract_lp_supply(data).unwrap_or(0);

        Some(PumpFunAmmPoolInfo {
            base_mint: pair_info.token_mint,
            quote_mint: pair_info.sol_mint,
            pool_base_token_account: pair_info.token_vault,
            pool_quote_token_account: pair_info.sol_vault,
            lp_supply,
        })
    }

    /// Calculate price for PumpFun AMM pool
    fn calculate_pump_fun_amm_price(
        pool_info: &PumpFunAmmPoolInfo,
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
        pool_account: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!(
                    "Calculating PumpFun price for token {} with quote {}",
                    base_mint,
                    quote_mint
                )
            );
        }

        // For PUMP.FUN, SOL should be the quote token (we've already ensured this in decode_pump_fun_amm_pool)
        let sol_mint_str = SOL_MINT;
        if pool_info.quote_mint != sol_mint_str {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "PumpFun pool does not contain SOL as quote. Quote: {}",
                        pool_info.quote_mint
                    )
                );
            }
            return None;
        }

        // Use the base mint (token) from the pool - this is the token we'll calculate price for
        let target_mint = pool_info.base_mint.clone();

        if is_debug_pool_decoders_enabled() {
            log(LogTag::PoolDecoder, "DEBUG", &format!("Using target mint: {}", target_mint));
        }

        // Get token reserves from vault accounts
        let token_reserve = Self::get_vault_balance_from_accounts(
            accounts,
            &pool_info.pool_base_token_account
        )?;
        let sol_reserve = Self::get_vault_balance_from_accounts(
            accounts,
            &pool_info.pool_quote_token_account
        )?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("Raw reserves - Token: {}, SOL: {}", token_reserve, sol_reserve)
            );
        }

        // Reserve validation
        if token_reserve == 0 || sol_reserve == 0 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "Zero reserves detected - Token: {}, SOL: {}",
                        token_reserve,
                        sol_reserve
                    )
                );
            }
            return None;
        }

        // Get token decimals - CRITICAL: must be available, no assumptions
        let token_decimals = match get_token_decimals_sync(&target_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!("No decimals found for PumpFun token: {}, skipping pool calculation", target_mint)
                    );
                }
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Adjust for decimals
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);

        if token_adjusted <= 0.0 {
            if is_debug_pool_decoders_enabled() {
                log(LogTag::PoolDecoder, "ERROR", "Token adjusted amount is zero or negative");
            }
            return None;
        }

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
                )
            );
        }

        Some(PriceResult {
            mint: target_mint,
            price_usd: 0.0, // We don't calculate USD price here
            price_sol,
            confidence: 0.9, // High confidence for PumpFun pools
            source_pool: Some("PumpFun".to_string()),
            pool_address: pool_account.to_string(),
            slot: 0, // Would need to be passed from the calling context
            timestamp: Instant::now(),
            sol_reserves: sol_adjusted,
            token_reserves: token_adjusted,
        })
    }

    /// Get token balance from vault account
    fn get_vault_balance_from_accounts(
        accounts: &HashMap<String, AccountData>,
        vault_account: &str
    ) -> Option<u64> {
        let vault_data = accounts.get(vault_account)?;

        if vault_data.data.len() < 72 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "Vault account {} has insufficient data: {} bytes",
                        &vault_account[..8],
                        vault_data.data.len()
                    )
                );
            }
            return None;
        }

        match Self::decode_token_account_amount(&vault_data.data) {
            Some(amount) => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "DEBUG",
                        &format!("Vault {} balance: {}", &vault_account[..8], amount)
                    );
                }
                Some(amount)
            }
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!("Failed to decode vault balance for {}", &vault_account[..8])
                    );
                }
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

    (
        if data.len() >= lp_supply_offset + 8 {
            let lp_supply_bytes = &data[lp_supply_offset..lp_supply_offset + 8];
            u64::from_le_bytes(lp_supply_bytes.try_into().ok()?)
        } else {
            0
        }
    ).into()
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

impl PumpFunAmmDecoder {
    /// Extract reserve account addresses from PumpFun AMM pool data
    pub fn extract_reserve_accounts(data: &[u8]) -> Option<Vec<String>> {
        // Use the local extraction method for consistent SOL detection
        let pool_info = Self::extract_pumpfun_mints_and_vaults(data)?;

        // Get vaults in the correct order for the decoder
        let vault_addresses = super::super::utils::get_analyzer_vault_order(pool_info);

        if vault_addresses.is_empty() {
            return None;
        }

        Some(vault_addresses)
    }
}
