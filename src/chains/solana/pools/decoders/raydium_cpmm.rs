// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raydium CPMM (CP-Swap) decoder
//!
//! Decodes the pool with the shared `PoolState` layout
//! (`crate::chains::solana::pools::layouts::raydium_cpmm`), reads the two vaults it
//! names and prices the token from the tradable reserves: each vault less the
//! protocol, fund and creator fees still held in it. A pool whose status does not
//! permit swapping, a vault missing from the bundle, a vault holding a different
//! mint than the pool names for that side, or a token whose cached decimals
//! disagree with the decimals the pool recorded yields no price.

use super::{AccountData, PoolDecoder};

use crate::chains::solana::constants::{RAYDIUM_CPMM_PROGRAM_ID, SOL_MINT};
use crate::chains::solana::layout::{pubkey_at, token_account_amount};
use crate::chains::solana::pools::layouts::raydium_cpmm::CpmmPoolState;
use crate::chains::solana::pools::types::ProgramKind;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

/// Raydium CPMM decoder implementation
pub struct RaydiumCpmmDecoder;

impl PoolDecoder for RaydiumCpmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumCpmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        _base_mint: &str,
        _quote_mint: &str,
    ) -> Option<PriceResult> {
        let pool_account = accounts
            .values()
            .find(|a| a.owner.to_string() == RAYDIUM_CPMM_PROGRAM_ID)?;
        let Some(state) = CpmmPoolState::decode(pool_account.pubkey, &pool_account.data) else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "CPMM pool {} does not match the PoolState layout ({} bytes)",
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
                    "CPMM pool {} status {} does not permit swapping",
                    state.pool, state.status
                ),
            );
            return None;
        }

        let balance_0 = vault_balance(accounts, &state.pool, &state.vault_0, &state.mint_0)?;
        let balance_1 = vault_balance(accounts, &state.pool, &state.vault_1, &state.mint_1)?;
        let (reserve_0, reserve_1) = state.tradable_reserves(balance_0, balance_1);

        let mint_0 = state.mint_0.to_string();
        let mint_1 = state.mint_1.to_string();
        let (sol_raw, sol_decimals, token_raw, token_decimals, token_mint) = if mint_0 == SOL_MINT {
            (
                reserve_0,
                state.decimals_0,
                reserve_1,
                state.decimals_1,
                mint_1,
            )
        } else if mint_1 == SOL_MINT {
            (
                reserve_1,
                state.decimals_1,
                reserve_0,
                state.decimals_0,
                mint_0,
            )
        } else {
            logger::error(
                LogTag::PoolDecoder,
                &format!("CPMM pool {} has no SOL side", state.pool),
            );
            return None;
        };

        // CRITICAL: decimals must be cached, no fallback
        let Some(cached_decimals) =
            get_cached_decimals(crate::chains::ChainId::Solana, &token_mint)
        else {
            logger::error(
                LogTag::PoolDecoder,
                &format!("CPMM: Token decimals not found for {token_mint}, skipping price"),
            );
            return None;
        };
        if cached_decimals != token_decimals {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "CPMM pool {}: cached decimals {} for {} disagree with the pool's {}",
                    state.pool, cached_decimals, token_mint, token_decimals
                ),
            );
            return None;
        }

        if sol_raw == 0 || token_raw == 0 {
            return None;
        }
        if sol_decimals > 18 || token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Raydium CPMM: Decimals too large: sol={}, token={}",
                    sol_decimals, token_decimals
                ),
            );
            return None;
        }
        let sol_adjusted = (sol_raw as f64) / (10f64).powi(sol_decimals as i32);
        let token_adjusted = (token_raw as f64) / (10f64).powi(token_decimals as i32);
        let price_sol = sol_adjusted / token_adjusted;
        if !price_sol.is_finite() || price_sol <= 0.0 || price_sol > 1_000_000.0 {
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "CPMM price: {:.12} SOL (sol_reserve={} token_reserve={} token_dec={} mint_0={} mint_1={})",
                price_sol, sol_adjusted, token_adjusted, token_decimals, state.mint_0, state.mint_1
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
            &format!("CPMM pool {pool}: vault {vault} is not in the account bundle"),
        );
        return None;
    };
    let held = pubkey_at(&account.data, 0)?;
    if held != *mint {
        logger::warning(
            LogTag::PoolDecoder,
            &format!("CPMM pool {pool}: vault {vault} holds mint {held}, the pool names {mint}"),
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
        pool: Pubkey,
        vault_1: Pubkey,
        token_mint: Pubkey,
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

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// A swappable WSOL/token pool (token_0 = WSOL, 9 dp; token_1 = the token,
    /// 6 dp) holding 2 SOL against 1,000 tokens, with no fees awaiting collection.
    fn pool() -> Pool {
        let pool = Pubkey::new_unique();
        let vault_0 = Pubkey::new_unique();
        let vault_1 = Pubkey::new_unique();
        let wsol = Pubkey::from_str(SOL_MINT).unwrap();
        let token_mint = Pubkey::new_unique();

        let mut data = vec![0u8; 637];
        data[72..104].copy_from_slice(vault_0.as_ref());
        data[104..136].copy_from_slice(vault_1.as_ref());
        data[168..200].copy_from_slice(wsol.as_ref());
        data[200..232].copy_from_slice(token_mint.as_ref());
        data[331] = 9;
        data[332] = 6;
        crate::tokens::cache_decimals(crate::chains::ChainId::Solana, &token_mint.to_string(), 6);

        let program = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap();
        let token_program = crate::chains::solana::spl_token::id();
        let accounts = HashMap::from([
            (pool.to_string(), account(pool, program, data)),
            (
                vault_0.to_string(),
                account(vault_0, token_program, token_account(&wsol, 2_000_000_000)),
            ),
            (
                vault_1.to_string(),
                account(
                    vault_1,
                    token_program,
                    token_account(&token_mint, 1_000_000_000),
                ),
            ),
        ]);
        Pool {
            accounts,
            pool,
            vault_1,
            token_mint,
        }
    }

    fn decode(pool: &Pool) -> Option<PriceResult> {
        RaydiumCpmmDecoder::decode_and_calculate(&pool.accounts, "", "")
    }

    fn pool_data(pool: &mut Pool) -> &mut Vec<u8> {
        &mut pool.accounts.get_mut(&pool.pool.to_string()).unwrap().data
    }

    #[test]
    fn the_price_is_sol_per_token_from_the_vaults_the_pool_names() {
        let p = pool();
        let price = decode(&p).expect("a swappable pool prices");
        assert_eq!(price.mint, p.token_mint.to_string());
        assert!((price.price_native - 0.002).abs() < 1e-12);
        assert!((price.native_reserves - 2.0).abs() < 1e-12);
        assert!((price.token_reserves - 1_000.0).abs() < 1e-9);
    }

    #[test]
    fn fees_awaiting_collection_are_not_priced_as_liquidity() {
        let mut p = pool();
        let data = pool_data(&mut p);
        write_u64(data, 341, 200_000_000); // protocol fees, SOL side
        write_u64(data, 357, 100_000_000); // fund fees, SOL side
        write_u64(data, 397, 200_000_000); // creator fees, SOL side
        let price = decode(&p).expect("prices");
        assert!((price.native_reserves - 1.5).abs() < 1e-12);
        assert!((price.price_native - 0.0015).abs() < 1e-12);
    }

    #[test]
    fn cached_decimals_that_disagree_with_the_pool_yield_no_price() {
        let mut p = pool();
        pool_data(&mut p)[332] = 9;
        assert!(decode(&p).is_none());
    }

    #[test]
    fn a_vault_holding_the_other_sides_mint_yields_no_price() {
        let mut p = pool();
        let wsol = Pubkey::from_str(SOL_MINT).unwrap();
        p.accounts.get_mut(&p.vault_1.to_string()).unwrap().data = token_account(&wsol, 1);
        assert!(decode(&p).is_none());
    }

    #[test]
    fn a_pool_whose_status_disables_swapping_yields_no_price() {
        let mut p = pool();
        pool_data(&mut p)[329] = 1 << 2;
        assert!(decode(&p).is_none());
    }
}
