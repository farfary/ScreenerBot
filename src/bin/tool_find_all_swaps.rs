/// Comprehensive Swap Transaction Discovery Tool
/// 
/// This tool analyzes all wallet transactions to find swap operations:
/// - Scans entire transaction history for swap patterns
/// - Detects token purchases and sales from instruction analysis
/// - Extracts swap amounts, prices, and fees
/// - Identifies swap routers (Jupiter, Raydium, etc.)
/// - Provides detailed swap analytics and statistics
/// - Exports swap data to JSON for further analysis
///
/// Usage:
///   cargo run --bin tool_find_all_swaps [--wallet WALLET] [--limit LIMIT] [--export] [--detailed]

use screenerbot::{
    rpc::{init_rpc_client, get_rpc_client},
    logger::{init_file_logging, log, LogTag},
    global::read_configs,
    transactions_tools::{
        Args, analyze_wallet_swaps, get_all_configured_wallets,
        display_comprehensive_results, export_results_to_json
    },
};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();
    
    let args = Args::parse();
    
    log(LogTag::System, "INFO", "🔍 Starting comprehensive swap transaction discovery");
    
    // Initialize RPC client
    init_rpc_client()?;
    let _rpc_client = get_rpc_client();
    
    // Read configurations
    let _configs = read_configs().map_err(|e| format!("Failed to read configs: {}", e))?;
    
    // Determine which wallets to analyze
    let wallets_to_analyze = if let Some(ref wallet_addr) = args.wallet {
        vec![wallet_addr.clone()]
    } else {
        // Get all configured wallets
        get_all_configured_wallets().await?
    };
    
    if wallets_to_analyze.is_empty() {
        return Err("No wallets found to analyze".into());
    }
    
    log(LogTag::System, "INFO", &format!("📊 Analyzing {} wallet(s)", wallets_to_analyze.len()));
    
    let mut all_reports = Vec::new();
    
    for wallet_address in &wallets_to_analyze {
        log(LogTag::System, "INFO", &format!("🔍 Analyzing wallet: {}", &wallet_address[..8]));
        
        match analyze_wallet_swaps(wallet_address, &args).await {
            Ok(report) => {
                log(LogTag::System, "SUCCESS", &format!(
                    "✅ Found {} swaps in wallet {}", 
                    report.analytics.total_swaps, 
                    &wallet_address[..8]
                ));
                all_reports.push(report);
            },
            Err(e) => {
                log(LogTag::System, "ERROR", &format!(
                    "❌ Failed to analyze wallet {}: {}", 
                    &wallet_address[..8], 
                    e
                ));
            }
        }
    }
    
    // Display comprehensive results
    display_comprehensive_results(&all_reports, &args)?;
    
    // Export to JSON if requested
    if args.export {
        export_results_to_json(&all_reports, &args)?;
    }
    
    log(LogTag::System, "SUCCESS", "🎉 Swap discovery analysis completed successfully");
    
    Ok(())
}