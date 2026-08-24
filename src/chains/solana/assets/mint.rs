//! SPL/Token-2022 mint account mechanics: on-chain reads, decimals,
//! token-program detection and mint/freeze authority extraction.
//!
//! Pure adapter over a single mint account fetch. Cache/DB/server-fallback
//! policy for decimals lives in `crate::tokens::decimals`, which calls
//! [`fetch_mint_account`] as its on-chain source of truth.

use std::str::FromStr;

use crate::chains::solana::constants::TOKEN_2022_PROGRAM_ID;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_program::program_option::COption;
use crate::chains::solana::solana_program::program_pack::Pack;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::spl_token::state::Mint as SplMint;
use crate::chains::solana::spl_token_2022::state::Mint as Mint2022;
use crate::chains::solana::{Error, Result};
use crate::rpc::errors::RpcError;

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
    let mint_pubkey = Pubkey::from_str(mint).map_err(|_| Error::InvalidAddress {
        kind: "mint",
        value: mint.to_owned(),
    })?;
    let rpc_client = get_rpc_client();

    let account_opt = rpc_client.get_account(&mint_pubkey).await.map_err(|e| {
        if matches!(&e, crate::Error::Rpc(RpcError::AccountNotFound { .. })) {
            Error::AccountNotFound {
                address: mint.to_owned(),
            }
        } else {
            Error::Rpc {
                operation: "get_account",
                detail: e.to_string(),
            }
        }
    })?;

    let account = account_opt.ok_or_else(|| Error::AccountNotFound {
        address: mint.to_owned(),
    })?;

    if account.data.is_empty() {
        return Err(Error::Decode {
            payload: "mint account",
            detail: "account data is empty".to_owned(),
        });
    }

    if account.owner == crate::chains::solana::spl_token::id() {
        let mint_data = SplMint::unpack(&account.data).map_err(|e| Error::Decode {
            payload: "spl token mint",
            detail: e.to_string(),
        })?;
        return Ok(from_spl_mint(&mint_data));
    }

    if account.owner == crate::chains::solana::spl_token_2022::id() {
        if let Ok(mint_data) = Mint2022::unpack(&account.data) {
            return Ok(from_2022_mint(&mint_data));
        }

        // Some Token-2022 mints carry extensions that require the
        // extensions-aware parser rather than the plain base unpack above.
        let state = crate::chains::solana::spl_token_2022::extension::StateWithExtensionsOwned::<
            Mint2022,
        >::unpack(account.data.clone())
        .map_err(|e| Error::Decode {
            payload: "token-2022 mint with extensions",
            detail: e.to_string(),
        })?;
        return Ok(from_2022_mint(&state.base));
    }

    Err(Error::Decode {
        payload: "mint account owner",
        detail: format!("owner {} is not a supported token program", account.owner),
    })
}

fn from_spl_mint(mint_data: &SplMint) -> MintAccountData {
    MintAccountData {
        decimals: mint_data.decimals,
        mint_authority: coption_to_string(mint_data.mint_authority),
        freeze_authority: coption_to_string(mint_data.freeze_authority),
        supply: mint_data.supply,
    }
}

fn from_2022_mint(mint_data: &Mint2022) -> MintAccountData {
    MintAccountData {
        decimals: mint_data.decimals,
        mint_authority: coption_to_string(mint_data.mint_authority),
        freeze_authority: coption_to_string(mint_data.freeze_authority),
        supply: mint_data.supply,
    }
}

fn coption_to_string(value: COption<Pubkey>) -> Option<String> {
    match value {
        COption::Some(pk) => Some(pk.to_string()),
        COption::None => None,
    }
}

/// Is this mint owned by the Token-2022 program? A single account fetch, no
/// decoding — callers that already need the full mint should prefer
/// [`fetch_mint_account`] instead of fetching the account twice.
pub async fn is_token_2022_mint(mint: &str) -> Result<bool> {
    let mint_pubkey = Pubkey::from_str(mint).map_err(|_| Error::InvalidAddress {
        kind: "mint",
        value: mint.to_owned(),
    })?;
    let rpc_client = get_rpc_client();

    let account = rpc_client
        .get_account(&mint_pubkey)
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_account",
            detail: e.to_string(),
        })?
        .ok_or_else(|| Error::AccountNotFound {
            address: mint.to_owned(),
        })?;

    Ok(account.owner.to_string() == TOKEN_2022_PROGRAM_ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spl_mint_maps_decimals_supply_and_authorities() {
        let authority = Pubkey::new_unique();
        let mint_data = SplMint {
            mint_authority: COption::Some(authority),
            supply: 1_000_000_000,
            decimals: 6,
            is_initialized: true,
            freeze_authority: COption::None,
        };

        let info = from_spl_mint(&mint_data);

        assert_eq!(info.decimals, 6);
        assert_eq!(info.supply, 1_000_000_000);
        assert_eq!(info.mint_authority, Some(authority.to_string()));
        assert_eq!(info.freeze_authority, None);
    }

    #[test]
    fn token_2022_mint_maps_decimals_supply_and_authorities() {
        let mint_authority = Pubkey::new_unique();
        let freeze_authority = Pubkey::new_unique();
        let mint_data = Mint2022 {
            mint_authority: COption::Some(mint_authority),
            supply: 42,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::Some(freeze_authority),
        };

        let info = from_2022_mint(&mint_data);

        assert_eq!(info.decimals, 9);
        assert_eq!(info.supply, 42);
        assert_eq!(info.mint_authority, Some(mint_authority.to_string()));
        assert_eq!(info.freeze_authority, Some(freeze_authority.to_string()));
    }

    #[test]
    fn coption_to_string_round_trips_some_and_none() {
        let pk = Pubkey::new_unique();
        assert_eq!(coption_to_string(COption::Some(pk)), Some(pk.to_string()));
        assert_eq!(coption_to_string(COption::None), None);
    }
}
