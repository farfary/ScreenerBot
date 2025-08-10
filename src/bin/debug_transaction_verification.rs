/// Simplified Transaction Verification Debug Tool
/// 
/// This tool analyzes transaction verification failures using the bot's existing infrastructure:
/// - Tests transaction verification methods step by step
/// - Analyzes RPC transaction fetching issues
/// - Debugs swap detection logic problems
/// - Provides actionable recommendations for fixes
///
/// Usage:
///   cargo run --bin debug_transaction_verification -- --signature <SIG>
///   cargo run --bin debug_transaction_verification -- --all-positions

use clap::Parser;
use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    positions::{get_open_positions, get_closed_positions},
    rpc::get_rpc_client,
    wallet_transactions::{initialize_wallet_transaction_manager, verify_swap_transaction_global},
    transactions_tools::{analyze_post_swap_transaction_simple},
    utils::get_wallet_address,
};
use serde::{Deserialize, Serialize};
use colored::Colorize;

#[derive(Parser)]
#[command(about = "Debug transaction verification failures")]
pub struct Args {
    /// Specific transaction signature to debug
    #[arg(short, long)]
    pub signature: Option<String>,
    
    /// Debug all positions with verification issues
    #[arg(short, long)]
    pub all_positions: bool,
    
    /// Enable verbose output for detailed analysis
    #[arg(short, long)]
    pub verbose: bool,
    
    /// Test RPC connectivity
    #[arg(short, long)]
    pub rpc_test: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugResult {
    pub signature: String,
    pub global_verification_success: bool,
    pub global_verification_error: Option<String>,
    pub simple_analysis_success: bool,
    pub simple_analysis_error: Option<String>,
    pub rpc_fetch_success: bool,
    pub rpc_fetch_error: Option<String>,
    pub wallet_address: String,
    pub recommendations: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging system
    init_file_logging();
    
    // Enable debug transactions for this session
    {
        use screenerbot::global::CMD_ARGS;
        if let Ok(mut cmd_args) = CMD_ARGS.lock() {
            cmd_args.push("--debug-transactions".to_string());
        }
    }
    
    log(LogTag::Transactions, "INFO", "🔍 STARTING TRANSACTION VERIFICATION DEBUGGING");
    
    // Initialize global transaction manager
    initialize_wallet_transaction_manager().await?;
    
    if args.rpc_test {
        test_rpc_basic().await;
    }
    
    if args.all_positions {
        debug_all_positions(&args).await?;
    } else if let Some(ref signature) = args.signature {
        debug_transaction_signature(signature, &args).await?;
    } else {
        eprintln!("Please specify --signature or --all-positions");
        std::process::exit(1);
    }
    
    Ok(())
}

async fn test_rpc_basic() {
    println!("\n{}", "🔗 TESTING BASIC RPC".bright_cyan().bold());
    
    let rpc_client = get_rpc_client();
    
    // Test basic connectivity
    match rpc_client.get_slot().await {
        Ok(slot) => println!("✅ Basic connectivity: Current slot {}", slot),
        Err(e) => println!("❌ Basic connectivity failed: {}", e),
    }
}

async fn debug_all_positions(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "📊 DEBUGGING ALL POSITIONS".bright_cyan().bold());
    
    let open_positions = get_open_positions();
    let closed_positions = get_closed_positions();
    
    // Combine all positions
    let mut all_positions = open_positions;
    all_positions.extend(closed_positions);
    
    let mut unverified_count = 0;
    let mut verified_count = 0;
    
    for position in &all_positions {
        let is_verified = position.transaction_entry_verified && 
                         position.effective_entry_price.is_some() &&
                         position.token_amount.is_some();
        
        if !is_verified {
            unverified_count += 1;
            println!("\n{} Position: {} ({})", 
                "🔍".bright_yellow(), 
                position.symbol.bright_white(), 
                position.mint.bright_blue()
            );
            
            if let Some(signature) = &position.entry_transaction_signature {
                let result = debug_transaction_signature_internal(signature, args).await?;
                display_debug_result(&result, args.verbose);
            } else {
                println!("❌ No entry transaction signature found");
            }
        } else {
            verified_count += 1;
        }
    }
    
    println!("\n{}", "📈 SUMMARY".bright_green().bold());
    println!("✅ Verified positions: {}", verified_count);
    println!("❌ Unverified positions: {}", unverified_count);
    println!("📊 Total positions: {}", all_positions.len());
    
    Ok(())
}

async fn debug_transaction_signature(signature: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} {}", "🔍 DEBUGGING TRANSACTION".bright_cyan().bold(), signature.bright_blue());
    
    let result = debug_transaction_signature_internal(signature, args).await?;
    display_debug_result(&result, args.verbose);
    
    Ok(())
}

async fn debug_transaction_signature_internal(signature: &str, _args: &Args) -> Result<DebugResult, Box<dyn std::error::Error>> {
    let wallet_address = get_wallet_address().unwrap_or_default();
    
    let mut result = DebugResult {
        signature: signature.to_string(),
        global_verification_success: false,
        global_verification_error: None,
        simple_analysis_success: false,
        simple_analysis_error: None,
        rpc_fetch_success: false,
        rpc_fetch_error: None,
        wallet_address: wallet_address.clone(),
        recommendations: Vec::new(),
    };
    
    // Test 1: Global verification method
    println!("🏦 Testing global verification...");
    match verify_swap_transaction_global(signature, "buy").await {
        Ok(verified_data) => {
            result.global_verification_success = true;
            println!("✅ Global verification succeeded: {} SOL", verified_data.sol_amount);
        },
        Err(e) => {
            result.global_verification_error = Some(e.to_string());
            println!("❌ Global verification failed: {}", e);
        }
    }
    
    // Test 2: Simple analysis method
    println!("📊 Testing simple transaction analysis...");
    match analyze_post_swap_transaction_simple(signature, &wallet_address).await {
        Ok(_analysis) => {
            result.simple_analysis_success = true;
            println!("✅ Simple analysis succeeded");
        },
        Err(e) => {
            result.simple_analysis_error = Some(e.clone());
            println!("❌ Simple analysis failed: {}", e);
        }
    }
    
    // Test 3: Check if transaction exists in cache
    println!("💾 Checking transaction cache...");
    let rpc_client = get_rpc_client();
    match rpc_client.get_transaction_details(signature).await {
        Ok(_) => {
            result.rpc_fetch_success = true;
            println!("✅ Transaction found via RPC");
        },
        Err(e) => {
            result.rpc_fetch_error = Some(e.to_string());
            println!("❌ Transaction fetch failed: {}", e);
        }
    }
    
    // Generate recommendations
    result.recommendations = generate_recommendations(&result);
    
    Ok(result)
}

fn generate_recommendations(result: &DebugResult) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    if !result.global_verification_success {
        if let Some(error) = &result.global_verification_error {
            if error.contains("Transaction not found") {
                recommendations.push("🔧 Transaction not found in cache - check wallet transaction manager sync".to_string());
            } else if error.contains("No swap detected") {
                recommendations.push("📊 Swap detection failing - check swap detection logic in transactions_tools.rs".to_string());
            } else if error.contains("Failed to decode") {
                recommendations.push("⚠️  Transaction decoding error - check RPC transaction format handling".to_string());
            } else if error.contains("WrongSize") {
                recommendations.push("⚠️  RPC WrongSize error - need to adjust transaction fetching parameters".to_string());
            } else {
                recommendations.push(format!("❌ Global verification error: {}", error));
            }
        }
    }
    
    if !result.simple_analysis_success {
        if let Some(error) = &result.simple_analysis_error {
            if error.contains("No swap detected") {
                recommendations.push("🔍 Simple analysis swap detection failing - check instruction parsing".to_string());
            } else if error.contains("Transaction not found") {
                recommendations.push("💾 Transaction missing from analysis - check cache sync and RPC fetch".to_string());
            } else {
                recommendations.push(format!("📊 Simple analysis error: {}", error));
            }
        }
    }
    
    if !result.rpc_fetch_success {
        if let Some(error) = &result.rpc_fetch_error {
            if error.contains("WrongSize") {
                recommendations.push("⚠️  RPC WrongSize error detected - check transaction encoding parameters".to_string());
            } else if error.contains("not found") {
                recommendations.push("🔍 Transaction not found on RPC - check if signature is correct".to_string());
            } else {
                recommendations.push(format!("🔗 RPC fetch error: {}", error));
            }
        }
    }
    
    if !result.global_verification_success && !result.simple_analysis_success {
        recommendations.push("🚨 ALL verification methods failing - likely transaction format or swap detection issue".to_string());
        recommendations.push("💡 Check logs for 'WrongSize' or 'Failed to decode' errors".to_string());
        recommendations.push("🔧 Consider checking if transaction is actually a swap or ATA creation only".to_string());
    }
    
    if recommendations.is_empty() {
        recommendations.push("✅ Verification seems to work - may be a timing or caching issue".to_string());
    }
    
    recommendations
}

fn display_debug_result(result: &DebugResult, verbose: bool) {
    println!("\n{}", "📋 DEBUG RESULTS".bright_green().bold());
    
    // Basic status
    println!("🏦 Global verification: {}", if result.global_verification_success { "✅ Success".bright_green() } else { "❌ Failed".bright_red() });
    println!("📊 Simple analysis: {}", if result.simple_analysis_success { "✅ Success".bright_green() } else { "❌ Failed".bright_red() });
    println!("🔗 RPC fetch: {}", if result.rpc_fetch_success { "✅ Success".bright_green() } else { "❌ Failed".bright_red() });
    println!("🔗 Wallet address: {}", result.wallet_address.bright_blue());
    
    if verbose {
        if let Some(error) = &result.global_verification_error {
            println!("🏦 Global verification error: {}", error.bright_red());
        }
        
        if let Some(error) = &result.simple_analysis_error {
            println!("📊 Simple analysis error: {}", error.bright_red());
        }
        
        if let Some(error) = &result.rpc_fetch_error {
            println!("🔗 RPC fetch error: {}", error.bright_red());
        }
    }
    
    // Recommendations
    println!("\n{}", "💡 RECOMMENDATIONS".bright_yellow().bold());
    for (i, rec) in result.recommendations.iter().enumerate() {
        println!("  {}. {}", i + 1, rec);
    }
    
    // Next steps
    println!("\n{}", "🔧 SUGGESTED FIXES".bright_cyan().bold());
    if !result.global_verification_success {
        println!("  1. Check wallet_transactions.rs verify_swap_transaction_global implementation");
        println!("  2. Verify RPC transaction fetching in get_transaction_details");
        println!("  3. Debug swap detection in analyze_transaction_for_verified_swap");
    }
    
    if !result.simple_analysis_success {
        println!("  4. Check transactions_tools.rs analyze_post_swap_transaction_simple");
        println!("  5. Verify instruction parsing logic for swap detection");
        println!("  6. Debug token balance change analysis");
    }
    
    if !result.rpc_fetch_success {
        println!("  7. Check RPC client configuration and transaction encoding");
        println!("  8. Verify transaction signature format and confirmation status");
    }
}