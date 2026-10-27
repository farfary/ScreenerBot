// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-trading decision core with paper simulation and guarded live submission.

mod analytics;
pub mod control;
pub(crate) mod database;
mod entries;
mod exits;
mod guards;
mod insights;
#[cfg(test)]
mod label_tests;
mod live;
mod matcher;
mod notify;
pub mod paper_book;
mod paper_execution;
mod pause;
mod risk;
#[cfg(test)]
mod round_fixtures;
mod service;
mod sizing;
mod types;
pub mod workspace;

pub use analytics::{
    arrival_distance_ms, build_task_stats, latency_should_pause, summarize_arrival_distances,
};
/// Rename declarations, reached by the schema owner's cross-store guard.
#[cfg(test)]
pub(crate) use database::column_names;
pub use database::{ActivityQuery, CopyDatabase, TargetObservations};
pub use entries::{execute_paper_with, leg_key, prepare_entry, PreparedEntry};
pub use exits::{
    execute_copy_sell_with, prepare_copy_sell, proportional_exit_percentage, CopySellSubmitResult,
    PreparedCopySell,
};
pub use insights::{build_insights, closed_rounds, CopyInsights, CopyRound, InsightRange};
pub use live::{
    execute_live_with, management_for_exit_mode, sync_open_position_management, LiveSubmitResult,
};
pub use matcher::matching_tasks;
pub use notify::{recent_notices, task_arg, CopyNotice};
pub use paper_execution::paper_trade_wanted;
pub use pause::pause_text;
pub use risk::precheck;
pub use service::run;
pub use sizing::size_for;
pub use types::*;
