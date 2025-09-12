// Position management module - clean modular design
pub mod state;
pub mod queue;
pub mod transitions;
pub mod verifier;
pub mod apply;
pub mod operations;
pub mod tracking;
pub mod metrics;
pub mod worker;

// Public API exports
pub use operations::{
    open_position_direct,
    close_position_direct,
};

pub use state::{
    get_open_positions,
    get_closed_positions,
    get_open_positions_count,
    is_open_position,
    get_open_mints,
    get_position_by_mint,
    acquire_position_lock,
    get_active_frozen_cooldowns,
    POSITIONS,
    SIG_TO_MINT_INDEX,
    MINT_TO_POSITION_INDEX,
};

pub use tracking::update_position_tracking;

pub use metrics::get_proceeds_metrics_snapshot;

pub use worker::{
    start_positions_manager_service,
    initialize_positions_system,
};

// Core types re-exports
pub use crate::positions_types::Position;
pub use state::PositionLockGuard;
pub use transitions::PositionTransition;
pub use queue::{VerificationItem, VerificationKind, enqueue_verification};
pub use metrics::ProceedsMetricsSnapshot;