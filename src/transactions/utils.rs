//! Transaction utilities — helper functions for signature formatting and data conversion.
//
// Utility functions and constants for the transactions module
//
// This module provides shared utility functions, constants, and helper code
// used throughout the transactions system.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
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
// GLOBAL STATE MANAGEMENT
// =============================================================================

/// Global known signatures cache — bounded moka cache with LRU eviction (max 50K entries).
/// Prevents unbounded growth (was ~2 MB/day with HashSet).
/// Keyed by `"<chain>:<subject address>:<signature>"` — see `subject_key`.
static GLOBAL_KNOWN_SIGNATURES: LazyLock<moka::sync::Cache<String, ()>> =
    LazyLock::new(|| moka::sync::Cache::builder().max_capacity(50_000).build());

/// Global pending transactions tracking.
/// Keyed by `"<chain>:<subject address>:<signature>"` — see `subject_key`.
static GLOBAL_PENDING_TRANSACTIONS: LazyLock<Arc<Mutex<HashMap<String, DateTime<Utc>>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Cache key for a subject's signature. The caches are process-wide and shared by
/// every watched wallet, so the subject has to be part of the key -- otherwise one
/// wallet's signature would mark another wallet's dedupe entry as already seen.
fn subject_key(subject: &Subject, signature: &str) -> String {
    format!("{}:{}:{}", subject.chain(), subject.address(), signature)
}

/// Check if signature is known globally across all managers, for the given subject
pub async fn is_signature_known_globally(subject: Subject, signature: &str) -> bool {
    GLOBAL_KNOWN_SIGNATURES.contains_key(&subject_key(&subject, signature))
}

/// Add signature to global known cache, for the given subject
pub async fn add_signature_to_known_globally(subject: Subject, signature: String) {
    let key = subject_key(&subject, &signature);
    let was_new = GLOBAL_KNOWN_SIGNATURES.get(&key).is_none();
    GLOBAL_KNOWN_SIGNATURES.insert(key, ());
    if was_new {
        logger::debug(
            LogTag::Transactions,
            &format!(
                "Added signature {} to known cache for {} (total={})",
                &signature,
                subject,
                GLOBAL_KNOWN_SIGNATURES.entry_count()
            ),
        );
    }
}

/// Remove signature from global known cache, for the given subject
pub async fn remove_signature_from_known_globally(subject: Subject, signature: &str) {
    GLOBAL_KNOWN_SIGNATURES.invalidate(&subject_key(&subject, signature));
}

/// Get count of globally known signatures (across all subjects)
pub async fn get_known_signatures_count() -> usize {
    GLOBAL_KNOWN_SIGNATURES.entry_count() as usize
}

/// Clear global known signatures cache (across all subjects)
pub async fn clear_global_known_signatures() {
    GLOBAL_KNOWN_SIGNATURES.invalidate_all();
    logger::info(
        LogTag::Transactions,
        "Cleared global known signatures cache",
    );
}

/// Add pending transaction globally, for the given subject
pub async fn add_pending_transaction_globally(
    subject: Subject,
    signature: String,
    timestamp: DateTime<Utc>,
) {
    let mut pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.insert(subject_key(&subject, &signature), timestamp);
}

/// Remove pending transaction globally, for the given subject
pub async fn remove_pending_transaction_globally(subject: Subject, signature: &str) {
    let mut pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
    pending.remove(&subject_key(&subject, signature));
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
            &format!("Cleaned up {expired_count} expired pending transactions"),
        );
    }

    expired_count
}

// =============================================================================
// VALIDATION UTILITIES
// =============================================================================

/// Check if an asset address denotes the chain's native asset.
pub fn is_wsol_mint(mint: &str) -> bool {
    crate::chains::adapter().is_native_asset(mint)
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
            &format!("Operation '{operation}' completed in {ms}ms"),
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

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Two subjects no other test can be using.
    ///
    /// These tests share one process-global signature map and cargo runs them in
    /// PARALLEL, so `clear_global_known_signatures()` inside a test wipes whatever a
    /// concurrently running sibling just inserted — an intermittent failure that has
    /// nothing to do with the behaviour under test. Unique subjects already isolate
    /// each test's entries, so no test here clears the global; each cleans up after
    /// itself instead.
    fn subjects() -> (Subject, Subject) {
        (
            crate::chains::solana::transactions::subject::from_pubkey(
                crate::chains::solana::solana_sdk::pubkey::Pubkey::new_unique(),
            ),
            crate::chains::solana::transactions::subject::from_pubkey(
                crate::chains::solana::solana_sdk::pubkey::Pubkey::new_unique(),
            ),
        )
    }

    #[tokio::test]
    async fn known_signature_is_scoped_to_its_subject() {
        let (subject_a, subject_b) = subjects();

        add_signature_to_known_globally(subject_a.clone(), "sigA1".to_owned()).await;

        assert!(is_signature_known_globally(subject_a.clone(), "sigA1").await);
        assert!(!is_signature_known_globally(subject_b, "sigA1").await);

        remove_signature_from_known_globally(subject_a, "sigA1").await;
    }

    #[tokio::test]
    async fn removing_a_signature_for_one_subject_does_not_affect_another() {
        let (subject_a, subject_b) = subjects();

        add_signature_to_known_globally(subject_a.clone(), "sigA2".to_owned()).await;
        add_signature_to_known_globally(subject_b.clone(), "sigA2".to_owned()).await;

        remove_signature_from_known_globally(subject_a.clone(), "sigA2").await;

        assert!(!is_signature_known_globally(subject_a, "sigA2").await);
        assert!(is_signature_known_globally(subject_b.clone(), "sigA2").await);

        remove_signature_from_known_globally(subject_b, "sigA2").await;
    }

    #[tokio::test]
    async fn pending_transaction_removal_is_scoped_to_its_subject() {
        let (subject_a, subject_b) = subjects();
        let now = Utc::now();

        add_pending_transaction_globally(subject_a.clone(), "sigB1".to_owned(), now).await;
        add_pending_transaction_globally(subject_b.clone(), "sigB1".to_owned(), now).await;

        // Removing subject B's pending entry must not remove subject A's.
        remove_pending_transaction_globally(subject_b.clone(), "sigB1").await;

        let pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
        assert!(pending.contains_key(&subject_key(&subject_a, "sigB1")));
        assert!(!pending.contains_key(&subject_key(&subject_b, "sigB1")));
        drop(pending);

        // Clean up so later tests in this process are not affected.
        remove_pending_transaction_globally(subject_a, "sigB1").await;
    }

    #[tokio::test]
    async fn two_subjects_pending_on_the_same_signature_do_not_collide() {
        // Two watched wallets can appear in one transaction, so the same signature
        // can legitimately be pending for both. Before the subject key, the second
        // insert overwrote the first and one of them silently stopped being tracked.
        let (subject_a, subject_b) = subjects();
        let now = Utc::now();

        add_pending_transaction_globally(subject_a.clone(), "sigC1".to_owned(), now).await;
        add_pending_transaction_globally(subject_b.clone(), "sigC1".to_owned(), now).await;

        {
            let pending = GLOBAL_PENDING_TRANSACTIONS.lock().await;
            assert!(pending.contains_key(&subject_key(&subject_a, "sigC1")));
            assert!(pending.contains_key(&subject_key(&subject_b, "sigC1")));
        }

        // Clean up so later tests in this process are not affected.
        remove_pending_transaction_globally(subject_a, "sigC1").await;
        remove_pending_transaction_globally(subject_b, "sigC1").await;
    }
}
