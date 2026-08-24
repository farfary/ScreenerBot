//! Transaction debug utilities — detailed logging and inspection for troubleshooting.
//
// Debug utilities and diagnostics for the transactions module
//
// This module provides comprehensive debugging tools, diagnostics, and troubleshooting
// utilities for the transactions system.
//
// Helper functions (validations, output formatting, database debug, performance profiling)
// live in the sibling `debug_helpers` module and are re-exported here.

use chrono::{DateTime, Utc};
use std::time::Instant;
use tabled::Tabled;

use crate::chains::adapter;
use crate::chains::solana::transactions::processor::TransactionProcessor;
use crate::logger::{self, LogTag};
use crate::transactions::error::Error;
use crate::transactions::types::*;

use super::debug_helpers::{perform_debug_validations, print_debug_analysis};

// Re-export helper functions and types for public API continuity
pub use super::debug_helpers::{
    debug_database_connection, print_debug_statistics, profile_transaction_processing,
    PerformanceProfile,
};

// =============================================================================
// DEBUG STRUCTURES
// =============================================================================

/// Debug information for a transaction
#[derive(Debug, Clone, Tabled)]
pub struct TransactionDebugInfo {
    #[tabled(rename = "Signature")]
    pub signature_short: String,
    #[tabled(rename = "Type")]
    pub transaction_type: String,
    #[tabled(rename = "Direction")]
    pub direction: String,
    #[tabled(rename = "Success")]
    pub success: String,
    #[tabled(rename = "Fee (SOL)")]
    pub fee_sol: String,
    #[tabled(rename = "Age")]
    pub age: String,
    #[tabled(rename = "Instructions")]
    pub instructions_count: usize,
    #[tabled(rename = "Analysis Time")]
    pub analysis_duration: String,
}

/// Debug statistics for transaction processing
#[derive(Debug, Clone)]
pub struct TransactionDebugStats {
    pub total_processed: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub swap_transactions: usize,
    pub transfer_transactions: usize,
    pub unknown_transactions: usize,
    pub average_processing_time_ms: f64,
    pub total_fees_sol: f64,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Debug analysis result
#[derive(Debug, Clone)]
pub struct DebugAnalysisResult {
    pub transaction: Transaction,
    pub debug_info: TransactionDebugInfo,
    pub analysis_steps: Vec<DebugStep>,
    pub performance_metrics: DebugPerformanceMetrics,
    pub validation_results: Vec<DebugValidation>,
}

/// Individual debug step information
#[derive(Debug, Clone)]
pub struct DebugStep {
    pub step_name: String,
    pub duration_ms: u64,
    pub success: bool,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

/// Performance metrics for debugging
#[derive(Debug, Clone)]
pub struct DebugPerformanceMetrics {
    pub total_duration_ms: u64,
    pub rpc_fetch_ms: Option<u64>,
    pub analysis_ms: Option<u64>,
    pub database_ms: Option<u64>,
    pub memory_usage_mb: Option<f64>,
}

/// Debug validation result
#[derive(Debug, Clone)]
pub struct DebugValidation {
    pub validation_name: String,
    pub passed: bool,
    pub message: String,
    pub severity: String,
}

// =============================================================================
// MAIN DEBUG FUNCTIONS
// =============================================================================

/// Debug a single transaction with comprehensive analysis
pub async fn debug_transaction(
    signature: &str,
    subject: &Subject,
    verbose: bool,
) -> Result<DebugAnalysisResult, Error> {
    logger::info(
        LogTag::Transactions,
        &format!("Starting debug analysis for transaction: {signature}"),
    );

    let start_time = Instant::now();
    let mut analysis_steps = Vec::new();
    let mut validation_results = Vec::new();

    // Step 1: Process transaction
    let step_start = Instant::now();
    let processor =
        TransactionProcessor::for_subject(subject).map_err(|e| Error::VerificationFailed {
            signature: signature.to_owned(),
            detail: e.to_string(),
        })?;
    let transaction = processor
        .process_transaction(signature)
        .await
        .map_err(|e| Error::VerificationFailed {
            signature: signature.to_owned(),
            detail: e.to_string(),
        })?;
    let processing_duration = step_start.elapsed();

    analysis_steps.push(DebugStep {
        step_name: "Transaction Processing".to_owned(),
        duration_ms: processing_duration.as_millis() as u64,
        success: true,
        details: format!("Processed transaction: {:?}", transaction.transaction_type),
        timestamp: Utc::now(),
    });

    // Step 2: Create debug info
    let debug_info = create_debug_info(&transaction);

    // Step 3: Perform validations
    let validation_start = Instant::now();
    validation_results.extend(perform_debug_validations(&transaction).await?);
    let validation_duration = validation_start.elapsed();

    analysis_steps.push(DebugStep {
        step_name: "Debug Validations".to_owned(),
        duration_ms: validation_duration.as_millis() as u64,
        success: true,
        details: format!("Performed {} validations", validation_results.len()),
        timestamp: Utc::now(),
    });

    // Step 4: Create performance metrics
    let total_duration = start_time.elapsed();
    let performance_metrics = DebugPerformanceMetrics {
        total_duration_ms: total_duration.as_millis() as u64,
        rpc_fetch_ms: None, // Would be tracked in actual implementation
        analysis_ms: transaction.analysis_duration_ms,
        database_ms: None,     // Would be tracked in actual implementation
        memory_usage_mb: None, // Would be tracked in actual implementation
    };

    let result = DebugAnalysisResult {
        transaction,
        debug_info,
        analysis_steps,
        performance_metrics,
        validation_results,
    };

    if verbose {
        print_debug_analysis(&result);
    }

    logger::info(
        LogTag::Transactions,
        &format!(
            "Debug analysis complete for {}: {} steps, {} validations, {}ms total",
            signature,
            result.analysis_steps.len(),
            result.validation_results.len(),
            result.performance_metrics.total_duration_ms
        ),
    );

    Ok(result)
}

/// Debug multiple transactions and generate summary
pub async fn debug_transactions_batch(
    signatures: Vec<String>,
    subject: &Subject,
) -> Result<Vec<DebugAnalysisResult>, Error> {
    logger::info(
        LogTag::Transactions,
        &format!(
            "Starting batch debug analysis for {} transactions",
            signatures.len()
        ),
    );

    let start_time = Instant::now();
    let mut results = Vec::new();

    // Process transactions concurrently
    let tasks: Vec<_> = signatures
        .into_iter()
        .map(|signature| async move { debug_transaction(&signature, subject, false).await })
        .collect();

    let batch_results = futures::future::join_all(tasks).await;

    let mut success_count = 0;
    for result in batch_results {
        match result {
            Ok(debug_result) => {
                success_count += 1;
                results.push(debug_result);
            }
            Err(e) => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to debug transaction: {e}"),
                );
            }
        }
    }

    let total_duration = start_time.elapsed();

    logger::info(
        LogTag::Transactions,
        &format!(
            "Batch debug complete: {}/{} successful in {}ms (avg: {}ms/tx)",
            success_count,
            results.len(),
            total_duration.as_millis(),
            if !results.is_empty() {
                total_duration.as_millis() / (results.len() as u128)
            } else {
                0
            }
        ),
    );

    Ok(results)
}

/// Generate debug statistics from transaction list
pub async fn generate_debug_statistics(transactions: &[Transaction]) -> TransactionDebugStats {
    let total_processed = transactions.len();
    let successful_transactions = transactions.iter().filter(|tx| tx.success).count();
    let failed_transactions = total_processed - successful_transactions;

    let swap_transactions = transactions
        .iter()
        .filter(|tx| {
            matches!(
                tx.transaction_type,
                TransactionType::Buy | TransactionType::Sell
            )
        })
        .count();

    let transfer_transactions = transactions
        .iter()
        .filter(|tx| matches!(tx.transaction_type, TransactionType::Transfer))
        .count();

    let unknown_transactions = transactions
        .iter()
        .filter(|tx| matches!(tx.transaction_type, TransactionType::Unknown))
        .count();

    let average_processing_time_ms = if total_processed > 0 {
        transactions
            .iter()
            .filter_map(|tx| tx.analysis_duration_ms)
            .map(|d| d as f64)
            .sum::<f64>()
            / (total_processed as f64)
    } else {
        0.0
    };

    let total_fees_sol = transactions
        .iter()
        .filter_map(|tx| tx.fee_lamports)
        .map(|fee| adapter().raw_to_native(fee))
        .sum();

    let date_range = if !transactions.is_empty() {
        let timestamps: Vec<_> = transactions.iter().map(|tx| tx.timestamp).collect();
        let min_time = timestamps.iter().min().copied();
        let max_time = timestamps.iter().max().copied();
        min_time.zip(max_time)
    } else {
        None
    };

    TransactionDebugStats {
        total_processed,
        successful_transactions,
        failed_transactions,
        swap_transactions,
        transfer_transactions,
        unknown_transactions,
        average_processing_time_ms,
        total_fees_sol,
        date_range,
    }
}

// =============================================================================
// DEBUG INFO CREATION
// =============================================================================

/// Create debug info structure from transaction
fn create_debug_info(transaction: &Transaction) -> TransactionDebugInfo {
    let age = if let Some(block_time) = transaction.block_time {
        let age_seconds = Utc::now().timestamp() - block_time;
        if age_seconds < 60 {
            format!("{age_seconds}s")
        } else if age_seconds < 3600 {
            format!("{}m", age_seconds / 60)
        } else if age_seconds < 86400 {
            format!("{}h", age_seconds / 3600)
        } else {
            format!("{}d", age_seconds / 86400)
        }
    } else {
        "Unknown".to_owned()
    };

    let fee_sol = transaction
        .fee_lamports
        .map(|f| format!("{:.6}", adapter().raw_to_native(f)))
        .unwrap_or_else(|| "Unknown".to_owned());

    let analysis_duration = transaction
        .analysis_duration_ms
        .map(|d| format!("{d}ms"))
        .unwrap_or_else(|| "N/A".to_owned());

    TransactionDebugInfo {
        signature_short: transaction.signature.clone(),
        transaction_type: format!("{:?}", transaction.transaction_type),
        direction: format!("{:?}", transaction.direction),
        success: (if transaction.success { "" } else { "" }).to_string(),
        fee_sol,
        age,
        instructions_count: transaction.instructions_count,
        analysis_duration,
    }
}
