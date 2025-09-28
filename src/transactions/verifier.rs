// Transaction verification for positions integration
//
// This module provides transaction verification functionality specifically
// designed for integration with the positions system to verify entry and exit transactions.

use chrono::{DateTime, Utc};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::logger::{log, LogTag};
use crate::transactions::{processor::TransactionProcessor, types::*, utils::*};

// =============================================================================
// VERIFICATION RESULT STRUCTURES
// =============================================================================

/// Result of transaction verification for positions
#[derive(Debug, Clone)]
pub struct TransactionVerificationResult {
    pub verified: bool,
    pub transaction: Option<Transaction>,
    pub verification_type: VerificationType,
    pub confidence_score: f64,
    pub issues: Vec<VerificationIssue>,
    pub verification_timestamp: DateTime<Utc>,
}

/// Type of verification performed
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationType {
    EntryTransaction,
    ExitTransaction,
    GeneralVerification,
}

/// Issues found during verification
#[derive(Debug, Clone)]
pub struct VerificationIssue {
    pub issue_type: IssueType,
    pub description: String,
    pub severity: IssueSeverity,
}

/// Types of verification issues
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IssueType {
    TransactionNotFound,
    TransactionFailed,
    UnexpectedTransactionType,
    InsufficientData,
    TimestampMismatch,
    AmountMismatch,
    TokenMismatch,
    DirectionMismatch,
    HighSlippage,
    SuspiciousPattern,
}

/// Severity of verification issues
#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

// =============================================================================
// MAIN VERIFICATION FUNCTIONS
// =============================================================================

/// Verify transaction for position integration
pub async fn verify_transaction_for_position(
    signature: &str,
    expected_type: TransactionType,
    wallet_pubkey: Pubkey,
) -> Result<TransactionVerificationResult, String> {
    let processor = TransactionProcessor::new(wallet_pubkey);

    log(
        LogTag::Transactions,
        "VERIFY",
        &format!(
            "Verifying transaction {} for position (expected: {:?})",
            signature,
            expected_type
        ),
    );

    // Step 1: Process transaction to get full details
    let transaction = match processor.process_transaction(signature).await {
        Ok(tx) => tx,
        Err(e) => {
            return Ok(TransactionVerificationResult {
                verified: false,
                transaction: None,
                verification_type: VerificationType::GeneralVerification,
                confidence_score: 0.0,
                issues: vec![VerificationIssue {
                    issue_type: if e.contains("not found") {
                        IssueType::TransactionNotFound
                    } else {
                        IssueType::InsufficientData
                    },
                    description: e,
                    severity: IssueSeverity::Critical,
                }],
                verification_timestamp: Utc::now(),
            });
        }
    };

    // Step 2: Perform comprehensive verification
    let verification_result =
        perform_comprehensive_verification(&transaction, expected_type, wallet_pubkey).await?;

    log(
        LogTag::Transactions,
        "VERIFY_RESULT",
        &format!(
            "Verification complete for {}: verified={}, confidence={:.2}, issues={}",
            signature,
            verification_result.verified,
            verification_result.confidence_score,
            verification_result.issues.len()
        ),
    );

    Ok(verification_result)
}

/// Verify entry transaction for position opening
pub async fn verify_entry_transaction(
    signature: &str,
    expected_mint: &str,
    expected_amount_range: Option<(f64, f64)>,
    wallet_pubkey: Pubkey,
) -> Result<TransactionVerificationResult, String> {
    let mut result =
        verify_transaction_for_position(signature, TransactionType::Buy, wallet_pubkey).await?;

    result.verification_type = VerificationType::EntryTransaction;

    // Additional entry-specific verification
    if let Some(ref transaction) = result.transaction {
        // Verify token mint matches expected
        if let Some(ref swap_info) = transaction.token_swap_info {
            if swap_info.output_mint != expected_mint {
                result.issues.push(VerificationIssue {
                    issue_type: IssueType::TokenMismatch,
                    description: format!(
                        "Expected mint {} but found {}",
                        expected_mint, swap_info.output_mint
                    ),
                    severity: IssueSeverity::Critical,
                });
                result.verified = false;
            }
        }

        // Verify amount is within expected range
        if let (Some(swap_info), Some((min_amount, max_amount))) =
            (&transaction.token_swap_info, expected_amount_range)
        {
            if swap_info.output_ui_amount < min_amount || swap_info.output_ui_amount > max_amount {
                result.issues.push(VerificationIssue {
                    issue_type: IssueType::AmountMismatch,
                    description: format!(
                        "Amount {:.6} outside expected range {:.6}-{:.6}",
                        swap_info.output_ui_amount, min_amount, max_amount
                    ),
                    severity: IssueSeverity::Warning,
                });
            }
        }
    }

    Ok(result)
}

/// Verify exit transaction for position closing
pub async fn verify_exit_transaction(
    signature: &str,
    expected_mint: &str,
    expected_amount_range: Option<(f64, f64)>,
    wallet_pubkey: Pubkey,
) -> Result<TransactionVerificationResult, String> {
    let mut result =
        verify_transaction_for_position(signature, TransactionType::Sell, wallet_pubkey).await?;

    result.verification_type = VerificationType::ExitTransaction;

    // Additional exit-specific verification
    if let Some(ref transaction) = result.transaction {
        // Verify token mint matches expected
        if let Some(ref swap_info) = transaction.token_swap_info {
            if swap_info.input_mint != expected_mint {
                result.issues.push(VerificationIssue {
                    issue_type: IssueType::TokenMismatch,
                    description: format!(
                        "Expected mint {} but found {}",
                        expected_mint, swap_info.input_mint
                    ),
                    severity: IssueSeverity::Critical,
                });
                result.verified = false;
            }
        }

        // Verify amount is within expected range
        if let (Some(swap_info), Some((min_amount, max_amount))) =
            (&transaction.token_swap_info, expected_amount_range)
        {
            if swap_info.input_ui_amount < min_amount || swap_info.input_ui_amount > max_amount {
                result.issues.push(VerificationIssue {
                    issue_type: IssueType::AmountMismatch,
                    description: format!(
                        "Amount {:.6} outside expected range {:.6}-{:.6}",
                        swap_info.input_ui_amount, min_amount, max_amount
                    ),
                    severity: IssueSeverity::Warning,
                });
            }
        }
    }

    Ok(result)
}

// =============================================================================
// COMPREHENSIVE VERIFICATION
// =============================================================================

/// Perform comprehensive verification of transaction
async fn perform_comprehensive_verification(
    transaction: &Transaction,
    expected_type: TransactionType,
    wallet_pubkey: Pubkey,
) -> Result<TransactionVerificationResult, String> {
    let mut issues = Vec::new();
    let mut confidence_score = 1.0;

    // Check 1: Transaction success
    if !transaction.success {
        issues.push(VerificationIssue {
            issue_type: IssueType::TransactionFailed,
            description: transaction
                .error_message
                .clone()
                .unwrap_or_else(|| "Transaction failed without specific error".to_string()),
            severity: IssueSeverity::Critical,
        });
        confidence_score = 0.0;
    }

    // Check 2: Transaction type matches expected
    if !transaction_types_compatible(&transaction.transaction_type, &expected_type) {
        issues.push(VerificationIssue {
            issue_type: IssueType::UnexpectedTransactionType,
            description: format!(
                "Expected {:?} but found {:?}",
                expected_type, transaction.transaction_type
            ),
            severity: IssueSeverity::Critical,
        });
        confidence_score *= 0.3;
    }

    // Check 3: Transaction has sufficient data
    if transaction.token_swap_info.is_none()
        && matches!(expected_type, TransactionType::Buy | TransactionType::Sell)
    {
        issues.push(VerificationIssue {
            issue_type: IssueType::InsufficientData,
            description: "Missing swap information for buy/sell transaction".to_string(),
            severity: IssueSeverity::Warning,
        });
        confidence_score *= 0.7;
    }

    // Check 4: Transaction age (not too old)
    if let Some(block_time) = transaction.block_time {
        let age_hours = (Utc::now().timestamp() - block_time) / 3600;
        if age_hours > 168 {
            // More than 1 week old
            issues.push(VerificationIssue {
                issue_type: IssueType::TimestampMismatch,
                description: format!("Transaction is {} hours old", age_hours),
                severity: IssueSeverity::Info,
            });
            confidence_score *= 0.9;
        }
    }

    // Check 5: Analyze for suspicious patterns
    let suspicious_patterns = analyze_suspicious_patterns(transaction).await?;
    for pattern in suspicious_patterns {
        issues.push(VerificationIssue {
            issue_type: IssueType::SuspiciousPattern,
            description: format!("Suspicious pattern detected: {}", pattern),
            severity: IssueSeverity::Warning,
        });
        confidence_score *= 0.8;
    }

    // Check 6: High slippage detection
    if let Some(ref swap_info) = transaction.token_swap_info {
        if let Some(slippage) = calculate_slippage_estimate(swap_info).await? {
            if slippage > 0.05 {
                // More than 5% slippage
                issues.push(VerificationIssue {
                    issue_type: IssueType::HighSlippage,
                    description: format!("High slippage detected: {:.2}%", slippage * 100.0),
                    severity: IssueSeverity::Warning,
                });
                confidence_score *= 0.9;
            }
        }
    }

    // Determine if verification passed
    let verified =
        confidence_score > 0.5 && !issues.iter().any(|i| i.severity == IssueSeverity::Critical);

    Ok(TransactionVerificationResult {
        verified,
        transaction: Some(transaction.clone()),
        verification_type: VerificationType::GeneralVerification,
        confidence_score,
        issues,
        verification_timestamp: Utc::now(),
    })
}

// =============================================================================
// VERIFICATION HELPERS
// =============================================================================

/// Check if transaction types are compatible
fn transaction_types_compatible(actual: &TransactionType, expected: &TransactionType) -> bool {
    match (actual, expected) {
        // Exact matches
        (a, e) if a == e => true,

        // Unknown is compatible with anything (low confidence)
        (TransactionType::Unknown, _) => true,
        (_, TransactionType::Unknown) => true,

        // Buy/Sell variations
        (TransactionType::Buy, TransactionType::Sell) => false,
        (TransactionType::Sell, TransactionType::Buy) => false,

        // Other cases
        _ => false,
    }
}

/// Analyze transaction for suspicious patterns
async fn analyze_suspicious_patterns(transaction: &Transaction) -> Result<Vec<String>, String> {
    let mut patterns = Vec::new();

    // Pattern 1: Failed transaction with high fee
    if !transaction.success {
        if let Some(fee) = transaction.fee_lamports {
            if fee > 100_000 {
                // > 0.0001 SOL
                patterns.push("failed_high_fee".to_string());
            }
        }
    }

    // Pattern 2: Very high fee for transaction
    if let Some(fee) = transaction.fee_lamports {
        if fee > 1_000_000 {
            // > 0.001 SOL
            patterns.push("unusually_high_fee".to_string());
        }
    }

    // Pattern 3: Complex transaction (many instructions) for simple swap
    if transaction.instructions_count > 20 {
        patterns.push("complex_for_swap".to_string());
    }

    // Pattern 4: Very recent transaction (might be pending)
    if let Some(block_time) = transaction.block_time {
        let age_minutes = (Utc::now().timestamp() - block_time) / 60;
        if age_minutes < 1 {
            patterns.push("very_recent".to_string());
        }
    }

    Ok(patterns)
}

/// Calculate estimated slippage for swap transaction
async fn calculate_slippage_estimate(swap_info: &TokenSwapInfo) -> Result<Option<f64>, String> {
    // This would calculate slippage based on expected vs actual amounts
    // For now, return None as placeholder - would require price oracle integration
    Ok(None)
}

// =============================================================================
// BATCH VERIFICATION
// =============================================================================

/// Verify multiple transactions in batch
pub async fn verify_transactions_batch(
    verifications: Vec<(String, TransactionType)>,
    wallet_pubkey: Pubkey,
) -> HashMap<String, TransactionVerificationResult> {
    let mut results = HashMap::new();

    log(
        LogTag::Transactions,
        "BATCH_VERIFY",
        &format!(
            "Starting batch verification of {} transactions",
            verifications.len()
        ),
    );

    // Process verifications concurrently
    let tasks: Vec<_> = verifications
        .into_iter()
        .map(|(signature, expected_type)| {
            let sig_clone = signature.clone();
            async move {
                let result =
                    verify_transaction_for_position(&sig_clone, expected_type, wallet_pubkey).await;
                (sig_clone, result)
            }
        })
        .collect();

    let batch_results = futures::future::join_all(tasks).await;

    let mut success_count = 0;
    for (signature, result) in batch_results {
        match result {
            Ok(verification_result) => {
                if verification_result.verified {
                    success_count += 1;
                }
                results.insert(signature, verification_result);
            }
            Err(e) => {
                results.insert(
                    signature.clone(),
                    TransactionVerificationResult {
                        verified: false,
                        transaction: None,
                        verification_type: VerificationType::GeneralVerification,
                        confidence_score: 0.0,
                        issues: vec![VerificationIssue {
                            issue_type: IssueType::InsufficientData,
                            description: e,
                            severity: IssueSeverity::Critical,
                        }],
                        verification_timestamp: Utc::now(),
                    },
                );
            }
        }
    }

    log(
        LogTag::Transactions,
        "BATCH_VERIFY_COMPLETE",
        &format!(
            "Batch verification complete: {}/{} verified successfully",
            success_count,
            results.len()
        ),
    );

    results
}

// =============================================================================
// VERIFICATION REPORTING
// =============================================================================

/// Generate verification summary report
pub fn generate_verification_report(
    results: &HashMap<String, TransactionVerificationResult>,
) -> VerificationReport {
    let total_verifications = results.len();
    let successful_verifications = results.values().filter(|r| r.verified).count();
    let failed_verifications = total_verifications - successful_verifications;

    let mut issue_counts = HashMap::new();
    for result in results.values() {
        for issue in &result.issues {
            *issue_counts.entry(issue.issue_type.clone()).or_insert(0) += 1;
        }
    }

    let average_confidence = if total_verifications > 0 {
        results.values().map(|r| r.confidence_score).sum::<f64>() / (total_verifications as f64)
    } else {
        0.0
    };

    VerificationReport {
        total_verifications,
        successful_verifications,
        failed_verifications,
        average_confidence_score: average_confidence,
        issue_counts,
        report_timestamp: Utc::now(),
    }
}

/// Verification summary report
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub total_verifications: usize,
    pub successful_verifications: usize,
    pub failed_verifications: usize,
    pub average_confidence_score: f64,
    pub issue_counts: HashMap<IssueType, usize>,
    pub report_timestamp: DateTime<Utc>,
}

impl VerificationReport {
    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_verifications == 0 {
            100.0
        } else {
            ((self.successful_verifications as f64) / (self.total_verifications as f64)) * 100.0
        }
    }

    /// Get most common issue type
    pub fn most_common_issue(&self) -> Option<IssueType> {
        self.issue_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(issue_type, _)| issue_type.clone())
    }
}

// =============================================================================
// LEGACY COMPATIBILITY
// =============================================================================

/// Legacy verification function for compatibility during migration
pub async fn verify_transaction_legacy(
    signature: &str,
    wallet_pubkey: Pubkey,
) -> Result<bool, String> {
    log(
        LogTag::Transactions,
        "WARN",
        "Using legacy verification function - please migrate to new verification API",
    );

    let result =
        verify_transaction_for_position(signature, TransactionType::Unknown, wallet_pubkey).await?;

    Ok(result.verified)
}
