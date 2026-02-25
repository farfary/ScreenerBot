//! ATA closing operations for reclaiming rent from empty token accounts.

use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::utils::format_mint_for_log;
use crate::{Error, Result};
use std::time::Duration;

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(wallet_address: &str, mint: &str) -> Result<String> {
    logger::info(
        LogTag::Wallet,
        &format!(
            "Attempting to close single ATA for mint {}",
            format_mint_for_log(&mint)
        ),
    );

    // Get all token accounts to find the specific one
    let token_accounts = super::balance::get_all_token_accounts(wallet_address).await?;

    // Find the account for this mint
    let target_account = token_accounts
        .iter()
        .find(|account| account.mint == mint && account.balance == 0);

    match target_account {
        Some(account) => {
            logger::info(
                LogTag::Wallet,
                &format!("Found empty ATA {} for mint {}", account.account, mint),
            );

            // Close the ATA
            match super::helpers::close_ata(
                wallet_address,
                &account.account,
                mint,
                account.is_token_2022,
            )
            .await
            {
                Ok(signature) => {
                    logger::info(
                        LogTag::Wallet,
                        &format!(
                            "Closed ATA {} for mint {}. TX: {}",
                            account.account, mint, signature
                        ),
                    );
                    Ok(signature)
                }
                Err(e) => {
                    logger::error(
                        LogTag::Wallet,
                        &format!(
                            "Failed to close ATA {} for mint {}: {}",
                            account.account, mint, e
                        ),
                    );
                    Err(e)
                }
            }
        }
        None => {
            let error_msg = format!("No empty ATA found for mint {mint}");
            logger::warning(LogTag::Wallet, &error_msg);
            Err(Error::invalid_amount(
                error_msg.clone(),
                "No empty ATA found".to_owned(),
            ))
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(wallet_address: &str) -> Result<(u32, Vec<String>)> {
    logger::info(
        LogTag::Wallet,
        "Checking for empty token accounts to close...",
    );

    // Get all token accounts for the wallet
    let all_accounts = super::balance::get_all_token_accounts(wallet_address).await?;

    if all_accounts.is_empty() {
        logger::info(LogTag::Wallet, "No token accounts found in wallet");
        return Ok((0, vec![]));
    }

    // Filter for empty accounts (balance = 0)
    let empty_accounts: Vec<&crate::rpc::TokenAccountInfo> = all_accounts
        .iter()
        .filter(|account| account.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        logger::info(LogTag::Wallet, "No empty token accounts found to close");
        return Ok((0, vec![]));
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Found {} empty token accounts to close",
            empty_accounts.len()
        ),
    );

    let mut signatures = Vec::new();
    let mut closed_count = 0u32;

    // Close each empty account
    for account_info in empty_accounts {
        logger::info(
            LogTag::Wallet,
            &format!(
                "Closing empty {} account {} for mint {}",
                if account_info.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                account_info.account,
                account_info.mint
            ),
        );

        match super::helpers::close_ata(
            wallet_address,
            &account_info.account,
            &account_info.mint,
            account_info.is_token_2022,
        )
        .await
        {
            Ok(signature) => {
                logger::info(
                    LogTag::Wallet,
                    &format!(
                        "Closed empty ATA {}. TX: {}",
                        account_info.account, signature
                    ),
                );
                signatures.push(signature);
                closed_count += 1;

                // Small delay between closures to avoid overwhelming the network
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                logger::error(
                    LogTag::Wallet,
                    &format!("Failed to close ATA {}: {}", account_info.account, e),
                );
                // Continue with other accounts even if one fails
            }
        }
    }

    let rent_reclaimed = (closed_count as f64) * 0.00203928; // Approximate ATA rent in SOL
    logger::info(
        LogTag::Wallet,
        &format!(
            "ATA cleanup complete! Closed {} accounts, reclaimed ~{:.6} SOL in rent",
            closed_count, rent_reclaimed
        ),
    );

    Ok((closed_count, signatures))
}

/// Closes the Associated Token Account (ATA) for a given token mint after selling all tokens
/// This reclaims the rent SOL (~0.002 SOL) from empty token accounts
/// Supports both regular SPL tokens and Token-2022 tokens
///
/// # Parameters
/// * `mint` - The token mint address
/// * `wallet_address` - The wallet address
/// * `recently_sold` - Optional flag indicating if tokens were recently sold (enables longer wait times)
pub async fn close_token_account(mint: &str, wallet_address: &str) -> Result<String> {
    close_token_account_with_context(mint, wallet_address, false).await
}

/// Enhanced version of close_token_account with additional context
pub async fn close_token_account_with_context(
    mint: &str,
    wallet_address: &str,
    recently_sold: bool,
) -> Result<String> {
    logger::info(
        LogTag::Wallet,
        &format!(
            "Attempting to close token account for mint: {}",
            format_mint_for_log(mint)
        ),
    );

    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_CLOSE_START: wallet={}, mint={}, recently_sold={}",
            wallet_address, mint, recently_sold
        ),
    );

    // First verify the token balance is actually zero with retry logic for blockchain propagation
    let mut balance_check_attempts = 0;
    let max_checks = if recently_sold { 8 } else { 5 }; // More attempts if recently sold
    let delay_ms = if recently_sold { 3000 } else { 2000 }; // Longer delay if recently sold

    if recently_sold {
        logger::debug(
            LogTag::Wallet,
            &format!(
                "ATA_RECENTLY_SOLD: using extended retry logic ({}x{}ms) for recently sold token",
                max_checks, delay_ms
            ),
        );
    }

    loop {
        balance_check_attempts += 1;

        logger::debug(
            LogTag::Wallet,
            &format!(
                "ATA_BALANCE_CHECK: attempt {}/{} for mint {}",
                balance_check_attempts, max_checks, mint
            ),
        );

        match super::balance::get_token_balance(wallet_address, mint).await {
            Ok(balance) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "ATA_BALANCE_RESULT: {} tokens remaining for mint {}",
                        balance, mint
                    ),
                );

                if balance > 0 {
                    if balance_check_attempts < max_checks {
                        logger::debug(
                            LogTag::Wallet,
                            &format!(
 "ATA_BALANCE_RETRY: {} tokens still present, waiting {}ms before retry (attempt {}/{})",
                balance,
                delay_ms,
                balance_check_attempts,
                max_checks
              ),
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        logger::debug(
                            LogTag::Wallet,
                            &format!(
                                "ATA_BALANCE_FAILED: {} tokens still present after {} attempts",
                                balance, max_checks
                            ),
                        );
                        return Err(Error::invalid_amount(
                            balance.to_string(),
                            format!(
                   "Cannot close token account - still has {} tokens after {} balance checks",
                   balance,
                   max_checks
                 ),
                        ));
                    }
                }

                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "ATA_BALANCE_ZERO: confirmed zero balance for mint {} after {} attempts",
                        mint, balance_check_attempts
                    ),
                );

                logger::info(
                    LogTag::Wallet,
                    &format!(
                        "Verified zero balance for {}, proceeding to close ATA",
                        mint
                    ),
                );
                break;
            }
            Err(e) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "ATA_BALANCE_ERROR: attempt {}/{} failed: {}",
                        balance_check_attempts, max_checks, e
                    ),
                );

                if balance_check_attempts < max_checks {
                    logger::debug(
                        LogTag::Wallet,
                        &format!(
                            "ATA_BALANCE_RETRY: waiting {}ms before retry due to error",
                            delay_ms
                        ),
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                } else {
                    logger::warning(
                        LogTag::Wallet,
                        &format!(
              "Could not verify token balance before closing ATA after {} attempts: {}",
              max_checks,
              e
            ),
                    );
                    // Continue anyway - the close instruction will fail if tokens remain
                    break;
                }
            }
        }
    }

    // Get the associated token account address
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_DISCOVER: finding associated token account for mint {}",
            mint
        ),
    );

    let token_account =
        match super::helpers::get_associated_token_account(wallet_address, mint).await {
            Ok(account) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!("ATA_FOUND: token_account={account} for mint={mint}"),
                );
                account
            }
            Err(e) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "ATA_NOT_FOUND: error finding token account for mint {}: {}",
                        mint, e
                    ),
                );
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Could not find associated token account for {}: {}",
                        mint, e
                    ),
                );
                return Err(e);
            }
        };

    logger::info(
        LogTag::Wallet,
        &format!("Found token account to close: {token_account}"),
    );

    // Determine if this is a Token-2022 account by checking the token ACCOUNT's program (not the mint)
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_PROGRAM_CHECK: determining token program for account {}",
            token_account
        ),
    );

    let rpc_client = get_rpc_client();
    let is_token_2022 = rpc_client
        .is_token_account_token_2022(&token_account)
        .await
        .unwrap_or_default();

    if is_token_2022 {
        logger::debug(
            LogTag::Wallet,
            &format!(
                "ATA_TOKEN2022: using Token Extensions program for account {}",
                token_account
            ),
        );
        logger::info(
            LogTag::Wallet,
            "Detected Token-2022, using Token Extensions program",
        );
    } else {
        logger::debug(
            LogTag::Wallet,
            &format!(
                "ATA_SPL_TOKEN: using standard SPL Token program for account {}",
                token_account
            ),
        );
        logger::info(LogTag::Wallet, "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_CLOSE_EXECUTE: initiating close instruction for account {}",
            token_account
        ),
    );
    match super::helpers::close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "ATA_CLOSE_SUCCESS: transaction={}, account={}, mint={}",
                    signature, token_account, mint
                ),
            );
            logger::info(
                LogTag::Wallet,
                &format!(
                    "Successfully closed token account for {}. TX: {}",
                    mint, signature
                ),
            );
            Ok(signature)
        }
        Err(e) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "ATA_CLOSE_FAILED: account={}, mint={}, error={}",
                    token_account, mint, e
                ),
            );
            logger::error(
                LogTag::Wallet,
                &format!("Failed to close token account for {mint}: {e}"),
            );
            Err(e)
        }
    }
}
