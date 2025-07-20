use screenerbot::wallet::{ get_wallet_address, get_token_balance };
use screenerbot::global::read_configs;
use screenerbot::logger::{ log, LogTag };
use serde::Deserialize;
use reqwest;
use std::error::Error;

/// Specific token to check
const TOKEN_MINT: &str = "E3kRwpjjt75R5KrXDHPkgZg4uskGR5BQnyc5wCRrbonk";

/// Transaction signature response from Solana RPC
#[derive(Debug, Deserialize)]
struct SignatureResponse {
    result: Option<Vec<SignatureInfo>>,
    error: Option<serde_json::Value>,
}

/// Individual signature information
#[derive(Debug, Deserialize, Clone)]
struct SignatureInfo {
    signature: String,
    slot: u64,
    #[serde(rename = "blockTime")]
    block_time: Option<u64>,
    err: Option<serde_json::Value>,
    memo: Option<String>,
    confirmationStatus: Option<String>,
}

/// Transaction details response from Solana RPC
#[derive(Debug, Deserialize)]
struct TransactionResponse {
    result: Option<TransactionResult>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TransactionResult {
    transaction: Transaction,
    meta: Option<TransactionMeta>,
    #[serde(rename = "blockTime")]
    block_time: Option<u64>,
    slot: u64,
}

#[derive(Debug, Deserialize)]
struct Transaction {
    message: TransactionMessage,
    signatures: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TransactionMessage {
    #[serde(rename = "accountKeys")]
    account_keys: Vec<String>,
    instructions: Vec<TransactionInstruction>,
}

#[derive(Debug, Deserialize)]
struct TransactionInstruction {
    #[serde(rename = "programId")]
    program_id: Option<String>,
    #[serde(rename = "programIdIndex")]
    program_id_index: Option<u8>,
    accounts: Vec<u8>,
    data: String,
    #[serde(rename = "stackHeight")]
    stack_height: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TransactionMeta {
    err: Option<serde_json::Value>,
    fee: u64,
    #[serde(rename = "preBalances")]
    pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pre_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "postTokenBalances")]
    post_token_balances: Option<Vec<TokenBalance>>,
}

#[derive(Debug, Deserialize)]
struct TokenBalance {
    #[serde(rename = "accountIndex")]
    account_index: u8,
    mint: String,
    #[serde(rename = "uiTokenAmount")]
    ui_token_amount: TokenAmount,
    owner: Option<String>,
    #[serde(rename = "programId")]
    program_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenAmount {
    amount: String,
    decimals: u8,
    #[serde(rename = "uiAmount")]
    ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    ui_amount_string: Option<String>,
}

/// Get recent transaction signatures for a wallet address
async fn get_recent_signatures(
    client: &reqwest::Client,
    wallet_address: &str,
    rpc_url: &str,
    limit: usize
) -> Result<Vec<SignatureInfo>, Box<dyn Error>> {
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": [
            wallet_address,
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]
    });

    log(
        LogTag::System,
        "INFO",
        &format!("Fetching recent {} signatures for wallet: {}", limit, wallet_address)
    );

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get signatures: {}", response.status()).into());
    }

    let response_text = response.text().await?;

    // Parse the JSON response manually for better error handling
    let rpc_response: SignatureResponse = serde_json
        ::from_str(&response_text)
        .map_err(|e|
            format!("Failed to parse response: {} - Response text: {}", e, response_text)
        )?;

    // Check for RPC errors
    if let Some(error) = rpc_response.error {
        return Err(format!("RPC error: {:?}", error).into());
    }

    match rpc_response.result {
        Some(signatures) => Ok(signatures),
        None => Err("No result in response".into()),
    }
}

/// Get detailed transaction information
async fn get_transaction_details(
    client: &reqwest::Client,
    signature: &str,
    rpc_url: &str
) -> Result<Option<TransactionResult>, Box<dyn Error>> {
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0,
                "commitment": "confirmed"
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

    let response_text = response.text().await?;

    // Parse the JSON response manually for better error handling
    let rpc_response: TransactionResponse = serde_json
        ::from_str(&response_text)
        .map_err(|e|
            format!(
                "Failed to parse transaction response: {} - Response text: {}",
                e,
                response_text
            )
        )?;

    // Check for RPC errors
    if let Some(error) = rpc_response.error {
        return Err(format!("RPC error getting transaction: {:?}", error).into());
    }

    Ok(rpc_response.result)
}

/// Check if transaction involves the specific token
fn transaction_involves_token(transaction: &TransactionResult, token_mint: &str) -> bool {
    // Check if token mint appears in account keys
    if transaction.transaction.message.account_keys.contains(&token_mint.to_string()) {
        return true;
    }

    // Check token balances
    if let Some(ref meta) = transaction.meta {
        // Check pre-token balances
        if let Some(ref pre_balances) = meta.pre_token_balances {
            for balance in pre_balances {
                if balance.mint == token_mint {
                    return true;
                }
            }
        }

        // Check post-token balances
        if let Some(ref post_balances) = meta.post_token_balances {
            for balance in post_balances {
                if balance.mint == token_mint {
                    return true;
                }
            }
        }
    }

    false
}

/// Analyze token balance changes in a transaction
fn analyze_token_changes(
    transaction: &TransactionResult,
    token_mint: &str,
    wallet_address: &str
) -> Option<(f64, f64)> {
    if let Some(ref meta) = transaction.meta {
        let mut pre_amount = 0.0;
        let mut post_amount = 0.0;

        // Find pre-balance for our token
        if let Some(ref pre_balances) = meta.pre_token_balances {
            for balance in pre_balances {
                if balance.mint == token_mint {
                    if let Some(owner) = &balance.owner {
                        if owner == wallet_address {
                            pre_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                            break;
                        }
                    }
                }
            }
        }

        // Find post-balance for our token
        if let Some(ref post_balances) = meta.post_token_balances {
            for balance in post_balances {
                if balance.mint == token_mint {
                    if let Some(owner) = &balance.owner {
                        if owner == wallet_address {
                            post_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                            break;
                        }
                    }
                }
            }
        }

        if pre_amount != 0.0 || post_amount != 0.0 {
            return Some((pre_amount, post_amount));
        }
    }

    None
}

/// Format timestamp for display
fn format_timestamp(timestamp: Option<u64>) -> String {
    match timestamp {
        Some(ts) => {
            use chrono::{ Utc, TimeZone };
            let dt = Utc.timestamp_opt(ts as i64, 0).single();
            match dt {
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                None => "Invalid timestamp".to_string(),
            }
        }
        None => "Unknown time".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔍 Checking Last Transactions for Token: {}", TOKEN_MINT);
    println!("{}", "=".repeat(80));

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => {
            println!("👛 Wallet Address: {}", addr);
            addr
        }
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return Ok(());
        }
    };

    // Check current token balance
    println!("\n💰 Current Token Balance:");
    match get_token_balance(&wallet_address, TOKEN_MINT).await {
        Ok(balance) => {
            println!("   Raw Balance: {} (raw units)", balance);
            // Note: We'd need token decimals to show UI amount
            println!("   Token Mint: {}", TOKEN_MINT);
        }
        Err(e) => println!("   ❌ Failed to get token balance: {}", e),
    }

    // Read configs to get RPC URL
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            println!("❌ Failed to read configs: {}", e);
            return Ok(());
        }
    };

    let client = reqwest::Client::new();
    let rpc_url = &configs.rpc_url;

    println!("\n🔄 Fetching recent transactions...");

    // Get recent transaction signatures (last 50)
    let signatures = match get_recent_signatures(&client, &wallet_address, rpc_url, 50).await {
        Ok(sigs) => {
            println!("   ✅ Found {} recent signatures", sigs.len());
            sigs
        }
        Err(e) => {
            println!("   ❌ Failed to get signatures: {}", e);
            return Ok(());
        }
    };

    println!("\n🔍 Analyzing transactions for token involvement...");
    let mut token_transactions = Vec::new();
    let mut checked_count = 0;

    for sig_info in &signatures {
        checked_count += 1;

        // Show progress every 10 transactions
        if checked_count % 10 == 0 {
            println!("   Checked {}/{} transactions...", checked_count, signatures.len());
        }

        // Add delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        match get_transaction_details(&client, &sig_info.signature, rpc_url).await {
            Ok(Some(transaction)) => {
                if transaction_involves_token(&transaction, TOKEN_MINT) {
                    println!("   🎯 Found token transaction: {}", sig_info.signature);
                    token_transactions.push((sig_info.clone(), transaction));
                }
            }
            Ok(None) => {
                // Transaction not found or failed
                continue;
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Rate limited, waiting longer... {}", sig_info.signature)
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                } else {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Failed to get transaction {}: {}", sig_info.signature, e)
                    );
                }
                continue;
            }
        }
    }

    println!("\n📊 Transaction Analysis Results:");
    println!("   Total signatures checked: {}", checked_count);
    println!("   Token-related transactions found: {}", token_transactions.len());

    if token_transactions.is_empty() {
        println!(
            "\n🤷 No transactions found involving this token in the last {} transactions.",
            signatures.len()
        );
        println!("   This could mean:");
        println!("   • No recent activity with this token");
        println!("   • Token transactions are older than the last {} signatures", signatures.len());
        println!("   • Token might not exist or be inactive");
    } else {
        println!("\n📋 Token Transaction Details:");
        println!("{}", "=".repeat(120));

        for (i, (sig_info, transaction)) in token_transactions.iter().enumerate() {
            println!("\n🔸 Transaction #{} ({})", i + 1, sig_info.signature);
            println!("   📅 Time: {}", format_timestamp(sig_info.block_time));
            println!("   🎰 Slot: {}", sig_info.slot);
            println!("   ✅ Status: {}", if sig_info.err.is_none() { "Success" } else { "Failed" });

            if let Some(ref err) = sig_info.err {
                println!("   ❌ Error: {:?}", err);
            }

            // Analyze token balance changes
            if
                let Some((pre_amount, post_amount)) = analyze_token_changes(
                    transaction,
                    TOKEN_MINT,
                    &wallet_address
                )
            {
                let change = post_amount - pre_amount;
                println!("   💰 Token Balance Change:");
                println!("      Before: {:.6}", pre_amount);
                println!("      After:  {:.6}", post_amount);
                if change > 0.0 {
                    println!("      Change: +{:.6} (BUY)", change);
                } else if change < 0.0 {
                    println!("      Change: {:.6} (SELL)", change);
                } else {
                    println!("      Change: No change");
                }
            } else {
                println!("   💰 Token Balance Change: Could not determine");
            }

            // Show fee information
            if let Some(ref meta) = transaction.meta {
                println!("   💸 Transaction Fee: {:.9} SOL", (meta.fee as f64) / 1_000_000_000.0);
            }

            println!("   🔗 Explorer: https://solscan.io/tx/{}", sig_info.signature);
        }
    }

    println!("\n✅ Transaction analysis completed!");

    Ok(())
}
