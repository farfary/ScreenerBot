//! Types for ATA cleanup operations

use serde::{Deserialize, Serialize};

// =============================================================================
// ATA INFO
// =============================================================================

/// Information about an Associated Token Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaInfo {
    /// ATA public key address
    pub ata_address: String,
    /// Token mint address
    pub mint: String,
    /// Current token balance (raw)
    pub balance: u64,
    /// Token decimals
    pub decimals: u8,
    /// Whether this uses Token-2022 program
    pub is_token_2022: bool,
    /// Whether this is an NFT
    pub is_nft: bool,
}

impl AtaInfo {
    /// Check if this ATA is empty (zero balance)
    pub fn is_empty(&self) -> bool {
        self.balance == 0
    }

    /// Get human-readable balance
    pub fn balance_ui(&self) -> f64 {
        self.balance as f64 / 10_f64.powi(self.decimals as i32)
    }
}

impl From<&crate::rpc::TokenAccountInfo> for AtaInfo {
    fn from(info: &crate::rpc::TokenAccountInfo) -> Self {
        Self {
            ata_address: info.account.clone(),
            mint: info.mint.clone(),
            balance: info.balance,
            decimals: info.decimals,
            is_token_2022: info.is_token_2022,
            is_nft: info.is_nft,
        }
    }
}

// =============================================================================
// CLEANUP STATS
// =============================================================================

/// Statistics for ATA cleanup operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtaCleanupStats {
    /// Total number of ATAs successfully closed
    pub total_closed: u32,
    /// Total SOL rent reclaimed
    pub total_rent_reclaimed: f64,
    /// Number of failed closure attempts
    pub failed_attempts: u32,
    /// Last cleanup timestamp (ISO 8601 format)
    pub last_cleanup_time: Option<String>,
}

// =============================================================================
// CLEANUP RESULT
// =============================================================================

/// Result of a single ATA cleanup cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaCleanupResult {
    /// Number of ATAs successfully closed
    pub closed_count: u32,
    /// Number of ATAs that failed to close
    pub failed_count: u32,
    /// Total SOL rent reclaimed
    pub rent_reclaimed: f64,
    /// Transaction signatures for successful closures
    pub signatures: Vec<String>,
}

impl Default for AtaCleanupResult {
    fn default() -> Self {
        Self {
            closed_count: 0,
            failed_count: 0,
            rent_reclaimed: 0.0,
            signatures: Vec::new(),
        }
    }
}

// =============================================================================
// DATABASE TYPES
// =============================================================================

/// ATA cleanup session record for database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaSession {
    /// Unique session ID
    pub session_id: String,
    /// Wallet address being cleaned
    pub wallet_address: String,
    /// Target number of ATAs to close (None = all empty)
    pub target_count: Option<i32>,
    /// Session status (ready/running/completed/failed)
    pub status: String,
    /// Session start time
    pub started_at: Option<String>,
    /// Session end time
    pub ended_at: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Total empty ATAs found
    pub total_atas_found: i32,
    /// Successfully closed count
    pub successful_closures: i32,
    /// Failed closure count
    pub failed_closures: i32,
    /// Total SOL recovered
    pub sol_recovered: f64,
    /// Record creation time
    pub created_at: String,
    /// Last update time
    pub updated_at: String,
}

/// Individual ATA closure record for database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaClosure {
    /// Session ID this closure belongs to
    pub session_id: String,
    /// ATA public key address
    pub ata_address: String,
    /// Token mint address
    pub token_mint: String,
    /// Transaction signature if successful
    pub signature: Option<String>,
    /// SOL recovered from this closure
    pub sol_recovered: f64,
    /// Status (pending/success/failed)
    pub status: String,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Execution time
    pub executed_at: Option<String>,
    /// Record creation time
    pub created_at: String,
}

/// Failed ATA record for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAta {
    /// ATA public key address
    pub ata_address: String,
    /// Token mint address (if known)
    pub token_mint: Option<String>,
    /// Wallet address
    pub wallet_address: String,
    /// Number of failure attempts
    pub failure_count: i32,
    /// Last error message
    pub last_error: Option<String>,
    /// First failure time
    pub first_failed_at: String,
    /// Last failure time
    pub last_failed_at: String,
    /// Next retry time (None = don't retry)
    pub next_retry_at: Option<String>,
    /// Whether this is a permanent failure
    pub is_permanent_failure: bool,
}
