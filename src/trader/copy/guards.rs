//! The guards that stand a copy task down or refuse an observation: a target
//! that is no longer watched, a pipeline that fell behind, a replay too old to
//! copy. A pause stores its reason on the task and is announced.

use chrono::Utc;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{ActivityKind, WalletActivity};

use super::service::{observation_telemetry, skipped};
use super::{
    arrival_distance_ms, control, latency_should_pause, notify, CopyDatabase, CopyPauseReason,
    CopySkip, CopyTask,
};

/// A task edited this recently may still be attaching its watch source, so the
/// reconciler never judges it detached.
const SOURCE_SETTLE_SECS: i64 = 120;

/// An enabled task whose target is no longer an enabled watch source (the watcher
/// disabled it as saturated, or it was removed) receives nothing, yet would still
/// read as running. Pause it so the task state tells the truth.
pub(super) async fn pause_detached_tasks(database: &CopyDatabase) -> crate::trader::Result<()> {
    let settled_before = Utc::now() - chrono::Duration::seconds(SOURCE_SETTLE_SECS);
    for task in database.list_tasks().await? {
        if !task.enabled {
            continue;
        }
        let budget_reason =
            match crate::wallets::watch::copy_source_disable_reason(task.id, &task.target_address)
                .await
            {
                Ok(Some(crate::wallets::watch::WatchDisableReason::SignatureBudget {
                    page_budget,
                    signatures_checked,
                })) => Some(CopyPauseReason::WatchBudgetExceeded {
                    page_budget,
                    signatures_checked,
                }),
                Ok(_) => None,
                Err(_) => return Ok(()),
            };
        if task.updated_at > settled_before && budget_reason.is_none() {
            continue;
        }
        match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
            Ok(true) => {}
            Ok(false) => {
                let reason = budget_reason.unwrap_or(CopyPauseReason::WatchDetached);
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
                Some(CopyPauseReason::WatchDetached | CopyPauseReason::WatchBudgetExceeded { .. })
            )
        {
            continue;
        }
        let attached =
            match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
                Ok(attached) => attached,
                // The watch service is not up yet; nothing can be judged this pass.
                Err(_) => return Ok(()),
            };
        if attached != control::wants_watch(database, &task).await? {
            control::sync_task_watch(database, &task).await?;
        }
    }
    Ok(())
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
            notify::record(
                database,
                skipped(
                    task.id,
                    activity,
                    &mint,
                    CopySkip::LatencyKillSwitch {
                        average_ms,
                        threshold_ms,
                    },
                ),
            )
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
        notify::record(
            database,
            skipped(
                task.id,
                activity,
                mint,
                CopySkip::StaleObservation {
                    arrival_ms,
                    threshold_ms,
                },
            ),
        )
        .await?;
    }
    Ok(true)
}
