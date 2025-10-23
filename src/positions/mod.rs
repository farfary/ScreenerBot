// Position management module - clean modular design
pub mod apply;
pub mod db;
pub mod lib;
pub mod loss_detection;
pub mod metrics;
pub mod operations;
pub mod queue;
pub mod state;
pub mod tracking;
pub mod transitions;
pub mod types;
pub mod verifier;
pub mod worker;

// Public API exports
pub use operations::{
    add_to_position, close_position_direct, open_position_direct, partial_close_position,
    update_position_price,
};

pub use state::{
    acquire_position_lock, get_active_frozen_cooldowns, get_closed_positions, get_open_mints,
    get_open_positions, get_open_positions_count, get_position_by_id, get_position_by_mint,
    init_global_position_semaphore, is_open_position, is_token_in_cooldown, 
    reconcile_global_position_semaphore, MINT_TO_POSITION_INDEX,
    POSITIONS, SIG_TO_MINT_INDEX,
};

pub use tracking::update_position_tracking;

pub use metrics::get_proceeds_metrics_snapshot;

pub use worker::{initialize_positions_system, start_positions_manager_service};

pub use loss_detection::{
    get_loss_thresholds, is_loss_blacklisting_enabled, process_position_loss_detection,
};

// Database and library exports
pub use db::{
    delete_position_by_id, force_database_sync, get_closed_positions as get_db_closed_positions,
    get_closed_positions_count_since as get_db_closed_positions_count_since,
    get_entry_history, get_exit_history, get_open_positions as get_db_open_positions,
    get_position_by_id as get_db_position_by_id,
    get_position_by_mint as get_db_position_by_mint, get_positions_database,
    get_recent_closed_positions_for_mint, get_token_snapshot, get_token_snapshots,
    initialize_positions_database, load_all_positions, save_entry_record, save_exit_record,
    save_position, save_token_snapshot, update_position, with_positions_database,
    with_positions_database_async, PositionState, PositionStateHistory, PositionTracking,
    PositionsDatabase, PositionsDatabaseStats, TokenSnapshot,
};

pub use lib::{
    add_signature_to_index, calculate_position_pnl, calculate_position_total_fees,
    calculate_split_pnl, get_position_index_by_mint, remove_position_by_signature,
    save_position_token_snapshot, sync_position_to_database, update_mint_position_index,
};

// Core types re-exports
pub use metrics::ProceedsMetricsSnapshot;
pub use queue::{enqueue_verification, VerificationItem, VerificationKind};
pub use state::PositionLockGuard;
pub use transitions::PositionTransition;
pub use types::{EntryRecord, ExitRecord, Position};
