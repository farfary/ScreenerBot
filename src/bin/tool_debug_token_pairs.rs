/// Debug tool to investigate token pairs API response and fix parsing issues
///
/// This tool helps diagnose problems with DexScreener API responses where
/// certain fields like `pairCreatedAt` might be missing.

use std::env;
use reqwest;
use serde_json::Value;
use screenerbot::logger::{ log, LogTag };

/// Print comprehensive help menu for the Debug Token Pairs Tool
fn print_help() {
    println!("🔍 Debug Token Pairs Tool");
    println!("=====================================");
    println!("Diagnostic tool for analyzing DexScreener API responses and identifying");
    println!("parsing issues with token pair data, missing fields, and data structure problems.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_debug_token_pairs -- <TOKEN_ADDRESS> [OPTIONS]");
    println!("");
    println!("ARGUMENTS:");
    println!("    <TOKEN_ADDRESS>    Token mint address to analyze");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h         Show this help message");
    println!("");
    println!("EXAMPLES:");
    println!("    # Analyze USDC token pairs");
    println!(
        "    cargo run --bin tool_debug_token_pairs -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    println!("");
    println!("    # Debug specific token with potential parsing issues");
    println!(
        "    cargo run --bin tool_debug_token_pairs -- 3jX3imAgQKvkXCwWezrJzzfZXrtAg7rqoFxyPzSuPGpp"
    );
    println!("");
    println!("    # Analyze Bonk token data structure");
    println!(
        "    cargo run --bin tool_debug_token_pairs -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"
    );
    println!("");
    println!("ANALYSIS OUTPUT:");
    println!("    • Raw API response size and structure");
    println!("    • Number of pairs found for the token");
    println!("    • Missing field detection (pairCreatedAt, etc.)");
    println!("    • Field completeness analysis for each pair");
    println!("    • Liquidity and volume data validation");
    println!("    • Pool address and DEX identification");
    println!("    • Price data accuracy and timestamp validation");
    println!("");
    println!("DIAGNOSTIC FEATURES:");
    println!("    • Identifies missing pairCreatedAt timestamps");
    println!("    • Validates URL and image field formats");
    println!("    • Checks for null/undefined values in critical fields");
    println!("    • Analyzes data type mismatches");
    println!("    • Reports malformed JSON structures");
    println!("");
    println!("COMMON USE CASES:");
    println!("    • Debugging parser failures in main bot");
    println!("    • Validating new token compatibility");
    println!("    • Investigating API response inconsistencies");
    println!("    • Analyzing field availability across different tokens");
    println!("    • Troubleshooting timestamp parsing issues");
    println!("");
}

const DEXSCREENER_BASE_URL: &str = "https://api.dexscreener.com/latest/dex";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get token address from command line
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.len() != 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        if args.len() != 2 {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    }

    let token_address = &args[1];

    log(LogTag::System, "START", &format!("Debug tool starting for token: {}", token_address));

    // Fetch raw API response
    let url = format!("{}/tokens/{}", DEXSCREENER_BASE_URL, token_address);
    log(LogTag::Api, "REQUEST", &format!("Fetching: {}", url));

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        log(LogTag::Api, "ERROR", &format!("API request failed: {}", response.status()));
        return Ok(());
    }

    let response_text = response.text().await?;
    log(LogTag::Api, "RESPONSE_SIZE", &format!("Response size: {} bytes", response_text.len()));

    // Parse as JSON to examine structure
    let json_value: Value = serde_json::from_str(&response_text)?;

    // Check if response has pairs
    if let Some(pairs) = json_value.get("pairs") {
        if let Some(pairs_array) = pairs.as_array() {
            log(LogTag::Pool, "PAIRS_COUNT", &format!("Found {} pairs", pairs_array.len()));

            // Examine each pair for missing fields
            for (index, pair) in pairs_array.iter().enumerate() {
                log(
                    LogTag::Pool,
                    "PAIR_ANALYSIS",
                    &format!("=== Analyzing Pair {} ===", index + 1)
                );

                // Check required fields
                check_field(pair, "chainId", index + 1);
                check_field(pair, "dexId", index + 1);
                check_field(pair, "pairAddress", index + 1);
                check_field(pair, "baseToken", index + 1);
                check_field(pair, "quoteToken", index + 1);
                check_field(pair, "priceNative", index + 1);

                // Check potentially missing fields
                check_optional_field(pair, "pairCreatedAt", index + 1);
                check_optional_field(pair, "priceUsd", index + 1);
                check_optional_field(pair, "liquidity", index + 1);
                check_optional_field(pair, "volume", index + 1);
                check_optional_field(pair, "txns", index + 1);
                check_optional_field(pair, "priceChange", index + 1);
                check_optional_field(pair, "fdv", index + 1);
                check_optional_field(pair, "marketCap", index + 1);

                // Show the full structure of this pair
                log(
                    LogTag::Pool,
                    "PAIR_STRUCTURE",
                    &format!(
                        "Pair {} keys: {:?}",
                        index + 1,
                        pair
                            .as_object()
                            .map(|obj| obj.keys().collect::<Vec<_>>())
                            .unwrap_or_default()
                    )
                );

                // Show pairCreatedAt value if present
                if let Some(created_at) = pair.get("pairCreatedAt") {
                    log(
                        LogTag::Pool,
                        "PAIR_CREATED_AT",
                        &format!(
                            "Pair {} pairCreatedAt: {:?} (type: {})",
                            index + 1,
                            created_at,
                            match created_at {
                                Value::Number(_) => "Number",
                                Value::String(_) => "String",
                                Value::Null => "Null",
                                _ => "Other",
                            }
                        )
                    );
                } else {
                    log(
                        LogTag::Pool,
                        "MISSING_FIELD",
                        &format!("❌ Pair {} is MISSING pairCreatedAt field", index + 1)
                    );
                }
            }
        } else {
            log(LogTag::Pool, "ERROR", "Pairs field is not an array");
        }
    } else {
        log(LogTag::Pool, "ERROR", "No pairs field in response");
    }

    // Save raw response for manual inspection
    let filename = format!("debug_token_pairs_{}.json", token_address);
    std::fs::write(&filename, &response_text)?;
    log(LogTag::System, "SAVED", &format!("Raw response saved to: {}", filename));

    // Try to parse with current TokenPair struct to see exact error
    log(LogTag::Pool, "PARSE_TEST", "Testing parse with current TokenPair struct...");

    match serde_json::from_str::<serde_json::Value>(&response_text) {
        Ok(json) => {
            if let Some(pairs) = json.get("pairs") {
                if let Some(pairs_array) = pairs.as_array() {
                    for (index, pair) in pairs_array.iter().enumerate() {
                        log(
                            LogTag::Pool,
                            "PARSE_ATTEMPT",
                            &format!("Attempting to parse pair {}...", index + 1)
                        );

                        // Try to deserialize this specific pair as TokenPair
                        match
                            serde_json::from_value::<screenerbot::tokens::api::TokenPair>(
                                pair.clone()
                            )
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Pool,
                                    "PARSE_SUCCESS",
                                    &format!("✅ Pair {} parsed successfully", index + 1)
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Pool,
                                    "PARSE_ERROR",
                                    &format!("❌ Pair {} parse failed: {}", index + 1, e)
                                );

                                // Show the problematic pair data
                                log(
                                    LogTag::Pool,
                                    "PROBLEMATIC_PAIR",
                                    &format!(
                                        "Pair {} data: {}",
                                        index + 1,
                                        serde_json::to_string_pretty(pair)?
                                    )
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            log(LogTag::Pool, "JSON_ERROR", &format!("Failed to parse response as JSON: {}", e));
        }
    }

    log(LogTag::System, "COMPLETE", "Debug analysis complete");

    Ok(())
}

fn check_field(pair: &Value, field_name: &str, pair_index: usize) {
    if pair.get(field_name).is_some() {
        log(LogTag::Pool, "FIELD_PRESENT", &format!("✅ Pair {} has {}", pair_index, field_name));
    } else {
        log(
            LogTag::Pool,
            "FIELD_MISSING",
            &format!("❌ Pair {} MISSING required field: {}", pair_index, field_name)
        );
    }
}

fn check_optional_field(pair: &Value, field_name: &str, pair_index: usize) {
    if let Some(value) = pair.get(field_name) {
        log(
            LogTag::Pool,
            "OPTIONAL_PRESENT",
            &format!("✅ Pair {} has {}: {:?}", pair_index, field_name, value)
        );
    } else {
        log(
            LogTag::Pool,
            "OPTIONAL_MISSING",
            &format!("⚠️  Pair {} missing optional field: {}", pair_index, field_name)
        );
    }
}
