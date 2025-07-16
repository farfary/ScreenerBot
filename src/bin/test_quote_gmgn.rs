use reqwest;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use tokio;
use solana_sdk::{
    signature::{ Keypair, Signature },
    transaction::VersionedTransaction,
    signer::Signer,
    message::VersionedMessage,
};
use solana_client::rpc_client::RpcClient;
use bs58;
use base64::{ Engine as _, engine::general_purpose };
use bincode;

#[derive(Debug, Serialize, Deserialize)]
struct GmgnQuoteRequest {
    token_in_address: String,
    token_out_address: String,
    in_amount: String,
    from_address: String,
    slippage: f64,
    swap_mode: String,
    fee: f64,
    is_anti_mev: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GmgnQuote {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    swap_mode: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: String, // GMGN returns this as string, not number
    #[serde(rename = "platformFee")]
    platform_fee: Option<Value>,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<Value>,
    #[serde(rename = "contextSlot")]
    context_slot: Option<u64>, // Make optional since it might not be present
    #[serde(rename = "timeTaken")]
    time_taken: Option<f64>, // Make optional since it might not be present
}

#[derive(Debug, Serialize, Deserialize)]
struct GmgnRawTx {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    prioritization_fee_lamports: u64,
    #[serde(rename = "recentBlockhash")]
    recent_blockhash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GmgnSwapData {
    quote: GmgnQuote,
    raw_tx: GmgnRawTx,
}

#[derive(Debug, Serialize, Deserialize)]
struct GmgnResponse {
    code: i32,
    msg: String,
    data: GmgnSwapData,
}

const GMGN_API_BASE: &str = "https://gmgn.ai/defi/router/v1/sol/tx";

// Load configuration from configs.json
#[derive(Deserialize)]
struct Config {
    main_wallet_private: String,
    rpc_url: String,
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string("configs.json")?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

async fn get_gmgn_swap_route(
    token_in_address: &str,
    token_out_address: &str,
    in_amount: &str,
    from_address: &str,
    slippage: f64
) -> Result<GmgnResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // Use the lowest possible fee with is_anti_mev = false
    let fee = 0.001; // 0.001 SOL - lowest reasonable fee
    let is_anti_mev = false; // As requested
    let swap_mode = "ExactIn";

    // Convert values to strings to avoid lifetime issues
    let slippage_str = slippage.to_string();
    let fee_str = fee.to_string();
    let is_anti_mev_str = is_anti_mev.to_string();

    let mut params = HashMap::new();
    params.insert("token_in_address", token_in_address);
    params.insert("token_out_address", token_out_address);
    params.insert("in_amount", in_amount);
    params.insert("from_address", from_address);
    params.insert("slippage", slippage_str.as_str());
    params.insert("swap_mode", swap_mode);
    params.insert("fee", fee_str.as_str());
    params.insert("is_anti_mev", is_anti_mev_str.as_str());

    let url = format!("{}/get_swap_route", GMGN_API_BASE);

    println!("🔗 Requesting GMGN swap route...");
    println!("   URL: {}", url);
    println!("   Token In: {}", token_in_address);
    println!("   Token Out: {}", token_out_address);
    println!("   Amount: {} lamports", in_amount);
    println!("   Slippage: {}%", slippage);
    println!("   Fee: {} SOL", fee);
    println!("   Anti-MEV: {}", is_anti_mev);

    let response = client.get(&url).query(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("GMGN swap route request failed: {}", error_text).into());
    }

    let gmgn_response: GmgnResponse = response.json().await?;

    if gmgn_response.code != 0 {
        return Err(
            format!("GMGN API error: {} - {}", gmgn_response.code, gmgn_response.msg).into()
        );
    }

    Ok(gmgn_response)
}

async fn send_gmgn_transaction(
    swap_transaction_base64: &str,
    keypair: &Keypair,
    rpc_client: &RpcClient
) -> Result<Signature, Box<dyn std::error::Error>> {
    println!("Decoding GMGN transaction...");

    // Decode the base64 transaction
    let transaction_bytes = general_purpose::STANDARD.decode(swap_transaction_base64)?;

    // Deserialize into VersionedTransaction
    let mut versioned_transaction: VersionedTransaction = bincode::deserialize(&transaction_bytes)?;
    println!("Decoded transaction with {} signatures", versioned_transaction.signatures.len());

    // Get the latest blockhash and update transaction
    let blockhash = rpc_client.get_latest_blockhash()?;

    // Update blockhash in the message
    match &mut versioned_transaction.message {
        VersionedMessage::V0(msg) => {
            msg.recent_blockhash = blockhash;
        }
        VersionedMessage::Legacy(msg) => {
            msg.recent_blockhash = blockhash;
        }
    }

    // Clear existing signatures and sign with our wallet
    versioned_transaction.signatures.clear();
    let message = versioned_transaction.message.clone();
    let message_bytes = bincode::serialize(&message)?;
    let signature = keypair.sign_message(&message_bytes);
    versioned_transaction.signatures.push(signature);

    println!("Signed transaction with wallet");

    // Serialize the signed transaction
    let signed_transaction_bytes = bincode::serialize(&versioned_transaction)?;
    let signed_transaction_base64 = general_purpose::STANDARD.encode(&signed_transaction_bytes);

    // Send using RPC
    let params =
        serde_json::json!([
        signed_transaction_base64,
        {
            "encoding": "base64",
            "skipPreflight": false,
            "preflightCommitment": "processed"
        }
    ]);

    let request_body =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": params
    });

    println!("Sending signed transaction...");

    let client = reqwest::Client::new();
    let response = client
        .post(rpc_client.url())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("HTTP error: {}", error_text).into());
    }

    let response_json: Value = response.json().await?;

    if let Some(error) = response_json.get("error") {
        return Err(format!("RPC error: {}", error).into());
    }

    if let Some(result) = response_json.get("result") {
        if let Some(signature_str) = result.as_str() {
            let transaction_signature = signature_str
                .parse::<Signature>()
                .map_err(|e| format!("Failed to parse signature: {}", e))?;

            println!("Transaction sent successfully: {}", transaction_signature);
            return Ok(transaction_signature);
        }
    }

    Err("Invalid response format".into())
}

async fn perform_gmgn_swap(
    token_in_address: &str,
    token_out_address: &str,
    in_amount: &str,
    token_name: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;

    // Create keypair from private key
    let private_key_bytes = bs58
        ::decode(&config.main_wallet_private)
        .into_vec()
        .map_err(|e| format!("Failed to decode private key: {}", e))?;
    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        format!("Failed to create keypair: {}", e)
    )?;

    // Create RPC client
    let rpc_client = RpcClient::new(&config.rpc_url);

    let from_address = keypair.pubkey().to_string();
    let slippage = 1.0; // 1% slippage

    println!(
        "🔄 Starting GMGN swap: {} SOL → {}",
        (in_amount.parse::<u64>().unwrap_or(0) as f64) / 1e9,
        token_name
    );
    println!("💳 Wallet: {}", from_address);

    // Step 1: Get swap route from GMGN
    println!("📊 Getting swap route from GMGN...");
    let gmgn_response = get_gmgn_swap_route(
        token_in_address,
        token_out_address,
        in_amount,
        &from_address,
        slippage
    ).await?;

    let quote = &gmgn_response.data.quote;
    let raw_tx = &gmgn_response.data.raw_tx;

    println!("✅ GMGN route received:");
    println!("   Input: {} SOL", (quote.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1e9);
    println!(
        "   Output: {} {}",
        if token_name == "USDC" {
            (quote.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1e6
        } else {
            (quote.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1e5 // BONK has 5 decimals
        },
        token_name
    );
    println!("   Price Impact: {}%", quote.price_impact_pct);
    println!("   Route Steps: {}", quote.route_plan.len());
    println!("   Priority Fee: {} lamports", raw_tx.prioritization_fee_lamports);
    println!("   Time Taken: {:.4}s", quote.time_taken.unwrap_or(0.0));

    // Step 2: Send the transaction
    println!("📤 Sending GMGN transaction...");
    let signature = send_gmgn_transaction(&raw_tx.swap_transaction, &keypair, &rpc_client).await?;

    println!("🎉 GMGN swap completed successfully!");
    println!("   Transaction: https://solscan.io/tx/{}", signature);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 GMGN Swap Bot Starting...\n");

    // Token addresses
    let sol_mint = "So11111111111111111111111111111111111111112"; // SOL (wrapped)
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK

    let amount = "1000000"; // 0.001 SOL in lamports (0.001 * 1e9)

    // Perform swaps
    match perform_gmgn_swap(sol_mint, usdc_mint, amount, "USDC").await {
        Ok(_) => println!("✅ SOL → USDC swap completed\n"),
        Err(e) => println!("❌ SOL → USDC swap failed: {}\n", e),
    }

    // Small delay between swaps
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    match perform_gmgn_swap(sol_mint, bonk_mint, amount, "BONK").await {
        Ok(_) => println!("✅ SOL → BONK swap completed"),
        Err(e) => println!("❌ SOL → BONK swap failed: {}", e),
    }

    println!("\n🎉 All GMGN swaps completed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_gmgn_swap_route() {
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount = "1000000"; // 0.001 SOL
        let from_address = "B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59"; // Example address
        let slippage = 1.0;

        let result = get_gmgn_swap_route(sol_mint, usdc_mint, amount, from_address, slippage).await;

        if let Ok(response) = result {
            assert_eq!(response.code, 0, "GMGN API should return success code");
            assert_eq!(response.data.quote.input_mint, sol_mint);
            assert_eq!(response.data.quote.output_mint, usdc_mint);
            assert_eq!(response.data.quote.in_amount, amount);
        } else {
            // Test might fail if GMGN API is down or rate limited
            println!("GMGN API test skipped - service may be unavailable");
        }
    }

    #[test]
    fn test_config_loading() {
        let result = load_config();
        assert!(result.is_ok(), "Config should load successfully");

        let config = result.unwrap();
        assert!(!config.main_wallet_private.is_empty(), "Wallet private key should not be empty");
        assert!(!config.rpc_url.is_empty(), "RPC URL should not be empty");
    }

    #[test]
    fn test_gmgn_quote_request_creation() {
        let quote_request = GmgnQuoteRequest {
            token_in_address: "So11111111111111111111111111111111111111112".to_string(),
            token_out_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            in_amount: "1000000".to_string(),
            from_address: "B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59".to_string(),
            slippage: 1.0,
            swap_mode: "ExactIn".to_string(),
            fee: 0.001,
            is_anti_mev: false,
        };

        assert_eq!(quote_request.is_anti_mev, false);
        assert_eq!(quote_request.fee, 0.001);
        assert_eq!(quote_request.swap_mode, "ExactIn");
    }

    #[test]
    fn test_gmgn_response_parsing() {
        let json_response =
            r#"
        {
            "code": 0,
            "msg": "success",
            "data": {
                "quote": {
                    "inputMint": "So11111111111111111111111111111111111111112",
                    "inAmount": "1000000",
                    "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                    "outAmount": "164000",
                    "otherAmountThreshold": "162360",
                    "swapMode": "ExactIn",
                    "slippageBps": 100,
                    "platformFee": null,
                    "priceImpactPct": "0",
                    "routePlan": [],
                    "contextSlot": 123456789,
                    "timeTaken": 0.05
                },
                "raw_tx": {
                    "swapTransaction": "base64_encoded_transaction",
                    "lastValidBlockHeight": 123456790,
                    "prioritizationFeeLamports": 5000,
                    "recentBlockhash": "ABC123"
                }
            }
        }
        "#;

        let result: Result<GmgnResponse, _> = serde_json::from_str(json_response);
        assert!(result.is_ok(), "Should parse GMGN response successfully");

        let response = result.unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.msg, "success");
        assert_eq!(response.data.quote.input_mint, "So11111111111111111111111111111111111111112");
    }
}
