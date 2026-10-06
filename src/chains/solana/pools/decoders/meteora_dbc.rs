// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora Dynamic Bonding Curve (DBC) decoder
//!
//! Program ID: METEORA_DBC_PROGRAM_ID (dbcij3LW...)
//!
//! Decodes the pool with the shared `VirtualPool` layout
//! (`crate::chains::solana::pools::layouts::meteora_dbc`). The pool is the account the
//! DBC program owns that carries the `VirtualPool` discriminator; its two vaults are
//! looked up by the addresses it names. The token is `base_mint`; the quote vault
//! must hold WSOL. The price comes from `sqrt_price` (Q64.64, quote per base in raw
//! units) and the reserves are the curve's own `base_reserve`/`quote_reserve`, which
//! exclude the uncollected fees the vaults also hold. A migrated curve yields no
//! price: its liquidity has moved to the post-migration pool.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::METEORA_DBC_PROGRAM_ID;
use crate::chains::solana::constants::{SOL_DECIMALS, SOL_MINT};
use crate::chains::solana::layout::pubkey_at;
use crate::chains::solana::pools::layouts::meteora_dbc::VirtualPoolState;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;

pub struct MeteoraDbcDecoder;

impl PoolDecoder for MeteoraDbcDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDbc]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        _base_mint: &str,
        _quote_mint: &str,
    ) -> Option<PriceResult> {
        let (pool_acc, state) = accounts.values().find_map(|a| {
            if a.owner.to_string() != METEORA_DBC_PROGRAM_ID {
                return None;
            }
            VirtualPoolState::decode(a.pubkey, &a.data).map(|state| (a, state))
        })?;

        if state.is_migrated {
            logger::debug(
                LogTag::PoolDecoder,
                &format!("Meteora DBC pool {} has migrated", state.pool),
            );
            return None;
        }

        let sol_mint = SOL_MINT.parse::<Pubkey>().ok()?;
        let base_vault = vault(accounts, &state.pool, &state.base_vault, &state.base_mint)?;
        let quote_vault = vault(accounts, &state.pool, &state.quote_vault, &sol_mint)?;
        let token_mint = state.base_mint.to_string();

        let token_decimals = get_cached_decimals(crate::chains::ChainId::Solana, &token_mint)?;
        let sol_decimals = SOL_DECIMALS;
        if sol_decimals > 18 || token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Meteora DBC: Decimals too large: sol={}, token={}",
                    sol_decimals, token_decimals
                ),
            );
            return None;
        }

        // Raw ratio (quote/base in raw units) = (sqrt_price / 2^64)^2
        // Convert to human units: divide by 10^(quote_decimals - base_decimals)
        let sqrt_price_f64 = (state.sqrt_price as f64) / ((1u128 << 64) as f64);
        let raw_ratio = sqrt_price_f64 * sqrt_price_f64;
        let decimals_scale = (10f64).powi((sol_decimals as i32) - (token_decimals as i32));
        let price_per_token = raw_ratio / decimals_scale;

        logger::debug(
            LogTag::Pool,
            &format!(
                "sqrt_price_f64: {}, raw_ratio: {}, decimals_scale: {}, price_per_token: {}",
                sqrt_price_f64, raw_ratio, decimals_scale, price_per_token
            ),
        );

        if !price_per_token.is_finite() || price_per_token <= 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Meteora DBC: Invalid price calculated: {price_per_token}"),
            );
            return None;
        }

        let sol = (state.quote_reserve as f64) / (10f64).powi(sol_decimals as i32);
        let tok = (state.base_reserve as f64) / (10f64).powi(token_decimals as i32);

        let mut pr = PriceResult::new(
            token_mint,
            0.0,
            price_per_token,
            sol,
            tok,
            state.pool.to_string(),
        );
        pr.source_pool = Some(ProgramKind::MeteoraDbc.display_name().to_string());
        pr.slot = quote_vault.slot.min(base_vault.slot).min(pool_acc.slot);
        Some(pr)
    }
}

/// The vault account at `address`, refused when the bundle lacks it or when it
/// holds a different mint than `mint`.
fn vault<'a>(
    accounts: &'a HashMap<String, AccountData>,
    pool: &Pubkey,
    address: &Pubkey,
    mint: &Pubkey,
) -> Option<&'a AccountData> {
    let Some(account) = accounts.get(&address.to_string()) else {
        logger::warning(
            LogTag::PoolDecoder,
            &format!("Meteora DBC pool {pool}: vault {address} is not in the account bundle"),
        );
        return None;
    };
    let held = pubkey_at(&account.data, 0)?;
    if held != *mint {
        logger::warning(
            LogTag::PoolDecoder,
            &format!("Meteora DBC pool {pool}: vault {address} holds mint {held}, not {mint}"),
        );
        return None;
    }
    Some(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::pools::layouts::meteora_dbc::VIRTUAL_POOL_DISCRIMINATOR;
    use std::str::FromStr;
    use std::time::Instant;

    struct Pool {
        accounts: HashMap<String, AccountData>,
        pool: Pubkey,
        quote_vault: Pubkey,
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

    /// A live token/WSOL curve with both vaults in the bundle.
    fn pool() -> Pool {
        let pool = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let base_vault = Pubkey::new_unique();
        let quote_vault = Pubkey::new_unique();
        let wsol = Pubkey::from_str(SOL_MINT).unwrap();

        let mut data = vec![0u8; 424];
        data[0..8].copy_from_slice(&VIRTUAL_POOL_DISCRIMINATOR);
        data[136..168].copy_from_slice(base_mint.as_ref());
        data[168..200].copy_from_slice(base_vault.as_ref());
        data[200..232].copy_from_slice(quote_vault.as_ref());
        data[280..296].copy_from_slice(&(1u128 << 64).to_le_bytes());

        let program = Pubkey::from_str(METEORA_DBC_PROGRAM_ID).unwrap();
        let token_program = crate::chains::solana::spl_token::id();
        let accounts = HashMap::from([
            (pool.to_string(), account(pool, program, data)),
            (
                base_vault.to_string(),
                account(base_vault, token_program, token_account(&base_mint, 1_000)),
            ),
            (
                quote_vault.to_string(),
                account(quote_vault, token_program, token_account(&wsol, 2_000)),
            ),
        ]);
        Pool {
            accounts,
            pool,
            quote_vault,
        }
    }

    fn decode(pool: &Pool) -> Option<PriceResult> {
        MeteoraDbcDecoder::decode_and_calculate(&pool.accounts, "", "")
    }

    #[test]
    fn a_migrated_curve_yields_no_price() {
        let mut p = pool();
        p.accounts.get_mut(&p.pool.to_string()).unwrap().data[305] = 1;
        assert!(decode(&p).is_none());
    }

    #[test]
    fn a_quote_vault_that_does_not_hold_wsol_yields_no_price() {
        let mut p = pool();
        let other = Pubkey::new_unique();
        p.accounts.get_mut(&p.quote_vault.to_string()).unwrap().data = token_account(&other, 2_000);
        assert!(decode(&p).is_none());
    }

    #[test]
    fn a_vault_missing_from_the_bundle_yields_no_price() {
        let mut p = pool();
        p.accounts.remove(&p.quote_vault.to_string());
        assert!(decode(&p).is_none());
    }

    #[test]
    fn an_account_of_the_program_without_the_pool_discriminator_is_not_the_pool() {
        let mut p = pool();
        p.accounts.get_mut(&p.pool.to_string()).unwrap().data[0] ^= 1;
        assert!(decode(&p).is_none());
    }
}
