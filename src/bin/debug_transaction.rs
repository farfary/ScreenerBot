use std::error::Error;
use screenerbot::global::read_configs;
use screenerbot::wallet::calculate_effective_price;
use reqwest;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Deserialize, Serialize)]
struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    pub fee: u64,
    pub pre_balances: Option<Vec<u64>>,
    pub post_balances: Option<Vec<u64>>,
    pub log_messages: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransactionData {
    pub signatures: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TransactionDetails {
    pub transaction: TransactionData,
    pub meta: Option<TransactionMeta>,
}

async fn get_transaction_details(
    client: &reqwest::Client,
    transaction_signature: &str,
    rpc_url: &str
) -> Result<TransactionDetails, Box<dyn Error>> {
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get transaction details: {}", response.status()).into());
    }

    let rpc_response: serde_json::Value = response.json().await?;

    if let Some(error) = rpc_response.get("error") {
        return Err(format!("RPC error getting transaction: {:?}", error).into());
    }

    if let Some(result) = rpc_response.get("result") {
        if result.is_null() {
            return Err("Transaction not found or not confirmed yet".into());
        }

        let transaction_details: TransactionDetails = serde_json::from_value(result.clone())?;
        return Ok(transaction_details);
    }

    Err("Invalid transaction response format".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::init();

    let tx_signature =
        "4v5gUgdxeE1gmeirsU6YRv41TxxhUdKbMeG2cmBh8TytitPeGJSLJEssB3GuepuHqSgJGDp3bNX7x1QBd91JtkJU";

    println!("🔍 Analyzing transaction: {}", tx_signature);
    println!("=========================================");

    // Load configuration
    let configs = read_configs("configs.json")?;
    let client = reqwest::Client::new();

    // Try all available RPC endpoints
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    let mut transaction_found = false;

    for (rpc_idx, rpc_endpoint) in rpc_endpoints.iter().enumerate() {
        println!("\n🔄 Trying RPC {}: {}", rpc_idx + 1, rpc_endpoint);

        match get_transaction_details(&client, tx_signature, rpc_endpoint).await {
            Ok(transaction_details) => {
                transaction_found = true;
                println!("✅ Transaction found on RPC {}!", rpc_idx + 1);
                println!("📊 Transaction Details:");
                println!("   Signature: {}", tx_signature);

                // Check if the transaction was successful
                if let Some(meta) = &transaction_details.meta {
                    let is_successful = meta.err.is_none();
                    println!("   Status: {}", if is_successful {
                        "✅ SUCCESS"
                    } else {
                        "❌ FAILED"
                    });

                    if let Some(err) = &meta.err {
                        println!("   Error Details: {:#}", err);

                        // Try to extract specific error information
                        if let Some(err_obj) = err.as_object() {
                            if let Some(instruction_error) = err_obj.get("InstructionError") {
                                println!("   Instruction Error: {:#}", instruction_error);
                            }
                        }
                    }

                    println!(
                        "   Fee: {} lamports ({:.9} SOL)",
                        meta.fee,
                        (meta.fee as f64) / 1_000_000_000.0
                    );

                    // Show balance changes
                    if let (Some(pre), Some(post)) = (&meta.pre_balances, &meta.post_balances) {
                        println!("   Balance Changes:");
                        let mut total_sol_change = 0i64;
                        for (i, (pre_bal, post_bal)) in pre.iter().zip(post.iter()).enumerate() {
                            let change = (*post_bal as i64) - (*pre_bal as i64);
                            if change != 0 {
                                println!(
                                    "     Account {}: {} -> {} (change: {} lamports = {:.9} SOL)",
                                    i,
                                    pre_bal,
                                    post_bal,
                                    change,
                                    (change as f64) / 1_000_000_000.0
                                );
                                total_sol_change += change;
                            }
                        }
                        println!(
                            "   Total Net SOL Change: {} lamports ({:.9} SOL)",
                            total_sol_change,
                            (total_sol_change as f64) / 1_000_000_000.0
                        );
                    }

                    // Show log messages if any
                    if let Some(log_messages) = &meta.log_messages {
                        println!("   Transaction Logs ({} messages):", log_messages.len());
                        for (i, msg) in log_messages.iter().enumerate() {
                            if
                                msg.contains("Error") ||
                                msg.contains("failed") ||
                                msg.contains("insufficient")
                            {
                                println!("     🔴 {}: {}", i, msg);
                            } else if msg.contains("success") || msg.contains("Program log: ") {
                                println!("     🟢 {}: {}", i, msg);
                            } else {
                                println!("     📝 {}: {}", i, msg);
                            }
                        }
                    }

                    // Conclusion
                    println!("\n📊 Analysis Result:");
                    if is_successful {
                        println!("   ✅ This transaction was SUCCESSFUL on-chain");
                        println!(
                            "   💡 If this transaction was saved as a failed position, there's a bug in our validation logic"
                        );
                    } else {
                        println!("   ❌ This transaction FAILED on-chain");
                        println!(
                            "   💡 This transaction should NOT be saved as a successful position"
                        );
                        println!(
                            "   � If this was saved as a position, our validation is missing failed transaction detection"
                        );
                    }
                } else {
                    println!("   ⚠️  No metadata available");
                }

                break;
            }
            Err(e) => {
                println!("   ❌ RPC {} failed: {}", rpc_idx + 1, e);
            }
        }
    }

    if !transaction_found {
        println!("\n❌ Transaction not found on any RPC endpoint");
        println!("💡 This could mean:");
        println!("   - Transaction signature is incorrect");
        println!("   - Transaction is too old and pruned");
        println!("   - All RPC endpoints are having issues");
    }

    println!("\n🔍 Analysis complete!");
    Ok(())
}
