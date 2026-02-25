//! Transaction debug helper functions — validations, formatting, database debug, and profiling.
//
// Split from `debug.rs` to keep that module focused on core structs and main debug entry points.
// Functions here are re-exported from `debug` for public API continuity.

use chrono::{DateTime, Utc};
use std::time::Instant;

use crate::transactions::{
    database::get_transaction_database, processor::TransactionProcessor, types::*,
};

use super::debug::{DebugAnalysisResult, DebugValidation, TransactionDebugStats};

// =============================================================================
// DEBUG VALIDATIONS
// =============================================================================

/// Perform comprehensive debug validations
pub(crate) async fn perform_debug_validations(
    transaction: &Transaction,
) -> Result<Vec<DebugValidation>, String> {
    let mut validations = Vec::new();

    // Validation 1: Basic transaction structure
    validations.push(DebugValidation {
        validation_name: "Basic Structure".to_owned(),
        passed: !transaction.signature.is_empty() && transaction.timestamp <= Utc::now(),
        message: "Transaction has valid signature and timestamp".to_owned(),
        severity: "Critical".to_owned(),
    });

    // Validation 2: Transaction success consistency
    let success_consistent = transaction.success == transaction.error_message.is_none();
    validations.push(DebugValidation {
        validation_name: "Success Consistency".to_owned(),
        passed: success_consistent,
        message: if success_consistent {
            "Success status matches error presence".to_owned()
        } else {
            "Success status inconsistent with error presence".to_owned()
        },
        severity: "Warning".to_owned(),
    });

    // Validation 3: Fee reasonableness
    let reasonable_fee = transaction
        .fee_lamports
        .map(|fee| fee < 10_000_000) // Less than 0.01 SOL
        .unwrap_or(true);
    validations.push(DebugValidation {
        validation_name: "Reasonable Fee".to_owned(),
        passed: reasonable_fee,
        message: format!(
            "Transaction fee is {}",
            if reasonable_fee {
                "reasonable"
            } else {
                "unusually high"
            }
        ),
        severity: (if reasonable_fee { "Info" } else { "Warning" }).to_string(),
    });

    // Validation 4: Analysis completeness
    let has_analysis = transaction.analysis_duration_ms.is_some();
    validations.push(DebugValidation {
        validation_name: "Analysis Completeness".to_owned(),
        passed: has_analysis,
        message: if has_analysis {
            "Transaction has analysis timing data".to_owned()
        } else {
            "Transaction missing analysis timing data".to_owned()
        },
        severity: "Info".to_owned(),
    });

    // Validation 5: Swap transaction data completeness
    if matches!(
        transaction.transaction_type,
        TransactionType::Buy | TransactionType::Sell
    ) {
        let has_swap_info = transaction.token_swap_info.is_some();
        validations.push(DebugValidation {
            validation_name: "Swap Data Completeness".to_owned(),
            passed: has_swap_info,
            message: if has_swap_info {
                "Swap transaction has complete swap information".to_owned()
            } else {
                "Swap transaction missing swap information".to_owned()
            },
            severity: "Warning".to_owned(),
        });
    }

    Ok(validations)
}

// =============================================================================
// DEBUG OUTPUT AND FORMATTING
// =============================================================================

/// Print comprehensive debug analysis
pub fn print_debug_analysis(result: &DebugAnalysisResult) {
    println!("\n TRANSACTION DEBUG ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════");

    // Basic transaction info
    println!("\n TRANSACTION DETAILS");
    println!("Signature: {}", &result.transaction.signature);
    println!("Type: {:?}", result.transaction.transaction_type);
    println!("Direction: {:?}", result.transaction.direction);
    println!(
        "Success: {}",
        if result.transaction.success {
            "Yes"
        } else {
            "No"
        }
    );

    if let Some(error) = &result.transaction.error_message {
        println!("Error: {error}");
    }

    if let Some(fee) = result.transaction.fee_lamports {
        println!("Fee: {:.6} SOL", (fee as f64) / 1_000_000_000.0);
    }

    if let Some(block_time) = result.transaction.block_time {
        let dt = DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now());
        println!("Block Time: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
    }

    println!("Instructions: {}", result.transaction.instructions_count);
    println!("Accounts: {}", result.transaction.accounts_count);

    // Swap information if available
    if let Some(ref swap_info) = result.transaction.token_swap_info {
        println!("\n SWAP INFORMATION");
        println!("Router: {}", swap_info.router);
        println!("Type: {}", swap_info.swap_type);
        println!("Input Mint: {}", &swap_info.input_mint);
        println!("Output Mint: {}", &swap_info.output_mint);
        println!(
            "Input Amount: {:.6} ({} raw)",
            swap_info.input_ui_amount, swap_info.input_amount
        );
        println!(
            "Output Amount: {:.6} ({} raw)",
            swap_info.output_ui_amount, swap_info.output_amount
        );

        if let Some(ref pool) = swap_info.pool_address {
            println!("Pool: {pool}");
        }
    }

    // P&L information if available
    if let Some(ref pnl_info) = result.transaction.swap_pnl_info {
        println!("\n P&L INFORMATION");
        println!("SOL Spent: {:.6}", pnl_info.sol_spent);
        println!("SOL Received: {:.6}", pnl_info.sol_received);
        println!("Tokens Bought: {:.6}", pnl_info.tokens_bought);
        println!("Tokens Sold: {:.6}", pnl_info.tokens_sold);
        println!("Net SOL Change: {:.6}", pnl_info.net_sol_change);
        println!("Fees Paid: {:.6} SOL", pnl_info.fees_paid_sol);

        if let Some(estimated_pnl) = pnl_info.estimated_pnl_sol {
            println!("Estimated P&L: {:.6} SOL", estimated_pnl);
        }
    }

    // ATA operations if any
    if !result.transaction.ata_operations.is_empty() {
        println!("\n ATA OPERATIONS");
        for (i, ata_op) in result.transaction.ata_operations.iter().enumerate() {
            println!(
                "{}. {:?} - Mint: {}",
                i + 1,
                ata_op.operation_type,
                &ata_op.mint
            );
            if let Some(cost) = ata_op.rent_cost_sol {
                println!("Rent Cost: {:.6} SOL", cost);
            }
        }
    }

    // Performance metrics
    println!("\n PERFORMANCE METRICS");
    println!(
        "Total Duration: {}ms",
        result.performance_metrics.total_duration_ms
    );

    if let Some(analysis_ms) = result.performance_metrics.analysis_ms {
        println!("Analysis Duration: {analysis_ms}ms");
    }

    // Analysis steps
    if !result.analysis_steps.is_empty() {
        println!("\n ANALYSIS STEPS");
        for step in &result.analysis_steps {
            let status = if step.success { "" } else { "" };
            println!(
                "{} {} - {}ms - {}",
                status, step.step_name, step.duration_ms, step.details
            );
        }
    }

    // Validations
    if !result.validation_results.is_empty() {
        println!("\n VALIDATIONS");
        for validation in &result.validation_results {
            let status = if validation.passed { "" } else { "" };
            println!(
                "{} [{}] {} - {}",
                status, validation.severity, validation.validation_name, validation.message
            );
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════");
}

/// Print debug statistics table
pub fn print_debug_statistics(stats: &TransactionDebugStats) {
    println!("\n TRANSACTION DEBUG STATISTICS");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Total Processed: {}", stats.total_processed);
    println!(
        "Successful: {} ({:.1}%)",
        stats.successful_transactions,
        if stats.total_processed > 0 {
            ((stats.successful_transactions as f64) / (stats.total_processed as f64)) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Failed: {} ({:.1}%)",
        stats.failed_transactions,
        if stats.total_processed > 0 {
            ((stats.failed_transactions as f64) / (stats.total_processed as f64)) * 100.0
        } else {
            0.0
        }
    );
    println!();
    println!("Transaction Types:");
    println!("• Swaps: {}", stats.swap_transactions);
    println!("• Transfers: {}", stats.transfer_transactions);
    println!("• Unknown: {}", stats.unknown_transactions);
    println!();
    println!(
        "Average Processing Time: {:.1}ms",
        stats.average_processing_time_ms
    );
    println!("Total Fees: {:.6} SOL", stats.total_fees_sol);

    if let Some((start, end)) = stats.date_range {
        println!(
            "Date Range: {} to {}",
            start.format("%Y-%m-%d %H:%M UTC"),
            end.format("%Y-%m-%d %H:%M UTC")
        );
    }

    println!("═══════════════════════════════════════════════════════════════");
}

// =============================================================================
// DATABASE DEBUG UTILITIES
// =============================================================================

/// Debug database connection and statistics
pub async fn debug_database_connection() -> Result<(), String> {
    println!("\n DATABASE DEBUG");
    println!("═══════════════════════════════════════════════════════════════");

    if let Some(db) = get_transaction_database().await {
        // Health check
        match db.health_check().await {
            Ok(()) => println!("Database connection: OK"),
            Err(e) => println!("Database connection: ERROR - {e}"),
        }

        // Statistics
        match db.get_stats().await {
            Ok(stats) => {
                println!("Database Statistics:");
                println!("• Raw Transactions: {}", stats.total_raw_transactions);
                println!(
                    "• Processed Transactions: {}",
                    stats.total_processed_transactions
                );
                println!("• Known Signatures: {}", stats.total_known_signatures);
                println!(
                    "• Pending Transactions: {}",
                    stats.total_pending_transactions
                );
                println!("• Deferred Retries: {}", stats.total_deferred_retries);
                println!(
                    "• Database Size: {:.2} MB",
                    (stats.database_size_bytes as f64) / 1_048_576.0
                );
                println!("• Schema Version: {}", stats.schema_version);
            }
            Err(e) => println!("Database statistics: ERROR - {e}"),
        }

        // Integrity check
        match db.get_integrity_report().await {
            Ok(report) => {
                println!("Database Integrity:");
                println!(
                    "• Schema Version Correct: {}",
                    if report.schema_version_correct {
                        ""
                    } else {
                        ""
                    }
                );
                println!(
                    "• Orphaned Processed: {}",
                    report.orphaned_processed_transactions
                );
                println!(
                    "• Missing Processed: {}",
                    report.missing_processed_transactions
                );
            }
            Err(e) => println!("Database integrity check: ERROR - {e}"),
        }
    } else {
        println!("No database connection available");
    }

    println!("═══════════════════════════════════════════════════════════════");
    Ok(())
}

// =============================================================================
// PERFORMANCE PROFILING
// =============================================================================

/// Profile transaction processing performance
pub async fn profile_transaction_processing(
    signatures: Vec<String>,
    wallet_pubkey: solana_sdk::pubkey::Pubkey,
) -> Result<PerformanceProfile, String> {
    let start_time = Instant::now();
    let processor = TransactionProcessor::new(wallet_pubkey);

    let mut processing_times = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    println!("\n PERFORMANCE PROFILING");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Processing {} transactions...", signatures.len());

    for (i, signature) in signatures.iter().enumerate() {
        let step_start = Instant::now();

        match processor.process_transaction(signature).await {
            Ok(_) => {
                success_count += 1;
                let duration = step_start.elapsed();
                processing_times.push(duration.as_millis() as f64);

                if (i + 1) % 10 == 0 {
                    println!("Processed {}/{} transactions...", i + 1, signatures.len());
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }

    let total_duration = start_time.elapsed();

    let profile = PerformanceProfile {
        total_transactions: signatures.len(),
        successful_transactions: success_count,
        failed_transactions: error_count,
        total_duration_ms: total_duration.as_millis() as f64,
        average_processing_time_ms: if !processing_times.is_empty() {
            processing_times.iter().sum::<f64>() / (processing_times.len() as f64)
        } else {
            0.0
        },
        min_processing_time_ms: processing_times
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min),
        max_processing_time_ms: processing_times.iter().cloned().fold(0.0, f64::max),
        transactions_per_second: if total_duration.as_secs_f64() > 0.0 {
            (signatures.len() as f64) / total_duration.as_secs_f64()
        } else {
            0.0
        },
    };

    print_performance_profile(&profile);
    Ok(profile)
}

/// Performance profile results
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub total_duration_ms: f64,
    pub average_processing_time_ms: f64,
    pub min_processing_time_ms: f64,
    pub max_processing_time_ms: f64,
    pub transactions_per_second: f64,
}

/// Print performance profile results
fn print_performance_profile(profile: &PerformanceProfile) {
    println!("\n PERFORMANCE PROFILE RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Total Transactions: {}", profile.total_transactions);
    println!(
        "Successful: {} ({:.1}%)",
        profile.successful_transactions,
        ((profile.successful_transactions as f64) / (profile.total_transactions as f64)) * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        profile.failed_transactions,
        ((profile.failed_transactions as f64) / (profile.total_transactions as f64)) * 100.0
    );
    println!();
    println!("Total Duration: {:.1}ms", profile.total_duration_ms);
    println!(
        "Average Processing Time: {:.1}ms",
        profile.average_processing_time_ms
    );
    println!(
        "Min Processing Time: {:.1}ms",
        profile.min_processing_time_ms
    );
    println!(
        "Max Processing Time: {:.1}ms",
        profile.max_processing_time_ms
    );
    println!(
        "Throughput: {:.1} transactions/second",
        profile.transactions_per_second
    );
    println!("═══════════════════════════════════════════════════════════════");
}
