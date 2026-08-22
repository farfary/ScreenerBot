//! Transaction processing — chain-neutral persistence, orchestration and reporting.
//
// Architecture:
// - `manager`: Core TransactionsManager struct and lifecycle management
// - `service`: Background monitoring service and coordination
// - `verifier`: Transaction verification logic for positions integration
// - `database`: High-performance SQLite-based caching and persistence
// - `debug`: Debug utilities, diagnostics, and troubleshooting tools
// - `types`: Core type definitions, enums, and data structures
// - `utils`: Helper functions, constants, and utility code
// - `deltas`: The chain-neutral subject-relative balance delta shape
//
// Chain-specific decoding (RPC fetching, wire-format analysis/classification,
// program ID recognition) lives under `crate::chains::solana::transactions` — this
// module consumes its output (`Transaction`, `SubjectAssetDelta`) but owns no Solana
// vendor types itself.
//
// Key Features:
// - Real-time transaction monitoring via WebSocket integration
// - Position integration for entry/exit transaction verification
// - Events system integration for analytics and debugging
// - Structured logging with full address visibility
// - SQLite-based caching with connection pooling
// - Retry logic for network resilience
//
// Usage:
// ```rust
// use crate::transactions::{TransactionsManager, TransactionType};
//
// let manager = TransactionsManager::new(wallet_pubkey).await?;
// manager.start_service().await?;
// ```

pub mod database;
pub mod debug;
mod debug_helpers;
pub mod deltas;
pub mod manager;
pub mod service;
pub mod subject;
pub mod types;
pub mod utils;
pub mod verifier;

// Public API exports - Core functionality
pub use manager::TransactionsManager;
pub use service::{
    get_global_transaction_manager, get_transaction, is_global_transaction_service_running,
    reprocess_transaction, start_global_transaction_service, stop_global_transaction_service,
};

// Public API exports - Subject-relative balance deltas (chain-neutral shape; extraction
// from a decoded transaction is chain-specific, see `chains::solana::transactions::deltas`)
pub use deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};

// Public API exports - Types
pub use subject::Subject;
pub use types::{
    AtaAnalysis, AtaOperation, AtaOperationType, CachedAnalysis, DeferredRetry, InstructionInfo,
    SolBalanceChange, SwapPnLInfo, TokenBalanceChange, TokenSwapInfo, TokenTransfer, Transaction,
    TransactionDirection, TransactionStats, TransactionStatus, TransactionType,
};

// Public API exports - Constants from types
pub use types::ANALYSIS_CACHE_VERSION;

// Public API exports - Verification
pub use verifier::{
    verify_entry_transaction, verify_exit_transaction, verify_transaction_for_position,
};

// Public API exports - Database operations
pub use database::{
    get_transaction_database, init_transaction_database, DatabaseStats, IntegrityReport,
    TransactionCursor, TransactionDatabase, TransactionListFilters, TransactionListResult,
    TransactionListRow, WalletFlowExportRow,
};

// Public API exports - Utilities
pub use utils::{
    add_signature_to_known_globally, get_pending_transactions_count, is_signature_known_globally,
};

// Constants re-exported for convenience
pub use utils::{
    ATA_RENT_COST_SOL, ATA_RENT_TOLERANCE_LAMPORTS, DEFAULT_COMPUTE_UNIT_PRICE,
    MIN_PENDING_LAMPORT_DELTA, NORMAL_CHECK_INTERVAL_SECS, PENDING_MAX_AGE_SECS,
    PROCESS_BATCH_SIZE, RPC_BATCH_SIZE, TRANSACTION_DATA_BATCH_SIZE, WSOL_MINT,
};
