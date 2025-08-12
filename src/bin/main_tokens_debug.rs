/// Enhanced Token Database Debug Tool with API & Pool Integration
/// 
/// Comprehensive command-line tool for token analysis, debugging, and testing.
/// Integrates database queries, decimals cache, API operations, and pool price monitoring.
/// Essential for debugging token-related issues and validating price feeds.
/// Provides real-time insights into token performance and system health.
/// 
/// Core Features:
/// - Database Operations: Search, analyze, and debug token data from local database
/// - Decimals Integration: Fetch, cache, and convert token decimal information
/// - API Testing: Test DexScreener API endpoints and validate token data retrieval
/// - Pool Analysis: Monitor pool prices, analyze liquidity, and validate price calculations
/// - Price Validation: Compare prices across different sources and validate accuracy
/// 
/// Usage Examples:
/// - Search by mint: cargo run --bin main_tokens_debug -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D
/// - Search by symbol: cargo run --bin main_tokens_debug -- --symbol PSAF
/// - Search by name: cargo run --bin main_tokens_debug -- --name "Alpha Fund"
/// - List all tokens: cargo run --bin main_tokens_debug -- --list
/// - Count tokens: cargo run --bin main_tokens_debug -- --count
/// - Recent tokens: cargo run --bin main_tokens_debug -- --recent 10
/// - Get decimals: cargo run --bin main_tokens_debug -- --decimals TOKEN_MINT
/// - Batch decimals: cargo run --bin main_tokens_debug -- --batch-decimals MINT1,MINT2,MINT3
/// - Cache stats: cargo run --bin main_tokens_debug -- --cache-stats
/// - Convert amounts: cargo run --bin main_tokens_debug -- --convert-raw 1000000 --decimals-for MINT
/// - API price test: cargo run --bin main_tokens_debug -- --api-price TOKEN_MINT
/// - Pool price test: cargo run --bin main_tokens_debug -- --pool-price TOKEN_MINT
/// - Price comparison: cargo run --bin main_tokens_debug -- --compare-prices TOKEN_MINT
/// - API stats: cargo run --bin main_tokens_debug -- --api-stats
/// - Pool monitoring: cargo run --bin main_tokens_debug -- --monitor-pools --duration 300
/// - Debug decimals: cargo run --bin main_tokens_debug -- --debug-decimals

use clap::{Arg, Command};
use std::path::Path;
use rusqlite::Connection;

// Import screenerbot modules for decimals functionality
extern crate screenerbot;
use screenerbot::tokens::decimals::{
    get_token_decimals_from_chain,
    batch_fetch_token_decimals,
    get_cached_decimals,
    get_cache_stats,
    save_decimal_cache,
    raw_to_ui_amount,
    ui_to_raw_amount,
    lamports_to_sol,
    sol_to_lamports,
};
use screenerbot::tokens::api::{
    get_token_price_from_global_api,
    get_token_from_mint_global_api,
    get_token_pairs_from_api,
};
use screenerbot::tokens::pool::{
    get_pool_program_display_name,
};

#[derive(Debug)]
struct TokenInfo {
    mint: String,
    symbol: String,
    name: String,
    price_usd: Option<f64>,
    price_sol: Option<f64>,
    liquidity_usd: Option<f64>,
    last_updated: Option<String>,
    decimals: Option<u8>, // Add decimals field
}

#[tokio::main]
async fn main() {
    let matches = Command::new("Token Database Debug Tool")
        .version("1.0")
        .about("Query tokens database for debugging")
        .arg(Arg::new("mint")
            .long("mint")
            .value_name("MINT")
            .help("Search by token mint address"))
        .arg(Arg::new("symbol")
            .long("symbol")
            .value_name("SYMBOL")
            .help("Search by token symbol"))
        .arg(Arg::new("name")
            .long("name")
            .value_name("NAME")
            .help("Search by token name (partial match)"))
        .arg(Arg::new("list")
            .long("list")
            .help("List all tokens")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("count")
            .long("count")
            .help("Count total tokens")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("recent")
            .long("recent")
            .value_name("N")
            .help("Show N most recently added tokens"))
        .arg(Arg::new("decimals")
            .long("decimals")
            .value_name("MINT")
            .help("Get decimals for a specific token mint"))
        .arg(Arg::new("batch-decimals")
            .long("batch-decimals")
            .value_name("MINTS")
            .help("Get decimals for multiple tokens (comma-separated)"))
        .arg(Arg::new("cache-stats")
            .long("cache-stats")
            .help("Show decimals cache statistics")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("save-cache")
            .long("save-cache")
            .help("Save decimals cache to disk")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("convert-raw")
            .long("convert-raw")
            .value_name("AMOUNT")
            .help("Convert raw amount to UI amount (requires --decimals-for)"))
        .arg(Arg::new("convert-ui")
            .long("convert-ui")
            .value_name("AMOUNT")
            .help("Convert UI amount to raw amount (requires --decimals-for)"))
        .arg(Arg::new("decimals-for")
            .long("decimals-for")
            .value_name("MINT")
            .help("Token mint for conversion operations"))
        .arg(Arg::new("debug-decimals")
            .long("debug-decimals")
            .help("Enable debug logging for decimals operations")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("api-price")
            .long("api-price")
            .value_name("MINT")
            .help("Test API price retrieval for a specific token"))
        .arg(Arg::new("pool-price")
            .long("pool-price")
            .value_name("MINT")
            .help("Test pool price calculation for a specific token"))
        .arg(Arg::new("compare-prices")
            .long("compare-prices")
            .value_name("MINT")
            .help("Compare prices from different sources (API vs Pool)"))
        .arg(Arg::new("api-stats")
            .long("api-stats")
            .help("Show DexScreener API statistics")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("monitor-pools")
            .long("monitor-pools")
            .help("Monitor pool prices in real-time")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("duration")
            .long("duration")
            .value_name("SECONDS")
            .help("Duration for monitoring operations (default: 60 seconds)"))
        .arg(Arg::new("test-api")
            .long("test-api")
            .value_name("MINT")
            .help("Comprehensive API testing for a token"))
        .arg(Arg::new("pool-info")
            .long("pool-info")
            .value_name("MINT")
            .help("Get detailed pool information for a token"))
        .get_matches();

    let db_path = "data/tokens.db";
    
    if !Path::new(db_path).exists() {
        eprintln!("❌ Token database not found at: {}", db_path);
        eprintln!("Make sure you're running from the project root directory.");
        return;
    }

    let conn = match Connection::open(db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            return;
        }
    };

    // Execute based on command line arguments
    if let Some(mint) = matches.get_one::<String>("mint") {
        search_by_mint(&conn, mint).await;
    } else if let Some(symbol) = matches.get_one::<String>("symbol") {
        search_by_symbol(&conn, symbol).await;
    } else if let Some(name) = matches.get_one::<String>("name") {
        search_by_name(&conn, name).await;
    } else if matches.get_flag("list") {
        list_all_tokens(&conn).await;
    } else if matches.get_flag("count") {
        count_tokens(&conn).await;
    } else if let Some(n) = matches.get_one::<String>("recent") {
        let count: usize = n.parse().unwrap_or(10);
        show_recent_tokens(&conn, count).await;
    } else if let Some(mint) = matches.get_one::<String>("decimals") {
        get_token_decimals(mint).await;
    } else if let Some(mints) = matches.get_one::<String>("batch-decimals") {
        batch_get_token_decimals(mints).await;
    } else if matches.get_flag("cache-stats") {
        show_cache_stats().await;
    } else if matches.get_flag("save-cache") {
        save_cache().await;
    } else if let Some(amount) = matches.get_one::<String>("convert-raw") {
        if let Some(mint) = matches.get_one::<String>("decimals-for") {
            convert_raw_amount(amount, mint).await;
        } else {
            eprintln!("❌ --convert-raw requires --decimals-for MINT");
        }
    } else if let Some(amount) = matches.get_one::<String>("convert-ui") {
        if let Some(mint) = matches.get_one::<String>("decimals-for") {
            convert_ui_amount(amount, mint).await;
        } else {
            eprintln!("❌ --convert-ui requires --decimals-for MINT");
        }
    } else if let Some(mint) = matches.get_one::<String>("api-price") {
        test_api_price(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("pool-price") {
        test_pool_price(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("compare-prices") {
        compare_token_prices(mint).await;
    } else if matches.get_flag("api-stats") {
        show_api_stats().await;
    } else if matches.get_flag("monitor-pools") {
        let duration = matches.get_one::<String>("duration")
            .and_then(|d| d.parse().ok())
            .unwrap_or(60);
        monitor_pool_prices(duration).await;
    } else if let Some(mint) = matches.get_one::<String>("test-api") {
        comprehensive_api_test(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("pool-info") {
        get_pool_information(mint).await;
    } else {
        show_help();
    }
}

async fn search_by_mint(conn: &Connection, mint: &str) {
    println!("🔍 Searching for token with mint: {}", mint);
    
    let mut stmt = match conn.prepare(
        "SELECT mint, symbol, name, price_usd, price_sol, liquidity_usd, last_updated FROM tokens WHERE mint = ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    let token_iter = stmt.query_map([mint], |row| {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            price_usd: row.get(3).ok(),
            price_sol: row.get(4).ok(),
            liquidity_usd: row.get(5).ok(),
            last_updated: row.get(6).ok(),
            decimals: None, // Will be populated later if needed
        })
    });

    match token_iter {
        Ok(tokens) => {
            let mut found = false;
            for token in tokens {
                if let Ok(token) = token {
                    display_token(&token);
                    found = true;
                }
            }
            if !found {
                println!("❌ No token found with mint: {}", mint);
            }
        }
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

async fn search_by_symbol(conn: &Connection, symbol: &str) {
    println!("🔍 Searching for tokens with symbol: {}", symbol);
    
    let mut stmt = match conn.prepare(
        "SELECT mint, symbol, name, price_usd, price_sol, liquidity_usd, last_updated FROM tokens WHERE symbol LIKE ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    let search_pattern = format!("%{}%", symbol);
    let token_iter = stmt.query_map([search_pattern], |row| {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            price_usd: row.get(3).ok(),
            price_sol: row.get(4).ok(),
            liquidity_usd: row.get(5).ok(),
            last_updated: row.get(6).ok(),
            decimals: None, // Will be populated later if needed
        })
    });

    match token_iter {
        Ok(tokens) => {
            let mut count = 0;
            for token in tokens {
                if let Ok(token) = token {
                    display_token(&token);
                    count += 1;
                }
            }
            if count == 0 {
                println!("❌ No tokens found with symbol containing: {}", symbol);
            } else {
                println!("\n📊 Found {} token(s)", count);
            }
        }
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

async fn search_by_name(conn: &Connection, name: &str) {
    println!("🔍 Searching for tokens with name containing: {}", name);
    
    let mut stmt = match conn.prepare(
        "SELECT mint, symbol, name, price_usd, price_sol, liquidity_usd, last_updated FROM tokens WHERE name LIKE ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    let search_pattern = format!("%{}%", name);
    let token_iter = stmt.query_map([search_pattern], |row| {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            price_usd: row.get(3).ok(),
            price_sol: row.get(4).ok(),
            liquidity_usd: row.get(5).ok(),
            last_updated: row.get(6).ok(),
            decimals: None, // Will be populated later if needed
        })
    });

    match token_iter {
        Ok(tokens) => {
            let mut count = 0;
            for token in tokens {
                if let Ok(token) = token {
                    display_token(&token);
                    count += 1;
                }
            }
            if count == 0 {
                println!("❌ No tokens found with name containing: {}", name);
            } else {
                println!("\n📊 Found {} token(s)", count);
            }
        }
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

async fn list_all_tokens(conn: &Connection) {
    println!("📋 Listing all tokens in database...");
    
    let mut stmt = match conn.prepare(
        "SELECT mint, symbol, name, price_usd, price_sol, liquidity_usd, last_updated FROM tokens ORDER BY symbol"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    let token_iter = stmt.query_map([], |row| {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            price_usd: row.get(3).ok(),
            price_sol: row.get(4).ok(),
            liquidity_usd: row.get(5).ok(),
            last_updated: row.get(6).ok(),
            decimals: None, // Will be populated later if needed
        })
    });

    match token_iter {
        Ok(tokens) => {
            let mut count = 0;
            for token in tokens {
                if let Ok(token) = token {
                    display_token_compact(&token);
                    count += 1;
                }
            }
            println!("\n📊 Total: {} tokens", count);
        }
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

async fn count_tokens(conn: &Connection) {
    let mut stmt = match conn.prepare("SELECT COUNT(*) FROM tokens") {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    match stmt.query_row([], |row| {
        let count: i64 = row.get(0)?;
        Ok(count)
    }) {
        Ok(count) => println!("📊 Total tokens in database: {}", count),
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

async fn show_recent_tokens(conn: &Connection, count: usize) {
    println!("🕒 Showing {} most recently added tokens...", count);
    
    let mut stmt = match conn.prepare(
        "SELECT mint, symbol, name, price_usd, price_sol, liquidity_usd, last_updated FROM tokens ORDER BY last_updated DESC LIMIT ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("❌ Failed to prepare statement: {}", e);
            return;
        }
    };

    let token_iter = stmt.query_map([count], |row| {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            price_usd: row.get(3).ok(),
            price_sol: row.get(4).ok(),
            liquidity_usd: row.get(5).ok(),
            last_updated: row.get(6).ok(),
            decimals: None, // Will be populated later if needed
        })
    });

    match token_iter {
        Ok(tokens) => {
            let mut found_count = 0;
            for token in tokens {
                if let Ok(token) = token {
                    display_token(&token);
                    found_count += 1;
                }
            }
            if found_count == 0 {
                println!("❌ No tokens found");
            } else {
                println!("\n📊 Showed {} recent token(s)", found_count);
            }
        }
        Err(e) => eprintln!("❌ Query failed: {}", e),
    }
}

fn display_token(token: &TokenInfo) {
    println!("┌─────────────────────────────────────────────────────────────");
    println!("│ 🪙 Token: {}", token.symbol);
    println!("│ 📛 Name: {}", token.name);
    println!("│ 🔑 Mint: {}", token.mint);
    if let Some(price_usd) = token.price_usd {
        println!("│ 💵 Price USD: ${:.6}", price_usd);
    }
    if let Some(price_sol) = token.price_sol {
        println!("│ ◎ Price SOL: {:.9}", price_sol);
    }
    if let Some(liquidity_usd) = token.liquidity_usd {
        println!("│ 💧 Liquidity: ${:.2}", liquidity_usd);
    }
    if let Some(updated) = &token.last_updated {
        println!("│ 🕐 Last Updated: {}", updated);
    }
    
    // Try to get decimals info
    if let Some(cached_decimals) = get_cached_decimals(&token.mint) {
        println!("│ 🔢 Decimals: {} (cached)", cached_decimals);
    } else {
        println!("│ 🔢 Decimals: Not cached (use --decimals {} to fetch)", token.mint);
    }
    
    println!("└─────────────────────────────────────────────────────────────");
}

fn display_token_compact(token: &TokenInfo) {
    let price_str = token.price_usd.map_or("N/A".to_string(), |p| format!("${:.6}", p));
    println!("{:8} | {:30} | {}", token.symbol, &token.name[..30.min(token.name.len())], price_str);
}

fn show_help() {
    println!("🔧 Enhanced Token Database Debug Tool with API & Pool Integration");
    println!();
    println!("Database Operations:");
    println!("  cargo run --bin main_tokens_debug -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D");
    println!("  cargo run --bin main_tokens_debug -- --symbol PSAF");
    println!("  cargo run --bin main_tokens_debug -- --name 'Alpha Fund'");
    println!("  cargo run --bin main_tokens_debug -- --list");
    println!("  cargo run --bin main_tokens_debug -- --count");
    println!("  cargo run --bin main_tokens_debug -- --recent 10");
    println!();
    println!("Decimals Operations:");
    println!("  cargo run --bin main_tokens_debug -- --decimals TOKEN_MINT");
    println!("  cargo run --bin main_tokens_debug -- --batch-decimals MINT1,MINT2,MINT3");
    println!("  cargo run --bin main_tokens_debug -- --cache-stats");
    println!("  cargo run --bin main_tokens_debug -- --save-cache");
    println!();
    println!("API Operations:");
    println!("  cargo run --bin main_tokens_debug -- --api-price TOKEN_MINT");
    println!("  cargo run --bin main_tokens_debug -- --test-api TOKEN_MINT");
    println!("  cargo run --bin main_tokens_debug -- --api-stats");
    println!();
    println!("Pool Operations:");
    println!("  cargo run --bin main_tokens_debug -- --pool-price TOKEN_MINT");
    println!("  cargo run --bin main_tokens_debug -- --pool-info TOKEN_MINT");
    println!("  cargo run --bin main_tokens_debug -- --monitor-pools --duration 300");
    println!();
    println!("Price Analysis:");
    println!("  cargo run --bin main_tokens_debug -- --compare-prices TOKEN_MINT");
    println!();
    println!("Conversion Utilities:");
    println!("  cargo run --bin main_tokens_debug -- --convert-raw 1000000 --decimals-for MINT");
    println!("  cargo run --bin main_tokens_debug -- --convert-ui 1.5 --decimals-for MINT");
    println!();
    println!("Debug Options:");
    println!("  cargo run --bin main_tokens_debug -- --debug-decimals");
}

// Decimals operation functions
async fn get_token_decimals(mint: &str) {
    println!("🔍 Getting decimals for token: {}", mint);
    
    // First check cache
    if let Some(cached_decimals) = get_cached_decimals(mint) {
        println!("✅ Found in cache: {} decimals", cached_decimals);
        return;
    }
    
    // Fetch from chain
    match get_token_decimals_from_chain(mint).await {
        Ok(decimals) => {
            println!("✅ Fetched from chain: {} decimals", decimals);
            println!("💾 Value cached for future use");
        }
        Err(e) => println!("❌ Failed to get decimals: {}", e),
    }
}

async fn batch_get_token_decimals(mints_str: &str) {
    let mints: Vec<String> = mints_str.split(',').map(|s| s.trim().to_string()).collect();
    println!("🔍 Getting decimals for {} tokens...", mints.len());
    
    let results = batch_fetch_token_decimals(&mints).await;
    
    for (mint, result) in results {
        match result {
            Ok(decimals) => {
                println!("✅ {}: {} decimals", mint, decimals);
            }
            Err(e) => {
                println!("❌ {}: {}", mint, e);
            }
        }
    }
}

async fn show_cache_stats() {
    println!("📊 Decimals Cache Statistics");
    println!("─────────────────────────────");
    
    let (cached_count, failed_count) = get_cache_stats();
    println!("🟢 Cached decimals: {}", cached_count);
    println!("🔴 Failed attempts: {}", failed_count);
    println!("📈 Total entries: {}", cached_count + failed_count);
    
    if cached_count + failed_count > 0 {
        let success_rate = (cached_count as f64 / (cached_count + failed_count) as f64) * 100.0;
        println!("✅ Success rate: {:.1}%", success_rate);
    }
}

async fn save_cache() {
    println!("💾 Saving decimals cache to disk...");
    save_decimal_cache();
    println!("✅ Decimals cache saved successfully");
}

async fn convert_raw_amount(amount_str: &str, mint: &str) {
    let raw_amount: u64 = match amount_str.parse() {
        Ok(amount) => amount,
        Err(_) => {
            println!("❌ Invalid raw amount: {}", amount_str);
            return;
        }
    };
    
    // Get decimals for the token
    let decimals = if let Some(cached) = get_cached_decimals(mint) {
        cached
    } else {
        match get_token_decimals_from_chain(mint).await {
            Ok(decimals) => decimals,
            Err(e) => {
                println!("❌ Failed to get decimals for {}: {}", mint, e);
                return;
            }
        }
    };
    
    let ui_amount = raw_to_ui_amount(raw_amount, decimals);
    println!("🔄 Conversion Result:");
    println!("   Raw Amount: {}", raw_amount);
    println!("   Decimals: {}", decimals);
    println!("   UI Amount: {}", ui_amount);
    
    // Special handling for SOL
    if mint == "So11111111111111111111111111111111111111112" {
        let sol_amount = lamports_to_sol(raw_amount);
        println!("   SOL Amount: {} SOL", sol_amount);
    }
}

async fn convert_ui_amount(amount_str: &str, mint: &str) {
    let ui_amount: f64 = match amount_str.parse() {
        Ok(amount) => amount,
        Err(_) => {
            println!("❌ Invalid UI amount: {}", amount_str);
            return;
        }
    };
    
    // Get decimals for the token
    let decimals = if let Some(cached) = get_cached_decimals(mint) {
        cached
    } else {
        match get_token_decimals_from_chain(mint).await {
            Ok(decimals) => decimals,
            Err(e) => {
                println!("❌ Failed to get decimals for {}: {}", mint, e);
                return;
            }
        }
    };
    
    let raw_amount = ui_to_raw_amount(ui_amount, decimals);
    println!("🔄 Conversion Result:");
    println!("   UI Amount: {}", ui_amount);
    println!("   Decimals: {}", decimals);
    println!("   Raw Amount: {}", raw_amount);
    
    // Special handling for SOL
    if mint == "So11111111111111111111111111111111111111112" {
        let lamports = sol_to_lamports(ui_amount);
        println!("   Lamports: {}", lamports);
    }
}

// API operation functions
async fn test_api_price(mint: &str) {
    println!("🌐 Testing API price retrieval for: {}", mint);
    
    match get_token_price_from_global_api(mint).await {
        Some(price) => {
            println!("✅ API Price: ${:.9}", price);
        }
        None => {
            println!("❌ Failed to get price from API");
        }
    }
}

async fn comprehensive_api_test(mint: &str) {
    println!("🧪 Comprehensive API test for token: {}", mint);
    println!("─────────────────────────────────────");
    
    // Test token data retrieval
    println!("📊 Testing token data retrieval...");
    match get_token_from_mint_global_api(mint).await {
        Ok(Some(token)) => {
            println!("✅ Token data retrieved successfully:");
            println!("   Symbol: {}", token.symbol);
            println!("   Name: {}", token.name);
            println!("   Chain: {}", token.chain);
            if let Some(logo) = &token.logo_url {
                println!("   Logo URL: {}", logo);
            }
            if let Some(website) = &token.website {
                println!("   Website: {}", website);
            }
            if let Some(price_usd) = token.price_dexscreener_usd {
                println!("   Price USD: ${:.9}", price_usd);
            }
            if let Some(price_sol) = token.price_dexscreener_sol {
                println!("   Price SOL: {:.9}", price_sol);
            }
            if let Some(market_cap) = token.market_cap {
                println!("   Market Cap: ${:.2}", market_cap);
            }
        }
        Ok(None) => {
            println!("❌ Token not found in API");
        }
        Err(e) => {
            println!("❌ API error: {}", e);
        }
    }
    
    // Test token pairs
    println!("\n🔗 Testing token pairs retrieval...");
    match get_token_pairs_from_api(mint).await {
        Ok(pairs) => {
            println!("✅ Found {} trading pairs:", pairs.len());
            for (i, pair) in pairs.iter().take(5).enumerate() {
                let price_display = pair.price_usd.as_ref()
                    .map(|p| p.clone())
                    .unwrap_or_else(|| "N/A".to_string());
                println!("   {}. {}/{} on {} - Price: ${}", 
                    i + 1, pair.base_token.symbol, pair.quote_token.symbol, 
                    pair.dex_id, price_display);
            }
            if pairs.len() > 5 {
                println!("   ... and {} more pairs", pairs.len() - 5);
            }
        }
        Err(e) => {
            println!("❌ Failed to get pairs: {}", e);
        }
    }
}

async fn show_api_stats() {
    println!("📊 DexScreener API Statistics");
    println!("─────────────────────────────");
    
    // Note: This would need to be implemented in the API module
    // For now, we'll show a placeholder
    println!("🔄 API statistics feature coming soon...");
    println!("💡 This will show request counts, rate limits, and success rates");
}

// Pool operation functions
async fn test_pool_price(mint: &str) {
    println!("🏊 Testing pool price calculation for: {}", mint);
    
    // This would need to be implemented using the PoolPriceManager
    println!("🔄 Pool price testing feature coming soon...");
    println!("💡 This will calculate prices from pool reserves");
}

async fn get_pool_information(mint: &str) {
    println!("📊 Pool information for token: {}", mint);
    println!("─────────────────────────────────────");
    
    // First get token pairs to find pools
    match get_token_pairs_from_api(mint).await {
        Ok(pairs) => {
            println!("🔍 Found {} pools:", pairs.len());
            for (i, pair) in pairs.iter().enumerate() {
                println!("\n   Pool {}:", i + 1);
                println!("   Pair: {}/{}", pair.base_token.symbol, pair.quote_token.symbol);
                println!("   DEX: {}", pair.dex_id);
                println!("   Program: {}", get_pool_program_display_name(&pair.pair_address));
                
                let price_display = pair.price_usd.as_ref()
                    .map(|p| p.clone())
                    .unwrap_or_else(|| "N/A".to_string());
                println!("   Price USD: ${}", price_display);
                
                if let Some(liquidity) = &pair.liquidity {
                    println!("   Liquidity USD: ${:.2}", liquidity.usd);
                    println!("   Base Liquidity: {:.2}", liquidity.base);
                    println!("   Quote Liquidity: {:.2}", liquidity.quote);
                }
                
                if let Some(h24) = pair.volume.h24 {
                    println!("   24h Volume: ${:.2}", h24);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get pool information: {}", e);
        }
    }
}

async fn compare_token_prices(mint: &str) {
    println!("⚖️  Comparing prices for token: {}", mint);
    println!("─────────────────────────────────────");
    
    // Get API price
    println!("🌐 Fetching API price...");
    let api_price = get_token_price_from_global_api(mint).await;
    
    // Get pool price (placeholder for now)
    println!("🏊 Calculating pool price...");
    let pool_price: Option<f64> = None; // TODO: Implement pool price calculation
    
    // Compare results
    println!("\n📊 Price Comparison Results:");
    match api_price {
        Some(price) => println!("   API Price:  ${:.9}", price),
        None => println!("   API Price:  ❌ Not available"),
    }
    
    match pool_price {
        Some(price) => println!("   Pool Price: ${:.9}", price),
        None => println!("   Pool Price: 🔄 Coming soon"),
    }
    
    if let Some(api) = api_price {
        if let Some(pool) = pool_price {
            let difference = ((api - pool) / api * 100.0).abs();
            println!("   Difference: {:.2}%", difference);
            
            if difference > 5.0 {
                println!("   ⚠️  Warning: Large price difference detected!");
            } else if difference > 1.0 {
                println!("   ⚠️  Moderate price difference");
            } else {
                println!("   ✅ Prices are closely aligned");
            }
        }
    }
}

async fn monitor_pool_prices(duration_seconds: u64) {
    println!("👀 Monitoring pool prices for {} seconds...", duration_seconds);
    println!("─────────────────────────────────────");
    
    // This would implement real-time pool price monitoring
    println!("🔄 Pool price monitoring feature coming soon...");
    println!("💡 This will show real-time price updates from pools");
    
    // Placeholder countdown
    for i in (1..=duration_seconds.min(10)).rev() {
        println!("⏱️  Monitoring simulation: {} seconds remaining", i);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    println!("✅ Monitoring simulation completed");
}
