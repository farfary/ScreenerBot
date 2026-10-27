// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The guards that stand a copy task down or refuse an observation: a target
//! that is no longer watched, a pipeline that fell behind, a replay too old to
//! copy. A pause stores its reason on the task and is announced.

use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;

use crate::config::with_config;
use crate::database::catalog::COPY_TRADING;
use crate::database::health::{self, StoreHealth};
use crate::logger::{self, LogTag};
use crate::wallets::watch::{ActivityKind, WalletActivity};

use super::paper_execution::{observation_telemetry, skipped, skipped_paper};
use super::{
    arrival_distance_ms, control, latency_should_pause, notify, CopyDatabase, CopyPauseReason,
    CopySkip, CopyTask,
};
use crate::trader::paper::TradingMode;

/// A task edited this recently may still be attaching its watch source, so the
/// reconciler never judges it detached.
const SOURCE_SETTLE_SECS: i64 = 120;

/// Set once this process has stood its tasks down for a restored copy store, so a
/// restarted copy service never pauses again a task resumed since.
static STORE_RESTORE_HANDLED: AtomicBool = AtomicBool::new(false);

/// A copy store that opened restored from a snapshot or recreated empty may hold
/// budgets, claims and task settings older than the failure. Every enabled task is
/// paused with [`CopyPauseReason::StorageRestored`] and is resumed only explicitly.
/// Runs until it has succeeded once in this process.
pub(super) async fn pause_after_store_restore(
    database: &CopyDatabase,
) -> crate::trader::Result<()> {
    if STORE_RESTORE_HANDLED.load(Ordering::SeqCst) {
        return Ok(());
    }
    let health = COPY_TRADING
        .paths()
        .iter()
        .find_map(|path| health::status(path))
        .map(|status| status.health);
    if let Some(health) = health {
        for task in stand_down_for_store_restore(database, &health).await? {
            notify::announce_pause(&task, &CopyPauseReason::StorageRestored).await;
            logger::warning(
                LogTag::Trader,
                &format!(
                    "Paused copy task {}: the copy store opened {}",
                    task.id,
                    health.as_str()
                ),
            );
        }
    }
    STORE_RESTORE_HANDLED.store(true, Ordering::SeqCst);
    Ok(())
}

/// Pause every enabled task when `health` is replacement contents; the tasks it
/// paused.
async fn stand_down_for_store_restore(
    database: &CopyDatabase,
    health: &StoreHealth,
) -> crate::trader::Result<Vec<CopyTask>> {
    let mut paused = Vec::new();
    if !health.is_replacement() {
        return Ok(paused);
    }
    for task in database.list_tasks().await? {
        if task.enabled
            && database
                .pause_task(task.id, CopyPauseReason::StorageRestored)
                .await?
        {
            paused.push(task);
        }
    }
    Ok(paused)
}

/// An enabled task whose target is no longer an enabled watch source (the watcher
/// disabled it as saturated, or it was removed) receives nothing, yet would still
/// read as running. Pause it so the task state tells the truth.
pub(super) async fn pause_detached_tasks(database: &CopyDatabase) -> crate::trader::Result<()> {
    let settled_before = Utc::now() - chrono::Duration::seconds(SOURCE_SETTLE_SECS);
    for task in database.list_tasks().await? {
        if !task.enabled {
            continue;
        }
        let pause_reason =
            match crate::wallets::watch::copy_source_disable_reason(task.id, &task.target_address)
                .await
            {
                Ok(reason) => watch_pause_reason(reason.as_ref()),
                Err(_) => return Ok(()),
            };
        if task.updated_at > settled_before && pause_reason.is_none() {
            continue;
        }
        match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
            Ok(true) => {}
            Ok(false) => {
                let reason = pause_reason.unwrap_or(CopyPauseReason::WatchDetached);
                if database.pause_task(task.id, reason.clone()).await? {
                    notify::announce_pause(&task, &reason).await;
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

/// A paused task watches its wallet only while the wallet's sells still close
/// something it holds (`control::wants_watch`); this attaches or releases the
/// watch once that changes. A task the watcher detached stays detached, since
/// attaching it again would fight the watcher's own saturation guard.
pub(super) async fn sync_paused_watches(database: &CopyDatabase) -> crate::trader::Result<()> {
    for task in database.list_tasks().await? {
        if task.enabled
            || matches!(
                task.pause_reason,
                Some(
                    CopyPauseReason::WatchDetached
                        | CopyPauseReason::WatchBudgetExceeded { .. }
                        | CopyPauseReason::HeliusUnavailable
                        | CopyPauseReason::WatchProcessingFailed
                )
            )
        {
            continue;
        }
        match crate::wallets::watch::copy_source_disable_reason(task.id, &task.target_address).await
        {
            Ok(Some(reason)) if watch_waits_for_user(&reason) => continue,
            Ok(_) => {}
            Err(error) => {
                if watch_store_unavailable(&error) {
                    return Ok(());
                }
                log_watch_state_error(task.id, &error);
                continue;
            }
        }
        let attached =
            match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
                Ok(attached) => attached,
                Err(error) => {
                    if watch_store_unavailable(&error) {
                        return Ok(());
                    }
                    log_watch_state_error(task.id, &error);
                    continue;
                }
            };
        let wants_watch = match control::wants_watch(database, &task).await {
            Ok(wants_watch) => wants_watch,
            Err(error) => {
                logger::warning(
                    LogTag::Trader,
                    &format!(
                        "Copy task {} watch requirement could not be read: {error}",
                        task.id
                    ),
                );
                continue;
            }
        };
        if attached != wants_watch {
            if let Err(error) = control::sync_task_watch(database, &task).await {
                logger::warning(
                    LogTag::Trader,
                    &format!(
                        "Copy task {} watch could not be reconciled: {error}",
                        task.id
                    ),
                );
            }
        }
    }
    Ok(())
}

fn watch_store_unavailable(error: &crate::wallets::Error) -> bool {
    matches!(
        error,
        crate::wallets::Error::Internal(
            crate::errors::InternalError::InvariantViolation { message }
        ) if message == "watch database not initialized"
    )
}

fn log_watch_state_error(task_id: i64, error: &crate::wallets::Error) {
    logger::warning(
        LogTag::Trader,
        &format!("Copy task {task_id} watch state could not be read: {error}"),
    );
}

fn watch_waits_for_user(reason: &crate::wallets::watch::WatchDisableReason) -> bool {
    matches!(
        reason,
        crate::wallets::watch::WatchDisableReason::SignatureBudget { .. }
            | crate::wallets::watch::WatchDisableReason::Unknown
            | crate::wallets::watch::WatchDisableReason::HeliusUnavailable
            | crate::wallets::watch::WatchDisableReason::ProcessingFailed
    )
}

fn watch_pause_reason(
    reason: Option<&crate::wallets::watch::WatchDisableReason>,
) -> Option<CopyPauseReason> {
    match reason {
        Some(crate::wallets::watch::WatchDisableReason::SignatureBudget {
            page_budget,
            signatures_checked,
        }) => Some(CopyPauseReason::WatchBudgetExceeded {
            page_budget: *page_budget,
            signatures_checked: *signatures_checked,
        }),
        Some(crate::wallets::watch::WatchDisableReason::HeliusUnavailable) => {
            Some(CopyPauseReason::HeliusUnavailable)
        }
        Some(crate::wallets::watch::WatchDisableReason::ProcessingFailed) => {
            Some(CopyPauseReason::WatchProcessingFailed)
        }
        _ => None,
    }
}

pub(super) async fn apply_latency_kill_switch(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &mut Vec<CopyTask>,
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
        // A paused task is here only to close what it holds; the switch stands
        // down tasks that still copy.
        if !task.enabled {
            retained.push(task);
            continue;
        }
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
            let reason = CopyPauseReason::LatencyKillSwitch {
                average_ms,
                threshold_ms,
            };
            if database.pause_task(task.id, reason.clone()).await? {
                notify::announce_pause(&task, &reason).await;
                let paused = CopyTask {
                    enabled: false,
                    ..task.clone()
                };
                control::sync_task_watch(database, &paused).await?;
            }
            let mint = match &activity.kind {
                ActivityKind::Swap { mint, .. } => mint.clone(),
                _ => String::new(),
            };
            let reason = CopySkip::LatencyKillSwitch {
                average_ms,
                threshold_ms,
            };
            let outcome = if task.mode == TradingMode::Paper {
                skipped_paper(
                    task.id,
                    activity,
                    &mint,
                    reason,
                    database.paper_epoch(task.id).await?,
                )
            } else {
                skipped(task.id, activity, &mint, reason)
            };
            notify::record(database, outcome).await?;
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

/// A gap-fill replays trades made while the bot was down or disconnected. Their
/// inventory effect is kept (recorded before this runs), but copying one that
/// arrived later than the arrival limit would trade on a price that no longer
/// exists, so each task records an explicit skip instead.
pub(super) async fn reject_stale_backfill(
    database: &CopyDatabase,
    activity: &WalletActivity,
    tasks: &[CopyTask],
    mint: &str,
) -> crate::trader::Result<bool> {
    let threshold_ms = with_config(|config| config.copy_trading.max_arrival_distance_ms);
    let Some(reason) = backfill_skip_reason(
        activity.backfill,
        arrival_distance_ms(&observation_telemetry(activity)),
        threshold_ms,
    ) else {
        return Ok(false);
    };
    for task in tasks {
        let outcome = if task.mode == TradingMode::Paper {
            skipped_paper(
                task.id,
                activity,
                mint,
                reason.clone(),
                database.paper_epoch(task.id).await?,
            )
        } else {
            skipped(task.id, activity, mint, reason.clone())
        };
        notify::record(database, outcome).await?;
    }
    Ok(true)
}

fn backfill_skip_reason(
    backfill: bool,
    arrival_ms: Option<u64>,
    threshold_ms: u64,
) -> Option<CopySkip> {
    if !backfill {
        return None;
    }
    match arrival_ms {
        Some(arrival_ms) if arrival_ms > threshold_ms => Some(CopySkip::StaleObservation {
            arrival_ms,
            threshold_ms,
        }),
        Some(_) => None,
        None => Some(CopySkip::UnknownObservationTime),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::{
        backfill_skip_reason, stand_down_for_store_restore, watch_pause_reason,
        watch_store_unavailable,
    };
    use crate::database::health::StoreHealth;
    use crate::trader::copy::{
        CopyDatabase, CopyPauseReason, CopySkip, CopyTask, ExitMode, SizingMode,
    };
    use crate::trader::paper::TradingMode;
    use crate::wallets::watch::WatchDisableReason;

    const THRESHOLD_MS: u64 = 5_000;

    #[test]
    fn only_an_uninitialized_watch_database_ends_the_reconciliation_pass() {
        let unavailable =
            crate::wallets::Error::Internal(crate::errors::InternalError::InvariantViolation {
                message: "watch database not initialized".to_owned(),
            });
        let other_invariant =
            crate::wallets::Error::Internal(crate::errors::InternalError::InvariantViolation {
                message: "another invariant failed".to_owned(),
            });
        let other_error = crate::wallets::Error::NotInitialized { database: "wallet" };

        assert!(watch_store_unavailable(&unavailable));
        assert!(!watch_store_unavailable(&other_invariant));
        assert!(!watch_store_unavailable(&other_error));
    }

    fn task(target: &str, enabled: bool) -> CopyTask {
        CopyTask {
            id: 0,
            chain: crate::chains::ChainId::Solana,
            target_address: target.to_owned(),
            label: None,
            enabled,
            mode: TradingMode::Paper,
            sizing: SizingMode::Fixed { sol: 0.1 },
            exit_mode: ExitMode::BuyOnly,
            exit_policy_overrides: Default::default(),
            max_native_per_trade: 0.2,
            max_native_per_token: 1.0,
            total_budget_native: 5.0,
            min_target_trade_native: None,
            max_target_trade_native: None,
            buy_once_per_token: false,
            slippage_pct: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            require_filter_pass: None,
            copy_stable_quoted: None,
            pause_reason: (!enabled).then_some(CopyPauseReason::User),
            paused_at: (!enabled).then(Utc::now),
        }
    }

    #[tokio::test]
    async fn a_restored_copy_store_pauses_every_enabled_task_until_resumed() {
        let dir = tempfile::tempdir().unwrap();
        let database = CopyDatabase::open(
            dir.path().join("copy_trading.db"),
            crate::chains::ChainId::Solana,
        )
        .unwrap();
        let first = database.insert_task(task("first", true)).await.unwrap();
        let second = database.insert_task(task("second", true)).await.unwrap();
        let user_paused = database.insert_task(task("third", false)).await.unwrap();

        let healthy = stand_down_for_store_restore(&database, &StoreHealth::Healthy)
            .await
            .unwrap();
        assert!(healthy.is_empty(), "a healthy store pauses nothing");
        assert!(database.get_task(first.id).await.unwrap().unwrap().enabled);

        let quarantine = PathBuf::from("/copy-guard-test/quarantine");
        let restored = StoreHealth::Restored {
            snapshot_at: None,
            quarantine: quarantine.clone(),
        };
        let mut paused: Vec<i64> = stand_down_for_store_restore(&database, &restored)
            .await
            .unwrap()
            .iter()
            .map(|task| task.id)
            .collect();
        paused.sort_unstable();
        assert_eq!(paused, [first.id, second.id]);

        // A later pass, a recreated store included, finds nothing enabled: nothing is
        // resumed on its own.
        let recovered = StoreHealth::Recovered { quarantine };
        for health in [&restored, &recovered] {
            assert!(stand_down_for_store_restore(&database, health)
                .await
                .unwrap()
                .is_empty());
        }
        for id in [first.id, second.id] {
            let stored = database.get_task(id).await.unwrap().unwrap();
            assert!(!stored.enabled, "task {id} stays paused");
            assert_eq!(stored.pause_reason, Some(CopyPauseReason::StorageRestored));
        }
        let kept = database.get_task(user_paused.id).await.unwrap().unwrap();
        assert_eq!(kept.pause_reason, Some(CopyPauseReason::User));
    }

    #[test]
    fn backfilled_observation_without_a_block_time_is_refused() {
        assert_eq!(
            backfill_skip_reason(true, None, THRESHOLD_MS),
            Some(CopySkip::UnknownObservationTime)
        );
    }

    #[test]
    fn stale_backfilled_observation_is_refused() {
        assert_eq!(
            backfill_skip_reason(true, Some(THRESHOLD_MS + 1), THRESHOLD_MS),
            Some(CopySkip::StaleObservation {
                arrival_ms: THRESHOLD_MS + 1,
                threshold_ms: THRESHOLD_MS,
            })
        );
    }

    #[test]
    fn fresh_backfilled_observation_is_allowed() {
        assert_eq!(
            backfill_skip_reason(true, Some(THRESHOLD_MS), THRESHOLD_MS),
            None
        );
    }

    #[test]
    fn non_backfill_without_a_block_time_is_unchanged() {
        assert_eq!(backfill_skip_reason(false, None, THRESHOLD_MS), None);
    }

    #[test]
    fn helius_capability_pause_keeps_its_reason() {
        assert_eq!(
            watch_pause_reason(Some(&WatchDisableReason::HeliusUnavailable)),
            Some(CopyPauseReason::HeliusUnavailable)
        );
    }

    #[test]
    fn watch_processing_pause_keeps_its_reason() {
        assert_eq!(
            watch_pause_reason(Some(&WatchDisableReason::ProcessingFailed)),
            Some(CopyPauseReason::WatchProcessingFailed)
        );
    }

    #[test]
    fn paused_watch_user_action_state_is_skipped_without_stopping_later_tasks() {
        let tasks = [
            (
                1,
                WatchDisableReason::SignatureBudget {
                    page_budget: 5,
                    signatures_checked: 500,
                },
            ),
            (2, WatchDisableReason::User),
        ];
        let reconciled: Vec<_> = tasks
            .iter()
            .filter(|(_, reason)| !super::watch_waits_for_user(reason))
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(reconciled, [2]);
    }
}
