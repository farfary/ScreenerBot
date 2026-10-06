// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raydium Legacy AMM (AMM v4) decoder
//!
//! Decodes the pool with the shared `AmmInfo` layout
//! (`crate::chains::solana::pools::layouts::raydium_amm_v4`), reads the two vaults it
//! names and prices the token from the tradable reserves: each vault less the profit
//! earmarked out of it (`need_take_pnl`). A pool whose status does not permit
//! swapping, a vault missing from the bundle, or a vault holding a different mint
//! than the pool names for that side yields no price.

use super::{AccountData, PoolDecoder};

use crate::chains::solana::constants::{RAYDIUM_LEGACY_AMM_PROGRAM_ID, SOL_DECIMALS, SOL_MINT};
use crate::chains::solana::layout::{pubkey_at, token_account_amount};
use crate::chains::solana::pools::layouts::raydium_amm_v4::AmmV4PoolState;
use crate::chains::solana::pools::types::ProgramKind;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

pub struct RaydiumLegacyAmmDecoder;

impl PoolDecoder for RaydiumLegacyAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumLegacyAmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        _base_mint: &str,
        _quote_mint: &str,
    ) -> Option<PriceResult> {
        let pool_account = accounts
            .values()
            .find(|a| a.owner.to_string() == RAYDIUM_LEGACY_AMM_PROGRAM_ID)?;
        let Some(state) = AmmV4PoolState::decode(pool_account.pubkey, &pool_account.data) else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Legacy AMM pool {} does not match the AmmInfo layout ({} bytes)",
                    pool_account.pubkey,
                    pool_account.data.len()
                ),
            );
            return None;
        };
        if !state.swap_enabled() {
            logger::debug(
                LogTag::PoolDecoder,
                &format!(
                    "Legacy AMM pool {} status {} does not permit swapping",
                    state.pool, state.status
                ),
            );
            return None;
        }

        let coin_balance =
            vault_balance(accounts, &state.pool, &state.coin_vault, &state.coin_mint)?;
        let pc_balance = vault_balance(accounts, &state.pool, &state.pc_vault, &state.pc_mint)?;
        let (coin_reserve, pc_reserve) = state.tradable_reserves(coin_balance, pc_balance);

        let coin_mint = state.coin_mint.to_string();
        let pc_mint = state.pc_mint.to_string();
        let (sol_reserve_raw, token_reserve_raw, token_mint) = if pc_mint == SOL_MINT {
            (pc_reserve, coin_reserve, coin_mint)
        } else if coin_mint == SOL_MINT {
            (coin_reserve, pc_reserve, pc_mint)
        } else {
            logger::error(LogTag::PoolDecoder, "Legacy AMM pool missing SOL mint");
            return None;
        };

        // CRITICAL: decimals must be cached, no fallback
        let Some(token_decimals) = get_cached_decimals(crate::chains::ChainId::Solana, &token_mint)
        else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Legacy AMM: Token decimals not found for {}, skipping price calculation",
                    token_mint
                ),
            );
            return None;
        };

        if sol_reserve_raw == 0 || token_reserve_raw == 0 {
            return None;
        }
        if token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Raydium Legacy AMM: Token decimals too large: {}",
                    token_decimals
                ),
            );
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

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Legacy AMM price: {:.12} SOL (sol_reserve={} token_reserve={} token_dec={} coin_mint={} pc_mint={})",
                price_sol,
                sol_adjusted,
                token_adjusted,
                token_decimals,
                state.coin_mint,
                state.pc_mint
            ),
        );

        Some(PriceResult::new(
            token_mint,
            0.0,
            price_sol,
            sol_adjusted,
            token_adjusted,
            state.pool.to_string(),
        ))
    }
}

/// The balance of `vault`, refused when the bundle lacks it or when it holds a
/// different mint than `mint`, the side the pool names it for.
fn vault_balance(
    accounts: &HashMap<String, AccountData>,
    pool: &Pubkey,
    vault: &Pubkey,
    mint: &Pubkey,
) -> Option<u64> {
    let Some(account) = accounts.get(&vault.to_string()) else {
        logger::warning(
            LogTag::PoolDecoder,
            &format!("Legacy AMM pool {pool}: vault {vault} is not in the account bundle"),
        );
        return None;
    };
    let held = pubkey_at(&account.data, 0)?;
    if held != *mint {
        logger::warning(
            LogTag::PoolDecoder,
            &format!(
                "Legacy AMM pool {pool}: vault {vault} holds mint {held}, the pool names {mint}"
            ),
        );
        return None;
    }
    token_account_amount(&account.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::time::Instant;

    struct Pool {
        accounts: HashMap<String, AccountData>,
        coin_vault: Pubkey,
        pc_vault: Pubkey,
        coin_mint: Pubkey,
        pc_mint: Pubkey,
    }

    fn account(pubkey: Pubkey, owner: Pubkey, data: Vec<u8>) -> AccountData {
        AccountData {
            pubkey,
            data,
            slot: 1,
            fetched_at: Instant::now(),
            lamports: 0,
            owner,
        }
    }

    fn token_account(mint: &Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; 165];
        data[0..32].copy_from_slice(mint.as_ref());
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data
    }

    /// A swappable token/WSOL pool with both vaults in the bundle.
    fn pool(status: u64) -> Pool {
        let pool = Pubkey::new_unique();
        let coin_vault = Pubkey::new_unique();
        let pc_vault = Pubkey::new_unique();
        let coin_mint = Pubkey::new_unique();
        let pc_mint = Pubkey::from_str(SOL_MINT).unwrap();

        let mut data = vec![0u8; 752];
        data[0..8].copy_from_slice(&status.to_le_bytes());
        data[336..368].copy_from_slice(coin_vault.as_ref());
        data[368..400].copy_from_slice(pc_vault.as_ref());
        data[400..432].copy_from_slice(coin_mint.as_ref());
        data[432..464].copy_from_slice(pc_mint.as_ref());

        let program = Pubkey::from_str(RAYDIUM_LEGACY_AMM_PROGRAM_ID).unwrap();
        let token_program = crate::chains::solana::spl_token::id();
        let accounts = HashMap::from([
            (pool.to_string(), account(pool, program, data)),
            (
                coin_vault.to_string(),
                account(coin_vault, token_program, token_account(&coin_mint, 1_000)),
            ),
            (
                pc_vault.to_string(),
                account(pc_vault, token_program, token_account(&pc_mint, 2_000)),
            ),
        ]);
        Pool {
            accounts,
            coin_vault,
            pc_vault,
            coin_mint,
            pc_mint,
        }
    }

    fn decode(pool: &Pool) -> Option<PriceResult> {
        RaydiumLegacyAmmDecoder::decode_and_calculate(&pool.accounts, "", "")
    }

    #[test]
    fn a_vault_holding_the_other_sides_mint_yields_no_price() {
        let mut p = pool(6);
        let swapped = token_account(&p.coin_mint, 2_000);
        p.accounts.get_mut(&p.pc_vault.to_string()).unwrap().data = swapped;
        assert!(decode(&p).is_none());
    }

    #[test]
    fn the_vault_balances_are_read_from_the_vaults_the_pool_names() {
        let p = pool(6);
        let coin = vault_balance(&p.accounts, &Pubkey::default(), &p.coin_vault, &p.coin_mint);
        let pc = vault_balance(&p.accounts, &Pubkey::default(), &p.pc_vault, &p.pc_mint);
        assert_eq!((coin, pc), (Some(1_000), Some(2_000)));
    }

    #[test]
    fn a_vault_missing_from_the_bundle_yields_no_price() {
        let mut p = pool(6);
        p.accounts.remove(&p.coin_vault.to_string());
        assert!(decode(&p).is_none());
    }

    #[test]
    fn a_pool_whose_status_does_not_permit_swapping_yields_no_price() {
        assert!(decode(&pool(4)).is_none());
    }
}
