/// Comprehensive Transaction Detection Test Tool
/// 
/// This tool demonstrates the comprehensive transaction detection system

use clap::Parser;
use screenerbot::{
    logger::init_file_logging,
    transactions_detector::{analyze_transaction_comprehensive, TransactionType, TransactionDirection},
    utils::get_wallet_address,
    transactions_manager::initialize_transactions_manager,
    transactions_tools::analyze_post_swap_transaction_simple,
};
use std::fs;
use colored::*;

#[derive(Debug, Clone)]
struct TransactionTableRow {
    signature: String,
    tx_type: String,
    direction: Option<String>,
    sol_change: f64,
    fees_paid: f64,
    effective_price: f64,
    token_mint: Option<String>,
    router: Option<String>,
    success: bool,
}

#[derive(Parser, Debug)]
#[command(name = "test_comprehensive_transaction_detection")]
#[command(about = "Test comprehensive transaction detection capabilities")]
struct Args {
    /// Analyze a specific transaction signature
    #[arg(long)]
    signature: Option<String>,

    /// Test recent N transactions
    #[arg(long)]
    analyze_recent: Option<usize>,

    /// Show only swap transactions
    #[arg(long)]
    swaps_only: bool,

    /// Show enhanced swap analysis
    #[arg(long)]
    show_enhanced: bool,

    /// Show usage examples and help
    #[arg(long)]
    help_examples: bool,

    /// Filter to show only swap transactions
    #[arg(long)]
    swaps: bool,

    /// Filter to show only transfer transactions (SOL and token transfers)
    #[arg(long)]
    transfers: bool,

    /// Filter to show only buy transactions
    #[arg(long)]
    buy: bool,

    /// Filter to show only sell transactions
    #[arg(long)]
    sell: bool,

    /// Display results in table format
    #[arg(long)]
    table: bool,

    /// Analyze all cached transactions
    #[arg(long)]
    all: bool,
}

#[derive(Default)]
struct TransactionStats {
    total: usize,
    analysis_errors: usize,
    swaps: usize,
    buys: usize,
    sells: usize,
    enhanced_buys: usize,
    enhanced_sells: usize,
    sol_transfers: usize,
    token_transfers: usize,
    multihop_swaps: usize,
    defi_interactions: usize,
    bulk_transfers: usize,
    liquidity_provisions: usize,
    unknown: usize,
    total_fees: f64,
    total_sol_flow: f64,
    total_swap_sol: f64,
    total_swap_tokens: f64,
}

impl TransactionStats {
    fn new() -> Self {
        Self::default()
    }

    fn add_analysis(&mut self, analysis: &screenerbot::transactions_detector::TransactionAnalysis) {
        self.total += 1;
        self.total_fees += analysis.fees_paid;
        self.total_sol_flow += analysis.sol_change.abs();

        match analysis.transaction_type {
            TransactionType::Swap => {
                self.swaps += 1;
                self.total_swap_sol += analysis.sol_change.abs();
                
                if let Some(direction) = &analysis.direction {
                    match direction {
                        TransactionDirection::Buy => self.buys += 1,
                        TransactionDirection::Sell => self.sells += 1,
                    }
                }

                // Count token amounts
                for token_change in &analysis.token_changes {
                    self.total_swap_tokens += token_change.amount_change.abs();
                }
            },
            TransactionType::SolTransfer => self.sol_transfers += 1,
            TransactionType::TokenTransfer => self.token_transfers += 1,
            TransactionType::MultiHopSwap => self.multihop_swaps += 1,
            TransactionType::DeFiInteraction => self.defi_interactions += 1,
            TransactionType::BulkTransfer => self.bulk_transfers += 1,
            TransactionType::LiquidityProvision => self.liquidity_provisions += 1,
            TransactionType::Unknown => self.unknown += 1,
        }
    }

    fn add_enhanced_analysis(&mut self, direction: &str) {
        match direction.to_uppercase().as_str() {
            "BUY" => self.enhanced_buys += 1,
            "SELL" => self.enhanced_sells += 1,
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.help_examples {
        show_help_examples();
        return Ok(());
    }

    // Initialize logging
    init_file_logging();

    println!("{}", "🔍 COMPREHENSIVE TRANSACTION DETECTION TEST".bright_blue().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_blue());

    let wallet_address = get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;
    println!("🏦 Wallet: {}\n", wallet_address);

    if let Some(ref signature) = args.signature {
        // Test single transaction
        test_single_transaction(&signature, &wallet_address, &args).await?;
    } else if let Some(count) = args.analyze_recent {
        // Test recent transactions
        test_recent_transactions(&wallet_address, count, &args).await?;
    } else if args.all {
        // Test all cached transactions
        test_all_transactions(&wallet_address, &args).await?;
    } else {
        println!("Please specify:");
        println!("  --signature <SIG>     Analyze specific transaction");
        println!("  --analyze-recent <N>  Analyze N most recent transactions");
        println!("  --all                 Analyze all cached transactions");
        println!("Use --help-examples for usage examples");
    }

    Ok(())
}

async fn test_single_transaction(signature: &str, wallet_address: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing single transaction: {}\n", signature);
    
    // Initialize transaction manager
    println!("⚡ Initializing transaction manager...");
    initialize_transactions_manager().await?;
    println!("✅ Transaction manager ready\n");
    
    // Analyze the transaction using comprehensive analysis
    match analyze_transaction_comprehensive(signature, wallet_address).await {
        Ok(result) => {
            print_transaction_analysis(&result, signature);
            print_analysis_insights(&result);
            
            if args.show_enhanced {
                // Try enhanced analysis for swaps
                if matches!(result.transaction_type, TransactionType::Swap) {
                    match analyze_post_swap_transaction_simple(signature, wallet_address).await {
                        Ok(analysis) => {
                            println!("\n🔬 ENHANCED SWAP ANALYSIS");
                            println!("📊 Enhanced Results:");
                            println!("   • Direction: {}", if analysis.direction == "BUY" { "BUY" } else { "SELL" });
                            println!("   • Effective Price: {:.12} SOL/token", analysis.effective_price);
                            println!("   • SOL Amount: {:.9} SOL", analysis.sol_amount);
                            println!("   • Token Amount: {:.6} tokens", analysis.token_amount);
                            if let Some(token_mint) = &analysis.token_mint {
                                println!("   • Token Mint: {}...{}", &token_mint[..8], &token_mint[token_mint.len()-8..]);
                            }
                            println!("   • Transaction Fee: {:.9} SOL", analysis.fees_paid);
                            if let Some(router) = &analysis.router_name {
                                println!("   • Router: {}", router);
                            }
                        }
                        Err(e) => {
                            println!("\n🔬 ENHANCED SWAP ANALYSIS");
                            println!("⚠️ Enhanced analysis failed: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Analysis failed: {}", e);
        }
    }
    
    Ok(())
}

async fn test_recent_transactions(wallet_address: &str, count: usize, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing {} most recent transactions...", count);
    
    // Initialize transaction manager
    initialize_transactions_manager().await?;
    
    let transaction_files = get_cached_transaction_files()?;
    let recent_transactions: Vec<_> = transaction_files.into_iter().rev().take(count).collect();
    
    analyze_transactions_with_filters(wallet_address, recent_transactions, args).await
}

async fn test_all_transactions(wallet_address: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing all cached transactions...");
    
    // Initialize transaction manager
    initialize_transactions_manager().await?;
    
    let transaction_files = get_cached_transaction_files()?;
    
    analyze_transactions_with_filters(wallet_address, transaction_files, args).await
}

async fn analyze_transactions_with_filters(
    wallet_address: &str, 
    transactions: Vec<String>, 
    args: &Args
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📁 Analyzing {} transactions", transactions.len());
    println!();
    
    let mut stats = TransactionStats::new();
    let mut table_rows: Vec<TransactionTableRow> = Vec::new();
    
    for (index, signature) in transactions.iter().enumerate() {
        if !args.table {
            println!("🔍 Transaction {}/{}: {}", index + 1, transactions.len(), &signature[..16]);
        }
        
        match analyze_transaction_comprehensive(signature, wallet_address).await {
            Ok(analysis) => {
                stats.add_analysis(&analysis);
                
                // Apply filters
                if should_display_transaction_with_filters(&analysis, args) {
                    if args.table {
                        // Collect data for table display
                        let mut row = TransactionTableRow {
                            signature: format!("{}...{}", &signature[..8], &signature[signature.len()-8..]),
                            tx_type: format!("{:?}", analysis.transaction_type),
                            direction: analysis.direction.as_ref().map(|d| format!("{:?}", d)),
                            sol_change: analysis.sol_change,
                            fees_paid: analysis.fees_paid,
                            effective_price: analysis.effective_price,
                            token_mint: None,
                            router: analysis.router.clone(),
                            success: analysis.success,
                        };
                        
                        // Enhanced analysis for swaps to get more details
                        if matches!(analysis.transaction_type, TransactionType::Swap) {
                            if let Ok(swap_analysis) = analyze_post_swap_transaction_simple(signature, wallet_address).await {
                                row.direction = Some(swap_analysis.direction.to_uppercase());
                                row.effective_price = swap_analysis.effective_price;
                                row.token_mint = swap_analysis.token_mint.as_ref().map(|mint| format!("{}...{}", &mint[..8], &mint[mint.len()-8..]));
                                if let Some(router) = &swap_analysis.router_name {
                                    row.router = Some(router.clone());
                                }
                                stats.add_enhanced_analysis(&swap_analysis.direction);
                            }
                        }
                        
                        table_rows.push(row);
                    } else {
                        // Original detailed display
                        print_transaction_analysis(&analysis, signature);
                        print_analysis_insights(&analysis);
                        
                        // Enhanced swap analysis for detected swaps
                        if matches!(analysis.transaction_type, TransactionType::Swap) && args.show_enhanced {
                            println!("🔬 {}", "ENHANCED SWAP ANALYSIS".bright_blue().bold());
                            match analyze_post_swap_transaction_simple(signature, wallet_address).await {
                                Ok(swap_analysis) => {
                                    println!("📊 Enhanced Results:");
                                    println!("   • Direction: {}", swap_analysis.direction.to_uppercase());
                                    println!("   • Effective Price: {:.12} SOL/token", swap_analysis.effective_price);
                                    println!("   • SOL Amount: {:.9} SOL", swap_analysis.sol_amount);
                                    println!("   • Token Amount: {:.6} tokens", swap_analysis.token_amount);
                                    if let Some(token_mint) = &swap_analysis.token_mint {
                                        println!("   • Token Mint: {}...{}", &token_mint[..8], &token_mint[token_mint.len()-8..]);
                                    }
                                    println!("   • Transaction Fee: {:.9} SOL", swap_analysis.fees_paid);
                                    if let Some(router) = &swap_analysis.router_name {
                                        println!("   • Router: {}", router);
                                    }
                                    
                                    // Update stats with enhanced data
                                    stats.add_enhanced_analysis(&swap_analysis.direction);
                                }
                                Err(e) => {
                                    println!("⚠️ Enhanced analysis failed: {}", e);
                                }
                            }
                        }
                        
                        println!("{}", "────────────────────────────────────────────────────────────────────────────────".bright_black());
                    }
                }
            }
            Err(e) => {
                stats.analysis_errors += 1;
                if !args.table {
                    println!("❌ Analysis failed: {}", e);
                }
            }
        }
    }
    
    // Display results
    if args.table && !table_rows.is_empty() {
        print_transaction_table(&table_rows);
    }
    
    // Print summary statistics
    print_transaction_statistics(&stats);
    
    Ok(())
}

fn get_cached_transaction_files() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let transactions_dir = "data/transactions";
    let mut signatures = Vec::new();
    
    if let Ok(entries) = fs::read_dir(transactions_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".json") {
                        let signature = filename.trim_end_matches(".json");
                        signatures.push(signature.to_string());
                    }
                }
            }
        }
    }
    
    signatures.sort();
    Ok(signatures)
}

fn should_display_transaction_with_filters(analysis: &screenerbot::transactions_detector::TransactionAnalysis, args: &Args) -> bool {
    // If swaps_only is specified (legacy), filter by swaps
    if args.swaps_only && !matches!(analysis.transaction_type, TransactionType::Swap) {
        return false;
    }
    
    // Apply new filter arguments
    let mut should_display = true;
    
    // If any specific filters are provided, start with false and enable if conditions match
    if args.swaps || args.transfers || args.buy || args.sell {
        should_display = false;
        
        // Swaps filter
        if args.swaps && matches!(analysis.transaction_type, TransactionType::Swap) {
            should_display = true;
        }
        
        // Transfers filter (SOL and token transfers)
        if args.transfers && matches!(analysis.transaction_type, TransactionType::SolTransfer | TransactionType::TokenTransfer) {
            should_display = true;
        }
        
        // Buy filter
        if args.buy {
            if let Some(TransactionDirection::Buy) = analysis.direction {
                should_display = true;
            }
        }
        
        // Sell filter
        if args.sell {
            if let Some(TransactionDirection::Sell) = analysis.direction {
                should_display = true;
            }
        }
    }
    
    should_display
}



fn print_transaction_table(rows: &[TransactionTableRow]) {
    println!("{}", "📊 TRANSACTION TABLE".bright_green().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════".bright_green());
    
    // Print header
    println!("{:<18} {:<15} {:<10} {:<12} {:<10} {:<15} {:<18} {:<12} {:<8}",
        "Signature".bright_cyan().bold(),
        "Type".bright_cyan().bold(),
        "Direction".bright_cyan().bold(),
        "SOL Change".bright_cyan().bold(),
        "Fees".bright_cyan().bold(),
        "Price".bright_cyan().bold(),
        "Token Mint".bright_cyan().bold(),
        "Router".bright_cyan().bold(),
        "Status".bright_cyan().bold()
    );
    
    println!("{}", "─".repeat(150).bright_black());
    
    // Print rows
    for row in rows {
        let direction_str = row.direction.as_deref().unwrap_or("N/A");
        let direction_colored = match direction_str {
            "BUY" | "Buy" => direction_str.green(),
            "SELL" | "Sell" => direction_str.red(),
            _ => direction_str.normal(),
        };
        
        let sol_change_colored = if row.sol_change > 0.0 {
            format!("{:+.6}", row.sol_change).green()
        } else if row.sol_change < 0.0 {
            format!("{:.6}", row.sol_change).red()
        } else {
            format!("{:.6}", row.sol_change).normal()
        };
        
        let status_colored = if row.success {
            "✅".green()
        } else {
            "❌".red()
        };
        
        let price_str = if row.effective_price > 0.0 {
            format!("{:.6e}", row.effective_price)
        } else {
            "N/A".to_string()
        };
        
        println!("{:<18} {:<15} {:<10} {:<12} {:<10.6} {:<15} {:<18} {:<12} {:<8}",
            row.signature,
            row.tx_type,
            direction_colored,
            sol_change_colored,
            row.fees_paid,
            price_str,
            row.token_mint.as_deref().unwrap_or("N/A"),
            row.router.as_deref().unwrap_or("N/A"),
            status_colored
        );
    }
    
    println!("{}", "─".repeat(150).bright_black());
    println!("📝 Total transactions displayed: {}", rows.len());
    println!();
}

fn print_transaction_analysis(analysis: &screenerbot::transactions_detector::TransactionAnalysis, signature: &str) {
    println!("📋 TRANSACTION ANALYSIS");
    if !signature.is_empty() {
        println!("🔗 Signature: {}...", &signature[..signature.len().min(64)]);
    }
    
    // Display transaction type with emoji
    let type_display = match analysis.transaction_type {
        TransactionType::Swap => {
            if let Some(direction) = &analysis.direction {
                match direction {
                    TransactionDirection::Buy => "🔄 Type: Swap\n📈 Direction: Buy",
                    TransactionDirection::Sell => "🔄 Type: Swap\n📉 Direction: Sell",
                }
            } else {
                "🔄 Type: Swap"
            }
        },
        TransactionType::SolTransfer => "💰 Type: SolTransfer",
        TransactionType::TokenTransfer => "🪙 Type: TokenTransfer", 
        TransactionType::MultiHopSwap => "🔀 Type: MultiHopSwap",
        TransactionType::DeFiInteraction => "🏦 Type: DeFiInteraction",
        TransactionType::BulkTransfer => "📦 Type: BulkTransfer",
        TransactionType::LiquidityProvision => "🌊 Type: LiquidityProvision",
        TransactionType::Unknown => "❓ Type: Unknown",
    };
    println!("{}", type_display);
    
    // Add router info if available
    if let Some(router) = &analysis.router {
        println!("🔄 Router: {}", router);
    }
    
    println!("💰 SOL Change: {:.9} SOL", analysis.sol_change);
    println!("💵 Fees Paid: {:.9} SOL", analysis.fees_paid);
    
    // Show effective price for swaps
    if matches!(analysis.transaction_type, TransactionType::Swap) && analysis.effective_price > 0.0 {
        println!("📈 Effective Price: {:.12} SOL/token", analysis.effective_price);
    }
    
    // Show token changes
    if !analysis.token_changes.is_empty() {
        println!("🪙 Token Changes:");
        for change in &analysis.token_changes {
            let sign = if change.amount_change > 0.0 { "+" } else { "" };
            println!("   {}{:.6} tokens ({}...{})", 
                sign, change.amount_change, 
                &change.mint[..8], &change.mint[change.mint.len()-8..]);
        }
    }
    
    let status = if analysis.success { "✅ Status: Success" } else { "❌ Status: Failed" };
    println!("{}", status);
}

fn print_analysis_insights(analysis: &screenerbot::transactions_detector::TransactionAnalysis) {
    println!();
    println!("{}", "💡 INSIGHTS".bright_yellow().bold());
    
    match analysis.transaction_type {
        TransactionType::Swap => {
            if let Some(direction) = &analysis.direction {
                match direction {
                    TransactionDirection::Buy => {
                        println!("📊 This was a token purchase using SOL");
                        if analysis.effective_price > 0.0 {
                            println!("💰 You paid {:.12} SOL per token", analysis.effective_price);
                        }
                    },
                    TransactionDirection::Sell => {
                        println!("📊 This was a token sale for SOL");
                        if analysis.effective_price > 0.0 {
                            println!("💰 You received {:.12} SOL per token", analysis.effective_price);
                        }
                    }
                }
            }
        },
        TransactionType::SolTransfer => {
            println!("📊 Simple SOL transfer between accounts");
        },
        TransactionType::BulkTransfer => {
            println!("📊 Bulk transfer operation - multiple small transfers in one transaction");
        },
        TransactionType::TokenTransfer => {
            println!("📊 Token transfer between accounts");
        },
        _ => {
            println!("📊 Transaction type: {:?}", analysis.transaction_type);
        }
    }
    
    if analysis.fees_paid < 0.00001 {
        println!("✅ Low fees: {:.6} SOL", analysis.fees_paid);
    } else if analysis.fees_paid > 0.001 {
        println!("⚠️ High fees: {:.6} SOL", analysis.fees_paid);
    } else {
        println!("💵 Fees: {:.6} SOL", analysis.fees_paid);
    }
    
    println!();
}

fn print_transaction_statistics(stats: &TransactionStats) {
    println!();
    println!("{}", "📊 TRANSACTION STATISTICS".bright_green().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_green());
    println!("📈 Total Transactions: {}", stats.total);
    println!("❌ Analysis Errors: {}", stats.analysis_errors);
    println!();
    
    println!("{}", "Transaction Types:".bright_yellow());
    println!("  🔄 Swaps: {} (Basic Buys: {}, Basic Sells: {})", 
        stats.swaps, stats.buys.to_string().green(), stats.sells.to_string().red());
    if stats.enhanced_buys > 0 || stats.enhanced_sells > 0 {
        println!("     📊 Enhanced Analysis: {} Buys, {} Sells", 
            stats.enhanced_buys.to_string().green(), stats.enhanced_sells.to_string().red());
    }
    println!("  💰 SOL Transfers: {}", stats.sol_transfers);
    println!("  🪙 Token Transfers: {}", stats.token_transfers);
    println!("  🔀 Multi-hop Swaps: {}", stats.multihop_swaps);
    println!("  🏦 DeFi Interactions: {}", stats.defi_interactions);
    println!("  📦 Bulk Transfers: {}", stats.bulk_transfers);
    println!("  🌊 Liquidity Provisions: {}", stats.liquidity_provisions);
    println!("  ❓ Unknown: {}", stats.unknown);
    println!();
    
    println!("{}", "Financial Summary:".bright_yellow());
    println!("  💵 Total Fees Paid: {:.6} SOL", stats.total_fees);
    println!("  🌊 Total SOL Flow: {:.6} SOL", stats.total_sol_flow);
    if stats.total_swap_sol > 0.0 {
        println!("  💰 Total Swap SOL: {:.6} SOL", stats.total_swap_sol);
    }
    if stats.total_swap_tokens > 0.0 {
        println!("  🪙 Total Swap Tokens: {:.6} tokens", stats.total_swap_tokens);
    }
    if stats.total > 0 {
        println!("  📊 Average Fee per Transaction: {:.6} SOL", stats.total_fees / stats.total as f64);
    }
    if stats.swaps > 0 && stats.total_swap_sol > 0.0 {
        println!("  📈 Average Swap Size: {:.6} SOL", stats.total_swap_sol / stats.swaps as f64);
    }
    
    println!();
    println!("{}", "🎉 Analysis Complete!".bright_green().bold());
}

fn show_help_examples() {
    println!("{}", "🔍 COMPREHENSIVE TRANSACTION DETECTION TOOL".bright_blue().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_blue());
    println!();
    println!("{}", "📚 USAGE EXAMPLES:".bright_yellow().bold());
    println!();
    println!("🔍 Analyze a specific transaction:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --signature 5RAM6wNqTwMmWNy7Vz1pdAdNWyisD5xBpcBsnV2td2JZsCB6dp7ivqf3eXuno7DyD9RMt5AH1cnoYZ3JoSwqTTL5 \\");
    println!("     --show-enhanced");
    println!();
    println!("📊 Analyze recent transactions:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --analyze-recent 10 --show-enhanced");
    println!();
    println!("🔄 Show only swap transactions in table format:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --analyze-recent 20 --swaps --table");
    println!();
    println!("📈 Show only buy transactions:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --all --buy --table");
    println!();
    println!("� Show only sell transactions:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --analyze-recent 50 --sell --table");
    println!();
    println!("💰 Show transfer transactions:");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --analyze-recent 30 --transfers --table");
    println!();
    println!("🔀 Combine filters (swaps AND buys):");
    println!("   cargo run --bin main_debug_transactions -- \\");
    println!("     --all --swaps --buy --table");
    println!();
    println!("{}", "🛠️ AVAILABLE OPTIONS:".bright_yellow().bold());
    println!("  --signature <SIG>     Analyze specific transaction");
    println!("  --analyze-recent <N>  Analyze N most recent transactions");
    println!("  --all                 Analyze all cached transactions");
    println!();
    println!("{}", "🔍 FILTERS:".bright_yellow().bold());
    println!("  --swaps               Show only swap transactions");
    println!("  --transfers           Show only transfer transactions (SOL + token)");
    println!("  --buy                 Show only buy transactions");
    println!("  --sell                Show only sell transactions");
    println!();
    println!("{}", "📊 DISPLAY OPTIONS:".bright_yellow().bold());
    println!("  --table               Display results in table format");
    println!("  --show-enhanced       Show enhanced swap analysis (non-table mode)");
    println!();
    println!("{}", "🏛️ LEGACY OPTIONS:".bright_yellow().bold());
    println!("  --swaps-only          Legacy: filter to show only swap transactions");
    println!("  --help-examples       Show this help with examples");
    println!();
    println!("{}", "💡 TIPS:".bright_green().bold());
    println!("• Use --table for compact overview of many transactions");
    println!("• Use --show-enhanced for detailed swap analysis (without --table)");
    println!("• Combine filters to narrow down results (e.g., --swaps --buy)");
    println!("• Use --all to analyze your entire transaction history");
}
