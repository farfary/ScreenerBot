use screenerbot::global::read_configs;
use screenerbot::wallet::lamports_to_sol;
use serde_json::Value;
use std::collections::HashMap;
use reqwest;
use serde::{ Deserialize, Serialize };

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionDetails {
    pub slot: u64,
    pub transaction: Transaction,
    pub meta: Option<TransactionMeta>,
    pub block_time: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction {
    pub message: Message,
    pub signatures: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    #[serde(rename = "accountKeys")]
    pub account_keys: Vec<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Instruction {
    #[serde(rename = "programIdIndex")]
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionMeta {
    pub err: Option<Value>,
    pub fee: u64,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pub pre_token_balances: Option<Vec<Value>>,
    #[serde(rename = "postTokenBalances")]
    pub post_token_balances: Option<Vec<Value>>,
}

async fn get_transaction_details(
    client: &reqwest::Client,
    transaction_signature: &str,
    rpc_url: &str
) -> Result<TransactionDetails, Box<dyn std::error::Error>> {
    let request_body =
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
        .json(&request_body)
        .send().await?;

    let response_text = response.text().await?;
    let response_json: Value = serde_json::from_str(&response_text)?;

    if let Some(error) = response_json.get("error") {
        return Err(format!("RPC error: {}", error).into());
    }

    let result = response_json.get("result").ok_or("No result in response")?;

    if result.is_null() {
        return Err("Transaction not found".into());
    }

    let transaction_details: TransactionDetails = serde_json::from_value(result.clone())?;
    Ok(transaction_details)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read transaction signature from command line args
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <transaction_signature>", args[0]);
        println!(
            "Example: cargo run --bin tool_transactions_swap -- 3LzkLjYifcq74PgpvWuc4UCmkjWttuLjxZd3FX3UrpRCBeyLxosy59STCenAAt4aBKpMy8Lcv69BQhH1fkV6dCFe"
        );
        return Ok(());
    }

    let transaction_signature = &args[1];
    println!("🔍 Investigating transaction: {}", transaction_signature);
    println!("📊 Transaction URL: https://solscan.io/tx/{}", transaction_signature);
    println!();

    // Load configs
    let configs = read_configs("configs.json")?;
    let client = reqwest::Client::new();

    // Get transaction details from all RPC endpoints
    println!("📡 Fetching transaction details...");
    let mut transaction_details = None;

    // Try primary RPC first
    match get_transaction_details(&client, transaction_signature, &configs.rpc_url).await {
        Ok(details) => {
            transaction_details = Some(details);
            println!("✅ Got transaction from primary RPC: {}", configs.rpc_url);
        }
        Err(e) => {
            println!("❌ Primary RPC failed: {}", e);

            // Try fallback RPCs
            for (i, fallback_url) in configs.rpc_fallbacks.iter().enumerate() {
                match get_transaction_details(&client, transaction_signature, fallback_url).await {
                    Ok(details) => {
                        transaction_details = Some(details);
                        println!(
                            "✅ Got transaction from fallback RPC {}: {}",
                            i + 1,
                            fallback_url
                        );
                        break;
                    }
                    Err(e) => {
                        println!("❌ Fallback RPC {} failed: {}", i + 1, e);
                    }
                }
            }
        }
    }

    let details = transaction_details.ok_or("Failed to get transaction details from any RPC")?;
    let meta = details.meta.as_ref().ok_or("Transaction metadata not available")?;

    println!();
    println!("🔍 TRANSACTION ANALYSIS");
    println!("=======================");

    // Basic transaction info
    println!("📋 Basic Info:");
    println!("   Slot: {}", details.slot);
    println!("   Block time: {}", details.block_time.unwrap_or(0));
    if let Some(err) = &meta.err {
        println!("   ❌ Transaction failed: {:?}", err);
        return Ok(());
    } else {
        println!("   ✅ Transaction succeeded");
    }
    println!("   Fee: {} lamports ({:.9} SOL)", meta.fee, lamports_to_sol(meta.fee));
    println!();

    // Account information
    println!("👥 Accounts ({} total):", details.transaction.message.account_keys.len());
    for (i, account) in details.transaction.message.account_keys.iter().enumerate() {
        let pre_balance = meta.pre_balances.get(i).unwrap_or(&0);
        let post_balance = meta.post_balances.get(i).unwrap_or(&0);
        let change = (*post_balance as i64) - (*pre_balance as i64);

        println!("   [{}] {}", i, account);
        println!(
            "       Pre:  {:>15} lamports ({:>12.9} SOL)",
            pre_balance,
            lamports_to_sol(*pre_balance)
        );
        println!(
            "       Post: {:>15} lamports ({:>12.9} SOL)",
            post_balance,
            lamports_to_sol(*post_balance)
        );
        if change != 0 {
            println!(
                "       📊 Change: {:>+10} lamports ({:>+9.9} SOL)",
                change,
                lamports_to_sol(change.abs() as u64) * (if change < 0 { -1.0 } else { 1.0 })
            );
        }
        println!();
    }

    // Look for wallet account (assume first account is the main wallet)
    let wallet_address = &details.transaction.message.account_keys[0];
    println!("🏦 Wallet Analysis:");
    println!("   Wallet address: {}", wallet_address);

    let wallet_pre = meta.pre_balances[0];
    let wallet_post = meta.post_balances[0];
    let wallet_change = (wallet_post as i64) - (wallet_pre as i64);

    println!(
        "   SOL balance change: {:+} lamports ({:+.9} SOL)",
        wallet_change,
        lamports_to_sol(wallet_change.abs() as u64) * (if wallet_change < 0 { -1.0 } else { 1.0 })
    );
    println!();

    // Token balance changes
    println!("🪙 Token Balance Changes:");
    if let Some(pre_token_balances) = &meta.pre_token_balances {
        if let Some(post_token_balances) = &meta.post_token_balances {
            analyze_token_changes(pre_token_balances, post_token_balances, wallet_address);
        }
    } else {
        println!("   No token balance data available");
    }
    println!();

    // Instruction analysis
    println!("📝 Instructions ({} total):", details.transaction.message.instructions.len());
    for (i, instruction) in details.transaction.message.instructions.iter().enumerate() {
        let program_account =
            &details.transaction.message.account_keys[instruction.program_id_index as usize];
        println!("   [{}] Program: {}", i, program_account);

        // Try to identify common program types
        match program_account.as_str() {
            "11111111111111111111111111111111" => println!("        🏗️  System Program"),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => println!("        🪙  Token Program"),
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" =>
                println!("        🏦  Associated Token Program"),
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => println!("        🔄  Jupiter V6"),
            "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => println!("        🔄  Jupiter V4"),
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => println!("        🌊  Whirlpool"),
            "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" => println!("        🔄  Orca"),
            _ => {
                if program_account.starts_with("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHwBASdmAfDEf") {
                    println!("        🔄  Raydium CPMM");
                } else {
                    println!("        ❓  Unknown Program");
                }
            }
        }

        if !instruction.data.is_empty() {
            println!("        📊  Data: {} bytes", instruction.data.len());
        }
        println!();
    }

    // Log messages analysis
    if let Some(log_messages) = &meta.log_messages {
        println!("📋 Transaction Logs ({} messages):", log_messages.len());
        for (i, log) in log_messages.iter().enumerate() {
            println!("   [{}] {}", i, log);
        }
        println!();
    }

    // Try to detect swap details
    println!("🔄 SWAP DETECTION");
    println!("==================");

    // Check if this looks like a token -> SOL swap
    let sol_change = (wallet_change as f64) / 1_000_000_000.0;

    if sol_change > 0.0 {
        println!("✅ SOL RECEIVED: {:.9} SOL", sol_change);
        println!("   This appears to be a SELL transaction (Token → SOL)");

        // Try to find which token was sold
        if let Some(pre_token_balances) = &meta.pre_token_balances {
            if let Some(post_token_balances) = &meta.post_token_balances {
                find_sold_token(
                    pre_token_balances,
                    post_token_balances,
                    wallet_address,
                    sol_change
                );
            }
        }
    } else if sol_change < 0.0 {
        println!("📉 SOL SPENT: {:.9} SOL", sol_change.abs());
        println!("   This appears to be a BUY transaction (SOL → Token)");
    } else {
        println!("➖ No net SOL change detected");
    }

    // Check for ATA account closures
    println!();
    println!("🏪 ATA CLOSURE DETECTION");
    println!("=========================");
    detect_ata_closures(&serde_json::to_value(&meta).unwrap());

    println!();
    println!("✅ Investigation complete!");

    Ok(())
}

fn analyze_token_changes(pre_balances: &[Value], post_balances: &[Value], wallet_address: &str) {
    let mut pre_map: HashMap<String, (u64, u8)> = HashMap::new();
    let mut post_map: HashMap<String, (u64, u8)> = HashMap::new();

    // Build pre-balance map
    for balance in pre_balances {
        if
            let (Some(mint), Some(owner), Some(amount), Some(decimals)) = (
                balance.get("mint").and_then(|v| v.as_str()),
                balance.get("owner").and_then(|v| v.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("decimals"))
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u8),
            )
        {
            if owner == wallet_address {
                pre_map.insert(mint.to_string(), (amount, decimals));
            }
        }
    }

    // Build post-balance map
    for balance in post_balances {
        if
            let (Some(mint), Some(owner), Some(amount), Some(decimals)) = (
                balance.get("mint").and_then(|v| v.as_str()),
                balance.get("owner").and_then(|v| v.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("decimals"))
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u8),
            )
        {
            if owner == wallet_address {
                post_map.insert(mint.to_string(), (amount, decimals));
            }
        }
    }

    // Find all mints that changed
    let mut all_mints: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_mints.extend(pre_map.keys().cloned());
    all_mints.extend(post_map.keys().cloned());

    for mint in all_mints {
        let (pre_amount, decimals) = pre_map.get(&mint).unwrap_or(&(0, 0));
        let default_value = (0, *decimals);
        let (post_amount, _) = post_map.get(&mint).unwrap_or(&default_value);

        if pre_amount != post_amount {
            let change = (*post_amount as i64) - (*pre_amount as i64);
            let ui_change = (change as f64) / (10_f64).powi(*decimals as i32);

            println!("   🪙 Token: {}", mint);
            println!(
                "      Pre:  {} raw ({:.6} UI)",
                pre_amount,
                (*pre_amount as f64) / (10_f64).powi(*decimals as i32)
            );
            println!(
                "      Post: {} raw ({:.6} UI)",
                post_amount,
                (*post_amount as f64) / (10_f64).powi(*decimals as i32)
            );
            println!("      📊 Change: {:+} raw ({:+.6} UI)", change, ui_change);
            println!();
        }
    }
}

fn find_sold_token(
    pre_balances: &[Value],
    post_balances: &[Value],
    wallet_address: &str,
    sol_received: f64
) {
    let mut pre_map: HashMap<String, (u64, u8)> = HashMap::new();
    let mut post_map: HashMap<String, (u64, u8)> = HashMap::new();

    // Build maps (same as above)
    for balance in pre_balances {
        if
            let (Some(mint), Some(owner), Some(amount), Some(decimals)) = (
                balance.get("mint").and_then(|v| v.as_str()),
                balance.get("owner").and_then(|v| v.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("decimals"))
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u8),
            )
        {
            if owner == wallet_address {
                pre_map.insert(mint.to_string(), (amount, decimals));
            }
        }
    }

    for balance in post_balances {
        if
            let (Some(mint), Some(owner), Some(amount), Some(decimals)) = (
                balance.get("mint").and_then(|v| v.as_str()),
                balance.get("owner").and_then(|v| v.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("amount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|v| v.get("decimals"))
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u8),
            )
        {
            if owner == wallet_address {
                post_map.insert(mint.to_string(), (amount, decimals));
            }
        }
    }

    // Find tokens that decreased (were sold)
    for (mint, (pre_amount, decimals)) in &pre_map {
        let default_value = (0, *decimals);
        let (post_amount, _) = post_map.get(mint).unwrap_or(&default_value);

        if post_amount < pre_amount {
            let tokens_sold =
                ((*pre_amount - post_amount) as f64) / (10_f64).powi(*decimals as i32);
            let effective_price = sol_received / tokens_sold;

            println!("🪙 TOKEN SOLD:");
            println!("   Mint: {}", mint);
            println!("   Amount sold: {:.6} tokens", tokens_sold);
            println!("   SOL received: {:.9} SOL", sol_received);
            println!("   🎯 EFFECTIVE PRICE: {:.12} SOL per token", effective_price);
            println!("   📈 Price in scientific: {:.6e} SOL per token", effective_price);
            break;
        }
    }
}

fn detect_ata_closures(meta: &Value) {
    println!("🔍 Checking for ATA account closures...");

    if
        let (Some(pre_balances), Some(post_balances)) = (
            meta.get("preBalances").and_then(|v| v.as_array()),
            meta.get("postBalances").and_then(|v| v.as_array()),
        )
    {
        let mut ata_closures = Vec::new();

        for (i, (pre_val, post_val)) in pre_balances.iter().zip(post_balances.iter()).enumerate() {
            if let (Some(pre_balance), Some(post_balance)) = (pre_val.as_u64(), post_val.as_u64()) {
                if post_balance < pre_balance {
                    let closed_amount = pre_balance - post_balance;

                    // Standard ATA rent is 2,039,280 lamports
                    if closed_amount >= 2_000_000 && closed_amount <= 2_100_000 {
                        ata_closures.push((i, closed_amount));
                    }
                }
            }
        }

        if ata_closures.is_empty() {
            println!("   ❌ No ATA closures detected");
        } else {
            println!("   ✅ {} ATA closure(s) detected:", ata_closures.len());
            for (account_idx, amount) in ata_closures {
                println!(
                    "      Account [{}]: {} lamports ({:.9} SOL) reclaimed",
                    account_idx,
                    amount,
                    lamports_to_sol(amount)
                );
            }
        }
    }
}
