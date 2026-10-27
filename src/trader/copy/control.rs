// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-task lifecycle: the one owner of create/update/delete/mode transitions,
//! the overview and the per-task books. The webserver routes and the agent tools
//! are both transports over these functions, so a guard added here (task limit,
//! watch attach/detach with rollback, live-delete refusal) covers every caller.

use super::CopyDatabase;
use crate::trader::{Error, Result};

mod books;
mod status;
mod tasks;
#[cfg(test)]
mod tests;

pub(super) use books::{
    all_positions, current_activity, paper_view, task_rounds, TASK_ACTIVITY_WINDOW,
};
pub use books::{
    book_stats, list_activity, overview, summarize, task_stats, task_stats_for, CopyTaskSummary,
    CopyTotals, CopyTradingOverview, TaskRounds,
};
/// The task state label, reached by the copy module's label tests.
#[cfg(test)]
pub(super) use status::effective_state;
pub use status::{
    build_status, followed_wallets, live_block_reason, status, stop_copy_trading, CopyLiveBlock,
    CopyTradingStatus,
};
pub use tasks::{
    create_task, delete_task, get_task, list_tasks, merge_task_patch, set_task_mode, update_task,
};
pub(super) use tasks::{holds_open_copies, sync_task_watch, wants_watch};

pub async fn open_database() -> Result<CopyDatabase> {
    tokio::task::spawn_blocking(|| CopyDatabase::shared(crate::chains::active_chain()))
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
}
