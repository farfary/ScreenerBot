use anyhow::Result;
use solana_sdk::{signature::{Keypair, Signer}, pubkey::Pubkey};
use solana_client::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;
use spl_token_2022::instruction::close_account;  // Use updated spl_token crate
use std::str::FromStr;

use screenerbot::swap_gmgn::sell_all_gmgn;
use screenerbot::configs::CONFIGS;
use screenerbot::helpers::{get_all_tokens, get_biggest_token_amount};

#[tokio::main]
async fn main() -> Result<()> {
    let wallet = {
        let bytes = bs58::decode(&CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let wallet_pubkey = wallet.pubkey();
    
    let rpc_client = RpcClient::new(CONFIGS.rpc_url.clone());
    
    // Step 1: Get all tokens in the wallet and sell them
    let tokens = get_all_tokens(); // Get all tokens in wallet using helpers
    for (token_mint, _amount) in tokens {
        let token_amount = get_biggest_token_amount(&token_mint); // Get the biggest amount for this token

        if token_amount > 0 {
            println!("Attempting to sell all tokens for mint: {}", token_mint);
            match sell_all_gmgn(&token_mint).await {
                Ok(tx_hash) => {
                    println!("✅ Sold successfully. Transaction hash: {}", tx_hash);
                },
                Err(e) => {
                    println!("❌ Failed to sell token: {:?}", e);
                }
            }
        }
    }
    
    // Step 2: Find and close all associated token accounts (ATAs)
    let at_as = get_associated_token_addresses(&rpc_client, &wallet_pubkey)?;
    for ata in at_as {
        println!("Attempting to close ATA: {:?}", ata); // Use `{:?}` to debug format
        match close_associated_token_account(&rpc_client, &wallet, ata).await {
            Ok(_) => {
                println!("✅ Closed ATA successfully: {:?}", ata); // Use `{:?}` to debug format
            },
            Err(e) => {
                println!("❌ Failed to close ATA {:?}: {:?}", ata, e); // Use `{:?}` to debug format
            }
        }
    }

    Ok(())
}

/// Get all ATAs for the wallet
fn get_associated_token_addresses(rpc_client: &RpcClient, wallet_pubkey: &Pubkey) -> Result<Vec<Pubkey>> {
    let at_as = rpc_client.get_token_accounts_by_owner(wallet_pubkey, solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id()))?;
    
    let mut atas = Vec::new();
    for ata in at_as {
        let ata_pubkey = Pubkey::from_str(&format!("{:?}", ata.account.data))?;
        atas.push(ata_pubkey);
    }

    Ok(atas)
}

/// Close an associated token account (ATA)
async fn close_associated_token_account(rpc_client: &RpcClient, wallet: &Keypair, ata: Pubkey) -> Result<()> {
    let ata_owner = wallet.pubkey();
    let ix = close_account(
        &spl_token_2022::id(),
        &ata,
        &ata_owner,
        &ata_owner,
        &[],
    )?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&wallet.pubkey()));
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    tx.sign(&[wallet], recent_blockhash);

    rpc_client.send_and_confirm_transaction(&tx)?; // Removed unused variable 'result'
    println!("Successfully closed ATA: {:?}", ata); // Use `{:?}` for debug output
    Ok(())
}
