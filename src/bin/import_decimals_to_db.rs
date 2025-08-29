/// Import decimals from JSON cache to SQLite database
/// This is a one-time migration tool
use screenerbot::global::TOKENS_DATABASE;
use screenerbot::logger::{init_file_logging, log, LogTag};
use std::fs;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, Result as SqliteResult};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the JSON decimal cache file
    #[arg(short, long, default_value = "data/decimal_cache.json")]
    json_file: String,
    
    /// Show database statistics only (no import)
    #[arg(short, long)]
    stats_only: bool,
    
    /// Test database functions after import
    #[arg(short, long)]
    test_functions: bool,
    
    /// Dry run - show what would be imported without actually importing
    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Serialize, Deserialize)]
struct DecimalCacheData {
    decimals: HashMap<String, u8>,
    failed_tokens: HashMap<String, String>,
}

/// Initialize the decimals database tables
fn init_decimals_database() -> SqliteResult<()> {
    let conn = Connection::open(TOKENS_DATABASE)?;
    
    // Create decimals table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS decimals (
            mint TEXT PRIMARY KEY,
            decimals INTEGER NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        []
    )?;
    
    // Create failed decimals table for retryable vs permanent failures
    conn.execute(
        "CREATE TABLE IF NOT EXISTS failed_decimals (
            mint TEXT PRIMARY KEY,
            error_message TEXT NOT NULL,
            is_permanent INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        []
    )?;
    
    // Create indices for performance
    conn.execute("CREATE INDEX IF NOT EXISTS idx_decimals_updated ON decimals(updated_at)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_failed_decimals_permanent ON failed_decimals(is_permanent)", [])?;
    
    Ok(())
}

/// Get database statistics
fn get_database_stats() -> SqliteResult<(usize, usize)> {
    let conn = Connection::open(TOKENS_DATABASE)?;
    
    // Count decimals
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM decimals")?;
    let decimals_count: i64 = stmt.query_row([], |row| row.get(0))?;
    
    // Count failed decimals
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM failed_decimals")?;
    let failed_count: i64 = stmt.query_row([], |row| row.get(0))?;
    
    Ok((decimals_count as usize, failed_count as usize))
}

/// Import decimals from JSON to database
fn import_decimals(decimals: &HashMap<String, u8>, dry_run: bool) -> SqliteResult<usize> {
    if dry_run {
        println!("DRY RUN: Would import {} decimal entries", decimals.len());
        return Ok(decimals.len());
    }
    
    let conn = Connection::open(TOKENS_DATABASE)?;
    let mut imported = 0;
    
    for (mint, &decimals_value) in decimals {
        // Skip SOL as it's always handled in code
        if mint == "So11111111111111111111111111111111111111112" {
            continue;
        }
        
        conn.execute(
            "INSERT OR REPLACE INTO decimals (mint, decimals, updated_at) VALUES (?1, ?2, datetime('now'))",
            [mint, &decimals_value.to_string()]
        )?;
        imported += 1;
    }
    
    Ok(imported)
}

/// Import failed tokens from JSON to database
fn import_failed_tokens(failed_tokens: &HashMap<String, String>, dry_run: bool) -> SqliteResult<usize> {
    if dry_run {
        println!("DRY RUN: Would import {} failed token entries", failed_tokens.len());
        return Ok(failed_tokens.len());
    }
    
    let conn = Connection::open(TOKENS_DATABASE)?;
    let mut imported = 0;
    
    for (mint, error) in failed_tokens {
        // Determine if this is a permanent error
        let is_permanent = should_cache_as_failed(error);
        
        conn.execute(
            "INSERT OR REPLACE INTO failed_decimals (mint, error_message, is_permanent, updated_at) VALUES (?1, ?2, ?3, datetime('now'))",
            [mint, error, &(if is_permanent { 1 } else { 0 }).to_string()]
        )?;
        imported += 1;
    }
    
    Ok(imported)
}

/// Check if error should be cached as failed (real errors) vs retried (rate limits)
fn should_cache_as_failed(error: &str) -> bool {
    let error_lower = error.to_lowercase();

    // Real blockchain state errors - cache as failed
    if
        error_lower.contains("account not found") ||
        error_lower.contains("invalid account") ||
        error_lower.contains("account does not exist") ||
        error_lower.contains("invalid mint") ||
        error_lower.contains("empty") ||
        error_lower.contains("account owner is not spl token program")
    {
        return true;
    }

    // Rate limiting and temporary issues - retry with different RPC
    if
        error_lower.contains("429") ||
        error_lower.contains("too many requests") ||
        error_lower.contains("rate limit") ||
        error_lower.contains("rate limited") ||
        error_lower.contains("timeout") ||
        error_lower.contains("connection") ||
        error_lower.contains("network") ||
        error_lower.contains("unavailable") ||
        error_lower.contains("error sending request") ||
        error_lower.contains("request failed") ||
        error_lower.contains("connection refused") ||
        error_lower.contains("connection reset") ||
        error_lower.contains("timed out") ||
        error_lower.contains("dns") ||
        error_lower.contains("ssl") ||
        error_lower.contains("tls") ||
        error_lower.contains("failed to get multiple accounts") ||
        error_lower.contains("batch fetch failed")
    {
        return false;
    }

    // Default to caching as failed for unknown errors
    true
}

/// Test database functions with sample data
async fn test_database_functions() -> Result<(), Box<dyn std::error::Error>> {
    use screenerbot::tokens::decimals::{
        get_cached_decimals, 
        batch_fetch_token_decimals,
        get_database_stats,
        get_failed_cache_stats
    };
    
    println!("\n=== TESTING DATABASE FUNCTIONS ===");
    
    // Test 1: Get cached decimals for a known token
    println!("Test 1: Getting cached decimals for SOL...");
    if let Some(decimals) = get_cached_decimals("So11111111111111111111111111111111111111112") {
        println!("✓ SOL decimals: {}", decimals);
    } else {
        println!("✗ Failed to get SOL decimals");
    }
    
    // Test 2: Database statistics
    println!("\nTest 2: Database statistics...");
    match get_database_stats() {
        Ok((decimals_count, failed_count)) => {
            println!("✓ Database stats: {} decimals, {} failed tokens", decimals_count, failed_count);
        }
        Err(e) => {
            println!("✗ Failed to get database stats: {}", e);
        }
    }
    
    // Test 3: Failed cache statistics
    println!("\nTest 3: Failed cache statistics...");
    match get_failed_cache_stats() {
        Ok((total, permanent, samples)) => {
            println!("✓ Failed cache stats: {} total, {} permanent", total, permanent);
            if !samples.is_empty() {
                println!("  Sample errors:");
                for sample in samples.iter().take(3) {
                    println!("    {}", sample);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to get failed cache stats: {}", e);
        }
    }
    
    // Test 4: Batch fetch (this will test database lookup)
    println!("\nTest 4: Batch fetch with database lookup...");
    let test_mints = vec![
        "So11111111111111111111111111111111111111112".to_string(), // SOL
    ];
    
    let results = batch_fetch_token_decimals(&test_mints).await;
    for (mint, result) in results {
        match result {
            Ok(decimals) => {
                println!("✓ {}: {} decimals", &mint[..8], decimals);
            }
            Err(e) => {
                println!("✗ {}: {}", &mint[..8], e);
            }
        }
    }
    
    println!("\n=== DATABASE FUNCTION TESTS COMPLETED ===");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    init_file_logging();
    
    println!("ScreenerBot Decimals Import Tool");
    println!("=================================");
    
    // Initialize database
    println!("Initializing database...");
    init_decimals_database()?;
    log(LogTag::Decimals, "IMPORT_INIT", "Database initialized successfully");
    
    // Show current database stats
    let (current_decimals, current_failed) = get_database_stats()?;
    println!("Current database: {} decimals, {} failed tokens", current_decimals, current_failed);
    
    if args.stats_only && !args.test_functions {
        println!("Stats-only mode - exiting.");
        return Ok(());
    }
    
    // Read JSON file
    println!("Reading JSON file: {}", args.json_file);
    let json_content = fs::read_to_string(&args.json_file)
        .map_err(|e| format!("Failed to read JSON file: {}", e))?;
    
    // Parse JSON
    let cache_data: DecimalCacheData = match serde_json::from_str(&json_content) {
        Ok(data) => data,
        Err(_) => {
            // Try old format (just decimals)
            println!("Trying old JSON format...");
            let decimals: HashMap<String, u8> = serde_json::from_str(&json_content)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;
            DecimalCacheData {
                decimals,
                failed_tokens: HashMap::new(),
            }
        }
    };
    
    println!("JSON loaded: {} decimals, {} failed tokens", 
             cache_data.decimals.len(), cache_data.failed_tokens.len());
    
    if args.dry_run {
        println!("\n=== DRY RUN MODE ===");
    }
    
    if !args.stats_only {
        // Import decimals
        println!("Importing decimals...");
        let imported_decimals = import_decimals(&cache_data.decimals, args.dry_run)?;
        println!("Imported {} decimal entries", imported_decimals);
        
        // Import failed tokens
        println!("Importing failed tokens...");
        let imported_failed = import_failed_tokens(&cache_data.failed_tokens, args.dry_run)?;
        println!("Imported {} failed token entries", imported_failed);
        
        if !args.dry_run {
            // Show final database stats
            let (final_decimals, final_failed) = get_database_stats()?;
            println!("\nFinal database: {} decimals, {} failed tokens", final_decimals, final_failed);
            
            log(LogTag::Decimals, "IMPORT_COMPLETE", 
                &format!("Import completed: {} decimals, {} failed tokens", 
                        imported_decimals, imported_failed));
        }
    }
    
    // Test functions if requested
    if args.test_functions {
        test_database_functions().await?;
    }
    
    println!("\nImport tool completed successfully!");
    
    Ok(())
}
