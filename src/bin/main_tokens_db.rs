/// Token Database Debug Tool
/// 
/// Simple command-line tool to query the tokens database for debugging.
/// This tool allows AI assistants and developers to quickly lookup token information.
/// 
/// Usage Examples:
/// - Search by mint: cargo run --bin main_tokens_db -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D
/// - Search by symbol: cargo run --bin main_tokens_db -- --symbol PSAF
/// - Search by name: cargo run --bin main_tokens_db -- --name "Alpha Fund"
/// - List all tokens: cargo run --bin main_tokens_db -- --list
/// - Count tokens: cargo run --bin main_tokens_db -- --count
/// - Recent tokens: cargo run --bin main_tokens_db -- --recent 10

use clap::{Arg, Command};
use std::path::Path;
use rusqlite::Connection;

#[derive(Debug)]
struct TokenInfo {
    mint: String,
    symbol: String,
    name: String,
    price_usd: Option<f64>,
    price_sol: Option<f64>,
    liquidity_usd: Option<f64>,
    last_updated: Option<String>,
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
    println!("└─────────────────────────────────────────────────────────────");
}

fn display_token_compact(token: &TokenInfo) {
    let price_str = token.price_usd.map_or("N/A".to_string(), |p| format!("${:.6}", p));
    println!("{:8} | {:30} | {}", token.symbol, &token.name[..30.min(token.name.len())], price_str);
}

fn show_help() {
    println!("🔧 Token Database Debug Tool");
    println!("Usage examples:");
    println!("  cargo run --bin main_tokens_db -- --mint PSAbMyzQqPu9dZpRNdtSxpnHY5CBwYwk7iVzZrNFg1D");
    println!("  cargo run --bin main_tokens_db -- --symbol PSAF");
    println!("  cargo run --bin main_tokens_db -- --name 'Alpha Fund'");
    println!("  cargo run --bin main_tokens_db -- --list");
    println!("  cargo run --bin main_tokens_db -- --count");
    println!("  cargo run --bin main_tokens_db -- --recent 10");
}
