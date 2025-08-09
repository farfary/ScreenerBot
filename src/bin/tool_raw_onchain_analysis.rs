/// Raw on-chain transaction analysis tool
/// Fetches real blockchain data without caching to analyze TROLLER position

use screenerbot::{
    tokens::get_token_decimals,
    swaps::transaction::get_wallet_address,
    rpc::get_rpc_client,
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::{
    UiTransactionEncoding,
    option_serializer::OptionSerializer,
};
use std::str::FromStr;
use tokio;

// TROLLER transaction signatures from positions.json
const ENTRY_TX: &str = "2aWspLk7FEk8YQUnj5EUwNotgmCZ3UPCeNePgZxKMtTGTNPuVmMFzUXyfGNjqBzjwjqkxLVL8ptWUcUtnPBNfBKD";
const EXIT_TX: &str = "3eRnN92TCkWSUhhpQBB7Pm94PpzZymUgHcESEhphSAdoFkWxNrMbJEvbwCiJPn6MTmN4ZT5U3e7uATkf4wSaVA8Q";
const TOKEN_MINT: &str = "DjPB9mLpfAACHoLvM1v7EksupqS4qK4HzGhsLqgnpump";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RAW ON-CHAIN ANALYSIS (NO CACHE) ===\n");

    // Get wallet address
    let wallet_address = get_wallet_address()?;
    let wallet_pubkey = Pubkey::from_str(&wallet_address)?;
    println!("Wallet: {}", wallet_address);
    println!("Token: {}", TOKEN_MINT);
    println!();

    // Get token decimals
    let token_decimals = get_token_decimals(TOKEN_MINT).await.ok_or("Failed to get token decimals")?;
    println!("Token decimals: {}", token_decimals);
    println!();

    // Get RPC client
    let rpc_client = get_rpc_client();

    // Analyze entry transaction
    println!("=== ENTRY TRANSACTION ANALYSIS ===");
    analyze_transaction(rpc_client, ENTRY_TX, &wallet_pubkey, TOKEN_MINT, token_decimals, true).await?;
    println!();

    // Analyze exit transaction
    println!("=== EXIT TRANSACTION ANALYSIS ===");
    analyze_transaction(rpc_client, EXIT_TX, &wallet_pubkey, TOKEN_MINT, token_decimals, false).await?;
    println!();

    Ok(())
}

async fn analyze_transaction(
    rpc_client: &screenerbot::rpc::RpcClient,
    tx_signature: &str,
    wallet_pubkey: &Pubkey,
    token_mint: &str,
    token_decimals: u8,
    is_entry: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = Signature::from_str(tx_signature)?;
    
    println!("Transaction: {}...", &tx_signature[..8]);
    
    // Get transaction with full details
    let transaction = rpc_client.get_transaction_with_config(
        &signature,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    )?;

    if let Some(tx) = transaction.transaction {
        // Get account balances before and after
        let pre_balances = tx.meta.as_ref().unwrap().pre_balances.clone();
        let post_balances = tx.meta.as_ref().unwrap().post_balances.clone();
        let account_keys = if let Some(loaded_addresses) = &tx.meta.as_ref().unwrap().loaded_addresses {
            let mut keys = tx.transaction.message.account_keys().unwrap();
            keys.extend(&loaded_addresses.writable);
            keys.extend(&loaded_addresses.readonly);
            keys
        } else {
            tx.transaction.message.account_keys().unwrap()
        };

        // Find wallet's SOL balance change
        let mut wallet_sol_change = 0i64;
        for (i, pubkey) in account_keys.iter().enumerate() {
            if pubkey == wallet_pubkey {
                if i < pre_balances.len() && i < post_balances.len() {
                    wallet_sol_change = post_balances[i] as i64 - pre_balances[i] as i64;
                    break;
                }
            }
        }

        println!("Wallet SOL change: {} lamports ({:.9} SOL)", 
                wallet_sol_change, wallet_sol_change as f64 / 1_000_000_000.0);

        // Analyze token balance changes if this involves tokens
        analyze_token_changes(&tx, wallet_pubkey, token_mint, token_decimals).await?;

        // Look for ATA creation/closure
        analyze_ata_operations(&tx, wallet_pubkey, token_mint).await?;

        // Calculate fees
        let fee = tx.meta.as_ref().unwrap().fee;
        println!("Transaction fee: {} lamports ({:.9} SOL)", fee, fee as f64 / 1_000_000_000.0);

        // For entry transactions, negative SOL change means SOL spent
        // For exit transactions, positive SOL change means SOL received
        let abs_sol_change = wallet_sol_change.abs() as f64 / 1_000_000_000.0;
        if is_entry {
            println!("SOL spent (total): {:.9} SOL", abs_sol_change);
            println!("SOL spent (excluding fee): {:.9} SOL", abs_sol_change - (fee as f64 / 1_000_000_000.0));
        } else {
            println!("SOL received (total): {:.9} SOL", abs_sol_change);
        }
    }

    Ok(())
}

async fn analyze_token_changes(
    tx: &solana_transaction_status::EncodedTransactionWithStatusMeta,
    wallet_pubkey: &Pubkey,
    token_mint: &str,
    token_decimals: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    
    if let Some(pre_token_balances) = &tx.meta.as_ref().unwrap().pre_token_balances {
        if let Some(post_token_balances) = &tx.meta.as_ref().unwrap().post_token_balances {
            
            // Track token balance changes for our wallet and token
            let mut pre_balance = 0u64;
            let mut post_balance = 0u64;
            
            for balance in pre_token_balances {
                if balance.mint == token_mint && balance.owner.as_ref() == Some(&wallet_pubkey.to_string()) {
                    if let OptionSerializer::Some(amount) = &balance.ui_token_amount.amount {
                        pre_balance = amount.parse().unwrap_or(0);
                    }
                }
            }
            
            for balance in post_token_balances {
                if balance.mint == token_mint && balance.owner.as_ref() == Some(&wallet_pubkey.to_string()) {
                    if let OptionSerializer::Some(amount) = &balance.ui_token_amount.amount {
                        post_balance = amount.parse().unwrap_or(0);
                    }
                }
            }
            
            let token_change = post_balance as i64 - pre_balance as i64;
            let token_change_ui = token_change as f64 / 10_f64.powi(token_decimals as i32);
            
            println!("Token balance change: {} raw ({:.6} UI tokens)", token_change, token_change_ui);
        }
    }
    
    Ok(())
}

async fn analyze_ata_operations(
    tx: &solana_transaction_status::EncodedTransactionWithStatusMeta,
    wallet_pubkey: &Pubkey,
    token_mint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    
    // Look for ATA creation/closure in the instructions
    if let Some(instructions) = &tx.transaction.message.instructions() {
        for instruction in instructions {
            // Check if this is an ATA creation instruction
            if let Ok(parsed) = serde_json::to_string(&instruction) {
                if parsed.contains("createAccount") || parsed.contains("CreateAccount") {
                    println!("Found account creation instruction");
                }
                if parsed.contains("closeAccount") || parsed.contains("CloseAccount") {
                    println!("Found account closure instruction");
                }
            }
        }
    }
    
    Ok(())
}
