//! Native SOL and SPL/Token-2022 transfer construction and submission, plus
//! the explicit-keypair ATA close used by multi-wallet tooling.
//!
//! `crate::chains::solana::assets::ata` closes an ATA for the configured
//! bot wallet; this module's [`close_ata`] takes an explicit keypair so
//! multi-wallet tooling can close accounts belonging to any wallet it holds.

use std::str::FromStr;

use crate::chains::solana::constants::TOKEN_2022_PROGRAM_ID;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use crate::chains::solana::{Error, Result};
use crate::logger::{self, LogTag};

/// Transfer SOL from one wallet to another.
pub async fn transfer_sol(
    from_keypair: &Keypair,
    to_address: &str,
    amount_sol: f64,
) -> Result<String> {
    let rpc_client = get_rpc_client();

    let from_pubkey = from_keypair.pubkey();
    let to_pubkey = Pubkey::from_str(to_address).map_err(|_| Error::InvalidAddress {
        kind: "recipient",
        value: to_address.to_owned(),
    })?;

    let lamports = crate::chains::solana::constants::sol_to_lamports(amount_sol);

    let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, lamports);

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_latest_blockhash",
            detail: e.to_string(),
        })?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&from_pubkey),
        &[from_keypair],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| Error::Rpc {
            operation: "send_and_confirm_signed_transaction",
            detail: e.to_string(),
        })?;

    logger::debug(
        LogTag::Tools,
        &format!(
            "SOL transfer: {} -> {}, amount={:.6} SOL, sig={}",
            &from_pubkey.to_string()[..8],
            &to_address[..8],
            amount_sol,
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

/// Transfer SPL/Token-2022 tokens from one wallet to another.
///
/// Fetches token decimals directly from the mint account to ensure accuracy.
pub async fn transfer_token(
    from_keypair: &Keypair,
    to_address: &str,
    mint: &str,
    amount: u64,
    is_token_2022: bool,
) -> Result<String> {
    let rpc_client = get_rpc_client();

    let from_pubkey = from_keypair.pubkey();
    let to_pubkey = Pubkey::from_str(to_address).map_err(|_| Error::InvalidAddress {
        kind: "recipient",
        value: to_address.to_owned(),
    })?;
    let mint_pubkey = Pubkey::from_str(mint).map_err(|_| Error::InvalidAddress {
        kind: "mint",
        value: mint.to_owned(),
    })?;

    // Fetch mint account to get decimals
    let mint_account = rpc_client
        .get_account(&mint_pubkey)
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_account",
            detail: e.to_string(),
        })?
        .ok_or_else(|| Error::AccountNotFound {
            address: mint.to_owned(),
        })?;

    // Parse decimals from mint data (offset 44, 1 byte for SPL Token)
    let decimals = if mint_account.data.len() >= 45 {
        mint_account.data[44]
    } else {
        return Err(Error::Decode {
            payload: "mint account",
            detail: "mint account data is too short to contain decimals".to_owned(),
        });
    };

    let token_program_id = if is_token_2022 {
        Pubkey::from_str(TOKEN_2022_PROGRAM_ID).expect("invalid TOKEN_2022_PROGRAM_ID constant")
    } else {
        crate::chains::solana::spl_token::id()
    };

    let source_ata =
        associated_token_address(&from_pubkey, &mint_pubkey, is_token_2022, &token_program_id);
    let dest_ata =
        associated_token_address(&to_pubkey, &mint_pubkey, is_token_2022, &token_program_id);

    let mut instructions = Vec::new();

    // Check if destination ATA exists, create if not
    let dest_account = rpc_client
        .get_account(&dest_ata)
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_account",
            detail: e.to_string(),
        })?;
    if dest_account.is_none() {
        instructions.push(
            crate::chains::solana::spl_associated_token_account::instruction::create_associated_token_account(
                &from_pubkey,
                &to_pubkey,
                &mint_pubkey,
                &token_program_id,
            ),
        );
    }

    let transfer_ix = crate::chains::solana::spl_token::instruction::transfer_checked(
        &token_program_id,
        &source_ata,
        &mint_pubkey,
        &dest_ata,
        &from_pubkey,
        &[],
        amount,
        decimals,
    )
    .map_err(|e| Error::InstructionBuild {
        instruction: "transfer_checked",
        detail: e.to_string(),
    })?;

    instructions.push(transfer_ix);

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_latest_blockhash",
            detail: e.to_string(),
        })?;

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&from_pubkey),
        &[from_keypair],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| Error::Rpc {
            operation: "send_and_confirm_signed_transaction",
            detail: e.to_string(),
        })?;

    let ui_amount = amount as f64 / 10f64.powi(decimals as i32);
    logger::debug(
        LogTag::Tools,
        &format!(
            "Token transfer: {} -> {}, mint={}, amount={:.6}, sig={}",
            &from_pubkey.to_string()[..8],
            &to_address[..8],
            &mint[..8],
            ui_amount,
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

/// Close an Associated Token Account owned by an explicit keypair, to
/// reclaim rent. Use `chains::solana::assets::ata::close_*` instead when
/// operating on the bot's own configured wallet.
pub async fn close_ata(owner_keypair: &Keypair, mint: &str, is_token_2022: bool) -> Result<String> {
    let rpc_client = get_rpc_client();

    let owner_pubkey = owner_keypair.pubkey();
    let mint_pubkey = Pubkey::from_str(mint).map_err(|_| Error::InvalidAddress {
        kind: "mint",
        value: mint.to_owned(),
    })?;

    let token_program_id = if is_token_2022 {
        Pubkey::from_str(TOKEN_2022_PROGRAM_ID).expect("invalid TOKEN_2022_PROGRAM_ID constant")
    } else {
        crate::chains::solana::spl_token::id()
    };

    let ata = associated_token_address(
        &owner_pubkey,
        &mint_pubkey,
        is_token_2022,
        &token_program_id,
    );

    let close_instruction = if is_token_2022 {
        build_token_2022_close_instruction(&ata, &owner_pubkey)?
    } else {
        crate::chains::solana::spl_token::instruction::close_account(
            &crate::chains::solana::spl_token::id(),
            &ata,
            &owner_pubkey,
            &owner_pubkey,
            &[],
        )
        .map_err(|e| Error::InstructionBuild {
            instruction: "close_account",
            detail: e.to_string(),
        })?
    };

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_latest_blockhash",
            detail: e.to_string(),
        })?;

    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[owner_keypair],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| Error::Rpc {
            operation: "send_and_confirm_signed_transaction",
            detail: e.to_string(),
        })?;

    logger::debug(
        LogTag::Tools,
        &format!(
            "Closed ATA: owner={}, mint={}, sig={}",
            &owner_pubkey.to_string()[..8],
            &mint[..8],
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

// =============================================================================
// WALLET-ID ENTRY POINTS
// =============================================================================
//
// Multi-wallet tooling (`crate::tools::multi_wallet`) identifies wallets only
// by database ID or "the main wallet" — it never holds a `Keypair`. These
// resolve+decrypt the signer here, inside `crate::chains::solana`, use it for
// exactly one transaction, and drop it.

/// Transfer SOL from the main wallet.
pub async fn transfer_sol_from_main(to_address: &str, amount_sol: f64) -> Result<String> {
    let keypair = crate::chains::solana::accounts::main_keypair().await?;
    transfer_sol(&keypair, to_address, amount_sol).await
}

/// Transfer SOL from a wallet identified by database ID.
pub async fn transfer_sol_for_wallet(
    wallet_id: i64,
    to_address: &str,
    amount_sol: f64,
) -> Result<String> {
    let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
    transfer_sol(&keypair, to_address, amount_sol).await
}

/// Transfer SPL/Token-2022 tokens from a wallet identified by database ID.
pub async fn transfer_token_for_wallet(
    wallet_id: i64,
    to_address: &str,
    mint: &str,
    amount: u64,
    is_token_2022: bool,
) -> Result<String> {
    let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
    transfer_token(&keypair, to_address, mint, amount, is_token_2022).await
}

/// Close an ATA owned by a wallet identified by database ID.
pub async fn close_ata_for_wallet(
    wallet_id: i64,
    mint: &str,
    is_token_2022: bool,
) -> Result<String> {
    let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
    close_ata(&keypair, mint, is_token_2022).await
}

fn associated_token_address(
    owner: &Pubkey,
    mint: &Pubkey,
    is_token_2022: bool,
    token_program_id: &Pubkey,
) -> Pubkey {
    if is_token_2022 {
        crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id(
            owner, mint, token_program_id,
        )
    } else {
        crate::chains::solana::spl_associated_token_account::get_associated_token_address(
            owner, mint,
        )
    }
}

/// Build the CloseAccount instruction for a Token-2022 account.
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction> {
    let token_2022_program_id =
        Pubkey::from_str(TOKEN_2022_PROGRAM_ID).map_err(|_| Error::InvalidAddress {
            kind: "program id",
            value: TOKEN_2022_PROGRAM_ID.to_owned(),
        })?;

    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8];

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false),         // Destination for lamports
        AccountMeta::new_readonly(*owner, true), // Owner/authority
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}
