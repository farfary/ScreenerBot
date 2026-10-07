// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Balance query functions for SOL and token accounts.

use crate::chains::settlement::Holding;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods, TokenAccountInfo};
use crate::chains::RawAmount;
use crate::errors::DataError;
use crate::logger::{self, LogTag};
use crate::utils::{format_mint_for_log, get_wallet_address};
use crate::{Error, Result};

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
    let rpc_client = get_rpc_client();
    rpc_client
        .get_sol_balance(wallet_address)
        .await
        .map_err(Error::from)
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

    let rpc_client = get_rpc_client();

    logger::debug(
        LogTag::Wallet,
        "TOKEN_BALANCE_RPC: querying RPC for balance",
    );

    match rpc_client.get_token_balance(wallet_address, mint).await {
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
            Err(Error::from(e))
        }
    }
}

/// The wallet's holding of `mint`, summed over its accounts of both token programs, and
/// frozen when any of those accounts is frozen.
pub async fn token_holding(wallet_address: &str, mint: &str) -> Result<Holding> {
    let holding = holding_of(&get_all_token_accounts(wallet_address).await?, mint);
    logger::debug(
        LogTag::Wallet,
        &format!(
            "Holding of mint {}: {} raw units (frozen: {})",
            format_mint_for_log(mint),
            holding.amount,
            holding.frozen
        ),
    );
    Ok(holding)
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
    let rpc_client = get_rpc_client();
    rpc_client
        .get_all_token_accounts_str(wallet_address)
        .await
        .map_err(Error::from)
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
