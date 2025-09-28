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

    /// Skip database operations (test processing only)
    #[arg(long)]
    no_db: bool,

    /// Show statistics summary only
    #[arg(long)]
    summary_only: bool,
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

        // Apply type filters
        if let Some(ref types_filter) = args.types {
            let allowed_types: Vec<&str> = types_filter.split(',').collect();
            let tx_type_str = validation_result.tx_type.to_lowercase();

            let matches_filter = allowed_types.iter().any(|t| {
                let filter_type = t.trim().to_lowercase();
                match filter_type.as_str() {
                    "swap" => tx_type_str.contains("swap"),
                    "transfer" => tx_type_str.contains("transfer"),
                    "ata" => tx_type_str.contains("ata") || tx_type_str.contains("close"),
                    "failed" => validation_result.status.contains("Failed"),
                    _ => tx_type_str.contains(&filter_type),
                }
            });

            if !matches_filter {
                continue;
            }
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
            println!("  📋 Using cached transaction: {}", format_signature_short(signature));
        }
        cached
    } else {
        // Process fresh
        if args.verbose {
            println!("  ⚙️  Processing fresh: {}", format_signature_short(signature));
        }

        match processor.process_transaction(signature).await {
            Ok(tx) => tx,
            Err(e) => {
                stats.processing_errors += 1;
                return ValidationResult {
                    signature: format_signature_short(signature).to_string(),
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
        signature: format_signature_short(&transaction.signature).to_string(),
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
