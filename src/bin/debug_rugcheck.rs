/// Debug tool for Rugcheck API
///
/// Tests the Rugcheck token security report endpoint.
///
/// Usage:
///   cargo run --bin debug_rugcheck
///   cargo run --bin debug_rugcheck --mint <MINT_ADDRESS>
///   cargo run --bin debug_rugcheck --bonk

use clap::Parser;
use screenerbot::tokens_new::api::rugcheck::RugcheckClient;

#[derive(Parser, Debug)]
#[command(name = "debug_rugcheck")]
#[command(about = "Test Rugcheck API endpoints")]
struct Args {
    /// Specific mint address to test
    #[arg(long)]
    mint: Option<String>,

    /// Test with BONK token (DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263)
    #[arg(long)]
    bonk: bool,

    /// Test with SOL (So11111111111111111111111111111111111111112)
    #[arg(long)]
    sol: bool,
}

fn print_separator() {
    println!("\n{}", "=".repeat(80));
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    println!("Rugcheck API Debug Tool");
    print_separator();

    // Determine which mint to test
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

    println!("\nTesting with mint: {}", test_mint);
    print_separator();

    // Create client (enabled=true, 60 req/min, 30sec timeout)
    let client = match RugcheckClient::new(true, 60, 30) {
        Ok(c) => c,
        Err(e) => {
            println!("✗ Failed to create Rugcheck client: {}", e);
            return;
        }
    };

    // Test fetch_report
    println!("\n[TEST] Fetching security report...");
    match client.fetch_report(&test_mint).await {
        Ok(report) => {
            println!("✓ Successfully fetched report");
            println!("\n[REPORT SUMMARY]");
            println!("  Mint: {}", report.mint);
            println!("  Token Program: {}", report.token_program.as_deref().unwrap_or("N/A"));
            println!("  Token Type: {}", report.token_type.as_deref().unwrap_or("N/A"));
            
            // Token info
            println!("\n[TOKEN INFO]");
            println!("  Name: {}", report.token_name.as_deref().unwrap_or("N/A"));
            println!("  Symbol: {}", report.token_symbol.as_deref().unwrap_or("N/A"));
            println!("  Decimals: {}", report.token_decimals.unwrap_or(0));
            println!("  Supply: {}", report.token_supply.as_deref().unwrap_or("N/A"));
            println!("  URI: {}", report.token_uri.as_deref().unwrap_or("N/A"));
            println!("  Mutable: {}", report.token_mutable.unwrap_or(false));
            
            // Authorities
            println!("\n[AUTHORITIES]");
            println!("  Mint Authority: {}", report.mint_authority.as_deref().unwrap_or("None"));
            println!("  Freeze Authority: {}", report.freeze_authority.as_deref().unwrap_or("None"));
            println!("  Update Authority: {}", report.token_update_authority.as_deref().unwrap_or("None"));
            
            // Creator
            println!("\n[CREATOR]");
            println!("  Address: {}", report.creator.as_deref().unwrap_or("N/A"));
            println!("  Balance: {}", report.creator_balance.unwrap_or(0));
            println!("  Tokens: {}", report.creator_tokens.as_deref().unwrap_or("N/A"));
            
            // Security score
            println!("\n[SECURITY SCORE]");
            println!("  Score: {}", report.score.unwrap_or(0));
            println!("  Score Normalized: {}", report.score_normalised.unwrap_or(0));
            println!("  Rugged: {}", report.rugged);
            
            // Risks
            println!("\n[RISKS] ({} total)", report.risks.len());
            if report.risks.is_empty() {
                println!("  No risks detected");
            } else {
                for (i, risk) in report.risks.iter().take(5).enumerate() {
                    println!("  {}. [{}] {} (score: {})", 
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
            println!("  Total Market Liquidity: ${:.2}", report.total_market_liquidity.unwrap_or(0.0));
            println!("  Total Stable Liquidity: ${:.2}", report.total_stable_liquidity.unwrap_or(0.0));
            println!("  Total LP Providers: {}", report.total_lp_providers.unwrap_or(0));
            
            // Holders
            println!("\n[HOLDERS]");
            println!("  Total Holders: {}", report.total_holders.unwrap_or(0));
            println!("  Top Holders: {} listed", report.top_holders.len());
            println!("  Graph Insiders Detected: {}", report.graph_insiders_detected.unwrap_or(0));
            
            if !report.top_holders.is_empty() {
                println!("\n[TOP 5 HOLDERS]");
                for (i, holder) in report.top_holders.iter().take(5).enumerate() {
                    println!("  {}. {} ({:.2}%)", 
                        i + 1,
                        &holder.address[..8],
                        holder.pct
                    );
                    println!("     Amount: {}", holder.amount);
                    println!("     Owner: {}", holder.owner.as_deref().unwrap_or("N/A"));
                    println!("     Insider: {}", holder.insider);
                }
            }
            
            // Transfer fee
            println!("\n[TRANSFER FEE]");
            println!("  Percentage: {}%", report.transfer_fee_pct.unwrap_or(0.0));
            println!("  Max Amount: {}", report.transfer_fee_max_amount.unwrap_or(0));
            println!("  Authority: {}", report.transfer_fee_authority.as_deref().unwrap_or("None"));
            
            // Metadata
            println!("\n[METADATA]");
            println!("  Detected At: {}", report.detected_at.as_deref().unwrap_or("N/A"));
            println!("  Analyzed At: {}", report.analyzed_at.as_deref().unwrap_or("N/A"));
            println!("  Fetched At: {}", report.fetched_at);
        }
        Err(e) => {
            println!("✗ Failed to fetch report: {}", e);
        }
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
    println!("  Avg Response Time: {:.2}ms", stats.average_response_time_ms);
    
    print_separator();
    println!("\nTest completed!");
}
