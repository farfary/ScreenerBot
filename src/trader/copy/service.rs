// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Runtime paper/live consumer of the shared wallet-observation broadcast.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Notify;

use super::paper_execution::{
    execute_paper_entry, in_paper_epoch, skipped, skipped_activity, skipped_paper,
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{
    subscribe_activity, ActivityKind, SwapSide, WalletActivity, WatchSource,
};

use super::{
    execute_copy_sell_with, execute_live_with, matching_tasks, notify, paper_sell_outcome,
    plan_paper_entries, prepare_copy_sell, prepare_live_entry, CopyDatabase, CopyMode, CopyOutcome,
    CopySellSubmitResult, CopySkip, ExitMode, LiveSubmitResult, PaperPosition, PipelinePolicy,
    RiskContext,
};

pub async fn run(shutdown: Arc<Notify>, database: CopyDatabase) {
    let (paper_runtime, stop_execution, execution) = super::paper_execution::install();
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
    let mut paper_exits = tokio::time::interval(std::time::Duration::from_secs(
        crate::trader::POSITION_MONITOR_INTERVAL_SECS,
    ));
    paper_exits.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
            _ = paper_exits.tick() => {
                if let Err(error) = super::paper_exits::sweep(&database).await {
                    logger::warning(LogTag::Trader, &format!("Paper exit sweep failed: {error}"));
                }
            }
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
    paper_runtime: Option<&super::paper_execution::PaperExecutionHandle>,
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
                    || super::control::holds_open_copies(database, &task, Some(mint)).await?)
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
        return process_sell_activity(database, activity, &tasks, mint, &target_holdings_before)
            .await;
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
        spend.insert(
            task.id,
            database.spend_state(task.id, task.mode, mint).await?,
        );
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
    let paper_tasks = tasks
        .iter()
        .filter(|task| task.mode == CopyMode::Paper)
        .cloned()
        .collect::<Vec<_>>();
    if !paper_tasks.is_empty() {
        let chain = paper_tasks[0].chain;
        debug_assert!(
            paper_tasks.iter().all(|task| task.chain == chain),
            "tasks of one wallet activity share a chain"
        );
        let paper_outcomes = if let Err(block) =
            crate::trader::admission::check_entry_admission(mint, &["rpc"]).await
        {
            paper_tasks
                .iter()
                .map(|task| {
                    Err((
                        task.clone(),
                        CopySkip::EntryBlocked {
                            block: block.clone(),
                        },
                    ))
                })
                .collect()
        } else {
            plan_paper_entries(activity, &paper_tasks, &spend, &risk, policy)
        };
        for planned in paper_outcomes {
            match planned {
                Ok(plan) => {
                    let epoch = database.paper_epoch(plan.task.id).await?;
                    let queued_at = Utc::now();
                    let task_id = plan.task.id;
                    let activity_copy = activity.clone();
                    let Some(runtime) = paper_runtime else {
                        notify::record(
                            database,
                            skipped_paper(
                                task_id,
                                activity,
                                mint,
                                CopySkip::PaperExecutionUnavailable,
                                epoch,
                            ),
                        )
                        .await?;
                        continue;
                    };
                    let db = database.clone();
                    let job_db = db.clone();
                    let fallback_activity = activity.clone();
                    if let Err(error) = runtime.enqueue(
                        task_id,
                        format!("activity:{}:{}", activity_copy.signature, epoch),
                        db,
                        Some(mint.to_owned()),
                        async move {
                            execute_paper_entry(&job_db, &activity_copy, plan, epoch, queued_at)
                                .await
                        },
                    ) {
                        if error == super::paper_execution::EnqueueError::Duplicate {
                            continue;
                        }
                        let reason = match error {
                            super::paper_execution::EnqueueError::Full => CopySkip::PaperQueueFull,
                            super::paper_execution::EnqueueError::Duplicate => unreachable!(),
                            super::paper_execution::EnqueueError::Unavailable => {
                                CopySkip::PaperExecutionUnavailable
                            }
                        };
                        notify::record(
                            database,
                            skipped_paper(task_id, &fallback_activity, mint, reason, epoch),
                        )
                        .await?;
                    }
                }
                Err((task, reason)) => {
                    let epoch = database.paper_epoch(task.id).await?;
                    notify::record(
                        database,
                        skipped_paper(task.id, activity, mint, reason, epoch),
                    )
                    .await?;
                }
            }
        }
    }

    for task in tasks.iter().filter(|task| task.mode == CopyMode::Live) {
        let decided_at = Utc::now();
        let plan = match prepare_live_entry(
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
                if !crate::trader::entry::try_reserve_entry(&entry_mint).await {
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
        CopyMode::Paper => in_paper_epoch(outcome, database.paper_epoch(task.id).await?),
        CopyMode::Live => outcome,
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

async fn process_sell_activity(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &[super::CopyTask],
    mint: &str,
    target_holdings_before: &HashMap<i64, f64>,
) -> crate::trader::Result<()> {
    for task in tasks.iter().filter(|task| task.mode == CopyMode::Paper) {
        let target_holding = target_holdings_before
            .get(&task.id)
            .copied()
            .unwrap_or_default();
        let epoch = database.paper_epoch(task.id).await?;
        let activity_copy = activity.clone();
        let fallback_activity = activity.clone();
        let database = database.clone();
        let runtime = super::paper_execution::active();
        let queued_at = Utc::now();
        let Some(runtime) = runtime else {
            notify::record(
                &database,
                skipped_paper(
                    task.id,
                    &fallback_activity,
                    mint,
                    CopySkip::PaperExecutionUnavailable,
                    epoch,
                ),
            )
            .await?;
            continue;
        };
        let task_id = task.id;
        let fallback_db = database.clone();
        let job_db = database.clone();
        let fallback_mint = mint.to_owned();
        let target_mint = mint.to_owned();
        if let Err(error) = runtime.enqueue(task_id, format!("activity:{}:{}", activity_copy.signature, epoch), database.clone(), Some(target_mint.clone()), async move {
            let outcome = async {
                if crate::global::is_force_stopped() {
                    return Err(CopySkip::ForceStopped);
                }
                let current_task = job_db.get_task(task_id).await
                    .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                    .filter(|current| current.mode == CopyMode::Paper && current.exit_mode != ExitMode::BuyOnly)
                    .ok_or(CopySkip::TaskDisabled)?;
                let current_position = job_db.paper_position(task_id, &target_mint).await
                    .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                    .filter(PaperPosition::is_open)
                    .ok_or(CopySkip::CopyPositionNotFound)?;
                if !matches!(job_db.paper_epoch(task_id).await, Ok(current) if current == epoch) {
                    return Err(CopySkip::PaperExecutionUnavailable);
                }
                let sell_outcome = paper_sell_outcome(
                    &activity_copy,
                    &current_task,
                    false,
                    target_holding,
                    &current_position,
                    &job_db,
                    epoch,
                    queued_at,
                    crate::swaps::try_get_best_quote,
                    |delay| async move {
                        tokio::time::sleep(delay).await;
                        if crate::global::is_force_stopped() { return Err(CopySkip::ForceStopped); }
                        Ok(())
                    },
                ).await?;
                if crate::global::is_force_stopped()
                    || !matches!(job_db.paper_epoch(task_id).await, Ok(current) if current == epoch)
                {
                    return Err(CopySkip::PaperExecutionUnavailable);
                }
                let latest_task = job_db.get_task(task_id).await
                    .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                    .filter(|current| current.mode == CopyMode::Paper && current.exit_mode != ExitMode::BuyOnly
                        && current.slippage_pct == current_task.slippage_pct)
                    .ok_or(CopySkip::TaskDisabled)?;
                let latest_position = job_db.paper_position(task_id, &target_mint).await
                    .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                    .filter(PaperPosition::is_open)
                    .filter(|current| current.opened_at == current_position.opened_at && current.sells == current_position.sells)
                    .ok_or(CopySkip::PaperExecutionUnavailable)?;
                if latest_position.raw_token_amount.is_none_or(|latest| {
                    current_position.raw_token_amount.is_none_or(|current| latest < current)
                })
                    || latest_task.mode != CopyMode::Paper
                {
                    return Err(CopySkip::PaperExecutionUnavailable);
                }
                Ok(sell_outcome)
            }.await;
            outcome.unwrap_or_else(|reason| {
                skipped_paper(task_id, &activity_copy, &fallback_mint, reason, epoch)
            })
        }) {
            if error == super::paper_execution::EnqueueError::Duplicate { continue; }
            let reason = match error {
                super::paper_execution::EnqueueError::Full => CopySkip::PaperQueueFull,
                super::paper_execution::EnqueueError::Duplicate => unreachable!(),
                super::paper_execution::EnqueueError::Unavailable => CopySkip::PaperExecutionUnavailable,
            };
            notify::record(&fallback_db, skipped_paper(task_id, &fallback_activity, mint, reason, epoch)).await?;
        }
    }

    for task in tasks.iter().filter(|task| task.mode == CopyMode::Live) {
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
        let symbol = position.as_ref().map(|position| position.symbol.clone());
        // A planned copy sell always sells this task's own copy position.
        let origin = position
            .as_ref()
            .map_or("copy", |position| position.origin.kind());
        let outcome = execute_copy_sell_with(plan, |decision| async move {
            CopySellSubmitResult::from_trade_result(
                crate::trader::executors::execute_tracked_exit(
                    &decision,
                    symbol.as_deref(),
                    origin,
                )
                .await,
            )
        })
        .await;
        notify::record(database, outcome).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
