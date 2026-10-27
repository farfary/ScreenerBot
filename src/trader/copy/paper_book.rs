// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy trading's read of the shared paper book: where a paper task's current run
//! starts, the rounds the task holds in that run, the spend those rounds make and the
//! paper exits of those rounds that found no fill.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::positions::db::{PaperEpoch, PaperExitOwner};
use crate::positions::Position;
use crate::trader::error::Error;
use crate::trader::paper::TradingMode;

use super::types::{CopyActivityRow, CopyOutcome, CopyTask, SpendState};

/// Where the current paper run of `task` starts: the later of the task's own start
/// (set when the archive moved and by a per-task reset) and the start of the
/// wallet's current paper epoch. Every figure a paper task shows covers only rounds
/// and decisions at or after this instant.
pub fn effective_start(task: &CopyTask, epoch: Option<&PaperEpoch>) -> DateTime<Utc> {
    [task.paper_started_at, epoch.map(|epoch| epoch.started_at)]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

/// The shared paper book of the active wallet, read once for any number of tasks: its
/// current epoch and its open rounds with their in-memory marks.
pub struct PaperBookView {
    wallet: String,
    epoch: Option<PaperEpoch>,
    open: HashMap<i64, Position>,
}

/// The rounds of one paper task in its current run.
#[derive(Debug, Clone)]
pub struct TaskPaperRun {
    pub start: DateTime<Utc>,
    /// The task's rounds entered at or after `start`, open and closed; open rounds
    /// carry their latest mark.
    pub rounds: Vec<Position>,
}

impl PaperBookView {
    /// Reads the epoch and the open rounds of the active wallet's paper book.
    pub async fn load() -> crate::trader::Result<Self> {
        let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
        let wallet = crate::utils::get_wallet_address().map_err(|_| Error::PaperBookUnavailable)?;
        let epoch = book.current_epoch(&wallet).await?;
        let open = book
            .open_positions()
            .await
            .into_iter()
            .filter_map(|position| position.id.map(|id| (id, position)))
            .collect();
        Ok(Self {
            wallet,
            epoch,
            open,
        })
    }

    /// Where the current run of `task` starts.
    pub fn start_of(&self, task: &CopyTask) -> DateTime<Utc> {
        effective_start(task, self.epoch.as_ref())
    }

    /// The rounds `task` entered in its current run, each open one replaced by the
    /// book's in-memory copy so it carries the latest mark.
    pub async fn task_run(&self, task: &CopyTask) -> crate::trader::Result<TaskPaperRun> {
        let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
        let start = self.start_of(task);
        let rounds = book
            .copy_task_rounds(&self.wallet, task.id, start)
            .await?
            .into_iter()
            .map(|stored| {
                stored
                    .id
                    .and_then(|id| self.open.get(&id))
                    .filter(|_| stored.exit_time.is_none())
                    .cloned()
                    .unwrap_or(stored)
            })
            .collect();
        Ok(TaskPaperRun { start, rounds })
    }

    /// Paper exits of the rounds of task `task_id` that found no fill, in a row up to
    /// the latest exit.
    pub async fn failed_exits(&self, task_id: i64) -> crate::trader::Result<u32> {
        let book = crate::positions::paper::book().ok_or(Error::PaperBookUnavailable)?;
        Ok(book
            .failed_exit_streak(&self.wallet, &PaperExitOwner::CopyTask(task_id))
            .await?)
    }
}

/// The rounds of `task` in its current paper run.
pub async fn task_run(task: &CopyTask) -> crate::trader::Result<TaskPaperRun> {
    PaperBookView::load().await?.task_run(task).await
}

impl TaskPaperRun {
    /// The open rounds of the run.
    pub fn open_rounds(&self) -> impl Iterator<Item = &Position> {
        self.rounds
            .iter()
            .filter(|position| position.exit_time.is_none())
    }
}

/// What a paper task spent in its run, in total and on `mint`: the copy buy of each
/// round (`entry_size_native`; automatic additions are not the task's spend, as live
/// spend leaves them out) and the number of rounds of `mint`.
pub fn paper_spend(rounds: &[Position], mint: &str) -> SpendState {
    rounds
        .iter()
        .fold(SpendState::default(), |mut spend, position| {
            spend.total_spent_native += position.entry_size_native;
            if position.mint == mint {
                spend.token_spent_native += position.entry_size_native;
                spend.token_buy_count += 1;
            }
            spend
        })
}

/// Whether `outcome` was decided in the run that starts at `start`.
pub fn in_current_run(outcome: &CopyOutcome, start: DateTime<Utc>) -> bool {
    outcome.decided_at() >= start
}

/// Flags every row of a paper task in `rows` decided before that task's current run.
/// `view` is the shared paper book; without it a run starts at the task's own start.
pub fn flag_earlier_runs(
    rows: &mut [CopyActivityRow],
    tasks: &[CopyTask],
    view: Option<&PaperBookView>,
) {
    let starts = tasks
        .iter()
        .filter(|task| task.mode == TradingMode::Paper)
        .map(|task| {
            let start =
                view.map_or_else(|| effective_start(task, None), |view| view.start_of(task));
            (task.id, start)
        })
        .collect::<HashMap<_, _>>();
    for row in rows {
        row.earlier_run = starts
            .get(&row.task_id)
            .is_some_and(|start| !in_current_run(&row.outcome, *start));
    }
}

#[cfg(test)]
#[path = "paper_book_tests.rs"]
mod tests;
