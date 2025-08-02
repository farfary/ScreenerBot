use std::env;
use tokio;

// Import all the necessary modules from your project
use screenerbot::global::*;
use screenerbot::rpc::*;
use screenerbot::swap_calculator::*;
use screenerbot::utils::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize systems
    init_rpc_client()?;

    let signature =
        "2HF75cBDmyzajhLihvj9UhZErQbS16t5y5qwXaXF5nf3r1R2nN2wFyfBB4G1BM1AFLjTa9KMMRBsLVof75NWE5Vk";
    let input_mint = "So11111111111111111111111111111111111111112"; // SOL
    let output_mint = "DGKj2gcKkrYnJYLGN89d1yStpx7r6yPkR166opx2bonk"; // Token
    let wallet_address = "B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59";

    println!("🔍 Testing enhanced inner instruction analysis");
    println!("📄 Transaction: {}", signature);
    println!("💱 Swap: SOL -> Token");
    println!("👛 Wallet: {}", &wallet_address[..8]);

    // Get RPC client
    let rpc_client = get_rpc_client();

    // Fetch transaction
    println!("\n📡 Fetching transaction from RPC...");
    let signature_obj = signature.parse().unwrap();
    let transaction = rpc_client.get_transaction(
        &signature_obj,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(solana_account_decoder::UiTransactionEncoding::Json),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        }
    ).await?;

    if let Some(tx) = transaction.transaction.transaction {
        if let solana_transaction_status::EncodedTransaction::Json(ui_tx) = tx {
            println!("✅ Transaction fetched successfully");

            // Convert to JSON Value for our analysis function
            let tx_json = serde_json::to_value(&transaction)?;

            // Test our enhanced analysis
            println!("\n🔍 Running enhanced inner instructions analysis...");
            match analyze_inner_instructions(&tx_json, input_mint, output_mint, wallet_address) {
                Ok(result) => {
                    println!("✅ Analysis successful!");
                    println!(
                        "📊 Input: {:.6} {} (decimals: {})",
                        result.input_amount,
                        if input_mint.contains("111111111112") {
                            "SOL"
                        } else {
                            "tokens"
                        },
                        result.input_decimals
                    );
                    println!(
                        "📊 Output: {:.6} {} (decimals: {})",
                        result.output_amount,
                        if output_mint.contains("111111111112") {
                            "SOL"
                        } else {
                            "tokens"
                        },
                        result.output_decimals
                    );
                    println!("🎯 Confidence: {:.2}", result.confidence);
                    println!("🔧 Method: {}", result.method);
                }
                Err(e) => {
                    println!("❌ Analysis failed: {}", e);
                }
            }
        } else {
            println!("❌ Unexpected transaction encoding");
        }
    } else {
        println!("❌ Transaction not found");
    }

    Ok(())
}
