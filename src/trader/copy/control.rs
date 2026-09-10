//! Copy-task lifecycle: the one owner of create/update/delete/mode transitions,
//! the overview and the per-task books. The webserver routes and the agent tools
//! are both transports over these functions, so a guard added here (task limit,
//! watch attach/detach with rollback, live-delete refusal) covers every caller.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{
    apply_paper_book, build_task_stats, confirm_mode_transition, sync_open_position_management,
    CopyActivityRow, CopyDatabase, CopyMode, CopyTask, CopyTaskInput, CopyTaskStats, PaperPosition,
};
use crate::positions::{Position, PositionOrigin};
use crate::trader::{Error, Result};
use crate::wallets::watch;

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
}

#[derive(Serialize)]
pub struct CopyTradingOverview {
    pub status: CopyTradingStatus,
    pub tasks: Vec<CopyTaskSummary>,
    pub activity: Vec<CopyActivityRow>,
}

/// One paper-ledger holding marked at the same price the task stats use.
#[derive(Serialize)]
pub struct PaperHolding {
    pub mint: String,
    pub open: bool,
    pub token_amount: f64,
    pub cost_basis_sol: f64,
    pub invested_sol: f64,
    pub realized_proceeds_sol: f64,
    pub realized_pnl_sol: f64,
    pub mark_price_sol: Option<f64>,
    pub market_value_sol: Option<f64>,
    pub unrealized_pnl_sol: Option<f64>,
    pub unrealized_pnl_pct: Option<f64>,
    pub buys: u64,
    pub sells: u64,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// Highest pool price of the open round; what arms the paper trailing stop.
    pub peak_price_sol: Option<f64>,
}

/// Everything known about one task: its summary, paper book and recent decisions.
#[derive(Serialize)]
pub struct CopyTaskDetail {
    #[serde(flatten)]
    pub summary: CopyTaskSummary,
    pub paper_holdings: Vec<PaperHolding>,
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
async fn all_positions() -> Vec<Position> {
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    positions
}

/// The price a paper holding is marked at: the live pool price, else the last
/// observed trade price.
fn paper_mark(position: &PaperPosition) -> Option<f64> {
    crate::pools::get_pool_price(&position.mint)
        .map(|price| price.price_sol)
        .or(position.last_price_sol)
}

/// Decision counts and latency come from the task's activity; position and P&L
/// figures come from the book of the mode it runs in -- its paper ledger while in
/// paper mode, the real positions it opened while live.
pub async fn task_stats_for(
    db: &CopyDatabase,
    task: &CopyTask,
    positions: &[Position],
) -> Result<CopyTaskStats> {
    let activity = db.list_task_activity(task.id, 10_000).await?;
    let mut stats = build_task_stats(task.id, &activity, positions);
    if task.mode == CopyMode::Paper {
        let book = db.paper_positions(task.id).await?;
        apply_paper_book(&mut stats, &book, paper_mark);
    }
    Ok(stats)
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

async fn summarize(
    db: &CopyDatabase,
    status: &CopyTradingStatus,
    task: CopyTask,
    positions: &[Position],
) -> Result<CopyTaskSummary> {
    let stats = task_stats_for(db, &task, positions).await?;
    let spent_sol = db.task_total_spent(task.id, task.mode).await?;
    Ok(CopyTaskSummary {
        stats,
        remaining_budget_sol: (task.total_budget_sol - spent_sol).max(0.0),
        spent_sol,
        effective_state: effective_state(status, &task),
        task,
    })
}

pub async fn overview(activity_limit: usize) -> Result<CopyTradingOverview> {
    let db = open_database().await?;
    let tasks = db.list_tasks().await?;
    let activity = db.list_activity(activity_limit).await?;
    let positions = all_positions().await;
    let status = build_status(&tasks);
    let mut summaries = Vec::with_capacity(tasks.len());
    for task in tasks {
        summaries.push(summarize(&db, &status, task, &positions).await?);
    }
    Ok(CopyTradingOverview {
        status,
        tasks: summaries,
        activity,
    })
}

pub async fn task_detail(id: i64, activity_limit: usize) -> Result<CopyTaskDetail> {
    let db = open_database().await?;
    let tasks = db.list_tasks().await?;
    let status = build_status(&tasks);
    let task = tasks
        .into_iter()
        .find(|task| task.id == id)
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let summary = summarize(&db, &status, task, &all_positions().await).await?;
    let paper_holdings = db
        .paper_positions(id)
        .await?
        .iter()
        .map(paper_holding)
        .collect();
    let activity = db.list_task_activity(id, activity_limit).await?;
    Ok(CopyTaskDetail {
        summary,
        paper_holdings,
        activity,
    })
}

fn paper_holding(position: &PaperPosition) -> PaperHolding {
    let open = position.is_open();
    let mark = if open { paper_mark(position) } else { None };
    let market_value = mark.map(|price| position.token_amount * price);
    let unrealized = market_value.map(|value| value - position.cost_basis_sol);
    PaperHolding {
        mint: position.mint.clone(),
        open,
        token_amount: position.token_amount,
        cost_basis_sol: position.cost_basis_sol,
        invested_sol: position.invested_sol,
        realized_proceeds_sol: position.realized_proceeds_sol,
        realized_pnl_sol: position.realized_proceeds_sol - position.realized_cost_sol,
        mark_price_sol: mark,
        market_value_sol: market_value,
        unrealized_pnl_sol: unrealized,
        unrealized_pnl_pct: unrealized
            .filter(|_| position.cost_basis_sol > 0.0)
            .map(|pnl| pnl / position.cost_basis_sol * 100.0),
        buys: position.buys,
        sells: position.sells,
        opened_at: position.opened_at,
        closed_at: position.closed_at,
        peak_price_sol: position.peak_price_sol.filter(|_| open),
    }
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
/// task's open positions consistent with the stored task; any failure after the
/// write rolls the task back.
pub async fn update_task(id: i64, patch: serde_json::Value) -> Result<CopyTask> {
    let db = open_database().await?;
    let original = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let mut task = merge_task_patch(&original, patch)?
        .into_task_for_update(crate::chains::active_chain(), Utc::now(), original.mode)
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    task.id = id;
    task.created_at = original.created_at;
    if task.enabled && !original.enabled {
        ensure_active_slot(&db).await?;
    }
    if task.enabled {
        watch::add_copy_source(id, &task.target_address, task.label.as_deref())
            .await
            .map_err(|e| Error::CopyWatchRejected {
                detail: e.to_string(),
            })?;
    }
    let new_address = task.target_address.clone();
    let source_was_added =
        task.enabled && (!original.enabled || original.target_address != new_address);
    let updated = match db.update_task(task).await {
        Ok(updated) => updated,
        Err(error) => {
            if source_was_added {
                let _ = watch::remove_copy_source(id, &new_address).await;
            }
            return Err(error);
        }
    };
    if original.enabled && (original.target_address != updated.target_address || !updated.enabled) {
        if let Err(error) = watch::remove_copy_source(id, &original.target_address).await {
            let database_rollback = db.update_task(original.clone()).await;
            if source_was_added {
                let _ = watch::remove_copy_source(id, &new_address).await;
            }
            return Err(Error::CopyReconciliation {
                detail: match database_rollback {
                    Ok(_) => format!(
                        "failed to detach the previous copy target; task update was rolled back: {error}"
                    ),
                    Err(rollback) => format!(
                        "failed to detach the previous copy target ({error}) and roll back the task ({rollback})"
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
    fn a_patch_with_an_unknown_or_mistyped_field_is_refused() {
        assert!(merge_task_patch(&stored(), serde_json::json!({ "enabld": false })).is_err());
        assert!(merge_task_patch(&stored(), serde_json::json!({ "enabled": "no" })).is_err());
        assert!(merge_task_patch(&stored(), serde_json::json!([1])).is_err());
    }
}
