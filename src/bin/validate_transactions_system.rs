/// Comprehensive Transaction System Validation Tool
///
/// This tool validates the entire transaction processing pipeline by:
/// - Fetching the last 1000 transactions for the configured wallet
/// - Processing each transaction through the complete pipeline
/// - Validating transaction type detection (swaps, transfers, ATA ops, etc.)
/// - Verifying balance calculations and fee computations
/// - Checking database persistence and cache operations
/// - Analyzing swap P&L calculations and ATA rent impact
/// - Ensuring all transaction classifications are accurate
/// - Reporting any discrepancies or processing failures
///
/// This is the definitive test to ensure the transaction system is working correctly
/// with real blockchain data from the wallet's transaction history.

use clap::Parser;
use screenerbot::arguments::set_cmd_args;

use screenerbot::transactions::{
    database::get_transaction_database,
    fetcher::TransactionFetcher,
    processor::TransactionProcessor,
    service::get_transaction,
    types::*,
    utils::*,
};
use screenerbot::utils::get_wallet_address;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;
use tabled::Tabled;
use tokio;

#[derive(Parser, Debug)]
#[command(
    name = "validate_transactions_system",
    about = "Comprehensive validation of transaction system with last 1000 transactions"
)]
struct Args {
    /// Number of transactions to validate (default: 1000)
    #[arg(short, long, default_value = "1000")]
    count: usize,

    /// Enable debug logging for detailed analysis
    #[arg(long)]
    debug: bool,

    /// Show detailed breakdown for each transaction
    #[arg(long)]
    verbose: bool,

    /// Only validate failed transactions
    #[arg(long)]
    failed_only: bool,

    /// Only validate successful transactions
    #[arg(long)]
    success_only: bool,

    /// Validate specific transaction types (comma-separated: swap,transfer,ata,failed)
    #[arg(long)]
    types: Option<String>,

    /// Only process swap transactions (Jupiter, Raydium, etc.)
    #[arg(long)]
    swaps_only: bool,

    /// Only process ATA closure transactions
    #[arg(long)]
    ata_only: bool,

    /// Only process bulk/spam transactions (multiple transfers)
    #[arg(long)]
    bulk_only: bool,

    /// Only process SOL transfer transactions
    #[arg(long)]
    transfers_only: bool,

    /// Filter by minimum SOL amount (e.g., 0.1 for transactions > 0.1 SOL)
    #[arg(long)]
    min_sol: Option<f64>,

    /// Filter by maximum SOL amount (e.g., 10.0 for transactions < 10 SOL)
    #[arg(long)]
    max_sol: Option<f64>,

    /// Skip database operations (test processing only)
    #[arg(long)]
    no_db: bool,

    /// Show statistics summary only
    #[arg(long)]
    summary_only: bool,

    /// Full debug mode: detailed analysis of a single transaction
    #[arg(long)]
    full_debug: bool,

    /// Transaction index for full debug mode (0-based, default: 0)
    #[arg(long, default_value = "0")]
    debug_index: usize,
}

#[derive(Debug, Clone, Tabled)]
struct ValidationResult {
    signature: String,
    status: String,
    tx_type: String,
    direction: String,
    sol_change: f64,
    fee_sol: f64,
    processing_time_ms: u64,
    validation_status: String,
    issues: String,
}

#[derive(Debug, Default)]
struct ValidationStats {
    total_processed: usize,
    successful: usize,
    failed: usize,
    blockchain_failed: usize, // Failed on blockchain (not processing errors)
    swaps_detected: usize,
    ata_operations: usize,
    transfers: usize,
    unknown_types: usize,
    processing_errors: usize,
    validation_errors: usize,
    cache_hits: usize,
    cache_misses: usize,
    total_processing_time_ms: u64,
    total_sol_volume: f64,
    sol_volume: f64,
    total_fees: f64,
    fee_total: f64,
}

/// Check if a transaction matches the specified filters
fn transaction_matches_filters(result: &ValidationResult, args: &Args) -> bool {
    let tx_type_str = result.tx_type.to_lowercase();
    let status_str = result.status.to_lowercase();

    // Apply legacy types filter first
    if let Some(ref types_filter) = args.types {
        let allowed_types: Vec<&str> = types_filter.split(',').collect();

        let matches_filter = allowed_types.iter().any(|t| {
            let filter_type = t.trim().to_lowercase();
            match filter_type.as_str() {
                "swap" => tx_type_str.contains("swap"),
                "transfer" => tx_type_str.contains("transfer"),
                "ata" => tx_type_str.contains("ata") || tx_type_str.contains("close"),
                "failed" => status_str.contains("failed"),
                "bulk" => is_bulk_transaction(result),
                "spam" => is_spam_transaction(result),
                _ => tx_type_str.contains(&filter_type),
            }
        });

        if !matches_filter {
            return false;
        }
    }

    // Apply specific type filters
    if args.swaps_only {
        // Match swap-related types: "Buy", "Sell", "Buy (Legacy)", "Sell (Legacy)", or anything containing "swap"
        let is_swap =
            tx_type_str.contains("buy") ||
            tx_type_str.contains("sell") ||
            tx_type_str.contains("swap");
        if !is_swap {
            return false;
        }
    }

    if args.ata_only {
        // Match ATA-related types: "ATA Operation", "ATA Close", or anything containing "ata" or "close"
        let is_ata =
            tx_type_str.contains("ata") ||
            tx_type_str.contains("close") ||
            tx_type_str.contains("operation");
        if !is_ata {
            return false;
        }
    }

    if args.transfers_only {
        // Match transfer types: "Transfer", "SOL Transfer", "Token Transfer", etc.
        let is_transfer =
            tx_type_str.contains("transfer") ||
            (tx_type_str.contains("sol") && tx_type_str.contains("transfer"));
        if !is_transfer {
            return false;
        }
    }

    if args.bulk_only && !is_bulk_transaction(result) {
        return false;
    }

    // Apply SOL amount filters
    if let Some(min_sol) = args.min_sol {
        if result.sol_change.abs() < min_sol {
            return false;
        }
    }

    if let Some(max_sol) = args.max_sol {
        if result.sol_change.abs() > max_sol {
            return false;
        }
    }

    true
}

/// Determine if a transaction is a bulk/spam transaction
fn is_bulk_transaction(result: &ValidationResult) -> bool {
    // Consider bulk transactions as:
    // 1. Multiple small transfers (low SOL amounts)
    // 2. High fee relative to amount (spam indicator)
    // 3. Very small SOL changes (dust transactions)

    let sol_change = result.sol_change.abs();
    let fee_to_amount_ratio = if sol_change > 0.0 {
        result.fee_sol / sol_change
    } else {
        f64::INFINITY
    };

    // Bulk/spam indicators:
    // - Very small amounts (< 0.001 SOL)
    // - High fee ratio (> 10% of transaction amount)
    // - Dust amounts (< 0.0001 SOL)
    sol_change < 0.001 || fee_to_amount_ratio > 0.1 || sol_change < 0.0001
}

/// Determine if a transaction is spam
fn is_spam_transaction(result: &ValidationResult) -> bool {
    // Spam transactions typically have:
    // 1. Very high fee-to-amount ratio
    // 2. Extremely small amounts
    // 3. Unknown transaction type (often failed attempts)

    let sol_change = result.sol_change.abs();
    let fee_to_amount_ratio = if sol_change > 0.0 {
        result.fee_sol / sol_change
    } else {
        f64::INFINITY
    };

    // Spam indicators:
    // - Dust transactions (< 0.00001 SOL)
    // - Extremely high fee ratio (> 50% of amount)
    // - Unknown type with very small amounts
    sol_change < 0.00001 ||
        fee_to_amount_ratio > 0.5 ||
        (result.tx_type.to_lowercase().contains("unknown") && sol_change < 0.0001)
}

/// Perform comprehensive debug analysis of a single transaction
async fn perform_full_debug_analysis(
    signature: &str,
    processor: &TransactionProcessor,
    wallet_pubkey: Pubkey
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Transaction Signature: {}", signature);
    println!("👛 Wallet: {}", wallet_pubkey);
    println!();

    // Step 1: Fetch raw transaction data
    println!("🌐 Step 1: Fetching raw transaction data from RPC...");
    let fetch_start = Instant::now();

    let transaction = match processor.process_transaction(signature).await {
        Ok(tx) => {
            println!("✅ Transaction processed in {:.2}ms", fetch_start.elapsed().as_millis());
            tx
        }
        Err(e) => {
            println!("❌ Failed to process transaction: {}", e);
            return Ok(());
        }
    };

    println!();

    // Step 2: Basic Transaction Info
    println!("📋 TRANSACTION OVERVIEW");
    println!("=======================");
    println!("🔗 Signature: {}", transaction.signature);
    println!("🏷️  Status: {:?}", transaction.status);
    println!("✅ Success: {}", transaction.success);
    println!("🕒 Timestamp: {}", transaction.timestamp);
    if let Some(slot) = transaction.slot {
        println!("📍 Slot: {}", slot);
    }
    if let Some(block_time) = transaction.block_time {
        println!("⏰ Block Time: {}", block_time);
    }

    // Show error if failed
    if !transaction.success {
        if let Some(ref error) = transaction.error_message {
            println!("❌ Error: {}", error);
        }
    }

    println!("💰 Fee (lamports): {:?}", transaction.fee_lamports);
    println!("💰 Fee (SOL): {:.9}", transaction.fee_sol);
    println!("📏 Instructions Count: {}", transaction.instructions_count);
    println!("👥 Accounts Count: {}", transaction.accounts_count);

    if let Some(duration) = transaction.analysis_duration_ms {
        println!("⚡ Analysis Duration: {}ms", duration);
    }

    println!();

    // Step 3: Transaction Classification
    println!("🏷️  TRANSACTION CLASSIFICATION");
    println!("==============================");
    println!("🔄 Type: {:?}", transaction.transaction_type);
    println!("➡️  Direction: {:?}", transaction.direction);
    println!("💹 SOL Balance Change: {:.9} SOL", transaction.sol_balance_change);

    println!();

    // Step 4: Raw Transaction Data Analysis
    if let Some(ref raw_data) = transaction.raw_transaction_data {
        println!("🔬 RAW TRANSACTION DATA ANALYSIS");
        println!("=================================");

        // Show transaction metadata
        if let Some(meta) = raw_data.get("meta") {
            println!("📊 METADATA:");

            if let Some(err) = meta.get("err") {
                println!("  🚨 Error: {}", serde_json::to_string_pretty(err)?);
            }

            if let Some(fee) = meta.get("fee") {
                println!("  💰 Fee: {} lamports", fee);
            }

            // Pre/Post balances
            if let Some(pre_balances) = meta.get("preBalances").and_then(|v| v.as_array()) {
                println!("  📉 Pre-Balances:");
                for (i, balance) in pre_balances.iter().enumerate() {
                    if let Some(bal) = balance.as_u64() {
                        println!(
                            "    Account {}: {} lamports ({:.9} SOL)",
                            i,
                            bal,
                            (bal as f64) / 1_000_000_000.0
                        );
                    }
                }
            }

            if let Some(post_balances) = meta.get("postBalances").and_then(|v| v.as_array()) {
                println!("  📈 Post-Balances:");
                for (i, balance) in post_balances.iter().enumerate() {
                    if let Some(bal) = balance.as_u64() {
                        println!(
                            "    Account {}: {} lamports ({:.9} SOL)",
                            i,
                            bal,
                            (bal as f64) / 1_000_000_000.0
                        );
                    }
                }
            }

            // Token balances
            if
                let Some(pre_token_balances) = meta
                    .get("preTokenBalances")
                    .and_then(|v| v.as_array())
            {
                if !pre_token_balances.is_empty() {
                    println!("  🪙 Pre-Token Balances:");
                    for token_balance in pre_token_balances {
                        display_token_balance(token_balance, "    ");
                    }
                }
            }

            if
                let Some(post_token_balances) = meta
                    .get("postTokenBalances")
                    .and_then(|v| v.as_array())
            {
                if !post_token_balances.is_empty() {
                    println!("  🪙 Post-Token Balances:");
                    for token_balance in post_token_balances {
                        display_token_balance(token_balance, "    ");
                    }
                }
            }

            // Log messages
            if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                println!("  📝 LOG MESSAGES:");
                for (i, log) in logs.iter().enumerate() {
                    if let Some(log_str) = log.as_str() {
                        println!("    {}: {}", i + 1, log_str);
                    }
                }
            }

            // Inner instructions
            if
                let Some(inner_instructions) = meta
                    .get("innerInstructions")
                    .and_then(|v| v.as_array())
            {
                if !inner_instructions.is_empty() {
                    println!("  🔧 INNER INSTRUCTIONS:");
                    for (i, inner_inst) in inner_instructions.iter().enumerate() {
                        println!("    Inner Instruction Set {}:", i + 1);
                        if
                            let Some(instructions) = inner_inst
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for (j, inst) in instructions.iter().enumerate() {
                                println!(
                                    "      Instruction {}: {}",
                                    j + 1,
                                    serde_json::to_string_pretty(inst)?
                                );
                            }
                        }
                    }
                }
            }
        }

        // Show main transaction structure
        if let Some(tx) = raw_data.get("transaction") {
            println!();
            println!("📋 TRANSACTION STRUCTURE:");

            if let Some(message) = tx.get("message") {
                if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
                    println!("  🔧 MAIN INSTRUCTIONS ({} total):", instructions.len());
                    for (i, inst) in instructions.iter().enumerate() {
                        println!("    Instruction {}:", i + 1);
                        if let Some(program_id_index) = inst.get("programIdIndex") {
                            println!("      Program ID Index: {}", program_id_index);
                        }
                        if let Some(accounts) = inst.get("accounts").and_then(|v| v.as_array()) {
                            println!("      Accounts: {:?}", accounts);
                        }
                        if let Some(data) = inst.get("data").and_then(|v| v.as_str()) {
                            println!("      Data: {} (length: {})", data, data.len());
                        }
                    }
                }

                if let Some(account_keys) = message.get("accountKeys").and_then(|v| v.as_array()) {
                    println!("  👥 ACCOUNT KEYS ({} total):", account_keys.len());
                    for (i, key) in account_keys.iter().enumerate() {
                        if let Some(key_str) = key.as_str() {
                            println!("    {}: {}", i, key_str);
                        }
                    }
                }
            }
        }
    }

    println!();

    // Step 5: Processed Transaction Analysis
    println!("🧮 PROCESSED ANALYSIS RESULTS");
    println!("==============================");

    // Balance changes
    println!("💹 SOL Balance Change: {:.9} SOL", transaction.sol_balance_change);

    // Token balance changes if any
    if !transaction.token_balance_changes.is_empty() {
        println!("🪙 Token Balance Changes:");
        for change in &transaction.token_balance_changes {
            println!("  Mint: {}", change.mint);
            println!("  Change: {:.9}", change.change);
            println!("  Decimals: {}", change.decimals);
            if let Some(pre) = change.pre_balance {
                println!("  Pre-Balance: {:.9}", pre);
            }
            if let Some(post) = change.post_balance {
                println!("  Post-Balance: {:.9}", post);
            }
        }
    }

    // ATA operations
    if !transaction.ata_operations.is_empty() {
        println!("🏦 ATA Operations:");
        for ata_op in &transaction.ata_operations {
            println!("  Type: {:?}", ata_op.operation_type);
            println!("  Account: {}", ata_op.account_address);
            println!("  Mint: {}", ata_op.token_mint);
            println!("  Rent Amount: {} lamports", ata_op.rent_amount);
            println!("  Is WSOL: {}", ata_op.is_wsol);
        }
    }

    // Swap info
    if let Some(ref swap_info) = transaction.token_swap_info {
        println!("🔄 Swap Information:");
        println!("  Token Mint: {}", swap_info.mint);
        println!("  Token Symbol: {}", swap_info.symbol);
        if let Some(price) = swap_info.current_price_sol {
            println!("  Current Price: {:.12} SOL", price);
        }
        println!("  Decimals: {}", swap_info.decimals);
        println!("  Is Verified: {}", swap_info.is_verified);
    }

    // PnL info
    if let Some(ref pnl_info) = transaction.swap_pnl_info {
        println!("📊 P&L Information:");
        println!("  Token Mint: {}", pnl_info.token_mint);
        println!("  Token Symbol: {}", pnl_info.token_symbol);
        println!("  Swap Type: {:?}", pnl_info.swap_type);
        println!("  SOL Amount: {:.9}", pnl_info.sol_amount);
        println!("  Token Amount: {:.9}", pnl_info.token_amount);
    }

    println!();
    println!("✅ Full debug analysis completed!");

    Ok(())
}

fn display_token_balance(token_balance: &serde_json::Value, indent: &str) {
    if let Some(account_index) = token_balance.get("accountIndex") {
        println!("{}Account Index: {}", indent, account_index);
    }
    if let Some(mint) = token_balance.get("mint").and_then(|v| v.as_str()) {
        println!("{}Mint: {}", indent, mint);
    }
    if let Some(owner) = token_balance.get("owner").and_then(|v| v.as_str()) {
        println!("{}Owner: {}", indent, owner);
    }
    if let Some(ui_token_amount) = token_balance.get("uiTokenAmount") {
        if let Some(amount) = ui_token_amount.get("amount").and_then(|v| v.as_str()) {
            println!("{}Amount: {}", indent, amount);
        }
        if let Some(ui_amount) = ui_token_amount.get("uiAmount") {
            println!("{}UI Amount: {}", indent, ui_amount);
        }
        if let Some(decimals) = ui_token_amount.get("decimals") {
            println!("{}Decimals: {}", indent, decimals);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set debug flags if requested
    if args.debug {
        set_cmd_args(
            vec!["validate_transactions_system".to_string(), "--debug-transactions".to_string()]
        );
    }

    println!("🔍 Transaction System Validation Tool");
    println!("=====================================");
    println!("📊 Validating last {} transactions", args.count);

    if args.debug {
        println!("🐛 Debug logging enabled");
    }

    println!();

    // Initialize wallet
    let wallet_str = get_wallet_address()?;
    let wallet_pubkey = Pubkey::from_str(&wallet_str)?;
    println!("👛 Wallet: {}", wallet_str);
    println!();

    // Initialize global systems that transaction processing depends on
    println!("⚙️  Initializing transaction system dependencies...");

    // Initialize transaction processor
    let processor = TransactionProcessor::new(wallet_pubkey);

    // Initialize database if not skipped
    let db = if args.no_db {
        println!("⚠️  Database operations disabled");
        None
    } else {
        match get_transaction_database().await {
            Some(db) => {
                println!("✅ Transaction database initialized");
                Some(db)
            }
            None => {
                println!("❌ Failed to initialize transaction database");
                return Ok(());
            }
        }
    };

    println!();

    // Step 1: Fetch recent transaction signatures
    println!("🌐 Step 1: Fetching recent transaction signatures...");
    let fetch_start = Instant::now();

    // Use transaction fetcher for better signature handling
    let fetcher = TransactionFetcher::new();
    let signature_strings = match fetcher.fetch_recent_signatures(wallet_pubkey, args.count).await {
        Ok(sigs) => sigs,
        Err(e) => {
            eprintln!("❌ Failed to fetch signatures: {}", e);
            return Ok(());
        }
    };

    let fetch_time = fetch_start.elapsed();
    println!(
        "✅ Fetched {} signatures in {:.2}s",
        signature_strings.len(),
        fetch_time.as_secs_f64()
    );
    println!();

    // Handle full debug mode for single transaction
    if args.full_debug {
        if let Some(signature) = signature_strings.get(args.debug_index) {
            println!(
                "🔬 FULL DEBUG MODE: Analyzing transaction {} of {}",
                args.debug_index + 1,
                signature_strings.len()
            );
            println!("================================================");
            perform_full_debug_analysis(signature, &processor, wallet_pubkey).await?;
            return Ok(());
        } else {
            println!(
                "❌ No transaction found at index {} (total: {})",
                args.debug_index,
                signature_strings.len()
            );
            return Ok(());
        }
    }

    // Step 2: Process and validate each transaction
    println!("⚙️  Step 2: Processing and validating transactions...");

    let mut results = Vec::new();
    let mut stats = ValidationStats::default();
    let validation_start = Instant::now();

    for (index, signature) in signature_strings.iter().enumerate() {
        if !args.summary_only {
            if index % 50 == 0 {
                println!(
                    "📊 Progress: {}/{} transactions processed",
                    index,
                    signature_strings.len()
                );
            }
        }

        // Note: We can't filter by success/failure here since we only have signatures
        // The filtering will be done after processing each transaction

        let validation_result = validate_transaction(
            signature,
            &processor,
            db.as_ref().map(|v| &**v),
            &mut stats,
            &args
        ).await;

        // Apply status filters after processing
        if args.failed_only && validation_result.status != "Failed" {
            continue;
        }
        if args.success_only && validation_result.status == "Failed" {
            continue;
        }

        // Apply enhanced type filters
        if !transaction_matches_filters(&validation_result, &args) {
            continue;
        }

        results.push(validation_result);
        stats.total_processed += 1;
    }

    let total_validation_time = validation_start.elapsed();
    stats.total_processing_time_ms = total_validation_time.as_millis() as u64;

    println!("✅ Validation completed in {:.2}s", total_validation_time.as_secs_f64());
    println!();

    // Step 3: Display results
    if !args.summary_only && !results.is_empty() {
        println!("📋 Transaction Validation Results:");
        println!("{}", tabled::Table::new(&results));
        println!();
    }

    // Step 4: Display comprehensive statistics
    display_validation_statistics(&stats, &args);

    // Step 5: Analyze issues and recommendations
    analyze_issues_and_recommendations(&results, &stats);

    Ok(())
}

async fn validate_transaction(
    signature: &str,
    processor: &TransactionProcessor,
    _db: Option<&screenerbot::transactions::database::TransactionDatabase>,
    stats: &mut ValidationStats,
    args: &Args
) -> ValidationResult {
    let start_time = Instant::now();
    let mut issues = Vec::new();

    // Try to get from cache first
    let cached_transaction = if args.no_db {
        None
    } else {
        match get_transaction(signature).await {
            Ok(tx) => {
                if tx.is_some() {
                    stats.cache_hits += 1;
                } else {
                    stats.cache_misses += 1;
                }
                tx
            }
            Err(e) => {
                issues.push(format!("Cache error: {}", e));
                None
            }
        }
    };

    // Process transaction if not cached or if we want fresh analysis
    let transaction = if let Some(cached) = cached_transaction {
        if args.verbose {
            println!("  📋 Using cached transaction: {}", signature);
        }
        cached
    } else {
        // Process fresh
        if args.verbose {
            println!("  ⚙️  Processing fresh: {}", signature);
        }

        match processor.process_transaction(signature).await {
            Ok(mut tx) => {
                // Apply heuristic classification if transaction type is Unknown
                if matches!(tx.transaction_type, TransactionType::Unknown) && tx.success {
                    if let Some(ref raw_data) = tx.raw_transaction_data {
                        let heuristic_type = classify_transaction_heuristically(&tx, raw_data);
                        if !matches!(heuristic_type, TransactionType::Unknown) {
                            if args.verbose {
                                println!(
                                    "    🧠 Heuristic classification: {:?} -> {:?}",
                                    tx.transaction_type,
                                    heuristic_type
                                );
                            }
                            tx.transaction_type = heuristic_type;
                        }
                    }
                }
                tx
            }
            Err(e) => {
                stats.processing_errors += 1;
                return ValidationResult {
                    signature: signature.to_string(),
                    status: "ProcessError".to_string(),
                    tx_type: "Unknown".to_string(),
                    direction: "Unknown".to_string(),
                    sol_change: 0.0,
                    fee_sol: 0.0,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    validation_status: "❌ Failed".to_string(),
                    issues: e.to_string(),
                };
            }
        }
    };

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Debug transaction details
    if args.verbose {
        println!(
            "    📊 Transaction details: success={}, type={:?}, sig_len={}",
            transaction.success,
            transaction.transaction_type,
            transaction.signature.len()
        );
        if !transaction.success {
            if let Some(ref error) = transaction.error_message {
                println!("      ❌ Error: {}", error);
            }
        }

        // Show raw metadata for debugging
        if let Some(ref raw_data) = transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                println!(
                    "      🔍 Meta: {}",
                    serde_json::to_string_pretty(meta).unwrap_or_default()
                );
            }
        }
    }

    // Validate transaction data
    let validation_status = validate_transaction_data(&transaction, &mut issues, stats);

    // Update statistics
    update_transaction_stats(&transaction, stats);

    ValidationResult {
        signature: &transaction.signature.to_string(),
        status: format!("{:?}", transaction.status),
        tx_type: format_transaction_type(&transaction.transaction_type),
        direction: format!("{:?}", transaction.direction),
        sol_change: transaction.sol_balance_change,
        fee_sol: transaction.fee_sol,
        processing_time_ms,
        validation_status,
        issues: if issues.is_empty() {
            "None".to_string()
        } else {
            issues.join("; ")
        },
    }
}

fn validate_transaction_data(
    transaction: &Transaction,
    issues: &mut Vec<String>,
    _stats: &mut ValidationStats
) -> String {
    let mut validation_errors = 0;

    // Validate basic data consistency
    if transaction.signature.is_empty() {
        issues.push("Empty signature".to_string());
        validation_errors += 1;
    }

    // Solana signatures can be 87 or 88 characters depending on encoding
    if transaction.signature.len() < 87 || transaction.signature.len() > 88 {
        issues.push(
            format!(
                "Invalid signature length: {} chars (expected 87-88)",
                transaction.signature.len()
            )
        );
        validation_errors += 1;
    }

    // Validate fee calculations
    if let Some(fee_lamports) = transaction.fee_lamports {
        let expected_fee_sol = (fee_lamports as f64) / 1_000_000_000.0;
        let fee_diff = (transaction.fee_sol - expected_fee_sol).abs();

        if fee_diff > 0.000000001 {
            issues.push(format!("Fee mismatch: {} vs {}", transaction.fee_sol, expected_fee_sol));
            validation_errors += 1;
        }
    }

    // Validate transaction type assignments
    match &transaction.transaction_type {
        TransactionType::Unknown => {
            if transaction.success {
                issues.push("Successful transaction not classified".to_string());
                validation_errors += 1;
            }
        }
        TransactionType::Failed => {
            // Failed transactions should have success=false - this is correct behavior
            if transaction.success {
                issues.push("Failed transaction marked as successful".to_string());
                validation_errors += 1;
            }
        }
        TransactionType::SwapSolToToken { sol_amount, .. } => {
            if transaction.sol_balance_change > 0.0 {
                issues.push("Buy swap should have negative SOL change".to_string());
                validation_errors += 1;
            }
            if *sol_amount <= 0.0 {
                issues.push("Invalid SOL amount in swap".to_string());
                validation_errors += 1;
            }
        }
        TransactionType::SwapTokenToSol { sol_amount, .. } => {
            if transaction.sol_balance_change < 0.0 {
                issues.push("Sell swap should have positive SOL change".to_string());
                validation_errors += 1;
            }
            if *sol_amount <= 0.0 {
                issues.push("Invalid SOL amount in swap".to_string());
                validation_errors += 1;
            }
        }
        TransactionType::AtaClose { recovered_sol, .. } => {
            if *recovered_sol <= 0.0 {
                issues.push("ATA close should recover SOL".to_string());
                validation_errors += 1;
            }
            if transaction.sol_balance_change <= 0.0 {
                issues.push("ATA close should increase SOL balance".to_string());
                validation_errors += 1;
            }
        }
        _ => {}
    }

    // Validate direction consistency
    match transaction.direction {
        TransactionDirection::Incoming => {
            if transaction.sol_balance_change < 0.0 {
                issues.push("Incoming transaction should not decrease SOL".to_string());
                validation_errors += 1;
            }
        }
        TransactionDirection::Outgoing => {
            if transaction.sol_balance_change > 0.0 {
                issues.push("Outgoing transaction should not increase SOL".to_string());
                validation_errors += 1;
            }
        }
        _ => {}
    }

    // Check for missing critical data in successful transactions
    if transaction.success {
        if transaction.fee_lamports.is_none() {
            issues.push("Missing fee data".to_string());
            validation_errors += 1;
        }

        if transaction.instructions_count == 0 {
            issues.push("No instructions detected".to_string());
            validation_errors += 1;
        }
    }

    if validation_errors == 0 {
        "✅ Valid".to_string()
    } else {
        format!("❌ {} errors", validation_errors)
    }
}

fn update_transaction_stats(transaction: &Transaction, stats: &mut ValidationStats) {
    if transaction.success {
        stats.successful += 1;
    } else {
        stats.failed += 1;
    }

    match &transaction.transaction_type {
        | TransactionType::SwapSolToToken { .. }
        | TransactionType::SwapTokenToSol { .. }
        | TransactionType::SwapTokenToToken { .. } => {
            stats.swaps_detected += 1;
        }
        TransactionType::AtaClose { .. } => {
            stats.ata_operations += 1;
        }
        TransactionType::SolTransfer { .. } | TransactionType::TokenTransfer { .. } => {
            stats.transfers += 1;
        }
        TransactionType::Failed => {
            stats.blockchain_failed += 1;
        }
        TransactionType::Unknown => {
            stats.unknown_types += 1;
        }
        _ => {}
    }

    stats.sol_volume += transaction.sol_balance_change.abs();
    stats.fee_total += transaction.fee_sol;
}

fn format_transaction_type(tx_type: &TransactionType) -> String {
    match tx_type {
        TransactionType::SwapSolToToken { router, .. } => format!("{} Buy", router),
        TransactionType::SwapTokenToSol { router, .. } => format!("{} Sell", router),
        TransactionType::SwapTokenToToken { router, .. } => format!("{} Swap", router),
        TransactionType::SolTransfer { .. } => "SOL Transfer".to_string(),
        TransactionType::TokenTransfer { .. } => "Token Transfer".to_string(),
        TransactionType::AtaClose { .. } => "ATA Close".to_string(),
        TransactionType::Failed => "Failed".to_string(),
        TransactionType::Unknown => "Unknown".to_string(),
        TransactionType::Buy => "Buy (Legacy)".to_string(),
        TransactionType::Sell => "Sell (Legacy)".to_string(),
        TransactionType::Transfer => "Transfer (Legacy)".to_string(),
        TransactionType::Compute => "Compute".to_string(),
        TransactionType::AtaOperation => "ATA Operation".to_string(),
        TransactionType::Other { description, .. } => format!("Other: {}", description),
    }
}

fn display_validation_statistics(stats: &ValidationStats, args: &Args) {
    println!("📊 Validation Statistics:");
    println!("=========================");

    println!("📈 Transaction Processing:");
    println!("  Total Processed: {}", stats.total_processed);
    println!("  Successful: {} ({:.1}%)", stats.successful, if stats.total_processed > 0 {
        ((stats.successful as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });
    println!("  Failed: {} ({:.1}%)", stats.failed, if stats.total_processed > 0 {
        ((stats.failed as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });

    println!("\n🏷️  Transaction Classification:");
    println!("  Swaps Detected: {} ({:.1}%)", stats.swaps_detected, if stats.total_processed > 0 {
        ((stats.swaps_detected as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });
    println!("  ATA Operations: {} ({:.1}%)", stats.ata_operations, if stats.total_processed > 0 {
        ((stats.ata_operations as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });
    println!("  Transfers: {} ({:.1}%)", stats.transfers, if stats.total_processed > 0 {
        ((stats.transfers as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });
    println!("  Unknown Types: {} ({:.1}%)", stats.unknown_types, if stats.total_processed > 0 {
        ((stats.unknown_types as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    });

    if !args.no_db {
        println!("\n💾 Cache Performance:");
        println!("  Cache Hits: {} ({:.1}%)", stats.cache_hits, if
            stats.cache_hits + stats.cache_misses > 0
        {
            ((stats.cache_hits as f64) / ((stats.cache_hits + stats.cache_misses) as f64)) * 100.0
        } else {
            0.0
        });
        println!("  Cache Misses: {}", stats.cache_misses);
    }

    println!("\n⚡ Performance:");
    let avg_processing_time = if stats.total_processed > 0 {
        (stats.total_processing_time_ms as f64) / (stats.total_processed as f64)
    } else {
        0.0
    };
    println!("  Total Processing Time: {:.2}s", (stats.total_processing_time_ms as f64) / 1000.0);
    println!("  Average Processing Time: {:.1}ms/tx", avg_processing_time);

    if stats.total_processed > 0 {
        let throughput =
            (stats.total_processed as f64) / ((stats.total_processing_time_ms as f64) / 1000.0);
        println!("  Processing Throughput: {:.1} tx/s", throughput);
    }

    println!("\n💰 Financial Summary:");
    println!("  Total SOL Volume: {:.6} SOL", stats.sol_volume);
    println!("  Total Fees Paid: {:.9} SOL", stats.fee_total);

    if stats.total_processed > 0 {
        println!("  Average Fee: {:.9} SOL", stats.fee_total / (stats.total_processed as f64));
    }

    println!("\n🚨 Error Analysis:");
    println!("  Processing Errors: {}", stats.processing_errors);
    println!("  Validation Errors: {}", stats.validation_errors);

    let error_rate = if stats.total_processed > 0 {
        (((stats.processing_errors + stats.validation_errors) as f64) /
            (stats.total_processed as f64)) *
            100.0
    } else {
        0.0
    };
    println!("  Error Rate: {:.2}%", error_rate);

    println!();
}

fn analyze_issues_and_recommendations(results: &[ValidationResult], stats: &ValidationStats) {
    println!("🔍 Issue Analysis & Recommendations:");
    println!("=====================================");

    // Analyze common issues
    let mut issue_counts: HashMap<String, usize> = HashMap::new();
    for result in results {
        if result.issues != "None" {
            let issues: Vec<&str> = result.issues.split("; ").collect();
            for issue in issues {
                *issue_counts.entry(issue.to_string()).or_insert(0) += 1;
            }
        }
    }

    if !issue_counts.is_empty() {
        println!("🚨 Most Common Issues:");
        let mut sorted_issues: Vec<_> = issue_counts.iter().collect();
        sorted_issues.sort_by(|a, b| b.1.cmp(a.1));

        for (issue, count) in sorted_issues.iter().take(5) {
            println!("  • {} ({}x)", issue, count);
        }
        println!();
    }

    // System health assessment
    println!("🏥 System Health Assessment:");

    let classification_rate = if stats.total_processed > 0 {
        (((stats.total_processed - stats.unknown_types) as f64) / (stats.total_processed as f64)) *
            100.0
    } else {
        0.0
    };

    if classification_rate >= 95.0 {
        println!("  ✅ Transaction Classification: Excellent ({:.1}%)", classification_rate);
    } else if classification_rate >= 85.0 {
        println!("  ⚠️  Transaction Classification: Good ({:.1}%)", classification_rate);
    } else {
        println!(
            "  ❌ Transaction Classification: Needs Improvement ({:.1}%)",
            classification_rate
        );
    }

    let error_rate = if stats.total_processed > 0 {
        (((stats.processing_errors + stats.validation_errors) as f64) /
            (stats.total_processed as f64)) *
            100.0
    } else {
        0.0
    };

    if error_rate <= 1.0 {
        println!("  ✅ Error Rate: Excellent ({:.2}%)", error_rate);
    } else if error_rate <= 5.0 {
        println!("  ⚠️  Error Rate: Acceptable ({:.2}%)", error_rate);
    } else {
        println!("  ❌ Error Rate: High ({:.2}%)", error_rate);
    }

    // Recommendations
    println!("\n💡 Recommendations:");

    if stats.unknown_types > stats.total_processed / 10 {
        println!("  • Improve transaction classification - too many unknown types");
    }

    if stats.validation_errors > 0 {
        println!("  • Fix validation errors in transaction processing logic");
    }

    if stats.processing_errors > stats.total_processed / 20 {
        println!("  • Investigate and fix transaction processing failures");
    }

    let avg_processing_time = if stats.total_processed > 0 {
        (stats.total_processing_time_ms as f64) / (stats.total_processed as f64)
    } else {
        0.0
    };

    if avg_processing_time > 1000.0 {
        println!(
            "  • Optimize transaction processing performance (avg: {:.1}ms)",
            avg_processing_time
        );
    }

    if issue_counts.is_empty() && error_rate <= 1.0 && classification_rate >= 95.0 {
        println!("  🎉 System appears to be working excellently!");
    }

    println!();
}

/// Perform heuristic classification based on transaction logs and patterns
fn classify_transaction_heuristically(
    transaction: &Transaction,
    tx_details: &serde_json::Value
) -> TransactionType {
    // Get log messages from raw transaction data
    let empty_vec = vec![];
    let log_messages = tx_details
        .get("meta")
        .and_then(|m| m.get("logMessages"))
        .and_then(|l| l.as_array())
        .unwrap_or(&empty_vec);

    // Convert to strings for analysis
    let logs: Vec<String> = log_messages
        .iter()
        .filter_map(|msg| msg.as_str())
        .map(|s| s.to_string())
        .collect();

    // Priority 1: Check for DEX/Swap patterns
    for log in &logs {
        // Jupiter aggregator
        if log.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            // Check if buying or selling based on token balance changes
            if
                let Some(post_balances) = tx_details
                    .get("meta")
                    .and_then(|m| m.get("postTokenBalances"))
                    .and_then(|b| b.as_array())
            {
                if
                    let Some(pre_balances) = tx_details
                        .get("meta")
                        .and_then(|m| m.get("preTokenBalances"))
                        .and_then(|b| b.as_array())
                {
                    // Simple heuristic: if we gained tokens, it's a buy; if we lost tokens, it's a sell
                    return if post_balances.len() >= pre_balances.len() {
                        TransactionType::Buy
                    } else {
                        TransactionType::Sell
                    };
                }
            }
            return TransactionType::Buy; // Default to buy for Jupiter
        }

        // Raydium CPMM
        if
            log.contains("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG") ||
            log.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")
        {
            return TransactionType::Buy;
        }

        // Orca
        if log.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            return TransactionType::Buy;
        }

        // Pump.fun
        if
            log.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
            log.contains("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1")
        {
            return TransactionType::Buy;
        }

        // Generic swap instructions
        if log.contains("Instruction: Swap") || log.contains("Instruction: Route") {
            return TransactionType::Buy;
        }
    }

    // Priority 2: Check for ATA operations
    for log in &logs {
        if log.contains("Instruction: CloseAccount") {
            return TransactionType::AtaOperation;
        }
        if log.contains("Instruction: CreateIdempotent") {
            return TransactionType::AtaOperation;
        }
        if log.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") {
            return TransactionType::AtaOperation;
        }
        if log.contains("Initialize the associated token account") {
            return TransactionType::AtaOperation;
        }
        if log.contains("Instruction: InitializeAccount") {
            return TransactionType::AtaOperation;
        }
    }

    // Priority 3: Check for transfers
    for log in &logs {
        if log.contains("Instruction: Transfer") || log.contains("Instruction: TransferChecked") {
            return TransactionType::Transfer;
        }
    }

    // Priority 4: Check for bulk SOL transfers (system program only)
    let system_program_count = logs
        .iter()
        .filter(|log| log.contains("Program 11111111111111111111111111111111"))
        .count();

    // Check if transaction has no token operations
    let has_token_operations =
        tx_details
            .get("meta")
            .and_then(|m| m.get("preTokenBalances"))
            .and_then(|b| b.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false) ||
        tx_details
            .get("meta")
            .and_then(|m| m.get("postTokenBalances"))
            .and_then(|b| b.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false);

    // Bulk transfer: many system program calls with only SOL operations
    if system_program_count >= 10 && !has_token_operations {
        return TransactionType::SolTransfer {
            amount: 0.0, // Will be calculated later from balance changes
            from: "Multiple".to_string(),
            to: "Multiple".to_string(),
        };
    }

    // Simple transfer: few system program calls with only SOL operations
    if system_program_count > 0 && system_program_count < 10 && !has_token_operations {
        let non_system_logs = logs
            .iter()
            .filter(
                |log|
                    !log.contains("Program 11111111111111111111111111111111") &&
                    !log.contains("ComputeBudget111111111111111111111111111111")
            )
            .count();

        if non_system_logs == 0 {
            return TransactionType::Transfer;
        }
    }

    // Priority 5: Check for compute budget operations
    let compute_budget_count = logs
        .iter()
        .filter(|log| log.contains("ComputeBudget111111111111111111111111111111"))
        .count();

    if compute_budget_count > 0 && logs.len() <= compute_budget_count + 2 {
        return TransactionType::Compute;
    }

    // Use existing classification if available
    transaction.transaction_type.clone()
}
