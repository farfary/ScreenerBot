/// Debug tool for Rugcheck API
///
/// Tests the Rugcheck API endpoints (report + stats).
///
/// Usage:
///   cargo run --bin debug_rugcheck                    # Test report with BONK
///   cargo run --bin debug_rugcheck --mint <MINT>      # Test report with custom mint
///   cargo run --bin debug_rugcheck --bonk             # Test report with BONK
///   cargo run --bin debug_rugcheck --sol              # Test report with SOL
///   cargo run --bin debug_rugcheck --new-tokens       # Test new tokens endpoint
///   cargo run --bin debug_rugcheck --recent           # Test recent (most viewed) endpoint
///   cargo run --bin debug_rugcheck --trending         # Test trending endpoint
///   cargo run --bin debug_rugcheck --verified         # Test verified tokens endpoint
///   cargo run --bin debug_rugcheck --all-stats        # Test all stats endpoints
use clap::Parser;
use screenerbot::tokens_new::api::rugcheck::RugcheckClient;

#[derive(Parser, Debug)]
#[command(name = "debug_rugcheck")]
#[command(about = "Test Rugcheck API endpoints")]
struct Args {
    /// Specific mint address to test (for report endpoint)
    #[arg(long)]
    mint: Option<String>,

    /// Test report with BONK token
    #[arg(long)]
    bonk: bool,

    /// Test report with SOL token
    #[arg(long)]
    sol: bool,

    /// Test /v1/stats/new_tokens endpoint
    #[arg(long)]
    new_tokens: bool,

    /// Test /v1/stats/recent endpoint (most viewed)
    #[arg(long)]
    recent: bool,

    /// Test /v1/stats/trending endpoint
    #[arg(long)]
    trending: bool,

    /// Test /v1/stats/verified endpoint
    #[arg(long)]
    verified: bool,

    /// Test all stats endpoints
    #[arg(long)]
    all_stats: bool,
}

fn print_separator() {
    println!("\n{}", "=".repeat(80));
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Rugcheck API Debug Tool");
    print_separator();

    // Create client (enabled=true, 60 req/min, 30sec timeout)
    let client = match RugcheckClient::new(true, 60, 30) {
        Ok(c) => c,
        Err(e) => {
            println!("✗ Failed to create Rugcheck client: {}", e);
            return;
        }
    };

    // Determine what to test
    if args.new_tokens || args.all_stats {
        test_new_tokens(&client).await;
    }

    if args.recent || args.all_stats {
        test_recent_tokens(&client).await;
    }

    if args.trending || args.all_stats {
        test_trending_tokens(&client).await;
    }

    if args.verified || args.all_stats {
        test_verified_tokens(&client).await;
    }

    // If no stats flags, test report endpoint
    if !args.new_tokens && !args.recent && !args.trending && !args.verified && !args.all_stats {
        let test_mint = if let Some(mint) = args.mint {
            mint
        } else if args.bonk {
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string()
        } else if args.sol {
            "So11111111111111111111111111111111111111112".to_string()
        } else {
            // Default to BONK
            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string()
        };
        test_report(&client, &test_mint).await;
    }

    // Print stats
    print_separator();
    let stats = client.get_stats().await;
    println!("\n[API STATS]");
    println!("  Total Requests: {}", stats.total_requests);
    println!("  Successful: {}", stats.successful_requests);
    println!("  Failed: {}", stats.failed_requests);
    println!("  Cache Hits: {}", stats.cache_hits);
    println!("  Cache Misses: {}", stats.cache_misses);
    println!(
        "  Avg Response Time: {:.2}ms",
        stats.average_response_time_ms
    );

    print_separator();
    println!("\nTest completed!");
}

async fn test_report(client: &RugcheckClient, mint: &str) {
    println!("\n[TEST] Fetching security report for: {}", mint);
    print_separator();

    match client.fetch_report(mint).await {
        Ok(report) => {
            println!("✓ Successfully fetched report");
            println!("\n[REPORT SUMMARY]");
            println!("  Mint: {}", report.mint);
            println!(
                "  Token Program: {}",
                report.token_program.as_deref().unwrap_or("N/A")
            );
            println!(
                "  Token Type: {}",
                report.token_type.as_deref().unwrap_or("N/A")
            );

            // Token info
            println!("\n[TOKEN INFO]");
            println!("  Name: {}", report.token_name.as_deref().unwrap_or("N/A"));
            println!(
                "  Symbol: {}",
                report.token_symbol.as_deref().unwrap_or("N/A")
            );
            println!("  Decimals: {}", report.token_decimals.unwrap_or(0));
            println!(
                "  Supply: {}",
                report.token_supply.as_deref().unwrap_or("N/A")
            );
            println!("  URI: {}", report.token_uri.as_deref().unwrap_or("N/A"));
            println!("  Mutable: {}", report.token_mutable.unwrap_or(false));

            // Authorities
            println!("\n[AUTHORITIES]");
            println!(
                "  Mint Authority: {}",
                report.mint_authority.as_deref().unwrap_or("None")
            );
            println!(
                "  Freeze Authority: {}",
                report.freeze_authority.as_deref().unwrap_or("None")
            );
            println!(
                "  Update Authority: {}",
                report.token_update_authority.as_deref().unwrap_or("None")
            );

            // Creator
            println!("\n[CREATOR]");
            println!("  Address: {}", report.creator.as_deref().unwrap_or("N/A"));
            println!("  Balance: {}", report.creator_balance.unwrap_or(0));
            println!(
                "  Tokens: {}",
                report.creator_tokens.as_deref().unwrap_or("N/A")
            );

            // Security score
            println!("\n[SECURITY SCORE]");
            println!("  Score: {}", report.score.unwrap_or(0));
            println!(
                "  Score Normalized: {}",
                report.score_normalised.unwrap_or(0)
            );
            println!("  Rugged: {}", report.rugged);

            // Risks
            println!("\n[RISKS] ({} total)", report.risks.len());
            if report.risks.is_empty() {
                println!("  No risks detected");
            } else {
                for (i, risk) in report.risks.iter().take(5).enumerate() {
                    println!(
                        "  {}. [{}] {} (score: {})",
                        i + 1,
                        risk.level,
                        risk.name,
                        risk.score
                    );
                    println!("     Value: {}", risk.value);
                    println!("     Description: {}", risk.description);
                }
                if report.risks.len() > 5 {
                    println!("  ... and {} more risks", report.risks.len() - 5);
                }
            }

            // Market data
            println!("\n[MARKET DATA]");
            println!(
                "  Total Market Liquidity: ${:.2}",
                report.total_market_liquidity.unwrap_or(0.0)
            );
            println!(
                "  Total Stable Liquidity: ${:.2}",
                report.total_stable_liquidity.unwrap_or(0.0)
            );
            println!(
                "  Total LP Providers: {}",
                report.total_lp_providers.unwrap_or(0)
            );

            // Holders
            println!("\n[HOLDERS]");
            println!("  Total Holders: {}", report.total_holders.unwrap_or(0));
            println!("  Top Holders: {} listed", report.top_holders.len());
            println!(
                "  Graph Insiders Detected: {}",
                report.graph_insiders_detected.unwrap_or(0)
            );

            if !report.top_holders.is_empty() {
                println!("\n[TOP 5 HOLDERS]");
                for (i, holder) in report.top_holders.iter().take(5).enumerate() {
                    println!("  {}. {} ({:.2}%)", i + 1, &holder.address[..8], holder.pct);
                    println!("     Amount: {}", holder.amount);
                    println!("     Owner: {}", holder.owner.as_deref().unwrap_or("N/A"));
                    println!("     Insider: {}", holder.insider);
                }
            }

            // Transfer fee
            println!("\n[TRANSFER FEE]");
            println!("  Percentage: {}%", report.transfer_fee_pct.unwrap_or(0.0));
            println!(
                "  Max Amount: {}",
                report.transfer_fee_max_amount.unwrap_or(0)
            );
            println!(
                "  Authority: {}",
                report.transfer_fee_authority.as_deref().unwrap_or("None")
            );

            // Metadata
            println!("\n[METADATA]");
            println!(
                "  Detected At: {}",
                report.detected_at.as_deref().unwrap_or("N/A")
            );
            println!(
                "  Analyzed At: {}",
                report.analyzed_at.as_deref().unwrap_or("N/A")
            );
            println!("  Fetched At: {}", report.fetched_at);
        }
        Err(e) => {
            println!("✗ Failed to fetch report: {}", e);
        }
    }
}

async fn test_new_tokens(client: &RugcheckClient) {
    println!("\n[TEST] Fetching new tokens from /v1/stats/new_tokens");
    print_separator();

    match client.fetch_new_tokens().await {
        Ok(tokens) => {
            println!("✓ Successfully fetched {} new tokens", tokens.len());
            println!("\n[TOP 10 NEW TOKENS]");
            for (i, token) in tokens.iter().take(10).enumerate() {
                println!("  {}. {} ({})", i + 1, token.symbol, token.mint);
                println!(
                    "     Decimals: {}, Creator: {}",
                    token.decimals, token.creator
                );
                println!("     Created: {}", token.create_at);
            }
            if tokens.len() > 10 {
                println!("  ... and {} more tokens", tokens.len() - 10);
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch new tokens: {}", e);
        }
    }
}

async fn test_recent_tokens(client: &RugcheckClient) {
    println!("\n[TEST] Fetching recent (most viewed) tokens from /v1/stats/recent");
    print_separator();

    match client.fetch_recent_tokens().await {
        Ok(tokens) => {
            println!("✓ Successfully fetched {} recent tokens", tokens.len());
            println!("\n[TOP 10 MOST VIEWED TOKENS]");
            for (i, token) in tokens.iter().take(10).enumerate() {
                println!(
                    "  {}. {} - {} (score: {})",
                    i + 1,
                    token.metadata.symbol,
                    token.metadata.name,
                    token.score
                );
                println!("     Mint: {}", token.mint);
                println!(
                    "     Visits: {} (user: {})",
                    token.visits, token.user_visits
                );
                println!("     Mutable: {}", token.metadata.mutable);
            }
            if tokens.len() > 10 {
                println!("  ... and {} more tokens", tokens.len() - 10);
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch recent tokens: {}", e);
        }
    }
}

async fn test_trending_tokens(client: &RugcheckClient) {
    println!("\n[TEST] Fetching trending tokens from /v1/stats/trending");
    print_separator();

    match client.fetch_trending_tokens().await {
        Ok(tokens) => {
            println!("✓ Successfully fetched {} trending tokens", tokens.len());
            println!("\n[TOP 10 TRENDING TOKENS]");
            for (i, token) in tokens.iter().take(10).enumerate() {
                println!("  {}. {} ({})", i + 1, &token.mint[..20], token.mint);
                println!("     Votes: {} (up: {})", token.vote_count, token.up_count);
                let up_pct = if token.vote_count > 0 {
                    (token.up_count as f64 / token.vote_count as f64) * 100.0
                } else {
                    0.0
                };
                println!("     Up Percentage: {:.1}%", up_pct);
            }
            if tokens.len() > 10 {
                println!("  ... and {} more tokens", tokens.len() - 10);
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch trending tokens: {}", e);
        }
    }
}

async fn test_verified_tokens(client: &RugcheckClient) {
    println!("\n[TEST] Fetching verified tokens from /v1/stats/verified");
    print_separator();

    match client.fetch_verified_tokens().await {
        Ok(tokens) => {
            println!("✓ Successfully fetched {} verified tokens", tokens.len());
            println!("\n[TOP 10 VERIFIED TOKENS]");
            for (i, token) in tokens.iter().take(10).enumerate() {
                println!("  {}. {} - {}", i + 1, token.symbol, token.name);
                println!("     Mint: {}", token.mint);
                println!(
                    "     Jupiter Verified: {}, Strict: {}",
                    token.jup_verified, token.jup_strict
                );
                println!("     Payer: {}", token.payer);
                // Safely truncate description respecting UTF-8 char boundaries
                let desc_preview = if token.description.chars().count() > 60 {
                    let truncated: String = token.description.chars().take(60).collect();
                    format!("{}...", truncated)
                } else {
                    token.description.clone()
                };
                println!("     Description: {}", desc_preview);
            }
            if tokens.len() > 10 {
                println!("  ... and {} more tokens", tokens.len() - 10);
            }
        }
        Err(e) => {
            println!("✗ Failed to fetch verified tokens: {}", e);
        }
    }
}
