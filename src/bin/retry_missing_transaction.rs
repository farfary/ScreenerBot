use std::env;
use std::path::Path;
use screenerbot::logger::{log, LogTag};
use screenerbot::rpc::get_rpc_client;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <transaction_signature>", args[0]);
        eprintln!("Example: {} zjwCpres7SZfUWg82EiJxfKjokpG7bbgoBtGDfZK87HQdCh2WCPsq2uBFTreq31NmvwurvFLU8LhrJzcNzoF3ZP", args[0]);
        std::process::exit(1);
    }

    let signature_str = &args[1];
    
    println!("🔍 Attempting to fetch transaction: {}", signature_str);
    
    // Parse signature
    let signature = match Signature::from_str(signature_str) {
        Ok(sig) => sig,
        Err(e) => {
            eprintln!("❌ Invalid signature format: {}", e);
            std::process::exit(1);
        }
    };
    
    // Get RPC client
    let rpc_client = get_rpc_client();
    
    // Try fetching with different methods
    println!("\n📡 Method 1: Standard get_transaction");
    match rpc_client.client().get_transaction(&signature, UiTransactionEncoding::Json) {
        Ok(tx) => {
            println!("✅ Successfully fetched transaction (standard method)");
            println!("   Slot: {:?}", tx.slot);
            println!("   Block time: {:?}", tx.block_time);
            save_transaction_to_file(signature_str, &tx).await?;
        }
        Err(e) => {
            println!("❌ Standard method failed: {}", e);
        }
    }
    
    println!("\n📡 Method 2: get_transaction_with_config (JsonParsed)");
    match rpc_client.client().get_transaction_with_config(
        &signature,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    ) {
        Ok(tx) => {
            println!("✅ Successfully fetched transaction (JsonParsed method)");
            println!("   Slot: {:?}", tx.slot);
            println!("   Block time: {:?}", tx.block_time);
            save_transaction_to_file(signature_str, &tx).await?;
        }
        Err(e) => {
            println!("❌ JsonParsed method failed: {}", e);
        }
    }
    
    println!("\n📡 Method 3: get_transaction_with_config (Base64)");
    match rpc_client.client().get_transaction_with_config(
        &signature,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    ) {
        Ok(tx) => {
            println!("✅ Successfully fetched transaction (Base64 method)");
            println!("   Slot: {:?}", tx.slot);
            println!("   Block time: {:?}", tx.block_time);
            save_transaction_to_file(signature_str, &tx).await?;
        }
        Err(e) => {
            println!("❌ Base64 method failed: {}", e);
        }
    }
    
    println!("\n📡 Method 4: Check if transaction exists at all");
    match rpc_client.client().get_signature_statuses(&[signature]) {
        Ok(response) => {
            if let Some(status_result) = response.value.get(0) {
                if let Some(status) = status_result {
                    println!("✅ Transaction exists on blockchain");
                    println!("   Confirmation status: {:?}", status.confirmation_status);
                    println!("   Error: {:?}", status.err);
                    println!("   Slot: {:?}", status.slot);
                } else {
                    println!("❌ Transaction not found on blockchain");
                }
            } else {
                println!("❌ No status response");
            }
        }
        Err(e) => {
            println!("❌ Failed to check transaction status: {}", e);
        }
    }
    
    // Try using our existing transaction tools
    println!("\n📊 Method 5: Using screenerbot's transaction analysis");
    match screenerbot::transactions_tools::analyze_post_swap_transaction_simple(signature_str, "FYmfcfwyx8K1MnBmk6d66eeNPoPMbTXEMve5Tk1pGgiC").await {
        Ok(analysis) => {
            println!("✅ Successfully analyzed transaction using screenerbot tools");
            println!("   SOL amount: {}", analysis.sol_amount);
            println!("   Token amount: {}", analysis.token_amount);
            println!("   Effective price: {:.12}", analysis.effective_price);
            println!("   Transaction fee: {:?}", analysis.transaction_fee);
            println!("   Block time: {:?}", analysis.block_time);
        }
        Err(e) => {
            println!("❌ Screenerbot analysis failed: {}", e);
        }
    }
    
    println!("\n🏁 Transaction fetch attempt completed.");
    Ok(())
}

async fn save_transaction_to_file(
    signature: &str, 
    tx: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = Path::new("data");
    let transactions_dir = data_dir.join("transactions");
    
    // Create directory if it doesn't exist
    if !transactions_dir.exists() {
        std::fs::create_dir_all(&transactions_dir)?;
    }
    
    let file_path = transactions_dir.join(format!("{}.json", signature));
    
    // Save transaction to file
    let json_content = serde_json::to_string_pretty(tx)?;
    std::fs::write(&file_path, json_content)?;
    
    println!("💾 Transaction saved to: {}", file_path.display());
    log(LogTag::Transactions, "SAVED", &format!("Transaction {} manually saved to disk", signature));
    
    Ok(())
}
