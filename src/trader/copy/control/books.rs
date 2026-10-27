// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The per-task books: each task's stats, summary and closed rounds from the book of
//! the mode it runs in, the totals across tasks, and the overview the dashboard polls.

use serde::Serialize;

use super::super::{
    apply_paper_book, arrival_distance_ms, build_task_stats, closed_rounds, pause_text,
    summarize_arrival_distances, ArrivalDistanceStats, CopyActivityRow, CopyDatabase, CopyMode,
    CopyOutcome, CopyRound, CopyTask, CopyTaskStats, PaperPosition,
};
use super::open_database;
use super::status::{build_status, effective_state, CopyTradingStatus};
use crate::i18n::{ids, UiArg, UiText};
use crate::positions::Position;
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
    /// Why the task is paused; absent while it is enabled.
    pub pause_text: Option<UiText>,
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

/// The price a paper holding is marked at: the live pool price only, the same
/// price its exits trade on. Without one the holding counts as unpriced; the last
/// observed trade price is usually its own entry and would hide the real move.
/// `chain` is the chain of the task that holds the position.
pub(in crate::trader::copy) fn paper_mark(
    chain: crate::chains::ChainId,
    position: &PaperPosition,
) -> Option<f64> {
    crate::pools::get_pool_price(chain, &position.mint).map(|price| price.price_native)
}

/// A task's stats and the closed rounds of its book, from what was already read.
/// Decision counts and latency come from the task's activity; position and P&L
/// figures come from the book of the mode it runs in -- its paper ledger while in
/// paper mode, the real positions it opened while live -- and wins and losses are
/// that book's closed rounds. Every path that reports a task's stats goes through
/// here, so no endpoint reports a partial set.
pub fn book_stats(
    task: &CopyTask,
    activity: &[CopyActivityRow],
    positions: &[Position],
    paper_book: &[PaperPosition],
    mark: impl Fn(&PaperPosition) -> Option<f64>,
) -> (CopyTaskStats, Vec<CopyRound>) {
    let current_activity = activity
        .iter()
        .filter(|row| match (&row.outcome, task.mode) {
            (CopyOutcome::PaperFilled(decision), CopyMode::Paper) => {
                decision.fill.has_quote_provenance()
            }
            (CopyOutcome::PaperSellObserved(decision), CopyMode::Paper) => decision
                .paper_fill
                .as_ref()
                .is_some_and(|fill| fill.has_quote_provenance()),
            (CopyOutcome::Skipped { paper, .. }, CopyMode::Paper) => paper
                .as_ref()
                .is_some_and(|context| context.model == super::super::PaperModel::Quote),
            (CopyOutcome::Skipped { paper, .. }, CopyMode::Live) => paper.is_none(),
            (_, CopyMode::Live) => !matches!(
                row.outcome,
                CopyOutcome::PaperFilled(_) | CopyOutcome::PaperSellObserved(_)
            ),
            _ => false,
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut stats = build_task_stats(task.id, &current_activity, positions);
    if task.mode == CopyMode::Paper {
        apply_paper_book(&mut stats, paper_book, mark);
    }
    let rounds = closed_rounds(task.id, stats.book, &current_activity, positions);
    stats.wins = rounds.iter().filter(|round| round.pnl_native > 0.0).count();
    stats.losses = rounds.len() - stats.wins;
    (stats, rounds)
}

/// The paper ledger a task's stats read; a live task's figures never come from it.
async fn stats_book(db: &CopyDatabase, task: &CopyTask) -> Result<Vec<PaperPosition>> {
    if task.mode == CopyMode::Paper {
        db.paper_positions(task.id).await
    } else {
        Ok(Vec::new())
    }
}

pub async fn task_stats_for(
    db: &CopyDatabase,
    task: &CopyTask,
    positions: &[Position],
) -> Result<CopyTaskStats> {
    let activity = db.list_task_activity(task.id, TASK_ACTIVITY_WINDOW).await?;
    let book = stats_book(db, task).await?;
    Ok(book_stats(task, &activity, positions, &book, |position| {
        paper_mark(task.chain, position)
    })
    .0)
}

pub async fn task_stats(id: i64) -> Result<CopyTaskStats> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    task_stats_for(&db, &task, &all_positions().await).await
}

/// One task's summary from what was already read, with the live-stream arrival
/// samples the overview aggregates across enabled tasks.
pub fn summarize(
    status: &CopyTradingStatus,
    task: CopyTask,
    activity: &[CopyActivityRow],
    positions: &[Position],
    paper_book: &[PaperPosition],
    spent_native: f64,
    mark: impl Fn(&PaperPosition) -> Option<f64>,
) -> (CopyTaskSummary, Vec<u64>) {
    let (stats, rounds) = book_stats(&task, activity, positions, paper_book, mark);
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
            pause_text: (!task.enabled).then(|| pause_text(task.pause_reason.as_ref())),
            exit_policy_issue: exit_policy_issue(&task, ExitPolicy::from_config()),
            task,
        },
        samples,
    )
}

pub async fn overview(activity_limit: usize) -> Result<CopyTradingOverview> {
    let db = open_database().await?;
    let tasks = db.list_tasks().await?;
    let activity = db.list_activity(activity_limit).await?;
    let positions = all_positions().await;
    let status = build_status(&tasks);
    // Each task's reads are independent of every other task's; the pool serves
    // them concurrently instead of one task after another on every poll.
    let reads = futures::future::try_join_all(tasks.iter().map(|task| {
        let db = &db;
        async move {
            Ok::<_, Error>((
                db.list_task_activity(task.id, TASK_ACTIVITY_WINDOW).await?,
                stats_book(db, task).await?,
                db.task_total_spent(task.id, task.mode).await?,
            ))
        }
    }))
    .await?;
    let mut active_samples = Vec::new();
    let mut summaries = Vec::with_capacity(tasks.len());
    for (task, (task_activity, book, spent_native)) in tasks.into_iter().zip(reads) {
        let chain = task.chain;
        let (summary, samples) = summarize(
            &status,
            task,
            &task_activity,
            &positions,
            &book,
            spent_native,
            |position| paper_mark(chain, position),
        );
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
    match task_id {
        Some(id) => db.list_task_activity(id, limit).await,
        None => db.list_activity(limit).await,
    }
}
