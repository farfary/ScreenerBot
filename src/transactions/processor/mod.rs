//! Transaction processing pipeline: extraction, analysis, and classification of on-chain data.
// Transaction processing pipeline for the transactions module
//
// This module handles the core transaction processing logic including
// data extraction, analysis, and classification of blockchain transactions.

// Module declarations
mod analysis;
mod core;
mod dex_corrections;
mod extraction;
mod helpers;

// Re-export the main processor struct
pub use core::TransactionProcessor;
