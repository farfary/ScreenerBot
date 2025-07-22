// Token database management and statistics utility
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use chrono::{ Utc, Duration };
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("stats");

    // Initialize token database
    initialize_token_database()?;

    match command {
        "stats" => show_stats().await?,
        "search" => {
            let query = args.get(2).map_or("USD", |s| s.as_str());
            search_tokens(query).await?;
        }
        "recent" => {
            let hours = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(24);
            show_recent_tokens(hours).await?;
        }
        "cleanup" => {
            let days = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(30);
            cleanup_old_tokens(days).await?;
        }
        "help" => show_help(),
        _ => {
            log(LogTag::System, "ERROR", &format!("Unknown command: {}", command));
            show_help();
        }
    }

    Ok(())
}

async fn show_stats() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Token Database Statistics");

    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let stats = db.get_stats()?;

            println!("═══════════════════════════════════════");
            println!("            TOKEN DATABASE STATS       ");
            println!("═══════════════════════════════════════");
            println!("Total Tokens:     {}", stats.total_tokens);
            println!("Unique Sources:   {}", stats.unique_sources);
            println!("Total Accesses:   {}", stats.total_accesses);
            println!("Most Recent:      {}", stats.most_recent_token.as_deref().unwrap_or("None"));
            println!("═══════════════════════════════════════");

            // Show breakdown by discovery source
            log(LogTag::System, "INFO", "Getting source breakdown...");
            // This would require additional database queries
        }
    }

    Ok(())
}

async fn search_tokens(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", &format!("Searching tokens for: '{}'", query));

    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let results = db.search_tokens(query)?;

            println!("═══════════════════════════════════════");
            println!("        SEARCH RESULTS: '{}'", query);
            println!("═══════════════════════════════════════");
            println!("Found {} tokens matching '{}'", results.len(), query);
            println!();

            for (i, token) in results.iter().enumerate().take(20) {
                println!("{}. {} ({}) - {}", i + 1, token.symbol, token.name, token.mint);
                if let Some(liquidity) = &token.liquidity {
                    if let Some(usd) = liquidity.usd {
                        println!("   Liquidity: ${:.2}", usd);
                    }
                }
                println!();
            }

            if results.len() > 20 {
                println!("... and {} more results", results.len() - 20);
            }

            println!("═══════════════════════════════════════");
        }
    }

    Ok(())
}

async fn show_recent_tokens(hours: i64) -> Result<(), Box<dyn std::error::Error>> {
    let since = Utc::now() - Duration::hours(hours);

    log(LogTag::System, "INFO", &format!("Showing tokens discovered in the last {} hours", hours));

    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let recent_tokens = db.get_tokens_since(since)?;

            println!("═══════════════════════════════════════");
            println!("     RECENT TOKENS ({} hours)", hours);
            println!("═══════════════════════════════════════");
            println!(
                "Found {} tokens discovered since {}",
                recent_tokens.len(),
                since.format("%Y-%m-%d %H:%M:%S")
            );
            println!();

            for token in recent_tokens.iter().take(30) {
                println!("• {} ({}) - {}", token.symbol, token.name, token.mint);
                if let Some(created) = token.created_at {
                    println!("  Discovered: {}", created.format("%Y-%m-%d %H:%M:%S"));
                }
                if let Some(liquidity) = &token.liquidity {
                    if let Some(usd) = liquidity.usd {
                        println!("  Liquidity: ${:.2}", usd);
                    }
                }
                println!();
            }

            if recent_tokens.len() > 30 {
                println!("... and {} more recent tokens", recent_tokens.len() - 30);
            }

            println!("═══════════════════════════════════════");
        }
    }

    Ok(())
}

async fn cleanup_old_tokens(days: i64) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", &format!("Cleaning up tokens older than {} days", days));

    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let deleted = db.cleanup_old_tokens(days)?;

            println!("═══════════════════════════════════════");
            println!("         CLEANUP COMPLETED");
            println!("═══════════════════════════════════════");
            println!("Deleted {} tokens older than {} days", deleted, days);
            println!("═══════════════════════════════════════");
        }
    }

    Ok(())
}

fn show_help() {
    println!("═══════════════════════════════════════");
    println!("       TOKEN DATABASE MANAGER");
    println!("═══════════════════════════════════════");
    println!("Usage: cargo run --bin token_db_manager [command] [args]");
    println!();
    println!("Commands:");
    println!("  stats                    - Show database statistics");
    println!("  search <query>           - Search tokens by symbol/name");
    println!("  recent <hours>           - Show tokens from last N hours");
    println!("  cleanup <days>           - Delete tokens older than N days");
    println!("  help                     - Show this help message");
    println!();
    println!("Examples:");
    println!("  cargo run --bin token_db_manager stats");
    println!("  cargo run --bin token_db_manager search USDC");
    println!("  cargo run --bin token_db_manager recent 6");
    println!("  cargo run --bin token_db_manager cleanup 30");
    println!("═══════════════════════════════════════");
}
