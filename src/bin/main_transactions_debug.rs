/// Transaction Manager & Analyzer Debug Tool
///
/// Comprehensive debugging and testing tool for the transactions management system.
/// This tool provides detailed analysis, monitoring, and debugging capabilities
/// for transaction processing, caching, and analysis.
///
/// Features:
/// - Monitor wallet transactions in real-time
/// - Analyze specific transactions by signature
/// - Test transaction type detection
/// - Debug transaction caching system
/// - Validate transaction analysis
/// - Performance benchmarking
/// - Cache management and stats
///
/// Usage Examples:
/// - Monitor wallet transactions: cargo run --bin main_transactions_debug -- --monitor
/// - Analyze specific transaction: cargo run --bin main_transactions_debug -- --signature <SIG>
/// - Test analyzer on recent transactions: cargo run --bin main_transactions_debug -- --test-analyzer --count 10
/// - Debug cache system: cargo run --bin main_transactions_debug -- --debug-cache
/// - Recalculate analysis: cargo run --bin main_transactions_debug -- --recalculate-cache
/// - Update and re-analyze cache: cargo run --bin main_transactions_debug -- --update-cache --count 50 (preserves raw data)
/// - Analyze all swaps with PnL: cargo run --bin main_transactions_debug -- --analyze-swaps
/// - Performance test: cargo run --bin main_transactions_debug -- --benchmark --count 100

use screenerbot::transactions_manager::{
    TransactionsManager, Transaction, TransactionType, TransactionDirection,
    get_transaction
};
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::global::{
    set_cmd_args, get_transactions_cache_dir
};
use screenerbot::rpc::get_rpc_client;
use screenerbot::utils::get_wallet_address;
use screenerbot::tokens::types::PriceSourceType;

use clap::{Arg, Command};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
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
            Arg::new("update-cache")
                .long("update-cache")
                .help("Re-analyze and update all cached transactions with new analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch-all")
                .long("fetch-all")
                .help("Fetch and analyze ALL wallet transactions from blockchain (not cached)")
                .action(clap::ArgAction::SetTrue)
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

    // Execute based on command line arguments
    if matches.get_flag("monitor") {
        let duration: u64 = matches.get_one::<String>("duration")
            .unwrap()
            .parse()
            .unwrap_or(60);
        monitor_transactions(wallet_pubkey, duration).await;
    } else if let Some(signature) = matches.get_one::<String>("signature") {
        analyze_specific_transaction(signature).await;
    } else if matches.get_flag("test-analyzer") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(10);
        test_transaction_analyzer(wallet_pubkey, count).await;
    } else if matches.get_flag("debug-cache") {
        debug_cache_system().await;
    } else if matches.get_flag("recalculate-cache") {
        recalculate_transaction_cache().await;
    } else if matches.get_flag("benchmark") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        run_benchmark_tests(wallet_pubkey, count).await;
    } else if matches.get_flag("analyze-swaps") {
        analyze_all_swaps(wallet_pubkey).await;
    } else if matches.get_flag("update-cache") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        update_transaction_cache(wallet_pubkey, count).await;
    } else if matches.get_flag("fetch-all") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(1000);
        fetch_all_wallet_transactions(wallet_pubkey, count).await;
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
fn display_detailed_swap_statistics(swaps: &[screenerbot::transactions_manager::SwapPnLInfo]) {
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
async fn analyze_specific_transaction(signature: &str) {
    log(LogTag::Transactions, "INFO", &format!("Analyzing transaction: {}", signature));

    // First check if it's already cached
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

    // Load wallet and create manager
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet: {}", e));
            return;
        }
    };

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };

    // Process the transaction with comprehensive analysis
    match manager.process_transaction(signature).await {
        Ok(mut transaction) => {
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

            for transaction in &transactions {
                match &transaction.transaction_type {
                    TransactionType::SwapSolToToken { .. } |
                    TransactionType::SwapTokenToSol { .. } |
                    TransactionType::SwapTokenToToken { .. } => swap_count += 1,
                    TransactionType::SolTransfer { .. } |
                    TransactionType::TokenTransfer { .. } => transfer_count += 1,
                    TransactionType::Spam => spam_count += 1,
                    TransactionType::Unknown => unknown_count += 1,
                }
            }

            log(LogTag::Transactions, "ANALYSIS", "=== COMPREHENSIVE WALLET ANALYSIS ===");
            log(LogTag::Transactions, "ANALYSIS", &format!("Total Transactions: {}", transactions.len()));
            log(LogTag::Transactions, "ANALYSIS", &format!("Swap Transactions: {}", swap_count));
            log(LogTag::Transactions, "ANALYSIS", &format!("Transfer Transactions: {}", transfer_count));
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
        log(LogTag::Transactions, "DETAIL", &format!("Total Fees: {:.9} SOL ({:.2}%)", fee_breakdown.total_fees, fee_breakdown.fee_percentage));
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
