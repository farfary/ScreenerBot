//! SPL/Token-2022 burn construction and submission for the configured
//! trading wallet, used by the dashboard's "burn selected tokens" tool.

use std::str::FromStr;

use crate::chains::solana::constants::TOKEN_2022_PROGRAM_ID;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    pubkey::Pubkey, signer::Signer as _, transaction::Transaction,
};
use crate::chains::solana::spl_token::instruction as spl_instruction;

/// Burn `amount` of `mint` from the configured wallet's associated token
/// account, signing with `configured_keypair()` — the keypair never leaves
/// this function.
pub async fn burn_configured_wallet_token(
    wallet_address: &str,
    ata_address: &str,
    mint: &str,
    amount: u64,
    is_token_2022: bool,
) -> Result<String, String> {
    let wallet_pubkey =
        Pubkey::from_str(wallet_address).map_err(|e| format!("Invalid wallet address: {e}"))?;
    let mint_pubkey = Pubkey::from_str(mint).map_err(|e| format!("Invalid mint address: {e}"))?;
    let ata_pubkey =
        Pubkey::from_str(ata_address).map_err(|e| format!("Invalid ATA address: {e}"))?;

    let token_program_id = if is_token_2022 {
        crate::chains::solana::spl_token_2022::id()
    } else {
        crate::chains::solana::spl_token::id()
    };

    let burn_instruction = spl_instruction::burn(
        &token_program_id,
        &ata_pubkey,
        &mint_pubkey,
        &wallet_pubkey,
        &[&wallet_pubkey],
        amount,
    )
    .map_err(|e| format!("Failed to create burn instruction: {e}"))?;

    let rpc_client = get_rpc_client();
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("Failed to get blockhash: {e}"))?;

    let keypair = crate::chains::solana::accounts::configured_keypair()?;

    let transaction = Transaction::new_signed_with_payer(
        &[burn_instruction],
        Some(&wallet_pubkey),
        &[&keypair],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| format!("Transaction failed: {e}"))?;

    Ok(signature.to_string())
}
