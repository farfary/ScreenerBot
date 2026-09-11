//! Copy-task lifecycle: the one owner of create/update/delete/mode transitions,
//! the overview and the per-task books. The webserver routes and the agent tools
//! are both transports over these functions, so a guard added here (task limit,
//! watch attach/detach with rollback, live-delete refusal) covers every caller.

use chrono::Utc;
use serde::Serialize;

use super::{
    apply_paper_book, arrival_distance_ms, build_task_stats, closed_rounds,
    confirm_mode_transition, summarize_arrival_distances, sync_open_position_management,
    ArrivalDistanceStats, CopyActivityRow, CopyDatabase, CopyMode, CopyPauseReason, CopyRound,
    CopyTask, CopyTaskInput, CopyTaskStats, PaperPosition,
};
use crate::positions::{Position, PositionOrigin};
use crate::trader::{Error, Result};
use crate::wallets::watch;

/// Decisions read per task for its stats, rounds and analytics.
pub(super) const TASK_ACTIVITY_WINDOW: usize = 10_000;
/// Points of the per-task cumulative P&L trend the task list draws.
const PNL_TREND_POINTS: usize = 30;

/// System-wide copy-trading state derived from config, safety gates and the task set.
#[derive(Clone, Serialize)]
pub struct CopyTradingStatus {
    pub enabled: bool,
    pub live_available: bool,
    pub blocked_reason: Option<&'static str>,
    pub default_mode: String,
    pub default_slippage_pct: f64,
    pub force_stop_blocks: bool,
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub paper_tasks: usize,
    pub live_tasks: usize,
}

/// A task with the figures of the book it runs in and its budget use.
#[derive(Serialize)]
pub struct CopyTaskSummary {
    #[serde(flatten)]
    pub task: CopyTask,
    pub stats: CopyTaskStats,
    pub spent_sol: f64,
    pub remaining_budget_sol: f64,
    pub effective_state: &'static str,
    /// The filter rule the task runs under after its own override.
    pub effective_require_filter_pass: bool,
    /// Cumulative realized P&L after each of the latest closed rounds.
    pub pnl_trend: Vec<f64>,
}

/// Figures across every task. P&L and holdings count paused tasks too (they
/// still hold what they bought); budget and arrival count enabled tasks only.
#[derive(Debug, Default, Serialize)]
pub struct CopyTotals {
    pub realized_pnl_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub open_holdings: usize,
    pub unpriced_holdings: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate_pct: Option<f64>,
    pub active_budget_sol: f64,
    pub active_spent_sol: f64,
    pub active_arrival: ArrivalDistanceStats,
}

impl CopyTotals {
    /// Totals over every task summary; `active_samples` are the live-stream
    /// arrival samples of the enabled tasks.
    pub fn from_summaries(summaries: &[CopyTaskSummary], active_samples: Vec<u64>) -> Self {
        let mut totals = Self::default();
        for summary in summaries {
            let stats = &summary.stats;
            totals.realized_pnl_sol += stats.realized_pnl_sol;
            totals.unrealized_pnl_sol += stats.unrealized_pnl_sol;
            totals.open_holdings += stats.open_positions;
            totals.unpriced_holdings += stats.unpriced_positions;
            totals.wins += stats.wins;
            totals.losses += stats.losses;
            if summary.task.enabled {
                totals.active_budget_sol += summary.task.total_budget_sol;
                totals.active_spent_sol += summary.spent_sol;
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

pub async fn open_database() -> Result<CopyDatabase> {
    tokio::task::spawn_blocking(|| CopyDatabase::shared(crate::chains::active_chain()))
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
}

pub fn build_status(tasks: &[CopyTask]) -> CopyTradingStatus {
    let (enabled, default_mode, default_slippage_pct, force_stop_blocks) =
        crate::config::with_config(|config| {
            (
                config.copy_trading.enabled,
                config.copy_trading.default_mode.clone(),
                config.copy_trading.default_slippage_pct,
                config.copy_trading.block_on_force_stop,
            )
        });
    CopyTradingStatus {
        enabled,
        live_available: live_block_reason().is_none(),
        blocked_reason: if crate::global::is_force_stopped() {
            Some("force_stop")
        } else if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
            Some("loss_limit")
        } else {
            None
        },
        default_mode,
        default_slippage_pct,
        force_stop_blocks,
        total_tasks: tasks.len(),
        active_tasks: tasks.iter().filter(|task| task.enabled).count(),
        paper_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Paper)
            .count(),
        live_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Live)
            .count(),
    }
}

/// Why a task cannot be armed for live right now, if it cannot: the one gate
/// behind both the status flag and the paper-to-live transition.
pub fn live_block_reason() -> Option<&'static str> {
    if !crate::global::is_initialization_complete() || crate::global::is_explore_mode() {
        Some("setup_incomplete")
    } else if crate::global::is_force_stopped() {
        Some("force_stop")
    } else if !crate::config::with_config(|config| config.copy_trading.enabled) {
        Some("copy_trading_disabled")
    } else {
        None
    }
}

pub async fn status() -> Result<CopyTradingStatus> {
    let tasks = open_database().await?.list_tasks().await?;
    Ok(build_status(&tasks))
}

pub async fn list_tasks() -> Result<Vec<CopyTask>> {
    open_database().await?.list_tasks().await
}

pub async fn get_task(id: i64) -> Result<CopyTask> {
    open_database()
        .await?
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })
}

/// Every position the stats may attribute to a live task, whatever its state.
pub(super) async fn all_positions() -> Vec<Position> {
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    positions
}

/// The price a paper holding is marked at: the live pool price only, the same
/// price its exits trade on. Without one the holding counts as unpriced; the last
/// observed trade price is usually its own entry and would hide the real move.
pub(super) fn paper_mark(position: &PaperPosition) -> Option<f64> {
    crate::pools::get_pool_price(&position.mint).map(|price| price.price_sol)
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
    let mut stats = build_task_stats(task.id, activity, positions);
    if task.mode == CopyMode::Paper {
        apply_paper_book(&mut stats, paper_book, mark);
    }
    let rounds = closed_rounds(task.id, stats.book, activity, positions);
    stats.wins = rounds.iter().filter(|round| round.pnl_sol > 0.0).count();
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
    Ok(book_stats(task, &activity, positions, &book, paper_mark).0)
}

pub async fn task_stats(id: i64) -> Result<CopyTaskStats> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    task_stats_for(&db, &task, &all_positions().await).await
}

fn effective_state(status: &CopyTradingStatus, task: &CopyTask) -> &'static str {
    if !status.enabled {
        "system_paused"
    } else if status.blocked_reason == Some("force_stop") {
        "force_stopped"
    } else if !task.enabled {
        "paused"
    } else if status.blocked_reason.is_some() {
        "entries_blocked"
    } else if task.mode == CopyMode::Live {
        "live"
    } else {
        "paper"
    }
}

/// One task's summary from what was already read, with the live-stream arrival
/// samples the overview aggregates across enabled tasks.
pub fn summarize(
    status: &CopyTradingStatus,
    task: CopyTask,
    activity: &[CopyActivityRow],
    positions: &[Position],
    paper_book: &[PaperPosition],
    spent_sol: f64,
    mark: impl Fn(&PaperPosition) -> Option<f64>,
) -> (CopyTaskSummary, Vec<u64>) {
    let (stats, rounds) = book_stats(&task, activity, positions, paper_book, mark);
    let mut cumulative = 0.0;
    let mut pnl_trend = rounds
        .iter()
        .map(|round| {
            cumulative += round.pnl_sol;
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
    let global_filter =
        crate::config::with_config(|config| config.copy_trading.require_filter_pass);
    (
        CopyTaskSummary {
            stats,
            remaining_budget_sol: (task.total_budget_sol - spent_sol).max(0.0),
            spent_sol,
            effective_state: effective_state(status, &task),
            effective_require_filter_pass: task.requires_filter_pass(global_filter),
            pnl_trend,
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
    for (task, (task_activity, book, spent_sol)) in tasks.into_iter().zip(reads) {
        let (summary, samples) = summarize(
            &status,
            task,
            &task_activity,
            &positions,
            &book,
            spent_sol,
            paper_mark,
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

async fn ensure_active_slot(db: &CopyDatabase) -> Result<()> {
    let active = db
        .list_tasks()
        .await?
        .into_iter()
        .filter(|task| task.enabled)
        .count();
    let maximum = crate::config::with_config(|config| config.copy_trading.max_active_tasks);
    if active >= maximum {
        return Err(Error::CopyTaskLimit { maximum });
    }
    Ok(())
}

/// Create a task. New tasks always start in paper mode; arming live is a
/// separate confirmed transition (`set_task_mode`).
pub async fn create_task(input: CopyTaskInput) -> Result<CopyTask> {
    let task = input
        .into_task(crate::chains::active_chain(), Utc::now())
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    crate::chains::adapter()
        .validate_address(&task.target_address)
        .map_err(|e| Error::CopyWatchRejected {
            detail: format!("invalid target wallet: {e}"),
        })?;
    let db = open_database().await?;
    if task.enabled {
        ensure_active_slot(&db).await?;
    }
    let inserted = db.insert_task(task).await?;
    if inserted.enabled {
        if let Err(error) = watch::add_copy_source(
            inserted.id,
            &inserted.target_address,
            inserted.label.as_deref(),
        )
        .await
        {
            let _ = db.delete_task(inserted.id).await;
            return Err(Error::CopyWatchRejected {
                detail: error.to_string(),
            });
        }
    }
    Ok(inserted)
}

/// PATCH semantics: fields present in the patch replace the stored ones, absent
/// fields keep their value, and an unknown field is refused rather than silently
/// dropped. The merged input then runs the full task validation.
pub fn merge_task_patch(original: &CopyTask, patch: serde_json::Value) -> Result<CopyTaskInput> {
    let invalid = |detail: String| Error::CopyValidation { detail };
    let serde_json::Value::Object(fields) = patch else {
        return Err(invalid("the patch must be a JSON object".to_owned()));
    };
    let mut merged =
        serde_json::to_value(CopyTaskInput::from(original)).map_err(|e| invalid(e.to_string()))?;
    let target = merged
        .as_object_mut()
        .ok_or_else(|| invalid("stored task did not serialize to an object".to_owned()))?;
    for (key, value) in fields {
        if !target.contains_key(&key) {
            return Err(invalid(format!("unknown field `{key}`")));
        }
        target.insert(key, value);
    }
    serde_json::from_value(merged).map_err(|e| invalid(e.to_string()))
}

/// Apply a partial update, keeping the watch source and the ownership of the
/// task's open positions consistent with the stored task. The task is written
/// first and every watch or position change after it rolls the task back on
/// failure, so a failed write never leaves the watch source carrying values the
/// task does not. The target wallet is the task's identity: a different wallet
/// is a new task (clone this one).
pub async fn update_task(id: i64, patch: serde_json::Value) -> Result<CopyTask> {
    let db = open_database().await?;
    let original = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let mut task = merge_task_patch(&original, patch)?
        .into_task_for_update(crate::chains::active_chain(), Utc::now(), original.mode)
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    if task.target_address != original.target_address {
        return Err(Error::CopyValidation {
            detail:
                "the target wallet of a task cannot change; clone the task to copy another wallet"
                    .to_owned(),
        });
    }
    task.id = id;
    task.created_at = original.created_at;
    (task.pause_reason, task.paused_at) = if task.enabled {
        (None, None)
    } else if original.enabled {
        (Some(CopyPauseReason::User), Some(Utc::now()))
    } else {
        (original.pause_reason.clone(), original.paused_at)
    };
    if task.enabled && !original.enabled {
        ensure_active_slot(&db).await?;
    }
    let updated = db.update_task(task).await?;
    if updated.enabled {
        if let Err(error) =
            watch::add_copy_source(id, &updated.target_address, updated.label.as_deref()).await
        {
            return Err(match db.update_task(original.clone()).await {
                Ok(_) => Error::CopyWatchRejected {
                    detail: error.to_string(),
                },
                Err(rollback) => Error::CopyReconciliation {
                    detail: format!(
                        "failed to attach the copy target ({error}) and roll back the task ({rollback})"
                    ),
                },
            });
        }
    }
    if original.enabled && !updated.enabled {
        if let Err(error) = watch::remove_copy_source(id, &original.target_address).await {
            let rollback = db.update_task(original.clone()).await;
            return Err(Error::CopyReconciliation {
                detail: match rollback {
                    Ok(_) => format!(
                        "failed to detach the copy target; task update was rolled back: {error}"
                    ),
                    Err(rollback) => format!(
                        "failed to detach the copy target ({error}) and roll back the task ({rollback})"
                    ),
                },
            });
        }
    }
    if let Err(error) = sync_open_position_management(id, updated.exit_mode).await {
        let rollback = db.update_task(original.clone()).await;
        let position_rollback = sync_open_position_management(id, original.exit_mode).await;
        return Err(Error::CopyReconciliation {
            detail: match (rollback, position_rollback) {
                (Ok(_), Ok(())) => format!(
                    "failed to update copy position ownership; task update was rolled back: {error}"
                ),
                (task_result, position_result) => format!(
                    "failed to update copy position ownership ({error}); rollback results: task={task_result:?}, positions={position_result:?}"
                ),
            },
        });
    }
    Ok(updated)
}

/// Delete a task and detach its watch source. Its decisions, spend and paper book
/// go with it (foreign-key cascade). Refused while it could still move money: an
/// enabled live task must be paused first, and a task whose live positions are
/// still open keeps them until they are closed.
pub async fn delete_task(id: i64) -> Result<()> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.enabled && task.mode == CopyMode::Live {
        return Err(Error::CopyTaskLive { task_id: id });
    }
    let open_positions = crate::positions::get_open_positions()
        .await
        .into_iter()
        .filter(|position| {
            matches!(position.origin, PositionOrigin::Copy { task_id, .. } if task_id == id)
        })
        .count();
    if open_positions > 0 {
        return Err(Error::CopyTaskOwnsPositions {
            task_id: id,
            open_positions,
        });
    }
    watch::remove_copy_source(id, &task.target_address)
        .await
        .map_err(|e| Error::CopyReconciliation {
            detail: format!("failed to detach the copy target: {e}"),
        })?;
    if db.delete_task(id).await? {
        Ok(())
    } else {
        Err(Error::CopyTaskNotFound { task_id: id })
    }
}

/// The guarded paper/live transition. Arming live requires the exact
/// `LIVE_ARM_CONFIRMATION` phrase and a runtime that could execute live copies
/// (`live_block_reason`); returning to paper is always allowed.
pub async fn set_task_mode(
    id: i64,
    mode: CopyMode,
    confirmation: Option<String>,
) -> Result<CopyTask> {
    let db = open_database().await?;
    let current = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if mode == CopyMode::Live && current.mode != CopyMode::Live {
        if let Some(reason) = live_block_reason() {
            return Err(Error::CopyLiveUnavailable { reason });
        }
    }
    let mode = confirm_mode_transition(current.mode, mode, confirmation.as_deref())
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    db.set_task_mode(id, mode, confirmation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trader::copy::{ExitMode, SizingMode};

    fn stored() -> CopyTask {
        CopyTask {
            id: 3,
            chain: crate::chains::ChainId::Solana,
            target_address: "target".to_owned(),
            label: Some("Kept".to_owned()),
            enabled: true,
            mode: CopyMode::Paper,
            sizing: SizingMode::Fixed { sol: 0.1 },
            exit_mode: ExitMode::Mirror,
            exit_policy_overrides: Default::default(),
            max_sol_per_trade: 0.2,
            max_sol_per_token: 1.0,
            total_budget_sol: 5.0,
            min_target_trade_sol: None,
            max_target_trade_sol: None,
            buy_once_per_token: false,
            slippage_pct: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            require_filter_pass: None,
            pause_reason: None,
            paused_at: None,
        }
    }

    #[test]
    fn a_partial_patch_changes_only_the_fields_it_names() {
        let merged = merge_task_patch(&stored(), serde_json::json!({ "enabled": false })).unwrap();
        assert!(!merged.enabled);
        assert_eq!(merged.label.as_deref(), Some("Kept"));
        assert_eq!(merged.exit_mode, ExitMode::Mirror);
        assert_eq!(merged.total_budget_sol, 5.0);
    }

    #[test]
    fn the_filter_override_is_patchable_and_inherits_when_null() {
        let merged = merge_task_patch(
            &stored(),
            serde_json::json!({ "require_filter_pass": false }),
        )
        .unwrap();
        assert_eq!(merged.require_filter_pass, Some(false));
        let merged = merge_task_patch(
            &stored(),
            serde_json::json!({ "require_filter_pass": null }),
        )
        .unwrap();
        assert_eq!(merged.require_filter_pass, None);
    }

    fn paper_row(
        id: i64,
        mint: &str,
        at: chrono::DateTime<Utc>,
        sell: Option<f64>,
    ) -> CopyActivityRow {
        use crate::trader::copy::{
            CopyOutcome, CopySellDecision, CopyTelemetry, PaperDecision, PaperFill, PaperSellFill,
        };
        let telemetry = CopyTelemetry {
            target_block_time: Some(at.timestamp() - 1),
            detected_at: at,
            decoded_at: at,
            decided_at: at,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: None,
            backfill: false,
        };
        let (kind, outcome) = match sell {
            None => (
                "paper_filled",
                CopyOutcome::PaperFilled(PaperDecision {
                    task_id: 3,
                    target_address: "target".to_owned(),
                    signature: format!("buy-{id}"),
                    mint: mint.to_owned(),
                    target_size_sol: 1.0,
                    target_token_amount: 100.0,
                    sized_sol: 1.0,
                    fill: PaperFill {
                        input_sol: 1.0,
                        market_price_sol: 0.01,
                        fill_price_sol: 0.01,
                        token_amount: 100.0,
                        referral_fee_sol: 0.0,
                        network_fee_sol: 0.0,
                        priority_fee_sol: 0.0,
                        total_cost_sol: 1.0,
                    },
                    telemetry,
                }),
            ),
            Some(proceeds) => (
                "paper_sell_observed",
                CopyOutcome::PaperSellObserved(CopySellDecision {
                    task_id: 3,
                    target_address: "target".to_owned(),
                    target_signature: format!("sell-{id}"),
                    mint: mint.to_owned(),
                    target_token_amount: 100.0,
                    target_sol_amount: proceeds,
                    exit_percentage: None,
                    transaction_signature: None,
                    error: None,
                    telemetry,
                    paper_fill: Some(PaperSellFill {
                        token_amount: 100.0,
                        market_price_sol: proceeds / 100.0,
                        fill_price_sol: proceeds / 100.0,
                        gross_sol: proceeds,
                        referral_fee_sol: 0.0,
                        network_fee_sol: 0.0,
                        priority_fee_sol: 0.0,
                        net_proceeds_sol: proceeds,
                    }),
                    exit_rule: None,
                }),
            ),
        };
        CopyActivityRow {
            id,
            task_id: 3,
            kind: kind.to_owned(),
            outcome,
            created_at: at,
        }
    }

    #[test]
    fn book_stats_count_wins_and_losses_from_the_closed_rounds() {
        let now = Utc::now();
        let activity = [
            paper_row(1, "won", now, None),
            paper_row(2, "won", now, Some(1.4)),
            paper_row(3, "lost", now, None),
            paper_row(4, "lost", now, Some(0.7)),
            paper_row(5, "open", now, None),
        ];
        let (stats, rounds) = book_stats(&stored(), &activity, &[], &[], |_| None);
        assert_eq!(rounds.len(), 2);
        assert_eq!((stats.wins, stats.losses), (1, 1));
        assert_eq!(stats.filled_buys, 3);
        assert_eq!(stats.target_sells, 2);
    }

    #[test]
    fn a_patch_with_an_unknown_or_mistyped_field_is_refused() {
        assert!(merge_task_patch(&stored(), serde_json::json!({ "enabld": false })).is_err());
        assert!(merge_task_patch(&stored(), serde_json::json!({ "enabled": "no" })).is_err());
        assert!(merge_task_patch(&stored(), serde_json::json!([1])).is_err());
    }
}
