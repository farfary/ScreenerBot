// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SPL/Token-2022 mint account mechanics: on-chain reads, decimals,
//! token-program detection and mint/freeze authority extraction.
//!
//! Pure adapter over a single mint account fetch. Cache/DB/server-fallback
//! policy for decimals lives in `crate::tokens::decimals`, which reads the
//! chain runtime's `read_token_account`; on Solana that is
//! [`fetch_mint_account`], its on-chain source of truth.

use std::str::FromStr;

use crate::chains::solana::access::{chain_access, MintLookup};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::{Error, Result};

/// Decimals, mint/freeze authority and supply read directly from a mint account.
#[derive(Debug, Clone)]
pub struct MintAccountData {
    pub decimals: u8,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub supply: u64,
}

/// Fetch and decode a mint account, auto-detecting SPL Token vs Token-2022.
pub async fn fetch_mint_account(mint: &str) -> Result<MintAccountData> {
    Pubkey::from_str(mint).map_err(|_| Error::InvalidAddress {
        kind: "mint",
        value: mint.to_owned(),
    })?;

    let lookup = chain_access()
        .mints(&[mint.to_owned()], false)
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_account",
            detail: e.to_string(),
        })?
        .pop()
        .unwrap_or(MintLookup::Absent);
    mint_account_data(mint, lookup)
}

/// The mint data of one lookup: an absent account is not found, an account
/// that is not a mint fails to decode.
fn mint_account_data(mint: &str, lookup: MintLookup) -> Result<MintAccountData> {
    match lookup {
        MintLookup::Mint(facts) => Ok(MintAccountData {
            decimals: facts.decimals,
            mint_authority: facts.mint_authority,
            freeze_authority: facts.freeze_authority,
            supply: facts.supply,
        }),
        MintLookup::Absent => Err(Error::AccountNotFound {
            address: mint.to_owned(),
        }),
        MintLookup::NotAMint { detail } => Err(Error::Decode {
            payload: "mint account",
            detail,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lookup_maps_to_mint_data_not_found_or_a_decode_failure() {
        assert!(matches!(
            mint_account_data("M", MintLookup::Absent),
            Err(Error::AccountNotFound { .. })
        ));
        assert!(matches!(
            mint_account_data(
                "M",
                MintLookup::NotAMint {
                    detail: "account data is empty".to_owned()
                }
            ),
            Err(Error::Decode { .. })
        ));
    }
}
