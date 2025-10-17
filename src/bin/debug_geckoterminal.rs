/// Comprehensive GeckoTerminal API debug tool
/// 
/// Tests ALL available GeckoTerminal endpoints and validates implementation

use clap::Parser;
use colored::Colorize;
use reqwest::Client;
use serde_json::Value;
use std::time::Instant;

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

    /// Test OHLCV endpoint
    #[clap(long)]
    ohlcv: bool,

    /// Test trending pools endpoint
    #[clap(long)]
    trending: bool,

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

    let test_all = args.all || (!args.token_pools && !args.ohlcv && !args.trending);

    if test_all || args.token_pools {
        test_token_pools(&client, &args).await;
    }

    if test_all || args.trending {
        test_trending_pools(&client, &args).await;
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
