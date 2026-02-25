//! SQLite persistence and caching layer for transaction history and statistics.
// Database operations and persistence for the transactions module
//
// This module provides high-performance SQLite-based caching and persistence
// for transaction data, replacing the previous JSON file-based approach.
//
// Module Structure:
// - `types`: Public type definitions for UI and statistics
// - `schema`: Internal database schema and constants
// - `operations`: Core TransactionDatabase implementation
// - `maintenance`: Statistics, cleanup, and bootstrap operations
// - `global`: Global instance management

mod bootstrap;
mod global;
mod maintenance;
mod operations;
mod operations_queries;
mod reporting;
mod reporting_aggregates;
mod schema;
mod types;

// Re-export public types
pub use types::{
    DatabaseStats, IntegrityReport, TransactionCursor, TransactionListFilters,
    TransactionListResult, TransactionListRow, WalletFlowExportRow,
};

// Re-export bootstrap types
pub use bootstrap::BootstrapState;

// Re-export main database struct
pub use operations::TransactionDatabase;

// Re-export global instance functions
pub use global::{get_transaction_database, init_transaction_database};
