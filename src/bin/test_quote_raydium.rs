use reqwest;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
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
struct RaydiumSwapCompute {
    id: String,
    success: bool,
    version: String,
    msg: Option<String>,
    data: RaydiumSwapData,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumSwapData {
    #[serde(rename = "swapType")]
    swap_type: String,
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "inputAmount")]
    input_amount: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "outputAmount")]
    output_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: u32,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: f64,
    #[serde(rename = "routePlan")]
    route_plan: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumTransactionRequest {
    #[serde(rename = "computeUnitPriceMicroLamports")]
    compute_unit_price_micro_lamports: String,
    #[serde(rename = "swapResponse")]
    swap_response: RaydiumSwapCompute,
    #[serde(rename = "txVersion")]
    tx_version: String,
    wallet: String,
    #[serde(rename = "wrapSol")]
    wrap_sol: bool,
    #[serde(rename = "unwrapSol")]
    unwrap_sol: bool,
    #[serde(rename = "inputAccount")]
    input_account: Option<String>,
    #[serde(rename = "outputAccount")]
    output_account: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumTransactionResponse {
    id: String,
    version: String,
    success: bool,
    data: Vec<RaydiumTransactionData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumTransactionData {
    transaction: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumPriorityFeeResponse {
    id: String,
    success: bool,
    data: RaydiumPriorityFeeData,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumPriorityFeeData {
    default: RaydiumPriorityFeeTiers,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumPriorityFeeTiers {
    vh: u64, // very high
    h: u64, // high
    m: u64, // medium
}

const RAYDIUM_API_BASE: &str = "https://transaction-v1.raydium.io";
const RAYDIUM_BASE_HOST: &str = "https://api-v3.raydium.io";

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

async fn get_raydium_priority_fee() -> Result<u64, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/main/priority-fee", RAYDIUM_BASE_HOST);

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        // If priority fee API fails, use a reasonable default
        return Ok(100000); // 0.1 lamports per compute unit
    }

    let priority_response: RaydiumPriorityFeeResponse = response.json().await?;

    if !priority_response.success {
        return Ok(100000); // Default fallback
    }

    // Use high priority fee
    Ok(priority_response.data.default.h)
}

async fn get_raydium_quote(
    input_mint: &str,
    output_mint: &str,
    amount: &str,
    slippage_bps: u32,
    tx_version: &str
) -> Result<RaydiumSwapCompute, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let url = format!(
        "{}/compute/swap-base-in?inputMint={}&outputMint={}&amount={}&slippageBps={}&txVersion={}",
        RAYDIUM_API_BASE,
        input_mint,
        output_mint,
        amount,
        slippage_bps,
        tx_version
    );

    println!("🔗 Requesting Raydium quote...");
    println!("   URL: {}", url);
    println!("   Token In: {}", input_mint);
    println!("   Token Out: {}", output_mint);
    println!("   Amount: {} lamports", amount);
    println!("   Slippage: {} bps", slippage_bps);
    println!("   TX Version: {}", tx_version);

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Raydium quote request failed: {}", error_text).into());
    }

    let quote_response: RaydiumSwapCompute = response.json().await?;

    if !quote_response.success {
        return Err(
            format!(
                "Raydium API error: {}",
                quote_response.msg.unwrap_or("Unknown error".to_string())
            ).into()
        );
    }

    Ok(quote_response)
}

async fn get_raydium_transaction(
    quote_response: RaydiumSwapCompute,
    wallet_pubkey: &str,
    priority_fee: u64,
    is_input_sol: bool,
    is_output_sol: bool
) -> Result<RaydiumTransactionResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let transaction_request = RaydiumTransactionRequest {
        compute_unit_price_micro_lamports: priority_fee.to_string(),
        swap_response: quote_response,
        tx_version: "V0".to_string(),
        wallet: wallet_pubkey.to_string(),
        wrap_sol: is_input_sol,
        unwrap_sol: is_output_sol,
        input_account: if is_input_sol {
            None
        } else {
            None
        }, // Let Raydium handle ATA
        output_account: if is_output_sol {
            None
        } else {
            None
        }, // Let Raydium handle ATA
    };

    let url = format!("{}/transaction/swap-base-in", RAYDIUM_API_BASE);

    let response = client.post(&url).json(&transaction_request).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Raydium transaction request failed: {}", error_text).into());
    }

    let transaction_response: RaydiumTransactionResponse = response.json().await?;

    if !transaction_response.success {
        return Err("Raydium transaction API returned error".into());
    }

    Ok(transaction_response)
}

async fn send_raydium_transaction(
    transaction_base64: &str,
    keypair: &Keypair,
    rpc_client: &RpcClient
) -> Result<Signature, Box<dyn std::error::Error>> {
    println!("Decoding Raydium transaction...");

    // Decode the base64 transaction
    let transaction_bytes = general_purpose::STANDARD.decode(transaction_base64)?;

    // Deserialize into VersionedTransaction (Raydium uses V0 transactions)
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

async fn perform_raydium_swap(
    input_mint: &str,
    output_mint: &str,
    amount: &str,
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

    let wallet_pubkey = keypair.pubkey().to_string();
    let slippage_bps = 100; // 1% slippage
    let tx_version = "V0";

    // Check if input/output is SOL
    let sol_mint = "So11111111111111111111111111111111111111112";
    let is_input_sol = input_mint == sol_mint;
    let is_output_sol = output_mint == sol_mint;

    println!(
        "🔄 Starting Raydium swap: {} SOL → {}",
        (amount.parse::<u64>().unwrap_or(0) as f64) / 1e9,
        token_name
    );
    println!("💳 Wallet: {}", wallet_pubkey);

    // Step 1: Get priority fee
    println!("💰 Getting priority fee...");
    let priority_fee = get_raydium_priority_fee().await?;
    println!("   Priority Fee: {} micro-lamports", priority_fee);

    // Step 2: Get quote from Raydium
    println!("📊 Getting quote from Raydium...");
    let quote_response = get_raydium_quote(
        input_mint,
        output_mint,
        amount,
        slippage_bps,
        tx_version
    ).await?;

    let quote_data = &quote_response.data;

    println!("✅ Raydium quote received:");
    println!(
        "   Input: {} SOL",
        (quote_data.input_amount.parse::<u64>().unwrap_or(0) as f64) / 1e9
    );
    println!(
        "   Output: {} {}",
        if token_name == "USDC" {
            (quote_data.output_amount.parse::<u64>().unwrap_or(0) as f64) / 1e6
        } else {
            (quote_data.output_amount.parse::<u64>().unwrap_or(0) as f64) / 1e5 // BONK has 5 decimals
        },
        token_name
    );
    println!("   Price Impact: {}%", quote_data.price_impact_pct);
    println!("   Route Steps: {}", quote_data.route_plan.len());
    println!("   Swap Type: {}", quote_data.swap_type);

    // Step 3: Get transaction
    println!("🔨 Building Raydium transaction...");
    let transaction_response = get_raydium_transaction(
        quote_response,
        &wallet_pubkey,
        priority_fee,
        is_input_sol,
        is_output_sol
    ).await?;

    println!("✅ Transaction prepared:");
    println!("   Total transactions: {}", transaction_response.data.len());

    // Step 4: Send transactions (Raydium may return multiple transactions)
    println!("📤 Sending Raydium transaction(s)...");
    for (idx, tx_data) in transaction_response.data.iter().enumerate() {
        println!("   Sending transaction {} of {}...", idx + 1, transaction_response.data.len());
        let signature = send_raydium_transaction(
            &tx_data.transaction,
            &keypair,
            &rpc_client
        ).await?;
        println!("   Transaction {}: https://solscan.io/tx/{}", idx + 1, signature);

        // Small delay between transactions if multiple
        if idx < transaction_response.data.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    println!("🎉 Raydium swap completed successfully!");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Raydium Swap Bot Starting...\n");

    // Token addresses
    let sol_mint = "So11111111111111111111111111111111111111112"; // SOL (wrapped)
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK

    let amount = "1000000"; // 0.001 SOL in lamports (0.001 * 1e9)

    // Perform swaps
    match perform_raydium_swap(sol_mint, usdc_mint, amount, "USDC").await {
        Ok(_) => println!("✅ SOL → USDC swap completed\n"),
        Err(e) => println!("❌ SOL → USDC swap failed: {}\n", e),
    }

    // Small delay between swaps
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    match perform_raydium_swap(sol_mint, bonk_mint, amount, "BONK").await {
        Ok(_) => println!("✅ SOL → BONK swap completed"),
        Err(e) => println!("❌ SOL → BONK swap failed: {}", e),
    }

    println!("\n🎉 All Raydium swaps completed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_raydium_priority_fee() {
        let result = get_raydium_priority_fee().await;
        assert!(result.is_ok(), "Priority fee request should succeed");

        let fee = result.unwrap();
        assert!(fee > 0, "Priority fee should be greater than 0");
    }

    #[tokio::test]
    async fn test_get_raydium_quote() {
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount = "1000000"; // 0.001 SOL
        let slippage_bps = 100;
        let tx_version = "V0";

        let result = get_raydium_quote(sol_mint, usdc_mint, amount, slippage_bps, tx_version).await;

        if let Ok(response) = result {
            assert!(response.success, "Raydium API should return success");
            assert_eq!(response.data.input_mint, sol_mint);
            assert_eq!(response.data.output_mint, usdc_mint);
            assert_eq!(response.data.input_amount, amount);
        } else {
            // Test might fail if Raydium API is down or rate limited
            println!("Raydium API test skipped - service may be unavailable");
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
    fn test_raydium_transaction_request_creation() {
        let quote_response = RaydiumSwapCompute {
            id: "test".to_string(),
            success: true,
            version: "1".to_string(),
            msg: None,
            data: RaydiumSwapData {
                swap_type: "BaseIn".to_string(),
                input_mint: "So11111111111111111111111111111111111111112".to_string(),
                input_amount: "1000000".to_string(),
                output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                output_amount: "164000".to_string(),
                other_amount_threshold: "162000".to_string(),
                slippage_bps: 100,
                price_impact_pct: 0.01,
                route_plan: vec![],
            },
        };

        let transaction_request = RaydiumTransactionRequest {
            compute_unit_price_micro_lamports: "100000".to_string(),
            swap_response: quote_response,
            tx_version: "V0".to_string(),
            wallet: "B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59".to_string(),
            wrap_sol: true,
            unwrap_sol: false,
            input_account: None,
            output_account: None,
        };

        assert_eq!(transaction_request.tx_version, "V0");
        assert_eq!(transaction_request.wrap_sol, true);
        assert_eq!(transaction_request.unwrap_sol, false);
    }

    #[test]
    fn test_raydium_response_parsing() {
        let json_response =
            r#"
        {
            "id": "test-123",
            "success": true,
            "version": "1.0",
            "data": {
                "swapType": "BaseIn",
                "inputMint": "So11111111111111111111111111111111111111112",
                "inputAmount": "1000000",
                "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                "outputAmount": "164000",
                "otherAmountThreshold": "162000",
                "slippageBps": 100,
                "priceImpactPct": 0.01,
                "routePlan": []
            }
        }
        "#;

        let result: Result<RaydiumSwapCompute, _> = serde_json::from_str(json_response);
        assert!(result.is_ok(), "Should parse Raydium response successfully");

        let response = result.unwrap();
        assert_eq!(response.success, true);
        assert_eq!(response.data.input_mint, "So11111111111111111111111111111111111111112");
        assert_eq!(response.data.swap_type, "BaseIn");
    }
}
