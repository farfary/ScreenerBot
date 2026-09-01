//! The background update service: check, fetch, and — when it is safe — apply.

use super::policy::UpdatePolicy;
use super::types::*;
use super::{core_install, mutate_state, ApplyReadiness, Error, Result};
use crate::logger::{self, LogTag};
use crate::telegram::types::{Notification, UpdateStage};
use chrono::Utc;
use std::time::Duration;

/// How long after boot the first check runs, so it never competes with startup.
const FIRST_CHECK_DELAY_SECS: u64 = 30;
/// How often the loop re-evaluates. The check interval itself comes from config
/// and is compared against the last successful check, so changing it in settings
/// takes effect within a minute instead of at the next long tick.
const TICK_SECS: u64 = 60;

pub fn start_update_check_service(
    shutdown: std::sync::Arc<tokio::sync::Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(monitor.instrument(async move {
        let policy = UpdatePolicy::load();
        logger::info(
            LogTag::System,
            &format!(
                "Update service started (every {}h, download={}, install={})",
                policy.check_interval_secs / 3600,
                policy.auto_download,
                policy.auto_install
            ),
        );

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(FIRST_CHECK_DELAY_SECS)) => {}
            _ = shutdown.notified() => return,
        }

        let mut announced: Option<(String, UpdateStage)> = None;
        loop {
            if !crate::connectivity::is_network_offline() {
                run_cycle(&mut announced).await;
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(TICK_SECS)) => {}
                _ = shutdown.notified() => break,
            }
        }
    }))
}

/// One pass of the update state machine. Every step is a no-op unless the
/// previous one left something to do, so a tick costs nothing while idle.
async fn run_cycle(announced: &mut Option<(String, UpdateStage)>) {
    let policy = UpdatePolicy::load();
    if !policy.auto_check {
        return;
    }

    let state = super::get_update_state().await;
    if state.phase.is_busy() {
        return;
    }

    let due = state
        .last_check
        .is_none_or(|last| (Utc::now() - last).num_seconds() as u64 >= policy.check_interval_secs);
    let state = if due {
        if let Err(error) = super::check_for_update().await {
            logger::warning(LogTag::System, &format!("Update check failed: {error}"));
            return;
        }
        super::get_update_state().await
    } else {
        state
    };

    let Some(update) = state.available_update.clone() else {
        return;
    };

    if matches!(state.phase, UpdatePhase::Available | UpdatePhase::Failed) {
        announce(
            announced,
            &update,
            UpdateStage::Available,
            policy.notify_telegram,
        )
        .await;
        if policy.auto_download && state.phase == UpdatePhase::Available {
            if let Err(error) = super::start_download(update.clone()).await {
                logger::warning(
                    LogTag::System,
                    &format!("Automatic update download did not start: {error}"),
                );
            }
        }
        return;
    }

    if state.phase == UpdatePhase::ReadyToApply {
        let readiness = super::apply_readiness(update.kind).await;
        match readiness {
            ApplyReadiness::Ready => {
                announce(
                    announced,
                    &update,
                    UpdateStage::Applying,
                    policy.notify_telegram,
                )
                .await;
                if let Err(error) = apply_now().await {
                    logger::warning(
                        LogTag::System,
                        &format!("Automatic update could not be applied: {error}"),
                    );
                }
            }
            ApplyReadiness::Deferred(reason) => {
                if state.deferred != Some(reason) {
                    logger::info(
                        LogTag::System,
                        &format!(
                            "Update v{} is staged but held back: {}",
                            update.version,
                            reason.message()
                        ),
                    );
                    mutate_state(|state| state.deferred = Some(reason)).await;
                }
                announce(
                    announced,
                    &update,
                    UpdateStage::Staged,
                    policy.notify_telegram,
                )
                .await;
            }
        }
        return;
    }

    if state.phase == UpdatePhase::ReadyToInstall {
        mutate_state(|state| state.deferred = Some(DeferReason::NeedsInstaller)).await;
        announce(
            announced,
            &update,
            UpdateStage::Staged,
            policy.notify_telegram,
        )
        .await;
    }
}

/// Apply a staged core update by restarting onto it.
///
/// The staged binary is never swapped over a running one: the restart is what
/// activates it, and the desktop shell re-verifies the recorded digest before it
/// launches. A failure here therefore leaves the current version in charge.
pub async fn apply_now() -> Result<()> {
    let state = super::get_update_state().await;
    let update = state.available_update.ok_or(Error::NoUpdateAvailable)?;

    if update.kind != UpdateKind::Core {
        return Err(Error::UnsupportedInstall {
            detail: "this release replaces the desktop shell and needs its installer".to_owned(),
        });
    }
    if state.phase != UpdatePhase::ReadyToApply {
        return Err(Error::NoUpdateAvailable);
    }
    let staged = core_install::read_staged_core().ok_or_else(|| Error::DigestMismatch {
        detail: "no verified core is staged".to_owned(),
    })?;
    if staged.version != update.version {
        return Err(Error::UpdateChanged);
    }
    if !super::is_newer_version(super::VERSION, &staged.version) {
        return Err(Error::UpdateChanged);
    }

    mutate_state(|state| {
        state.phase = UpdatePhase::Applying;
        state.deferred = None;
    })
    .await;

    logger::info(
        LogTag::System,
        &format!(
            "Applying core update v{} — restarting the backend onto the staged binary",
            staged.version
        ),
    );

    // Give the HTTP response that triggered this a moment to flush, then let the
    // normal run loop stop services and release the process lock in order.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(350)).await;
        crate::global::request_restart();
    });
    Ok(())
}

/// Send one Telegram message per (version, stage) so a held-back update does not
/// re-announce itself every minute.
async fn announce(
    announced: &mut Option<(String, UpdateStage)>,
    update: &UpdateInfo,
    stage: UpdateStage,
    enabled: bool,
) {
    let key = (update.version.clone(), stage);
    if announced.as_ref() == Some(&key) {
        return;
    }
    *announced = Some(key);
    if !enabled {
        return;
    }
    crate::telegram::notifier::send_notification(Notification::update_status(
        update.version.clone(),
        stage,
        update.kind.is_silent(),
        update.transfer_size(),
    ))
    .await;
}

#[cfg(test)]
mod tests {
    use super::super::policy::decide;
    use super::*;

    #[test]
    fn the_decision_table_drives_the_service() {
        // The service only ever restarts on a Ready verdict; every other verdict
        // records a reason and leaves the staged core waiting.
        assert!(decide(UpdateKind::Core, true, true, 0, false).is_ready());
        assert!(!decide(UpdateKind::Core, true, true, 1, false).is_ready());
        assert!(!decide(UpdateKind::Full, true, true, 0, false).is_ready());
    }
}
