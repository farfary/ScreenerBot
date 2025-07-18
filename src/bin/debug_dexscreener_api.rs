use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio;

const BASE_URL: &str = "https://api.dexscreener.com";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔍 DexScreener API Debug Tool");
    println!("════════════════════════════════");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenerBot/1.0")
        .build()?;

    // Test tokens - using popular Solana tokens
    let test_tokens = vec![
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", // Jupiter
        "So11111111111111111111111111111111111111112", // Wrapped SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" // USDT
    ];

    for token in test_tokens {
        println!("\n🧪 Testing token: {}", token);
        println!("{}", "─".repeat(50));

        test_token_endpoint(&client, token).await?;

        // Add delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Test with multiple tokens (comma-separated)
    println!("\n🧪 Testing multiple tokens");
    println!("{}", "─".repeat(50));
    let multiple_tokens =
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN,So11111111111111111111111111111111111111112";
    test_token_endpoint(&client, multiple_tokens).await?;

    // Test invalid token
    println!("\n🧪 Testing invalid token");
    println!("{}", "─".repeat(50));
    test_token_endpoint(&client, "invalid_token_address").await?;

    Ok(())
}

async fn test_token_endpoint(client: &Client, token_address: &str) -> Result<()> {
    let url = format!("{}/tokens/v1/solana/{}", BASE_URL, token_address);

    println!("📡 Request URL: {}", url);

    let response = client.get(&url).send().await?;
    let status = response.status();

    println!("📊 Response Status: {}", status);

    // Get raw response text
    let response_text = response.text().await?;

    println!("📝 Response Length: {} bytes", response_text.len());

    if response_text.is_empty() {
        println!("⚠️  Empty response!");
        return Ok(());
    }

    // Show first 500 characters of response
    let preview = if response_text.len() > 500 {
        format!("{}... (truncated)", &response_text[..500])
    } else {
        response_text.clone()
    };

    println!("📄 Response Preview:");
    println!("{}", preview);

    // Try to parse as JSON
    match serde_json::from_str::<Value>(&response_text) {
        Ok(json_value) => {
            println!("✅ Valid JSON structure");
            analyze_json_structure(&json_value);

            // Try to parse as our expected structure
            match serde_json::from_str::<Vec<screenerbot::pairs::types::TokenPair>>(&response_text) {
                Ok(pairs) => {
                    println!("✅ Successfully parsed as Vec<TokenPair>");
                    println!("📊 Found {} pairs", pairs.len());

                    if !pairs.is_empty() {
                        println!("🔍 First pair details:");
                        let first_pair = &pairs[0];
                        println!("   Chain ID: {}", first_pair.chain_id);
                        println!("   DEX ID: {}", first_pair.dex_id);
                        println!("   Pair Address: {}", first_pair.pair_address);
                        println!(
                            "   Base Token: {} ({})",
                            first_pair.base_token.symbol,
                            first_pair.base_token.address
                        );
                        println!(
                            "   Quote Token: {} ({})",
                            first_pair.quote_token.symbol,
                            first_pair.quote_token.address
                        );
                        println!("   Price USD: {}", first_pair.price_usd);

                        if let Some(liquidity) = &first_pair.liquidity {
                            println!("   Liquidity USD: ${:.2}", liquidity.usd);
                        } else {
                            println!("   Liquidity: None");
                        }

                        println!("   Volume 24h: ${:.2}", first_pair.volume.h24);
                    }
                }
                Err(e) => {
                    println!("❌ Failed to parse as Vec<TokenPair>: {}", e);

                    // Try to understand what the structure actually is
                    if let Ok(json_value) = serde_json::from_str::<Value>(&response_text) {
                        suggest_fix_based_on_structure(&json_value);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Invalid JSON: {}", e);
            println!("🔍 Raw response (first 1000 chars):");
            println!("{}", &response_text[..response_text.len().min(1000)]);
        }
    }

    Ok(())
}

fn analyze_json_structure(value: &Value) {
    match value {
        Value::Object(map) => {
            println!("🔍 JSON Object with keys:");
            for key in map.keys() {
                println!("   - {}", key);
            }

            // Check if it has a common wrapper structure
            if map.contains_key("pairs") {
                println!("📦 Found 'pairs' key - this might be wrapped data");
                if let Some(pairs) = map.get("pairs") {
                    println!("   Pairs type: {}", get_value_type(pairs));
                    if let Value::Array(arr) = pairs {
                        println!("   Pairs count: {}", arr.len());
                    }
                }
            }

            if map.contains_key("data") {
                println!("📦 Found 'data' key - this might be wrapped data");
                if let Some(data) = map.get("data") {
                    println!("   Data type: {}", get_value_type(data));
                    if let Value::Array(arr) = data {
                        println!("   Data count: {}", arr.len());
                    }
                }
            }
        }
        Value::Array(arr) => {
            println!("🔍 JSON Array with {} items", arr.len());
            if !arr.is_empty() {
                println!("   First item type: {}", get_value_type(&arr[0]));
                if let Value::Object(obj) = &arr[0] {
                    println!("   First item keys:");
                    for key in obj.keys().take(10) {
                        println!("     - {}", key);
                    }
                }
            }
        }
        _ => {
            println!("🔍 JSON type: {}", get_value_type(value));
        }
    }
}

fn get_value_type(value: &Value) -> &str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn suggest_fix_based_on_structure(value: &Value) {
    println!("\n💡 Suggestions to fix the parsing issue:");

    match value {
        Value::Object(map) => {
            if map.contains_key("pairs") {
                println!("   1. The API returns wrapped data with 'pairs' key");
                println!("   2. You need to parse as: ResponseWrapper {{ pairs: Vec<TokenPair> }}");
                println!(
                    "   3. Then access response.pairs instead of parsing directly as Vec<TokenPair>"
                );
            } else if map.contains_key("data") {
                println!("   1. The API returns wrapped data with 'data' key");
                println!("   2. You need to parse as: ResponseWrapper {{ data: Vec<TokenPair> }}");
                println!(
                    "   3. Then access response.data instead of parsing directly as Vec<TokenPair>"
                );
            } else {
                println!("   1. The API returns a single object, not an array");
                println!("   2. You might need to parse as TokenPair directly, not Vec<TokenPair>");
                println!("   3. Or the API structure has changed");
            }
        }
        Value::Array(_) => {
            println!("   1. The API returns an array as expected");
            println!("   2. The issue is likely with the TokenPair struct fields");
            println!("   3. Check if all required fields exist in the response");
            println!("   4. Consider making more fields optional with Option<T>");
        }
        _ => {
            println!("   1. The API is not returning the expected JSON structure");
            println!("   2. Check if the API endpoint has changed");
            println!("   3. Verify the token address format");
        }
    }
}
