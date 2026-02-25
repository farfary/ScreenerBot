//! SQLite persistence layer for position management with connection pooling.
mod convenience;
mod global;
mod operations;
mod queries;
mod tracking;
/// Database module for positions management
/// Replaces JSON file-based storage with high-performance SQLite database
///
/// This module provides:
/// - Thread-safe database operations using connection pooling
/// - ACID transactions for data integrity
/// - High-performance batch operations
/// - Comprehensive position state management
mod types;

// Re-export types
pub use types::{
    PeriodTradingStats, PositionState, PositionStateHistory, PositionTracking, PositionsDatabase,
    PositionsDatabaseStats, TokenSnapshot,
};

// Re-export global database functions
pub use global::{
    get_positions_database, initialize_positions_database, with_positions_database,
    with_positions_database_async,
};

// Re-export convenience functions
pub use convenience::{
    delete_position_by_id, force_database_sync, get_closed_positions,
    get_closed_positions_count_since, get_closed_positions_since, get_entry_history,
    get_exit_history, get_metadata, get_open_positions, get_period_trading_stats,
    get_position_by_id, get_position_by_mint, get_recent_closed_positions_for_mint,
    get_token_snapshot, get_token_snapshots, load_all_positions, save_entry_record,
    save_exit_record, save_position, save_token_snapshot, set_metadata, update_position,
    update_position_price_fields,
};
