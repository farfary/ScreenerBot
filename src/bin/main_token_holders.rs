/// Token Holder Analysis Tool
///
/// This binary provides comprehensive token holder analysis by querying Solana RPC directly.
/// It can count holders, analyze top holders, and compute holder statistics.
///
/// Usage:
///   cargo run --bin main_token_holders -- --mint <MINT_ADDRESS>
///   cargo run --bin main_token_holders -- --mint <MINT_ADDRESS> --analyze-top-holders
///   cargo run --bin main_token_holders -- --mint <MINT_ADDRESS> --analyze-top-holders --limit 20
///   cargo run --bin main_token_holders -- --mint <MINT_ADDRESS> --stats
///   cargo run --bin main_token_holders -- --mint HZjwdor9NdCBod1ka1AE5TWXzjSHezYsEfjoWom4pump --analyze-top-holders
///
/// Features:
///   - Count total token holders
///   - Analyze top N holders with balances and addresses
///   - Compute holder statistics (average, median, top 10 concentration)
///   - Support for both SPL Token and Token-2022 programs
///   - Safe string handling with no panic on short addresses
use screenerbot::{
    global::read_configs,
    logger::{ init_file_logging, log, LogTag },
    tokens::holders::{ get_count_holders, get_holder_stats, get_top_holders_analysis },
    utils::safe_truncate,
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Token Holder Analysis Tool");
    println!("=============================");

    // Initialize logging
    init_file_logging();
    log(LogTag::System, "INFO", "Token holder analysis starting...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let mut mint_address = "HZjwdor9NdCBod1ka1AE5TWXzjSHezYsEfjoWom4pump"; // Default safe test token
    let mut analyze_top_holders = false;
    let mut show_stats = false;
    let mut limit = 50u32;

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mint" => {
                if i + 1 < args.len() {
                    mint_address = &args[i + 1];
                    i += 2;
                } else {
                    eprintln!("Error: --mint requires a value");
                    return Ok(());
                }
            }
            "--analyze-top-holders" => {
                analyze_top_holders = true;
                i += 1;
            }
            "--stats" => {
                show_stats = true;
                i += 1;
            }
            "--limit" => {
                if i + 1 < args.len() {
                    limit = args[i + 1].parse().unwrap_or(50);
                    i += 2;
                } else {
                    eprintln!("Error: --limit requires a value");
                    return Ok(());
                }
            }
            "--help" => {
                println!("Usage:");
                println!("  --mint <ADDRESS>           Token mint address to analyze");
                println!("  --analyze-top-holders      Analyze top holders (not just count)");
                println!(
                    "  --stats                    Show holder statistics (average, median, concentration)"
                );
                println!(
                    "  --limit <NUMBER>           Number of top holders to show (default: 50)"
                );
                println!("  --help                     Show this help message");
                return Ok(());
            }
            _ => {
                i += 1;
            }
        }
    }

    println!("🎯 Target Token: {}", mint_address);
    if analyze_top_holders {
        println!("📊 Analysis Mode: Top {} Holders", limit);
    } else if show_stats {
        println!("📊 Analysis Mode: Holder Statistics");
    } else {
        println!("📊 Analysis Mode: Count Only");
    }
    println!("📡 Querying Solana RPC for token holders...\n");

    let configs = read_configs().map_err(|e| format!("Failed to read configs: {}", e))?;

    println!("🔗 RPC Endpoints:");
    println!("   Primary: {}", configs.rpc_url);
    println!("   Premium: {}", configs.rpc_url_premium);
    println!();

    let start_time = Instant::now();

    if analyze_top_holders {
        println!("📊 Analyzing top {} token holders...", limit);
        match get_top_holders_analysis(mint_address, Some(limit)).await {
            Ok(analysis) => {
                println!("   ✅ Total holders: {}", analysis.total_holders);
                println!("   📋 Total accounts: {}", analysis.total_accounts);
                println!("   🏷️  Token type: {}", if analysis.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                });
                println!("\n🏆 Top {} Holders:", analysis.top_holders.len());
                println!("   {:<44} {:>20} {:>15}", "Owner", "UI Amount", "Raw Amount");
                println!("   {}", "─".repeat(80));

                for (i, holder) in analysis.top_holders.iter().enumerate() {
                    println!(
                        "{:2}. {:<44} {:>20.6} {:>15}",
                        i + 1,
                        safe_truncate(&holder.owner, 44), // Safe truncation for display
                        holder.ui_amount,
                        holder.amount
                    );
                }
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }
    } else if show_stats {
        println!("📊 Computing holder statistics...");
        match get_holder_stats(mint_address).await {
            Ok(stats) => {
                println!("   ✅ Total holders: {}", stats.total_holders);
                println!("   📋 Total accounts: {}", stats.total_accounts);
                println!("   🏷️  Token type: {}", if stats.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                });
                println!("   📈 Average balance: {:.6}", stats.average_balance);
                println!("   📊 Median balance: {:.6}", stats.median_balance);
                println!("   🎯 Top 10 concentration: {:.2}%", stats.top_10_concentration);
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }
    } else {
        println!("📊 Counting token holders...");
        match get_count_holders(mint_address).await {
            Ok(count) => println!("   ✅ Total holders: {}", count),
            Err(e) => println!("   ❌ Error: {}", e),
        }
    }

    let elapsed = start_time.elapsed();
    println!("\n⏱️  Total time: {:?}", elapsed);
    println!("✅ Token holder analysis completed!");

    Ok(())
}
