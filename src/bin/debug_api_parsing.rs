/*
 * Debug API Parsing - Test actual TokenPair deserialization
 */

use anyhow::{ Context, Result };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::time::Duration;

// Simplified version of TokenPair to test parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestTokenPair {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "dexId")]
    pub dex_id: String,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    #[serde(rename = "baseToken")]
    pub base_token: TestToken,
    #[serde(rename = "quoteToken")]
    pub quote_token: TestToken,
    #[serde(rename = "priceUsd")]
    pub price_usd: String,
    pub liquidity: TestLiquidityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestLiquidityMetrics {
    pub usd: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔍 Testing TokenPair Deserialization");
    println!("====================================\n");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenerBot/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    // Test tokens that are failing
    let test_tokens = vec![
        "CkDU2HSnsrcPX6V1PiHPYUd9v3nkxjkBEV4EYfp8cuCh", // test token
        "HFDHRhswmYfECmzx4SLJgpLH6RLafM8fuzKg8hVUpump", // RAR
        "2sFykFB5PDm8iGPg4cUDuPh153YjoiqYy5US9KfbnMCi" // GOODRUNNER
    ];

    for token in test_tokens {
        println!("\n🧪 Testing token: {}", token);
        println!("{}", "-".repeat(60));

        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", token);

        match client.get(&url).send().await {
            Ok(response) => {
                let status = response.status();
                println!("📊 Status: {}", status);

                if status.is_success() {
                    match response.text().await {
                        Ok(raw_text) => {
                            println!("📄 Raw response length: {} chars", raw_text.len());

                            // Try to parse as our simplified TokenPair structure
                            match serde_json::from_str::<Vec<TestTokenPair>>(&raw_text) {
                                Ok(pairs) => {
                                    println!("✅ Successfully parsed {} pairs", pairs.len());
                                    for (i, pair) in pairs.iter().enumerate() {
                                        println!(
                                            "  Pair {}: {} on {} (${:.8})",
                                            i + 1,
                                            pair.pair_address,
                                            pair.dex_id,
                                            pair.price_usd.parse::<f64>().unwrap_or(0.0)
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Failed to parse as TokenPair array: {}", e);

                                    // Try to see what's wrong by checking the structure
                                    if
                                        let Ok(json_value) =
                                            serde_json::from_str::<serde_json::Value>(&raw_text)
                                    {
                                        match &json_value {
                                            serde_json::Value::Array(arr) => {
                                                println!("  📋 Array with {} elements", arr.len());
                                                if !arr.is_empty() {
                                                    println!(
                                                        "  🔍 First element type: {:?}",
                                                        arr[0]
                                                    );
                                                }
                                            }
                                            _ => {
                                                println!("  📋 Not an array: {:?}", json_value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to get response text: {}", e);
                        }
                    }
                } else {
                    println!("❌ HTTP error: {}", status);
                }
            }
            Err(e) => {
                println!("❌ Request failed: {}", e);
            }
        }
    }

    println!("\n🔍 Testing with the full TokenPair import...");

    // Now test with the actual TokenPair struct from our codebase
    use screenerbot::pairs::types::TokenPair;

    let test_token = "CkDU2HSnsrcPX6V1PiHPYUd9v3nkxjkBEV4EYfp8cuCh";
    let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", test_token);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.text().await {
                    Ok(raw_text) => {
                        match serde_json::from_str::<Vec<TokenPair>>(&raw_text) {
                            Ok(pairs) => {
                                println!(
                                    "✅ Full TokenPair parsing successful! {} pairs",
                                    pairs.len()
                                );
                            }
                            Err(e) => {
                                println!("❌ Full TokenPair parsing failed: {}", e);

                                // Try to find which field is causing issues
                                if
                                    let Ok(json_value) = serde_json::from_str::<serde_json::Value>(
                                        &raw_text
                                    )
                                {
                                    if let serde_json::Value::Array(arr) = &json_value {
                                        if !arr.is_empty() {
                                            if let serde_json::Value::Object(obj) = &arr[0] {
                                                println!("  Available fields in first pair:");
                                                for key in obj.keys() {
                                                    println!("    - {}", key);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to get response text: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Request failed: {}", e);
        }
    }

    Ok(())
}
