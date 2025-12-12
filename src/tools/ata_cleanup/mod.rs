//! ATA Cleanup module
//!
//! Provides functionality for scanning and closing empty Associated Token Accounts
//! to reclaim rent SOL (~0.002 SOL per account).
//!
//! ## Features
//! - Scan wallet for all ATAs
//! - Identify empty ATAs that can be closed
//! - Close individual or all empty ATAs
//! - Track failed ATAs to avoid retrying
//! - Background service for automatic cleanup
//!
//! ## Usage
//! ```rust,ignore
//! use crate::tools::ata_cleanup::{scan_wallet_atas, cleanup_empty_atas, start_ata_cleanup_service};
//!
//! // Scan for empty ATAs
//! let atas = scan_wallet_atas(&wallet_address).await?;
//!
//! // Clean up all empty ATAs
//! let result = cleanup_empty_atas(&wallet_address).await?;
//! println!("Closed {} ATAs, reclaimed {} SOL", result.closed_count, result.rent_reclaimed);
//! ```

mod operations;
mod service;
mod types;

// Re-export types
pub use types::{AtaClosure, AtaCleanupResult, AtaCleanupStats, AtaInfo, AtaSession, FailedAta};

// Re-export operations
pub use operations::{
    cleanup_empty_atas, clear_failed_ata_cache, close_ata, get_ata_status, get_cleanup_stats,
    get_failed_ata_count, is_ata_in_failed_cache, scan_closeable_atas, scan_empty_atas,
    scan_wallet_atas,
};

// Re-export service functions
pub use service::{start_ata_cleanup_service, trigger_immediate_cleanup};

// Backward compatibility aliases
pub use get_cleanup_stats as get_ata_cleanup_statistics;
pub use trigger_immediate_cleanup as trigger_immediate_ata_cleanup;
pub use get_ata_status as get_ata_cleanup_stats;
