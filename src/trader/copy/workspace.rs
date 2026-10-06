// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The per-task workspace behind the Copy Trading page: detail with a marked
//! paper book and the rules each holding is under, paged activity, analytics,
//! readiness for live, the wallet profile, and paper-book maintenance (close a
//! holding, reset the book, clone the task). Routes and agent tools are both
//! transports over these functions.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::control::{self, open_database, CopyTaskSummary, TASK_ACTIVITY_WINDOW};
use super::insights::CurvePoint;
use super::{
    build_insights, closed_rounds, management_for_exit_mode, notify, ActivityQuery,
    CopyActivityRow, CopyBook, CopyInsights, CopyMode, CopyTask, InsightRange, PaperPosition,
    LIVE_ARM_CONFIRMATION,
};
use crate::config::with_config;
use crate::positions::{Position, PositionManagement};
use crate::trader::policy::{ExitPolicy, RoiPolicy, StopLossPolicy, TimePolicy, TrailingPolicy};
use crate::trader::{Error, Result};

mod book;
mod profile;
mod readiness;

pub use book::{
    clone_task, close_paper_holding, reset_paper_book, CloneRequest, ClosedHolding, ResetResult,
};
pub use profile::{wallet_profile, WalletProfile, WalletWatch};
pub use readiness::{Readiness, ReadinessCheck};

/// Newest decisions shipped with the workspace; the Activity tab pages the rest.
const WORKSPACE_ACTIVITY: usize = 20;
const DEFAULT_PAGE: usize = 50;
const MAX_PAGE: usize = 500;

/// The four exit rules as they apply, serializable for the editor and the Rules tab.
#[derive(Debug, Clone, Serialize)]
pub struct EffectiveExitPolicy {
    pub stop_loss: StopLossPolicy,
    pub trailing: TrailingPolicy,
    pub roi: RoiPolicy,
    pub time: TimePolicy,
}

impl From<&ExitPolicy> for EffectiveExitPolicy {
    fn from(policy: &ExitPolicy) -> Self {
        Self {
            stop_loss: policy.stop_loss.clone(),
            trailing: policy.trailing.clone(),
            roi: policy.roi.clone(),
            time: policy.time.clone(),
        }
    }
}

fn task_policy(task: &CopyTask) -> ExitPolicy {
    let mut policy = ExitPolicy::from_config();
    policy.apply_overrides(&task.exit_policy_overrides);
    policy
}

/// Whether the exit policy manages this task's holdings (`mirror` hands exits
/// to the target's sells alone).
fn policy_manages_exits(task: &CopyTask) -> bool {
    management_for_exit_mode(task.exit_mode) != PositionManagement::CopyTask
}

/// The book a task's results come from: its paper ledger while in paper mode,
/// the real positions it opened while live.
pub fn book_of(task: &CopyTask) -> CopyBook {
    match task.mode {
        CopyMode::Paper => CopyBook::Paper,
        CopyMode::Live => CopyBook::Live,
    }
}

/// Where the task's exit rules act on a holding, from its entry price and peak.
#[derive(Debug, Clone, Serialize)]
pub struct HoldingExitWatch {
    #[serde(rename = "stop_loss_price_sol")]
    pub stop_loss_price_native: Option<f64>,
    /// The stop loss is held off until this moment (its minimum hold).
    pub stop_loss_armed_at: Option<DateTime<Utc>>,
    #[serde(rename = "take_profit_price_sol")]
    pub take_profit_price_native: Option<f64>,
    #[serde(rename = "trailing_activation_price_sol")]
    pub trailing_activation_price_native: Option<f64>,
    pub trailing_armed: bool,
    #[serde(rename = "trailing_stop_price_sol")]
    pub trailing_stop_price_native: Option<f64>,
    /// From this moment the time rule sells while the price is at or below
    /// `time_rule_price_native`.
    pub time_rule_from: Option<DateTime<Utc>>,
    #[serde(rename = "time_rule_price_sol")]
    pub time_rule_price_native: Option<f64>,
}

fn exit_watch(position: &PaperPosition, entry: f64, policy: &ExitPolicy) -> HoldingExitWatch {
    let stop = &policy.stop_loss;
    let trailing = &policy.trailing;
    let time = &policy.time;
    let activation = trailing
        .enabled
        .then(|| entry * (1.0 + trailing.activation_pct / 100.0));
    let armed_peak = position
        .peak_price_native
        .filter(|peak| activation.is_some_and(|activation| *peak >= activation));
    HoldingExitWatch {
        stop_loss_price_native: stop
            .enabled
            .then(|| entry * (1.0 - stop.threshold_pct / 100.0)),
        stop_loss_armed_at: (stop.enabled && stop.min_hold_seconds > 0).then(|| {
            position.opened_at
                + Duration::seconds(stop.min_hold_seconds.min(i64::MAX as u64) as i64)
        }),
        take_profit_price_native: policy
            .roi
            .enabled
            .then(|| entry * (1.0 + policy.roi.target_profit_pct / 100.0)),
        trailing_activation_price_native: activation,
        trailing_armed: armed_peak.is_some(),
        trailing_stop_price_native: armed_peak
            .map(|peak| peak * (1.0 - trailing.distance_pct / 100.0)),
        time_rule_from: time.enabled.then(|| {
            position.opened_at + Duration::milliseconds((time.duration_seconds * 1000.0) as i64)
        }),
        time_rule_price_native: time
            .enabled
            .then(|| entry * (1.0 + time.loss_threshold_pct / 100.0)),
    }
}

/// One paper-ledger holding marked at the same price the task stats use.
#[derive(Debug, Serialize)]
pub struct PaperHolding {
    pub mint: String,
    pub open: bool,
    pub token_amount: f64,
    #[serde(rename = "cost_basis_sol")]
    pub cost_basis_native: f64,
    #[serde(rename = "invested_sol")]
    pub invested_native: f64,
    #[serde(rename = "realized_proceeds_sol")]
    pub realized_proceeds_native: f64,
    #[serde(rename = "realized_pnl_sol")]
    pub realized_pnl_native: f64,
    #[serde(rename = "entry_price_sol")]
    pub entry_price_native: Option<f64>,
    #[serde(rename = "mark_price_sol")]
    pub mark_price_native: Option<f64>,
    #[serde(rename = "market_value_sol")]
    pub market_value_native: Option<f64>,
    #[serde(rename = "unrealized_pnl_sol")]
    pub unrealized_pnl_native: Option<f64>,
    pub unrealized_pnl_pct: Option<f64>,
    pub buys: u64,
    pub sells: u64,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub held_seconds: i64,
    /// Highest pool price of the open round; what arms the paper trailing stop.
    #[serde(rename = "peak_price_sol")]
    pub peak_price_native: Option<f64>,
    /// `None` when the policy does not manage this task's exits.
    pub exit_watch: Option<HoldingExitWatch>,
}

/// `mark` is the holding's pool price; a closed holding is never marked, and a
/// price the stats would not mark at is no mark here either.
fn paper_holding(
    position: &PaperPosition,
    policy: Option<&ExitPolicy>,
    mark: Option<f64>,
    now: DateTime<Utc>,
) -> PaperHolding {
    let open = position.is_open();
    let mark = mark.filter(|price| open && price.is_finite() && *price > 0.0);
    let entry = (open && position.token_amount > 0.0)
        .then(|| position.cost_basis_native / position.token_amount);
    let market_value = mark.map(|price| position.token_amount * price);
    let unrealized = market_value.map(|value| value - position.cost_basis_native);
    PaperHolding {
        mint: position.mint.clone(),
        open,
        token_amount: position.token_amount,
        cost_basis_native: position.cost_basis_native,
        invested_native: position.invested_native,
        realized_proceeds_native: position.realized_proceeds_native,
        realized_pnl_native: position.realized_proceeds_native - position.realized_cost_native,
        entry_price_native: entry,
        mark_price_native: mark,
        market_value_native: market_value,
        unrealized_pnl_native: unrealized,
        unrealized_pnl_pct: unrealized
            .filter(|_| position.cost_basis_native > 0.0)
            .map(|pnl| pnl / position.cost_basis_native * 100.0),
        buys: position.buys,
        sells: position.sells,
        opened_at: position.opened_at,
        closed_at: position.closed_at,
        held_seconds: (position.closed_at.unwrap_or(now) - position.opened_at).num_seconds(),
        peak_price_native: position.peak_price_native.filter(|_| open),
        exit_watch: entry
            .zip(policy)
            .map(|(entry, policy)| exit_watch(position, entry, policy)),
    }
}

/// Everything the task workspace shows, in one read.
#[derive(Serialize)]
pub struct CopyTaskWorkspace {
    #[serde(flatten)]
    pub summary: CopyTaskSummary,
    /// The Trader values a task inherits for every rule it does not override.
    pub trader_defaults: EffectiveExitPolicy,
    /// The rules after this task's overrides.
    pub effective_policy: EffectiveExitPolicy,
    /// What the task may still spend once live. Paper and live spend are separate
    /// ledgers, so this is the budget less live spend only, whatever paper spent.
    #[serde(rename = "live_remaining_budget_sol")]
    pub live_remaining_budget_native: f64,
    pub policy_manages_exits: bool,
    pub global_require_filter_pass: bool,
    pub paper_holdings: Vec<PaperHolding>,
    pub readiness: Readiness,
    pub activity: Vec<CopyActivityRow>,
}

pub async fn task_workspace(id: i64) -> Result<CopyTaskWorkspace> {
    let db = open_database().await?;
    let tasks = db.list_tasks().await?;
    let status = control::build_status(&tasks);
    let task = tasks
        .into_iter()
        .find(|task| task.id == id)
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let positions = control::all_positions().await;
    let activity = db.list_task_activity(id, TASK_ACTIVITY_WINDOW).await?;
    let paper_book = db.paper_positions(id).await?;
    let spent_native = db.task_total_spent(id, task.mode).await?;
    let live_spent_native = db.task_total_spent(id, CopyMode::Live).await?;
    let (summary, _) = control::summarize(
        &status,
        task,
        &activity,
        &positions,
        &paper_book,
        spent_native,
        control::paper_mark,
    );
    Ok(build_workspace(
        summary,
        live_spent_native,
        &activity,
        &positions,
        &paper_book,
        control::paper_mark,
        control::live_block_reason(),
        Utc::now(),
    ))
}

/// The workspace from what was already read: the task's `live_spent_native`,
/// `activity` newest first, the task's paper ledger (read in either mode; a live
/// task can still hold paper history), `mark` pricing a paper holding and `block`
/// why live execution is unavailable.
#[allow(clippy::too_many_arguments)]
pub fn build_workspace(
    summary: CopyTaskSummary,
    live_spent_native: f64,
    activity: &[CopyActivityRow],
    positions: &[Position],
    paper_book: &[PaperPosition],
    mark: impl Fn(&PaperPosition) -> Option<f64>,
    block: Option<&'static str>,
    now: DateTime<Utc>,
) -> CopyTaskWorkspace {
    let policy = task_policy(&summary.task);
    let manages = policy_manages_exits(&summary.task);
    let paper_holdings = paper_book
        .iter()
        .map(|position| paper_holding(position, manages.then_some(&policy), mark(position), now))
        .collect::<Vec<_>>();
    let paper_rounds = closed_rounds(summary.task.id, CopyBook::Paper, activity, positions);
    let readiness = readiness::readiness(&summary, &paper_rounds, &paper_holdings, block);
    CopyTaskWorkspace {
        live_remaining_budget_native: (summary.task.total_budget_native - live_spent_native)
            .max(0.0),
        summary,
        trader_defaults: (&ExitPolicy::from_config()).into(),
        effective_policy: (&policy).into(),
        policy_manages_exits: manages,
        global_require_filter_pass: with_config(|config| config.copy_trading.require_filter_pass),
        paper_holdings,
        readiness,
        activity: activity.iter().take(WORKSPACE_ACTIVITY).cloned().collect(),
    }
}

/// Activity query string: `filter` is one of all/fills/exits/skips/errors.
#[derive(Debug, Default, Deserialize)]
pub struct ActivityFilter {
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub before: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ActivityPage {
    pub activity: Vec<CopyActivityRow>,
    /// Cursor for the next (older) page; absent on the last page.
    pub next_before: Option<i64>,
}

impl ActivityPage {
    /// A page from the rows a query returned; a full page carries the cursor to
    /// the next, older one.
    pub fn new(activity: Vec<CopyActivityRow>, limit: usize) -> Self {
        let next_before = (activity.len() == limit)
            .then(|| activity.last().map(|row| row.id))
            .flatten();
        Self {
            activity,
            next_before,
        }
    }
}

pub async fn activity_page(task_id: Option<i64>, filter: ActivityFilter) -> Result<ActivityPage> {
    let query = activity_query(task_id, filter)?;
    let limit = query.limit;
    let activity = open_database().await?.activity_page(query).await?;
    Ok(ActivityPage::new(activity, limit))
}

/// The store query behind an activity page: the filter's decision kinds, the
/// token, the keyset cursor and a bounded page size.
pub fn activity_query(task_id: Option<i64>, filter: ActivityFilter) -> Result<ActivityQuery> {
    let kinds = match filter.filter.as_deref().unwrap_or("all") {
        "" | "all" => Vec::new(),
        "fills" => vec!["paper_filled", "live_submitted", "live_confirmed"],
        "exits" => vec!["paper_sell_observed", "live_sell_submitted"],
        "skips" => vec!["skipped"],
        "errors" => vec!["live_failed", "live_sell_failed"],
        other => {
            return Err(Error::CopyValidation {
                detail: format!("unknown activity filter `{other}`"),
            })
        }
    };
    Ok(ActivityQuery {
        task_id,
        before_id: filter.before,
        kinds,
        mint: filter.mint.filter(|mint| !mint.trim().is_empty()),
        limit: filter.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE),
    })
}

/// Insight window as query parameters (RFC 3339).
#[derive(Debug, Default, Clone, Copy, Deserialize)]
pub struct RangeQuery {
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
}

impl From<RangeQuery> for InsightRange {
    fn from(query: RangeQuery) -> Self {
        Self {
            from: query.from,
            to: query.to,
        }
    }
}

pub async fn task_insights(id: i64, range: InsightRange) -> Result<CopyInsights> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let activity = db.list_task_activity(id, TASK_ACTIVITY_WINDOW).await?;
    let positions = control::all_positions().await;
    Ok(build_insights(
        id,
        book_of(&task),
        &activity,
        &positions,
        range,
        Some(arrival_limit_ms()),
    ))
}

/// The arrival limit the latency kill switch and the readiness check hold trades to.
pub fn arrival_limit_ms() -> u64 {
    with_config(|config| config.copy_trading.max_arrival_distance_ms)
}

/// One row of the task comparison.
#[derive(Debug, Serialize)]
pub struct TaskComparison {
    pub task_id: i64,
    pub name: String,
    pub target_address: String,
    pub mode: CopyMode,
    pub enabled: bool,
    pub rounds: usize,
    pub wins: usize,
    pub win_rate_pct: Option<f64>,
    #[serde(rename = "realized_pnl_sol")]
    pub realized_pnl_native: f64,
    pub profit_factor: Option<f64>,
    pub average_hold_seconds: Option<f64>,
    pub arrival_median_ms: Option<u64>,
    pub slippage_median_pct: Option<f64>,
    pub fills: usize,
    pub skips: usize,
    pub pnl_curve: Vec<CurvePoint>,
}

pub fn comparison(task: &CopyTask, insights: CopyInsights) -> TaskComparison {
    TaskComparison {
        task_id: task.id,
        name: notify::task_name(task),
        target_address: task.target_address.clone(),
        mode: task.mode,
        enabled: task.enabled,
        rounds: insights.rounds,
        wins: insights.wins,
        win_rate_pct: insights.win_rate_pct,
        realized_pnl_native: insights.realized_pnl_native,
        profit_factor: insights.profit_factor,
        average_hold_seconds: insights.average_hold_seconds,
        arrival_median_ms: insights.arrival.median_ms,
        slippage_median_pct: insights.slippage.median_pct,
        fills: insights.decisions.fills,
        skips: insights.decisions.skips,
        pnl_curve: insights.pnl_curve,
    }
}

async fn compare(tasks: &[CopyTask], range: InsightRange) -> Result<Vec<TaskComparison>> {
    let db = open_database().await?;
    let positions = control::all_positions().await;
    let activities = futures::future::try_join_all(
        tasks
            .iter()
            .map(|task| db.list_task_activity(task.id, TASK_ACTIVITY_WINDOW)),
    )
    .await?;
    Ok(tasks
        .iter()
        .zip(activities)
        .map(|(task, activity)| {
            let insights =
                build_insights(task.id, book_of(task), &activity, &positions, range, None);
            comparison(task, insights)
        })
        .collect())
}

/// Every task side by side over the same window.
pub async fn compare_tasks(range: InsightRange) -> Result<Vec<TaskComparison>> {
    compare(&control::list_tasks().await?, range).await
}

/// What a new task starts from and inherits.
#[derive(Debug, Serialize)]
pub struct CopyDefaults {
    pub trader_defaults: EffectiveExitPolicy,
    pub require_filter_pass: bool,
    pub default_slippage_pct: f64,
    pub min_slippage_pct: f64,
    pub max_slippage_pct: f64,
    /// The smallest copy sizing will place; a smaller size or per-trade cap never copies.
    pub min_trade_size_native: f64,
    /// The swap fee every paper and live fill pays on each side, in percent.
    pub swap_fee_pct: f64,
    pub max_active_tasks: usize,
    pub latency_kill_switch_enabled: bool,
    pub max_arrival_distance_ms: u64,
    pub latency_window_size: usize,
    pub readiness_min_closed_rounds: usize,
    pub live_confirmation: &'static str,
}

pub fn defaults() -> CopyDefaults {
    let config = with_config(|config| config.copy_trading.clone());
    CopyDefaults {
        trader_defaults: (&ExitPolicy::from_config()).into(),
        require_filter_pass: config.require_filter_pass,
        default_slippage_pct: config.default_slippage_pct,
        min_slippage_pct: super::MIN_COPY_SLIPPAGE_PCT,
        max_slippage_pct: crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT,
        min_trade_size_native: crate::trader::constants::MIN_TRADE_SIZE_NATIVE,
        swap_fee_pct: f64::from(super::PAPER_REFERRAL_FEE_BPS) / 100.0,
        max_active_tasks: config.max_active_tasks,
        latency_kill_switch_enabled: config.latency_kill_switch_enabled,
        max_arrival_distance_ms: config.max_arrival_distance_ms,
        latency_window_size: config.latency_window_size,
        readiness_min_closed_rounds: config.readiness_min_closed_rounds,
        live_confirmation: LIVE_ARM_CONFIRMATION,
    }
}
