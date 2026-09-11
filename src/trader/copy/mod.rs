//! Copy-trading decision core with paper simulation and guarded live submission.

mod analytics;
pub mod control;
mod database;
mod exits;
mod guards;
mod insights;
mod live;
mod matcher;
mod notify;
mod paper;
mod paper_exits;
mod pipeline;
mod risk;
mod service;
mod sizing;
mod types;
pub mod workspace;

pub use analytics::{
    apply_paper_book, arrival_distance_ms, build_task_stats, latency_should_pause,
    summarize_arrival_distances,
};
pub use database::{ActivityQuery, CopyDatabase, TargetObservations};
pub use exits::{
    execute_copy_sell_with, paper_sell_outcome, prepare_copy_sell, proportional_exit_percentage,
    CopySellSubmitResult, PreparedCopySell,
};
pub use insights::{build_insights, closed_rounds, CopyInsights, CopyRound, InsightRange};
pub use live::{
    execute_live_with, management_for_exit_mode, prepare_live_entry, sync_open_position_management,
    LiveSubmitResult, PreparedLiveEntry,
};
pub use matcher::matching_tasks;
pub use notify::{recent_notices, CopyNotice};
pub use paper::{simulate_fill, simulate_sell, PaperCosts, PAPER_REFERRAL_FEE_BPS};
pub use paper_exits::held_paper_mints;
pub use pipeline::run_paper_pipeline;
pub use risk::precheck;
pub use service::run;
pub use sizing::size_for;
pub use types::*;
