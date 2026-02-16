// Service configuration, constants, and deferred retry queue

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

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

/// Global deferred retry queue
/// Transactions are added here when they fail due to temporary issues like RPC indexing delays
pub static DEFERRED_RETRIES: LazyLock<Arc<Mutex<BTreeMap<String, DeferredRetry>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(BTreeMap::new())));

/// Add a transaction to the deferred retry queue
pub async fn defer_transaction_retry(signature: String, delay_secs: i64, reason: String) {
    let now = Utc::now();
    let retry = DeferredRetry {
        signature: signature.clone(),
        next_retry_at: now + chrono::Duration::seconds(delay_secs),
        attempts: 1,
        current_delay_secs: delay_secs,
        last_error: Some(reason.clone()),
        first_seen: now,
    };

    let mut deferred = DEFERRED_RETRIES.lock().await;

    // Check if already exists, increment attempts if so
    if let Some(existing) = deferred.get_mut(&signature) {
        existing.attempts += 1;
        existing.current_delay_secs = delay_secs * (existing.attempts as i64);
        existing.next_retry_at = now + chrono::Duration::seconds(existing.current_delay_secs);
        existing.last_error = Some(reason);
    } else {
        deferred.insert(signature, retry);
    }
}

/// Get count of deferred retries
pub async fn get_deferred_retries_count() -> usize {
    let deferred = DEFERRED_RETRIES.lock().await;
    deferred.len()
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
