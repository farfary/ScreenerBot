/*
 * Debug API Response - Check DexScreener API format
 */

use anyhow::{ Context, Result };
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔍 Debugging DexScreener API Response Format");
    println!("=============================================\n");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenerBot/1.0")
        .build()
        .context("Failed to create HTTP client")?;

    // Test with one of the failing tokens
    let test_token = "CkDU2HSnsrcPX6V1PiHPYUd9v3nkxjkBEV4EYfp8cuCh";
    let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", test_token);

    println!("🌐 Making request to: {}", url);

    let response = client.get(&url).send().await.context("Failed to send request")?;

    let status = response.status();
    println!("📊 Response status: {}", status);

    if !status.is_success() {
        println!("❌ API request failed with status: {}", status);
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("Error: {}", error_text);
        return Ok(());
    }

    // Get raw response text first
    let raw_text = response.text().await.context("Failed to get response text")?;
    println!("📄 Raw response (first 500 chars):");
    println!("{}", &raw_text[..std::cmp::min(500, raw_text.len())]);
    println!("...\n");

    // Try to parse as generic JSON to see the structure
    match serde_json::from_str::<Value>(&raw_text) {
        Ok(json_value) => {
            println!("✅ Successfully parsed as JSON");
            println!("🔍 JSON structure:");

            match &json_value {
                Value::Object(map) => {
                    println!("  Root is an Object with {} keys:", map.len());
                    for key in map.keys() {
                        println!("    - {}", key);
                    }

                    // Check if there's a 'pairs' key
                    if let Some(pairs) = map.get("pairs") {
                        match pairs {
                            Value::Array(arr) => {
                                println!("  'pairs' is an array with {} elements", arr.len());
                                if !arr.is_empty() {
                                    println!("  First pair structure:");
                                    if let Value::Object(first_pair) = &arr[0] {
                                        for key in first_pair.keys() {
                                            println!("    - {}", key);
                                        }
                                    }
                                }
                            }
                            _ => println!("  'pairs' is not an array: {:?}", pairs),
                        }
                    }

                    // Check if there's a 'schemaVersion' or similar
                    if let Some(schema) = map.get("schemaVersion") {
                        println!("  Schema version: {:?}", schema);
                    }
                }
                Value::Array(arr) => {
                    println!("  Root is an Array with {} elements", arr.len());
                }
                _ => {
                    println!("  Root is neither Object nor Array: {:?}", json_value);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to parse as JSON: {}", e);
        }
    }

    // Try with a known good token that has lots of pairs
    println!("\n{}", "=".repeat(50));
    println!("Testing with a well-known token (SOL)...");

    let sol_token = "So11111111111111111111111111111111111111112";
    let sol_url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", sol_token);

    println!("🌐 Making request to: {}", sol_url);

    let sol_response = client.get(&sol_url).send().await.context("Failed to send SOL request")?;

    let sol_status = sol_response.status();
    println!("📊 SOL response status: {}", sol_status);

    if sol_status.is_success() {
        let sol_raw = sol_response.text().await.context("Failed to get SOL response text")?;
        println!("📄 SOL response structure:");

        if let Ok(sol_json) = serde_json::from_str::<Value>(&sol_raw) {
            match &sol_json {
                Value::Object(map) => {
                    println!("  SOL root is an Object with {} keys:", map.len());
                    for key in map.keys() {
                        println!("    - {}", key);
                    }

                    if let Some(pairs) = map.get("pairs") {
                        if let Value::Array(arr) = pairs {
                            println!("  SOL 'pairs' array has {} elements", arr.len());
                        }
                    }
                }
                Value::Array(arr) => {
                    println!("  SOL root is an Array with {} elements", arr.len());
                }
                _ => {
                    println!("  SOL root is unexpected type");
                }
            }
        }
    }

    Ok(())
}
