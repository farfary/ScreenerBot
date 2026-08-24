//! Balance query functions for SOL and token accounts.

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
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

/// Get TOTAL token balance across ALL token accounts for a mint (USE FOR EXITS TO SELL ALL)
pub async fn get_total_token_balance(wallet_address: &str, mint: &str) -> Result<u64> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "TOTAL_TOKEN_BALANCE_START: wallet={}, mint={}",
            wallet_address, mint
        ),
    );

    // Get all token accounts for this wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    // Filter accounts for the specific mint and sum balances
    let mut total_balance = 0u64;
    let mut account_count = 0usize;

    for account in all_accounts {
        if account.mint == mint {
            total_balance = total_balance.saturating_add(account.balance);
            account_count += 1;

            logger::debug(
                LogTag::Wallet,
                &format!(
                    "Found account {} with {} tokens ({})",
                    &account.account,
                    account.balance,
                    if account.is_token_2022 {
                        "Token-2022"
                    } else {
                        "SPL Token"
                    }
                ),
            );
        }
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Total balance for mint {}: {} tokens across {} accounts",
            mint, total_balance, account_count
        ),
    );

    if account_count > 1 {
        logger::info(
            LogTag::Wallet,
            &format!(
                "MULTIPLE ACCOUNTS DETECTED for mint {}: {} accounts with total {} tokens",
                mint, account_count, total_balance
            ),
        );
    }

    Ok(total_balance)
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str,
) -> Result<Vec<crate::chains::solana::rpc::TokenAccountInfo>> {
    let rpc_client = get_rpc_client();
    rpc_client
        .get_all_token_accounts_str(wallet_address)
        .await
        .map_err(Error::from)
}
