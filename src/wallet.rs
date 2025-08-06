use crate::global::read_configs;
use crate::logger::{ log, LogTag };
use crate::rpc::SwapError;

// Re-export for backward compatibility
pub use crate::swaps::interface::SwapResult;
pub use crate::swaps::get_wallet_address;

use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    pubkey::Pubkey,
    instruction::{ Instruction, AccountMeta },
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use bs58;
use std::str::FromStr;

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>), SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        "⚠️ Manual ATA cleanup triggered (normally handled by background service)"
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
}



/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_sol_balance(wallet_address).await
}

/// Checks wallet balance for a specific token
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_token_balance(wallet_address, mint).await
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str
) -> Result<Vec<crate::rpc::TokenAccountInfo>, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_all_token_accounts(wallet_address).await
}

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(wallet_address: &str, mint: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close single ATA for mint {}", &mint[..8]));

    // Get all token accounts to find the specific one
    let token_accounts = get_all_token_accounts(wallet_address).await?;

    // Find the account for this mint
    let target_account = token_accounts
        .iter()
        .find(|account| account.mint == mint && account.balance == 0);

    match target_account {
        Some(account) => {
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Found empty ATA {} for mint {}", account.account, &mint[..8])
            );

            // Close the ATA
            match close_ata(wallet_address, &account.account, mint, account.is_token_2022).await {
                Ok(signature) => {
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Closed ATA {} for mint {}. TX: {}",
                            account.account,
                            &mint[..8],
                            signature
                        )
                    );
                    Ok(signature)
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "ERROR",
                        &format!(
                            "Failed to close ATA {} for mint {}: {}",
                            account.account,
                            &mint[..8],
                            e
                        )
                    );
                    Err(e)
                }
            }
        }
        None => {
            let error_msg = format!("No empty ATA found for mint {}", &mint[..8]);
            log(LogTag::Wallet, "WARNING", &error_msg);
            Err(SwapError::InvalidAmount(error_msg))
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(wallet_address: &str) -> Result<(u32, Vec<String>), SwapError> {
    log(LogTag::Wallet, "ATA", "🔍 Checking for empty token accounts to close...");

    // Get all token accounts for the wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    if all_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No token accounts found in wallet");
        return Ok((0, vec![]));
    }

    // Filter for empty accounts (balance = 0)
    let empty_accounts: Vec<&crate::rpc::TokenAccountInfo> = all_accounts
        .iter()
        .filter(|account| account.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No empty token accounts found to close");
        return Ok((0, vec![]));
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Found {} empty token accounts to close", empty_accounts.len())
    );

    let mut signatures = Vec::new();
    let mut closed_count = 0u32;

    // Close each empty account
    for account_info in empty_accounts {
        log(
            LogTag::Wallet,
            "ATA",
            &format!(
                "Closing empty {} account {} for mint {}",
                if account_info.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                account_info.account,
                account_info.mint
            )
        );

        match
            close_ata(
                wallet_address,
                &account_info.account,
                &account_info.mint,
                account_info.is_token_2022
            ).await
        {
            Ok(signature) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("✅ Closed empty ATA {}. TX: {}", account_info.account, signature)
                );
                signatures.push(signature);
                closed_count += 1;

                // Small delay between closures to avoid overwhelming the network
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("❌ Failed to close ATA {}: {}", account_info.account, e)
                );
                // Continue with other accounts even if one fails
            }
        }
    }

    let rent_reclaimed = (closed_count as f64) * 0.00203928; // Approximate ATA rent in SOL
    log(
        LogTag::Wallet,
        "ATA",
        &format!(
            "🎉 ATA cleanup complete! Closed {} accounts, reclaimed ~{:.6} SOL in rent",
            closed_count,
            rent_reclaimed
        )
    );

    Ok((closed_count, signatures))
}

/// Closes the Associated Token Account (ATA) for a given token mint after selling all tokens
/// This reclaims the rent SOL (~0.002 SOL) from empty token accounts
/// Supports both regular SPL tokens and Token-2022 tokens
pub async fn close_token_account(mint: &str, wallet_address: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close token account for mint: {}", mint));

    // First verify the token balance is actually zero
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => {
            if balance > 0 {
                return Err(
                    SwapError::InvalidAmount(
                        format!("Cannot close token account - still has {} tokens", balance)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Verified zero balance for {}, proceeding to close ATA", mint)
            );
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not verify token balance before closing ATA: {}", e)
            );
            // Continue anyway - the close instruction will fail if tokens remain
        }
    }

    // Get the associated token account address
    let token_account = match get_associated_token_account(wallet_address, mint).await {
        Ok(account) => account,
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not find associated token account for {}: {}", mint, e)
            );
            return Err(e);
        }
    };

    log(LogTag::Wallet, "ATA", &format!("Found token account to close: {}", token_account));

    // Determine if this is a Token-2022 account by checking the token ACCOUNT's program (not the mint)
    let rpc_client = crate::rpc::get_rpc_client();
    let is_token_2022 = rpc_client
        .is_token_account_token_2022(&token_account).await
        .unwrap_or(false);

    if is_token_2022 {
        log(LogTag::Wallet, "ATA", "Detected Token-2022, using Token Extensions program");
    } else {
        log(LogTag::Wallet, "ATA", "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!("Successfully closed token account for {}. TX: {}", mint, signature)
            );
            Ok(signature)
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to close token account for {}: {}", mint, e)
            );
            Err(e)
        }
    }
}

/// Gets the associated token account address for a wallet and mint
async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str
) -> Result<String, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_associated_token_account(wallet_address, mint).await
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        &format!("Closing ATA {} for mint {} using {} program", token_account, mint, if
            is_token_2022
        {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Use proper Solana SDK to build and send close instruction
    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            log(LogTag::Wallet, "SUCCESS", &format!("ATA closed successfully. TX: {}", signature));
            Ok(signature)
        }
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to close ATA: {}", e));
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Parse addresses
    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid wallet address: {}", e))
    )?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid token account: {}", e))
    )?;

    // Decode private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Build close account instruction
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
            &[]
        ).map_err(|e|
            SwapError::TransactionError(format!("Failed to build close instruction: {}", e))
        )?
    };

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Built close instruction for {} account", if is_token_2022 {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Get recent blockhash via RPC
    let rpc_client = crate::rpc::get_rpc_client();
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Build transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    log(LogTag::Wallet, "ATA", "Built and signed close transaction");

    // Send transaction via RPC
    rpc_client.send_transaction(&transaction).await
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey
) -> Result<Instruction, SwapError> {
    // Token-2022 uses the same close account instruction format as SPL Token
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).map_err(|e| SwapError::TransactionError(format!("Invalid Token-2022 program ID: {}", e)))?;

    // Manually build the close account instruction for Token-2022
    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8]; // CloseAccount instruction ID

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false), // Destination for lamports
        AccountMeta::new_readonly(*owner, true) // Authority (signer)
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}


