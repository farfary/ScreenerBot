use super::super::utils::read_pubkey_at;
/// Raydium Legacy AMM decoder
///
/// Ported minimal working logic from old pool system (pool_old.rs) lines ~6885-7210.
/// Uses fixed offsets discovered via hex analysis to locate mints and vaults.
/// Strategy:
/// - Identify pool account by size (>= 752 bytes in old code; we relax to >= 600)
/// - Parse vault + mint pubkeys at legacy offsets
/// - Fetch vault token account balances from provided accounts map
/// - Compute SOL price for target token
use super::{AccountData, PoolDecoder};
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{log, LogTag};
use crate::pools::types::{PriceResult, ProgramKind, SOL_MINT};
use crate::constants::SOL_DECIMALS;
use crate::tokens::get_cached_decimals;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

pub struct RaydiumLegacyAmmDecoder;

impl PoolDecoder for RaydiumLegacyAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumLegacyAmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        // Pick pool account (largest length heuristic)
        let pool_account = accounts.values().max_by_key(|a| a.data.len())?;
        let pool_data = &pool_account.data;
        if pool_data.len() < 600 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("Legacy AMM pool data too small: {}", pool_data.len()),
                );
            }
            return None;
        }
        let info = LegacyPoolInfo::parse(pool_data)?;
        // Adjust vaults if initial offsets missing in accounts map
        let adjusted = adjust_vaults(&info, accounts);
        let info = adjusted.unwrap_or(info);

        // Determine target token mint
        let target_mint = if info.coin_mint == base_mint {
            base_mint
        } else if info.coin_mint == quote_mint {
            quote_mint
        } else {
            &info.coin_mint
        }; // fallback token mint

        // Fetch reserves from vault token accounts (must be present)
        let coin_reserve = get_token_account_amount(accounts, &info.coin_vault);
        let pc_reserve = get_token_account_amount(accounts, &info.pc_vault);

        // If vault fetch failed, try extracting reserves directly from pool data
        let (coin_reserve, pc_reserve) = match (coin_reserve, pc_reserve) {
            (Some(c), Some(p)) => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "INFO",
                        &format!("Using vault reserves: coin={} pc={}", c, p),
                    );
                }
                (c, p)
            }
            _ => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "WARN",
                        "Vault fetch failed, extracting reserves from pool data",
                    );
                }
                extract_reserves_from_pool_data(pool_data)?
            }
        };

        // Map SOL vs token - CRITICAL: decimals must be cached, no fallback
        let (sol_reserve_raw, token_reserve_raw, token_decimals) = if info.pc_mint == SOL_MINT {
            // pc=SOL vault at pc_vault, coin=token vault at coin_vault
            let decimals = match get_cached_decimals(&info.coin_mint) {
                Some(decimals) => decimals,
                None => {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "ERROR",
                            &format!(
                                "Legacy AMM: Token decimals not found for {}, skipping price calculation",
                                info.coin_mint
                            )
                        );
                    }
                    return None;
                }
            };
            (pc_reserve, coin_reserve, decimals)
        } else if info.coin_mint == SOL_MINT {
            let decimals = match get_cached_decimals(&info.pc_mint) {
                Some(decimals) => decimals,
                None => {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "ERROR",
                            &format!(
                                "Legacy AMM: Token decimals not found for {}, skipping price calculation",
                                info.pc_mint
                            )
                        );
                    }
                    return None;
                }
            };
            (coin_reserve, pc_reserve, decimals)
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    "Legacy AMM pool missing SOL mint",
                );
            }
            return None;
        };

        if sol_reserve_raw == 0 || token_reserve_raw == 0 {
            return None;
        }
        let sol_adjusted = (sol_reserve_raw as f64) / (10f64).powi(SOL_DECIMALS as i32);
        let token_adjusted = (token_reserve_raw as f64) / (10f64).powi(token_decimals as i32);
        if token_adjusted <= 0.0 {
            return None;
        }
        let price_sol = sol_adjusted / token_adjusted;
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            return None;
        }

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "SUCCESS",
                &format!(
                    "Legacy AMM price: {:.12} SOL (sol_reserve={} token_reserve={} token_dec={} coin_mint={} pc_mint={})",
                    price_sol,
                    sol_adjusted,
                    token_adjusted,
                    token_decimals,
                    info.coin_mint,
                    info.pc_mint
                )
            );
        }

        Some(PriceResult::new(
            target_mint.to_string(),
            0.0,
            price_sol,
            sol_adjusted,
            token_adjusted,
            pool_account.pubkey.to_string(),
        ))
    }
}

impl RaydiumLegacyAmmDecoder {
    /// Extract reserve account addresses from Legacy AMM pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [coin_vault, pc_vault]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 0x1c0 {
            return None;
        }
        let mut out = Vec::new();
        for off in [0x150usize, 0x160, 0x170, 0x180] {
            if let Some(pk) = read_pubkey_at(pool_data, off) {
                out.push(pk);
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

struct LegacyPoolInfo {
    coin_mint: String,
    pc_mint: String,
    coin_vault: String,
    pc_vault: String,
}

impl LegacyPoolInfo {
    fn parse(data: &[u8]) -> Option<Self> {
        // Offsets from legacy implementation
        if data.len() < 0x1c0 {
            return None;
        }
        // Extract mints and vaults from fixed offsets, but don't assume which is which
        let vault_a = read_pubkey_at(data, 0x150)?; // vault at 0x150
        let vault_b = read_pubkey_at(data, 0x160)?; // vault at 0x160
        let mint_a = read_pubkey_at(data, 0x190)?; // mint at 0x190
        let mint_b = read_pubkey_at(data, 0x1b0)?; // mint at 0x1b0

        // Determine which mint is SOL and which is token
        let (coin_mint, pc_mint, coin_vault, pc_vault) = if mint_a == SOL_MINT {
            // mint_a is SOL, mint_b is token
            (mint_b, mint_a, vault_b, vault_a)
        } else if mint_b == SOL_MINT {
            // mint_b is SOL, mint_a is token
            (mint_a, mint_b, vault_a, vault_b)
        } else {
            // Neither mint is SOL, use original assumption (mint_a=token, mint_b=SOL)
            // This handles wrapped SOL cases
            (mint_a, mint_b, vault_a, vault_b)
        };

        Some(Self {
            coin_mint,
            pc_mint,
            coin_vault,
            pc_vault,
        })
    }
}

fn get_token_account_amount(accounts: &HashMap<String, AccountData>, key: &str) -> Option<u64> {
    let acc = accounts.get(key)?;
    if acc.data.len() < 72 {
        return None;
    }
    let amount = u64::from_le_bytes(acc.data[64..72].try_into().ok()?);
    Some(amount)
}

fn adjust_vaults(
    info: &LegacyPoolInfo,
    accounts: &HashMap<String, AccountData>,
) -> Option<LegacyPoolInfo> {
    let mut coin_vault = info.coin_vault.clone();
    let mut pc_vault = info.pc_vault.clone();
    let need_coin = !accounts.contains_key(&coin_vault);
    let need_pc = !accounts.contains_key(&pc_vault);

    // Check if existing vaults have wrong mints
    let mut coin_vault_wrong_mint = false;
    let mut pc_vault_wrong_mint = false;

    if !need_coin {
        if let Some(acc) = accounts.get(&coin_vault) {
            if acc.data.len() >= 32 {
                if let Ok(mint_bytes) = acc.data[0..32].try_into() {
                    let mint = Pubkey::new_from_array(mint_bytes).to_string();
                    if mint != info.coin_mint {
                        coin_vault_wrong_mint = true;
                        if is_debug_pool_decoders_enabled() {
                            log(
                                LogTag::PoolDecoder,
                                "WARN",
                                &format!(
                                    "coin_vault {} has wrong mint {} expected {}",
                                    coin_vault, mint, info.coin_mint
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    if !need_pc {
        if let Some(acc) = accounts.get(&pc_vault) {
            if acc.data.len() >= 32 {
                if let Ok(mint_bytes) = acc.data[0..32].try_into() {
                    let mint = Pubkey::new_from_array(mint_bytes).to_string();
                    if mint != info.pc_mint {
                        pc_vault_wrong_mint = true;
                        if is_debug_pool_decoders_enabled() {
                            log(
                                LogTag::PoolDecoder,
                                "WARN",
                                &format!(
                                    "pc_vault {} has wrong mint {} expected {}",
                                    pc_vault, mint, info.pc_mint
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    let need_adjustment = need_coin || need_pc || coin_vault_wrong_mint || pc_vault_wrong_mint;
    if !need_adjustment {
        return None;
    }

    if is_debug_pool_decoders_enabled() {
        log(
            LogTag::PoolDecoder,
            "DEBUG",
            &format!(
                "adjust_vaults: need_coin={} need_pc={} wrong_coin_mint={} wrong_pc_mint={}",
                need_coin || coin_vault_wrong_mint,
                need_pc || pc_vault_wrong_mint,
                coin_vault_wrong_mint,
                pc_vault_wrong_mint
            ),
        );
    }

    // Build map from mint->vault pubkey where account present
    for (k, acc) in accounts {
        // Only consider accounts that look like token accounts (>=80 bytes)
        if acc.data.len() >= 80 {
            if let Ok(mint_bytes) = acc.data[0..32].try_into() {
                let mint = Pubkey::new_from_array(mint_bytes).to_string();
                if mint == info.coin_mint && (need_coin || coin_vault_wrong_mint) {
                    coin_vault = k.clone();
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "DEBUG",
                            &format!("Found coin_vault: {}", coin_vault),
                        );
                    }
                }
                if mint == info.pc_mint && (need_pc || pc_vault_wrong_mint) {
                    pc_vault = k.clone();
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "DEBUG",
                            &format!("Found pc_vault: {}", pc_vault),
                        );
                    }
                }
            }
        }
    }

    // Ensure we don't use the same vault for both (emergency fallback)
    if coin_vault == pc_vault {
        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "ERROR",
                "Same vault found for both coin and pc - this will cause incorrect pricing",
            );
        }
        return None;
    }

    if is_debug_pool_decoders_enabled() {
        log(
            LogTag::PoolDecoder,
            "INFO",
            &format!(
                "Adjusted vaults: coin_vault={} pc_vault={}",
                coin_vault, pc_vault
            ),
        );
    }

    Some(LegacyPoolInfo {
        coin_mint: info.coin_mint.clone(),
        pc_mint: info.pc_mint.clone(),
        coin_vault,
        pc_vault,
    })
}

/// Extract reserves directly from pool data when vault fetch fails
/// Based on offsets found in working pool_old.rs implementation
fn extract_reserves_from_pool_data(data: &[u8]) -> Option<(u64, u64)> {
    // Promising reserve offsets from pool_old.rs analysis
    let promising_offsets = [
        (208, 216), // Primary candidate: quoteTotalPnl, baseTotalPnl
        (256, 272), // Secondary alternative
        (288, 296), // Backup alternative
    ];

    for &(offset1, offset2) in &promising_offsets {
        if offset1 + 8 <= data.len() && offset2 + 8 <= data.len() {
            if let (Ok(reserve1_bytes), Ok(reserve2_bytes)) = (
                data[offset1..offset1 + 8].try_into(),
                data[offset2..offset2 + 8].try_into(),
            ) {
                let reserve1 = u64::from_le_bytes(reserve1_bytes);
                let reserve2 = u64::from_le_bytes(reserve2_bytes);

                // For Raydium Legacy with substantial liquidity, reserves should be significant
                if reserve1 > 10_000_000
                    && reserve1 < 1_000_000_000_000_000
                    && reserve2 > 10_000_000
                    && reserve2 < 1_000_000_000_000_000
                {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "INFO",
                            &format!(
                                "Found pool data reserves at offsets {} and {}: {} and {}",
                                offset1, offset2, reserve1, reserve2
                            ),
                        );
                    }
                    // Return (coin_reserve, pc_reserve) - token first, SOL second based on size heuristic
                    return if reserve1 < reserve2 {
                        Some((reserve1, reserve2)) // Smaller value likely token, larger SOL
                    } else {
                        Some((reserve2, reserve1)) // Ensure token gets smaller reserve
                    };
                }
            }
        }
    }
    None
}
