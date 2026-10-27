// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Runtime paper/live consumer of the shared wallet-observation broadcast.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Notify;

use super::paper_execution::{
    activity_key, as_paper, execute_paper_entry, skipped, skipped_activity, skipped_paper,
    EnqueueError, PaperDispatch, PaperExecutionHandle,
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::Book;
use crate::wallets::watch::{
    subscribe_activity, ActivityKind, SwapSide, WalletActivity, WatchSource,
};

use super::{
    execute_copy_sell_with, execute_live_with, matching_tasks, notify, prepare_copy_sell,
    prepare_entry, CopyDatabase, CopyOutcome, CopySellSubmitResult, CopySkip, CopyTask, ExitMode,
    LiveSubmitResult, PipelinePolicy, RiskContext,
};
use crate::trader::paper::TradingMode;

/// The paper side of the copy service: the per-task FIFO and the dispatch of paper
/// decisions to the shared paper executor.
pub(super) struct PaperRuntime {
    pub handle: PaperExecutionHandle,
    pub dispatch: PaperDispatch,
}

/// The dispatch of a copy task's paper decision: the shared trade dispatcher, run on its
/// own task so a stop of the FIFO never cancels a fill half booked.
pub(super) fn paper_dispatch() -> PaperDispatch {
    Arc::new(|decision, context| {
        Box::pin(crate::trader::manual::detached(async move {
            crate::trader::executors::execute_trade(&decision, context).await
        }))
    })
}

pub async fn run(shutdown: Arc<Notify>, database: CopyDatabase) {
    let (handle, stop_execution, execution) = super::paper_execution::install();
    let paper_runtime = PaperRuntime {
        handle,
        dispatch: paper_dispatch(),
    };
    let mut receiver = subscribe_activity();
    let mut watch_changes = crate::wallets::watch::subscribe_target_changes();
    if let Err(error) = reconcile_runtime_state(&database).await {
        logger::warning(
            LogTag::Trader,
            &format!("Copy reconciliation failed: {error}"),
        );
    }
    let mut reconciliation = tokio::time::interval(std::time::Duration::from_secs(60));
    reconciliation.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    reconciliation.reset();
    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = reconciliation.tick() => {
                if let Err(error) = reconcile_runtime_state(&database).await {
                    logger::warning(LogTag::Trader, &format!("Copy reconciliation failed: {error}"));
                }
            }
            changed = watch_changes.recv() => match changed {
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    if let Err(error) = super::guards::pause_detached_tasks(&database).await {
                        logger::warning(LogTag::Trader, &format!("Copy watch reconciliation failed: {error}"));
                    }
                }
                Err(RecvError::Closed) => break,
            },
            received = receiver.recv() => match received {
                Ok(activity) => {
                    if let Err(error) = process_activity(
                        &database,
                        Some(&paper_runtime),
                        &activity,
                    ).await {
                        logger::warning(LogTag::Trader, &format!("Copy activity failed: {error}"));
                    }
                }
                Err(RecvError::Lagged(count)) => logger::warning(
                    LogTag::Trader,
                    &format!("Copy consumer lagged by {count} wallet activities"),
                ),
                Err(RecvError::Closed) => break,
            }
        }
    }
    let _ = stop_execution.send(true);
    if let Err(error) = execution.await {
        logger::error(
            LogTag::Trader,
            &format!("Paper execution dispatcher failed: {error}"),
        );
    }
}

async fn reconcile_runtime_state(database: &CopyDatabase) -> crate::trader::Result<()> {
    super::guards::pause_after_store_restore(database).await?;
    let abandoned = database.reconcile_stale_claims(300).await?;
    if abandoned > 0 {
        logger::warning(
            LogTag::Trader,
            &format!("Reconciled {abandoned} stale copy claims as fail-closed abandoned"),
        );
    }

    let pending_entries = database.list_unconfirmed_live_entries().await?;
    let mut positions = crate::positions::get_open_positions().await;
    positions.extend(crate::positions::get_closed_positions().await);
    positions.extend(crate::positions::get_archived_positions().await);
    for mut decision in pending_entries {
        let Some(signature) = decision.transaction_signature.as_deref() else {
            continue;
        };
        let confirmed = positions.iter().any(|position| {
            position.transaction_entry_verified
                && position.entry_transaction_signature.as_deref() == Some(signature)
                && matches!(
                    position.origin,
                    crate::positions::PositionOrigin::Copy { task_id, .. }
                        if task_id == decision.task_id
                )
        });
        if confirmed {
            decision.telemetry.confirmed_at = Some(Utc::now());
            notify::record(database, CopyOutcome::LiveConfirmed(decision)).await?;
        }
    }
    super::guards::pause_detached_tasks(database).await?;
    super::guards::sync_paused_watches(database).await
}

async fn process_activity(
    database: &CopyDatabase,
    paper_runtime: Option<&PaperRuntime>,
    observed: &WalletActivity,
) -> crate::trader::Result<()> {
    let (copy_enabled, require_filter_pass, global_stable_quoted) = with_config(|config| {
        (
            config.copy_trading.enabled,
            config.copy_trading.require_filter_pass,
            config.copy_trading.copy_stable_quoted,
        )
    });
    if !observed
        .sources
        .iter()
        .any(|source| matches!(source, WatchSource::Copy { .. }))
    {
        return Ok(());
    }
    // A stable-quoted trade is copied in native terms at the current price, so
    // sizing, limits and the paper book all read one native-quoted swap.
    let converted;
    let activity = match &observed.kind {
        ActivityKind::Swap { .. } => observed,
        ActivityKind::StableSwap { mint, .. } => {
            let native_usd = crate::apis::native_price::get_native_price();
            let Some(kind) = observed.kind.stable_swap_in_native(native_usd) else {
                return record_uncopyable(
                    database,
                    observed,
                    Some(mint),
                    CopySkip::StableQuotePriceUnavailable,
                    copy_enabled,
                )
                .await;
            };
            converted = WalletActivity {
                kind,
                ..observed.clone()
            };
            &converted
        }
        ActivityKind::UnresolvedSwap { mint, reason, .. } => {
            return record_uncopyable(
                database,
                observed,
                mint.as_deref(),
                CopySkip::from(*reason),
                copy_enabled,
            )
            .await;
        }
        ActivityKind::Transfer { .. } | ActivityKind::Other => return Ok(()),
    };
    let stable_quoted = matches!(observed.kind, ActivityKind::StableSwap { .. });
    let ActivityKind::Swap { mint, side, .. } = &activity.kind else {
        return Ok(());
    };
    let watching = database.tasks_for_subject(&activity.subject).await?;
    let watching = matching_tasks(activity, &watching)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    if watching.is_empty() {
        return Ok(());
    }
    // Every watching task keeps the wallet's inventory, paused or not, so a later
    // proportional exit still sells the right fraction.
    let target_holdings_before = observe_target_inventory(database, activity, &watching).await?;
    // Pausing, one task or copy processing as a whole, stops new copies; the
    // wallet's sells still close what a mirror or hybrid task holds. A task on its
    // own exit rules ignores the wallet's sells, so they are no decision of it.
    let selling = matches!(side, SwapSide::Sell);
    if !selling && !copy_enabled {
        return Ok(());
    }
    let mut tasks = Vec::with_capacity(watching.len());
    for task in watching {
        let acts = if selling {
            task.exit_mode != ExitMode::BuyOnly
                && ((task.enabled && copy_enabled)
                    || super::control::holds_open_copies(&task, Some(mint)).await)
        } else {
            task.enabled
        };
        if acts {
            tasks.push(task);
        }
    }
    if tasks.is_empty() {
        return Ok(());
    }
    if super::guards::reject_stale_backfill(database, activity, &tasks, mint).await? {
        return Ok(());
    }
    super::guards::apply_latency_kill_switch(database, activity, &mut tasks).await?;
    // A held copy follows the wallet's exit whatever coin it is paid in; only a
    // new stable-quoted buy needs the task's consent.
    if stable_quoted && !selling {
        tasks = admit_stable_quoted(database, tasks, activity, mint, global_stable_quoted).await?;
    }
    if tasks.is_empty() {
        return Ok(());
    }
    if selling {
        return process_sell_activity(
            database,
            paper_runtime,
            activity,
            &tasks,
            mint,
            &target_holdings_before,
        )
        .await;
    }
    for task in tasks.iter().filter(|task| task.mode == TradingMode::Paper) {
        let job_database = database.clone();
        let job_activity = activity.clone();
        let job_mint = mint.clone();
        let task_id = task.id;
        enqueue_paper(
            database,
            paper_runtime,
            task_id,
            activity,
            mint,
            |dispatch| async move {
                execute_paper_entry(&job_database, &job_activity, task_id, &job_mint, dispatch)
                    .await
            },
        )
        .await?;
    }
    let tasks = tasks
        .into_iter()
        .filter(|task| task.mode == TradingMode::Live)
        .collect::<Vec<_>>();
    if tasks.is_empty() {
        return Ok(());
    }
    // The filter check is fetched once per chain of the tasks that require it;
    // a task that does not require it sees a pass.
    let any_requires_filter = tasks
        .iter()
        .any(|task| task.requires_filter_pass(require_filter_pass));
    let mut filter_passed_chains = HashSet::new();
    let mut filter_checked_chains = HashSet::new();
    for task in &tasks {
        if !task.requires_filter_pass(require_filter_pass)
            || !filter_checked_chains.insert(task.chain)
        {
            continue;
        }
        let passed = crate::filtering::get_filtered_token_mints(task.chain)
            .await
            .map_err(|e| crate::trader::Error::Dependency {
                dependency: "filtering",
                detail: e.to_string(),
            })?
            .iter()
            .any(|passed| passed == mint);
        if passed {
            filter_passed_chains.insert(task.chain);
        }
    }
    let mut spend = HashMap::new();
    let mut risk = HashMap::new();
    let own_addresses = crate::wallets::list_wallets(true)
        .await
        .map_err(|e| crate::trader::Error::Dependency {
            dependency: "wallets",
            detail: e.to_string(),
        })?
        .into_iter()
        .map(|wallet| wallet.address)
        .collect::<Vec<_>>();
    for task in &tasks {
        spend.insert(task.id, database.live_spend_state(task.id, mint).await?);
        risk.insert(
            task.id,
            RiskContext {
                is_self_wallet: own_addresses
                    .iter()
                    .any(|address| address == &task.target_address),
                mint_blacklisted: crate::trader::safety::is_blacklisted(mint).await,
                filter_passed: filter_passed_chains.contains(&task.chain)
                    || !task.requires_filter_pass(require_filter_pass),
                ..RiskContext::default()
            },
        );
    }
    let trade_size_native = with_config(|config| config.trader.trade_size_sol);
    let policy = PipelinePolicy {
        require_filter_pass: any_requires_filter,
        engine_trade_size_native: trade_size_native,
    };
    for task in &tasks {
        let decided_at = Utc::now();
        let plan = match prepare_entry(
            activity,
            task,
            spend.get(&task.id).copied().unwrap_or_default(),
            risk.get(&task.id).copied().unwrap_or_default(),
            policy,
            decided_at,
        ) {
            Ok(plan) => plan,
            Err(reason) => {
                notify::record(database, skipped(task.id, activity, mint, reason)).await?;
                continue;
            }
        };
        if !database
            .claim_live_activity(task.id, &activity.signature)
            .await?
        {
            continue;
        }
        let outcome = execute_live_with(
            plan,
            |entry_mint| async move {
                crate::trader::admission::check_entry_admission(&entry_mint, &["rpc"]).await?;
                crate::trader::admission::check_open_cooldown().await?;
                if !crate::trader::entry::try_reserve_entry(
                    crate::positions::Book::Live,
                    &entry_mint,
                )
                .await
                {
                    return Err(crate::trader::admission::EntryBlock::EntryReserved);
                }
                Ok(())
            },
            |decision, context| async move {
                LiveSubmitResult::from_trade_result(
                    crate::trader::entry::submit_entry_with_context(decision, context).await,
                )
            },
        )
        .await;
        notify::record(database, outcome).await?;
    }
    Ok(())
}

/// Keeps the tasks that copy a stable-quoted buy and records why the others did not.
async fn admit_stable_quoted(
    database: &CopyDatabase,
    tasks: Vec<super::CopyTask>,
    activity: &WalletActivity,
    mint: &str,
    global_stable_quoted: bool,
) -> crate::trader::Result<Vec<super::CopyTask>> {
    let mut consenting = Vec::with_capacity(tasks.len());
    for task in tasks {
        if task.copies_stable_quoted(global_stable_quoted) {
            consenting.push(task);
        } else {
            record_skip(
                database,
                &task,
                activity,
                Some(mint),
                CopySkip::StableQuoteDisabled,
            )
            .await?;
        }
    }
    Ok(consenting)
}

/// Records a skip for one task, bound to the paper book a paper task runs in.
async fn record_skip(
    database: &CopyDatabase,
    task: &super::CopyTask,
    activity: &WalletActivity,
    mint: Option<&str>,
    reason: CopySkip,
) -> crate::trader::Result<()> {
    let outcome = skipped_activity(task.id, activity, mint, reason);
    let outcome = match task.mode {
        TradingMode::Paper => as_paper(outcome),
        TradingMode::Live => outcome,
    };
    notify::record(database, outcome).await?;
    Ok(())
}

/// A wallet trade no task can copy is shown, with its reason, in every enabled
/// task that would otherwise have acted on it.
async fn record_uncopyable(
    database: &CopyDatabase,
    activity: &WalletActivity,
    mint: Option<&str>,
    reason: CopySkip,
    copy_enabled: bool,
) -> crate::trader::Result<()> {
    if !copy_enabled {
        return Ok(());
    }
    let watching = database.tasks_for_subject(&activity.subject).await?;
    for task in matching_tasks(activity, &watching) {
        if task.enabled {
            record_skip(database, task, activity, mint, reason.clone()).await?;
        }
    }
    Ok(())
}

async fn observe_target_inventory(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &[super::CopyTask],
) -> crate::trader::Result<HashMap<i64, f64>> {
    let ActivityKind::Swap {
        mint,
        side,
        token_amount,
        ..
    } = &activity.kind
    else {
        return Ok(HashMap::new());
    };
    let delta = match side {
        crate::wallets::watch::SwapSide::Buy => *token_amount,
        crate::wallets::watch::SwapSide::Sell => -*token_amount,
    };
    let mut before = HashMap::new();
    for task in tasks {
        before.insert(
            task.id,
            database
                .observe_target_inventory(task.id, &activity.signature, mint, delta)
                .await?,
        );
    }
    Ok(before)
}

/// Queues the paper work `job` of task `task_id` for the target transaction of
/// `activity` on the task's FIFO, recording why when it cannot be queued. A
/// transaction already queued for the task is not queued twice.
async fn enqueue_paper<J, JFut>(
    database: &CopyDatabase,
    paper_runtime: Option<&PaperRuntime>,
    task_id: i64,
    activity: &WalletActivity,
    mint: &str,
    job: J,
) -> crate::trader::Result<()>
where
    J: FnOnce(PaperDispatch) -> JFut,
    JFut: std::future::Future<Output = CopyOutcome> + Send + 'static,
{
    let reason = match paper_runtime {
        Some(runtime) => match runtime.handle.enqueue(
            task_id,
            activity_key(&activity.signature),
            database.clone(),
            Some(mint.to_owned()),
            job(runtime.dispatch.clone()),
        ) {
            Ok(()) | Err(EnqueueError::Duplicate) => return Ok(()),
            Err(EnqueueError::Full) => CopySkip::PaperQueueFull,
            Err(EnqueueError::Unavailable) => CopySkip::PaperExecutionUnavailable,
        },
        None => CopySkip::PaperExecutionUnavailable,
    };
    notify::record(database, skipped_paper(task_id, activity, mint, reason)).await?;
    Ok(())
}

async fn process_sell_activity(
    database: &CopyDatabase,
    paper_runtime: Option<&PaperRuntime>,
    activity: &WalletActivity,
    tasks: &[CopyTask],
    mint: &str,
    target_holdings_before: &HashMap<i64, f64>,
) -> crate::trader::Result<()> {
    for task in tasks.iter().filter(|task| task.mode == TradingMode::Paper) {
        let target_holding = target_holdings_before
            .get(&task.id)
            .copied()
            .unwrap_or_default();
        let job_database = database.clone();
        let job_activity = activity.clone();
        let job_mint = mint.to_owned();
        let task_id = task.id;
        enqueue_paper(
            database,
            paper_runtime,
            task_id,
            activity,
            mint,
            |_| async move {
                paper_copy_sell(
                    &job_database,
                    &job_activity,
                    task_id,
                    &job_mint,
                    target_holding,
                )
                .await
            },
        )
        .await?;
    }

    for task in tasks.iter().filter(|task| task.mode == TradingMode::Live) {
        let target_holding = target_holdings_before
            .get(&task.id)
            .copied()
            .unwrap_or_default();
        if !database
            .claim_live_activity(task.id, &activity.signature)
            .await?
        {
            continue;
        }
        let position = crate::positions::get_position_by_mint(mint).await;
        let plan = match prepare_copy_sell(
            activity,
            task,
            position.as_ref(),
            crate::global::is_force_stopped(),
            target_holding,
            Utc::now(),
        ) {
            Ok(plan) => plan,
            Err(reason) => {
                notify::record(database, skipped(task.id, activity, mint, reason)).await?;
                continue;
            }
        };
        let outcome = sell_through_exit_pipeline(plan, position.as_ref()).await;
        notify::record(database, outcome).await?;
    }
    Ok(())
}

/// Mirrors a target sell on the task's open paper round of `mint`: the sale the live
/// copy sell would make, booked by the shared paper executor. Runs on the task's FIFO.
async fn paper_copy_sell(
    database: &CopyDatabase,
    activity: &WalletActivity,
    task_id: i64,
    mint: &str,
    target_holding: f64,
) -> CopyOutcome {
    let task = match database.get_task(task_id).await {
        Ok(Some(task)) if task.mode == TradingMode::Paper => task,
        Ok(_) => return skipped_paper(task_id, activity, mint, CopySkip::TaskDisabled),
        Err(_) => {
            return skipped_paper(task_id, activity, mint, CopySkip::PaperExecutionUnavailable)
        }
    };
    let position = crate::positions::books::open_round(Book::Paper, mint).await;
    let plan = match prepare_copy_sell(
        activity,
        &task,
        position.as_ref(),
        crate::global::is_force_stopped(),
        target_holding,
        Utc::now(),
    ) {
        Ok(plan) => plan,
        Err(reason) => return skipped_paper(task_id, activity, mint, reason),
    };
    sell_through_exit_pipeline(plan, position.as_ref()).await
}

/// Runs a planned copy sell through the shared exit pipeline, on its own task so no
/// stop or dropped caller cancels the sale half booked.
async fn sell_through_exit_pipeline(
    plan: super::PreparedCopySell,
    position: Option<&crate::positions::Position>,
) -> CopyOutcome {
    let symbol = position.map(|position| position.symbol.clone());
    // A planned copy sell always sells this task's own copy position.
    let origin = position.map_or("copy", |position| position.origin.kind());
    execute_copy_sell_with(plan, |decision| async move {
        CopySellSubmitResult::from_trade_result(
            crate::trader::manual::detached(async move {
                crate::trader::executors::execute_tracked_exit(&decision, symbol.as_deref(), origin)
                    .await
            })
            .await,
        )
    })
    .await
}

#[cfg(test)]
mod tests;
