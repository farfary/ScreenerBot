/// Comprehensive Transaction Detection Test Tool
/// 
/// This tool demonstrates the new comprehensive transaction detection system
/// that can identify and analyze various transaction types including:
/// - Token swaps (BUY/SELL)
/// - SOL transfers
/// - Token transfers  
/// - Multi-hop swaps
/// - DeFi interactions
///
/// Usage:
///   cargo run --bin test_comprehensive_transaction_detection -- --signature <SIGNATURE>
///   cargo run --bin test_comprehensive_transaction_detection -- --test-all-cached
///   cargo run --bin test_comprehensive_transaction_detection -- --analyze-recent 10

use clap::Parser;
use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    transaction_detector::{analyze_transaction_comprehensive, format_transaction_analysis, TransactionType, TransactionDirection},
    utils::get_wallet_address,
    wallet_transactions::initialize_wallet_transaction_manager,
};
use std::fs;
use colored::Colorize;

#[derive(Parser)]
#[command(about = "Test comprehensive transaction detection system")]
pub struct Args {
    /// Specific transaction signature to analyze
    #[arg(short, long)]
    pub signature: Option<String>,
    
    /// Test all cached transactions
    #[arg(short, long)]
    pub test_all_cached: bool,
    
    /// Analyze the N most recent transactions
    #[arg(short, long)]
    pub analyze_recent: Option<usize>,
    
    /// Enable verbose debug output
    #[arg(short, long)]
    pub verbose: bool,
    
    /// Filter by transaction type (swap, transfer, defi, etc.)
    #[arg(short, long)]
    pub filter_type: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    init_file_logging();
    
    println!("{}", "🔍 COMPREHENSIVE TRANSACTION DETECTION TEST".bright_blue().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_blue());
    
    // Get wallet address
    let wallet_address = get_wallet_address()?;
    println!("🏦 Wallet: {}", wallet_address.bright_yellow());
    println!();
    
    // Initialize wallet transaction manager for cached transactions
    if args.test_all_cached || args.analyze_recent.is_some() {
        println!("⚡ Initializing wallet transaction manager...");
        initialize_wallet_transaction_manager().await?;
        println!("✅ Wallet transaction manager ready");
        println!();
    }
    
    if let Some(signature) = args.signature {
        // Test single transaction
        test_single_transaction(&signature, &wallet_address).await?;
    } else if args.test_all_cached {
        // Test all cached transactions
        test_all_cached_transactions(&wallet_address, args.filter_type.as_deref()).await?;
    } else if let Some(count) = args.analyze_recent {
        // Test recent transactions
        test_recent_transactions(&wallet_address, count, args.filter_type.as_deref()).await?;
    } else {
        println!("❌ Please specify a signature (--signature), test all cached (--test-all-cached), or analyze recent (--analyze-recent N)");
        return Ok(());
    }
    
    Ok(())
}

async fn test_single_transaction(signature: &str, wallet_address: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing single transaction: {}", signature.bright_cyan());
    println!();
    
    match analyze_transaction_comprehensive(signature, wallet_address).await {
        Ok(analysis) => {
            print_transaction_analysis(&analysis, signature);
            
            // Additional insights
            print_analysis_insights(&analysis);
        }
        Err(e) => {
            println!("❌ {}: {}", "Analysis failed".bright_red(), e);
        }
    }
    
    Ok(())
}

async fn test_all_cached_transactions(wallet_address: &str, filter_type: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing all cached transactions...");
    
    // Get all transaction files
    let transaction_files = get_cached_transaction_files()?;
    println!("📁 Found {} cached transactions", transaction_files.len());
    println!();
    
    let mut stats = TransactionStats::new();
    
    for (index, signature) in transaction_files.iter().enumerate() {
        if index % 10 == 0 {
            println!("📊 Processing transaction {}/{}", index + 1, transaction_files.len());
        }
        
        match analyze_transaction_comprehensive(signature, wallet_address).await {
            Ok(analysis) => {
                stats.add_analysis(&analysis);
                
                // Apply filter if specified
                if let Some(filter) = filter_type {
                    if should_display_transaction(&analysis, filter) {
                        println!("🔍 Transaction {}: {}", index + 1, &signature[..16]);
                        print_transaction_analysis(&analysis, signature);
                        println!("{}", "─".repeat(80).bright_black());
                    }
                } else {
                    // Just count, don't display all
                }
            }
            Err(e) => {
                stats.errors += 1;
                log(LogTag::Transactions, "ERROR", &format!("Failed to analyze {}: {}", &signature[..8], e));
            }
        }
    }
    
    // Print summary statistics
    print_transaction_statistics(&stats);
    
    Ok(())
}

async fn test_recent_transactions(wallet_address: &str, count: usize, filter_type: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing {} most recent transactions...", count);
    
    let transaction_files = get_cached_transaction_files()?;
    let recent_transactions: Vec<_> = transaction_files.into_iter().rev().take(count).collect();
    
    println!("📁 Analyzing {} recent transactions", recent_transactions.len());
    println!();
    
    let mut stats = TransactionStats::new();
    
    for (index, signature) in recent_transactions.iter().enumerate() {
        println!("🔍 Transaction {}/{}: {}", index + 1, recent_transactions.len(), &signature[..16]);
        
        match analyze_transaction_comprehensive(signature, wallet_address).await {
            Ok(analysis) => {
                stats.add_analysis(&analysis);
                
                if filter_type.is_none() || should_display_transaction(&analysis, filter_type.unwrap()) {
                    print_transaction_analysis(&analysis, signature);
                    print_analysis_insights(&analysis);
                    println!("{}", "─".repeat(80).bright_black());
                }
            }
            Err(e) => {
                stats.errors += 1;
                println!("❌ {}: {}", "Analysis failed".bright_red(), e);
                println!("{}", "─".repeat(80).bright_black());
            }
        }
    }
    
    print_transaction_statistics(&stats);
    
    Ok(())
}

fn print_transaction_analysis(analysis: &screenerbot::transaction_detector::TransactionAnalysis, signature: &str) {
    println!("📋 {}", "TRANSACTION ANALYSIS".bright_green().bold());
    println!("🔗 Signature: {}", signature.bright_cyan());
    
    // Transaction type with color coding
    let type_color = match analysis.transaction_type {
        TransactionType::Swap => "🔄".to_string(),
        TransactionType::SolTransfer => "💰".to_string(),
        TransactionType::TokenTransfer => "🪙".to_string(),
        TransactionType::MultiHopSwap => "🔀".to_string(),
        TransactionType::DeFiInteraction => "🏦".to_string(),
        TransactionType::LiquidityProvision => "💧".to_string(),
        TransactionType::Unknown => "❓".to_string(),
    };
    println!("{} Type: {:?}", type_color, analysis.transaction_type);
    
    // Direction with color coding
    if let Some(direction) = &analysis.direction {
        let direction_color = match direction {
            TransactionDirection::Buy => "📈".green(),
            TransactionDirection::Sell => "📉".red(),
        };
        println!("{} Direction: {:?}", direction_color, direction);
    }
    
    // Router
    if let Some(router) = &analysis.router {
        println!("🔄 Router: {}", router.bright_blue());
    }
    
    // Financial details
    println!("💰 SOL Change: {:.9} SOL", analysis.sol_change);
    println!("💵 Fees Paid: {:.9} SOL", analysis.fees_paid);
    
    if analysis.effective_price > 0.0 {
        println!("📈 Effective Price: {:.12} SOL/token", analysis.effective_price);
    }
    
    // Token changes
    if !analysis.token_changes.is_empty() {
        println!("🪙 Token Changes:");
        for token in &analysis.token_changes {
            let change_sign = if token.amount_change >= 0.0 { "+" } else { "" };
            let change_color = if token.amount_change >= 0.0 { 
                format!("{}{:.6}", change_sign, token.amount_change).green()
            } else {
                format!("{:.6}", token.amount_change).red()
            };
            println!("   {} {} ({}...)", change_color, "tokens", &token.mint[..8]);
        }
    }
    
    // Success/Error status
    if analysis.success {
        println!("✅ Status: {}", "Success".bright_green());
    } else {
        println!("❌ Status: {}", "Failed".bright_red());
        if let Some(error) = &analysis.error_message {
            println!("💬 Error: {}", error);
        }
    }
    
    println!();
}

fn print_analysis_insights(analysis: &screenerbot::transaction_detector::TransactionAnalysis) {
    println!("💡 {}", "INSIGHTS".bright_yellow().bold());
    
    match analysis.transaction_type {
        TransactionType::Swap => {
            if let Some(direction) = &analysis.direction {
                match direction {
                    TransactionDirection::Buy => {
                        println!("📊 This was a token purchase using SOL");
                        if analysis.effective_price > 0.0 {
                            println!("💰 You paid {:.12} SOL per token", analysis.effective_price);
                        }
                    }
                    TransactionDirection::Sell => {
                        println!("📊 This was a token sale for SOL");
                        if analysis.effective_price > 0.0 {
                            println!("💰 You received {:.12} SOL per token", analysis.effective_price);
                        }
                    }
                }
            }
        }
        TransactionType::SolTransfer => {
            println!("📊 Simple SOL transfer between accounts");
        }
        TransactionType::TokenTransfer => {
            println!("📊 Token transfer without SOL exchange");
        }
        TransactionType::MultiHopSwap => {
            println!("📊 Complex multi-token swap transaction");
        }
        TransactionType::DeFiInteraction => {
            println!("📊 DeFi protocol interaction detected");
        }
        TransactionType::LiquidityProvision => {
            println!("📊 Liquidity provision or removal");
        }
        TransactionType::Unknown => {
            println!("📊 Unknown transaction type - may need enhanced detection");
        }
    }
    
    // Fee analysis
    if analysis.fees_paid > 0.01 {
        println!("⚠️ High transaction fees: {:.6} SOL", analysis.fees_paid);
    } else if analysis.fees_paid > 0.001 {
        println!("💵 Moderate fees: {:.6} SOL", analysis.fees_paid);
    } else {
        println!("✅ Low fees: {:.6} SOL", analysis.fees_paid);
    }
    
    println!();
}

fn get_cached_transaction_files() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let transaction_dir = "data/transactions";
    
    if !std::path::Path::new(transaction_dir).exists() {
        return Err("Transaction directory not found".into());
    }
    
    let mut signatures = Vec::new();
    
    for entry in fs::read_dir(transaction_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        
        if file_name_str.ends_with(".json") {
            let signature = file_name_str.trim_end_matches(".json").to_string();
            signatures.push(signature);
        }
    }
    
    Ok(signatures)
}

fn should_display_transaction(analysis: &screenerbot::transaction_detector::TransactionAnalysis, filter: &str) -> bool {
    match filter.to_lowercase().as_str() {
        "swap" => matches!(analysis.transaction_type, TransactionType::Swap),
        "transfer" => matches!(analysis.transaction_type, TransactionType::SolTransfer | TransactionType::TokenTransfer),
        "defi" => matches!(analysis.transaction_type, TransactionType::DeFiInteraction),
        "multihop" => matches!(analysis.transaction_type, TransactionType::MultiHopSwap),
        "buy" => matches!(analysis.direction, Some(TransactionDirection::Buy)),
        "sell" => matches!(analysis.direction, Some(TransactionDirection::Sell)),
        "unknown" => matches!(analysis.transaction_type, TransactionType::Unknown),
        _ => true,
    }
}

#[derive(Default)]
struct TransactionStats {
    total: usize,
    swaps: usize,
    buys: usize,
    sells: usize,
    sol_transfers: usize,
    token_transfers: usize,
    multihop_swaps: usize,
    defi_interactions: usize,
    unknown: usize,
    errors: usize,
    total_fees: f64,
    total_sol_flow: f64,
}

impl TransactionStats {
    fn new() -> Self {
        Default::default()
    }
    
    fn add_analysis(&mut self, analysis: &screenerbot::transaction_detector::TransactionAnalysis) {
        self.total += 1;
        self.total_fees += analysis.fees_paid;
        self.total_sol_flow += analysis.sol_change.abs();
        
        match analysis.transaction_type {
            TransactionType::Swap => {
                self.swaps += 1;
                if let Some(direction) = &analysis.direction {
                    match direction {
                        TransactionDirection::Buy => self.buys += 1,
                        TransactionDirection::Sell => self.sells += 1,
                    }
                }
            }
            TransactionType::SolTransfer => self.sol_transfers += 1,
            TransactionType::TokenTransfer => self.token_transfers += 1,
            TransactionType::MultiHopSwap => self.multihop_swaps += 1,
            TransactionType::DeFiInteraction => self.defi_interactions += 1,
            TransactionType::Unknown => self.unknown += 1,
            _ => {}
        }
    }
}

fn print_transaction_statistics(stats: &TransactionStats) {
    println!();
    println!("{}", "📊 TRANSACTION STATISTICS".bright_blue().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_blue());
    
    println!("📈 Total Transactions: {}", stats.total.to_string().bright_green());
    println!("❌ Analysis Errors: {}", stats.errors.to_string().bright_red());
    println!();
    
    println!("{}", "Transaction Types:".bright_yellow());
    println!("  🔄 Swaps: {} (Buys: {}, Sells: {})", stats.swaps, stats.buys.to_string().green(), stats.sells.to_string().red());
    println!("  💰 SOL Transfers: {}", stats.sol_transfers);
    println!("  🪙 Token Transfers: {}", stats.token_transfers);
    println!("  🔀 Multi-hop Swaps: {}", stats.multihop_swaps);
    println!("  🏦 DeFi Interactions: {}", stats.defi_interactions);
    println!("  ❓ Unknown: {}", stats.unknown);
    println!();
    
    println!("{}", "Financial Summary:".bright_yellow());
    println!("  💵 Total Fees Paid: {:.6} SOL", stats.total_fees);
    println!("  🌊 Total SOL Flow: {:.6} SOL", stats.total_sol_flow);
    if stats.total > 0 {
        println!("  📊 Average Fee per Transaction: {:.6} SOL", stats.total_fees / stats.total as f64);
    }
    
    println!();
    println!("{}", "🎉 Analysis Complete!".bright_green().bold());
}
