/// Comprehensive GeckoTerminal API debug tool
/// 
/// Tests ALL available GeckoTerminal endpoints and validates implementation

use clap::Parser;
use colored::Colorize;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[clap(name = "debug_geckoterminal")]
#[clap(about = "Debug and test GeckoTerminal API endpoints")]
struct Args {
    /// Test all endpoints
    #[clap(long)]
    all: bool,

    /// Test token pools endpoint
    #[clap(long)]
    token_pools: bool,

    /// Test top pools by token endpoint (with params)
    #[clap(long)]
    top_pools_token: bool,

    /// Test OHLCV endpoint
    #[clap(long)]
    ohlcv: bool,

    /// Test trending pools endpoint
    #[clap(long)]
    trending: bool,

    /// Test specific pool endpoint
    #[clap(long)]
    pool_data: bool,

    /// Test multi-pool endpoint
    #[clap(long)]
    multi_pools: bool,

    /// Test top pools endpoint
    #[clap(long)]
    top_pools: bool,

    /// Test DEXes endpoint
    #[clap(long)]
    dexes: bool,

    /// Test new pools endpoint
    #[clap(long)]
    new_pools: bool,

    /// Test tokens multi endpoint
    #[clap(long)]
    tokens_multi: bool,

    /// Test token info endpoint
    #[clap(long)]
    token_info: bool,

    /// Verbose output (show response bodies)
    #[clap(short, long)]
    verbose: bool,

    /// Network ID for testing
    #[clap(long, default_value = "solana")]
    network: String,

    /// Custom token address for testing (defaults to SOL)
    #[clap(long, default_value = "So11111111111111111111111111111111111111112")]
    token: String,

    /// Custom pool address for testing (defaults to SOL/USDC on Raydium)
    #[clap(long, default_value = "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj")]
    pool: String,

    /// OHLCV timeframe (day, hour, minute)
    #[clap(long, default_value = "day")]
    timeframe: String,

    /// OHLCV aggregate period
    #[clap(long, default_value = "1")]
    aggregate: u32,

    /// OHLCV limit
    #[clap(long, default_value = "10")]
    limit: u32,

    /// OHLCV currency (usd or token)
    #[clap(long, default_value = "usd")]
    currency: String,
}

const BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = Client::new();

    println!("\n{}", "🦎 GeckoTerminal API Debug Tool".bold().green());
    println!("{}", "=".repeat(60).green());
    println!("Base URL: {}\n", BASE_URL.yellow());

    let test_all = args.all || (!args.token_pools && !args.top_pools_token && !args.ohlcv && !args.trending && !args.pool_data && !args.multi_pools && !args.top_pools && !args.dexes && !args.new_pools && !args.tokens_multi && !args.token_info);

    if test_all || args.token_pools {
        test_token_pools(&client, &args).await;
    }

    if test_all || args.top_pools_token {
        test_top_pools_by_token(&client, &args).await;
    }

    if test_all || args.trending {
        test_trending_pools(&client, &args).await;
    }

    if test_all || args.top_pools {
        test_top_pools(&client, &args).await;
    }

    if test_all || args.dexes {
        test_dexes(&client, &args).await;
    }

    if test_all || args.new_pools {
        test_new_pools(&client, &args).await;
    }

    if test_all || args.tokens_multi {
        test_tokens_multi(&client, &args).await;
    }

    if test_all || args.token_info {
        test_token_info(&client, &args).await;
    }

    if test_all || args.pool_data {
        test_pool_by_address(&client, &args).await;
    }

    if test_all || args.multi_pools {
        test_multi_pools(&client, &args).await;
    }

    if test_all || args.ohlcv {
        test_ohlcv_day(&client, &args).await;
        test_ohlcv_hour(&client, &args).await;
        test_ohlcv_minute(&client, &args).await;
        test_ohlcv_with_params(&client, &args).await;
    }

    println!("\n{}", "✅ Debug session complete".bold().green());
}

/// Test: GET /networks/{network}/tokens/{token}/pools
/// Get all pools for a token
async fn test_token_pools(client: &Client, args: &Args) {
    print_test_header("Token Pools", &format!("/networks/{}/tokens/{}/pools", args.network, "{{token}}"));

    let url = format!("{}/networks/{}/tokens/{}/pools", BASE_URL, args.network, args.token);
    
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
                                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                    println!("  {} {}", "Pools found:".cyan(), data.len().to_string().green().bold());
                                    
                                    if args.verbose && !data.is_empty() {
                                        println!("\n  {}", "First pool structure:".cyan());
                                        if let Some(first) = data.first() {
                                            if let Some(attrs) = first.get("attributes") {
                                                print_json_structure(attrs, 2);
                                            }
                                        }
                                    }

                                    // Validate structure
                                    if let Some(first) = data.first() {
                                        validate_pool_structure(first);
                                    }
                                } else {
                                    println!("  {} Expected data array, got: {:?}", "⚠️".yellow(), json.get("data"));
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

    // Test with include parameter
    print_test_header(
        &format!("Trending Pools ({}, with includes)", args.network), 
        &format!("/networks/{}/trending_pools?include=base_token,quote_token,dex", args.network)
    );

    let url = format!("{}/networks/{}/trending_pools?include=base_token,quote_token,dex", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            println!("  {} {}", "Pools with includes:".cyan(), data.len().to_string().green().bold());
                            
                            // Check if includes are present
                            if let Some(first_pool) = data.get(0) {
                                let has_base_token = first_pool.get("relationships")
                                    .and_then(|r| r.get("base_token"))
                                    .is_some();
                                let has_quote_token = first_pool.get("relationships")
                                    .and_then(|r| r.get("quote_token"))
                                    .is_some();
                                let has_dex = first_pool.get("relationships")
                                    .and_then(|r| r.get("dex"))
                                    .is_some();
                                
                                println!("  {} base_token: {}, quote_token: {}, dex: {}", 
                                    "Includes present:".cyan(),
                                    if has_base_token { "✓".green() } else { "✗".red() },
                                    if has_quote_token { "✓".green() } else { "✗".red() },
                                    if has_dex { "✓".green() } else { "✗".red() }
                                );
                            }
                        }
                    }
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

async fn test_top_pools_by_token(client: &Client, args: &Args) {
    // Test 1: Basic top pools by token (no params)
    print_test_header(
        &format!("Top Pools by Token ({}, basic)", args.network),
        &format!("/networks/{}/tokens/{}/pools", args.network, args.token)
    );

    let url = format!("{}/networks/{}/tokens/{}/pools", BASE_URL, args.network, args.token);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools found", "✓".green(), data.len());
                        
                        println!("  → First 3 pools:");
                        for (i, pool) in data.iter().take(3).enumerate() {
                            if let Some(attrs) = pool.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let volume = attrs.get("volume_usd").and_then(|v| v.get("h24")).and_then(|h| h.as_str()).unwrap_or("N/A");
                                println!("    {}. {} (24h vol: ${})", i+1, name, volume);
                            }
                        }
                    }
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

    // Test 2: With sorting (by volume)
    print_test_header(
        &format!("Top Pools by Token ({}, sorted by volume)", args.network),
        &format!("/networks/{}/tokens/{}/pools?sort=h24_volume_usd_desc", args.network, args.token)
    );

    let url = format!("{}/networks/{}/tokens/{}/pools?sort=h24_volume_usd_desc", BASE_URL, args.network, args.token);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools sorted by 24h volume", "✓".green(), data.len());
                        
                        println!("  → Top 5 by volume:");
                        for (i, pool) in data.iter().take(5).enumerate() {
                            if let Some(attrs) = pool.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let volume = attrs.get("volume_usd").and_then(|v| v.get("h24")).and_then(|h| h.as_str()).unwrap_or("N/A");
                                let txns = attrs.get("transactions").and_then(|t| t.get("h24")).and_then(|h| h.get("buys")).and_then(|b| b.as_u64()).unwrap_or(0);
                                println!("    {}. {} - ${}vol, {} txns", i+1, name, volume, txns);
                            }
                        }
                    }
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

    // Test 3: With includes (base_token, quote_token, dex)
    print_test_header(
        &format!("Top Pools by Token ({}, with includes)", args.network),
        &format!("/networks/{}/tokens/{}/pools?include=base_token,quote_token,dex", args.network, args.token)
    );

    let url = format!("{}/networks/{}/tokens/{}/pools?include=base_token,quote_token,dex", BASE_URL, args.network, args.token);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools with relationships", "✓".green(), data.len());
                        
                        if let Some(included) = json.get("included").and_then(|i| i.as_array()) {
                            println!("  {} {} included relationships", "✓".green(), included.len());
                        }

                        if let Some(first) = data.first() {
                            if let Some(rels) = first.get("relationships") {
                                let has_base = rels.get("base_token").is_some();
                                let has_quote = rels.get("quote_token").is_some();
                                let has_dex = rels.get("dex").is_some();
                                println!("  {} base_token: {}, quote_token: {}, dex: {}", 
                                    "Relationships:".cyan(),
                                    if has_base { "✓".green() } else { "✗".red() },
                                    if has_quote { "✓".green() } else { "✗".red() },
                                    if has_dex { "✓".green() } else { "✗".red() }
                                );
                            }
                        }
                    }
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

    // Test 4: Sorted by transaction count
    print_test_header(
        &format!("Top Pools by Token ({}, sorted by tx count)", args.network),
        &format!("/networks/{}/tokens/{}/pools?sort=h24_tx_count_desc", args.network, args.token)
    );

    let url = format!("{}/networks/{}/tokens/{}/pools?sort=h24_tx_count_desc", BASE_URL, args.network, args.token);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools sorted by transaction count", "✓".green(), data.len());
                        
                        println!("  → Top 3 by transaction count:");
                        for (i, pool) in data.iter().take(3).enumerate() {
                            if let Some(attrs) = pool.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let buys = attrs.get("transactions").and_then(|t| t.get("h24")).and_then(|h| h.get("buys")).and_then(|b| b.as_u64()).unwrap_or(0);
                                let sells = attrs.get("transactions").and_then(|t| t.get("h24")).and_then(|h| h.get("sells")).and_then(|s| s.as_u64()).unwrap_or(0);
                                println!("    {}. {} - {} buys, {} sells", i+1, name, buys, sells);
                            }
                        }
                    }
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

/// Test: GET /networks/{network}/trending_pools
/// Get trending pools by network
async fn test_trending_pools(client: &Client, args: &Args) {
    // Test default (solana, 24h)
    print_test_header(
        &format!("Trending Pools ({}, 24h)", args.network),
        &format!("/networks/{}/trending_pools", args.network)
    );

    let url = format!("{}/networks/{}/trending_pools", BASE_URL, args.network);
    
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
                                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                    println!("  {} {}", "Trending pools:".cyan(), data.len().to_string().green().bold());
                                    
                                    if args.verbose && !data.is_empty() {
                                        println!("\n  {}", "First 3 trending pools:".cyan());
                                        for (i, pool) in data.iter().take(3).enumerate() {
                                            if let Some(attrs) = pool.get("attributes") {
                                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                                println!("    {}. {}", i + 1, name);
                                            }
                                        }
                                    }

                                    validate_pool_structure(data.first().unwrap());
                                } else {
                                    println!("  {} Expected data array", "⚠️".yellow());
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

    // Test different durations
    for duration in &["5m", "1h", "6h"] {
        print_test_header(
            &format!("Trending Pools ({}, {})", args.network, duration), 
            &format!("/networks/{}/trending_pools?duration={}", args.network, duration)
        );

        let url = format!("{}/networks/{}/trending_pools?duration={}", BASE_URL, args.network, duration);
        
        let start = Instant::now();
        match client.get(&url).send().await {
            Ok(response) => {
                let duration_time = start.elapsed();
                let status = response.status();
                
                print_status(status.as_u16(), duration_time);

                if status.is_success() {
                    if let Ok(body) = response.text().await {
                        if let Ok(json) = serde_json::from_str::<Value>(&body) {
                            if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                                println!("  {} {}", "Pools:".cyan(), data.len().to_string().green().bold());
                            }
                        }
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

    // Test pagination
    print_test_header(
        &format!("Trending Pools ({}, Page 2)", args.network), 
        &format!("/networks/{}/trending_pools?page=2", args.network)
    );

    let url = format!("{}/networks/{}/trending_pools?page=2", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            println!("  {} {}", "Pools on page 2:".cyan(), data.len().to_string().green().bold());
                        }
                    }
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

    // Test with include parameter
    print_test_header(
        &format!("Trending Pools ({}, with include)", args.network), 
        &format!("/networks/{}/trending_pools?include=base_token,quote_token,dex", args.network)
    );

    let url = format!("{}/networks/{}/trending_pools?include=base_token,quote_token,dex", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            println!("  {} {}", "Pools with include:".cyan(), data.len().to_string().green().bold());
                            
                            // Check if includes are present  
                            if let Some(first_pool) = data.get(0) {
                                let has_base = first_pool.get("relationships").and_then(|r| r.get("base_token")).is_some();
                                let has_quote = first_pool.get("relationships").and_then(|r| r.get("quote_token")).is_some();
                                let has_dex = first_pool.get("relationships").and_then(|r| r.get("dex")).is_some();
                                
                                println!("  {} base_token: {}, quote_token: {}, dex: {}", 
                                    "Include fields:".cyan(),
                                    if has_base { "✓".green() } else { "✗".red() },
                                    if has_quote { "✓".green() } else { "✗".red() },
                                    if has_dex { "✓".green() } else { "✗".red() }
                                );
                            }
                        }
                    }
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

/// Test: GET /networks/{network}/pools
/// Get top pools by network
async fn test_top_pools(client: &Client, args: &Args) {
    // Test 1: Basic top pools (default sort)
    print_test_header(
        &format!("Top Pools ({}, h24_tx_count)", args.network),
        &format!("/networks/{}/pools", args.network)
    );

    let url = format!("{}/networks/{}/pools", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {}", "Top pools:".cyan(), data.len().to_string().green().bold());
                        
                        // Show first 3 pools
                        for (i, pool) in data.iter().take(3).enumerate() {
                            if let Some(attrs) = pool["attributes"].as_object() {
                                let name = attrs.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let volume = attrs.get("volume_usd")
                                    .and_then(|v| v.get("h24"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("N/A");
                                println!("    {}. {} - Vol: ${}", i + 1, name.yellow(), volume.cyan());
                            }
                        }
                        
                        if args.verbose {
                            println!("\n{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                        }
                    }
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

    // Test 2: Sort by volume
    print_test_header(
        &format!("Top Pools ({}, h24_volume)", args.network),
        &format!("/networks/{}/pools?sort=h24_volume_usd_desc", args.network)
    );

    let url = format!("{}/networks/{}/pools?sort=h24_volume_usd_desc", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {}", "Top by volume:".cyan(), data.len().to_string().green().bold());
                        
                        // Show first 3 pools with volume
                        for (i, pool) in data.iter().take(3).enumerate() {
                            if let Some(attrs) = pool["attributes"].as_object() {
                                let name = attrs.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let volume = attrs.get("volume_usd")
                                    .and_then(|v| v.get("h24"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("N/A");
                                let price = attrs.get("base_token_price_usd")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("N/A");
                                println!("    {}. {} - Vol: ${} | Price: ${}", 
                                    i + 1, name.yellow(), volume.cyan(), price.green());
                            }
                        }
                    }
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

    // Test 3: With include parameters
    print_test_header(
        &format!("Top Pools ({}, with include)", args.network),
        &format!("/networks/{}/pools?include=base_token,quote_token,dex", args.network)
    );

    let url = format!("{}/networks/{}/pools?include=base_token,quote_token,dex", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(included) = json.get("included").and_then(|v| v.as_array()) {
                        let has_base_token = included.iter().any(|item| item["type"] == "token");
                        let has_dex = included.iter().any(|item| item["type"] == "dex");
                        
                        println!(
                            "  {} base_token: {}, quote_token: {}, dex: {}",
                            "Include fields:".cyan(),
                            if has_base_token { "✓".green() } else { "✗".red() },
                            if has_base_token { "✓".green() } else { "✗".red() },
                            if has_dex { "✓".green() } else { "✗".red() }
                        );
                    }
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

    // Test 4: Page 2
    print_test_header(
        &format!("Top Pools ({}, page 2)", args.network),
        &format!("/networks/{}/pools?page=2", args.network)
    );

    let url = format!("{}/networks/{}/pools?page=2", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {} pools on page 2", "✓".green(), data.len().to_string().cyan());
                    }
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

    // Test 5: Ethereum network
    print_test_header(
        "Top Pools (eth, volume sort)",
        "/networks/eth/pools?sort=h24_volume_usd_desc"
    );

    let url = format!("{}/networks/eth/pools?sort=h24_volume_usd_desc", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {} Ethereum pools", "✓".green(), data.len().to_string().cyan());
                        
                        // Show top 3
                        for (i, pool) in data.iter().take(3).enumerate() {
                            if let Some(attrs) = pool["attributes"].as_object() {
                                let name = attrs.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let volume = attrs.get("volume_usd")
                                    .and_then(|v| v.get("h24"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("N/A");
                                println!("    {}. {} - Vol: ${}", i + 1, name.yellow(), volume.cyan());
                            }
                        }
                    }
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

/// Test: GET /networks/{network}/dexes
/// Get supported DEXes list by network
async fn test_dexes(client: &Client, args: &Args) {
    // Test 1: Solana DEXes
    print_test_header(
        &format!("DEXes List ({})", args.network),
        &format!("/networks/{}/dexes", args.network)
    );

    let url = format!("{}/networks/{}/dexes", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {} DEXes supported", "✓".green(), data.len().to_string().cyan().bold());
                        
                        // Show first 10 DEXes
                        println!("  {} First 10 DEXes:", "→".cyan());
                        for (i, dex) in data.iter().take(10).enumerate() {
                            if let Some(attrs) = dex["attributes"].as_object() {
                                let name = attrs.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let id = dex["id"].as_str().unwrap_or("unknown");
                                println!("    {}. {} ({})", i + 1, name.yellow(), id.bright_black());
                            }
                        }
                        
                        if args.verbose {
                            println!("\n{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                        }
                    }
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

    // Test 2: Ethereum DEXes
    print_test_header(
        "DEXes List (eth)",
        "/networks/eth/dexes"
    );

    let url = format!("{}/networks/eth/dexes", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {} Ethereum DEXes", "✓".green(), data.len().to_string().cyan());
                        
                        // Show top 5
                        for (i, dex) in data.iter().take(5).enumerate() {
                            if let Some(attrs) = dex["attributes"].as_object() {
                                let name = attrs.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                println!("    {}. {}", i + 1, name.yellow());
                            }
                        }
                    }
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

    // Test 3: Page 2
    print_test_header(
        &format!("DEXes List ({}, page 2)", args.network),
        &format!("/networks/{}/dexes?page=2", args.network)
    );

    let url = format!("{}/networks/{}/dexes?page=2", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} {} DEXes on page 2", "✓".green(), data.len().to_string().cyan());
                    }
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

async fn test_new_pools(client: &Client, args: &Args) {
    // Test 1: Latest new pools on Solana (basic)
    print_test_header(
        &format!("Latest New Pools ({})", args.network),
        &format!("/networks/{}/new_pools", args.network)
    );

    let url = format!("{}/networks/{}/new_pools", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();
                
                if args.verbose {
                    println!("  {}", "Response Body:".cyan());
                    println!("{}", body);
                }

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} new pools", "✓".green(), data.len());
                        
                        println!("  → First 5 pools:");
                        for (i, pool) in data.iter().take(5).enumerate() {
                            if let Some(attrs) = pool.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let address = attrs.get("address").and_then(|a| a.as_str()).unwrap_or("N/A");
                                let created = attrs.get("pool_created_at").and_then(|c| c.as_str()).unwrap_or("N/A");
                                println!("    {}. {} ({}) - created: {}", i+1, name, &address[..8], created);
                            }
                        }
                    }
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

    // Test 2: New pools with includes (base_token, quote_token)
    print_test_header(
        &format!("New Pools ({}, with includes)", args.network),
        &format!("/networks/{}/new_pools?include=base_token,quote_token", args.network)
    );

    let url = format!("{}/networks/{}/new_pools?include=base_token,quote_token", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools with token data", "✓".green(), data.len());
                        
                        if let Some(included) = json.get("included").and_then(|i| i.as_array()) {
                            println!("  {} {} included relationships", "✓".green(), included.len());
                        }

                        if let Some(first) = data.first() {
                            if let Some(rels) = first.get("relationships") {
                                let has_base = rels.get("base_token").is_some();
                                let has_quote = rels.get("quote_token").is_some();
                                println!("  {} base_token: {}, quote_token: {}", 
                                    "Relationships:".cyan(),
                                    if has_base { "✓".green() } else { "✗".red() },
                                    if has_quote { "✓".green() } else { "✗".red() }
                                );
                            }
                        }
                    }
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

    // Test 3: Ethereum new pools
    print_test_header(
        "New Pools (eth)",
        "/networks/eth/new_pools"
    );

    let url = format!("{}/networks/eth/new_pools", BASE_URL);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} Ethereum new pools", "✓".green(), data.len());
                        
                        println!("  → First 5 pools:");
                        for (i, pool) in data.iter().take(5).enumerate() {
                            if let Some(attrs) = pool.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let dex = attrs.get("dex_id").and_then(|d| d.as_str()).unwrap_or("N/A");
                                println!("    {}. {} (DEX: {})", i+1, name, dex);
                            }
                        }
                    }
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

    // Test 4: Page 2
    print_test_header(
        &format!("New Pools ({}, page 2)", args.network),
        &format!("/networks/{}/new_pools?page=2", args.network)
    );

    let url = format!("{}/networks/{}/new_pools?page=2", BASE_URL, args.network);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} pools on page 2", "✓".green(), data.len());
                    }
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

async fn test_tokens_multi(client: &Client, args: &Args) {
    // Test 1: Fetch 2 tokens (WETH and USDC on Ethereum)
    print_test_header(
        "Tokens Multi (eth, WETH + USDC)",
        "/networks/eth/tokens/multi/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2,0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
    );

    let addresses = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2,0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
    let url = format!("{}/networks/eth/tokens/multi/{}", BASE_URL, addresses);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();
                
                if args.verbose {
                    println!("  {}", "Response Body:".cyan());
                    println!("{}", body);
                }

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} tokens retrieved", "✓".green(), data.len());
                        
                        for token in data.iter() {
                            if let Some(attrs) = token.get("attributes") {
                                let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let price = attrs.get("price_usd").and_then(|p| p.as_str()).unwrap_or("N/A");
                                let decimals = attrs.get("decimals").and_then(|d| d.as_u64()).unwrap_or(0);
                                println!("    • {} ({}) - ${} - {} decimals", symbol, name, price, decimals);
                            }
                        }
                    }
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

    // Test 2: With top_pools include
    print_test_header(
        "Tokens Multi (eth, with top_pools)",
        "/networks/eth/tokens/multi/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2?include=top_pools"
    );

    let addresses = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    let url = format!("{}/networks/eth/tokens/multi/{}?include=top_pools", BASE_URL, addresses);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} token retrieved", "✓".green(), data.len());
                        
                        if let Some(included) = json.get("included").and_then(|i| i.as_array()) {
                            println!("  {} {} included items (top pools)", "✓".green(), included.len());
                            
                            println!("  → First 3 top pools:");
                            for (i, pool) in included.iter().take(3).enumerate() {
                                if let Some(attrs) = pool.get("attributes") {
                                    let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                    let volume = attrs.get("volume_usd").and_then(|v| v.get("h24")).and_then(|h| h.as_str()).unwrap_or("N/A");
                                    println!("    {}. {} (24h vol: ${})", i+1, name, volume);
                                }
                            }
                        }
                    }
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

    // Test 3: Solana tokens (SOL + USDC)
    print_test_header(
        "Tokens Multi (solana, SOL + USDC)",
        "/networks/solana/tokens/multi/So11111111111111111111111111111111111111112,EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );

    let addresses = "So11111111111111111111111111111111111111112,EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let url = format!("{}/networks/solana/tokens/multi/{}", BASE_URL, addresses);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} {} Solana tokens", "✓".green(), data.len());
                        
                        for token in data.iter() {
                            if let Some(attrs) = token.get("attributes") {
                                let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                                let price = attrs.get("price_usd").and_then(|p| p.as_str()).unwrap_or("N/A");
                                let mcap = attrs.get("market_cap_usd").and_then(|m| m.as_str()).unwrap_or("N/A");
                                println!("    • {} ({}) - ${} - MCap: ${}", symbol, name, price, mcap);
                            }
                        }
                    }
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

    // Test 4: With include_composition
    print_test_header(
        "Tokens Multi (eth, with composition)",
        "/networks/eth/tokens/multi/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2?include=top_pools&include_composition=true"
    );

    let addresses = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    let url = format!("{}/networks/eth/tokens/multi/{}?include=top_pools&include_composition=true", BASE_URL, addresses);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        println!("  {} Token data with composition", "✓".green());
                        
                        if let Some(first) = data.first() {
                            if let Some(attrs) = first.get("attributes") {
                                let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                                let volume_24h = attrs.get("volume_usd").and_then(|v| v.get("h24")).and_then(|h| h.as_str()).unwrap_or("N/A");
                                println!("  {} {} - 24h volume: ${}", "Token:".cyan(), symbol, volume_24h);
                            }
                        }

                        if let Some(included) = json.get("included").and_then(|i| i.as_array()) {
                            println!("  {} {} included items with composition", "✓".green(), included.len());
                        }
                    }
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

async fn test_token_info(client: &Client, args: &Args) {
    // Test 1: WETH on Ethereum
    print_test_header(
        "Token Info (eth, WETH)",
        "/networks/eth/tokens/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2/info"
    );

    let address = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    let url = format!("{}/networks/eth/tokens/{}/info", BASE_URL, address);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();
                
                if args.verbose {
                    println!("  {}", "Response Body:".cyan());
                    println!("{}", body);
                }

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data") {
                        if let Some(attrs) = data.get("attributes") {
                            let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                            let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                            let coingecko_id = attrs.get("coingecko_coin_id").and_then(|c| c.as_str()).unwrap_or("N/A");
                            let gt_score = attrs.get("gt_score").and_then(|g| g.as_f64()).unwrap_or(0.0);
                            
                            println!("  {} {}", "✓".green(), "Token metadata retrieved");
                            println!("    • Name: {}", name);
                            println!("    • Symbol: {}", symbol);
                            println!("    • CoinGecko ID: {}", coingecko_id);
                            println!("    • GT Score: {:.2}", gt_score);
                            
                            if let Some(image) = attrs.get("image_url").and_then(|i| i.as_str()) {
                                println!("    • Image: {}", if image.is_empty() { "N/A" } else { image });
                            }
                            
                            if let Some(twitter) = attrs.get("twitter_handle").and_then(|t| t.as_str()) {
                                println!("    • Twitter: @{}", twitter);
                            }
                            
                            if let Some(websites) = attrs.get("websites").and_then(|w| w.as_array()) {
                                if !websites.is_empty() {
                                    println!("    • Websites: {} links", websites.len());
                                }
                            }
                            
                            if let Some(desc) = attrs.get("description").and_then(|d| d.as_str()) {
                                let preview = if desc.len() > 100 { 
                                    format!("{}...", &desc[..100])
                                } else {
                                    desc.to_string()
                                };
                                println!("    • Description: {}", preview);
                            }
                        }
                    }
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

    // Test 2: USDC on Ethereum
    print_test_header(
        "Token Info (eth, USDC)",
        "/networks/eth/tokens/0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48/info"
    );

    let address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
    let url = format!("{}/networks/eth/tokens/{}/info", BASE_URL, address);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data") {
                        if let Some(attrs) = data.get("attributes") {
                            let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                            let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                            
                            println!("  {} {}", "✓".green(), "Token info retrieved");
                            println!("    • {} ({})", name, symbol);
                            
                            // Show social links
                            let mut socials = Vec::new();
                            if attrs.get("twitter_handle").is_some() { socials.push("Twitter"); }
                            if attrs.get("telegram_handle").is_some() { socials.push("Telegram"); }
                            if attrs.get("discord_url").is_some() { socials.push("Discord"); }
                            
                            if !socials.is_empty() {
                                println!("    • Socials: {}", socials.join(", "));
                            }
                        }
                    }
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

    // Test 3: Solana token (SOL)
    print_test_header(
        "Token Info (solana, SOL)",
        "/networks/solana/tokens/So11111111111111111111111111111111111111112/info"
    );

    let address = "So11111111111111111111111111111111111111112";
    let url = format!("{}/networks/solana/tokens/{}/info", BASE_URL, address);
    
    let start = Instant::now();
    match client.get(&url).timeout(Duration::from_secs(10)).send().await {
        Ok(response) => {
            let elapsed = start.elapsed();
            let status = response.status();
            
            println!("  {} {} ({:.2}ms)", "Status:".cyan(), status.as_u16().to_string().green(), elapsed.as_millis());

            if status.is_success() {
                let body = response.text().await.unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(data) = json.get("data") {
                        if let Some(attrs) = data.get("attributes") {
                            let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                            let symbol = attrs.get("symbol").and_then(|s| s.as_str()).unwrap_or("N/A");
                            let coingecko_id = attrs.get("coingecko_coin_id").and_then(|c| c.as_str()).unwrap_or("N/A");
                            
                            println!("  {} Solana token info", "✓".green());
                            println!("    • {} ({})", name, symbol);
                            println!("    • CoinGecko: {}", coingecko_id);
                            
                            if let Some(updated) = attrs.get("metadata_updated_at").and_then(|u| u.as_str()) {
                                println!("    • Last updated: {}", updated);
                            }
                        }
                    }
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

/// Test: GET /networks/{network}/pools/{address}
/// Get specific pool data by pool address
async fn test_pool_by_address(client: &Client, args: &Args) {
    // Test basic pool fetch
    print_test_header(
        &format!("Pool Data ({}, basic)", args.network),
        &format!("/networks/{}/pools/{}", args.network, args.pool)
    );

    let url = format!("{}/networks/{}/pools/{}", BASE_URL, args.network, args.pool);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data") {
                            if let Some(attrs) = data.get("attributes") {
                                let name = attrs.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                let address = attrs.get("address").and_then(|a| a.as_str()).unwrap_or("?");
                                let price_usd = attrs.get("base_token_price_usd").and_then(|p| p.as_str()).unwrap_or("0");
                                
                                println!("  {} {}", "Pool name:".cyan(), name.green());
                                println!("  {} {}", "Pool address:".cyan(), address);
                                println!("  {} ${}", "Price USD:".cyan(), price_usd);
                            }
                        }
                    }
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

    // Test with include parameters
    print_test_header(
        &format!("Pool Data ({}, with include)", args.network),
        &format!("/networks/{}/pools/{}?include=base_token,quote_token,dex", args.network, args.pool)
    );

    let url = format!("{}/networks/{}/pools/{}?include=base_token,quote_token,dex", BASE_URL, args.network, args.pool);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data") {
                            let has_base = data.get("relationships").and_then(|r| r.get("base_token")).is_some();
                            let has_quote = data.get("relationships").and_then(|r| r.get("quote_token")).is_some();
                            let has_dex = data.get("relationships").and_then(|r| r.get("dex")).is_some();
                            
                            println!("  {} base_token: {}, quote_token: {}, dex: {}", 
                                "Include fields:".cyan(),
                                if has_base { "✓".green() } else { "✗".red() },
                                if has_quote { "✓".green() } else { "✗".red() },
                                if has_dex { "✓".green() } else { "✗".red() }
                            );
                        }
                    }
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

    // Test with volume breakdown
    print_test_header(
        &format!("Pool Data ({}, volume breakdown)", args.network),
        &format!("/networks/{}/pools/{}?include_volume_breakdown=true", args.network, args.pool)
    );

    let url = format!("{}/networks/{}/pools/{}?include_volume_breakdown=true", BASE_URL, args.network, args.pool);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(body) = response.text().await {
                    if let Ok(json) = serde_json::from_str::<Value>(&body) {
                        if let Some(data) = json.get("data") {
                            if let Some(attrs) = data.get("attributes") {
                                let has_volume_breakdown = attrs.get("volume_usd").is_some();
                                println!("  {} {}", 
                                    "Volume breakdown:".cyan(),
                                    if has_volume_breakdown { "✓".green() } else { "✗".red() }
                                );
                            }
                        }
                    }
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

    // Test with composition
    print_test_header(
        &format!("Pool Data ({}, composition)", args.network),
        &format!("/networks/{}/pools/{}?include_composition=true", args.network, args.pool)
    );

    let url = format!("{}/networks/{}/pools/{}?include_composition=true", BASE_URL, args.network, args.pool);
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                println!("  {} Pool composition data retrieved", "✓".green());
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

/// Test: Multi-Pool endpoint
/// Tests fetching multiple pools in a single request (up to 30 addresses)
async fn test_multi_pools(client: &Client, args: &Args) {
    print_test_header(
        "Multi-Pool Data", 
        &format!("/networks/{}/pools/multi/{{addresses}}", args.network)
    );

    // Test 1: Basic multi-pool request with 2 pools
    println!("  {} Test 1: Basic multi-pool (2 pools)", "→".cyan());
    
    let pool1 = "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"; // SOL/USDC
    let pool2 = "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"; // SOL/USDT
    
    let url = format!(
        "{}/networks/{}/pools/multi/{},{}",
        BASE_URL, args.network, pool1, pool2
    );
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} Pools returned: {}", "✓".green(), data.len());
                        
                        for pool in data {
                            if let Some(attributes) = pool["attributes"].as_object() {
                                let name = attributes.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let address = attributes.get("address").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                println!("    - {} ({})", name.yellow(), address.bright_black());
                            }
                        }
                        
                        if args.verbose {
                            println!("\n{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                        }
                    }
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

    // Test 2: With include parameters
    println!("  {} Test 2: With include parameters", "→".cyan());
    
    let url = format!(
        "{}/networks/{}/pools/multi/{}?include=base_token,quote_token,dex",
        BASE_URL, args.network, pool1
    );
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    let has_base_token = json["included"].as_array()
                        .and_then(|arr| arr.iter().find(|item| item["type"] == "token"))
                        .is_some();
                    let has_quote_token = json["included"].as_array()
                        .and_then(|arr| arr.iter().filter(|item| item["type"] == "token").nth(1))
                        .is_some();
                    let has_dex = json["included"].as_array()
                        .and_then(|arr| arr.iter().find(|item| item["type"] == "dex"))
                        .is_some();
                    
                    println!(
                        "  {} Include fields: base_token: {}, quote_token: {}, dex: {}",
                        "✓".green(),
                        if has_base_token { "✓".green() } else { "✗".red() },
                        if has_quote_token { "✓".green() } else { "✗".red() },
                        if has_dex { "✓".green() } else { "✗".red() }
                    );
                    
                    if args.verbose {
                        println!("\n{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                    }
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

    // Test 3: With volume breakdown
    println!("  {} Test 3: With volume breakdown", "→".cyan());
    
    let url = format!(
        "{}/networks/{}/pools/multi/{}?include_volume_breakdown=true",
        BASE_URL, args.network, pool1
    );
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                println!("  {} Volume breakdown data retrieved", "✓".green());
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

    // Test 4: With composition
    println!("  {} Test 4: With composition", "→".cyan());
    
    let url = format!(
        "{}/networks/{}/pools/multi/{}?include_composition=true",
        BASE_URL, args.network, pool1
    );
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                println!("  {} Pool composition data retrieved", "✓".green());
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

    // Test 5: Multiple networks (Ethereum)
    println!("  {} Test 5: Ethereum network (3 pools)", "→".cyan());
    
    let eth_pool1 = "0x60594a405d53811d3bc4766596efd80fd545a270"; // DAI/WETH
    let eth_pool2 = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"; // USDC/WETH
    let eth_pool3 = "0x4e68ccd3e89f51c3074ca5072bbac773960dfa36"; // WETH/USDT
    
    let url = format!(
        "{}/networks/eth/pools/multi/{},{},{}",
        BASE_URL, eth_pool1, eth_pool2, eth_pool3
    );
    
    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(data) = json["data"].as_array() {
                        println!("  {} Pools returned: {}", "✓".green(), data.len());
                        
                        for pool in data {
                            if let Some(attributes) = pool["attributes"].as_object() {
                                let name = attributes.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let address = attributes.get("address").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                let price = attributes.get("base_token_price_usd")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("N/A");
                                println!("    - {} ({}) - ${}", name.yellow(), address.bright_black(), price.cyan());
                            }
                        }
                        
                        if args.verbose {
                            println!("\n{}", serde_json::to_string_pretty(&json).unwrap_or_default());
                        }
                    }
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

/// Test: OHLCV with day timeframe
async fn test_ohlcv_day(client: &Client, args: &Args) {
    print_test_header(
        "OHLCV (Day)", 
        &format!("/networks/{}/pools/{}/ohlcv/day", args.network, "{{pool}}")
    );

    let url = format!(
        "{}/networks/{}/pools/{}/ohlcv/day?aggregate=1&limit={}&currency={}",
        BASE_URL, args.network, args.pool, args.limit, args.currency
    );
    
    test_ohlcv_endpoint(client, &url, args).await;
    println!();
}

/// Test: OHLCV with hour timeframe
async fn test_ohlcv_hour(client: &Client, args: &Args) {
    print_test_header(
        "OHLCV (Hour)", 
        &format!("/networks/{}/pools/{}/ohlcv/hour", args.network, "{{pool}}")
    );

    let url = format!(
        "{}/networks/{}/pools/{}/ohlcv/hour?aggregate=1&limit={}&currency={}",
        BASE_URL, args.network, args.pool, args.limit, args.currency
    );
    
    test_ohlcv_endpoint(client, &url, args).await;
    println!();
}

/// Test: OHLCV with minute timeframe
async fn test_ohlcv_minute(client: &Client, args: &Args) {
    print_test_header(
        "OHLCV (Minute)", 
        &format!("/networks/{}/pools/{}/ohlcv/minute", args.network, "{{pool}}")
    );

    let url = format!(
        "{}/networks/{}/pools/{}/ohlcv/minute?aggregate=5&limit={}&currency={}",
        BASE_URL, args.network, args.pool, args.limit, args.currency
    );
    
    test_ohlcv_endpoint(client, &url, args).await;
    println!();
}

/// Test: OHLCV with various parameters
async fn test_ohlcv_with_params(client: &Client, args: &Args) {
    print_test_header(
        "OHLCV (With Token Param)", 
        "Test with token=base and currency=token"
    );

    let url = format!(
        "{}/networks/{}/pools/{}/ohlcv/hour?aggregate=4&limit=5&currency=token&token=base",
        BASE_URL, args.network, args.pool
    );
    
    test_ohlcv_endpoint(client, &url, args).await;
    println!();
}

/// Generic OHLCV endpoint tester
async fn test_ohlcv_endpoint(client: &Client, url: &str, args: &Args) {
    let start = Instant::now();
    match client.get(url).send().await {
        Ok(response) => {
            let duration = start.elapsed();
            let status = response.status();
            
            print_status(status.as_u16(), duration);

            if status.is_success() {
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<Value>(&body) {
                            Ok(json) => {
                                if let Some(ohlcv_list) = json.get("data")
                                    .and_then(|d| d.get("attributes"))
                                    .and_then(|a| a.get("ohlcv_list"))
                                    .and_then(|o| o.as_array()) 
                                {
                                    println!("  {} {}", "Candles:".cyan(), ohlcv_list.len().to_string().green().bold());
                                    
                                    if args.verbose && !ohlcv_list.is_empty() {
                                        println!("\n  {}", "First candle [timestamp, open, high, low, close, volume]:".cyan());
                                        if let Some(first) = ohlcv_list.first() {
                                            if let Some(arr) = first.as_array() {
                                                println!("    Timestamp: {}", arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as i64);
                                                println!("    Open:      {:.8}", arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0));
                                                println!("    High:      {:.8}", arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0));
                                                println!("    Low:       {:.8}", arr.get(3).and_then(|v| v.as_f64()).unwrap_or(0.0));
                                                println!("    Close:     {:.8}", arr.get(4).and_then(|v| v.as_f64()).unwrap_or(0.0));
                                                println!("    Volume:    {:.2}", arr.get(5).and_then(|v| v.as_f64()).unwrap_or(0.0));
                                            }
                                        }

                                        // Show token info
                                        if let Some(meta) = json.get("meta") {
                                            println!("\n  {}", "Token Info:".cyan());
                                            if let Some(base) = meta.get("base") {
                                                println!("    Base:  {} ({})", 
                                                    base.get("symbol").and_then(|s| s.as_str()).unwrap_or("?"),
                                                    base.get("name").and_then(|n| n.as_str()).unwrap_or("?")
                                                );
                                            }
                                            if let Some(quote) = meta.get("quote") {
                                                println!("    Quote: {} ({})", 
                                                    quote.get("symbol").and_then(|s| s.as_str()).unwrap_or("?"),
                                                    quote.get("name").and_then(|n| n.as_str()).unwrap_or("?")
                                                );
                                            }
                                        }
                                    }

                                    // Validate OHLCV data
                                    validate_ohlcv_structure(ohlcv_list);
                                } else {
                                    println!("  {} No ohlcv_list in response", "⚠️".yellow());
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
            for (key, val) in map.iter().take(15) {
                let type_str = match val {
                    Value::Null => "null",
                    Value::Bool(_) => "bool",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                };
                
                let preview = match val {
                    Value::String(s) => format!(": \"{}\"", s.chars().take(40).collect::<String>()),
                    Value::Number(n) => format!(": {}", n),
                    Value::Bool(b) => format!(": {}", b),
                    Value::Array(a) => format!(" [{}]", a.len()),
                    Value::Object(o) => format!(" {{{} fields}}", o.len()),
                    Value::Null => String::new(),
                };

                println!("{}  {} ({}){}", prefix, key.cyan(), type_str.bright_black(), preview);
            }
            
            if map.len() > 15 {
                println!("{}  ... {} more fields", prefix, map.len() - 15);
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
        "attributes", "id", "type", "relationships"
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

    // Validate attributes
    if let Some(attrs) = pool.get("attributes") {
        let attr_fields = vec![
            "address", "name", "base_token_price_usd", "pool_created_at"
        ];
        let mut attr_present = 0;
        for field in &attr_fields {
            if attrs.get(field).is_some() {
                attr_present += 1;
            }
        }
        println!("    {} {}/{}", "Attributes:".cyan(), attr_present.to_string().green(), attr_fields.len());
    }
}

fn validate_ohlcv_structure(ohlcv_list: &[Value]) {
    println!("\n  {}", "OHLCV Validation:".cyan().bold());
    
    if ohlcv_list.is_empty() {
        println!("    {} No candles in response", "⚠️".yellow());
        return;
    }

    // Check first candle structure
    if let Some(first) = ohlcv_list.first() {
        if let Some(arr) = first.as_array() {
            if arr.len() == 6 {
                println!("    {} Candle format correct [6 elements]", "✓".green());
                
                // Validate each element is a number
                let all_numbers = arr.iter().all(|v| v.is_number());
                if all_numbers {
                    println!("    {} All elements are numbers", "✓".green());
                } else {
                    println!("    {} Some elements are not numbers", "⚠️".yellow());
                }
            } else {
                println!("    {} Invalid candle format: expected 6 elements, got {}", "❌".red(), arr.len());
            }
        }
    }

    // Check for monotonic timestamps
    let mut prev_ts = 0i64;
    let mut monotonic = true;
    for candle in ohlcv_list {
        if let Some(arr) = candle.as_array() {
            if let Some(ts) = arr.get(0).and_then(|v| v.as_f64()) {
                let ts_i64 = ts as i64;
                if ts_i64 >= prev_ts {
                    prev_ts = ts_i64;
                } else {
                    monotonic = false;
                    break;
                }
            }
        }
    }

    if monotonic {
        println!("    {} Timestamps are in order", "✓".green());
    } else {
        println!("    {} Timestamps are not monotonic", "⚠️".yellow());
    }
}
