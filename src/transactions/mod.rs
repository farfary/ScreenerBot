// Transactions Module - Modern modular architecture for Solana DEX bot transaction management
//
// This module provides comprehensive transaction monitoring, analysis, and verification
// for the ScreenerBot trading system. It replaces the previous flat file structure
// with a clean, maintainable modular design.
//
// Architecture:
// - `manager`: Core TransactionsManager struct and lifecycle management
// - `service`: Background monitoring service and coordination
// - `analyzer`: Transaction analysis, classification, and pattern detection
// - `processor`: Transaction processing pipeline and batch operations
// - `fetcher`: RPC batching, caching, and blockchain data retrieval
// - `verifier`: Transaction verification logic for positions integration
// - `database`: High-performance SQLite-based caching and persistence
// - `debug`: Debug utilities, diagnostics, and troubleshooting tools
// - `types`: Core type definitions, enums, and data structures
// - `utils`: Helper functions, constants, and utility code
//
// Key Features:
// - Real-time transaction monitoring via WebSocket integration
// - High-performance batch RPC operations (50-account limit compliance)
// - Comprehensive DEX transaction analysis (Jupiter, Raydium, Orca, etc.)
// - Advanced swap detection and P&L calculation
// - ATA operation tracking and rent calculation
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

pub mod analyzer;
pub mod database;
pub mod debug;
pub mod fetcher;
pub mod manager;
pub mod processor;
pub mod program_ids;
pub mod service;
pub mod types;
pub mod utils;
pub mod verifier;
pub mod websocket;

// Public API exports - Core functionality
pub use manager::TransactionsManager;
pub use service::{
    get_global_transaction_manager, get_transaction, is_global_transaction_service_running,
    start_global_transaction_service, stop_global_transaction_service,
};

// Public API exports - Types
pub use types::{
    AtaAnalysis, AtaOperation, AtaOperationType, CachedAnalysis, DeferredRetry, InstructionInfo,
    SolBalanceChange, SwapPnLInfo, TokenBalanceChange, TokenSwapInfo, TokenTransfer, Transaction,
    TransactionDirection, TransactionStats, TransactionStatus, TransactionType,
};

// Public API exports - Constants from types
pub use types::ANALYSIS_CACHE_VERSION;

// Public API exports - Analysis and verification
pub use analyzer::{
    confidence_to_score, is_analysis_reliable, AnalysisConfidence, CompleteAnalysis,
    TransactionAnalyzer,
};

pub use verifier::{
    verify_entry_transaction, verify_exit_transaction, verify_transaction_for_position,
};

// Public API exports - Database operations
pub use database::{get_transaction_database, init_transaction_database, TransactionDatabase};

// Public API exports - Program IDs and router detection
pub use program_ids::{
    detect_router_from_logs, detect_router_from_program_id, is_dex_aggregator_program_id,
    is_jupiter_program_id, is_mev_tip_address, JUPITER_V6_PROGRAM_ID, PUMP_FUN_AMM_PROGRAM_ID,
    PUMP_FUN_LEGACY_PROGRAM_ID,
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
