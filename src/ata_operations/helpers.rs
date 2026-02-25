//! Internal helper functions for ATA operations.

use crate::constants::TOKEN_2022_PROGRAM_ID;
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::{Error, Result};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use std::str::FromStr;

/// Gets the associated token account address for a wallet and mint
pub(super) async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str,
) -> Result<String> {
    let rpc_client = get_rpc_client();
    rpc_client
        .get_associated_token_account(wallet_address, mint)
        .await
        .map_err(Error::from)
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
