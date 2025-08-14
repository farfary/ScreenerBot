
/// Transaction Manager & Analyzer Debug Tool
///
/// Comprehensive debugging and testing tool for the transactions management system.
/// This tool provides detailed analysis, monitoring, and debugging capabilities
/// for transaction processing, caching, and analysis.
///
/// Features:
/// - Monitor wallet transactions in real-time
/// - Fetch new transactions from wallet (not in cache)
/// - Fetch all cached transactions (fast operation)
/// - Fetch all transactions from blockchain (comprehensive)
/// - Analyze specific transactions by signature
/// - Test transaction type detection
/// - Debug transaction caching system
/// - Validate transaction analysis
/// - Performance benchmarking
/// - Cache management and stats
/// - Execute real swap tests with transaction analysis
/// - Combinable analysis flag for comprehensive insights
///
/// Usage Examples:
/// - Monitor wallet transactions: cargo run --bin main_transactions_debug -- --monitor
/// - Fetch new transactions: cargo run --bin main_transactions_debug -- --fetch-new --count 50
/// - Fetch all cached: cargo run --bin main_transactions_debug -- --fetch-all --count 100
/// - Fetch from blockchain: cargo run --bin main_transactions_debug -- --fetch-all-blockchain --count 1000
/// - Analyze specific transaction: cargo run --bin main_transactions_debug -- --signature <SIG>
/// - Force recalculate transaction: cargo run --bin main_transactions_debug -- --signature <SIG> --force-recalculate
/// - Test analyzer on recent transactions: cargo run --bin main_transactions_debug -- --test-analyzer --count 10
/// - Debug cache system: cargo run --bin main_transactions_debug -- --debug-cache
/// - Recalculate analysis: cargo run --bin main_transactions_debug -- --recalculate-cache
/// - Update and re-analyze cache: cargo run --bin main_transactions_debug -- --update-cache --count 50 (preserves raw data)
/// - Analyze all swaps with PnL: cargo run --bin main_transactions_debug -- --analyze-swaps
/// - Analyze position lifecycle: cargo run --bin main_transactions_debug -- --analyze-positions
/// - Analyze ALL transaction types: cargo run --bin main_transactions_debug -- --analyze-all --count 500
/// - Performance test: cargo run --bin main_transactions_debug -- --benchmark --count 100
/// - Fetch and analyze: cargo run --bin main_transactions_debug -- --fetch-new --analyze --count 50
/// - Monitor and analyze: cargo run --bin main_transactions_debug -- --monitor --analyze --duration 300
/// - Just analyze: cargo run --bin main_transactions_debug -- --analyze
/// - Test real swaps: cargo run --bin main_transactions_debug -- --test-swap --swap-type round-trip --token-mint <MINT> --sol-amount 0.002
/// - Test real position management: cargo run --bin main_transactions_debug -- --test-position --token-mint <MINT> --sol-amount 0.002

use screenerbot::transactions::{
    TransactionsManager, Transaction, TransactionType, TransactionDirection,
    get_transaction, initialize_global_transaction_manager, wait_for_transaction_verification
};
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::global::{
    set_cmd_args, get_transactions_cache_dir
};
use screenerbot::arguments::{get_cmd_args, set_cmd_args as args_set_cmd_args};
use screenerbot::rpc::{get_rpc_client, SwapError, sol_to_lamports};
use screenerbot::utils::get_wallet_address;
use screenerbot::tokens::types::PriceSourceType;
use screenerbot::tokens::{Token, get_token_decimals_sync};
use screenerbot::swaps::{
    get_jupiter_quote, execute_jupiter_swap, get_gmgn_quote,
    JupiterSwapResult, buy_token, sell_token, wait_for_swap_verification
};
use screenerbot::positions;

use spl_associated_token_account::get_associated_token_address;
use clap::{Arg, Command};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use chrono::{DateTime, Utc};
use solana_sdk::pubkey::Pubkey;
use serde_json;

#[tokio::main]
async fn main() {
    // Initialize logger first
    init_file_logging();

        let matches = Command::new("Transaction Manager & Analyzer Debug Tool")
        .version("1.0")
        .about("Comprehensive debugging tool for transactions management system")
        .arg(
            Arg::new("monitor")
                .long("monitor")
                .help("Monitor wallet transactions in real-time")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("signature")
                .long("signature")
                .help("Analyze specific transaction by signature")
                .value_name("SIGNATURE")
        )
        .arg(
            Arg::new("force-recalculate")
                .long("force-recalculate")
                .help("Force recalculation of analysis even if cached data exists")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-analyzer")
                .long("test-analyzer")
                .help("Test transaction analyzer on recent transactions")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-cache")
                .long("debug-cache")
                .help("Debug the transaction cache system")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("recalculate-cache")
                .long("recalculate-cache")
                .help("Recalculate all analysis parameters without deleting raw transaction data")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Run performance benchmark tests")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-swaps")
                .long("analyze-swaps")
                .help("Analyze all swap transactions with comprehensive PnL")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-positions")
                .long("analyze-positions")
                .help("Analyze position lifecycle with entry/exit tracking and PnL")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-all")
                .long("analyze-all")
                .help("Analyze ALL transaction types (not just swaps) with comprehensive breakdown")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("update-cache")
                .long("update-cache")
                .help("Re-analyze and update all cached transactions with new analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch-new")
                .long("fetch-new")
                .help("Fetch new transactions from the wallet (not in cache)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch-all")
                .long("fetch-all")
                .help("Fetch all transactions from cache or blockchain")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch-all-blockchain")
                .long("fetch-all-blockchain")
                .help("Fetch and analyze ALL wallet transactions from blockchain (comprehensive)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze")
                .long("analyze")
                .help("Perform comprehensive analysis (can be combined with other commands)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-swap")
                .long("test-swap")
                .help("Execute real swap test with transaction analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-position")
                .long("test-position")
                .help("Test real position management with transaction verification")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("swap-type")
                .long("swap-type")
                .help("Swap type: 'sol-to-token', 'token-to-sol', or 'round-trip' (default: round-trip)")
                .value_name("TYPE")
                .default_value("round-trip")
        )
        .arg(
            Arg::new("token-mint")
                .long("token-mint")
                .help("Token mint address for swap test")
                .value_name("MINT")
                .default_value("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263") // BONK
        )
        .arg(
            Arg::new("token-symbol")
                .long("token-symbol")
                .help("Token symbol for display purposes")
                .value_name("SYMBOL")
                .default_value("BONK")
        )
        .arg(
            Arg::new("sol-amount")
                .long("sol-amount")
                .help("SOL amount to trade (default: 0.002 SOL)")
                .value_name("AMOUNT")
                .default_value("0.002")
        )
        .arg(
            Arg::new("slippage")
                .long("slippage")
                .help("Slippage tolerance percentage (default: 15%)")
                .value_name("PERCENT")
                .default_value("15.0")
        )
        .arg(
            Arg::new("router")
                .long("router")
                .help("Swap router: 'jupiter' or 'gmgn' (default: jupiter)")
                .value_name("ROUTER")
                .default_value("jupiter")
        )
        .arg(
            Arg::new("count")
                .long("count")
                .help("Number of transactions to process")
                .value_name("COUNT")
                .default_value("10")
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .help("Duration in seconds for monitoring")
                .value_name("SECONDS")
                .default_value("60")
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Enable verbose debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Set command args for debug flags
    let mut args = vec!["main_transactions_debug".to_string()];
    if matches.get_flag("verbose") || matches.get_one::<String>("signature").is_some() {
        args.push("--debug-transactions".to_string());
    }
    set_cmd_args(args);

    log(LogTag::System, "INFO", "Starting Transaction Manager & Analyzer Debug Tool");

    // Initialize RPC client (it's automatically initialized when first used)
    let _rpc_client = get_rpc_client();

    // Load wallet configuration
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to load wallet: {}", e));
            std::process::exit(1);
        }
    };

    log(LogTag::System, "INFO", &format!("Loaded wallet: {}", wallet_pubkey));

    // Check for combinable analyze flag
    let should_analyze = matches.get_flag("analyze");
    
    // Execute based on command line arguments
    if matches.get_flag("monitor") {
        let duration: u64 = matches.get_one::<String>("duration")
            .unwrap()
            .parse()
            .unwrap_or(60);
        monitor_transactions(wallet_pubkey, duration).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after monitoring...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if let Some(signature) = matches.get_one::<String>("signature") {
        let force_recalculate = matches.get_flag("force-recalculate");
        analyze_specific_transaction(signature, force_recalculate).await;
    } else if matches.get_flag("fetch-new") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        fetch_new_transactions(wallet_pubkey, count).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching new transactions...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("fetch-all") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(1000);
        fetch_all_cached_transactions(wallet_pubkey, count).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching all transactions...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("fetch-all-blockchain") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(1000);
        fetch_all_wallet_transactions(wallet_pubkey, count).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching from blockchain...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("test-analyzer") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(10);
        test_transaction_analyzer(wallet_pubkey, count).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after testing analyzer...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("debug-cache") {
        debug_cache_system().await;
    } else if matches.get_flag("recalculate-cache") {
        recalculate_transaction_cache().await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after recalculating cache...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("benchmark") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        run_benchmark_tests(wallet_pubkey, count).await;
    } else if matches.get_flag("analyze-swaps") {
        analyze_all_swaps(wallet_pubkey).await;
    } else if matches.get_flag("analyze-positions") {
        analyze_all_positions(wallet_pubkey).await;
    } else if matches.get_flag("analyze-all") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(1000);
        analyze_all_transactions(wallet_pubkey, count).await;
    } else if matches.get_flag("test-swap") {
        let swap_type = matches.get_one::<String>("swap-type").unwrap();
        let token_mint = matches.get_one::<String>("token-mint").unwrap();
        let token_symbol = matches.get_one::<String>("token-symbol").unwrap();
        let sol_amount: f64 = matches.get_one::<String>("sol-amount")
            .unwrap()
            .parse()
            .unwrap_or(0.002);
        let slippage: f64 = matches.get_one::<String>("slippage")
            .unwrap()
            .parse()
            .unwrap_or(15.0);
        let router = matches.get_one::<String>("router").unwrap();
        
        test_real_swap(
            wallet_pubkey,
            swap_type,
            token_mint,
            token_symbol,
            sol_amount,
            slippage,
            router
        ).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after swap test...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if matches.get_flag("test-position") {
        let token_mint = matches.get_one::<String>("token-mint").unwrap();
        let token_symbol = matches.get_one::<String>("token-symbol").unwrap();
        let sol_amount: f64 = matches.get_one::<String>("sol-amount")
            .unwrap()
            .parse()
            .unwrap_or(0.002);
        
        test_real_position_management(
            wallet_pubkey,
            token_mint,
            token_symbol,
            sol_amount
        ).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after position test...");
            analyze_all_positions(wallet_pubkey).await;
        }
    } else if matches.get_flag("update-cache") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        update_transaction_cache(wallet_pubkey, count).await;
        
        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after updating cache...");
            analyze_all_swaps(wallet_pubkey).await;
        }
    } else if should_analyze {
        // If only --analyze is specified, run comprehensive analysis
        log(LogTag::System, "INFO", "Running comprehensive transaction analysis...");
        analyze_all_swaps(wallet_pubkey).await;
    } else {
        log(LogTag::System, "ERROR", "No command specified. Use --help for usage information.");
        std::process::exit(1);
    }

    log(LogTag::System, "INFO", "Transaction Manager & Analyzer Debug Tool completed");
}

/// Analyze all swap transactions with comprehensive PnL
async fn analyze_all_swaps(wallet_pubkey: Pubkey) {
    log(LogTag::Transactions, "INFO", "Starting comprehensive swap analysis for all transactions");

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Get all swap transactions
    match manager.get_all_swap_transactions().await {
        Ok(swaps) => {
            log(LogTag::Transactions, "SUCCESS", &format!("Found {} swap transactions", swaps.len()));
            
            // Display comprehensive analysis table
            manager.display_swap_analysis_table(&swaps);
            
            // Additional statistics
            display_detailed_swap_statistics(&swaps);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to analyze swaps: {}", e));
        }
    }
}

/// Display detailed swap statistics
fn display_detailed_swap_statistics(swaps: &[screenerbot::transactions::SwapPnLInfo]) {
    if swaps.is_empty() {
        return;
    }

    log(LogTag::Transactions, "STATS", "=== DETAILED SWAP STATISTICS ===");
    
    let mut token_stats: std::collections::HashMap<String, TokenSwapStats> = std::collections::HashMap::new();
    let mut router_stats: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    
    let mut total_profit_loss = 0.0;
    let mut profitable_swaps = 0;
    let mut loss_swaps = 0;
    
    for swap in swaps {
        // Token statistics
        let token_stat = token_stats.entry(swap.token_symbol.clone()).or_insert(TokenSwapStats::new());
        if swap.swap_type == "Buy" {
            token_stat.buy_count += 1;
            token_stat.total_sol_spent += swap.sol_amount;
        } else {
            token_stat.sell_count += 1;
            token_stat.total_sol_received += swap.sol_amount;
        }
        token_stat.total_fees += swap.fee_sol;
        
        // Router statistics
        *router_stats.entry(swap.router.clone()).or_insert(0) += 1;
        
        // Simplified PnL calculation (buy vs sell difference)
        if swap.swap_type == "Sell" {
            profitable_swaps += 1;
            total_profit_loss += swap.sol_amount;
        } else {
            loss_swaps += 1;
            total_profit_loss -= swap.sol_amount;
        }
    }
    
    // Display token statistics
    log(LogTag::Transactions, "STATS", "Token Trading Summary:");
    for (token, stats) in &token_stats {
        let net_sol = stats.total_sol_received - stats.total_sol_spent - stats.total_fees;
        log(LogTag::Transactions, "STATS", &format!(
            "  {}: {} buys ({:.3} SOL), {} sells ({:.3} SOL), fees: {:.6} SOL, net: {:.3} SOL",
            token, stats.buy_count, stats.total_sol_spent, stats.sell_count, 
            stats.total_sol_received, stats.total_fees, net_sol
        ));
    }
    
    // Display router statistics
    log(LogTag::Transactions, "STATS", "Router Usage:");
    for (router, count) in &router_stats {
        log(LogTag::Transactions, "STATS", &format!("  {}: {} swaps", router, count));
    }
    
    // Display overall PnL
    log(LogTag::Transactions, "STATS", &format!(
        "Overall Performance: {} profitable, {} loss swaps, estimated P&L: {:.6} SOL",
        profitable_swaps, loss_swaps, total_profit_loss
    ));
    
    log(LogTag::Transactions, "STATS", "=== END STATISTICS ===");
}

/// Analyze all positions with comprehensive lifecycle tracking and PnL
async fn analyze_all_positions(wallet_pubkey: Pubkey) {
    log(LogTag::Transactions, "INFO", "Starting comprehensive position analysis for all transactions");

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Analyze positions
    match manager.analyze_positions(None).await {
        Ok(()) => {
            log(LogTag::Transactions, "SUCCESS", "Position analysis completed successfully");
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to analyze positions: {}", e));
        }
    }
}

#[derive(Debug)]
struct TokenSwapStats {
    buy_count: i32,
    sell_count: i32,
    total_sol_spent: f64,
    total_sol_received: f64,
    total_fees: f64,
}

impl TokenSwapStats {
    fn new() -> Self {
        Self {
            buy_count: 0,
            sell_count: 0,
            total_sol_spent: 0.0,
            total_sol_received: 0.0,
            total_fees: 0.0,
        }
    }
}

/// Analyze ALL transaction types (not just swaps) with comprehensive breakdown
async fn analyze_all_transactions(wallet_pubkey: Pubkey, max_count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Starting comprehensive analysis of ALL transaction types (max {} transactions)", max_count
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Get all cached transactions first
    match manager.recalculate_all_cached_transactions(Some(max_count)).await {
        Ok(transactions) => {
            log(LogTag::Transactions, "SUCCESS", &format!(
                "Loaded {} total transactions for comprehensive analysis", transactions.len()
            ));
            
            if transactions.is_empty() {
                log(LogTag::Transactions, "WARN", "No transactions found. Try fetching from blockchain first with --fetch-all-blockchain");
                return;
            }

            // Display comprehensive transaction analysis table
            display_all_transactions_table(&transactions);
            
            // Display detailed statistics breakdown
            display_comprehensive_transaction_statistics(&transactions);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load transactions: {}", e));
        }
    }
}

/// Display comprehensive table of ALL transaction types
fn display_all_transactions_table(transactions: &[screenerbot::transactions::Transaction]) {
    log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE TRANSACTION ANALYSIS ===");
    log(LogTag::Transactions, "TABLE", "Sig      Slot         Type                    Details                          SOL Change   Fee SOL      Success");
    log(LogTag::Transactions, "TABLE", "-----------------------------------------------------------------------------------------------------------------------");
    
    for transaction in transactions.iter().take(50) { // Show first 50 transactions
        let slot = transaction.slot.unwrap_or(0);
        let sol_change_str = format!("{:+.6}", transaction.sol_balance_change);
        let fee_str = format!("{:.6}", transaction.fee_sol);
        let success_icon = if transaction.success { "✅" } else { "❌" };
        let sig_short = &transaction.signature[..8.min(transaction.signature.len())];
        let _timestamp = transaction.timestamp.format("%H:%M:%S").to_string();
        
        let (tx_type, details) = match &transaction.transaction_type {
            screenerbot::transactions::TransactionType::SwapSolToToken { token_mint: _, sol_amount, token_amount, router } => {
                ("SOL->Token", format!("{:.4} SOL -> {:.0} tokens via {}", sol_amount, token_amount, router))
            }
            screenerbot::transactions::TransactionType::SwapTokenToSol { token_mint: _, token_amount, sol_amount, router } => {
                ("Token->SOL", format!("{:.0} tokens -> {:.4} SOL via {}", token_amount, sol_amount, router))
            }
            screenerbot::transactions::TransactionType::SwapTokenToToken { from_mint: _, to_mint: _, from_amount, to_amount, router } => {
                ("Token->Token", format!("{:.0} -> {:.0} via {}", from_amount, to_amount, router))
            }
            screenerbot::transactions::TransactionType::SolTransfer { from, to, amount } => {
                let from_short = if from.len() >= 8 { &from[..8] } else { from };
                let to_short = if to.len() >= 8 { &to[..8] } else { to };
                ("SOL Transfer", format!("{:.4} SOL: {}...->{}...", amount, from_short, to_short))
            }
            screenerbot::transactions::TransactionType::TokenTransfer { mint: _, from, to, amount } => {
                let from_short = if from.len() >= 8 { &from[..8] } else { from };
                let to_short = if to.len() >= 8 { &to[..8] } else { to };
                ("Token Transfer", format!("{:.0} tokens: {}...->{}...", amount, from_short, to_short))
            }
            screenerbot::transactions::TransactionType::AtaCreate { mint, owner: _, ata_address: _, cost } => {
                let mint_short = if mint.len() >= 8 { &mint[..8] } else { mint };
                ("ATA Create", format!("Token: {}..., Cost: {:.6} SOL", mint_short, cost))
            }
            screenerbot::transactions::TransactionType::AtaClose { mint, owner: _, ata_address: _, rent_reclaimed } => {
                let mint_short = if mint.len() >= 8 { &mint[..8] } else { mint };
                ("ATA Close", format!("Token: {}..., Reclaimed: {:.6} SOL", mint_short, rent_reclaimed))
            }
            screenerbot::transactions::TransactionType::StakingDelegate { stake_account: _, validator, amount } => {
                let validator_short = if validator.len() >= 8 { &validator[..8] } else { validator };
                ("Staking", format!("Delegate {:.4} SOL to {}...", amount, validator_short))
            }
            screenerbot::transactions::TransactionType::StakingWithdraw { stake_account: _, amount } => {
                ("Staking", format!("Withdraw {:.4} SOL", amount))
            }
            screenerbot::transactions::TransactionType::ProgramDeploy { program_id, deployer: _ } => {
                let program_short = if program_id.len() >= 8 { &program_id[..8] } else { program_id };
                ("Program Deploy", format!("Program: {}...", program_short))
            }
            screenerbot::transactions::TransactionType::ProgramUpgrade { program_id, authority: _ } => {
                let program_short = if program_id.len() >= 8 { &program_id[..8] } else { program_id };
                ("Program Upgrade", format!("Program: {}...", program_short))
            }
            screenerbot::transactions::TransactionType::ComputeBudget { compute_units, compute_unit_price } => {
                ("Compute Budget", format!("Units: {}, Price: {}", compute_units, compute_unit_price))
            }
            screenerbot::transactions::TransactionType::SpamBulk { transaction_count, suspected_spam_type } => {
                ("Spam Bulk", format!("{} txs, Type: {}", transaction_count, suspected_spam_type))
            }
            screenerbot::transactions::TransactionType::NftMint { collection_id: _, leaf_asset_id, nft_type } => {
                let leaf_short = if leaf_asset_id.len() >= 8 { &leaf_asset_id[..8] } else { leaf_asset_id };
                ("NFT Mint", format!("{}: {}...", nft_type, leaf_short))
            }
            screenerbot::transactions::TransactionType::Spam => {
                ("Spam", "Spam transaction".to_string())
            }
            screenerbot::transactions::TransactionType::Unknown => {
                ("Unknown", "Unidentified transaction type".to_string())
            }
        };

        log(LogTag::Transactions, "TABLE", &format!(
            "{:<8} {:<12} {:<19} {:<32} {:<12} {:<12} {}",
            sig_short, slot, tx_type, 
            if details.len() > 30 { format!("{}...", &details[..27]) } else { details },
            sol_change_str, fee_str, success_icon
        ));
    }
    
    if transactions.len() > 50 {
        log(LogTag::Transactions, "TABLE", &format!(
            "... and {} more transactions (showing first 50)", transactions.len() - 50
        ));
    }
    
    log(LogTag::Transactions, "TABLE", "-----------------------------------------------------------------------------------------------------------------------");
    log(LogTag::Transactions, "TABLE", "=== END TRANSACTION TABLE ===");
}

/// Display comprehensive statistics for all transaction types
fn display_comprehensive_transaction_statistics(transactions: &[screenerbot::transactions::Transaction]) {
    log(LogTag::Transactions, "STATS", "=== COMPREHENSIVE TRANSACTION STATISTICS ===");
    
    let mut type_counts = HashMap::new();
    let mut successful_count = 0;
    let mut failed_count = 0;
    let mut total_fees = 0.0;
    let mut total_sol_in = 0.0;
    let mut total_sol_out = 0.0;
    let mut oldest_timestamp = transactions[0].timestamp;
    let mut newest_timestamp = transactions[0].timestamp;
    
    // Count transaction types and calculate statistics
    for transaction in transactions {
        let tx_type = match &transaction.transaction_type {
            screenerbot::transactions::TransactionType::SwapSolToToken { .. } => "Swap: SOL->Token",
            screenerbot::transactions::TransactionType::SwapTokenToSol { .. } => "Swap: Token->SOL", 
            screenerbot::transactions::TransactionType::SwapTokenToToken { .. } => "Swap: Token->Token",
            screenerbot::transactions::TransactionType::SolTransfer { .. } => "SOL Transfer",
            screenerbot::transactions::TransactionType::TokenTransfer { .. } => "Token Transfer",
            screenerbot::transactions::TransactionType::AtaCreate { .. } => "ATA Create",
            screenerbot::transactions::TransactionType::AtaClose { .. } => "ATA Close",
            screenerbot::transactions::TransactionType::StakingDelegate { .. } => "Staking Delegate",
            screenerbot::transactions::TransactionType::StakingWithdraw { .. } => "Staking Withdraw",
            screenerbot::transactions::TransactionType::ProgramDeploy { .. } => "Program Deploy",
            screenerbot::transactions::TransactionType::ProgramUpgrade { .. } => "Program Upgrade", 
            screenerbot::transactions::TransactionType::ComputeBudget { .. } => "Compute Budget",
            screenerbot::transactions::TransactionType::SpamBulk { .. } => "Spam Bulk",
            screenerbot::transactions::TransactionType::NftMint { .. } => "NFT Mint",
            screenerbot::transactions::TransactionType::Spam => "Spam",
            screenerbot::transactions::TransactionType::Unknown => "Unknown",
        };
        
        *type_counts.entry(tx_type.to_string()).or_insert(0) += 1;
        
        if transaction.success {
            successful_count += 1;
        } else {
            failed_count += 1;
        }
        
        total_fees += transaction.fee_sol;
        
        if transaction.sol_balance_change > 0.0 {
            total_sol_in += transaction.sol_balance_change;
        } else {
            total_sol_out += transaction.sol_balance_change.abs();
        }
        
        if transaction.timestamp < oldest_timestamp {
            oldest_timestamp = transaction.timestamp;
        }
        if transaction.timestamp > newest_timestamp {
            newest_timestamp = transaction.timestamp;
        }
    }
    
    // Display overall statistics
    log(LogTag::Transactions, "STATS", &format!("Total Transactions: {}", transactions.len()));
    log(LogTag::Transactions, "STATS", &format!("Successful: {} ({:.1}%)", 
        successful_count, (successful_count as f64 / transactions.len() as f64) * 100.0));
    log(LogTag::Transactions, "STATS", &format!("Failed: {} ({:.1}%)", 
        failed_count, (failed_count as f64 / transactions.len() as f64) * 100.0));
    log(LogTag::Transactions, "STATS", &format!("Time Range: {} to {}", oldest_timestamp, newest_timestamp));
    
    let time_span = newest_timestamp.signed_duration_since(oldest_timestamp);
    log(LogTag::Transactions, "STATS", &format!("Time Span: {} days", time_span.num_days()));
    
    log(LogTag::Transactions, "STATS", &format!("Total Fees Paid: {:.6} SOL", total_fees));
    log(LogTag::Transactions, "STATS", &format!("Total SOL Received: +{:.6} SOL", total_sol_in));
    log(LogTag::Transactions, "STATS", &format!("Total SOL Spent: -{:.6} SOL", total_sol_out));
    log(LogTag::Transactions, "STATS", &format!("Net SOL Change: {:.6} SOL", total_sol_in - total_sol_out));
    
    log(LogTag::Transactions, "STATS", "");
    log(LogTag::Transactions, "STATS", "Transaction Type Breakdown:");
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending
    
    for (tx_type, count) in sorted_types {
        let percentage = (*count as f64 / transactions.len() as f64) * 100.0;
        log(LogTag::Transactions, "STATS", &format!(
            "  {}: {} ({:.1}%)", tx_type, count, percentage
        ));
    }
    
    log(LogTag::Transactions, "STATS", "=== END COMPREHENSIVE STATISTICS ===");
}

/// Fetch new transactions from the wallet that are not yet in cache
async fn fetch_new_transactions(wallet_pubkey: Pubkey, max_count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Fetching new transactions from wallet (max {} transactions)", max_count
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Initialize known signatures to detect new ones
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize known signatures: {}", e));
        return;
    }

    log(LogTag::Transactions, "INFO", &format!(
        "Loaded {} known signatures from cache", manager.known_signatures.len()
    ));

    // Get new transactions
    match manager.check_new_transactions().await {
        Ok(new_signatures) => {
            let limited_signatures: Vec<_> = new_signatures.into_iter().take(max_count).collect();
            
            log(LogTag::Transactions, "SUCCESS", &format!(
                "Found {} new transactions", limited_signatures.len()
            ));

            if limited_signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No new transactions found");
                return;
            }

            let mut processed_count = 0;
            let mut error_count = 0;
            let start_time = Instant::now();

            for (index, signature) in limited_signatures.iter().enumerate() {
                log(LogTag::Transactions, "PROGRESS", &format!(
                    "Processing new transaction {}/{}: {}...", 
                    index + 1, limited_signatures.len(), &signature[..8]
                ));

                match manager.process_transaction(signature).await {
                    Ok(_) => {
                        processed_count += 1;
                        log(LogTag::Transactions, "SUCCESS", &format!("✅ Processed {}", &signature[..8]));
                    }
                    Err(e) => {
                        error_count += 1;
                        log(LogTag::Transactions, "ERROR", &format!("❌ Failed to process {}: {}", &signature[..8], e));
                    }
                }

                // Add small delay to avoid overwhelming the system
                if index % 5 == 0 && index > 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            let total_time = start_time.elapsed();
            log(LogTag::Transactions, "RESULTS", "=== FETCH NEW TRANSACTIONS RESULTS ===");
            log(LogTag::Transactions, "RESULTS", &format!("New Transactions Found: {}", limited_signatures.len()));
            log(LogTag::Transactions, "RESULTS", &format!("Successfully Processed: {}", processed_count));
            log(LogTag::Transactions, "RESULTS", &format!("Errors: {}", error_count));
            log(LogTag::Transactions, "RESULTS", &format!("Success Rate: {:.1}%", 
                (processed_count as f64 / limited_signatures.len() as f64) * 100.0));
            log(LogTag::Transactions, "RESULTS", &format!("Processing Time: {:.2}s", total_time.as_secs_f64()));
            log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to fetch new transactions: {}", e));
        }
    }
}

/// Fetch all transactions from cache (fast operation)
async fn fetch_all_cached_transactions(_wallet_pubkey: Pubkey, max_count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Fetching all cached transactions (max {} transactions)", max_count
    ));

    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        log(LogTag::Transactions, "WARN", "Cache directory does not exist");
        return;
    }

    let mut cached_transactions = Vec::new();
    let mut error_count = 0;

    match fs::read_dir(&cache_dir) {
        Ok(entries) => {
            for entry in entries {
                if cached_transactions.len() >= max_count {
                    break;
                }

                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                        match analyze_cache_file(&path).await {
                            Ok(transaction) => {
                                cached_transactions.push(transaction);
                            }
                            Err(e) => {
                                error_count += 1;
                                log(LogTag::Transactions, "WARN", &format!(
                                    "Failed to parse cached transaction {}: {}", 
                                    path.file_name().unwrap_or_default().to_string_lossy(), e
                                ));
                            }
                        }
                    }
                }
            }

            log(LogTag::Transactions, "SUCCESS", &format!(
                "Loaded {} cached transactions from disk", cached_transactions.len()
            ));

            if error_count > 0 {
                log(LogTag::Transactions, "WARN", &format!(
                    "Failed to load {} cached transactions due to parse errors", error_count
                ));
            }

            // Display summary statistics
            display_transaction_summary(&cached_transactions);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to read cache directory: {}", e));
        }
    }
}

/// Display transaction summary statistics
fn display_transaction_summary(transactions: &[Transaction]) {
    if transactions.is_empty() {
        return;
    }

    log(LogTag::Transactions, "SUMMARY", "=== TRANSACTION SUMMARY ===");
    
    let mut type_counts = HashMap::new();
    let mut successful_count = 0;
    let mut total_fees = 0.0;
    let mut total_sol_volume = 0.0;

    let mut oldest_timestamp = transactions[0].timestamp;
    let mut newest_timestamp = transactions[0].timestamp;

    for transaction in transactions {
        // Count transaction types
        let tx_type = format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Unknown").to_string();
        *type_counts.entry(tx_type).or_insert(0) += 1;

        if transaction.success {
            successful_count += 1;
        }

        total_fees += transaction.fee_sol;
        total_sol_volume += transaction.sol_balance_change.abs();

        if transaction.timestamp < oldest_timestamp {
            oldest_timestamp = transaction.timestamp;
        }
        if transaction.timestamp > newest_timestamp {
            newest_timestamp = transaction.timestamp;
        }
    }

    log(LogTag::Transactions, "SUMMARY", &format!("Total Transactions: {}", transactions.len()));
    log(LogTag::Transactions, "SUMMARY", &format!("Successful: {} ({:.1}%)", 
        successful_count, (successful_count as f64 / transactions.len() as f64) * 100.0));
    log(LogTag::Transactions, "SUMMARY", &format!("Time Range: {} to {}", oldest_timestamp, newest_timestamp));
    
    let time_span = newest_timestamp.signed_duration_since(oldest_timestamp);
    log(LogTag::Transactions, "SUMMARY", &format!("Time Span: {} days", time_span.num_days()));
    
    log(LogTag::Transactions, "SUMMARY", &format!("Total Fees Paid: {:.6} SOL", total_fees));
    log(LogTag::Transactions, "SUMMARY", &format!("Total SOL Volume: {:.6} SOL", total_sol_volume));

    log(LogTag::Transactions, "SUMMARY", "Transaction Types:");
    for (tx_type, count) in &type_counts {
        let percentage = (count * 100) as f64 / transactions.len() as f64;
        log(LogTag::Transactions, "SUMMARY", &format!("  {}: {} ({:.1}%)", tx_type, count, percentage));
    }
    
    log(LogTag::Transactions, "SUMMARY", "=== END SUMMARY ===");
}

/// Load wallet pubkey from configuration
async fn load_wallet_pubkey() -> Result<Pubkey, Box<dyn std::error::Error>> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    
    Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address: {}", e).into())
}

/// Monitor wallet transactions in real-time
async fn monitor_transactions(wallet_pubkey: Pubkey, duration_seconds: u64) {
    log(LogTag::Transactions, "INFO", &format!(
        "Starting real-time transaction monitoring for {} seconds", duration_seconds
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };
    
    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize known signatures: {}", e));
        return;
    }

    log(LogTag::Transactions, "INFO", &format!(
        "Loaded {} known signatures from cache", manager.known_signatures.len()
    ));

    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(duration_seconds);
    let mut check_interval = interval(Duration::from_secs(5));

    let mut total_new_transactions = 0;
    let mut total_processed = 0;

    while Instant::now() < end_time {
        tokio::select! {
            _ = check_interval.tick() => {
                match manager.check_new_transactions().await {
                    Ok(new_signatures) => {
                        if !new_signatures.is_empty() {
                            total_new_transactions += new_signatures.len();
                            log(LogTag::Transactions, "NEW", &format!(
                                "Found {} new transactions", new_signatures.len()
                            ));

                            // Process each new transaction
                            for signature in new_signatures {
                                match manager.process_transaction(&signature).await {
                                    Ok(transaction) => {
                                        total_processed += 1;
                                        log_transaction_summary(&transaction);
                                    }
                                    Err(e) => {
                                        log(LogTag::Transactions, "ERROR", &format!(
                                            "Failed to process transaction {}: {}", 
                                            &signature[..8], e
                                        ));
                                    }
                                }
                            }
                        } else {
                            log(LogTag::Transactions, "DEBUG", "No new transactions found");
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to check new transactions: {}", e));
                    }
                }

                // Display stats
                let elapsed = start_time.elapsed().as_secs();
                let remaining = duration_seconds.saturating_sub(elapsed);
                log(LogTag::Transactions, "STATS", &format!(
                    "Elapsed: {}s | Remaining: {}s | New: {} | Processed: {}",
                    elapsed, remaining, total_new_transactions, total_processed
                ));
            }
        }
    }

    log(LogTag::Transactions, "INFO", &format!(
        "Monitoring completed. Total new transactions: {}, Total processed: {}",
        total_new_transactions, total_processed
    ));
}

/// Analyze a specific transaction by signature
async fn analyze_specific_transaction(signature: &str, force_recalculate: bool) {
    log(LogTag::Transactions, "INFO", &format!(
        "Analyzing transaction: {} (force_recalculate: {})", 
        signature, force_recalculate
    ));

    // First check if it's already cached (skip if force_recalculate is true)
    if !force_recalculate {
        match get_transaction(signature).await {
            Ok(Some(transaction)) => {
                log(LogTag::Transactions, "CACHE", "Transaction found in cache");
                
                // Check if we have comprehensive analysis data (fee_breakdown)
                if transaction.fee_breakdown.is_some() {
                    log(LogTag::Transactions, "INFO", "Comprehensive analysis data found in cache");
                    display_detailed_transaction_info(&transaction);
                    return;
                } else {
                    log(LogTag::Transactions, "INFO", "No comprehensive analysis in cache, forcing re-analysis");
                    // Continue to re-analysis below
                }
            }
            Ok(None) => {
                log(LogTag::Transactions, "INFO", "Transaction not in cache, fetching from RPC");
            }
            Err(e) => {
                log(LogTag::Transactions, "WARN", &format!("Error checking cache: {}", e));
            }
        }
    } else {
        log(LogTag::Transactions, "INFO", "Force recalculation enabled - bypassing cache");
        
        // Delete cached transaction file to force complete recalculation
        let cache_dir = get_transactions_cache_dir();
        let cache_file = cache_dir.join(format!("{}.json", signature));
        if cache_file.exists() {
            if let Err(e) = fs::remove_file(&cache_file) {
                log(LogTag::Transactions, "WARN", &format!(
                    "Failed to delete cached transaction file: {}", e
                ));
            } else {
                log(LogTag::Transactions, "INFO", "Deleted cached transaction file for complete recalculation");
            }
        }
    }

    // Load wallet and create manager
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet: {}", e));
            return;
        }
    };

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(mut manager) => {
            // Force enable debug mode for enhanced ATA analysis when force_recalculate is true
            if force_recalculate {
                manager.debug_enabled = true;
                log(LogTag::Transactions, "INFO", "Debug mode enabled for enhanced ATA analysis");
                
                // Add debug-transactions flag to enable enhanced ATA analysis logging
                let mut current_args = get_cmd_args();
                current_args.push("--debug-transactions".to_string());
                args_set_cmd_args(current_args);
            }
            manager
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Process the transaction with comprehensive analysis
    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(LogTag::Transactions, "SUCCESS", "Transaction analyzed successfully");
            
            // Force comprehensive analysis if not already done (check if fee_breakdown is None)
            if transaction.fee_breakdown.is_none() {
                log(LogTag::Transactions, "INFO", "Running additional comprehensive analysis for complete fee breakdown");
                // Comprehensive analysis is already called in process_transaction, but let's ensure debug mode is enabled
            }
            
            display_detailed_transaction_info(&transaction);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to analyze transaction: {}", e));
        }
    }
}

/// Test transaction analyzer on recent transactions
async fn test_transaction_analyzer(wallet_pubkey: Pubkey, count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Testing transaction analyzer on {} recent transactions", count
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Get recent transactions
    match manager.check_new_transactions().await {
        Ok(signatures) => {
            let test_signatures: Vec<_> = signatures.into_iter().take(count).collect();
            
            log(LogTag::Transactions, "INFO", &format!(
                "Found {} signatures to test", test_signatures.len()
            ));

            let mut stats = AnalyzerTestStats::new();
            let start_time = Instant::now();

            for (index, signature) in test_signatures.iter().enumerate() {
                let tx_start = Instant::now();
                
                match manager.process_transaction(signature).await {
                    Ok(transaction) => {
                        let processing_time = tx_start.elapsed();
                        stats.record_success(&transaction, processing_time);
                        
                        log(LogTag::Transactions, "TEST", &format!(
                            "[{}/{}] {} - {:?} - {:.2}ms",
                            index + 1,
                            test_signatures.len(),
                            &signature[..8],
                            transaction.transaction_type,
                            processing_time.as_millis()
                        ));
                    }
                    Err(e) => {
                        stats.record_error(&e);
                        log(LogTag::Transactions, "ERROR", &format!(
                            "[{}/{}] {} - Error: {}",
                            index + 1,
                            test_signatures.len(),
                            &signature[..8],
                            e
                        ));
                    }
                }
            }

            let total_time = start_time.elapsed();
            stats.display_results(total_time);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get recent transactions: {}", e));
        }
    }
}

/// Debug the transaction cache system
async fn debug_cache_system() {
    log(LogTag::Transactions, "INFO", "Debugging transaction cache system");

    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        log(LogTag::Transactions, "WARN", "Cache directory does not exist");
        return;
    }

    // Scan cache directory
    match fs::read_dir(&cache_dir) {
        Ok(entries) => {
            let mut cache_stats = CacheStats::new();

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        match analyze_cache_file(&path).await {
                            Ok(transaction) => {
                                cache_stats.record_transaction(&transaction);
                            }
                            Err(e) => {
                                cache_stats.record_error();
                                log(LogTag::Transactions, "ERROR", &format!(
                                    "Failed to read cache file {}: {}", 
                                    path.display(), e
                                ));
                            }
                        }
                    }
                }
            }

            cache_stats.display_results();
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to read cache directory: {}", e));
        }
    }
}

/// Recalculate all analysis parameters without deleting raw transaction data
async fn recalculate_transaction_cache() {
    log(LogTag::Transactions, "INFO", "Recalculating transaction cache (preserving raw data)");

    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        log(LogTag::Transactions, "INFO", "Cache directory does not exist");
        return;
    }

    // Get wallet pubkey for the transactions manager
    let wallet_address = match get_wallet_address() {
        Ok(address) => address,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get wallet address: {}", e));
            return;
        }
    };

    let wallet_pubkey = match Pubkey::from_str(&wallet_address) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to parse wallet address: {}", e));
            return;
        }
    };

    // Create manager for re-analysis
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    match fs::read_dir(&cache_dir) {
        Ok(entries) => {
            let mut updated_count = 0;
            let mut error_count = 0;
            let mut total_files = 0;

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        total_files += 1;
                        
                        // Read existing transaction
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                match serde_json::from_str::<Transaction>(&content) {
                                    Ok(mut transaction) => {
                                        let signature = transaction.signature.clone();
                                        
                                        log(LogTag::Transactions, "RECALC", &format!(
                                            "Recalculating analysis for: {}...", &signature[..8]
                                        ));

                                        // Preserve raw blockchain data but recalculate all analysis
                                        match manager.recalculate_transaction_analysis(&mut transaction).await {
                                            Ok(_) => {
                                                // Save updated transaction back to file
                                                match serde_json::to_string_pretty(&transaction) {
                                                    Ok(updated_json) => {
                                                        match fs::write(&path, updated_json) {
                                                            Ok(_) => {
                                                                updated_count += 1;
                                                                log(LogTag::Transactions, "SUCCESS", &format!(
                                                                    "✅ Updated analysis: {}", &signature[..8]
                                                                ));
                                                            }
                                                            Err(e) => {
                                                                error_count += 1;
                                                                log(LogTag::Transactions, "ERROR", &format!(
                                                                    "Failed to save updated transaction {}: {}", &signature[..8], e
                                                                ));
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error_count += 1;
                                                        log(LogTag::Transactions, "ERROR", &format!(
                                                            "Failed to serialize updated transaction {}: {}", &signature[..8], e
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error_count += 1;
                                                log(LogTag::Transactions, "ERROR", &format!(
                                                    "Failed to recalculate analysis for {}: {}", &signature[..8], e
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error_count += 1;
                                        log(LogTag::Transactions, "ERROR", &format!(
                                            "Failed to parse transaction file {}: {}", path.display(), e
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                error_count += 1;
                                log(LogTag::Transactions, "ERROR", &format!(
                                    "Failed to read transaction file {}: {}", path.display(), e
                                ));
                            }
                        }
                    }
                }
            }

            log(LogTag::Transactions, "SUCCESS", &format!(
                "Cache recalculation complete: {} of {} files updated, {} errors", 
                updated_count, total_files, error_count
            ));
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to read cache directory: {}", e));
        }
    }
}

/// Update and re-analyze all cached transactions (preserving raw data)
async fn update_transaction_cache(wallet_pubkey: Pubkey, max_count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Updating transaction cache with re-analysis (max {} transactions) - preserving raw data", max_count
    ));

    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        log(LogTag::Transactions, "INFO", "Cache directory does not exist");
        return;
    }

    // Create manager for re-analysis
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    log(LogTag::Transactions, "INFO", "Scanning cache directory for transactions to update");

    let mut updated_count = 0;
    let mut error_count = 0;
    let mut signatures_to_process = Vec::new();

    // Collect all transaction signatures from cache files
    match fs::read_dir(&cache_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                            signatures_to_process.push(file_name.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to read cache directory: {}", e));
            return;
        }
    }

    let total_signatures = signatures_to_process.len().min(max_count);
    signatures_to_process.truncate(max_count);

    log(LogTag::Transactions, "INFO", &format!(
        "Found {} cached transactions, processing {} with updated analysis", 
        signatures_to_process.len(), total_signatures
    ));

    let start_time = Instant::now();
    let mut swap_count = 0;
    let mut unknown_count = 0;

    for (index, signature) in signatures_to_process.iter().enumerate() {
        log(LogTag::Transactions, "PROGRESS", &format!(
            "Processing transaction {}/{}: {}...", 
            index + 1, total_signatures, &signature[..8]
        ));

        // Read existing cached transaction
        let transaction_path = cache_dir.join(format!("{}.json", signature));
        match fs::read_to_string(&transaction_path) {
            Ok(content) => {
                match serde_json::from_str::<Transaction>(&content) {
                    Ok(mut transaction) => {
                        // Recalculate analysis preserving raw data
                        match manager.recalculate_transaction_analysis(&mut transaction).await {
                            Ok(_) => {
                                // Save updated transaction back to cache
                                match serde_json::to_string_pretty(&transaction) {
                                    Ok(updated_json) => {
                                        match fs::write(&transaction_path, updated_json) {
                                            Ok(_) => {
                                                updated_count += 1;
                                                
                                                // Log transaction type for statistics
                                                match &transaction.transaction_type {
                                                    TransactionType::SwapSolToToken { router, .. } |
                                                    TransactionType::SwapTokenToSol { router, .. } |
                                                    TransactionType::SwapTokenToToken { router, .. } => {
                                                        swap_count += 1;
                                                        log(LogTag::Transactions, "SWAP", &format!(
                                                            "✅ Updated swap via {}: {} ({})", 
                                                            router, &signature[..8], 
                                                            format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Swap")
                                                        ));
                                                    }
                                                    TransactionType::Unknown => {
                                                        unknown_count += 1;
                                                        log(LogTag::Transactions, "UNKNOWN", &format!(
                                                            "❓ Updated unknown transaction: {}", &signature[..8]
                                                        ));
                                                    }
                                                    _ => {
                                                        log(LogTag::Transactions, "OTHER", &format!(
                                                            "ℹ️  Updated {}: {}", 
                                                            format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Other"),
                                                            &signature[..8]
                                                        ));
                                                    }
                                                }

                                                // Show comprehensive token info if it's a swap with token data
                                                if let Some(ref token_info) = transaction.token_info {
                                                    log(LogTag::Transactions, "TOKEN", &format!(
                                                        "   Token: {} ({}) - Price: {:.9} SOL (source: {:?})",
                                                        token_info.symbol, 
                                                        &token_info.mint[..8],
                                                        token_info.current_price_sol.unwrap_or(0.0),
                                                        token_info.price_source.as_ref().unwrap_or(&PriceSourceType::DexScreenerApi)
                                                    ));
                                                }
                                            }
                                            Err(e) => {
                                                error_count += 1;
                                                log(LogTag::Transactions, "ERROR", &format!(
                                                    "Failed to save updated transaction {}: {}", &signature[..8], e
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error_count += 1;
                                        log(LogTag::Transactions, "ERROR", &format!(
                                            "Failed to serialize updated transaction {}: {}", &signature[..8], e
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                error_count += 1;
                                log(LogTag::Transactions, "ERROR", &format!(
                                    "Failed to recalculate analysis for {}: {}", &signature[..8], e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        log(LogTag::Transactions, "ERROR", &format!(
                            "Failed to parse cached transaction {}: {}", &signature[..8], e
                        ));
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                log(LogTag::Transactions, "ERROR", &format!(
                    "Failed to read cached transaction {}: {}", &signature[..8], e
                ));
            }
        }

        // Add small delay to avoid overwhelming the system
        if index % 10 == 0 && index > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    let total_time = start_time.elapsed();

    log(LogTag::Transactions, "RESULTS", "=== CACHE UPDATE RESULTS ===");
    log(LogTag::Transactions, "RESULTS", &format!("Total Processed: {}", total_signatures));
    log(LogTag::Transactions, "RESULTS", &format!("Successfully Updated: {}", updated_count));
    log(LogTag::Transactions, "RESULTS", &format!("Errors: {}", error_count));
    log(LogTag::Transactions, "RESULTS", &format!("Swap Transactions: {}", swap_count));
    log(LogTag::Transactions, "RESULTS", &format!("Unknown Transactions: {}", unknown_count));
    log(LogTag::Transactions, "RESULTS", &format!("Other Transactions: {}", updated_count - swap_count - unknown_count));
    log(LogTag::Transactions, "RESULTS", &format!("Success Rate: {:.1}%", 
        (updated_count as f64 / total_signatures as f64) * 100.0));
    log(LogTag::Transactions, "RESULTS", &format!("Processing Time: {:.2}s", total_time.as_secs_f64()));
    
    if updated_count > 0 {
        let avg_time = total_time / updated_count as u32;
        log(LogTag::Transactions, "RESULTS", &format!("Avg Time per Transaction: {:.2}ms", avg_time.as_millis()));
    }
    
    log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");

    // After updating cache, show comprehensive swap analysis if any swaps were found
    if swap_count > 0 {
        log(LogTag::Transactions, "INFO", "Performing comprehensive swap analysis on updated cache...");
        
        match manager.get_all_swap_transactions().await {
            Ok(swaps) => {
                log(LogTag::Transactions, "SUCCESS", &format!("Found {} total swap transactions for analysis", swaps.len()));
                
                // Display comprehensive analysis table
                manager.display_swap_analysis_table(&swaps);
                
                // Additional statistics
                display_detailed_swap_statistics(&swaps);
            }
            Err(e) => {
                log(LogTag::Transactions, "ERROR", &format!("Failed to analyze updated swaps: {}", e));
            }
        }
    }
}

/// Fetch and analyze ALL wallet transactions from blockchain (comprehensive analysis)
async fn fetch_all_wallet_transactions(wallet_pubkey: Pubkey, max_count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Fetching and analyzing ALL wallet transactions from blockchain (max {} transactions)", max_count
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Use the comprehensive transaction fetching method
    match manager.fetch_all_wallet_transactions(max_count).await {
        Ok(transactions) => {
            log(LogTag::Transactions, "SUCCESS", &format!(
                "Successfully fetched and analyzed {} transactions", transactions.len()
            ));

            // Analyze and categorize transactions
            let mut swap_count = 0;
            let mut unknown_count = 0;
            let mut transfer_count = 0;
            let mut spam_count = 0;
            let mut ata_count = 0;
            let mut staking_count = 0;
            let mut program_count = 0;
            let mut other_count = 0;

            for transaction in &transactions {
                match &transaction.transaction_type {
                    TransactionType::SwapSolToToken { .. } |
                    TransactionType::SwapTokenToSol { .. } |
                    TransactionType::SwapTokenToToken { .. } => swap_count += 1,
                    TransactionType::SolTransfer { .. } |
                    TransactionType::TokenTransfer { .. } => transfer_count += 1,
                    TransactionType::Spam => spam_count += 1,
                    TransactionType::SpamBulk { .. } => spam_count += 1,
                    TransactionType::AtaCreate { .. } |
                    TransactionType::AtaClose { .. } => ata_count += 1,
                    TransactionType::StakingDelegate { .. } |
                    TransactionType::StakingWithdraw { .. } => staking_count += 1,
                    TransactionType::ProgramDeploy { .. } |
                    TransactionType::ProgramUpgrade { .. } => program_count += 1,
                    TransactionType::ComputeBudget { .. } => other_count += 1,
                    TransactionType::NftMint { .. } => other_count += 1,
                    TransactionType::Unknown => unknown_count += 1,
                }
            }

            log(LogTag::Transactions, "ANALYSIS", "=== COMPREHENSIVE WALLET ANALYSIS ===");
            log(LogTag::Transactions, "ANALYSIS", &format!("Total Transactions: {}", transactions.len()));
            log(LogTag::Transactions, "ANALYSIS", &format!("Swap Transactions: {}", swap_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Transfer Transactions: {}", transfer_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("ATA Operations: {}", ata_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Staking Operations: {}", staking_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Program Operations: {}", program_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Other Operations: {}", other_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Spam Transactions: {}", spam_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Unknown Transactions: {}", unknown_count));
            log(LogTag::Transactions, "ANALYSIS", "=== END ANALYSIS ===");

            // Get comprehensive swap analysis if any swaps were found
            if swap_count > 0 {
                log(LogTag::Transactions, "INFO", "Performing comprehensive swap analysis...");
                
                match manager.get_all_swap_transactions().await {
                    Ok(swaps) => {
                        log(LogTag::Transactions, "SUCCESS", &format!("Found {} swap transactions for detailed analysis", swaps.len()));
                        
                        // Display comprehensive analysis table
                        manager.display_swap_analysis_table(&swaps);
                        
                        // Additional statistics
                        display_detailed_swap_statistics(&swaps);
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to analyze swaps: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to fetch all wallet transactions: {}", e));
        }
    }
}

/// Run performance benchmark tests
async fn run_benchmark_tests(wallet_pubkey: Pubkey, count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Running performance benchmark with {} transactions", count
    ));

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Get signatures for testing
    let signatures = match manager.check_new_transactions().await {
        Ok(sigs) => sigs.into_iter().take(count).collect::<Vec<_>>(),
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get signatures: {}", e));
            return;
        }
    };

    if signatures.is_empty() {
        log(LogTag::Transactions, "WARN", "No signatures available for benchmarking");
        return;
    }

    let mut benchmark = BenchmarkStats::new();
    let start_time = Instant::now();

    log(LogTag::Transactions, "INFO", &format!("Benchmarking {} signatures", signatures.len()));

    for (index, signature) in signatures.iter().enumerate() {
        let tx_start = Instant::now();
        
        match manager.process_transaction(signature).await {
            Ok(transaction) => {
                let processing_time = tx_start.elapsed();
                benchmark.record_transaction(&transaction, processing_time);
                
                if (index + 1) % 10 == 0 {
                    log(LogTag::Transactions, "PROGRESS", &format!(
                        "Processed {}/{} transactions", index + 1, signatures.len()
                    ));
                }
            }
            Err(e) => {
                benchmark.record_error();
                log(LogTag::Transactions, "ERROR", &format!("Benchmark error: {}", e));
            }
        }
    }

    let total_time = start_time.elapsed();
    benchmark.display_results(total_time, signatures.len());
}

/// Display transaction summary for monitoring
fn log_transaction_summary(transaction: &Transaction) {
    let tx_type_str = match &transaction.transaction_type {
        TransactionType::SwapSolToToken { token_mint: _, sol_amount, token_amount, router } => {
            format!("SOL->Token: {:.4} SOL -> {:.2} tokens via {}", sol_amount, token_amount, router)
        }
        TransactionType::SwapTokenToSol { token_mint: _, token_amount, sol_amount, router } => {
            format!("Token->SOL: {:.2} tokens -> {:.4} SOL via {}", token_amount, sol_amount, router)
        }
        TransactionType::SwapTokenToToken { from_mint: _, to_mint: _, from_amount, to_amount, router } => {
            format!("Token->Token: {:.2} -> {:.2} via {}", from_amount, to_amount, router)
        }
        TransactionType::SolTransfer { amount, .. } => {
            format!("SOL Transfer: {:.4} SOL", amount)
        }
        TransactionType::TokenTransfer { amount, .. } => {
            format!("Token Transfer: {:.2} tokens", amount)
        }
        TransactionType::AtaCreate { mint, cost, .. } => {
            format!("ATA Create: {} (cost: {:.6} SOL)", mint, cost)
        }
        TransactionType::AtaClose { mint, rent_reclaimed, .. } => {
            format!("ATA Close: {} (reclaimed: {:.6} SOL)", mint, rent_reclaimed)
        }
        TransactionType::StakingDelegate { stake_account, validator, amount } => {
            format!("Stake Delegate: {:.4} SOL to {} (account: {})", amount, validator, stake_account)
        }
        TransactionType::StakingWithdraw { stake_account, amount } => {
            format!("Stake Withdraw: {:.4} SOL (account: {})", amount, stake_account)
        }
        TransactionType::ProgramDeploy { program_id, deployer } => {
            format!("Program Deploy: {} by {}", program_id, deployer)
        }
        TransactionType::ProgramUpgrade { program_id, authority } => {
            format!("Program Upgrade: {} by {}", program_id, authority)
        }
        TransactionType::SpamBulk { transaction_count, suspected_spam_type } => {
            format!("Spam Bulk: {} transactions ({})", transaction_count, suspected_spam_type)
        }
        TransactionType::ComputeBudget { compute_units, compute_unit_price } => {
            format!("Compute Budget: {} units @ {} μSOL/unit", compute_units, compute_unit_price)
        }
        TransactionType::NftMint { collection_id: _, leaf_asset_id, nft_type } => {
            format!("NFT Mint: {} ({})", leaf_asset_id, nft_type)
        }
        TransactionType::Spam => "Spam".to_string(),
        TransactionType::Unknown => "Unknown".to_string(),
    };

    let direction_emoji = match transaction.direction {
        TransactionDirection::Incoming => "⬇️",
        TransactionDirection::Outgoing => "⬆️",
        TransactionDirection::Internal => "🔄",
    };

    log(LogTag::Transactions, "TX", &format!(
        "{} {} - {} - Fee: {:.6} SOL - {}",
        direction_emoji,
        &transaction.signature[..8],
        tx_type_str,
        transaction.fee_sol,
        if transaction.success { "✅" } else { "❌" }
    ));
}

/// Display detailed transaction information
fn display_detailed_transaction_info(transaction: &Transaction) {
    log(LogTag::Transactions, "DETAIL", "=== TRANSACTION DETAILS ===");
    log(LogTag::Transactions, "DETAIL", &format!("Signature: {}", transaction.signature));
    
    // Use blockchain timestamp if available, otherwise fall back to transaction timestamp
    let display_timestamp = if let Some(block_time) = transaction.block_time {
        DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
    } else {
        transaction.timestamp
    };
    log(LogTag::Transactions, "DETAIL", &format!("Timestamp: {}", display_timestamp));
    log(LogTag::Transactions, "DETAIL", &format!("Success: {}", transaction.success));
    log(LogTag::Transactions, "DETAIL", &format!("Finalized: {}", transaction.finalized));
    log(LogTag::Transactions, "DETAIL", &format!("Direction: {:?}", transaction.direction));
    log(LogTag::Transactions, "DETAIL", &format!("Fee (SOL): {:.9}", transaction.fee_sol));
    log(LogTag::Transactions, "DETAIL", &format!("SOL Balance Change: {:.9}", transaction.sol_balance_change));

    // Display comprehensive fee information if available
    if let Some(fee_breakdown) = &transaction.fee_breakdown {
        log(LogTag::Transactions, "DETAIL", "=== COMPREHENSIVE FEE BREAKDOWN ===");
        log(LogTag::Transactions, "DETAIL", &format!("Transaction Fee: {:.9} SOL", fee_breakdown.transaction_fee));
        log(LogTag::Transactions, "DETAIL", &format!("Router Fee: {:.9} SOL", fee_breakdown.router_fee));
        log(LogTag::Transactions, "DETAIL", &format!("Platform Fee: {:.9} SOL", fee_breakdown.platform_fee));
        log(LogTag::Transactions, "DETAIL", &format!("Priority Fee: {:.9} SOL", fee_breakdown.priority_fee));
        log(LogTag::Transactions, "DETAIL", &format!("ATA Creation Cost: {:.9} SOL", fee_breakdown.ata_creation_cost));
        log(LogTag::Transactions, "DETAIL", &format!("Rent Costs: {:.9} SOL", fee_breakdown.rent_costs));
        log(LogTag::Transactions, "DETAIL", &format!("Trading Fees Total: {:.9} SOL ({:.2}%)", fee_breakdown.total_fees, fee_breakdown.fee_percentage));
        log(LogTag::Transactions, "DETAIL", &format!("Infrastructure Costs: {:.9} SOL (one-time setup)", fee_breakdown.rent_costs));
        log(LogTag::Transactions, "DETAIL", &format!("Compute Units: {} consumed / {} price = Priority: {}", 
            fee_breakdown.compute_units_consumed, 
            fee_breakdown.compute_unit_price,
            fee_breakdown.compute_unit_price.saturating_sub(fee_breakdown.compute_units_consumed)
        ));
        
        // Display swap analysis information if available
        if let Some(swap_analysis) = &transaction.swap_analysis {
            log(LogTag::Transactions, "DETAIL", &format!("Effective Price: {:.12}", swap_analysis.effective_price));
            log(LogTag::Transactions, "DETAIL", &format!("Slippage: {:.2}%", swap_analysis.slippage));
        }
        
        log(LogTag::Transactions, "DETAIL", "=== END FEE BREAKDOWN ===");
    }
    
    // Transaction type details
    match &transaction.transaction_type {
        TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, router } => {
            log(LogTag::Transactions, "DETAIL", &format!("Type: SOL to Token Swap"));
            log(LogTag::Transactions, "DETAIL", &format!("  Router: {}", router));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Mint: {}", token_mint));
            log(LogTag::Transactions, "DETAIL", &format!("  SOL Amount: {:.6}", sol_amount));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Amount: {:.2}", token_amount));
        }
        TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, router } => {
            log(LogTag::Transactions, "DETAIL", &format!("Type: Token to SOL Swap"));
            log(LogTag::Transactions, "DETAIL", &format!("  Router: {}", router));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Mint: {}", token_mint));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Amount: {:.2}", token_amount));
            log(LogTag::Transactions, "DETAIL", &format!("  SOL Amount: {:.6}", sol_amount));
        }
        _ => {
            log(LogTag::Transactions, "DETAIL", &format!("Type: {:?}", transaction.transaction_type));
        }
    }

    // Token transfers
    if !transaction.token_transfers.is_empty() {
        log(LogTag::Transactions, "DETAIL", "Token Transfers:");
        for transfer in &transaction.token_transfers {
            let from_display = if transfer.from.len() >= 8 { &transfer.from[..8] } else { &transfer.from };
            let to_display = if transfer.to.len() >= 8 { &transfer.to[..8] } else { &transfer.to };
            let mint_display = if transfer.mint.len() >= 8 { &transfer.mint[..8] } else { &transfer.mint };
            
            log(LogTag::Transactions, "DETAIL", &format!(
                "  {} -> {}: {:.6} ({})",
                from_display,
                to_display,
                transfer.amount,
                mint_display
            ));
        }
    }

    // Instructions
    if !transaction.instructions.is_empty() {
        log(LogTag::Transactions, "DETAIL", &format!("Instructions: {}", transaction.instructions.len()));
        for (i, instruction) in transaction.instructions.iter().enumerate() {
            log(LogTag::Transactions, "DETAIL", &format!(
                "  [{}] {} - {} - {} accounts",
                i,
                &instruction.program_id[..8],
                instruction.instruction_type,
                instruction.accounts.len()
            ));
        }
    }

    if let Some(error) = &transaction.error_message {
        log(LogTag::Transactions, "DETAIL", &format!("Error: {}", error));
    }

    log(LogTag::Transactions, "DETAIL", "=== END DETAILS ===");
}

/// Analyze a cache file and return the transaction
async fn analyze_cache_file(path: &Path) -> Result<Transaction, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let transaction: Transaction = serde_json::from_str(&content)?;
    Ok(transaction)
}

/// Statistics for analyzer testing
#[derive(Debug)]
struct AnalyzerTestStats {
    total_processed: usize,
    successful: usize,
    errors: usize,
    transaction_types: HashMap<String, usize>,
    total_processing_time: Duration,
    min_time: Duration,
    max_time: Duration,
}

impl AnalyzerTestStats {
    fn new() -> Self {
        Self {
            total_processed: 0,
            successful: 0,
            errors: 0,
            transaction_types: HashMap::new(),
            total_processing_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
        }
    }

    fn record_success(&mut self, transaction: &Transaction, processing_time: Duration) {
        self.total_processed += 1;
        self.successful += 1;
        self.total_processing_time += processing_time;
        
        if processing_time < self.min_time {
            self.min_time = processing_time;
        }
        if processing_time > self.max_time {
            self.max_time = processing_time;
        }

        let tx_type = format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Unknown").to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;
    }

    fn record_error(&mut self, _error: &str) {
        self.total_processed += 1;
        self.errors += 1;
    }

    fn display_results(&self, total_time: Duration) {
        log(LogTag::Transactions, "RESULTS", "=== ANALYZER TEST RESULTS ===");
        log(LogTag::Transactions, "RESULTS", &format!("Total Processed: {}", self.total_processed));
        log(LogTag::Transactions, "RESULTS", &format!("Successful: {}", self.successful));
        log(LogTag::Transactions, "RESULTS", &format!("Errors: {}", self.errors));
        log(LogTag::Transactions, "RESULTS", &format!("Success Rate: {:.1}%", 
            (self.successful as f64 / self.total_processed as f64) * 100.0));
        
        if self.successful > 0 {
            let avg_time = self.total_processing_time / self.successful as u32;
            log(LogTag::Transactions, "RESULTS", &format!("Avg Processing Time: {:.2}ms", avg_time.as_millis()));
            log(LogTag::Transactions, "RESULTS", &format!("Min Processing Time: {:.2}ms", self.min_time.as_millis()));
            log(LogTag::Transactions, "RESULTS", &format!("Max Processing Time: {:.2}ms", self.max_time.as_millis()));
        }

        log(LogTag::Transactions, "RESULTS", &format!("Total Test Time: {:.2}s", total_time.as_secs_f64()));
        
        log(LogTag::Transactions, "RESULTS", "Transaction Types:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "RESULTS", &format!("  {}: {}", tx_type, count));
        }
        
        log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
    }
}

/// Statistics for cache analysis
#[derive(Debug)]
struct CacheStats {
    total_files: usize,
    valid_files: usize,
    invalid_files: usize,
    transaction_types: HashMap<String, usize>,
    oldest_transaction: Option<DateTime<Utc>>,
    newest_transaction: Option<DateTime<Utc>>,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            total_files: 0,
            valid_files: 0,
            invalid_files: 0,
            transaction_types: HashMap::new(),
            oldest_transaction: None,
            newest_transaction: None,
        }
    }

    fn record_transaction(&mut self, transaction: &Transaction) {
        self.total_files += 1;
        self.valid_files += 1;

        let tx_type = format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Unknown").to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;

        if self.oldest_transaction.is_none() || transaction.timestamp < self.oldest_transaction.unwrap() {
            self.oldest_transaction = Some(transaction.timestamp);
        }
        if self.newest_transaction.is_none() || transaction.timestamp > self.newest_transaction.unwrap() {
            self.newest_transaction = Some(transaction.timestamp);
        }
    }

    fn record_error(&mut self) {
        self.total_files += 1;
        self.invalid_files += 1;
    }

    fn display_results(&self) {
        log(LogTag::Transactions, "CACHE", "=== CACHE ANALYSIS RESULTS ===");
        log(LogTag::Transactions, "CACHE", &format!("Total Files: {}", self.total_files));
        log(LogTag::Transactions, "CACHE", &format!("Valid Files: {}", self.valid_files));
        log(LogTag::Transactions, "CACHE", &format!("Invalid Files: {}", self.invalid_files));
        
        if let (Some(oldest), Some(newest)) = (self.oldest_transaction, self.newest_transaction) {
            log(LogTag::Transactions, "CACHE", &format!("Oldest Transaction: {}", oldest));
            log(LogTag::Transactions, "CACHE", &format!("Newest Transaction: {}", newest));
            
            let time_span = newest.signed_duration_since(oldest);
            log(LogTag::Transactions, "CACHE", &format!("Time Span: {} days", time_span.num_days()));
        }

        log(LogTag::Transactions, "CACHE", "Transaction Types in Cache:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "CACHE", &format!("  {}: {}", tx_type, count));
        }
        
        log(LogTag::Transactions, "CACHE", "=== END CACHE ANALYSIS ===");
    }
}

/// Execute real swap test with comprehensive transaction analysis
async fn test_real_swap(
    wallet_pubkey: Pubkey,
    swap_type: &str,
    token_mint: &str,
    token_symbol: &str,
    sol_amount: f64,
    slippage: f64,
    router: &str,
) {
    log(LogTag::Transactions, "SWAP_TEST", "=== REAL SWAP TEST STARTING ===");
    log(LogTag::Transactions, "SWAP_TEST", &format!(
        "📋 Test Configuration:\n  • Swap Type: {}\n  • Token: {} ({})\n  • SOL Amount: {:.6} SOL\n  • Slippage: {:.1}%\n  • Router: {}",
        swap_type, token_symbol, &token_mint[..8], sol_amount, slippage, router
    ));

    // Safety warning
    log(LogTag::Transactions, "WARNING", "⚠️ This test performs REAL blockchain transactions with REAL SOL!");
    log(LogTag::Transactions, "WARNING", "⚠️ Starting in 5 seconds... Press Ctrl+C to cancel!");
    
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Create transactions manager for monitoring
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Pre-flight checks
    if let Err(e) = perform_preflight_checks(wallet_pubkey, sol_amount, token_mint, router).await {
        log(LogTag::Transactions, "ERROR", &format!("Pre-flight check failed: {}", e));
        return;
    }

    log(LogTag::Transactions, "SUCCESS", "✅ All pre-flight checks passed");

    // Load token with updated information from tokens module
    let test_token = match load_token_with_updated_info(token_mint, token_symbol).await {
        Ok(token) => token,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load token info: {}", e));
            log(LogTag::Transactions, "INFO", "Creating basic token for testing...");
            create_basic_token(token_mint, token_symbol)
        }
    };

    match swap_type {
        "sol-to-token" => {
            execute_sol_to_token_test(&mut manager, &test_token, sol_amount, slippage, router).await;
        }
        "token-to-sol" => {
            execute_token_to_sol_test(&mut manager, &test_token, slippage, router).await;
        }
        "round-trip" => {
            execute_round_trip_test(&mut manager, &test_token, sol_amount, slippage, router).await;
        }
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown swap type: {}", swap_type));
        }
    }

    log(LogTag::Transactions, "SWAP_TEST", "=== REAL SWAP TEST COMPLETED ===");
}

/// Perform pre-flight safety checks before executing swaps
async fn perform_preflight_checks(
    wallet_pubkey: Pubkey,
    sol_amount: f64,
    token_mint: &str,
    router: &str,
) -> Result<(), String> {
    log(LogTag::Transactions, "PREFLIGHT", "🔍 Performing pre-flight checks...");
    
    let slippage = 1.0; // 1% slippage for testing

    // Check wallet SOL balance
    let rpc_client = get_rpc_client();
    let sol_balance = match rpc_client.get_sol_balance(&wallet_pubkey.to_string()).await {
        Ok(balance) => balance,
        Err(e) => return Err(format!("Failed to get wallet balance: {}", e)),
    };

    let minimum_required = sol_amount + 0.01; // Buffer for fees
    if sol_balance < minimum_required {
        return Err(format!(
            "Insufficient SOL balance: {:.6} SOL, required: {:.6} SOL",
            sol_balance, minimum_required
        ));
    }

    log(LogTag::Transactions, "PREFLIGHT", &format!(
        "✅ Wallet balance check: {:.6} SOL (required: {:.6} SOL)",
        sol_balance, minimum_required
    ));

    // Test quote availability
    let wallet_address = get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;
    let lamport_amount = sol_to_lamports(sol_amount);

    let quote_result = match router {
        "jupiter" => {
            get_jupiter_quote(
                "So11111111111111111111111111111111111111112", // SOL mint
                token_mint,
                lamport_amount,
                &wallet_address,
                slippage,
                "ExactIn",
                0.25,
                false,
            ).await
        }
        "gmgn" => {
            get_gmgn_quote(
                "So11111111111111111111111111111111111111112",
                token_mint,
                lamport_amount,
                &wallet_address,
                slippage,
                "ExactIn",
                0.25,
                false,
            ).await
        }
        _ => return Err(format!("Unknown router: {}", router)),
    };

    match quote_result {
        Ok(quote) => {
            log(LogTag::Transactions, "PREFLIGHT", &format!(
                "✅ {} quote test: {} SOL -> {} tokens",
                router, quote.quote.in_amount, quote.quote.out_amount
            ));
        }
        Err(e) => {
            return Err(format!("{} quote test failed: {}", router, e));
        }
    }

    log(LogTag::Transactions, "PREFLIGHT", "✅ All pre-flight checks completed successfully");
    Ok(())
}

/// Execute SOL to Token swap test
async fn execute_sol_to_token_test(
    manager: &mut TransactionsManager,
    token: &Token,
    sol_amount: f64,
    slippage: f64,
    router: &str,
) {
    log(LogTag::Transactions, "BUY_TEST", &format!(
        "🔵 Starting {} BUY test (SOL -> {})",
        router.to_uppercase(), token.symbol
    ));

    let start_time = Instant::now();
    
    let swap_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, sol_amount, slippage, true).await,
        "gmgn" => execute_gmgn_swap_test(token, sol_amount, slippage, true).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match swap_result {
        Ok(result) => {
            let execution_time = start_time.elapsed();
            log(LogTag::Transactions, "SUCCESS", &format!(
                "✅ {} BUY completed in {:.2}s!",
                router.to_uppercase(), execution_time.as_secs_f64()
            ));

            if let Some(signature) = &result.transaction_signature {
                log(LogTag::Transactions, "BUY_RESULT", &format!(
                    "• Signature: {}\n  • Input: {} SOL\n  • Output: {} tokens\n  • Price Impact: {}%\n  • Fee: {} lamports",
                    &signature[..12], result.input_amount, result.output_amount,
                    result.price_impact, result.fee_lamports
                ));

                // Wait for transaction confirmation and analyze
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "BUY").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ {} BUY failed: {}", router.to_uppercase(), e));
        }
    }
}

/// Execute Token to SOL swap test
async fn execute_token_to_sol_test(
    manager: &mut TransactionsManager,
    token: &Token,
    slippage: f64,
    router: &str,
) {
    log(LogTag::Transactions, "SELL_TEST", &format!(
        "🟠 Starting {} SELL test ({} -> SOL)",
        router.to_uppercase(), token.symbol
    ));

    // Get wallet address for balance check
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get wallet address: {}", e));
            return;
        }
    };

    // Check existing token balance
    let token_balance = match screenerbot::utils::get_token_balance(&wallet_address, &token.mint).await {
        Ok(balance) => balance,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get token balance: {}", e));
            return;
        }
    };

    if token_balance == 0 {
        log(LogTag::Transactions, "ERROR", &format!(
            "No {} balance found in wallet. Cannot execute token-to-sol test.",
            token.symbol
        ));
        return;
    }

    // Get token decimals for proper amount calculation
    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(LogTag::Transactions, "WARN", "Could not get token decimals from cache, using default 9");
            9
        }
    };

    let token_amount_raw = token_balance as f64 / 10_f64.powi(token_decimals as i32);
    
    log(LogTag::Transactions, "SELL_TEST", &format!(
        "🔍 Found token balance: {} raw tokens ({:.6} decimal-adjusted tokens)",
        token_balance, token_amount_raw
    ));

    // Use all available tokens for the sell test
    let tokens_to_sell = token_amount_raw;

    let start_time = Instant::now();
    
    let swap_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, tokens_to_sell, slippage, false).await,
        "gmgn" => execute_gmgn_swap_test(token, tokens_to_sell, slippage, false).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match swap_result {
        Ok(result) => {
            let execution_time = start_time.elapsed();
            log(LogTag::Transactions, "SUCCESS", &format!(
                "✅ {} SELL completed in {:.2}s!",
                router.to_uppercase(), execution_time.as_secs_f64()
            ));

            if let Some(signature) = &result.transaction_signature {
                log(LogTag::Transactions, "SELL_RESULT", &format!(
                    "• Signature: {}\n  • Input: {} tokens\n  • Output: {} SOL\n  • Price Impact: {}%\n  • Fee: {} lamports",
                    &signature[..12], result.input_amount, result.output_amount,
                    result.price_impact, result.fee_lamports
                ));

                // Wait for transaction confirmation and analyze
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "SELL").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ {} SELL failed: {}", router.to_uppercase(), e));
        }
    }
}

/// Execute round-trip swap test (SOL -> Token -> SOL)
async fn execute_round_trip_test(
    manager: &mut TransactionsManager,
    token: &Token,
    sol_amount: f64,
    slippage: f64,
    router: &str,
) {
    log(LogTag::Transactions, "ROUND_TRIP", &format!(
        "🔄 Starting {} ROUND-TRIP test (SOL -> {} -> SOL)",
        router.to_uppercase(), token.symbol
    ));

    let mut test_results = SwapTestResults::new();

    // Phase 1: SOL -> Token (BUY)
    log(LogTag::Transactions, "BUY_PHASE", &format!(
        "🔵 Phase 1: {} BUY (SOL -> {})", router.to_uppercase(), token.symbol
    ));

    let buy_start = Instant::now();
    let buy_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, sol_amount, slippage, true).await,
        "gmgn" => execute_gmgn_swap_test(token, sol_amount, slippage, true).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    let mut tokens_received = 0.0;
    let mut _buy_signature = String::new();

    match buy_result {
        Ok(result) => {
            let buy_time = buy_start.elapsed();
            test_results.buy_success = true;
            test_results.buy_execution_time = buy_time.as_secs_f64();
            
            if let Some(signature) = &result.transaction_signature {
                _buy_signature = signature.clone();
                test_results.buy_signature = Some(signature.clone());
                
                tokens_received = result.output_amount.parse::<f64>().unwrap_or(0.0);
                test_results.tokens_received = tokens_received;
                test_results.sol_spent = result.input_amount.parse::<f64>().unwrap_or(0.0);

                log(LogTag::Transactions, "BUY_SUCCESS", &format!(
                    "✅ BUY completed in {:.2}s: {} SOL -> {} tokens ({})",
                    buy_time.as_secs_f64(), result.input_amount, result.output_amount, &signature[..12]
                ));

                // Analyze buy transaction
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "BUY").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ BUY phase failed: {}", e));
            test_results.display_results();
            return;
        }
    }

    // Wait a bit between phases
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Phase 2: Token -> SOL (SELL)
    log(LogTag::Transactions, "SELL_PHASE", &format!(
        "🔴 Phase 2: {} SELL ({} -> SOL)", router.to_uppercase(), token.symbol
    ));

    if tokens_received <= 0.0 {
        log(LogTag::Transactions, "ERROR", "❌ No tokens received from buy phase, cannot proceed with sell");
        test_results.display_results();
        return;
    }

    let sell_start = Instant::now();
    let sell_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, tokens_received, slippage, false).await,
        "gmgn" => execute_gmgn_swap_test(token, tokens_received, slippage, false).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match sell_result {
        Ok(result) => {
            let sell_time = sell_start.elapsed();
            test_results.sell_success = true;
            test_results.sell_execution_time = sell_time.as_secs_f64();
            
            if let Some(signature) = &result.transaction_signature {
                test_results.sell_signature = Some(signature.clone());
                test_results.sol_received = result.output_amount.parse::<f64>().unwrap_or(0.0);

                log(LogTag::Transactions, "SELL_SUCCESS", &format!(
                    "✅ SELL completed in {:.2}s: {} tokens -> {} SOL ({})",
                    sell_time.as_secs_f64(), result.input_amount, result.output_amount, &signature[..12]
                ));

                // Analyze sell transaction
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "SELL").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ SELL phase failed: {}", e));
        }
    }

    // Display comprehensive results
    test_results.display_results();
}

/// Execute Jupiter swap test
async fn execute_jupiter_swap_test(
    token: &Token,
    amount: f64,
    slippage: f64,
    is_buy: bool, // true for SOL->Token, false for Token->SOL
) -> Result<JupiterSwapResult, SwapError> {
    let wallet_address = get_wallet_address()?;
    let sol_mint = "So11111111111111111111111111111111111111112";
    
    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(LogTag::Transactions, "WARN", "Could not get token decimals from cache, using default 9");
            9
        }
    };
    
    let (input_mint, output_mint, input_amount) = if is_buy {
        // SOL -> Token
        (sol_mint.to_string(), token.mint.clone(), sol_to_lamports(amount))
    } else {
        // Token -> SOL
        let token_amount = (amount * 10_f64.powi(token_decimals as i32)) as u64;
        (token.mint.clone(), sol_mint.to_string(), token_amount)
    };

    // Get quote first
    let quote = get_jupiter_quote(
        &input_mint,
        &output_mint,
        input_amount,
        &wallet_address,
        slippage,
        "ExactIn",
        0.25,
        false,
    ).await?;

    // Execute the swap
    execute_jupiter_swap(token, &input_mint, &output_mint, quote).await
}

/// Execute GMGN swap test
async fn execute_gmgn_swap_test(
    token: &Token,
    amount: f64,
    slippage: f64,
    is_buy: bool,
) -> Result<JupiterSwapResult, SwapError> {
    let wallet_address = get_wallet_address()?;
    let sol_mint = "So11111111111111111111111111111111111111112";
    
    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(LogTag::Transactions, "WARN", "Could not get token decimals from cache, using default 9");
            9
        }
    };
    
    let (input_mint, output_mint, input_amount) = if is_buy {
        (sol_mint.to_string(), token.mint.clone(), sol_to_lamports(amount))
    } else {
        let token_amount = (amount * 10_f64.powi(token_decimals as i32)) as u64;
        (token.mint.clone(), sol_mint.to_string(), token_amount)
    };

    // Get quote first
    let _quote = get_gmgn_quote(
        &input_mint,
        &output_mint,
        input_amount,
        &wallet_address,
        slippage,
        "ExactIn",
        0.25,
        false,
    ).await?;

    // Execute the swap (note: execute_gmgn_swap has different signature, adapt as needed)
    // For now, return a placeholder result
    Err(SwapError::ConfigError("GMGN swap execution not yet implemented in test".to_string()))
}

/// Analyze a specific swap transaction
async fn analyze_swap_transaction(
    manager: &mut TransactionsManager,
    signature: &str,
    swap_type: &str,
) {
    log(LogTag::Transactions, "ANALYSIS", &format!(
        "📊 Analyzing {} transaction: {}...", swap_type, &signature[..12]
    ));

    tokio::time::sleep(Duration::from_secs(2)).await; // Wait for RPC propagation

    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(LogTag::Transactions, "ANALYSIS_SUCCESS", &format!(
                "✅ Transaction analysis completed for {}", &signature[..12]
            ));
            
            // Display comprehensive transaction details
            display_detailed_transaction_info(&transaction);
            
            // Add to swap analysis if it's a swap
            if matches!(transaction.transaction_type, 
                       TransactionType::SwapSolToToken { .. } | 
                       TransactionType::SwapTokenToSol { .. } |
                       TransactionType::SwapTokenToToken { .. }) {
                log(LogTag::Transactions, "SWAP_DETECTED", "✅ Transaction confirmed as swap and analyzed");
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ANALYSIS_ERROR", &format!(
                "❌ Failed to analyze transaction {}: {}", &signature[..12], e
            ));
        }
    }
}

/// Test results tracking structure
struct SwapTestResults {
    buy_success: bool,
    sell_success: bool,
    buy_signature: Option<String>,
    sell_signature: Option<String>,
    buy_execution_time: f64,
    sell_execution_time: f64,
    sol_spent: f64,
    tokens_received: f64,
    sol_received: f64,
}

impl SwapTestResults {
    fn new() -> Self {
        Self {
            buy_success: false,
            sell_success: false,
            buy_signature: None,
            sell_signature: None,
            buy_execution_time: 0.0,
            sell_execution_time: 0.0,
            sol_spent: 0.0,
            tokens_received: 0.0,
            sol_received: 0.0,
        }
    }

    fn display_results(&self) {
        log(LogTag::Transactions, "RESULTS", "📊 === COMPLETE SWAP TEST RESULTS ===");
        
        log(LogTag::Transactions, "RESULTS", " 🔵 BUY PHASE:");
        if self.buy_success {
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Status: ✅ Success ({:.2}s)", self.buy_execution_time
            ));
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • SOL Spent: {:.6} SOL", self.sol_spent
            ));
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Tokens Received: {:.2} tokens", self.tokens_received
            ));
            if let Some(sig) = &self.buy_signature {
                log(LogTag::Transactions, "RESULTS", &format!("  • TX: {}...", &sig[..12]));
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Status: ❌ Failed");
        }

        log(LogTag::Transactions, "RESULTS", " 🔴 SELL PHASE:");
        if self.sell_success {
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Status: ✅ Success ({:.2}s)", self.sell_execution_time
            ));
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Tokens Sold: {:.2} tokens", self.tokens_received
            ));
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • SOL Received: {:.6} SOL", self.sol_received
            ));
            if let Some(sig) = &self.sell_signature {
                log(LogTag::Transactions, "RESULTS", &format!("  • TX: {}...", &sig[..12]));
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Status: ❌ Failed");
        }

        log(LogTag::Transactions, "RESULTS", " 💰 NET RESULT:");
        if self.buy_success && self.sell_success {
            let net_sol = self.sol_received - self.sol_spent;
            let success_indicator = if net_sol >= -0.001 { "✅ Good" } else { "⚠️ High Cost" };
            
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Net SOL Change: {:.6} SOL", net_sol
            ));
            log(LogTag::Transactions, "RESULTS", &format!(
                "  • Success: {}", success_indicator
            ));
            
            if self.tokens_received > 0.0 {
                let effective_price = self.sol_spent / self.tokens_received;
                log(LogTag::Transactions, "RESULTS", &format!(
                    "  • Effective Price: {:.12} SOL per token", effective_price
                ));
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Net SOL Change: N/A (incomplete test)");
        }

        log(LogTag::Transactions, "RESULTS", " 📋 SIGNATURES:");
        if let Some(buy_sig) = &self.buy_signature {
            log(LogTag::Transactions, "RESULTS", &format!("  • Buy TX: {}", buy_sig));
        }
        if let Some(sell_sig) = &self.sell_signature {
            log(LogTag::Transactions, "RESULTS", &format!("  • Sell TX: {}", sell_sig));
        }

        log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
    }
}

/// Statistics for benchmark testing
#[derive(Debug)]
struct BenchmarkStats {
    successful: usize,
    errors: usize,
    total_processing_time: Duration,
    processing_times: Vec<Duration>,
    transaction_types: HashMap<String, usize>,
}

impl BenchmarkStats {
    fn new() -> Self {
        Self {
            successful: 0,
            errors: 0,
            total_processing_time: Duration::ZERO,
            processing_times: Vec::new(),
            transaction_types: HashMap::new(),
        }
    }

    fn record_transaction(&mut self, transaction: &Transaction, processing_time: Duration) {
        self.successful += 1;
        self.total_processing_time += processing_time;
        self.processing_times.push(processing_time);

        let tx_type = format!("{:?}", transaction.transaction_type).split('{').next().unwrap_or("Unknown").to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;
    }

    fn record_error(&mut self) {
        self.errors += 1;
    }

    fn display_results(&self, total_time: Duration, total_transactions: usize) {
        log(LogTag::Transactions, "BENCHMARK", "=== BENCHMARK RESULTS ===");
        log(LogTag::Transactions, "BENCHMARK", &format!("Total Transactions: {}", total_transactions));
        log(LogTag::Transactions, "BENCHMARK", &format!("Successful: {}", self.successful));
        log(LogTag::Transactions, "BENCHMARK", &format!("Errors: {}", self.errors));
        log(LogTag::Transactions, "BENCHMARK", &format!("Success Rate: {:.1}%", 
            (self.successful as f64 / total_transactions as f64) * 100.0));
        
        if !self.processing_times.is_empty() {
            let avg_time = self.total_processing_time / self.processing_times.len() as u32;
            let min_time = self.processing_times.iter().min().unwrap();
            let max_time = self.processing_times.iter().max().unwrap();
            
            // Calculate percentiles
            let mut sorted_times = self.processing_times.clone();
            sorted_times.sort();
            let p50 = sorted_times[sorted_times.len() / 2];
            let p95 = sorted_times[(sorted_times.len() * 95) / 100];

            log(LogTag::Transactions, "BENCHMARK", &format!("Avg Processing Time: {:.2}ms", avg_time.as_millis()));
            log(LogTag::Transactions, "BENCHMARK", &format!("Min Processing Time: {:.2}ms", min_time.as_millis()));
            log(LogTag::Transactions, "BENCHMARK", &format!("Max Processing Time: {:.2}ms", max_time.as_millis()));
            log(LogTag::Transactions, "BENCHMARK", &format!("P50 Processing Time: {:.2}ms", p50.as_millis()));
            log(LogTag::Transactions, "BENCHMARK", &format!("P95 Processing Time: {:.2}ms", p95.as_millis()));
        }

        log(LogTag::Transactions, "BENCHMARK", &format!("Total Benchmark Time: {:.2}s", total_time.as_secs_f64()));
        
        if total_time.as_secs() > 0 {
            let throughput = self.successful as f64 / total_time.as_secs_f64();
            log(LogTag::Transactions, "BENCHMARK", &format!("Throughput: {:.2} tx/sec", throughput));
        }
        
        log(LogTag::Transactions, "BENCHMARK", "Transaction Types:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "BENCHMARK", &format!("  {}: {}", tx_type, count));
        }
        
        log(LogTag::Transactions, "BENCHMARK", "=== END BENCHMARK ===");
    }
}

/// Test real position management with transaction verification (like main bot)
async fn test_real_position_management(
    wallet_pubkey: Pubkey,
    token_mint: &str,
    token_symbol: &str,
    sol_amount: f64,
) {
    log(LogTag::Transactions, "POSITION_TEST", "=== REAL POSITION MANAGEMENT TEST ===");
    log(LogTag::Transactions, "POSITION_TEST", &format!(
        "📋 Test Configuration:\n  • Token: {} ({})\n  • SOL Amount: {:.6} SOL\n  • This test mimics main bot position management",
        token_symbol, &token_mint[..8], sol_amount
    ));

    // Safety warning
    log(LogTag::Transactions, "WARNING", "⚠️ This test performs REAL blockchain transactions with REAL SOL!");
    log(LogTag::Transactions, "WARNING", "⚠️ This test will open and close a position like the main bot!");
    log(LogTag::Transactions, "WARNING", "⚠️ Starting in 10 seconds... Press Ctrl+C to cancel!");
    
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Initialize token system and price service
    log(LogTag::Transactions, "POSITION_TEST", "🔧 Initializing token systems...");
    
    // Initialize token database
    let _token_database = match screenerbot::tokens::TokenDatabase::new() {
        Ok(db) => {
            log(LogTag::Transactions, "POSITION_TEST", "✅ Token database initialized");
            db
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to initialize token database: {}", e));
            return;
        }
    };

    // Initialize price service
    if let Err(e) = screenerbot::tokens::initialize_price_service().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize price service: {}", e));
        return;
    }
    log(LogTag::Transactions, "POSITION_TEST", "✅ Price service initialized");

    // Initialize DexScreener API
    if let Err(e) = screenerbot::tokens::init_dexscreener_api().await {
        log(LogTag::Transactions, "WARN", &format!("Failed to initialize DexScreener API: {}", e));
        // Continue anyway as this is not critical for position testing
    } else {
        log(LogTag::Transactions, "POSITION_TEST", "✅ DexScreener API initialized");
    }

    // Initialize global transaction manager for monitoring
    if let Err(e) = initialize_global_transaction_manager(wallet_pubkey).await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize transaction manager: {}", e));
        return;
    }

    // Start lightweight transaction monitoring for the test
    log(LogTag::Transactions, "POSITION_TEST", "🔄 Starting transaction monitoring for position test...");
    let shutdown_monitor = Arc::new(tokio::sync::Notify::new());
    let monitor_handle = {
        let shutdown_clone = shutdown_monitor.clone();
        tokio::spawn(async move {
            // Run monitoring for 5 minutes max (longer than position test)
            tokio::select! {
                _ = shutdown_clone.notified() => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring stopped");
                }
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring timeout (5 minutes)");
                }
                _ = start_lightweight_transaction_monitoring(wallet_pubkey) => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring completed");
                }
            }
        })
    };

    // Load token with updated information from tokens module
    let test_token = match load_token_with_updated_info(token_mint, token_symbol).await {
        Ok(token) => {
            log(LogTag::Transactions, "POSITION_TEST", &format!(
                "✅ Loaded token: {} ({}) with updated info - Price: {} SOL, Liquidity: {} USD",
                token.symbol,
                &token.mint[..8],
                token.price_dexscreener_sol.map(|p| format!("{:.12}", p)).unwrap_or("N/A".to_string()),
                token.liquidity.as_ref().and_then(|l| l.usd).map(|l| format!("{:.0}", l)).unwrap_or("N/A".to_string())
            ));
            token
        },
        Err(e) => {
            log(LogTag::Transactions, "WARNING", &format!("Failed to load token info: {}", e));
            log(LogTag::Transactions, "INFO", "Creating basic token for testing...");
            create_basic_token(token_mint, token_symbol)
        }
    };

    // Get current price for the test
    let current_price = sol_amount / 1000.0; // Simulate price for testing

    log(LogTag::Transactions, "POSITION_TEST", "🟢 STEP 1: Opening position with transaction verification...");

    // Open position using the main bot logic
    positions::open_position(&test_token, current_price, -5.0).await;

    // Wait a moment for position to be created
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check if position was created
    let open_positions = positions::get_open_positions();
    let test_position = open_positions.iter().find(|p| p.mint == token_mint);

    if let Some(position) = test_position {
        log(LogTag::Transactions, "POSITION_TEST", &format!(
            "✅ Position opened successfully: {} | Entry: {:.9} SOL | TX: {}",
            position.symbol,
            position.entry_price,
            position.entry_transaction_signature.as_ref().unwrap_or(&"None".to_string())
        ));

        log(LogTag::Transactions, "POSITION_TEST", "⏳ Waiting 10 seconds before closing position...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        log(LogTag::Transactions, "POSITION_TEST", "🔴 STEP 2: Closing position with transaction verification...");

        // Get the position again in case it was updated
        let open_positions = positions::get_open_positions();
        if let Some(mut position) = open_positions.iter().find(|p| p.mint == token_mint).cloned() {
            let exit_price = current_price * 1.02; // Simulate 2% profit
            let exit_time = chrono::Utc::now();

            let success = positions::close_position(&mut position, &test_token, exit_price, exit_time).await;

            if success {
                log(LogTag::Transactions, "POSITION_TEST", &format!(
                    "✅ Position closed successfully: {} | Exit: {:.9} SOL | TX: {}",
                    position.symbol,
                    exit_price,
                    position.exit_transaction_signature.as_ref().unwrap_or(&"None".to_string())
                ));
                
                // Generate comprehensive test report
                generate_comprehensive_position_test_report(
                    &test_token,
                    token_symbol,
                    token_mint,
                    sol_amount,
                    wallet_pubkey,
                    &position,
                ).await;
            } else {
                log(LogTag::Transactions, "POSITION_TEST", &format!(
                    "❌ Failed to close position for {}", position.symbol
                ));
                
                // Generate report even if closing failed
                generate_comprehensive_position_test_report(
                    &test_token,
                    token_symbol,
                    token_mint,
                    sol_amount,
                    wallet_pubkey,
                    &position,
                ).await;
            }
        } else {
            log(LogTag::Transactions, "POSITION_TEST", "❌ Position not found for closing");
        }
    } else {
        log(LogTag::Transactions, "POSITION_TEST", "❌ Position was not created");
    }

    // Stop transaction monitoring
    shutdown_monitor.notify_one();
    
    // Give monitor a moment to stop
    tokio::time::sleep(Duration::from_secs(2)).await;

    log(LogTag::Transactions, "POSITION_TEST", "=== REAL POSITION MANAGEMENT TEST COMPLETED ===");
}

/// Load token with updated information from tokens module
async fn load_token_with_updated_info(token_mint: &str, token_symbol: &str) -> Result<Token, String> {
    log(LogTag::Transactions, "TOKEN_LOAD", &format!("Loading token {} ({}) with updated info...", token_symbol, &token_mint[..8]));

    // Initialize tokens system if not already done
    if let Err(e) = screenerbot::tokens::initialize_tokens_system().await {
        log(LogTag::Transactions, "WARNING", &format!("Failed to initialize tokens system: {}", e));
    }

    // Try to get token from database first
    if let Some(mut token) = screenerbot::tokens::get_token_from_db(token_mint).await {
        log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Found token in database: {}", token.symbol));
        
        // Update with current price if available
        if let Some(current_price) = screenerbot::tokens::get_current_token_price(token_mint).await {
            token.price_dexscreener_sol = Some(current_price);
            log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Updated current price: {:.12} SOL", current_price));
        }
        
        // Get token decimals and ensure they are set
        if token.price_dexscreener_sol.is_none() {
            log(LogTag::Transactions, "TOKEN_LOAD", "⚠️ No price available, fetching decimals for safety...");
        }
        
        // Ensure decimals are available
        if let Some(decimals) = screenerbot::tokens::get_token_decimals(token_mint).await {
            log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Token decimals: {}", decimals));
        }
        
        return Ok(token);
    }

    // If not in database, try to fetch from discovery system
    log(LogTag::Transactions, "TOKEN_LOAD", "Token not in database, attempting discovery...");
    
    // Run discovery to fetch the token
    if let Err(e) = screenerbot::tokens::discover_tokens_once().await {
        log(LogTag::Transactions, "WARNING", &format!("Discovery failed: {}", e));
    }
    
    // Try again after discovery
    if let Some(token) = screenerbot::tokens::get_token_from_db(token_mint).await {
        log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Found token after discovery: {}", token.symbol));
        return Ok(token);
    }

    // If still not found, create a basic token but try to get current info
    log(LogTag::Transactions, "TOKEN_LOAD", "Token not found in discovery, creating with current data...");
    let mut basic_token = create_basic_token(token_mint, token_symbol);
    
    // Try to get current price
    if let Some(current_price) = screenerbot::tokens::get_current_token_price(token_mint).await {
        basic_token.price_dexscreener_sol = Some(current_price);
        log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Got current price: {:.12} SOL", current_price));
    }
    
    Ok(basic_token)
}

/// Create a basic token structure for testing
fn create_basic_token(token_mint: &str, token_symbol: &str) -> Token {
    Token {
        mint: token_mint.to_string(),
        symbol: token_symbol.to_string(),
        name: token_symbol.to_string(),
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
    }
}

async fn generate_comprehensive_position_test_report(
    original_token: &Token,
    token_symbol: &str,
    token_mint: &str,
    sol_amount: f64,
    wallet_pubkey: Pubkey,
    position: &positions::Position,
) {
    println!("\n{}", "=".repeat(80));
    println!("🔍 COMPREHENSIVE POSITION TEST REPORT");
    println!("{}", "=".repeat(80));

    // Get current wallet balance
    let rpc_client = get_rpc_client();
    let current_sol_balance = match rpc_client.get_sol_balance(&wallet_pubkey.to_string()).await {
        Ok(balance) => balance,
        Err(_) => 0.0,
    };

    // Position Overview
    println!("\n📊 POSITION OVERVIEW:");
    println!("Token Address: {}", token_mint);
    println!("Token Symbol: {}", token_symbol);
    println!("Token Name: {}", original_token.name);
    println!("Test SOL Amount: {:.6} SOL", sol_amount);
    
    // Price Analysis
    println!("\n💰 PRICE ANALYSIS:");
    println!("Entry Price: {:.9} SOL", position.entry_price);
    if let Some(exit_price) = position.exit_price {
        println!("Exit Price: {:.9} SOL", exit_price);
        
        let price_change = ((exit_price - position.entry_price) / position.entry_price) * 100.0;
        let price_change_emoji = if price_change > 0.0 { "📈" } else { "📉" };
        println!("{} Price Change: {:.2}%", price_change_emoji, price_change);
    }

    // Token Amount Analysis
    println!("\n🪙 TOKEN AMOUNT ANALYSIS:");
    if let Some(token_amount) = position.token_amount {
        // Token amount is stored in raw units, need to convert to human readable
        let human_readable = token_amount as f64 / 1_000_000.0; // Assuming 6 decimals for most tokens
        println!("Token Amount (raw units): {}", token_amount);
        println!("Token Amount (human readable): {:.6}", human_readable);
    }

    // SOL Balance Analysis
    println!("\n💎 SOL BALANCE ANALYSIS:");
    println!("Entry SOL Size: {:.9} SOL", position.entry_size_sol);
    println!("Total Position SOL: {:.9} SOL", position.total_size_sol);
    println!("Current Wallet Balance: {:.9} SOL", current_sol_balance);
    
    if let Some(sol_received) = position.sol_received {
        println!("SOL Received on Exit: {:.9} SOL", sol_received);
        let sol_change = sol_received - position.entry_size_sol;
        let sol_change_emoji = if sol_change > 0.0 { "📈" } else { "📉" };
        println!("{} SOL P&L: {:.9} SOL", sol_change_emoji, sol_change);
        
        if position.entry_size_sol > 0.0 {
            let sol_change_percent = (sol_change / position.entry_size_sol) * 100.0;
            println!("SOL P&L Percentage: {:.2}%", sol_change_percent);
        }
    }

    // Price Tracking
    println!("\n� PRICE TRACKING:");
    println!("Highest Price: {:.9} SOL", position.price_highest);
    println!("Lowest Price: {:.9} SOL", position.price_lowest);
    
    let highest_gain = ((position.price_highest - position.entry_price) / position.entry_price) * 100.0;
    let lowest_loss = ((position.price_lowest - position.entry_price) / position.entry_price) * 100.0;
    println!("Max Gain Potential: {:.2}%", highest_gain);
    println!("Max Loss Experienced: {:.2}%", lowest_loss);

    // Transaction Analysis
    println!("\n📋 TRANSACTION ANALYSIS:");
    if let Some(entry_sig) = &position.entry_transaction_signature {
        println!("Entry Transaction: https://solscan.io/tx/{}", entry_sig);
    }
    if let Some(exit_sig) = &position.exit_transaction_signature {
        println!("Exit Transaction: https://solscan.io/tx/{}", exit_sig);
    }

    // Verification Status
    println!("\n✅ VERIFICATION STATUS:");
    println!("Entry Transaction Verified: {}", if position.transaction_entry_verified { "✅ Yes" } else { "❌ No" });
    
    // Position Timing
    println!("\n⏱️ TIMING ANALYSIS:");
    println!("Position Opened: {}", position.entry_time.format("%Y-%m-%d %H:%M:%S UTC"));
    if let Some(exit_time) = position.exit_time {
        println!("Position Closed: {}", exit_time.format("%Y-%m-%d %H:%M:%S UTC"));
        
        if let Ok(duration) = exit_time.signed_duration_since(position.entry_time).to_std() {
            println!("Position Duration: {:.2} seconds", duration.as_secs_f64());
        }
    }

    // Performance Summary
    println!("\n🎯 PERFORMANCE SUMMARY:");
    if let (Some(exit_price), Some(sol_received)) = (position.exit_price, position.sol_received) {
        let price_pnl = ((exit_price - position.entry_price) / position.entry_price) * 100.0;
        let sol_pnl = sol_received - position.entry_size_sol;
        let sol_pnl_percent = (sol_pnl / position.entry_size_sol) * 100.0;
        
        let performance_emoji = if sol_pnl > 0.0 { "🟢" } else { "🔴" };
        println!("{} Price P&L: {:.2}%", performance_emoji, price_pnl);
        println!("{} SOL P&L: {:.9} SOL ({:.2}%)", performance_emoji, sol_pnl, sol_pnl_percent);
        
        // Risk assessment
        let risk_level = if sol_pnl_percent.abs() > 10.0 {
            "HIGH"
        } else if sol_pnl_percent.abs() > 5.0 {
            "MEDIUM"
        } else {
            "LOW"
        };
        println!("Risk Level: {}", risk_level);
    }

    // Position Status
    println!("\n📈 POSITION STATUS:");
    if position.exit_price.is_some() {
        println!("✅ Position closed successfully");
        println!("Status: COMPLETED");
    } else {
        println!("⚠️ Position still open or closure failed");
        println!("Status: OPEN/FAILED");
    }

    // Smart Targeting Analysis
    println!("\n🎯 SMART TARGETING:");
    if let Some(min_target) = position.profit_target_min {
        println!("Minimum Profit Target: {:.2}%", min_target);
    }
    if let Some(max_target) = position.profit_target_max {
        println!("Maximum Profit Target: {:.2}%", max_target);
    }
    if let Some(liquidity_tier) = &position.liquidity_tier {
        println!("Liquidity Tier: {}", liquidity_tier);
    }

    // ATA Information
    println!("\n🏦 ATA (Associated Token Account) STATUS:");
    println!("Token ATA Address: {}", get_associated_token_address(
        &wallet_pubkey,
        &Pubkey::from_str(token_mint).unwrap_or_default()
    ));

    // Additional Insights
    println!("\n💡 INSIGHTS & ANALYSIS:");
    
    // Transaction verification insights
    if position.transaction_entry_verified {
        println!("✅ Transaction verification system worked correctly");
    } else {
        println!("⚠️ Transaction verification may have issues");
    }
    
    // Effective pricing insights
    if let Some(effective_entry) = position.effective_entry_price {
        println!("Effective Entry Price: {:.9} SOL", effective_entry);
        let slippage = ((effective_entry - position.entry_price) / position.entry_price) * 100.0;
        if slippage.abs() < 1.0 {
            println!("✅ Low slippage detected ({:.2}%)", slippage);
        } else {
            println!("⚠️ High slippage detected ({:.2}%)", slippage);
        }
    }
    
    if let Some(effective_exit) = position.effective_exit_price {
        println!("Effective Exit Price: {:.9} SOL", effective_exit);
    }
    
    // Position size analysis
    if position.total_size_sol > position.entry_size_sol {
        println!("📊 Position was accumulated (total > entry size)");
    } else {
        println!("📊 Single entry position");
    }

    // Market timing insights
    if let (Some(exit_price), Some(exit_time)) = (position.exit_price, position.exit_time) {
        let hold_duration = exit_time.signed_duration_since(position.entry_time);
        if let Ok(duration) = hold_duration.to_std() {
            if duration.as_secs() < 60 {
                println!("⚡ Very short position (< 1 minute) - High frequency strategy");
            } else if duration.as_secs() < 300 {
                println!("🕐 Short position (< 5 minutes) - Quick scalp trade");
            } else {
                println!("🕰️ Extended position (> 5 minutes) - Swing trade");
            }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("📊 END OF COMPREHENSIVE POSITION TEST REPORT");
    println!("{}", "=".repeat(80));
}

/// Lightweight transaction monitoring for position tests
async fn start_lightweight_transaction_monitoring(wallet_pubkey: Pubkey) {
    log(LogTag::Transactions, "MONITOR", "Starting lightweight transaction monitoring...");
    
    // Create a monitoring manager
    let mut manager = match screenerbot::transactions::TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create monitoring manager: {}", e));
            return;
        }
    };
    
    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize monitoring: {}", e));
        return;
    }

    log(LogTag::Transactions, "MONITOR", &format!(
        "Monitoring initialized with {} known transactions", 
        manager.known_signatures.len()
    ));

    // Monitor frequently for position tests (every 2 seconds)
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    
    loop {
        interval.tick().await;
        
        // Check for new transactions
        match manager.check_new_transactions().await {
            Ok(new_signatures) => {
                if !new_signatures.is_empty() {
                    log(LogTag::Transactions, "MONITOR", &format!(
                        "Found {} new transactions, processing...", 
                        new_signatures.len()
                    ));
                    
                    // Process each new transaction
                    for signature in new_signatures {
                        if let Err(e) = manager.process_transaction(&signature).await {
                            log(LogTag::Transactions, "WARN", &format!(
                                "Failed to process transaction {}: {}", 
                                &signature[..8], e
                            ));
                        } else {
                            log(LogTag::Transactions, "SUCCESS", &format!(
                                "Successfully processed transaction {}", 
                                &signature[..8]
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                log(LogTag::Transactions, "WARN", &format!("Monitoring cycle failed: {}", e));
            }
        }
        
        // Check priority transactions
        if let Err(e) = manager.check_priority_transactions().await {
            log(LogTag::Transactions, "WARN", &format!("Priority check failed: {}", e));
        }
    }
}
