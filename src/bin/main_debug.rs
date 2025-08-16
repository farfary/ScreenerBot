#![allow(warnings)]

/// Enhanced ScreenerBot Debug Tool - Comprehensive Token & Wallet Analysis
/// 
/// Master debugging tool for the ScreenerBot trading system. Provides comprehensive 
/// analysis of tokens, wallets, transactions, positions, and trading performance.
/// Integrates database queries, real-time balances, transaction history, and P&L analysis.
/// Essential for debugging trading issues, investigating positions, and monitoring system health.
/// 
/// Core Features:
/// - Token Database: Search, analyze, and debug token data from local database
/// - Wallet Analysis: Real-time SOL and token balance checking with ATA inspection
/// - Transaction History: Complete transaction analysis from the transaction manager
/// - Position Analysis: P&L calculations and position lifecycle tracking from actual swaps
/// - Trading Performance: ROI analysis, fee tracking, and profit/loss calculations
/// - Decimals Integration: Fetch, cache, and convert token decimal information
/// - API Testing: Test DexScreener API endpoints and validate token data retrieval
/// - Pool Analysis: Monitor pool prices, analyze liquidity, and validate price calculations
/// - Price Validation: Compare prices across different sources and validate accuracy
/// 
/// Usage Examples:
/// - Search by mint: cargo run --bin main_debug -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D
/// - Comprehensive analysis: cargo run --bin main_debug -- --wallet-analysis TOKEN_MINT
/// - Balance check: cargo run --bin main_debug -- --balance-check TOKEN_MINT
/// - Transaction history: cargo run --bin main_debug -- --transaction-history TOKEN_MINT
/// - Position analysis: cargo run --bin main_debug -- --position-analysis TOKEN_MINT
/// - Wallet summary: cargo run --bin main_debug -- --wallet-summary
/// - Search by symbol: cargo run --bin main_debug -- --symbol PSAF
/// - Search by name: cargo run --bin main_debug -- --name "Alpha Fund"
/// - List all tokens: cargo run --bin main_debug -- --list
/// - Count tokens: cargo run --bin main_debug -- --count
/// - Recent tokens: cargo run --bin main_debug -- --recent 10
/// - Get decimals: cargo run --bin main_debug -- --decimals TOKEN_MINT
/// - Batch decimals: cargo run --bin main_debug -- --batch-decimals MINT1,MINT2,MINT3
/// - Cache stats: cargo run --bin main_debug -- --cache-stats
/// - Convert amounts: cargo run --bin main_debug -- --convert-raw 1000000 --decimals-for MINT
/// - API price test: cargo run --bin main_debug -- --api-price TOKEN_MINT
/// - Pool price test: cargo run --bin main_debug -- --pool-price TOKEN_MINT
/// - Price comparison: cargo run --bin main_debug -- --compare-prices TOKEN_MINT
/// - API stats: cargo run --bin main_debug -- --api-stats
/// - Pool monitoring: cargo run --bin main_debug -- --monitor-pools --duration 300
/// - Debug decimals: cargo run --bin main_debug -- --debug-decimals

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
use screenerbot::tokens::dexscreener::{
    get_token_price_from_global_api,
    get_token_from_mint_global_api,
    get_token_pairs_from_api,
};
use screenerbot::tokens::pool::{
    get_pool_program_display_name,
};
use screenerbot::transactions::{
    TransactionsManager,
    Transaction,
    TransactionType,
    SwapPnLInfo,
};
use screenerbot::utils::{
    get_sol_balance,
    get_token_balance,
    get_all_token_accounts,
    get_wallet_address,
};
use screenerbot::global::{read_configs, load_wallet_from_config};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;

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
    let matches = Command::new("ScreenerBot Master Debug Tool")
        .version("1.0")
        .about("Comprehensive token, wallet, and trading analysis for ScreenerBot")
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
        .arg(Arg::new("wallet-analysis")
            .long("wallet-analysis")
            .value_name("MINT")
            .help("Comprehensive wallet analysis for a specific token"))
        .arg(Arg::new("balance-check")
            .long("balance-check")
            .value_name("MINT")
            .help("Check current wallet and ATA balances for a token"))
        .arg(Arg::new("transaction-history")
            .long("transaction-history")
            .value_name("MINT")
            .help("Show transaction history for a specific token"))
        .arg(Arg::new("position-analysis")
            .long("position-analysis")
            .value_name("MINT")
            .help("Analyze positions and trades for a specific token from transactions"))
        .arg(Arg::new("wallet-summary")
            .long("wallet-summary")
            .help("Show overall wallet summary with SOL and all token balances")
            .action(clap::ArgAction::SetTrue))
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
    } else if let Some(mint) = matches.get_one::<String>("wallet-analysis") {
        comprehensive_wallet_token_analysis(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("balance-check") {
        check_token_balances(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("transaction-history") {
        show_token_transaction_history(mint).await;
    } else if let Some(mint) = matches.get_one::<String>("position-analysis") {
        analyze_token_positions(mint).await;
    } else if matches.get_flag("wallet-summary") {
        show_wallet_summary().await;
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
    println!("🔧 ScreenerBot Master Debug Tool - Comprehensive Token, Wallet & Trading Analysis");
    println!();
    println!("Database Operations:");
    println!("  cargo run --bin main_debug -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D");
    println!("  cargo run --bin main_debug -- --symbol PSAF");
    println!("  cargo run --bin main_debug -- --name 'Alpha Fund'");
    println!("  cargo run --bin main_debug -- --list");
    println!("  cargo run --bin main_debug -- --count");
    println!("  cargo run --bin main_debug -- --recent 10");
    println!();
    println!("Wallet & Balance Analysis:");
    println!("  cargo run --bin main_debug -- --balance-check TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --wallet-summary");
    println!();
    println!("Transaction & Position Analysis:");
    println!("  cargo run --bin main_debug -- --transaction-history TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --position-analysis TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --wallet-analysis TOKEN_MINT  # Comprehensive analysis");
    println!();
    println!("Decimals Operations:");
    println!("  cargo run --bin main_debug -- --decimals TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --batch-decimals MINT1,MINT2,MINT3");
    println!("  cargo run --bin main_debug -- --cache-stats");
    println!("  cargo run --bin main_debug -- --save-cache");
    println!();
    println!("API Operations:");
    println!("  cargo run --bin main_debug -- --api-price TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --test-api TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --api-stats");
    println!();
    println!("Pool Operations:");
    println!("  cargo run --bin main_debug -- --pool-price TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --pool-info TOKEN_MINT");
    println!("  cargo run --bin main_debug -- --monitor-pools --duration 300");
    println!();
    println!("Price Analysis:");
    println!("  cargo run --bin main_debug -- --compare-prices TOKEN_MINT");
    println!();
    println!("Conversion Utilities:");
    println!("  cargo run --bin main_debug -- --convert-raw 1000000 --decimals-for MINT");
    println!("  cargo run --bin main_debug -- --convert-ui 1.5 --decimals-for MINT");
    println!();
    println!("Debug Options:");
    println!("  cargo run --bin main_debug -- --debug-decimals");
    println!();
    println!("🆕 ENHANCED FEATURES:");
    println!("• Comprehensive wallet analysis with balances, transactions, and positions");
    println!("• Real-time balance checking for SOL and specific tokens");
    println!("• Transaction history analysis from the transactions manager");
    println!("• Position analysis and P&L calculations from actual swaps");
    println!("• Complete wallet summary with all token holdings");
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

// =============================================================================
// ENHANCED WALLET AND TOKEN ANALYSIS FUNCTIONS
// =============================================================================

/// Comprehensive wallet analysis for a specific token
async fn comprehensive_wallet_token_analysis(mint: &str) {
    println!("🔍 Comprehensive Token Analysis for: {}", mint);
    println!("════════════════════════════════════════════════════════════");
    
    // Step 1: Token Database Information
    println!("\n📊 1. DATABASE INFORMATION");
    println!("─────────────────────────────");
    
    let db_path = "data/tokens.db";
    if let Ok(conn) = Connection::open(db_path) {
        search_by_mint(&conn, mint).await;
    } else {
        println!("❌ Could not access token database");
    }
    
    // Step 2: Wallet Balances
    println!("\n💰 2. WALLET BALANCES");
    println!("─────────────────────────────");
    check_token_balances(mint).await;
    
    // Step 3: Transaction History
    println!("\n📜 3. TRANSACTION HISTORY");
    println!("─────────────────────────────");
    show_token_transaction_history(mint).await;
    
    // Step 4: Position Analysis
    println!("\n📈 4. POSITION ANALYSIS");
    println!("─────────────────────────────");
    analyze_token_positions(mint).await;
    
    // Step 5: Price Information
    println!("\n💵 5. PRICE INFORMATION");
    println!("─────────────────────────────");
    compare_token_prices(mint).await;
    
    // Step 6: Pool Information
    println!("\n🏊 6. POOL INFORMATION");
    println!("─────────────────────────────");
    get_pool_information(mint).await;
    
    println!("\n✅ Comprehensive analysis completed for token: {}", mint);
}

/// Check current wallet and ATA balances for a specific token
async fn check_token_balances(mint: &str) {
    println!("💰 Checking balances for token: {}", mint);
    
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return;
        }
    };
    
    println!("👛 Wallet: {}", wallet_address);
    
    // Get SOL balance
    match get_sol_balance(&wallet_address).await {
        Ok(sol_balance) => {
            println!("◎ SOL Balance: {:.6} SOL", sol_balance);
        }
        Err(e) => {
            println!("❌ Failed to get SOL balance: {}", e);
        }
    }
    
    // Get token balance  
    match get_token_balance(&wallet_address, mint).await {
        Ok(token_balance) => {
            // Get decimals to convert raw amount
            let decimals = if let Some(cached) = get_cached_decimals(mint) {
                cached
            } else {
                match get_token_decimals_from_chain(mint).await {
                    Ok(d) => d,
                    Err(_) => 6, // Default fallback
                }
            };
            
            let ui_amount = raw_to_ui_amount(token_balance, decimals);
            println!("🪙 Token Balance: {} raw units ({} UI amount)", token_balance, ui_amount);
            println!("🔢 Token Decimals: {}", decimals);
            
            if token_balance == 0 {
                println!("⚠️  No tokens found in wallet");
            }
        }
        Err(e) => {
            println!("❌ Failed to get token balance: {}", e);
        }
    }
    
    // Get all token accounts to check for ATAs
    match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => {
            let token_accounts: Vec<_> = accounts.iter()
                .filter(|acc| acc.mint == mint)
                .collect();
                
            if !token_accounts.is_empty() {
                println!("\n🔗 Associated Token Accounts:");
                for (i, account) in token_accounts.iter().enumerate() {
                    let ui_balance = if let Some(decimals) = get_cached_decimals(mint) {
                        raw_to_ui_amount(account.balance, decimals)
                    } else {
                        account.balance as f64
                    };
                    
                    println!("   {}. Account: {}", i + 1, account.account);
                    println!("      Balance: {} raw ({} UI)", account.balance, ui_balance);
                    println!("      Token 2022: {}", account.is_token_2022);
                }
            } else {
                println!("📭 No ATAs found for this token");
            }
        }
        Err(e) => {
            println!("❌ Failed to get token accounts: {}", e);
        }
    }
}

/// Show transaction history for a specific token
async fn show_token_transaction_history(mint: &str) {
    println!("📜 Transaction History for token: {}", mint);
    
    // Load wallet configuration
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            println!("❌ Failed to load wallet: {}", e);
            return;
        }
    };
    
    // Create transaction manager
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            println!("❌ Failed to create transaction manager: {}", e);
            return;
        }
    };
    
    // Get all cached transactions
    match manager.recalculate_all_cached_transactions(None).await {
        Ok(transactions) => {
            // Filter transactions for this specific token
            let token_transactions: Vec<_> = transactions.iter()
                .filter(|tx| is_transaction_for_token(tx, mint))
                .collect();
                
            if token_transactions.is_empty() {
                println!("📭 No transactions found for this token");
                return;
            }
            
            println!("📊 Found {} transactions for this token", token_transactions.len());
            println!("─────────────────────────────────────────────");
            
            // Sort by timestamp (newest first)
            let mut sorted_transactions = token_transactions.clone();
            sorted_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            for (i, tx) in sorted_transactions.iter().take(20).enumerate() {
                println!("\n{}. Transaction: {}", i + 1, &tx.signature[..16]);
                println!("   Time: {}", tx.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("   Type: {:?}", tx.transaction_type);
                println!("   Success: {}", tx.success);
                println!("   SOL Change: {:.6}", tx.sol_balance_change);
                println!("   Fee: {:.6} SOL", tx.fee_sol);
                
                // Show token transfers
                for transfer in &tx.token_transfers {
                    if transfer.mint == mint {
                        let from_display = if transfer.from.len() >= 8 { &transfer.from[..8] } else { &transfer.from };
                        let to_display = if transfer.to.len() >= 8 { &transfer.to[..8] } else { &transfer.to };
                        
                        println!("   Token Transfer: {:.6} {} ({} -> {})", 
                            transfer.amount, 
                            mint, // Use mint directly since we don't have mint_symbol field
                            from_display,
                            to_display
                        );
                    }
                }
            }
            
            if sorted_transactions.len() > 20 {
                println!("\n... and {} more transactions", sorted_transactions.len() - 20);
            }
        }
        Err(e) => {
            println!("❌ Failed to load transactions: {}", e);
        }
    }
}

/// Analyze positions and trades for a specific token from transactions
async fn analyze_token_positions(mint: &str) {
    println!("📈 Position Analysis for token: {}", mint);
    
    // Load wallet configuration
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            println!("❌ Failed to load wallet: {}", e);
            return;
        }
    };
    
    // Create transaction manager
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            println!("❌ Failed to create transaction manager: {}", e);
            return;
        }
    };
    
    // Get all swap transactions
    match manager.get_all_swap_transactions().await {
        Ok(swaps) => {
            // Filter swaps for this specific token
            let token_swaps: Vec<_> = swaps.iter()
                .filter(|swap| is_swap_for_token(swap, mint))
                .collect();
                
            if token_swaps.is_empty() {
                println!("📭 No swap transactions found for this token");
                return;
            }
            
            println!("📊 Found {} swap transactions for this token", token_swaps.len());
            
            // Analyze positions
            analyze_token_position_lifecycle(&token_swaps, mint);
            
            // Calculate summary statistics
            calculate_token_position_summary(&token_swaps, mint);
        }
        Err(e) => {
            println!("❌ Failed to load swap transactions: {}", e);
        }
    }
}

/// Show overall wallet summary with SOL and all token balances
async fn show_wallet_summary() {
    println!("👛 WALLET SUMMARY");
    println!("═══════════════════════════════════════════");
    
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return;
        }
    };
    
    println!("📋 Wallet Address: {}", wallet_address);
    
    // Get SOL balance
    println!("\n◎ SOL BALANCE");
    println!("─────────────────");
    match get_sol_balance(&wallet_address).await {
        Ok(sol_balance) => {
            println!("💰 Current SOL: {:.6} SOL", sol_balance);
            let usd_value = sol_balance * 150.0; // Approximate SOL price
            println!("💵 Approximate USD: ${:.2}", usd_value);
        }
        Err(e) => {
            println!("❌ Failed to get SOL balance: {}", e);
        }
    }
    
    // Get all token accounts
    println!("\n🪙 TOKEN BALANCES");
    println!("─────────────────");
    match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => {
            if accounts.is_empty() {
                println!("📭 No token accounts found");
                return;
            }
            
            println!("📊 Found {} token accounts", accounts.len());
            
            // Group by mint and sum balances
            let mut token_balances: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
            let mut token_programs: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
            
            for account in &accounts {
                *token_balances.entry(account.mint.clone()).or_insert(0) += account.balance;
                token_programs.insert(account.mint.clone(), account.is_token_2022);
            }
            
            // Display non-zero balances
            let mut non_zero_count = 0;
            for (mint, balance) in &token_balances {
                if *balance > 0 {
                    non_zero_count += 1;
                    
                    let decimals = get_cached_decimals(mint).unwrap_or(6);
                    let ui_amount = raw_to_ui_amount(*balance, decimals);
                    let is_token_2022 = token_programs.get(mint).copied().unwrap_or(false);
                    
                    println!("\n{}. Token: {}", non_zero_count, &mint[..16]);
                    println!("   Balance: {} raw ({:.6} UI)", balance, ui_amount);
                    println!("   Decimals: {}", decimals);
                    println!("   Program: {}", if is_token_2022 { "Token-2022" } else { "SPL Token" });
                    
                    // Try to get token symbol from database
                    if let Ok(conn) = Connection::open("data/tokens.db") {
                        if let Ok(mut stmt) = conn.prepare("SELECT symbol, name FROM tokens WHERE mint = ?1") {
                            if let Ok(row) = stmt.query_row([mint], |row| {
                                let symbol: String = row.get(0)?;
                                let name: String = row.get(1)?;
                                Ok((symbol, name))
                            }) {
                                println!("   Symbol: {}", row.0);
                                println!("   Name: {}", row.1);
                            }
                        }
                    }
                }
            }
            
            println!("\n📈 SUMMARY");
            println!("─────────────────");
            println!("Total token accounts: {}", accounts.len());
            println!("Accounts with balance: {}", non_zero_count);
            println!("Empty accounts: {}", accounts.len() - non_zero_count);
        }
        Err(e) => {
            println!("❌ Failed to get token accounts: {}", e);
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Check if transaction involves a specific token
fn is_transaction_for_token(transaction: &Transaction, mint: &str) -> bool {
    // Check transaction type
    let matches_type = match &transaction.transaction_type {
        TransactionType::SwapSolToToken { token_mint, .. } => token_mint == mint,
        TransactionType::SwapTokenToSol { token_mint, .. } => token_mint == mint,
        TransactionType::SwapTokenToToken { from_mint, to_mint, .. } => {
            from_mint == mint || to_mint == mint
        }
        TransactionType::TokenTransfer { mint: tx_mint, .. } => tx_mint == mint,
        _ => false,
    };
    
    // Also check token transfers
    let matches_transfers = transaction.token_transfers.iter().any(|transfer| transfer.mint == mint);
    
    matches_type || matches_transfers
}

/// Check if swap transaction involves a specific token
fn is_swap_for_token(swap: &SwapPnLInfo, mint: &str) -> bool {
    swap.token_mint == mint
}

/// Analyze position lifecycle for a token
fn analyze_token_position_lifecycle(swaps: &[&SwapPnLInfo], _mint: &str) {
    println!("\n📊 POSITION LIFECYCLE ANALYSIS");
    println!("─────────────────────────────────");
    
    let mut buy_swaps: Vec<_> = swaps.iter()
        .filter(|swap| swap.swap_type == "Buy")
        .collect();
        
    let mut sell_swaps: Vec<_> = swaps.iter()
        .filter(|swap| swap.swap_type == "Sell")
        .collect();
    
    // Sort by timestamp
    buy_swaps.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    sell_swaps.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    println!("🟢 Buy Transactions: {}", buy_swaps.len());
    for (i, swap) in buy_swaps.iter().enumerate() {
        println!("   {}. {} - SOL: {:.6} -> Tokens: {:.2} (Price: {:.9})", 
            i + 1,
            swap.timestamp.format("%Y-%m-%d %H:%M"),
            swap.sol_amount.abs(),
            swap.token_amount,
            swap.calculated_price_sol
        );
    }
    
    println!("\n🔴 Sell Transactions: {}", sell_swaps.len());
    for (i, swap) in sell_swaps.iter().enumerate() {
        println!("   {}. {} - Tokens: {:.2} -> SOL: {:.6} (Price: {:.9})", 
            i + 1,
            swap.timestamp.format("%Y-%m-%d %H:%M"),
            swap.token_amount,
            swap.sol_amount.abs(),
            swap.calculated_price_sol
        );
    }
}

/// Calculate position summary statistics
fn calculate_token_position_summary(swaps: &[&SwapPnLInfo], _mint: &str) {
    println!("\n📈 POSITION SUMMARY");
    println!("─────────────────────────────────");
    
    let mut total_sol_invested = 0.0;
    let mut total_sol_received = 0.0;
    let mut total_tokens_bought = 0.0;
    let mut total_tokens_sold = 0.0;
    let mut total_fees = 0.0;
    
    for swap in swaps {
        total_fees += swap.fee_sol;
        
        if swap.swap_type == "Buy" {
            total_sol_invested += swap.sol_amount.abs();
            total_tokens_bought += swap.token_amount;
        } else if swap.swap_type == "Sell" {
            total_sol_received += swap.sol_amount.abs();
            total_tokens_sold += swap.token_amount;
        }
    }
    
    let net_sol_change = total_sol_received - total_sol_invested;
    let net_token_change = total_tokens_bought - total_tokens_sold;
    
    println!("💰 SOL SUMMARY:");
    println!("   Invested: {:.6} SOL", total_sol_invested);
    println!("   Received: {:.6} SOL", total_sol_received);
    println!("   Net Change: {:.6} SOL", net_sol_change);
    
    println!("\n🪙 TOKEN SUMMARY:");
    println!("   Bought: {:.6} tokens", total_tokens_bought);
    println!("   Sold: {:.6} tokens", total_tokens_sold);
    println!("   Net Holdings: {:.6} tokens", net_token_change);
    
    println!("\n💸 FEES:");
    println!("   Total Fees: {:.6} SOL", total_fees);
    
    if total_sol_invested > 0.0 {
        let roi_percentage = (net_sol_change / total_sol_invested) * 100.0;
        println!("\n📊 PERFORMANCE:");
        println!("   ROI: {:.2}%", roi_percentage);
        
        if roi_percentage > 0.0 {
            println!("   Status: 🟢 Profitable");
        } else {
            println!("   Status: 🔴 Loss");
        }
    }
}

/// Load wallet pubkey from configuration (helper function)
async fn load_wallet_pubkey() -> Result<Pubkey, Box<dyn std::error::Error>> {
    let configs = read_configs()?;
    let wallet = load_wallet_from_config(&configs)?;
    
    // Get pubkey directly from the wallet
    let pubkey = wallet.pubkey();
    Ok(pubkey)
}
