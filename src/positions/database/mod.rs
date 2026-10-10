// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SQLite persistence layer for position management with connection pooling.
mod booking;
mod column_names;
mod convenience;
mod global;
mod open_round;
mod operations;
mod provenance;
mod queries;
mod raw_migration;
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
pub(crate) use types::PENDING_PARTIAL_EXIT_METADATA_KEY;
pub use types::{
    DailyTradingStats, PeriodTradingStats, PositionState, PositionStateHistory, PositionTracking,
    PositionsDatabase, PositionsDatabaseStats, TokenSnapshot,
};

// Re-export global database functions
pub use global::{
    get_positions_database, initialize_positions_database, with_positions_database,
    with_positions_database_async,
};

pub(crate) use booking::{Booking, BookingReads, BookingRecord, Committed, OtherOpenHeld};
pub(crate) use operations::carry_columns_not_booked;
pub use queries::{MintRow, TraderSwapLeg};

// Re-export convenience functions
pub(crate) use convenience::{
    commit_booking, get_open_round_id, get_other_open_held, refuse_reactivating_open_round,
};
pub use convenience::{
    delete_archived_positions, delete_position_by_id, force_database_sync,
    get_all_positions_for_mint, get_closed_positions, get_closed_positions_count_since,
    get_closed_positions_since, get_daily_trading_stats, get_entry_history, get_exit_history,
    get_latest_position_by_mint, get_metadata, get_open_positions, get_period_trading_stats,
    get_position_by_id, get_recent_closed_positions_for_mint, get_store_chain, get_token_snapshot,
    get_token_snapshots, get_trader_swap_legs, load_all_positions, record_exit_submission,
    save_position, save_token_snapshot, set_metadata, set_position_archived_db,
    set_position_management_db, update_position_price_and_pnl_fields, update_position_price_fields,
};
