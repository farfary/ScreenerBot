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
    commitment_config::CommitmentConfig,
    message::VersionedMessage,
};
use solana_client::rpc_client::RpcClient;
use bs58;
use base64::{ Engine as _, engine::general_purpose };
use bincode;

#[derive(Debug, Serialize, Deserialize)]
struct QuoteRequest {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    amount: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuoteResponse {
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
    slippage_bps: u16,
    #[serde(rename = "platformFee")]
    platform_fee: Option<Value>,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SwapRequest {
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "quoteResponse")]
    quote_response: QuoteResponse,
    #[serde(rename = "wrapAndUnwrapSol")]
    wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    use_shared_accounts: bool,
    #[serde(rename = "feeAccount")]
    fee_account: Option<String>,
    #[serde(rename = "trackingAccount")]
    tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction")]
    as_legacy_transaction: bool,
    #[serde(rename = "useTokenLedger")]
    use_token_ledger: bool,
    #[serde(rename = "destinationTokenAccount")]
    destination_token_account: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    prioritization_fee_lamports: u64,
    #[serde(rename = "computeUnitLimit")]
    compute_unit_limit: u64,
    #[serde(rename = "setupInstructions")]
    setup_instructions: Option<Vec<Value>>,
    #[serde(rename = "swapInstruction")]
    swap_instruction: Option<Value>,
    #[serde(rename = "cleanupInstruction")]
    cleanup_instruction: Option<Value>,
    #[serde(rename = "addressLookupTableAddresses")]
    address_lookup_table_addresses: Option<Vec<String>>,
}

const JUPITER_API_BASE: &str = "https://quote-api.jup.ag/v6";

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

async fn get_quote(
    input_mint: &str,
    output_mint: &str,
    amount: &str,
    slippage_bps: u16
) -> Result<QuoteResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let slippage_string = slippage_bps.to_string();
    let mut params = HashMap::new();
    params.insert("inputMint", input_mint);
    params.insert("outputMint", output_mint);
    params.insert("amount", amount);
    params.insert("slippageBps", slippage_string.as_str());

    let url = format!("{}/quote", JUPITER_API_BASE);

    let response = client.get(&url).query(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Quote request failed: {}", error_text).into());
    }

    let quote: QuoteResponse = response.json().await?;
    Ok(quote)
}

async fn send_swap_transaction(
    swap_transaction_base64: &str,
    keypair: &Keypair,
    rpc_client: &RpcClient
) -> Result<Signature, Box<dyn std::error::Error>> {
    println!("Decoding transaction from Jupiter...");

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

async fn get_swap_transaction(
    user_public_key: &str,
    quote: QuoteResponse,
    token_name: &str
) -> Result<SwapResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let swap_request = SwapRequest {
        user_public_key: user_public_key.to_string(),
        quote_response: quote,
        wrap_and_unwrap_sol: true,
        use_shared_accounts: if token_name == "BONK" {
            false
        } else {
            true
        },
        fee_account: None,
        tracking_account: None,
        compute_unit_price_micro_lamports: None,
        prioritization_fee_lamports: Some(1000),
        as_legacy_transaction: false,
        use_token_ledger: false,
        destination_token_account: None,
    };

    let url = format!("{}/swap", JUPITER_API_BASE);

    let response = client.post(&url).json(&swap_request).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Swap request failed: {}", error_text).into());
    }

    let swap_response: SwapResponse = response.json().await?;
    Ok(swap_response)
}

async fn perform_swap(
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

    let user_public_key = keypair.pubkey().to_string();
    let slippage_bps = 100; // 1% slippage

    println!(
        "🔄 Starting swap: {} SOL → {}",
        (amount.parse::<u64>().unwrap_or(0) as f64) / 1e9,
        token_name
    );
    println!("💳 Wallet: {}", user_public_key);

    // Step 1: Get quote
    println!("📊 Getting quote from Jupiter...");
    let quote = get_quote(input_mint, output_mint, amount, slippage_bps).await?;

    println!("✅ Quote received:");
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
    println!("   Route: {} step(s)", quote.route_plan.len());

    // Step 2: Get swap transaction
    println!("🔨 Building swap transaction...");
    let swap_response = get_swap_transaction(&user_public_key, quote, token_name).await?;

    println!("✅ Transaction prepared:");
    println!("   Compute Units: {}", swap_response.compute_unit_limit);
    println!("   Priority Fee: {} lamports", swap_response.prioritization_fee_lamports);

    // Step 3: Send the transaction
    println!("📤 Sending transaction...");
    let signature = send_swap_transaction(
        &swap_response.swap_transaction,
        &keypair,
        &rpc_client
    ).await?;

    println!("🎉 Swap completed successfully!");
    println!("   Transaction: https://solscan.io/tx/{}", signature);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Jupiter Swap Bot Starting...\n");

    // Token addresses
    let sol_mint = "So11111111111111111111111111111111111111112"; // SOL (wrapped)
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK

    let amount = "1000000"; // 0.001 SOL in lamports (0.001 * 1e9)

    // Perform swaps
    match perform_swap(sol_mint, usdc_mint, amount, "USDC").await {
        Ok(_) => println!("✅ SOL → USDC swap completed\n"),
        Err(e) => println!("❌ SOL → USDC swap failed: {}\n", e),
    }

    // Small delay between swaps
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    match perform_swap(sol_mint, bonk_mint, amount, "BONK").await {
        Ok(_) => println!("✅ SOL → BONK swap completed"),
        Err(e) => println!("❌ SOL → BONK swap failed: {}", e),
    }

    println!("\n🎉 All swaps completed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_quote() {
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount = "1000000"; // 0.001 SOL
        let slippage_bps = 100;

        let result = get_quote(sol_mint, usdc_mint, amount, slippage_bps).await;
        assert!(result.is_ok(), "Quote request should succeed");

        let quote = result.unwrap();
        assert_eq!(quote.input_mint, sol_mint);
        assert_eq!(quote.output_mint, usdc_mint);
        assert_eq!(quote.in_amount, amount);
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
    fn test_quote_request_serialization() {
        let quote_request = QuoteRequest {
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: "1000000".to_string(),
            slippage_bps: 100,
        };

        let json = serde_json::to_string(&quote_request).unwrap();
        assert!(json.contains("inputMint"));
        assert!(json.contains("outputMint"));
        assert!(json.contains("slippageBps"));
    }
}
