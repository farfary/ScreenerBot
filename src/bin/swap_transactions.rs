#![allow(warnings)]

use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::{Keypair, Signer}, transaction::Transaction, instruction::Instruction};
use serde_json::json;  // Importing the json macro
use std::{str::FromStr};
use screenerbot::configs::{CONFIGS};  // Importing CONFIGS from your configs
use bs58;
use solana_transaction_status::{UiTransactionEncoding, EncodedTransactionWithStatusMeta};
use solana_sdk::signature::Signature;

fn main() {
    // Load wallet keypair from the private key in `configs`
    let wallet_private_key = CONFIGS.main_wallet_private.clone();
    let keypair_bytes = bs58::decode(&wallet_private_key).into_vec().expect("Invalid private key");
    let keypair = Keypair::try_from(keypair_bytes.as_slice()).expect("Failed to create keypair from bytes");

    // Initialize Solana RPC client using the RPC URL from the config
    let client = RpcClient::new(CONFIGS.rpc_url.clone());

    // Fetch the last 10 transactions for the wallet
    match get_last_10_transactions(&client, &keypair.pubkey()) {
        Ok(transactions) => {
            println!("Last 10 transactions for wallet {}:", keypair.pubkey());
            for tx in transactions {
                println!("{:?}", tx);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch transactions: {}", e);
        }
    }
}

fn get_last_10_transactions(client: &RpcClient, wallet_pubkey: &Pubkey) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Get the confirmed signatures for the wallet
    let tx_signatures = client.get_signatures_for_address(wallet_pubkey)?;

    // Limit to the last 10 transactions
    let last_10_signatures = tx_signatures.into_iter().take(10).collect::<Vec<_>>();

    let mut transactions = Vec::new();

    // Loop through each transaction signature
    for tx in last_10_signatures {
        // Fetch the transaction details using the signature
        let txn_details = client.get_transaction(
            &tx.signature.parse::<Signature>()?,
            UiTransactionEncoding::Json,
        )?;

        // Collect transaction details as a JSON string
        let txn_data = json!({
            "signature": tx.signature,
            "transaction": txn_details.transaction
        });

        transactions.push(txn_data.to_string());
    }

    Ok(transactions)
}
