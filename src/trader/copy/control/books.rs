// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The per-task books: each task's stats, summary and closed rounds from the book of
//! the mode it runs in, the totals across tasks, and the overview the dashboard polls.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::super::paper_book::{self, PaperBookView};
use super::super::{
    arrival_distance_ms, build_task_stats, closed_rounds, pause_text, summarize_arrival_distances,
    ArrivalDistanceStats, CopyActivityRow, CopyDatabase, CopyOutcome, CopyRecovery, CopyRound,
    CopyTask, CopyTaskStats,
};
use super::open_database;
use super::status::{build_status, effective_state, CopyTradingStatus};
use crate::i18n::{ids, UiArg, UiText};
use crate::positions::Position;
use crate::trader::paper::TradingMode;
use crate::trader::policy::ExitPolicy;
use crate::trader::{Error, Result};

/// Decisions read per task for its stats, rounds and analytics.
pub(in crate::trader::copy) const TASK_ACTIVITY_WINDOW: usize = 10_000;
/// Points of the per-task cumulative P&L trend the task list draws.
const PNL_TREND_POINTS: usize = 30;

/// A task with the figures of the book it runs in and its budget use.
#[derive(Serialize)]
pub struct CopyTaskSummary {
    #[serde(flatten)]
    pub task: CopyTask,
    pub stats: CopyTaskStats,
    pub spent_native: f64,
    pub remaining_budget_native: f64,
    pub effective_state: &'static str,
    /// The filter rule the task runs under after its own override.
    pub effective_require_filter_pass: bool,
    /// Whether the task copies stable-quoted buys after its own override.
    pub effective_copy_stable_quoted: bool,
    /// Cumulative realized P&L after each of the latest closed rounds.
    pub pnl_trend: Vec<f64>,
    /// Realized plus unrealized P&L of the task's book; unpriced holdings count as zero.
    pub net_pnl_native: f64,
    /// Why the task is paused; absent while it is enabled.
    pub pause_text: Option<UiText>,
    /// How the paused task comes back; absent while it is enabled.
    pub recovery: Option<CopyRecovery>,
    /// Why the task's stored exit overrides no longer fit the global exit policy, so an
    /// edit of them would be refused; absent while they fit. Its open positions run the
    /// global rule in place of each rule the overrides break
    /// (`trader::policy::ExitPolicy::apply_overrides`).
    pub exit_policy_issue: Option<UiText>,
}

/// The warning for a task whose exit overrides break a rule of the policy they make of
/// `global`, with that rule as the reason.
pub(super) fn exit_policy_issue(task: &CopyTask, global: ExitPolicy) -> Option<UiText> {
    task.exit_policy_overrides
        .validate_over(global)
        .err()
        .map(|fault| {
            UiText::new(ids::COPY_EXIT_ISSUE_WARNING)
                .arg("reason", UiArg::Nested(Box::new(fault.ui_text())))
        })
}

/// Figures across every task. P&L and holdings count paused tasks too (they
/// still hold what they bought); budget and arrival count enabled tasks only.
#[derive(Debug, Default, Serialize)]
pub struct CopyTotals {
    pub realized_pnl_native: f64,
    pub unrealized_pnl_native: f64,
    pub open_holdings: usize,
    pub unpriced_holdings: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate_pct: Option<f64>,
    pub active_budget_native: f64,
    pub active_spent_native: f64,
    pub active_arrival: ArrivalDistanceStats,
}

impl CopyTotals {
    /// Totals over every task summary; `active_samples` are the live-stream
    /// arrival samples of the enabled tasks.
    pub fn from_summaries(summaries: &[CopyTaskSummary], active_samples: Vec<u64>) -> Self {
        let mut totals = Self::default();
        for summary in summaries {
            let stats = &summary.stats;
            totals.realized_pnl_native += stats.realized_pnl_native;
            totals.unrealized_pnl_native += stats.unrealized_pnl_native;
            totals.open_holdings += stats.open_positions;
            totals.unpriced_holdings += stats.unpriced_positions;
            totals.wins += stats.wins;
            totals.losses += stats.losses;
            if summary.task.enabled {
                totals.active_budget_native += summary.task.total_budget_native;
                totals.active_spent_native += summary.spent_native;
            }
        }
        let rounds = totals.wins + totals.losses;
        totals.win_rate_pct = (rounds > 0).then(|| totals.wins as f64 / rounds as f64 * 100.0);
        totals.active_arrival = summarize_arrival_distances(active_samples);
        totals
    }
}

#[derive(Serialize)]
pub struct CopyTradingOverview {
    pub status: CopyTradingStatus,
    pub totals: CopyTotals,
    pub tasks: Vec<CopyTaskSummary>,
    pub activity: Vec<CopyActivityRow>,
}

/// Every position the stats may attribute to a live task, whatever its state.
pub(in crate::trader::copy) async fn all_positions() -> Vec<Position> {
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    positions
}

/// The rounds a task's figures read: the live positions it opened, or its rounds of
/// the current paper run with the instant that run starts.
#[derive(Debug, Clone, Default)]
pub struct TaskRounds {
    pub positions: Vec<Position>,
    /// Where a paper task's current run starts; absent for a live task.
    pub run_start: Option<DateTime<Utc>>,
}

impl TaskRounds {
    /// What the task spent in its book: its live spend ledger, or the copy buys of
    /// its current paper run.
    pub(in crate::trader::copy) async fn spent_native(
        &self,
        db: &CopyDatabase,
        task: &CopyTask,
    ) -> Result<f64> {
        match task.mode {
            TradingMode::Live => db.live_total_spent(task.id).await,
            TradingMode::Paper => {
                Ok(paper_book::paper_spend(&self.positions, "").total_spent_native)
            }
        }
    }
}

/// The rounds of `task` in the book of its mode. `live` is every live position (see
/// [`all_positions`]); `paper` is the shared paper book, read once for every task,
/// and a paper task reads no rounds without it.
pub(in crate::trader::copy) async fn task_rounds(
    task: &CopyTask,
    live: &[Position],
    paper: Option<&PaperBookView>,
) -> Result<TaskRounds> {
    match task.mode {
        TradingMode::Live => Ok(TaskRounds {
            positions: live
                .iter()
                .filter(|position| {
                    matches!(
                        position.origin,
                        crate::positions::PositionOrigin::Copy { task_id, .. } if task_id == task.id
                    )
                })
                .cloned()
                .collect(),
            run_start: None,
        }),
        TradingMode::Paper => match paper {
            Some(view) => {
                let run = view.task_run(task).await?;
                Ok(TaskRounds {
                    positions: run.rounds,
                    run_start: Some(run.start),
                })
            }
            None => Ok(TaskRounds::default()),
        },
    }
}

/// The shared paper book when any of `tasks` runs in paper. A book that cannot be read
/// leaves the paper tasks without rounds rather than failing every task's figures.
pub(in crate::trader::copy) async fn paper_view(tasks: &[CopyTask]) -> Option<PaperBookView> {
    if !tasks.iter().any(|task| task.mode == TradingMode::Paper) {
        return None;
    }
    match PaperBookView::load().await {
        Ok(view) => Some(view),
        Err(error) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!("Copy paper figures read no rounds: {error}"),
            );
            None
        }
    }
}

/// The decisions of `activity` in the book of `task`, and for a paper task only
/// those of its current run, the run that starts at `run_start`.
pub(in crate::trader::copy) fn current_activity(
    task: &CopyTask,
    activity: &[CopyActivityRow],
    run_start: Option<DateTime<Utc>>,
) -> Vec<CopyActivityRow> {
    activity
        .iter()
        .filter(|row| match (&row.outcome, task.mode) {
            (CopyOutcome::PaperFilled(decision), TradingMode::Paper) => {
                decision.fill.has_quote_provenance()
            }
            (CopyOutcome::PaperSellObserved(decision), TradingMode::Paper) => decision
                .paper_fill
                .as_ref()
                .is_some_and(|fill| fill.has_quote_provenance()),
            (CopyOutcome::Skipped { paper, .. }, TradingMode::Paper) => paper
                .as_ref()
                .is_some_and(|context| context.model == crate::trader::paper::PaperModel::Quote),
            (CopyOutcome::Skipped { paper, .. }, TradingMode::Live) => paper.is_none(),
            (_, TradingMode::Live) => !matches!(
                row.outcome,
                CopyOutcome::PaperFilled(_) | CopyOutcome::PaperSellObserved(_)
            ),
            _ => false,
        })
        .filter(|row| run_start.is_none_or(|start| paper_book::in_current_run(&row.outcome, start)))
        .cloned()
        .collect()
}

/// A task's stats and the closed rounds of its book, from what was already read.
/// Decision counts and latency come from the task's activity of its current run;
/// position and P&L figures come from `rounds`, the rounds of the book of the mode it
/// runs in -- its paper rounds in the shared paper book while in paper mode, the real
/// positions it opened while live -- and wins and losses are that book's closed
/// rounds. Every path that reports a task's stats goes through here, so no endpoint
/// reports a partial set.
pub fn book_stats(
    task: &CopyTask,
    activity: &[CopyActivityRow],
    rounds: &TaskRounds,
) -> (CopyTaskStats, Vec<CopyRound>) {
    let current = current_activity(task, activity, rounds.run_start);
    let mut stats = build_task_stats(
        task.id,
        super::super::workspace::book_of(task),
        &current,
        &rounds.positions,
    );
    let closed = closed_rounds(task.id, stats.book, &rounds.positions);
    stats.wins = closed.iter().filter(|round| round.pnl_native > 0.0).count();
    stats.losses = closed.len() - stats.wins;
    (stats, closed)
}

pub async fn task_stats_for(
    db: &CopyDatabase,
    task: &CopyTask,
    rounds: &TaskRounds,
) -> Result<CopyTaskStats> {
    let activity = db.list_task_activity(task.id, TASK_ACTIVITY_WINDOW).await?;
    Ok(book_stats(task, &activity, rounds).0)
}

pub async fn task_stats(id: i64) -> Result<CopyTaskStats> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let paper = paper_view(std::slice::from_ref(&task)).await;
    let rounds = task_rounds(&task, &all_positions().await, paper.as_ref()).await?;
    task_stats_for(&db, &task, &rounds).await
}

/// One task's summary from what was already read, with the live-stream arrival
/// samples the overview aggregates across enabled tasks.
pub fn summarize(
    status: &CopyTradingStatus,
    task: CopyTask,
    activity: &[CopyActivityRow],
    rounds: &TaskRounds,
    spent_native: f64,
) -> (CopyTaskSummary, Vec<u64>) {
    let (stats, rounds) = book_stats(&task, activity, rounds);
    let net_pnl_native = stats.realized_pnl_native + stats.unrealized_pnl_native;
    let mut cumulative = 0.0;
    let mut pnl_trend = rounds
        .iter()
        .map(|round| {
            cumulative += round.pnl_native;
            cumulative
        })
        .collect::<Vec<_>>();
    pnl_trend.drain(..pnl_trend.len().saturating_sub(PNL_TREND_POINTS));
    let samples = activity
        .iter()
        .filter(|row| row.task_id == task.id)
        .filter_map(|row| row.outcome.telemetry())
        .filter(|telemetry| !telemetry.backfill)
        .filter_map(arrival_distance_ms)
        .collect();
    let (global_filter, global_stable) = crate::config::with_config(|config| {
        (
            config.copy_trading.require_filter_pass,
            config.copy_trading.copy_stable_quoted,
        )
    });
    (
        CopyTaskSummary {
            stats,
            remaining_budget_native: (task.total_budget_native - spent_native).max(0.0),
            spent_native,
            effective_state: effective_state(status, &task),
            effective_require_filter_pass: task.requires_filter_pass(global_filter),
            effective_copy_stable_quoted: task.copies_stable_quoted(global_stable),
            pnl_trend,
            net_pnl_native,
            pause_text: (!task.enabled).then(|| pause_text(task.pause_reason.as_ref())),
            recovery: task.recovery(),
            exit_policy_issue: exit_policy_issue(&task, ExitPolicy::from_config()),
            task,
        },
        samples,
    )
}

pub async fn overview(activity_limit: usize) -> Result<CopyTradingOverview> {
    let db = open_database().await?;
    let tasks = db.list_tasks().await?;
    let mut activity = db.list_activity(activity_limit).await?;
    let positions = all_positions().await;
    let paper = paper_view(&tasks).await;
    paper_book::flag_earlier_runs(&mut activity, &tasks, paper.as_ref());
    let status = build_status(&tasks);
    // Each task's reads are independent of every other task's; the pool serves
    // them concurrently instead of one task after another on every poll.
    let reads = futures::future::try_join_all(tasks.iter().map(|task| {
        let db = &db;
        let positions = &positions;
        let paper = paper.as_ref();
        async move {
            let rounds = task_rounds(task, positions, paper).await?;
            let spent_native = rounds.spent_native(db, task).await?;
            Ok::<_, Error>((
                db.list_task_activity(task.id, TASK_ACTIVITY_WINDOW).await?,
                rounds,
                spent_native,
            ))
        }
    }))
    .await?;
    let mut active_samples = Vec::new();
    let mut summaries = Vec::with_capacity(tasks.len());
    for (task, (task_activity, rounds, spent_native)) in tasks.into_iter().zip(reads) {
        let (summary, samples) = summarize(&status, task, &task_activity, &rounds, spent_native);
        if summary.task.enabled {
            active_samples.extend(samples);
        }
        summaries.push(summary);
    }
    Ok(CopyTradingOverview {
        status,
        totals: CopyTotals::from_summaries(&summaries, active_samples),
        tasks: summaries,
        activity,
    })
}

pub async fn list_activity(task_id: Option<i64>, limit: usize) -> Result<Vec<CopyActivityRow>> {
    let db = open_database().await?;
    let mut activity = match task_id {
        Some(id) => db.list_task_activity(id, limit).await?,
        None => db.list_activity(limit).await?,
    };
    let tasks = db.list_tasks().await?;
    let paper = paper_view(&tasks).await;
    paper_book::flag_earlier_runs(&mut activity, &tasks, paper.as_ref());
    Ok(activity)
}
