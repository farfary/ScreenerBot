// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora DAMM v2 (cp-amm) pool decoder
//!
//! Decodes the pool with the shared `Pool` layout
//! (`crate::chains::solana::pools::layouts::meteora_damm`). The price comes from
//! `sqrt_price` (Q64.64, raw token B per raw token A), so the human price of B in A is
//! `(sqrt_price / 2^64)^2 * 10^(decimals_a - decimals_b)`, inverted when WSOL is token A.
//! A pool without active liquidity, or whose `sqrt_price` is zero or outside its own
//! `[sqrt_min_price, sqrt_max_price]` range, yields no price: its stored sqrt price is
//! the last traded one, not a live market. The reserves are the raw vault balances,
//! which also hold unclaimed protocol fees, so they are reported but never priced from.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::METEORA_DAMM_PROGRAM_ID;
use crate::chains::solana::constants::SOL_DECIMALS;
use crate::chains::solana::layout::token_account_amount;
use crate::chains::solana::pools::decode_utils::is_sol_mint;
use crate::chains::solana::pools::layouts::meteora_damm::DammPoolState;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;
use std::time::Instant;

pub struct MeteoraDammDecoder;

impl PoolDecoder for MeteoraDammDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDamm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        let (pool_account, state) = accounts.values().find_map(|a| {
            if a.owner.to_string() != METEORA_DAMM_PROGRAM_ID {
                return None;
            }
            DammPoolState::decode(a.pubkey, &a.data).map(|state| (a, state))
        })?;

        let (token_mint, sol_vault, token_vault, sol_is_a) =
            if is_sol_mint(&state.mint_b.to_string()) {
                (state.mint_a, state.vault_b, state.vault_a, false)
            } else if is_sol_mint(&state.mint_a.to_string()) {
                (state.mint_b, state.vault_a, state.vault_b, true)
            } else {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "DAMM pool {} has no SOL token: {} / {}",
                        state.pool, state.mint_a, state.mint_b
                    ),
                );
                return None;
            };
        let token_mint = token_mint.to_string();

        if token_mint != base_mint && token_mint != quote_mint {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "DAMM pool token {} doesn't match requested base {} or quote {}",
                    token_mint, base_mint, quote_mint
                ),
            );
            return None;
        }

        if state.liquidity == 0 {
            logger::debug(
                LogTag::PoolDecoder,
                &format!("DAMM pool {} has no active liquidity", state.pool),
            );
            return None;
        }
        if state.sqrt_price == 0
            || state.sqrt_price < state.sqrt_min_price
            || state.sqrt_price > state.sqrt_max_price
        {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "DAMM pool {}: sqrt_price {} outside its range [{}, {}]",
                    state.pool, state.sqrt_price, state.sqrt_min_price, state.sqrt_max_price
                ),
            );
            return None;
        }

        let sol_balance = vault_amount(accounts, &state.pool, &sol_vault)?;
        let token_balance = vault_amount(accounts, &state.pool, &token_vault)?;

        let token_decimals = match get_cached_decimals(crate::chains::ChainId::Solana, &token_mint)
        {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "DAMM: Token decimals not found for {}, skipping price calculation",
                        token_mint
                    ),
                );
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;
        if sol_decimals > 18 || token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Meteora DAMM: Decimals too large: sol={}, token={}",
                    sol_decimals, token_decimals
                ),
            );
            return None;
        }

        let (decimals_a, decimals_b) = if sol_is_a {
            (sol_decimals, token_decimals)
        } else {
            (token_decimals, sol_decimals)
        };
        let sqrt_f = state.sqrt_price as f64;
        // (sqrt / 2^64)^2 == sqrt^2 / 2^128: raw token B per raw token A.
        let ratio = (sqrt_f * sqrt_f) / (2_f64).powi(128);
        let price_b_per_a = ratio * (10_f64).powi((decimals_a as i32) - (decimals_b as i32));
        let price_sol = if sol_is_a {
            1.0 / price_b_per_a
        } else {
            price_b_per_a
        };

        if price_sol <= 0.0 || !price_sol.is_finite() {
            logger::error(
                LogTag::PoolDecoder,
                &format!("DAMM: Invalid price calculation result: {price_sol}"),
            );
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "DAMM pool {}: sqrt_price={} liquidity={} sol_is_a={} price={:.18e} SOL per token",
                state.pool, state.sqrt_price, state.liquidity, sol_is_a, price_sol
            ),
        );

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // We don't calculate USD prices, only SOL
            price_native: price_sol,
            native_reserves: (sol_balance as f64) / (10_f64).powi(sol_decimals as i32),
            token_reserves: (token_balance as f64) / (10_f64).powi(token_decimals as i32),
            confidence: 0.9,
            source_pool: Some("METEORA_DAMM_Q64_CANON".to_owned()),
            pool_address: pool_account.pubkey.to_string(),
            slot: 0, // Will be updated by the system
            timestamp: Instant::now(),
        })
    }
}

impl MeteoraDammDecoder {
    /// Extract reserve account addresses from DAMM pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [token_a_vault, token_b_vault]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        let state = DammPoolState::decode(Pubkey::default(), pool_data)?;
        Some(vec![state.vault_a.to_string(), state.vault_b.to_string()])
    }
}

/// The token amount of the vault at `address`, refused when the bundle lacks it.
fn vault_amount(
    accounts: &HashMap<String, AccountData>,
    pool: &Pubkey,
    address: &Pubkey,
) -> Option<u64> {
    let Some(account) = accounts.get(&address.to_string()) else {
        logger::warning(
            LogTag::PoolDecoder,
            &format!("DAMM pool {pool}: vault {address} is not in the account bundle"),
        );
        return None;
    };
    token_account_amount(&account.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::constants::SOL_MINT;
    use std::str::FromStr;

    /// `Pool` account length on mainnet.
    const POOL_LEN: usize = 1112;
    const LIQUIDITY: std::ops::Range<usize> = 360..376;

    struct Pool {
        accounts: HashMap<String, AccountData>,
        pool: Pubkey,
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

    /// A WSOL/token pool (WSOL is token A, the token has 6 decimals) whose
    /// `sqrt_price` is `10 * 2^64`: 100 raw token per raw lamport, so 1e5 tokens per
    /// SOL and 1e-5 SOL per token.
    fn sol_as_a_pool() -> Pool {
        let pool = Pubkey::new_unique();
        let wsol = Pubkey::from_str(SOL_MINT).unwrap();
        let token_mint = Pubkey::new_unique();
        let vault_a = Pubkey::new_unique();
        let vault_b = Pubkey::new_unique();

        let mut data = vec![0u8; POOL_LEN];
        data[168..200].copy_from_slice(wsol.as_ref());
        data[200..232].copy_from_slice(token_mint.as_ref());
        data[232..264].copy_from_slice(vault_a.as_ref());
        data[264..296].copy_from_slice(vault_b.as_ref());
        data[LIQUIDITY].copy_from_slice(&(1u128 << 80).to_le_bytes());
        data[424..440].copy_from_slice(&1u128.to_le_bytes());
        data[440..456].copy_from_slice(&u128::MAX.to_le_bytes());
        data[456..472].copy_from_slice(&(10u128 << 64).to_le_bytes());

        crate::tokens::cache_decimals(crate::chains::ChainId::Solana, &token_mint.to_string(), 6);
        let program = Pubkey::from_str(METEORA_DAMM_PROGRAM_ID).unwrap();
        let token_program = crate::chains::solana::spl_token::id();
        let accounts = HashMap::from([
            (pool.to_string(), account(pool, program, data)),
            (
                vault_a.to_string(),
                account(vault_a, token_program, token_account(&wsol, 2_000_000_000)),
            ),
            (
                vault_b.to_string(),
                account(
                    vault_b,
                    token_program,
                    token_account(&token_mint, 200_000_000_000),
                ),
            ),
        ]);
        Pool {
            accounts,
            pool,
            token_mint,
        }
    }

    fn decode(pool: &Pool) -> Option<PriceResult> {
        MeteoraDammDecoder::decode_and_calculate(
            &pool.accounts,
            &pool.token_mint.to_string(),
            SOL_MINT,
        )
    }

    fn pool_data(pool: &mut Pool) -> &mut Vec<u8> {
        &mut pool.accounts.get_mut(&pool.pool.to_string()).unwrap().data
    }

    #[test]
    fn short_data_is_rejected_without_panicking() {
        assert!(MeteoraDammDecoder::extract_reserve_accounts(&[]).is_none());
        assert!(MeteoraDammDecoder::extract_reserve_accounts(&vec![0u8; 484]).is_none());
    }

    #[test]
    fn a_sol_as_a_pool_prices_the_token_in_sol() {
        let pool = sol_as_a_pool();
        let price = decode(&pool).expect("a live pool prices");
        assert_eq!(price.mint, pool.token_mint.to_string());
        assert!(
            (price.price_native / 1e-5 - 1.0).abs() < 1e-12,
            "price {} SOL per token",
            price.price_native
        );
        assert_eq!(price.native_reserves, 2.0);
        assert_eq!(price.token_reserves, 200_000.0);
    }

    #[test]
    fn a_pool_without_active_liquidity_yields_no_price() {
        let mut pool = sol_as_a_pool();
        pool_data(&mut pool)[LIQUIDITY].fill(0);
        assert!(decode(&pool).is_none());
    }

    #[test]
    fn a_sqrt_price_outside_the_pool_range_yields_no_price() {
        let mut pool = sol_as_a_pool();
        pool_data(&mut pool)[440..456].copy_from_slice(&(1u128 << 64).to_le_bytes());
        assert!(decode(&pool).is_none());
    }

    #[test]
    fn reserve_accounts_are_the_layout_vaults() {
        let pool = sol_as_a_pool();
        let data = &pool.accounts[&pool.pool.to_string()].data;
        let state = DammPoolState::decode(pool.pool, data).unwrap();
        assert_eq!(
            MeteoraDammDecoder::extract_reserve_accounts(data),
            Some(vec![state.vault_a.to_string(), state.vault_b.to_string()])
        );
    }
}
