//! Promo generator for Copy Trading: every read the Copy Trading page, the
//! header's copy card and the wallet profile make, answered from one promo desk
//! (`desk.rs`). The answers come from the product's own stats, rounds, insights
//! and workspace assembly, so the wallet list, the workspace tabs, the comparison
//! and the profile reconcile with each other, with the Positions page (the live
//! tasks' positions carry their copy origin there) and with the Watched tab.

mod desk;

use chrono::Utc;

use crate::trader::copy::control::{
    self, CopyTaskSummary, CopyTotals, CopyTradingOverview, CopyTradingStatus,
};
use crate::trader::copy::workspace::{
    self, ActivityFilter, ActivityPage, CopyTaskWorkspace, TaskComparison, WalletProfile,
    WalletWatch,
};
use crate::trader::copy::{
    build_insights, CopyActivityRow, CopyInsights, CopyMode, CopyTask, CopyTaskStats, InsightRange,
};
use crate::trader::{Error, Result};
use crate::webserver::routes::header::CopyHeaderInfo;

use super::data::PROMO_WALLET_ADDRESS;
use desk::Desk;

pub(super) use desk::{position_owner, PROMO_COPY_WALLETS};

/// Copy processing on, nothing blocking it, live execution available: the
/// state a capture shows.
fn status(tasks: &[CopyTask]) -> CopyTradingStatus {
    let running = |mode: CopyMode| {
        tasks
            .iter()
            .filter(|task| task.enabled && task.mode == mode)
            .count()
    };
    CopyTradingStatus {
        enabled: true,
        live_available: true,
        blocked_reason: None,
        default_mode: "paper".to_owned(),
        default_slippage_pct: 1.5,
        force_stop_blocks: true,
        total_tasks: tasks.len(),
        active_tasks: tasks.iter().filter(|task| task.enabled).count(),
        paper_tasks: running(CopyMode::Paper),
        live_tasks: running(CopyMode::Live),
    }
}

fn find(desk: &Desk, id: i64) -> Result<&CopyTask> {
    desk.task(id).ok_or(Error::CopyTaskNotFound { task_id: id })
}

fn summarize(
    desk: &Desk,
    status: &CopyTradingStatus,
    task: &CopyTask,
) -> (CopyTaskSummary, Vec<u64>) {
    control::summarize(
        status,
        task.clone(),
        &desk.task_activity(task.id),
        &desk.positions,
        &desk.paper_book(task.id),
        desk.spent_sol(task.id),
        desk::mark,
    )
}

fn insights(desk: &Desk, task: &CopyTask, range: InsightRange) -> CopyInsights {
    build_insights(
        task.id,
        workspace::book_of(task),
        &desk.task_activity(task.id),
        &desk.positions,
        range,
    )
}

/// The overview: status strip, totals, one summary per task, the decision feed.
pub fn get_promo_copy_trading_overview(activity_limit: usize) -> CopyTradingOverview {
    let desk = Desk::build();
    let status = status(&desk.tasks);
    let mut active_samples = Vec::new();
    let mut summaries = Vec::with_capacity(desk.tasks.len());
    for task in &desk.tasks {
        let (summary, samples) = summarize(&desk, &status, task);
        if task.enabled {
            active_samples.extend(samples);
        }
        summaries.push(summary);
    }
    CopyTradingOverview {
        status,
        totals: CopyTotals::from_summaries(&summaries, active_samples),
        tasks: summaries,
        activity: desk.activity.into_iter().take(activity_limit).collect(),
    }
}

pub fn get_promo_copy_status() -> CopyTradingStatus {
    status(&desk::tasks(Utc::now()))
}

pub fn get_promo_copy_tasks() -> Vec<CopyTask> {
    desk::tasks(Utc::now())
}

pub fn get_promo_copy_task(id: i64) -> Result<CopyTask> {
    get_promo_copy_tasks()
        .into_iter()
        .find(|task| task.id == id)
        .ok_or(Error::CopyTaskNotFound { task_id: id })
}

pub fn get_promo_copy_task_stats(id: i64) -> Result<CopyTaskStats> {
    let desk = Desk::build();
    let task = find(&desk, id)?;
    Ok(control::book_stats(
        task,
        &desk.task_activity(id),
        &desk.positions,
        &desk.paper_book(id),
        desk::mark,
    )
    .0)
}

pub fn get_promo_copy_workspace(id: i64) -> Result<CopyTaskWorkspace> {
    let desk = Desk::build();
    let status = status(&desk.tasks);
    let task = find(&desk, id)?;
    let activity = desk.task_activity(id);
    let paper_book = desk.paper_book(id);
    let (summary, _) = control::summarize(
        &status,
        task.clone(),
        &activity,
        &desk.positions,
        &paper_book,
        desk.spent_sol(id),
        desk::mark,
    );
    // Nothing blocks live execution in a capture, as `status` reports.
    Ok(workspace::build_workspace(
        summary,
        &activity,
        &desk.positions,
        &paper_book,
        desk::mark,
        None,
        Utc::now(),
    ))
}

/// A page of decisions, selected and paged exactly as the store's query does.
pub fn get_promo_copy_activity(
    task_id: Option<i64>,
    filter: ActivityFilter,
) -> Result<ActivityPage> {
    let query = workspace::activity_query(task_id, filter)?;
    let activity = Desk::build()
        .activity
        .into_iter()
        .filter(|row| query.task_id.is_none_or(|id| row.task_id == id))
        .filter(|row| query.before_id.is_none_or(|before| row.id < before))
        .filter(|row| query.kinds.is_empty() || query.kinds.contains(&row.kind.as_str()))
        .filter(|row| {
            query
                .mint
                .as_deref()
                .is_none_or(|mint| desk::outcome_mint(&row.outcome) == Some(mint))
        })
        .take(query.limit)
        .collect();
    Ok(ActivityPage::new(activity, query.limit))
}

pub fn get_promo_copy_recent_activity(limit: usize) -> Vec<CopyActivityRow> {
    Desk::build().activity.into_iter().take(limit).collect()
}

pub fn get_promo_copy_insights(id: i64, range: InsightRange) -> Result<CopyInsights> {
    let desk = Desk::build();
    let task = find(&desk, id)?;
    Ok(insights(&desk, task, range))
}

pub fn get_promo_copy_comparison(range: InsightRange) -> Vec<TaskComparison> {
    let desk = Desk::build();
    desk.tasks
        .iter()
        .map(|task| workspace::comparison(task, insights(&desk, task, range)))
        .collect()
}

/// The wallet profile: the Watched tab's row for the address, the target trades
/// its tasks observed, and each task's all-time results.
pub fn get_promo_copy_wallet_profile(address: &str) -> Result<WalletProfile> {
    crate::chains::adapter()
        .validate_address(address)
        .map_err(|e| Error::CopyValidation {
            detail: format!("invalid wallet address: {e}"),
        })?;
    let desk = Desk::build();
    let tasks = desk
        .tasks
        .iter()
        .filter(|task| task.target_address == address)
        .collect::<Vec<_>>();
    let task_ids = tasks.iter().map(|task| task.id).collect::<Vec<_>>();
    let watch = super::wallets::get_promo_watch_targets()
        .into_iter()
        .find(|target| target.address == address)
        .and_then(|target| super::wallets::get_promo_watch_status(target.id?))
        .map(|status| WalletWatch {
            label: status.target.label,
            enabled: status.target.enabled,
            sources: status.target.sources.len(),
            subscribed: status.subscribed,
            last_activity_at: status.last_activity_at,
            last_error: status.last_error,
        });
    Ok(WalletProfile {
        address: address.to_owned(),
        own_wallet: address == PROMO_WALLET_ADDRESS,
        watch,
        observations: desk.observations(&task_ids),
        tasks: tasks
            .iter()
            .map(|task| workspace::comparison(task, insights(&desk, task, InsightRange::default())))
            .collect(),
    })
}

/// The header's copy card, counting the tasks the page lists. Notices belong to
/// a running process's own decisions, so a capture session has none.
pub(super) fn get_promo_copy_header() -> CopyHeaderInfo {
    let status = get_promo_copy_status();
    CopyHeaderInfo {
        enabled: status.enabled,
        total_tasks: status.total_tasks,
        paper_tasks: status.paper_tasks,
        live_tasks: status.live_tasks,
        notices: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trader::copy::CopyOutcome;

    fn ensure_config() {
        use crate::config::schemas::Config;
        let _ =
            crate::config::utils::CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
    }

    #[test]
    fn the_list_the_comparison_and_the_totals_agree() {
        ensure_config();
        let overview = get_promo_copy_trading_overview(50);
        let comparison = get_promo_copy_comparison(InsightRange::default());
        assert_eq!(overview.tasks.len(), 3);
        for summary in &overview.tasks {
            let row = comparison
                .iter()
                .find(|row| row.task_id == summary.task.id)
                .expect("every task is compared");
            assert!(row.rounds > 0, "task {} has closed rounds", row.task_id);
            assert_eq!(summary.stats.wins, row.wins);
            assert_eq!(summary.stats.wins + summary.stats.losses, row.rounds);
            assert!((summary.stats.realized_pnl_sol - row.realized_pnl_sol).abs() < 1e-9);
            assert_eq!(summary.stats.unpriced_positions, 0);
            assert!(summary.remaining_budget_sol < summary.task.total_budget_sol);
            let stats = get_promo_copy_task_stats(summary.task.id).unwrap();
            assert_eq!(
                (stats.wins, stats.losses),
                (summary.stats.wins, summary.stats.losses)
            );
        }
        let wins: usize = overview.tasks.iter().map(|task| task.stats.wins).sum();
        assert_eq!(overview.totals.wins, wins);
        assert_eq!(overview.totals.unpriced_holdings, 0);
        assert_eq!(get_promo_copy_header().total_tasks, 3);
    }

    #[test]
    fn the_paper_task_shows_a_ready_book_under_its_own_rules() {
        ensure_config();
        let workspace = get_promo_copy_workspace(2).unwrap();
        assert!(workspace.policy_manages_exits);
        assert!(
            workspace.readiness.ready,
            "{:?}",
            workspace.readiness.checks
        );
        let open = workspace
            .paper_holdings
            .iter()
            .filter(|holding| holding.open)
            .collect::<Vec<_>>();
        assert_eq!(open.len(), 3);
        assert!(open.iter().all(|holding| holding.mark_price_sol.is_some()));
        let armed = open
            .iter()
            .filter(|holding| {
                holding
                    .exit_watch
                    .as_ref()
                    .is_some_and(|watch| watch.trailing_armed)
            })
            .count();
        assert_eq!(armed, 1);
        assert_eq!(workspace.summary.stats.open_positions, 3);
        assert!(workspace.summary.stats.policy_exits > 0);
        assert!(workspace.summary.stats.target_sells > 0);
    }

    #[test]
    fn activity_is_filtered_and_paged_like_the_store() {
        ensure_config();
        let filter = |before| ActivityFilter {
            filter: Some("exits".to_owned()),
            mint: None,
            before,
            limit: Some(5),
        };
        let first = get_promo_copy_activity(Some(2), filter(None)).unwrap();
        assert_eq!(first.activity.len(), 5);
        assert!(first.activity.iter().all(
            |row| row.task_id == 2 && matches!(row.outcome, CopyOutcome::PaperSellObserved(_))
        ));
        let cursor = first.next_before.expect("a full page has a cursor");
        let second = get_promo_copy_activity(Some(2), filter(Some(cursor))).unwrap();
        assert!(second.activity.iter().all(|row| row.id < cursor));
        let everything = get_promo_copy_recent_activity(500);
        assert!(everything.windows(2).all(|pair| pair[0].id > pair[1].id));
    }

    #[test]
    fn live_copy_positions_carry_their_task_and_replays_stay_out_of_arrival() {
        ensure_config();
        let (origin, _) = position_owner(super::super::data::PROMO_OPEN_TOKENS[0].2);
        assert!(matches!(
            origin,
            crate::positions::PositionOrigin::Copy { task_id: 1, .. }
        ));
        let (origin, _) = position_owner(super::super::data::PROMO_OPEN_TOKENS[6].2);
        assert!(matches!(
            origin,
            crate::positions::PositionOrigin::Auto { .. }
        ));
        let insights = get_promo_copy_insights(2, InsightRange::default()).unwrap();
        assert!(insights.arrival.maximum_ms.unwrap() < 4_000);
        let profile = get_promo_copy_wallet_profile(PROMO_COPY_WALLETS[1].2).unwrap();
        assert_eq!(profile.tasks.len(), 1);
        assert!(profile.watch.is_some_and(|watch| watch.subscribed));
        assert!(profile.observations.sells > 0);
    }
}
