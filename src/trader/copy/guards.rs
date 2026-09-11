//! The guards that stand a copy task down or refuse an observation: a target
//! that is no longer watched, a pipeline that fell behind, a replay too old to
//! copy. A pause stores its reason on the task and is announced.

use chrono::Utc;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{ActivityKind, WalletActivity};

use super::service::{observation_telemetry, skipped};
use super::{
    arrival_distance_ms, latency_should_pause, notify, CopyDatabase, CopyPauseReason, CopySkip,
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
        if !task.enabled || task.updated_at > settled_before {
            continue;
        }
        match crate::wallets::watch::copy_source_active(task.id, &task.target_address).await {
            Ok(true) => {}
            Ok(false) => {
                if database
                    .pause_task(task.id, CopyPauseReason::WatchDetached)
                    .await?
                {
                    notify::announce_pause(&task, &CopyPauseReason::WatchDetached).await;
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

pub(super) async fn apply_latency_kill_switch(
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
            let reason = CopyPauseReason::LatencyKillSwitch {
                average_ms,
                threshold_ms,
            };
            if database.pause_task(task.id, reason.clone()).await? {
                notify::announce_pause(&task, &reason).await;
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
