use screenerbot::tokens::cache::TokenDatabase;
use screenerbot::tokens::decimals::{ batch_fetch_token_decimals, get_cached_decimals };
use screenerbot::rpc::init_rpc_client;
use std::collections::HashMap;
use std::env;
use tokio;

/// Comprehensive decimals debugging tool
///
/// Usage:
/// --test-failed        : Test tokens that previously failed
/// --test-database      : Test tokens from database
/// --test-discovery     : Test recently discovered tokens
/// --test-batch <mints> : Test specific mints (comma-separated)
/// --stats              : Show decimals statistics
/// --clean-failed       : Clean failed decimals cache
/// --all                : Run all tests

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RPC client - remove logging init as it's not available
    if let Err(e) = init_rpc_client() {
        eprintln!("❌ Failed to initialize RPC client: {}", e);
        return Err(e.into());
    }

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "--test-failed" => test_failed_tokens().await?,
        "--test-database" => test_database_tokens().await?,
        "--test-discovery" => test_discovery_tokens().await?,
        "--test-specific" => {
            // Test the specific token from GeckoTerminal example
            let test_token = "bv2Rv7uyiEQxxjjsLxABjcw6mH8XzUDnm5oNuZDpump".to_string();
            test_specific_tokens(&[test_token]).await?;
        }
        "--test-batch" => {
            if args.len() < 3 {
                eprintln!("❌ Please provide comma-separated mint addresses");
                return Ok(());
            }
            let mints: Vec<String> = args[2]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            test_specific_tokens(&mints).await?;
        }
        "--stats" => show_decimals_stats().await?,
        "--clean-failed" => clean_failed_cache().await?,
        "--all" => {
            println!("🔍 Running comprehensive decimals analysis...\n");
            show_decimals_stats().await?;
            println!("\n{}", "═".repeat(60));
            test_database_tokens().await?;
            println!("\n{}", "═".repeat(60));
            test_failed_tokens().await?;
            println!("\n{}", "═".repeat(60));
            test_discovery_tokens().await?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("🔍 ScreenerBot Decimals Debugging Tool");
    println!("{}", "═".repeat(50));
    println!("Usage: cargo run --bin main_debug_decimals [OPTION]");
    println!();
    println!("Options:");
    println!("  --test-failed        Test tokens that previously failed");
    println!("  --test-database      Test tokens from database (recent)");
    println!("  --test-discovery     Test recently discovered tokens");
    println!("  --test-specific      Test specific problematic token from GeckoTerminal");
    println!("  --test-batch <mints> Test specific tokens (comma-separated)");
    println!("  --stats              Show decimals statistics");
    println!("  --clean-failed       Clean failed decimals cache");
    println!("  --all                Run comprehensive analysis");
    println!();
    println!("Examples:");
    println!("  cargo run --bin main_debug_decimals -- --stats");
    println!("  cargo run --bin main_debug_decimals -- --test-batch \"MINT1,MINT2\"");
    println!("  cargo run --bin main_debug_decimals -- --all");
}

async fn show_decimals_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 DECIMALS SYSTEM STATISTICS");
    println!("{}", "═".repeat(50));

    // Open database to get statistics
    let _db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to open token database: {}", e);
            return Ok(());
        }
    };

    // Get decimals table stats
    let decimals_count = get_table_count("decimals").await;
    let failed_decimals_count = get_table_count("failed_decimals").await;
    let permanent_failed_count = get_permanent_failed_count().await;
    let temporary_failed_count = failed_decimals_count - permanent_failed_count;

    println!("📈 Database Statistics:");
    println!("  ✅ Successful decimals cached: {}", decimals_count);
    println!("  ❌ Total failed decimals: {}", failed_decimals_count);
    println!("     🔴 Permanent failures: {}", permanent_failed_count);
    println!("     🟡 Temporary failures: {}", temporary_failed_count);

    let success_rate = if decimals_count + failed_decimals_count > 0 {
        ((decimals_count as f64) / ((decimals_count + failed_decimals_count) as f64)) * 100.0
    } else {
        0.0
    };
    println!("  📊 Success rate: {:.1}%", success_rate);

    // Get sample of recent failed tokens
    println!("\n🔍 Recent Failed Tokens (last 10):");
    let recent_failed = get_recent_failed_tokens(10).await;
    for (mint, error, is_permanent) in recent_failed {
        let status = if is_permanent { "🔴 PERM" } else { "🟡 TEMP" };
        println!("  {} {} - {}", status, &mint[..8], error);
    }

    // Get sample of tokens without decimals
    println!("\n🔍 Tokens in database without decimals (sample 10):");
    let tokens_without_decimals = get_tokens_without_decimals(10).await;
    for (mint, symbol) in tokens_without_decimals {
        println!("  🔍 {} ({}) - no decimals cached", &mint[..8], symbol);
    }

    Ok(())
}

async fn test_failed_tokens() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 TESTING PREVIOUSLY FAILED TOKENS");
    println!("{}", "═".repeat(50));

    // Get some failed tokens to retry
    let failed_tokens = get_recent_failed_tokens(20).await;

    if failed_tokens.is_empty() {
        println!("✅ No failed tokens found in cache");
        return Ok(());
    }

    println!("🧪 Testing {} previously failed tokens...", failed_tokens.len());

    let mints: Vec<String> = failed_tokens
        .iter()
        .map(|(mint, _, _)| mint.clone())
        .collect();
    let results = batch_fetch_token_decimals(&mints).await;

    let mut success_count = 0;
    let mut still_failing = 0;

    println!("\n📋 Results:");
    for (mint, result) in results {
        match result {
            Ok(decimals) => {
                success_count += 1;
                println!("  ✅ {} - SUCCESS: {} decimals", &mint[..8], decimals);
            }
            Err(error) => {
                still_failing += 1;
                println!("  ❌ {} - FAILED: {}", &mint[..8], error);
            }
        }
    }

    println!("\n📊 Retry Results:");
    println!("  ✅ Now successful: {}", success_count);
    println!("  ❌ Still failing: {}", still_failing);

    if success_count > 0 {
        println!("  🎉 {} tokens recovered from failed cache!", success_count);
    }

    Ok(())
}

async fn test_database_tokens() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗄️ TESTING TOKENS FROM DATABASE");
    println!("{}", "═".repeat(50));

    // Get recent tokens without decimals
    let tokens_without_decimals = get_tokens_without_decimals(50).await;

    if tokens_without_decimals.is_empty() {
        println!("✅ All tokens in database have decimals cached");
        return Ok(());
    }

    println!("🧪 Testing {} tokens without cached decimals...", tokens_without_decimals.len());

    let mints: Vec<String> = tokens_without_decimals
        .iter()
        .map(|(mint, _)| mint.clone())
        .collect();
    let results = batch_fetch_token_decimals(&mints).await;

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut error_categories: HashMap<String, usize> = HashMap::new();

    println!("\n📋 Results:");
    for (mint, result) in results {
        let symbol = tokens_without_decimals
            .iter()
            .find(|(m, _)| m == &mint)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        match result {
            Ok(decimals) => {
                success_count += 1;
                println!("  ✅ {} ({}) - SUCCESS: {} decimals", &mint[..8], symbol, decimals);
            }
            Err(error) => {
                failed_count += 1;

                // Categorize error
                let category = categorize_error(&error);
                *error_categories.entry(category.clone()).or_insert(0) += 1;

                println!("  ❌ {} ({}) - FAILED: {}", &mint[..8], symbol, error);
            }
        }
    }

    println!("\n📊 Database Token Results:");
    println!("  ✅ Successfully fetched: {}", success_count);
    println!("  ❌ Failed to fetch: {}", failed_count);

    if !error_categories.is_empty() {
        println!("\n🔍 Error Categories:");
        let mut sorted_errors: Vec<_> = error_categories.into_iter().collect();
        sorted_errors.sort_by(|a, b| b.1.cmp(&a.1));

        for (category, count) in sorted_errors {
            println!("  📊 {}: {} tokens", category, count);
        }
    }

    Ok(())
}

async fn test_discovery_tokens() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING RECENTLY DISCOVERED TOKENS");
    println!("{}", "═".repeat(50));

    // Get recent tokens from discovery (last 24 hours)
    let recent_tokens = get_recent_discovery_tokens(30).await;

    if recent_tokens.is_empty() {
        println!("ℹ️ No recent discovery tokens found");
        return Ok(());
    }

    println!("🧪 Testing {} recently discovered tokens...", recent_tokens.len());

    let mints: Vec<String> = recent_tokens
        .iter()
        .map(|(mint, _)| mint.clone())
        .collect();
    let results = batch_fetch_token_decimals(&mints).await;

    let mut success_count = 0;
    let mut failed_count = 0;

    println!("\n📋 Results:");
    for (mint, result) in results {
        let symbol = recent_tokens
            .iter()
            .find(|(m, _)| m == &mint)
            .map(|(_, s)| s.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        match result {
            Ok(decimals) => {
                success_count += 1;
                println!("  ✅ {} ({}) - SUCCESS: {} decimals", &mint[..8], symbol, decimals);
            }
            Err(error) => {
                failed_count += 1;
                println!("  ❌ {} ({}) - FAILED: {}", &mint[..8], symbol, error);
            }
        }
    }

    println!("\n📊 Discovery Token Results:");
    println!("  ✅ Successfully fetched: {}", success_count);
    println!("  ❌ Failed to fetch: {}", failed_count);

    Ok(())
}

async fn test_specific_tokens(mints: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 TESTING SPECIFIC TOKENS");
    println!("{}", "═".repeat(50));

    if mints.is_empty() {
        println!("❌ No tokens provided");
        return Ok(());
    }

    println!("🧪 Testing {} specific tokens...", mints.len());

    // Test each token individually for detailed debugging
    for mint in mints {
        println!("\n🔍 Testing token: {}", mint);

        // First check GeckoTerminal API for comparison
        if let Ok(gecko_decimals) = fetch_gecko_terminal_decimals(mint).await {
            println!("  🦎 GeckoTerminal decimals: {}", gecko_decimals);
        } else {
            println!("  🦎 GeckoTerminal: Failed to fetch or not found");
        }

        // Test individual fetch from cache
        match get_cached_decimals(mint) {
            Some(decimals) => {
                println!("  ✅ Cache lookup: {} decimals", decimals);
            }
            None => {
                println!("  ❌ Cache lookup failed - not in cache");
            }
        }

        // Test batch fetch with detailed debugging
        println!("  🔄 Running batch fetch with debug logging...");
        let batch_results = batch_fetch_token_decimals(&[mint.clone()]).await;
        if let Some((_, result)) = batch_results.first() {
            match result {
                Ok(decimals) => {
                    println!("  ✅ Batch fetch: {} decimals", decimals);
                }
                Err(error) => {
                    println!("  ❌ Batch fetch failed: {}", error);
                }
            }
        }
    }

    Ok(())
}

/// Fetch token decimals from GeckoTerminal API for comparison
async fn fetch_gecko_terminal_decimals(mint: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let url = format!("https://api.geckoterminal.com/api/v2/networks/solana/tokens/{}", mint);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status(), response.text().await?).into());
    }

    let json: serde_json::Value = response.json().await?;

    // Extract decimals from response
    if
        let Some(decimals) = json
            .get("data")
            .and_then(|data| data.get("attributes"))
            .and_then(|attrs| attrs.get("decimals"))
            .and_then(|d| d.as_u64())
    {
        Ok(decimals as u8)
    } else {
        Err("Decimals not found in response".into())
    }
}

async fn clean_failed_cache() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 CLEANING FAILED DECIMALS CACHE");
    println!("{}", "═".repeat(50));

    // This would require access to the database cleaning functions
    // For now, just report what would be cleaned
    let failed_count = get_table_count("failed_decimals").await;
    let permanent_count = get_permanent_failed_count().await;
    let temporary_count = failed_count - permanent_count;

    println!("📊 Current failed cache status:");
    println!("  🔴 Permanent failures: {}", permanent_count);
    println!("  🟡 Temporary failures: {}", temporary_count);
    println!("  📋 Total failed entries: {}", failed_count);

    if temporary_count > 0 {
        println!("\n🔄 Recommendation: Temporary failures can be retried");
        println!("   Consider removing temporary failures older than 24h");
    }

    if permanent_count > 100 {
        println!("\n🗑️ Recommendation: Large permanent failure cache");
        println!("   Consider periodic cleanup of very old permanent failures");
    }

    Ok(())
}

// Helper functions to access database

async fn get_table_count(table: &str) -> usize {
    match open_database_connection() {
        Ok(conn) => {
            let query = format!("SELECT COUNT(*) FROM {}", table);
            match
                conn
                    .prepare(&query)
                    .and_then(|mut stmt| { stmt.query_row([], |row| row.get::<_, i64>(0)) })
            {
                Ok(count) => count as usize,
                Err(e) => {
                    eprintln!("❌ Failed to count {} table: {}", table, e);
                    0
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            0
        }
    }
}

async fn get_permanent_failed_count() -> usize {
    match open_database_connection() {
        Ok(conn) => {
            match
                conn
                    .prepare("SELECT COUNT(*) FROM failed_decimals WHERE is_permanent = 1")
                    .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            {
                Ok(count) => count as usize,
                Err(e) => {
                    eprintln!("❌ Failed to count permanent failures: {}", e);
                    0
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            0
        }
    }
}

async fn get_recent_failed_tokens(limit: usize) -> Vec<(String, String, bool)> {
    match open_database_connection() {
        Ok(conn) => {
            let query =
                "SELECT mint, error_message, is_permanent FROM failed_decimals ORDER BY updated_at DESC LIMIT ?1";
            match conn.prepare(query) {
                Ok(mut stmt) => {
                    match
                        stmt.query_map([limit as i64], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, i32>(2)? == 1,
                            ))
                        })
                    {
                        Ok(rows) => { rows.filter_map(|r| r.ok()).collect() }
                        Err(e) => {
                            eprintln!("❌ Failed to query failed tokens: {}", e);
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to prepare failed tokens query: {}", e);
                    vec![]
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            vec![]
        }
    }
}

async fn get_tokens_without_decimals(limit: usize) -> Vec<(String, String)> {
    match open_database_connection() {
        Ok(conn) => {
            let query =
                "SELECT mint, symbol FROM tokens WHERE mint NOT IN (SELECT mint FROM decimals) ORDER BY created_at DESC LIMIT ?1";
            match conn.prepare(query) {
                Ok(mut stmt) => {
                    match
                        stmt.query_map([limit as i64], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        })
                    {
                        Ok(rows) => { rows.filter_map(|r| r.ok()).collect() }
                        Err(e) => {
                            eprintln!("❌ Failed to query tokens without decimals: {}", e);
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to prepare tokens query: {}", e);
                    vec![]
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            vec![]
        }
    }
}

async fn get_recent_discovery_tokens(limit: usize) -> Vec<(String, String)> {
    match open_database_connection() {
        Ok(conn) => {
            let query =
                "SELECT mint, symbol FROM tokens WHERE created_at > datetime('now', '-24 hours') ORDER BY created_at DESC LIMIT ?1";
            match conn.prepare(query) {
                Ok(mut stmt) => {
                    match
                        stmt.query_map([limit as i64], |row| {
                            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                        })
                    {
                        Ok(rows) => { rows.filter_map(|r| r.ok()).collect() }
                        Err(e) => {
                            eprintln!("❌ Failed to query recent tokens: {}", e);
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to prepare recent tokens query: {}", e);
                    vec![]
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to open database: {}", e);
            vec![]
        }
    }
}

fn open_database_connection() -> Result<rusqlite::Connection, rusqlite::Error> {
    use screenerbot::global::TOKENS_DATABASE;
    rusqlite::Connection::open(TOKENS_DATABASE)
}

fn categorize_error(error: &str) -> String {
    let error_lower = error.to_lowercase();

    if error_lower.contains("account not found") || error_lower.contains("account does not exist") {
        "Account Not Found".to_string()
    } else if error_lower.contains("invalid mint") || error_lower.contains("invalid account") {
        "Invalid Mint/Account".to_string()
    } else if error_lower.contains("429") || error_lower.contains("rate limit") {
        "Rate Limited".to_string()
    } else if error_lower.contains("timeout") || error_lower.contains("connection") {
        "Network Issues".to_string()
    } else if error_lower.contains("program") {
        "Program Issues".to_string()
    } else {
        "Other".to_string()
    }
}
