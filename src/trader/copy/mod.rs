//! Copy-trading decision core with paper simulation and guarded live submission.

mod database;
mod live;
mod matcher;
mod paper;
mod pipeline;
mod risk;
mod service;
mod sizing;
mod types;

pub use database::CopyDatabase;
pub use live::{
    execute_live_with, management_for_exit_mode, prepare_live_entry, LiveSubmitResult,
    PreparedLiveEntry,
};
pub use matcher::matching_tasks;
pub use paper::{simulate_fill, PaperCosts, PAPER_REFERRAL_FEE_BPS};
pub use pipeline::run_paper_pipeline;
pub use risk::precheck;
pub use service::run;
pub use sizing::size_for;
pub use types::*;
