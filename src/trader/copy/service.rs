//! Runtime paper/live consumer of the shared wallet-observation broadcast.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Notify;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{subscribe_activity, ActivityKind, WalletActivity, WatchSource};

use super::{
    arrival_distance_ms, execute_copy_sell_with, execute_live_with, latency_should_pause,
    matching_tasks, paper_sell_outcome, prepare_copy_sell, prepare_live_entry, run_paper_pipeline,
    CopyDatabase, CopyMode, CopyOutcome, CopySellSubmitResult, CopySkip, CopyTelemetry,
    LiveSubmitResult, PaperCosts, PaperPosition, PipelinePolicy, RiskContext,
};

/// A task edited this recently may still be attaching its watch source, so the
/// reconciler never judges it detached.
const SOURCE_SETTLE_SECS: i64 = 120;

pub async fn run(shutdown: Arc<Notify>, database: CopyDatabase) {
    let mut receiver = subscribe_activity();
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
            _ = shutdown.notified() => return,
            _ = reconciliation.tick() => {
                if let Err(error) = reconcile_runtime_state(&database).await {
                    logger::warning(LogTag::Trader, &format!("Copy reconciliation failed: {error}"));
                }
            }
            _ = paper_exits.tick() => {
                if let Err(error) = super::paper_exits::sweep(&database, paper_costs()).await {
                    logger::warning(LogTag::Trader, &format!("Paper exit sweep failed: {error}"));
                }
            }
            received = receiver.recv() => match received {
                Ok(activity) => {
                    if let Err(error) = process_activity(&database, &activity).await {
                        logger::warning(LogTag::Trader, &format!("Copy activity failed: {error}"));
                    }
                }
                Err(RecvError::Lagged(count)) => logger::warning(
                    LogTag::Trader,
                    &format!("Copy consumer lagged by {count} wallet activities"),
                ),
                Err(RecvError::Closed) => return,
            }
        }
    }
}

async fn reconcile_runtime_state(database: &CopyDatabase) -> crate::trader::Result<()> {
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
            database
                .record_outcome(CopyOutcome::LiveConfirmed(decision))
                .await?;
        }
    }
    pause_detached_tasks(database).await
}

/// An enabled task whose target is no longer an enabled watch source (the watcher
/// disabled it as saturated, or it was removed) receives nothing, yet would still
/// read as running. Pause it so the task state tells the truth.
async fn pause_detached_tasks(database: &CopyDatabase) -> crate::trader::Result<()> {
    let settled_before = Utc::now() - chrono::Duration::seconds(SOURCE_SETTLE_SECS);
    for task in database.list_tasks().await? {
        if !task.enabled || task.updated_at > settled_before {
            continue;
        }
        match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
            Ok(true) => {}
            Ok(false) => {
                if database.pause_task(task.id).await? {
                    logger::warning(
                        LogTag::Trader,
                        &format!(
                            "Paused copy task {}: target {} is no longer watched",
                            task.id, task.target_address
                        ),
                    );
                }
            }
            // The watch service is not up yet; nothing can be judged this pass.
            Err(_) => return Ok(()),
        }
    }
    Ok(())
}

async fn process_activity(
    database: &CopyDatabase,
    activity: &WalletActivity,
) -> crate::trader::Result<()> {
    let (copy_enabled, require_filter_pass) = with_config(|config| {
        (
            config.copy_trading.enabled,
            config.copy_trading.require_filter_pass,
        )
    });
    if !copy_enabled {
        return Ok(());
    }
    if !activity
        .sources
        .iter()
        .any(|source| matches!(source, WatchSource::Copy { .. }))
    {
        return Ok(());
    }
    let ActivityKind::Swap {
        mint, price_sol, ..
    } = &activity.kind
    else {
        return Ok(());
    };
    let tasks = database
        .enabled_tasks_for_subject(&activity.subject)
        .await?;
    let mut tasks = matching_tasks(activity, &tasks)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    if tasks.is_empty() {
        return Ok(());
    }
    let target_holdings_before = observe_target_inventory(database, activity, &tasks).await?;
    if reject_stale_backfill(database, activity, &tasks, mint).await? {
        return Ok(());
    }
    apply_latency_kill_switch(database, activity, &mut tasks).await?;
    if tasks.is_empty() {
        return Ok(());
    }
    if matches!(
        activity.kind,
        ActivityKind::Swap {
            side: crate::wallets::watch::SwapSide::Sell,
            ..
        }
    ) {
        return process_sell_activity(database, activity, &tasks, mint, &target_holdings_before)
            .await;
    }
    let filter_passed = if require_filter_pass {
        crate::filtering::get_filtered_token_mints()
            .await
            .map_err(|e| crate::trader::Error::Dependency {
                dependency: "filtering",
                detail: e.to_string(),
            })?
            .iter()
            .any(|passed| passed == mint)
    } else {
        true
    };
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
                filter_passed,
                ..RiskContext::default()
            },
        );
    }
    let trade_size_sol = with_config(|config| config.trader.trade_size_sol);
    let policy = PipelinePolicy {
        require_filter_pass,
        engine_trade_size_sol: trade_size_sol,
    };
    let paper_tasks = tasks
        .iter()
        .filter(|task| task.mode == CopyMode::Paper)
        .cloned()
        .collect::<Vec<_>>();
    if !paper_tasks.is_empty() {
        let decision_price = decision_price(mint, *price_sol);
        let paper_outcomes = if let Err(block) =
            crate::trader::admission::check_entry_admission(mint, &["rpc"]).await
        {
            paper_tasks
                .iter()
                .map(|task| {
                    skipped(
                        task.id,
                        activity,
                        mint,
                        CopySkip::EntryBlocked {
                            block: block.clone(),
                        },
                    )
                })
                .collect()
        } else {
            run_paper_pipeline(
                activity,
                &paper_tasks,
                &spend,
                &risk,
                policy,
                decision_price,
                paper_costs(),
                Utc::now(),
            )
        };
        for outcome in paper_outcomes {
            database.record_outcome(outcome).await?;
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
                database
                    .record_outcome(skipped(task.id, activity, mint, reason))
                    .await?;
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
        database.record_outcome(outcome).await?;
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

async fn apply_latency_kill_switch(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &mut Vec<super::CopyTask>,
) -> crate::trader::Result<()> {
    let (enabled, window_size, threshold_ms) = with_config(|config| {
        (
            config.copy_trading.latency_kill_switch_enabled,
            config.copy_trading.latency_window_size,
            config.copy_trading.max_arrival_distance_ms,
        )
    });
    if !enabled {
        return Ok(());
    }
    // A replayed observation measures downtime, not pipeline latency.
    if activity.backfill {
        return Ok(());
    }
    let Some(current_distance) = arrival_distance_ms(&observation_telemetry(activity)) else {
        return Ok(());
    };
    let mut retained = Vec::with_capacity(tasks.len());
    for task in tasks.drain(..) {
        let rows = database.list_task_activity(task.id, window_size).await?;
        let mut samples = rows
            .iter()
            .filter_map(|row| row.outcome.telemetry())
            .filter(|telemetry| !telemetry.backfill)
            .filter_map(arrival_distance_ms)
            .collect::<Vec<_>>();
        samples.reverse();
        samples.push(current_distance);
        if latency_should_pause(&samples, window_size, threshold_ms) {
            let window = &samples[samples.len() - window_size..];
            let average_ms = (window.iter().map(|value| u128::from(*value)).sum::<u128>()
                / window_size as u128) as u64;
            if database.pause_task(task.id).await? {
                crate::wallets::watch::remove_copy_source(task.id, &task.target_address)
                    .await
                    .map_err(|e| crate::trader::Error::Dependency {
                        dependency: "wallets",
                        detail: e.to_string(),
                    })?;
            }
            let mint = match &activity.kind {
                ActivityKind::Swap { mint, .. } => mint.clone(),
                _ => String::new(),
            };
            database
                .record_outcome(skipped(
                    task.id,
                    activity,
                    &mint,
                    CopySkip::LatencyKillSwitch {
                        average_ms,
                        threshold_ms,
                    },
                ))
                .await?;
            logger::warning(
                LogTag::Trader,
                &format!(
                    "Paused copy task {}: trailing arrival delay {}ms exceeds {}ms",
                    task.id, average_ms, threshold_ms
                ),
            );
        } else {
            retained.push(task);
        }
    }
    *tasks = retained;
    Ok(())
}

async fn process_sell_activity(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &[super::CopyTask],
    mint: &str,
    target_holdings_before: &HashMap<i64, f64>,
) -> crate::trader::Result<()> {
    let force_stopped = crate::global::is_force_stopped();
    let target_price = match &activity.kind {
        ActivityKind::Swap { price_sol, .. } => *price_sol,
        _ => None,
    };
    let market_price = decision_price(mint, target_price);
    let costs = paper_costs();
    for task in tasks.iter().filter(|task| task.mode == CopyMode::Paper) {
        let target_holding = target_holdings_before
            .get(&task.id)
            .copied()
            .unwrap_or_default();
        let paper_held = database
            .paper_position(task.id, mint)
            .await?
            .filter(PaperPosition::is_open)
            .map_or(0.0, |position| position.token_amount);
        let outcome = match paper_sell_outcome(
            activity,
            task,
            force_stopped,
            target_holding,
            paper_held,
            market_price,
            costs,
            Utc::now(),
        ) {
            Ok(outcome) => outcome,
            Err(reason) => skipped(task.id, activity, mint, reason),
        };
        database.record_outcome(outcome).await?;
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
                database
                    .record_outcome(skipped(task.id, activity, mint, reason))
                    .await?;
                continue;
            }
        };
        let outcome = execute_copy_sell_with(plan, |decision| async move {
            CopySellSubmitResult::from_trade_result(
                crate::trader::executors::execute_trade(&decision).await,
            )
        })
        .await;
        database.record_outcome(outcome).await?;
    }
    Ok(())
}

/// A gap-fill replays trades made while the bot was down or disconnected. Their
/// inventory effect is kept (recorded before this runs), but copying one that
/// arrived later than the arrival limit would trade on a price that no longer
/// exists, so each task records an explicit skip instead.
async fn reject_stale_backfill(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &[super::CopyTask],
    mint: &str,
) -> crate::trader::Result<bool> {
    if !activity.backfill {
        return Ok(false);
    }
    let threshold_ms = with_config(|config| config.copy_trading.max_arrival_distance_ms);
    let Some(arrival_ms) = arrival_distance_ms(&observation_telemetry(activity)) else {
        return Ok(false);
    };
    if arrival_ms <= threshold_ms {
        return Ok(false);
    }
    for task in tasks {
        database
            .record_outcome(skipped(
                task.id,
                activity,
                mint,
                CopySkip::StaleObservation {
                    arrival_ms,
                    threshold_ms,
                },
            ))
            .await?;
    }
    Ok(true)
}

/// Pool price first (the trading price system); the target's own swap price when
/// the pool service does not track the token. NaN means no price at all, which
/// the paper simulators refuse as `InvalidPrice`.
fn decision_price(mint: &str, target_price_sol: Option<f64>) -> f64 {
    crate::pools::get_pool_price(mint)
        .map(|price| price.price_sol)
        .or(target_price_sol)
        .unwrap_or(f64::NAN)
}

fn paper_costs() -> PaperCosts {
    let priority_lamports = with_config(|config| config.swaps.jupiter.default_priority_fee);
    PaperCosts {
        network_fee_sol: 0.000005,
        priority_fee_sol: crate::chains::adapter().raw_to_native(priority_lamports),
    }
}

fn observation_telemetry(activity: &WalletActivity) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: activity.block_time,
        detected_at: activity.detected_at,
        decoded_at: activity.decoded_at,
        decided_at: Utc::now(),
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: match &activity.kind {
            ActivityKind::Swap { price_sol, .. } => *price_sol,
            _ => None,
        },
        fill_price_sol: None,
        backfill: activity.backfill,
    }
}

fn skipped(task_id: i64, activity: &WalletActivity, mint: &str, reason: CopySkip) -> CopyOutcome {
    let telemetry = observation_telemetry(activity);
    CopyOutcome::Skipped {
        task_id,
        signature: activity.signature.clone(),
        mint: Some(mint.to_owned()),
        reason,
        decided_at: telemetry.decided_at,
        telemetry: Some(telemetry),
    }
}
