//! Associated Token Account (ATA) operations
//!
//! Balance queries and token account management including ATA closing.
//! Note: Automatic ATA cleanup is handled by the background service (see ata_cleanup.rs).

use crate::constants::TOKEN_2022_PROGRAM_ID;
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::utils::{format_mint_for_log, get_wallet_address};
use crate::{Error, Result};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use std::str::FromStr;

// =============================================================================
// BALANCE QUERIES
// =============================================================================

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>)> {
    logger::info(
        LogTag::Wallet,
        "Manual ATA cleanup triggered (normally handled by background service)",
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
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
            let blockchain_error =
                crate::errors::parse_solana_error(&e.to_string(), None, "get_token_balance");
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "TOKEN_BALANCE_ERROR: {} for mint {}",
                    blockchain_error,
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
) -> Result<Vec<crate::rpc::TokenAccountInfo>> {
    let rpc_client = get_rpc_client();
    rpc_client
        .get_all_token_accounts_str(wallet_address)
        .await
        .map_err(Error::from)
}

// =============================================================================
// ATA CLOSING OPERATIONS
// =============================================================================

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
    let token_accounts = get_all_token_accounts(wallet_address).await?;

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
            match close_ata(
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
    let all_accounts = get_all_token_accounts(wallet_address).await?;

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

        match close_ata(
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
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

        match get_token_balance(wallet_address, mint).await {
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
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
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
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
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

    let token_account = match get_associated_token_account(wallet_address, mint).await {
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
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
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

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Gets the associated token account address for a wallet and mint
async fn get_associated_token_account(wallet_address: &str, mint: &str) -> Result<String> {
    let rpc_client = get_rpc_client();
    rpc_client
        .get_associated_token_account(wallet_address, mint)
        .await
        .map_err(Error::from)
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool,
) -> Result<String> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_SDK_START: wallet={}, account={}, mint={}, program={}",
            &wallet_address[..8],
            &token_account[..8],
            &mint[..8],
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            }
        ),
    );

    logger::info(
        LogTag::Wallet,
        &format!(
            "Closing ATA {} for mint {} using {} program",
            token_account,
            mint,
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            }
        ),
    );

    // Use proper Solana SDK to build and send close instruction
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_BUILD_INSTRUCTION: preparing close instruction for account {}",
            &token_account[..8]
        ),
    );

    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "ATA_SDK_SUCCESS: instruction executed, transaction={}",
                    &signature[..8]
                ),
            );
            logger::info(
                LogTag::Wallet,
                &format!("ATA closed successfully. TX: {signature}"),
            );
            Ok(signature)
        }
        Err(e) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "ATA_SDK_FAILED: instruction failed for account {}: {}",
                    &token_account[..8],
                    e
                ),
            );
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool,
) -> Result<String> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_INSTRUCTION_START: building close instruction for account {}",
            &token_account[..8]
        ),
    );

    // Parse addresses
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_PARSE_ADDRESSES: wallet={}, account={}",
            &wallet_address[..8],
            &token_account[..8]
        ),
    );

    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e| {
        Error::invalid_amount(
            format!("Invalid wallet address: {e}"),
            "Wallet validation failed".to_owned(),
        )
    })?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e| {
        Error::invalid_amount(
            format!("Invalid token account: {e}"),
            "Token account validation failed".to_owned(),
        )
    })?;

    // Load keypair from config
    logger::debug(
        LogTag::Wallet,
        "ATA_KEYPAIR: creating keypair from config",
    );

    let keypair = crate::config::get_wallet_keypair().map_err(|e| {
        Error::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
            error: format!("Failed to load wallet keypair: {e}"),
        })
    })?;

    // Build close account instruction
    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_INSTRUCTION_BUILD: creating {} close instruction",
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            }
        ),
    );

    let close_instruction = if is_token_2022 {
        // For Token-2022, use the Token Extensions program
        build_token_2022_close_instruction(&token_account_pubkey, &owner_pubkey)?
    } else {
        // For regular SPL tokens, use standard close_account instruction
        close_account(
            &spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[],
        )
        .map_err(|e| {
            Error::Blockchain(crate::errors::BlockchainError::InvalidInstruction {
                signature: "unknown".to_owned(),
                instruction_index: 0,
                reason: format!("Failed to build close instruction: {e}"),
            })
        })?
    };

    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_INSTRUCTION_BUILT: close instruction created for {} program",
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            }
        ),
    );

    logger::info(
        LogTag::Wallet,
        &format!(
            "Built close instruction for {} account",
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            }
        ),
    );

    // Get recent blockhash via RPC
    logger::debug(
        LogTag::Wallet,
        "ATA_BLOCKHASH: fetching recent blockhash via RPC",
    );

    let rpc_client = get_rpc_client();
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(Error::from)?;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA_BLOCKHASH_OK: blockhash={}",
            &recent_blockhash.to_string()[..8]
        ),
    );

    // Build transaction
    logger::debug(
        LogTag::Wallet,
        "ATA_TRANSACTION_BUILD: creating signed transaction",
    );

    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash,
    );

    logger::debug(
        LogTag::Wallet,
        "ATA_TRANSACTION_READY: transaction built and signed",
    );

    logger::info(LogTag::Wallet, "Built and signed close transaction");

    // Send transaction via RPC with confirmation
    logger::debug(
        LogTag::Wallet,
        "ATA_TRANSACTION_SEND: submitting transaction to network with confirmation",
    );

    let result = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map(|sig| sig.to_string())
        .map_err(Error::from);

    match &result {
        Ok(signature) => {
            logger::debug(
                LogTag::Wallet,
                &format!(
                    "ATA_TRANSACTION_CONFIRMED: transaction confirmed, signature={}",
                    &signature[..8]
                ),
            );
        }
        Err(e) => {
            let blockchain_error =
                crate::errors::parse_solana_error(&e.to_string(), None, "create_ata_transaction");
            logger::debug(
                LogTag::Wallet,
                &format!("ATA_TRANSACTION_FAILED: {blockchain_error}"),
            );
        }
    }

    result
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction> {
    // Token-2022 uses the same close account instruction format as SPL Token
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID).map_err(|e| {
        Error::Blockchain(crate::errors::BlockchainError::InvalidAccountData {
            signature: "unknown".to_owned(),
            account: TOKEN_2022_PROGRAM_ID.to_string(),
            expected_owner: "Program ID".to_owned(),
            actual_owner: None,
        })
    })?;

    // Manually build the close account instruction for Token-2022
    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8]; // CloseAccount instruction ID

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false),         // Destination for lamports
        AccountMeta::new_readonly(*owner, true), // Authority (signer)
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}
