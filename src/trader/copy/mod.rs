//! Paper copy-trading decision core.
//!
//! The runtime consumes the shared wallet-observation broadcast but deliberately has
//! no call to `submit_entry`: `Live` tasks are rejected during input validation and
//! again by `risk::precheck` before execution can be reached.

mod database;
mod matcher;
mod paper;
mod pipeline;
mod risk;
mod service;
mod sizing;
mod types;

pub use database::CopyDatabase;
pub use matcher::matching_tasks;
pub use paper::{simulate_fill, PaperCosts, PAPER_REFERRAL_FEE_BPS};
pub use pipeline::run_paper_pipeline;
pub use risk::precheck;
pub use service::run;
pub use sizing::size_for;
pub use types::*;
