// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Balance query functions for SOL and token accounts.

use crate::chains::settlement::Holding;
use crate::chains::solana::access::{chain_access, Freshness};
use crate::chains::solana::rpc::{TokenAccountInfo, TokenAccountsSnapshot};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::RawAmount;
use crate::errors::DataError;
use crate::logger::{self, LogTag};
use crate::utils::{format_mint_for_log, get_wallet_address};
use crate::{Error, Result};
use std::str::FromStr;

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>)> {
    logger::info(
        LogTag::Wallet,
        "Manual ATA cleanup triggered (normally handled by background service)",
    );
    let wallet_address = get_wallet_address()?;
    super::close::close_all_empty_atas(&wallet_address).await
}

/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64> {
    chain_access().sol_balance(wallet_address).await
}

/// Checks wallet balance for a specific token (SINGLE ACCOUNT ONLY - use get_total_token_balance for exits)
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "TOKEN_BALANCE_START: wallet={}, mint={}",
            wallet_address, mint
        ),
    );

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Fetching token balance: wallet={}, mint={}",
            wallet_address, mint
        ),
    );

    logger::debug(
        LogTag::Wallet,
        "TOKEN_BALANCE_RPC: querying RPC for balance",
    );

    match chain_access().token_balance(wallet_address, mint).await {
        Ok(balance) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "Token balance fetched successfully: {} units for mint {}",
                    balance, mint
                ),
            );
            Ok(balance)
        }
        Err(e) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "TOKEN_BALANCE_ERROR: {} for mint {}",
                    e,
                    format_mint_for_log(&mint)
                ),
            );
            logger::error(
                LogTag::Wallet,
                &format!(
                    "Failed to fetch token balance for mint {}: {}",
                    format_mint_for_log(&mint),
                    e
                ),
            );
            Err(e)
        }
    }
}

/// The wallet's holding of `mint`, summed over its accounts of both token programs, and
/// frozen when any of those accounts is frozen.
pub async fn token_holding(wallet_address: &str, mint: &str) -> Result<Holding> {
    Ok(
        token_holding_with_slot(wallet_address, mint, Freshness::Any)
            .await?
            .0,
    )
}

/// The wallet's holding of `mint` as [`token_holding`] reads it, read at `freshness`, with
/// the context slot the accounts were read at; `None` when a response carried no slot.
pub async fn token_holding_with_slot(
    wallet_address: &str,
    mint: &str,
    freshness: Freshness,
) -> Result<(Holding, Option<u64>)> {
    let snapshot = token_accounts_snapshot(wallet_address, freshness).await?;
    let holding = holding_of(&snapshot.accounts, mint);
    logger::debug(
        LogTag::Wallet,
        &format!(
            "Holding of mint {}: {} raw units (frozen: {})",
            format_mint_for_log(mint),
            holding.amount,
            holding.frozen
        ),
    );
    Ok((holding, snapshot.slot))
}

/// The wallet's total balance of `mint` across all its token accounts, for selling all of
/// it; an error when the total does not fit a single transfer amount.
pub async fn get_total_token_balance(wallet_address: &str, mint: &str) -> Result<u64> {
    let total = token_holding(wallet_address, mint).await?.amount;
    u64::try_from(total).map_err(|_| {
        Error::Data(DataError::InvalidAmount {
            amount: total.to_string(),
            reason: "token balance exceeds a u64 transfer amount".to_owned(),
        })
    })
}

/// Sums the balances of `mint`'s accounts in full width and flags the holding frozen when
/// any of them is frozen.
fn holding_of(accounts: &[TokenAccountInfo], mint: &str) -> Holding {
    let held = accounts.iter().filter(|account| account.mint == mint);
    Holding {
        amount: RawAmount::new(
            held.clone()
                .map(|account| u128::from(account.balance))
                .sum(),
        ),
        frozen: held.clone().any(|account| account.is_frozen),
    }
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(wallet_address: &str) -> Result<Vec<TokenAccountInfo>> {
    chain_access().all_token_accounts(wallet_address).await
}

/// Gets all token accounts for a wallet with the slot they were read at.
pub async fn get_token_accounts_snapshot(wallet_address: &str) -> Result<TokenAccountsSnapshot> {
    token_accounts_snapshot(wallet_address, Freshness::Any).await
}

/// All token accounts of a wallet with the slot they were read at, read at `freshness`.
async fn token_accounts_snapshot(
    wallet_address: &str,
    freshness: Freshness,
) -> Result<TokenAccountsSnapshot> {
    let owner = Pubkey::from_str(wallet_address).map_err(|e| {
        Error::Data(DataError::ParseError {
            data_type: "pubkey".to_string(),
            error: format!("Invalid owner '{wallet_address}': {e}"),
        })
    })?;
    chain_access().token_accounts(&owner, freshness).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(mint: &str, balance: u64, is_frozen: bool) -> TokenAccountInfo {
        TokenAccountInfo {
            account: format!("{mint}-{balance}-{is_frozen}"),
            mint: mint.to_owned(),
            balance,
            decimals: 6,
            is_token_2022: false,
            is_nft: false,
            is_frozen,
            lamports: 1_488_440,
        }
    }

    #[test]
    fn a_holding_sums_every_account_of_the_mint_and_flags_any_frozen_one() {
        let accounts = [
            account("mint", 700, false),
            account("other", 5_000, true),
            account("mint", 300, true),
        ];
        assert_eq!(
            holding_of(&accounts, "mint"),
            Holding {
                amount: RawAmount::new(1_000),
                frozen: true,
            }
        );
        assert_eq!(
            holding_of(&accounts[..1], "mint"),
            Holding {
                amount: RawAmount::new(700),
                frozen: false,
            }
        );
        assert_eq!(
            holding_of(&accounts, "absent"),
            Holding {
                amount: RawAmount::ZERO,
                frozen: false,
            }
        );
    }

    #[test]
    fn a_holding_above_the_u64_range_keeps_every_unit() {
        let accounts = [
            account("mint", u64::MAX, false),
            account("mint", u64::MAX, false),
            account("mint", 2, false),
        ];
        assert_eq!(
            holding_of(&accounts, "mint").amount,
            RawAmount::new(u128::from(u64::MAX) * 2 + 2)
        );
    }
}
