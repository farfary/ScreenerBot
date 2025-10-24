// Utility functions and constants for the transactions module
//
// This module provides shared utility functions, constants, and helper code
// used throughout the transactions system.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::logger::{self, LogTag};
use crate::transactions::types::*;

// =============================================================================
// TIMING AND BATCH CONSTANTS
// =============================================================================

/// Normal transaction checking interval (3 seconds for faster position verification)
pub const NORMAL_CHECK_INTERVAL_SECS: u64 = 3;

/// Minimum pending lamport delta to consider (ignore WebSocket pendings with <0.000001 SOL impact)
pub const MIN_PENDING_LAMPORT_DELTA: i64 = 1_000;

/// Maximum age for pending signatures before dropping (3 minutes without progress)
pub const PENDING_MAX_AGE_SECS: i64 = 180;

/// Transaction signatures fetch batch size
pub const RPC_BATCH_SIZE: usize = 1000;

/// Transaction processing batch size
pub const PROCESS_BATCH_SIZE: usize = 50;

/// Transaction data fetch batch size
pub const TRANSACTION_DATA_BATCH_SIZE: usize = 1;

// =============================================================================
// SOLANA NETWORK CONSTANTS
// =============================================================================

/// Standard ATA creation/closure cost in SOL
pub const ATA_RENT_COST_SOL: f64 = 0.00203928;

/// Tolerance for ATA rent variations (lamports)
pub const ATA_RENT_TOLERANCE_LAMPORTS: i64 = 10000;

/// Default compute unit price (micro-lamports)
pub const DEFAULT_COMPUTE_UNIT_PRICE: u64 = 1000;

/// Wrapped SOL mint address
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// =============================================================================
// GLOBAL STATE MANAGEMENT
// =============================================================================

/// Global known signatures cache for cross-manager coordination
static GLOBAL_KNOWN_SIGNATURES: Lazy<Arc<Mutex<HashSet<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

/// Global pending transactions tracking
static GLOBAL_PENDING_TRANSACTIONS: Lazy<Arc<Mutex<HashMap<String, DateTime<Utc>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if signature is known globally across all managers
pub async fn is_signature_known_globally(signature: &str) -> bool {
    let known_sigs = GLOBAL_KNOWN_SIGNATURES.lock().await;
    known_sigs.contains(signature)
}

/// Add signature to global known cache
pub async fn add_signature_to_known_globally(signature: String) {
    let mut known_sigs = GLOBAL_KNOWN_SIGNATURES.lock().await;
    let inserted = known_sigs.insert(signature.clone());
    if inserted {
        logger::debug(
            LogTag::Transactions,
            &format!(
                "Added signature {} to known cache (total={})",
                &signature,
                known_sigs.len()
            ),
        );
    }
}

/// Remove signature from global known cache
pub async fn remove_signature_from_known_globally(signature: &str) {
    let mut known_sigs = GLOBAL_KNOWN_SIGNATURES.lock().await;
    known_sigs.remove(signature);
}

/// Get count of globally known signatures
pub async fn get_known_signatures_count() -> usize {
    let known_sigs = GLOBAL_KNOWN_SIGNATURES.lock().await;
    known_sigs.len()
}

/// Clear global known signatures cache
pub async fn clear_global_known_signatures() {
    let mut known_sigs = GLOBAL_KNOWN_SIGNATURES.lock().await;
    known_sigs.clear();
    logger::info(
        LogTag::Transactions,
        "Cleared global known signatures cache",
    );
}

/// Add pending transaction globally
pub async fn add_pending_transaction_globally(signature: String, timestamp: DateTime<Utc>) {
    let mut pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.insert(signature, timestamp);
}

/// Remove pending transaction globally
pub async fn remove_pending_transaction_globally(signature: &str) {
    let mut pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.remove(signature);
}

/// Get count of pending transactions
pub async fn get_pending_transactions_count() -> usize {
    let pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.len()
}

/// Clean up expired pending transactions
pub async fn cleanup_expired_pending_transactions() -> usize {
    let mut pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    let now = Utc::now();
    let mut expired_count = 0;

    pending.retain(|signature, timestamp| {
        let age_secs = (now - *timestamp).num_seconds();
        if age_secs > PENDING_MAX_AGE_SECS {
            logger::debug(
                LogTag::Transactions,
                &format!(
                    "Expired pending transaction: {} (age: {}s)",
                    &signature[..8],
                    age_secs
                ),
            );
            expired_count += 1;
            false
        } else {
            true
        }
    });

    if expired_count > 0 {
        logger::info(
            LogTag::Transactions,
            &format!("Cleaned up {} expired pending transactions", expired_count),
        );
    }

    expired_count
}

/// Get list of pending transaction signatures
pub async fn get_pending_transaction_signatures() -> Vec<String> {
    let pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.keys().cloned().collect()
}

// =============================================================================
// STRING AND FORMATTING UTILITIES
// =============================================================================

/// Format lamports as SOL with appropriate precision
pub fn format_lamports_as_sol(lamports: u64) -> String {
    let sol = (lamports as f64) / 1e9;
    if sol < 0.001 {
        format!("{:.6} SOL", sol)
    } else {
        format!("{:.3} SOL", sol)
    }
}

/// Format change in lamports as SOL with sign
pub fn format_lamports_change_as_sol(change_lamports: i64) -> String {
    let sol_change = (change_lamports as f64) / 1e9;
    if sol_change >= 0.0 {
        format!("+{:.6} SOL", sol_change)
    } else {
        format!("{:.6} SOL", sol_change)
    }
}

// =============================================================================
// VALIDATION UTILITIES
// =============================================================================

/// Validate that a string is a valid Solana signature
pub fn is_valid_signature(signature: &str) -> bool {
    // Solana signatures are base58 encoded and should be 88 characters long
    signature.len() == 88
        && signature.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == '1'
                || c == '2'
                || c == '3'
                || c == '4'
                || c == '5'
                || c == '6'
                || c == '7'
                || c == '8'
                || c == '9'
                || c == 'A'
                || c == 'B'
                || c == 'C'
                || c == 'D'
                || c == 'E'
                || c == 'F'
                || c == 'G'
                || c == 'H'
                || c == 'J'
                || c == 'K'
                || c == 'L'
                || c == 'M'
                || c == 'N'
                || c == 'P'
                || c == 'Q'
                || c == 'R'
                || c == 'S'
                || c == 'T'
                || c == 'U'
                || c == 'V'
                || c == 'W'
                || c == 'X'
                || c == 'Y'
                || c == 'Z'
                || c == 'a'
                || c == 'b'
                || c == 'c'
                || c == 'd'
                || c == 'e'
                || c == 'f'
                || c == 'g'
                || c == 'h'
                || c == 'i'
                || c == 'j'
                || c == 'k'
                || c == 'm'
                || c == 'n'
                || c == 'o'
                || c == 'p'
                || c == 'q'
                || c == 'r'
                || c == 's'
                || c == 't'
                || c == 'u'
                || c == 'v'
                || c == 'w'
                || c == 'x'
                || c == 'y'
                || c == 'z'
        })
}

/// Validate that a string is a valid Solana pubkey
pub fn is_valid_pubkey(pubkey: &str) -> bool {
    // Solana pubkeys are base58 encoded and should be 32-44 characters long
    pubkey.len() >= 32
        && pubkey.len() <= 44
        && pubkey.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == '1'
                || c == '2'
                || c == '3'
                || c == '4'
                || c == '5'
                || c == '6'
                || c == '7'
                || c == '8'
                || c == '9'
                || c == 'A'
                || c == 'B'
                || c == 'C'
                || c == 'D'
                || c == 'E'
                || c == 'F'
                || c == 'G'
                || c == 'H'
                || c == 'J'
                || c == 'K'
                || c == 'L'
                || c == 'M'
                || c == 'N'
                || c == 'P'
                || c == 'Q'
                || c == 'R'
                || c == 'S'
                || c == 'T'
                || c == 'U'
                || c == 'V'
                || c == 'W'
                || c == 'X'
                || c == 'Y'
                || c == 'Z'
                || c == 'a'
                || c == 'b'
                || c == 'c'
                || c == 'd'
                || c == 'e'
                || c == 'f'
                || c == 'g'
                || c == 'h'
                || c == 'i'
                || c == 'j'
                || c == 'k'
                || c == 'm'
                || c == 'n'
                || c == 'o'
                || c == 'p'
                || c == 'q'
                || c == 'r'
                || c == 's'
                || c == 't'
                || c == 'u'
                || c == 'v'
                || c == 'w'
                || c == 'x'
                || c == 'y'
                || c == 'z'
        })
}

/// Check if mint address is WSOL (wrapped SOL)
pub fn is_wsol_mint(mint: &str) -> bool {
    mint == WSOL_MINT
}

// =============================================================================
// PERFORMANCE UTILITIES
// =============================================================================

/// Create a duration measurement helper
pub struct DurationMeasure {
    start: std::time::Instant,
    operation: String,
}

impl DurationMeasure {
    /// Start measuring an operation
    pub fn start(operation: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            operation: operation.to_string(),
        }
    }

    /// Finish measuring and return duration in milliseconds
    pub fn finish(self) -> u64 {
        let duration = self.start.elapsed();
        let ms = duration.as_millis() as u64;

        if ms > 1000 {
            logger::info(
                LogTag::Transactions,
                &format!("Slow operation '{}': {}ms", self.operation, ms),
            );
        }

        ms
    }

    /// Finish measuring and log the result
    pub fn finish_and_log(self) -> u64 {
        let operation = self.operation.clone();
        let ms = self.finish();
        logger::debug(
            LogTag::Transactions,
            &format!("Operation '{}' completed in {}ms", operation, ms),
        );
        ms
    }
}

// =============================================================================
// BATCH PROCESSING UTILITIES
// =============================================================================

/// Split a vector into chunks of specified size
pub fn chunk_signatures(signatures: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    signatures
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Merge transaction statistics
pub fn merge_transaction_stats(
    stats1: TransactionStats,
    stats2: TransactionStats,
) -> TransactionStats {
    TransactionStats {
        total_transactions: stats1.total_transactions + stats2.total_transactions,
        new_transactions_count: stats1.new_transactions_count + stats2.new_transactions_count,
        known_signatures_count: stats1.known_signatures_count + stats2.known_signatures_count,
        pending_transactions_count: stats1.pending_transactions_count
            + stats2.pending_transactions_count,
        failed_transactions_count: stats1.failed_transactions_count
            + stats2.failed_transactions_count,
        successful_transactions_count: stats1.successful_transactions_count
            + stats2.successful_transactions_count,
    }
}

// =============================================================================
// ERROR HANDLING UTILITIES
// =============================================================================

/// Create a standardized error message for transaction operations
pub fn create_transaction_error(operation: &str, signature: &str, error: &str) -> String {
    format!(
        "Transaction {} failed for {}: {}",
        operation, signature, error
    )
}

/// Create a standardized error message with full address (for debugging)
pub fn create_transaction_error_with_full_address(
    operation: &str,
    signature: &str,
    error: &str,
) -> String {
    format!(
        "Transaction {} failed for {}: {}",
        operation,
        signature, // Full address as required by project guidelines
        error
    )
}
