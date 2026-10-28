// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Internal helper functions for ATA operations.

use crate::chains::solana::access::chain_access;
use crate::chains::solana::constants::TOKEN_2022_PROGRAM_ID;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
};
use crate::chains::solana::spl_token::instruction::close_account;
use crate::chains::solana::swaps::presend::send_and_settle_signed;
use crate::logger::{self, LogTag};
use crate::{Error, Result};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock, Mutex};

/// One lock per token account with a close in flight. The post-swap WSOL sweep and
/// the periodic ATA cleanup can pick the same empty account at the same moment;
/// without this both would send a close and the second would fail against an
/// account that no longer exists.
static CLOSE_LOCKS: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The close lock of `token_account`, dropping the locks no close holds any more.
fn close_lock(token_account: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = CLOSE_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    locks.retain(|_, lock| Arc::strong_count(lock) > 1);
    locks.entry(token_account.to_owned()).or_default().clone()
}

/// Whether `error` reports that the account is already gone: a close that lost
/// the race to another one, which reclaimed the rent itself.
pub(super) fn is_already_closed(error: &Error) -> bool {
    matches!(
        error,
        Error::Solana(crate::chains::solana::Error::AccountNotFound { .. })
    )
}

/// Gets the associated token account address for a wallet and mint
pub(super) async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str,
) -> Result<String> {
    chain_access()
        .associated_token_account(wallet_address, mint)
        .await
}

/// Closes ATA using proper Solana SDK for real ATA closing
pub(super) async fn close_ata(
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

    // A close that had to wait for another close of the same account sends only
    // if the account survived it.
    let lock = close_lock(token_account);
    let _closing = match lock.clone().try_lock_owned() {
        Ok(guard) => guard,
        Err(_) => {
            let guard = lock.lock_owned().await;
            let pubkey = Pubkey::from_str(token_account).map_err(|e| {
                Error::invalid_amount(
                    format!("Invalid token account: {e}"),
                    "Token account validation failed".to_owned(),
                )
            })?;
            let exists = chain_access().account_exists(&pubkey).await?;
            if !exists {
                return Err(crate::chains::solana::Error::AccountNotFound {
                    address: token_account.to_owned(),
                }
                .into());
            }
            guard
        }
    };

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
    logger::debug(LogTag::Wallet, "ATA_KEYPAIR: creating keypair from config");

    let keypair = crate::chains::solana::accounts::configured_keypair().map_err(|e| {
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
            &crate::chains::solana::spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[],
        )
        .map_err(|e| crate::chains::solana::Error::InstructionBuild {
            instruction: "close_account",
            detail: e.to_string(),
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

    let (recent_blockhash, last_valid_block_height) = chain_access().latest_blockhash().await?;

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

    let result = send_and_settle_signed(&transaction, last_valid_block_height)
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
            logger::debug(LogTag::Wallet, &format!("ATA_TRANSACTION_FAILED: {e}"));
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
        crate::chains::solana::Error::Decode {
            payload: "token-2022 program id",
            detail: e.to_string(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closes_of_one_account_share_a_lock_and_idle_locks_are_dropped() {
        let first = close_lock("CloseLockAccountA");
        let second = close_lock("CloseLockAccountA");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &close_lock("CloseLockAccountB")));

        drop((first, second));
        let _other = close_lock("CloseLockAccountC");
        let locks = CLOSE_LOCKS.lock().unwrap();
        assert!(!locks.contains_key("CloseLockAccountA"));
    }

    #[test]
    fn only_a_missing_account_counts_as_already_closed() {
        let gone: Error = crate::chains::solana::Error::AccountNotFound {
            address: "x".to_owned(),
        }
        .into();
        assert!(is_already_closed(&gone));
        let other: Error = crate::chains::solana::Error::InvalidPool {
            reason: "x".to_owned(),
        }
        .into();
        assert!(!is_already_closed(&other));
    }
}
