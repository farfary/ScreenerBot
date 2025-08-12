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
/// - Clear transaction cache: cargo run --bin main_transactions_debug -- --clear-cache
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
                .short('m')
                .long("monitor")
                .help("Monitor wallet transactions in real-time")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("signature")
                .short('s')
                .long("signature")
                .value_name("SIGNATURE")
                .help("Analyze specific transaction by signature")
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
                .help("Debug transaction cache system")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("clear-cache")
                .long("clear-cache")
                .help("Clear all transaction cache files")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Run performance benchmark tests")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .value_name("COUNT")
                .help("Number of transactions to process (for test-analyzer, benchmark)")
                .default_value("10")
        )
        .arg(
            Arg::new("duration")
                .short('d')
                .long("duration")
                .value_name("SECONDS")
                .help("Duration to monitor (for --monitor)")
                .default_value("60")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Set command args for debug flags
    let mut args = vec!["main_transactions_debug".to_string()];
    if matches.get_flag("verbose") {
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
    } else if matches.get_flag("clear-cache") {
        clear_transaction_cache().await;
    } else if matches.get_flag("benchmark") {
        let count: usize = matches.get_one::<String>("count")
            .unwrap()
            .parse()
            .unwrap_or(100);
        run_benchmark_tests(wallet_pubkey, count).await;
    } else {
        log(LogTag::System, "ERROR", "No command specified. Use --help for usage information.");
        std::process::exit(1);
    }

    log(LogTag::System, "INFO", "Transaction Manager & Analyzer Debug Tool completed");
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

    let mut manager = TransactionsManager::new(wallet_pubkey);
    
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
            display_detailed_transaction_info(&transaction);
            return;
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

    let mut manager = TransactionsManager::new(wallet_pubkey);

    // Process the transaction
    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(LogTag::Transactions, "SUCCESS", "Transaction analyzed successfully");
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

    let mut manager = TransactionsManager::new(wallet_pubkey);

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

/// Clear all transaction cache files
async fn clear_transaction_cache() {
    log(LogTag::Transactions, "INFO", "Clearing transaction cache");

    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        log(LogTag::Transactions, "INFO", "Cache directory does not exist");
        return;
    }

    match fs::read_dir(&cache_dir) {
        Ok(entries) => {
            let mut deleted_count = 0;
            let mut error_count = 0;

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        match fs::remove_file(&path) {
                            Ok(_) => {
                                deleted_count += 1;
                                log(LogTag::Transactions, "DEBUG", &format!(
                                    "Deleted: {}", path.file_name().unwrap().to_string_lossy()
                                ));
                            }
                            Err(e) => {
                                error_count += 1;
                                log(LogTag::Transactions, "ERROR", &format!(
                                    "Failed to delete {}: {}", path.display(), e
                                ));
                            }
                        }
                    }
                }
            }

            log(LogTag::Transactions, "SUCCESS", &format!(
                "Cache cleared: {} files deleted, {} errors", deleted_count, error_count
            ));
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to read cache directory: {}", e));
        }
    }
}

/// Run performance benchmark tests
async fn run_benchmark_tests(wallet_pubkey: Pubkey, count: usize) {
    log(LogTag::Transactions, "INFO", &format!(
        "Running performance benchmark with {} transactions", count
    ));

    let mut manager = TransactionsManager::new(wallet_pubkey);

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
    log(LogTag::Transactions, "DETAIL", &format!("Timestamp: {}", transaction.timestamp));
    log(LogTag::Transactions, "DETAIL", &format!("Success: {}", transaction.success));
    log(LogTag::Transactions, "DETAIL", &format!("Finalized: {}", transaction.finalized));
    log(LogTag::Transactions, "DETAIL", &format!("Direction: {:?}", transaction.direction));
    log(LogTag::Transactions, "DETAIL", &format!("Fee (SOL): {:.6}", transaction.fee_sol));
    log(LogTag::Transactions, "DETAIL", &format!("SOL Balance Change: {:.6}", transaction.sol_balance_change));
    
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
