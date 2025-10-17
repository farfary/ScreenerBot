/// Comprehensive DexScreener API debug tool
/// 
/// Tests ALL documented endpoints from https://docs.dexscreener.com/api/reference
/// and validates the implementation against actual API behavior.

use clap::Parser;
use colored::Colorize;
use reqwest::Client;
use serde_json::Value;
use std::time::Instant;

#[derive(Parser, Debug)]
#[clap(name = "debug_dexscreener")]
#[clap(about = "Debug and test DexScreener API endpoints")]
struct Args {
    /// Test all endpoints
    #[clap(long)]
    all: bool,

    /// Test token pools endpoint
    #[clap(long)]
    token_pools: bool,

    /// Test single pair endpoint
    #[clap(long)]
    single_pair: bool,

    /// Test search endpoint
    #[clap(long)]
    search: bool,

    /// Test token profiles (latest)
    #[clap(long)]
    profiles: bool,

    /// Test token boosts
    #[clap(long)]
    boosts: bool,

    /// Test orders endpoint
    #[clap(long)]
    orders: bool,

    /// Test batch tokens endpoint
    #[clap(long)]
    batch_tokens: bool,

    /// Test GeckoTerminal OHLCV endpoint
    #[clap(long)]
    geckoterminal_ohlcv: bool,

    /// Verbose output (show response bodies)
    #[clap(short, long)]
    verbose: bool,

    /// Custom token address for testing (defaults to SOL)
    #[clap(long, default_value = "So11111111111111111111111111111111111111112")]
    token: String,

    /// Custom pair address for testing
    #[clap(long, default_value = "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ")]
    pair: String,

    /// Chain ID for testing
    #[clap(long, default_value = "solana")]
    chain: String,
}

const BASE_URL: &str = "https://api.dexscreener.com";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Client::new();

    println!("\n{}", "🔍 DexScreener API Debug Tool".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("Base URL: {}\n", BASE_URL.yellow());

    let test_all = args.all || (!args.token_pools && !args.single_pair && !args.search 
        && !args.profiles && !args.boosts && !args.orders && !args.batch_tokens && !args.geckoterminal_ohlcv);

    if test_all || args.token_pools {
        test_token_pools(&client, &args).await;
    }

    if test_all || args.single_pair {
        test_single_pair(&client, &args).await;
    }

    if test_all || args.search {
        test_search(&client, &args).await;
    }

    if test_all || args.batch_tokens {
        test_batch_tokens(&client, &args).await;
    }

    if test_all || args.profiles {
        test_profiles(&client, &args).await;
    }

    if test_all || args.boosts {
        test_boosts(&client, &args).await;
    }

    if test_all || args.orders {
        test_orders(&client, &args).await;
    }

    if test_all || args.geckoterminal_ohlcv {
        test_geckoterminal_ohlcv(&client, &args).await;
    }

    println!("\n{}", "✅ Debug session complete".bold().green());
}

/// Test: GET /token-pairs/v1/{chainId}/{tokenAddress}
/// This is the PRIMARY endpoint for getting all pools for a token
async fn test_token_pools(client: &Client, args: &Args) {
    print_test_header("Token Pools (Primary Endpoint)", "/token-pairs/v1/{chainId}/{tokenAddress}");

    let url = format!("{}/token-pairs/v1/{}/{}", BASE_URL, args.chain, args.token);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Pools found:".cyan(), arr.len().to_string().green().bold());
                                    
                                    if args.verbose && !arr.is_empty() {
                                        println!("\n  {}", "First pool structure:".cyan());
                                        if let Some(first) = arr.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }

                                    // Validate structure
                                    if let Some(first) = arr.first() {
                                        validate_pool_structure(first);
                                    }
                                } else {
                                    println!("  {} Expected array, got: {:?}", "⚠️".yellow(), json);
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /latest/dex/pairs/{chainId}/{pairId}
/// Single pair lookup
async fn test_single_pair(client: &Client, args: &Args) {
    print_test_header("Single Pair", "/latest/dex/pairs/{chainId}/{pairId}");

    let url = format!("{}/latest/dex/pairs/{}/{}", BASE_URL, args.chain, args.pair);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(pairs) = json.get("pairs").and_then(|p| p.as_array()) {
                                    println!("  {} {}", "Pairs in response:".cyan(), pairs.len().to_string().green().bold());
                                    
                                    if args.verbose && !pairs.is_empty() {
                                        println!("\n  {}", "Pair structure:".cyan());
                                        if let Some(first) = pairs.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} No pairs array in response", "⚠️".yellow());
                                    if args.verbose {
                                        println!("  Response: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /latest/dex/search?q={query}
async fn test_search(client: &Client, args: &Args) {
    print_test_header("Search", "/latest/dex/search?q={query}");

    let query = "SOL/USDC";
    let url = format!("{}/latest/dex/search?q={}", BASE_URL, query);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(pairs) = json.get("pairs").and_then(|p| p.as_array()) {
                                    println!("  {} {} for '{}'", "Results:".cyan(), pairs.len().to_string().green().bold(), query);
                                    
                                    if args.verbose && !pairs.is_empty() {
                                        println!("\n  {}", "First result:".cyan());
                                        if let Some(first) = pairs.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} No pairs in response", "⚠️".yellow());
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /tokens/v1/{chainId}/{tokenAddresses}
/// Batch fetch up to 30 tokens
async fn test_batch_tokens(client: &Client, args: &Args) {
    print_test_header("Batch Tokens", "/tokens/v1/{chainId}/{tokenAddresses}");

    // Test with SOL and USDC
    let tokens = "So11111111111111111111111111111111111111112,EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let url = format!("{}/tokens/v1/{}/{}", BASE_URL, args.chain, tokens);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Pools found:".cyan(), arr.len().to_string().green().bold());
                                    
                                    // Group by token
                                    let mut token_counts = std::collections::HashMap::new();
                                    for item in arr {
                                        if let Some(base) = item.get("baseToken").and_then(|t| t.get("address")).and_then(|a| a.as_str()) {
                                            *token_counts.entry(base).or_insert(0) += 1;
                                        }
                                    }
                                    
                                    println!("  {} {}", "Tokens with pools:".cyan(), token_counts.len());
                                    for (token, count) in token_counts {
                                        println!("    {} {} pools", token.trim_start_matches("0x").chars().take(8).collect::<String>(), count);
                                    }

                                    if args.verbose && !arr.is_empty() {
                                        println!("\n  {}", "First pool:".cyan());
                                        if let Some(first) = arr.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} Expected array, got object", "⚠️".yellow());
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /token-profiles/latest/v1
async fn test_profiles(client: &Client, args: &Args) {
    print_test_header("Latest Token Profiles", "/token-profiles/latest/v1");

    let url = format!("{}/token-profiles/latest/v1", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Profiles:".cyan(), arr.len().to_string().green().bold());
                                    
                                    if args.verbose && !arr.is_empty() {
                                        println!("\n  {}", "First profile:".cyan());
                                        if let Some(first) = arr.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} Unexpected response format", "⚠️".yellow());
                                    if args.verbose {
                                        println!("  {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /token-boosts/latest/v1 and /token-boosts/top/v1
async fn test_boosts(client: &Client, args: &Args) {
    // Test latest boosts
    print_test_header("Latest Boosted Tokens", "/token-boosts/latest/v1");

    let url = format!("{}/token-boosts/latest/v1", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Boosted tokens:".cyan(), arr.len().to_string().green().bold());
                                } else {
                                    println!("  {} Unexpected format", "⚠️".yellow());
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();

    // Test top boosts
    print_test_header("Top Boosted Tokens", "/token-boosts/top/v1");

    let url = format!("{}/token-boosts/top/v1", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Top boosts:".cyan(), arr.len().to_string().green().bold());
                                    
                                    if args.verbose && !arr.is_empty() {
                                        println!("\n  {}", "First boost:".cyan());
                                        if let Some(first) = arr.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} Unexpected format", "⚠️".yellow());
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

/// Test: GET /orders/v1/{chainId}/{tokenAddress}
async fn test_orders(client: &Client, args: &Args) {
    print_test_header("Token Orders", "/orders/v1/{chainId}/{tokenAddress}");

    let url = format!("{}/orders/v1/{}/{}", BASE_URL, args.chain, args.token);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(arr) = json.as_array() {
                                    println!("  {} {}", "Orders:".cyan(), arr.len().to_string().green().bold());
                                    
                                    if args.verbose && !arr.is_empty() {
                                        println!("\n  {}", "First order:".cyan());
                                        if let Some(first) = arr.first() {
                                            print_json_structure(first, 2);
                                        }
                                    }
                                } else {
                                    println!("  {} Unexpected format", "⚠️".yellow());
                                    if args.verbose {
                                        println!("  {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  {} Parse error: {}", "❌".red(), e);
                                if args.verbose {
                                    println!("  Body: {}", body);
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {} Body read error: {}", "❌".red(), e),
                }
            } else {
                let body = response.text().await.unwrap_or_default();
                println!("  {} {}", "Error:".red(), body);
            }
        }
        Err(e) => {
            println!("  {} Request failed: {}", "❌".red(), e);
        }
    }

    println!();
}

// Helper functions

fn print_test_header(name: &str, endpoint: &str) {
    println!("{}", "─".repeat(60).bright_black());
    println!("{} {}", "Testing:".bold(), name.bold().white());
    println!("{} {}", "Endpoint:".cyan(), endpoint.yellow());
}

fn print_status(status: u16, duration: std::time::Duration) {
    let status_str = format!("{}", status);
    let colored_status = if status >= 200 && status < 300 {
        status_str.green()
    } else if status >= 400 {
        status_str.red()
    } else {
        status_str.yellow()
    };

    println!("  {} {} ({:.2}ms)", "Status:".cyan(), colored_status.bold(), duration.as_secs_f64() * 1000.0);
}

fn print_json_structure(value: &Value, indent: usize) {
    let prefix = " ".repeat(indent);
    
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter().take(10) {
                let type_str = match val {
                    Value::Null => "null",
                    Value::Bool(_) => "bool",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                };
                
                let preview = match val {
                    Value::String(s) => format!(": \"{}\"", s.chars().take(50).collect::<String>()),
                    Value::Number(n) => format!(": {}", n),
                    Value::Bool(b) => format!(": {}", b),
                    Value::Array(a) => format!(" [{}]", a.len()),
                    Value::Object(o) => format!(" {{{} fields}}", o.len()),
                    Value::Null => String::new(),
                };

                println!("{}  {} ({}){}", prefix, key.cyan(), type_str.bright_black(), preview);
            }
            
            if map.len() > 10 {
                println!("{}  ... {} more fields", prefix, map.len() - 10);
            }
        }
        _ => {
            println!("{}{:?}", prefix, value);
        }
    }
}

fn validate_pool_structure(pool: &Value) {
    println!("\n  {}", "Structure Validation:".cyan().bold());
    
    let required_fields = vec![
        "chainId", "dexId", "pairAddress", "baseToken", "quoteToken",
        "priceNative", "priceUsd", "liquidity"
    ];

    let mut missing = Vec::new();
    let mut present = Vec::new();

    for field in &required_fields {
        if pool.get(field).is_some() {
            present.push(*field);
        } else {
            missing.push(*field);
        }
    }

    println!("    {} {}/{}", "Fields:".cyan(), present.len().to_string().green(), required_fields.len());
    
    if !missing.is_empty() {
        println!("    {} {:?}", "Missing:".red(), missing);
    } else {
        println!("    {} All required fields present", "✓".green());
    }
}
