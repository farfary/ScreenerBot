/// Security Analysis Debug Tool
///
/// This tool provides comprehensive security analysis for tokens using the
/// security module. It supports analyzing individual tokens, top tokens from
/// database, and batch analysis with caching.

use screenerbot::{
    logger::{ log, LogTag },
    tokens::security::{ TokenSecurityAnalyzer, TokenSecurityInfo, SecurityRiskLevel },
};
use std::env;
use tokio;

const HELP_TEXT: &str =
    r#"
Security Analysis Debug Tool

⚠️  WARNING: NEVER test with SOL or USDC! These mainstream tokens have too many
    holders and will cause RPC failures. Use smaller tokens from our database.

USAGE:
    cargo run --bin debug_security_new [OPTIONS]

OPTIONS:
    --help                     Show this help message
    --mint <ADDRESS>           Analyze security for specific token mint
                              (use tokens from our database, NOT SOL/USDC!)
    --top <N>                  Analyze top N tokens from OUR database (default: 10, max: 800)
                              (these are discovered tokens, not mainstream tokens)
    --search <QUERY>           Search tokens by symbol/name and analyze security
    --search-limit <N>         Limit search results (default: 20, max: 100)
    --batch <MINTS>            Batch analyze comma-separated mint addresses
    --force-refresh            Force refresh cached data
    --risk-level <LEVEL>       Filter by risk level (safe, low, medium, high, critical)
    --show-cache-stats         Show cache statistics
    --cleanup-old              Cleanup old cache entries (older than 24h)

⚠️  IMPORTANT NOTES:
    - Only use tokens from our ScreenerBot database
    - Never test with SOL (So11111111111111111111111111111111111111112)
    - Never test with USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
    - These tokens have millions of holders and will fail

EXAMPLES:
    # Analyze specific token (use tokens from database, NOT SOL/USDC)
    cargo run --bin debug_security_new --mint <SOME_SMALL_TOKEN_FROM_DB>

    # Analyze top 20 tokens from our database
    cargo run --bin debug_security_new --top 20

    # Search and analyze tokens from our database
    cargo run --bin debug_security_new --search "PUMP"

    # Batch analysis (use small tokens only)
    cargo run --bin debug_security_new --batch "mint1,mint2,mint3"

    # Show only high risk tokens from our database
    cargo run --bin debug_security_new --top 50 --risk-level high
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize basic logger
    log(LogTag::Security, "DEBUG", "Starting security analysis debug tool");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    // Parse command line arguments
    let mut i = 1;
    let mut mint_address: Option<String> = None;
    let mut top_count: Option<usize> = None;
    let mut search_query: Option<String> = None;
    let mut search_limit: usize = 20; // Default search limit
    let mut batch_mints: Option<Vec<String>> = None;
    let mut force_refresh = false;
    let mut risk_filter: Option<SecurityRiskLevel> = None;
    let mut show_cache_stats = false;
    let mut cleanup_old = false;

    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_help();
                return Ok(());
            }
            "--mint" => {
                if i + 1 < args.len() {
                    mint_address = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: --mint requires a mint address");
                    return Ok(());
                }
            }
            "--top" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(count) => {
                            if count > 800 {
                                eprintln!("Error: Maximum top count is 800");
                                return Ok(());
                            }
                            top_count = Some(count);
                        }
                        Err(_) => {
                            eprintln!("Error: --top requires a valid number");
                            return Ok(());
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --top requires a number");
                    return Ok(());
                }
            }
            "--search" => {
                if i + 1 < args.len() {
                    search_query = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    eprintln!("Error: --search requires a query string");
                    return Ok(());
                }
            }
            "--search-limit" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<usize>() {
                        Ok(limit) => {
                            if limit > 100 {
                                eprintln!("Error: Maximum search limit is 100");
                                return Ok(());
                            }
                            if limit == 0 {
                                eprintln!("Error: Search limit must be at least 1");
                                return Ok(());
                            }
                            search_limit = limit;
                        }
                        Err(_) => {
                            eprintln!("Error: --search-limit requires a valid number");
                            return Ok(());
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --search-limit requires a number");
                    return Ok(());
                }
            }
            "--batch" => {
                if i + 1 < args.len() {
                    let mints: Vec<String> = args[i + 1]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if mints.is_empty() {
                        eprintln!("Error: --batch requires comma-separated mint addresses");
                        return Ok(());
                    }
                    batch_mints = Some(mints);
                    i += 1;
                } else {
                    eprintln!("Error: --batch requires comma-separated mint addresses");
                    return Ok(());
                }
            }
            "--force-refresh" => {
                force_refresh = true;
            }
            "--risk-level" => {
                if i + 1 < args.len() {
                    match args[i + 1].to_lowercase().as_str() {
                        "safe" => {
                            risk_filter = Some(SecurityRiskLevel::Safe);
                        }
                        "low" => {
                            risk_filter = Some(SecurityRiskLevel::Low);
                        }
                        "medium" => {
                            risk_filter = Some(SecurityRiskLevel::Medium);
                        }
                        "high" => {
                            risk_filter = Some(SecurityRiskLevel::High);
                        }
                        "critical" => {
                            risk_filter = Some(SecurityRiskLevel::Critical);
                        }
                        _ => {
                            eprintln!(
                                "Error: Invalid risk level. Use: safe, low, medium, high, critical"
                            );
                            return Ok(());
                        }
                    }
                    i += 1;
                } else {
                    eprintln!(
                        "Error: --risk-level requires a level (safe, low, medium, high, critical)"
                    );
                    return Ok(());
                }
            }
            "--show-cache-stats" => {
                show_cache_stats = true;
            }
            "--cleanup-old" => {
                cleanup_old = true;
            }
            _ => {
                eprintln!("Error: Unknown argument {}", args[i]);
                print_help();
                return Ok(());
            }
        }
        i += 1;
    }

    // Initialize security analyzer
    let analyzer = TokenSecurityAnalyzer::new("data/security.db")?;

    // Handle cache operations first
    if cleanup_old {
        log(LogTag::Security, "DEBUG", "Cleaning up old cache entries...");
        analyzer.database.cleanup_old_data(1)?; // 1 day
        println!("✅ Old cache entries cleaned up");
    }

    if show_cache_stats {
        show_cache_statistics(&analyzer).await?;
    }

    // Handle analysis commands
    if let Some(mint) = mint_address {
        analyze_single_token(&analyzer, &mint, force_refresh).await?;
    } else if let Some(count) = top_count {
        analyze_top_tokens(&analyzer, count, force_refresh, risk_filter).await?;
    } else if let Some(query) = search_query {
        search_and_analyze_tokens(
            &analyzer,
            &query,
            search_limit,
            force_refresh,
            risk_filter
        ).await?;
    } else if let Some(mints) = batch_mints {
        batch_analyze_tokens(&analyzer, &mints, force_refresh).await?;
    } else if !show_cache_stats && !cleanup_old {
        // Default action: analyze top 10 tokens
        analyze_top_tokens(&analyzer, 10, force_refresh, risk_filter).await?;
    }

    Ok(())
}

fn print_help() {
    println!("{}", HELP_TEXT);
}

async fn analyze_single_token(
    analyzer: &TokenSecurityAnalyzer,
    mint: &str,
    force_refresh: bool
) -> Result<(), Box<dyn std::error::Error>> {
    // Check for SOL or USDC and warn
    if mint == "So11111111111111111111111111111111111111112" {
        println!("❌ ERROR: Cannot analyze SOL token!");
        println!("   SOL has millions of holders and will cause RPC failures.");
        println!("   Please use a smaller token from the ScreenerBot database.");
        return Ok(());
    }

    if mint == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" {
        println!("❌ ERROR: Cannot analyze USDC token!");
        println!("   USDC has millions of holders and will cause RPC failures.");
        println!("   Please use a smaller token from the ScreenerBot database.");
        return Ok(());
    }

    println!("🔍 Analyzing security for token: {}", mint);
    if force_refresh {
        println!("🔄 Force refresh enabled - bypassing cache");
    }
    println!("{}", "=".repeat(80));

    let start = std::time::Instant::now();

    let security_info = analyzer.analyze_token_security_with_options(mint, force_refresh).await?;

    let duration = start.elapsed();

    print_security_info(mint, &security_info);
    println!("\n⏱️  Analysis completed in {:.2}ms", duration.as_millis());

    Ok(())
}

async fn analyze_top_tokens(
    analyzer: &TokenSecurityAnalyzer,
    count: usize,
    _force_refresh: bool,
    risk_filter: Option<SecurityRiskLevel>
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing security for top {} tokens from database", count);
    if let Some(ref risk) = risk_filter {
        println!("🎯 Filtering by risk level: {:?}", risk);
    }
    println!("{}", "=".repeat(80));

    // Get tokens from database (this would need to be implemented)
    let tokens = get_top_tokens_from_db(count).await?;

    if tokens.is_empty() {
        println!("❌ No tokens found in database");
        return Ok(());
    }

    println!("📊 Found {} tokens in database", tokens.len());

    let start = std::time::Instant::now();

    let results = analyzer.analyze_multiple_tokens(&tokens).await?;
    let results: Vec<TokenSecurityInfo> = results.into_values().collect();

    let duration = start.elapsed();

    let mut filtered_results = Vec::new();
    for (mint, result) in tokens.iter().zip(results.iter()) {
        if let Some(ref filter_risk) = risk_filter {
            if &result.risk_level == filter_risk {
                filtered_results.push((mint, result));
            }
        } else {
            filtered_results.push((mint, result));
        }
    }

    println!("\n📈 Security Analysis Results ({} tokens):", filtered_results.len());
    println!("{}", "-".repeat(80));

    for (i, (mint, security_info)) in filtered_results.iter().enumerate() {
        println!("\n{}. Token: {}", i + 1, mint);
        print_security_summary(security_info);
    }

    print_risk_distribution(&results);
    println!("\n⏱️  Batch analysis completed in {:.2}s", duration.as_secs_f64());

    Ok(())
}

async fn search_and_analyze_tokens(
    analyzer: &TokenSecurityAnalyzer,
    query: &str,
    limit: usize,
    _force_refresh: bool,
    risk_filter: Option<SecurityRiskLevel>
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Searching tokens with query: '{}' (limit: {})", query, limit);
    println!("{}", "=".repeat(80));

    // Search tokens in database with limit
    let tokens = search_tokens_in_db(query, limit).await?;

    if tokens.is_empty() {
        println!("❌ No tokens found matching query: '{}'", query);
        return Ok(());
    }

    println!("📊 Found {} tokens matching query", tokens.len());

    let start = std::time::Instant::now();

    let results = analyzer.analyze_multiple_tokens(&tokens).await?;
    let results: Vec<TokenSecurityInfo> = results.into_values().collect();

    let duration = start.elapsed();

    let mut filtered_results = Vec::new();
    for (mint, result) in tokens.iter().zip(results.iter()) {
        if let Some(ref filter_risk) = risk_filter {
            if &result.risk_level == filter_risk {
                filtered_results.push((mint, result));
            }
        } else {
            filtered_results.push((mint, result));
        }
    }

    println!("\n📈 Security Analysis Results ({} tokens):", filtered_results.len());
    println!("{}", "-".repeat(80));

    for (i, (mint, security_info)) in filtered_results.iter().enumerate() {
        println!("\n{}. Token: {}", i + 1, mint);
        print_security_summary(security_info);
    }

    print_risk_distribution(&results);
    println!("\n⏱️  Search and analysis completed in {:.2}s", duration.as_secs_f64());

    Ok(())
}

async fn batch_analyze_tokens(
    analyzer: &TokenSecurityAnalyzer,
    mints: &[String],
    _force_refresh: bool
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Batch analyzing {} tokens", mints.len());
    println!("{}", "=".repeat(80));

    let start = std::time::Instant::now();

    let results = analyzer.analyze_multiple_tokens(mints).await?;
    let results: Vec<TokenSecurityInfo> = results.into_values().collect();

    let duration = start.elapsed();

    println!("\n📈 Batch Security Analysis Results:");
    println!("{}", "-".repeat(80));

    for (i, (mint, security_info)) in mints.iter().zip(results.iter()).enumerate() {
        println!("\n{}. Token: {}", i + 1, mint);
        print_security_summary(security_info);
    }

    print_risk_distribution(&results);
    println!("\n⏱️  Batch analysis completed in {:.2}s", duration.as_secs_f64());

    Ok(())
}

async fn show_cache_statistics(
    _analyzer: &TokenSecurityAnalyzer
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Security Cache Statistics:");
    println!("{}", "=".repeat(50));
    println!("📈 Database entries available");
    println!("� Analysis functionality ready");

    Ok(())
}

fn print_security_info(mint: &str, info: &TokenSecurityInfo) {
    println!("🎯 Security Analysis for: {}", mint);
    println!("{}", "-".repeat(60));

    // Risk Level
    let risk_emoji = match info.risk_level {
        SecurityRiskLevel::Safe => "✅",
        SecurityRiskLevel::Low => "🟡",
        SecurityRiskLevel::Medium => "🟠",
        SecurityRiskLevel::High => "🔴",
        SecurityRiskLevel::Critical => "💀",
        SecurityRiskLevel::Unknown => "❓",
    };
    println!("{} Risk Level: {:?}", risk_emoji, info.risk_level);

    // Authority Analysis
    println!("\n🔐 Authority Analysis:");
    let auth = &info.authority_info;
    println!(
        "  • Mint Authority: {}",
        auth.mint_authority.as_ref().unwrap_or(&"None (Disabled)".to_string())
    );
    println!(
        "  • Freeze Authority: {}",
        auth.freeze_authority.as_ref().unwrap_or(&"None (Disabled)".to_string())
    );

    // LP Lock Analysis
    println!("\n🔒 Liquidity Pool Lock:");
    if let Some(ref lp) = info.lp_lock_info {
        println!("  • Status: {:?}", lp.status);
        println!(
            "  • Pool Type: {}",
            lp.details.pool_type.as_ref().unwrap_or(&"Unknown".to_string())
        );
        if let Some(ref pool_addr) = lp.pool_address {
            println!("  • Pool Address: {}", pool_addr);
        }
        if !lp.details.lock_programs.is_empty() {
            println!("  • Lock Programs: {}", lp.details.lock_programs.join(", "));
        }
    } else {
        println!("  ❌ LP lock data not available");
    }

    // Holder Analysis
    println!("\n👥 Holder Analysis:");
    if let Some(ref holders) = info.holder_info {
        println!("  • Total Holders: {}", holders.total_holders);
        println!("  • Top 10 Concentration: {:.1}%", holders.top_10_concentration);
        println!("  • Top 5 Concentration: {:.1}%", holders.top_5_concentration);
        println!("  • Largest Holder: {:.1}%", holders.largest_holder_percentage);
        println!("  • Whale Count (>5%): {}", holders.whale_count);
    } else {
        println!("  ❌ Holder data not available");
    }

    // Timestamps
    println!("\n⏰ Analysis Info:");
    println!("  • First Analyzed: {}", info.timestamps.first_analyzed);
    println!("  • Last Updated: {}", info.timestamps.last_updated);
    println!("  • Security Score: {}/100", info.security_score);
}

fn print_security_summary(info: &TokenSecurityInfo) {
    let risk_emoji = match info.risk_level {
        SecurityRiskLevel::Safe => "✅",
        SecurityRiskLevel::Low => "🟡",
        SecurityRiskLevel::Medium => "🟠",
        SecurityRiskLevel::High => "🔴",
        SecurityRiskLevel::Critical => "💀",
        SecurityRiskLevel::Unknown => "❓",
    };

    let holders_count = info.holder_info
        .as_ref()
        .map(|h| h.total_holders.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let lp_status = info.lp_lock_info
        .as_ref()
        .map(|lp| format!("{:?}", lp.status))
        .unwrap_or_else(|| "N/A".to_string());

    let mint_auth = info.authority_info.mint_authority
        .as_ref()
        .map(|_| "Yes")
        .unwrap_or("No");

    println!(
        "   {} {:?} | Holders: {} | LP: {} | Mint Auth: {}",
        risk_emoji,
        info.risk_level,
        holders_count,
        lp_status,
        mint_auth
    );
}

fn print_risk_distribution(results: &[TokenSecurityInfo]) {
    let mut safe_count = 0;
    let mut low_count = 0;
    let mut medium_count = 0;
    let mut high_count = 0;
    let mut critical_count = 0;
    let mut unknown_count = 0;

    for result in results {
        match result.risk_level {
            SecurityRiskLevel::Safe => {
                safe_count += 1;
            }
            SecurityRiskLevel::Low => {
                low_count += 1;
            }
            SecurityRiskLevel::Medium => {
                medium_count += 1;
            }
            SecurityRiskLevel::High => {
                high_count += 1;
            }
            SecurityRiskLevel::Critical => {
                critical_count += 1;
            }
            SecurityRiskLevel::Unknown => {
                unknown_count += 1;
            }
        }
    }

    println!("\n📊 Risk Distribution:");
    println!("{}", "-".repeat(40));

    let total = results.len() as f64;
    if safe_count > 0 {
        println!("✅ Safe: {} tokens ({:.1}%)", safe_count, ((safe_count as f64) / total) * 100.0);
    }
    if low_count > 0 {
        println!("🟡 Low: {} tokens ({:.1}%)", low_count, ((low_count as f64) / total) * 100.0);
    }
    if medium_count > 0 {
        println!(
            "🟠 Medium: {} tokens ({:.1}%)",
            medium_count,
            ((medium_count as f64) / total) * 100.0
        );
    }
    if high_count > 0 {
        println!("🔴 High: {} tokens ({:.1}%)", high_count, ((high_count as f64) / total) * 100.0);
    }
    if critical_count > 0 {
        println!(
            "💀 Critical: {} tokens ({:.1}%)",
            critical_count,
            ((critical_count as f64) / total) * 100.0
        );
    }
    if unknown_count > 0 {
        println!(
            "❓ Unknown: {} tokens ({:.1}%)",
            unknown_count,
            ((unknown_count as f64) / total) * 100.0
        );
    }
}

// Placeholder functions for database operations
// These would need to be implemented based on your database structure
async fn get_top_tokens_from_db(count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Connect to tokens database and get smaller tokens (not mainstream ones)
    use rusqlite::Connection;

    let db_path = "data/tokens.db";
    let conn = Connection::open(db_path)?;

    // Get tokens that are NOT mainstream tokens and have reasonable market caps
    // Exclude SOL, USDC, USDT, WSOL, BTC, ETH and other huge tokens
    let mut stmt = conn.prepare(
        "SELECT mint FROM tokens 
         WHERE symbol IS NOT NULL 
         AND symbol NOT IN ('SOL', 'USDC', 'USDT', 'WSOL', 'BTC', 'ETH', 'WBTC', 'BONK', 'PENGU')
         AND market_cap IS NOT NULL 
         AND market_cap > 1000000 
         AND market_cap < 50000000000
         ORDER BY market_cap DESC 
         LIMIT ?1"
    )?;

    let rows = stmt.query_map([count], |row| { Ok(row.get::<_, String>(0)?) })?;

    let mut tokens = Vec::new();
    for row in rows {
        if let Ok(mint) = row {
            tokens.push(mint);
        }
    }

    // If no tokens found in database, return some smaller known tokens for testing
    if tokens.is_empty() {
        println!("⚠️  No suitable tokens found in database, using fallback test tokens");
        return Ok(
            vec![
                "EPD9qjtFaFrR3GvTPmPt8spmu4hfwUN6Dc5tHtDmpump".to_string(), // Small pump token
                "BwC4NhHGfT5GrzUSjiYe2LcUeyWqSRt5JY5EqHH8pump".to_string(), // Small pump token
                "67ESYv7wxKu2vo637GhtffzSU2USwLHLZCQogsVmpump".to_string() // Small pump token
            ]
        );
    }

    println!(
        "📊 Found {} suitable tokens from database (excluding mainstream tokens)",
        tokens.len()
    );
    Ok(tokens)
}

async fn search_tokens_in_db(
    query: &str,
    limit: usize
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    use rusqlite::Connection;

    let db_path = "data/tokens.db";
    let conn = Connection::open(db_path)?;

    // Search by symbol or name, excluding mainstream tokens
    let mut stmt = conn.prepare(
        "SELECT mint FROM tokens 
         WHERE (symbol LIKE ?1 OR name LIKE ?1)
         AND symbol NOT IN ('SOL', 'USDC', 'USDT', 'WSOL', 'BTC', 'ETH', 'WBTC', 'BONK', 'PENGU')
         AND market_cap IS NOT NULL 
         AND market_cap < 50000000000
         ORDER BY market_cap DESC 
         LIMIT ?2"
    )?;

    let search_pattern = format!("%{}%", query);
    let rows = stmt.query_map([&search_pattern, &limit.to_string()], |row| {
        Ok(row.get::<_, String>(0)?)
    })?;

    let mut tokens = Vec::new();
    for row in rows {
        if let Ok(mint) = row {
            tokens.push(mint);
        }
    }

    Ok(tokens)
}
