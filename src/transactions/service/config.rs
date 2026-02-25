//! Transaction service configuration — runtime settings for connection management and retries.
//
// Service configuration, constants, and deferred retry queue

use chrono::{DateTime, Utc};
use std::sync::LazyLock;
use std::time::Duration;

use crate::transactions::types::DeferredRetry;
use crate::transactions::utils::{NORMAL_CHECK_INTERVAL_SECS, RPC_BATCH_SIZE};

// =============================================================================
// BOOTSTRAP CONFIGURATION
// =============================================================================

/// Number of transactions to process concurrently during bootstrap
/// Change this value to adjust parallel processing batch size
pub const CONCURRENT_BATCH_SIZE: usize = 10;

/// Timeout for individual transaction processing during bootstrap (in seconds)
/// Transactions taking longer than this will be timed out and retried later
pub const TRANSACTION_TIMEOUT_SECS: u64 = 15;

/// Maximum retry attempts for failed/timed-out transactions
pub const MAX_RETRY_ATTEMPTS: usize = 3;

/// Base delay between retry attempts (increases exponentially)
pub const RETRY_BASE_DELAY_SECS: u64 = 2;

// =============================================================================
// DEFERRED RETRY QUEUE
// =============================================================================

/// Global deferred retry queue — bounded moka cache (max 1K entries, 5min TTL).
/// Transactions are added here when they fail due to temporary issues like RPC indexing delays.
pub static DEFERRED_RETRIES: LazyLock<moka::sync::Cache<String, DeferredRetry>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(1_000)
            .time_to_live(Duration::from_secs(300))
            .build()
    });

/// Parallel key index for DEFERRED_RETRIES — moka doesn't expose iter().
/// Keys are added on insert and removed on invalidate/get-miss.
pub static DEFERRED_RETRY_KEYS: LazyLock<dashmap::DashSet<String>> =
    LazyLock::new(|| dashmap::DashSet::new());

/// Get all deferred retry keys (for iteration since moka has no iter()).
pub fn get_deferred_retry_keys() -> Vec<String> {
    DEFERRED_RETRY_KEYS.iter().map(|k| k.key().clone()).collect()
}

/// Add a transaction to the deferred retry queue
pub async fn defer_transaction_retry(signature: String, delay_secs: i64, reason: String) {
    let now = Utc::now();

    // Check if already exists, increment attempts if so
    if let Some(mut existing) = DEFERRED_RETRIES.get(&signature) {
        existing.attempts += 1;
        existing.current_delay_secs = delay_secs * (existing.attempts as i64);
        existing.next_retry_at = now + chrono::Duration::seconds(existing.current_delay_secs);
        existing.last_error = Some(reason);
        DEFERRED_RETRIES.insert(signature.clone(), existing);
        DEFERRED_RETRY_KEYS.insert(signature);
    } else {
        let retry = DeferredRetry {
            signature: signature.clone(),
            next_retry_at: now + chrono::Duration::seconds(delay_secs),
            attempts: 1,
            current_delay_secs: delay_secs,
            last_error: Some(reason),
            first_seen: now,
        };
        DEFERRED_RETRIES.insert(signature.clone(), retry);
        DEFERRED_RETRY_KEYS.insert(signature);
    }
}

/// Get count of deferred retries
pub async fn get_deferred_retries_count() -> usize {
    DEFERRED_RETRIES.entry_count() as usize
}

// =============================================================================
// SERVICE CONFIGURATION
// =============================================================================

/// Service configuration structure
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Wallet public key to monitor
    pub wallet_pubkey: solana_sdk::pubkey::Pubkey,
    /// Check interval for transaction monitoring
    pub check_interval_secs: u64,
    /// Enable WebSocket real-time monitoring
    pub enable_websocket: bool,
    /// Maximum concurrent transaction processing
    pub max_concurrent_processing: usize,
    /// Retry configuration
    pub max_retry_attempts: usize,
    pub retry_base_delay_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            wallet_pubkey: solana_sdk::pubkey::Pubkey::default(),
            check_interval_secs: NORMAL_CHECK_INTERVAL_SECS,
            enable_websocket: true,
            max_concurrent_processing: 10,
            max_retry_attempts: 3,
            retry_base_delay_secs: 30,
        }
    }
}
