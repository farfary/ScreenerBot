//! Transaction service configuration — runtime settings for connection management and retries.
//
// Service configuration, constants, and deferred retry queue

use crate::transactions::utils::NORMAL_CHECK_INTERVAL_SECS;

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
// SERVICE CONFIGURATION
// =============================================================================

/// Service configuration structure
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Wallet public key to monitor
    pub wallet_pubkey: crate::chains::solana::solana_sdk::pubkey::Pubkey,
    /// Interval for expired-pending cleanup. Real-time
    /// detection (WebSocket + poll fallback + gap-fill) is owned by
    /// `wallets::watch`, not this service -- see `processing.rs`.
    pub check_interval_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            wallet_pubkey: crate::chains::solana::solana_sdk::pubkey::Pubkey::default(),
            check_interval_secs: NORMAL_CHECK_INTERVAL_SECS,
        }
    }
}
