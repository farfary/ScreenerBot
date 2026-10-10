// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
/// How long a failed check waits before it is repeated. Without it every running
/// installation asks the release service once a minute for as long as it is down.
const CHECK_RETRY_SECS: u64 = 15 * 60;
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
        // A launch always asks the server once, whatever the interval says: the
        // persisted last check can predate a release by up to the full interval,
        // and a freshly opened app must not report itself current in the meantime.
        let mut launch_check_pending = true;
        loop {
            if !crate::connectivity::is_network_offline() {
                run_cycle(&mut announced, &mut launch_check_pending).await;
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
async fn run_cycle(announced: &mut Option<(String, UpdateStage)>, launch_check_pending: &mut bool) {
    let policy = UpdatePolicy::load();
    if !policy.auto_check {
        return;
    }

    let state = super::get_update_state().await;
    if state.phase.is_busy() {
        return;
    }

    let due = *launch_check_pending || check_due(&state, policy.check_interval_secs, Utc::now());
    let state = if due {
        *launch_check_pending = false;
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
        if policy.auto_download
            && download_wanted(&state, Utc::now())
            && super::self_install_supported(update.kind)
        {
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

                // Reserve only after optional notification I/O, then repeat the
                // activity verdict. begin_trade() and begin_tool() refuse new
                // work under the hold; work that started just before it remains
                // counted and makes this final verdict defer the restart.
                let Some(restart_hold) = crate::global::try_hold_activity_for_update_restart()
                else {
                    return;
                };
                if let ApplyReadiness::Deferred(reason) = super::apply_readiness(update.kind).await
                {
                    drop(restart_hold);
                    record_deferred(&state, &update, reason, announced, policy.notify_telegram)
                        .await;
                } else if let Err(error) = apply_now_while_held().await {
                    drop(restart_hold);
                    logger::warning(
                        LogTag::System,
                        &format!("Automatic update could not be applied: {error}"),
                    );
                } else {
                    // No HTTP response is waiting on the automatic path.
                    restart_hold.commit();
                    crate::global::request_restart();
                }
            }
            ApplyReadiness::Deferred(reason) => {
                record_deferred(&state, &update, reason, announced, policy.notify_telegram).await;
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

/// Whether the interval since the last successful check has run out, holding a
/// check that just failed back for [`CHECK_RETRY_SECS`].
fn check_due(state: &UpdateState, interval_secs: u64, now: chrono::DateTime<Utc>) -> bool {
    let seconds_since = |at: chrono::DateTime<Utc>| (now - at).num_seconds().max(0) as u64;
    let failed_recently = state.check_error.is_some()
        && state
            .last_check_attempt
            .is_some_and(|attempt| seconds_since(attempt) < CHECK_RETRY_SECS.min(interval_secs));
    !failed_recently
        && state
            .last_check
            .is_none_or(|last| seconds_since(last) >= interval_secs)
}

/// Whether the service starts the download of the available update: a fresh
/// one, or one that failed for a reason a later attempt can overcome once its
/// retry time has come. The kept partial file makes that repeat a resume.
fn download_wanted(state: &UpdateState, now: chrono::DateTime<Utc>) -> bool {
    match state.phase {
        UpdatePhase::Available => true,
        UpdatePhase::Failed => state
            .download_progress
            .retry_at
            .is_some_and(|retry_at| retry_at <= now),
        _ => false,
    }
}

async fn record_deferred(
    prior_state: &UpdateState,
    update: &UpdateInfo,
    reason: DeferReason,
    announced: &mut Option<(String, UpdateStage)>,
    notify_telegram: bool,
) {
    if prior_state.deferred != Some(reason) {
        logger::info(
            LogTag::System,
            &format!(
                "Update v{} is staged but held back: {}",
                update.version,
                reason
                    .ui_text()
                    .render_plain(&crate::i18n::source_locale().parse().unwrap_or_default())
            ),
        );
        mutate_state(|state| state.deferred = Some(reason)).await;
    }
    announce(announced, update, UpdateStage::Staged, notify_telegram).await;
}

/// Apply a staged core update by restarting onto it.
///
/// The staged binary is never swapped over a running one: the restart is what
/// activates it, and the desktop shell re-verifies the recorded digest before it
/// launches. A failure here therefore leaves the current version in charge.
pub async fn apply_now() -> Result<()> {
    let Some(restart_hold) = crate::global::try_hold_activity_for_update_restart() else {
        return Err(Error::RestartInProgress);
    };
    apply_now_while_held().await?;
    restart_hold.commit();

    // Give the HTTP response that triggered this a moment to flush, then let the
    // normal run loop stop services and release the process lock in order.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(350)).await;
        crate::global::request_restart();
    });
    Ok(())
}

async fn apply_now_while_held() -> Result<()> {
    let staged = core_install::read_staged_core().ok_or_else(|| Error::DigestMismatch {
        detail: "no verified core is staged".to_owned(),
    })?;
    if !super::is_newer_version(super::VERSION, &staged.version) {
        return Err(Error::UpdateChanged);
    }

    // Validate and claim the apply slot under the same writer lock used by
    // downloads. No caller can begin a transfer between this verdict and the
    // Applying transition.
    super::mutate_state_with_result(|state| {
        let update = state
            .available_update
            .as_ref()
            .ok_or(Error::NoUpdateAvailable)?;
        if update.kind != UpdateKind::Core {
            return Err(Error::UnsupportedInstall {
                detail: "this release replaces the desktop shell and needs its installer"
                    .to_owned(),
            });
        }
        if state.phase != UpdatePhase::ReadyToApply {
            return Err(Error::NoUpdateAvailable);
        }
        if staged.version != update.version {
            return Err(Error::UpdateChanged);
        }
        state.phase = UpdatePhase::Applying;
        state.deferred = None;
        Ok(())
    })
    .await?;

    logger::info(
        LogTag::System,
        &format!(
            "Applying core update v{} — restarting the backend onto the staged binary",
            staged.version
        ),
    );

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

    #[test]
    fn a_failed_check_waits_before_it_is_repeated() {
        let now = Utc::now();
        let interval = 6 * 3600;
        let mut state = UpdateState {
            last_check: Some(now - chrono::Duration::hours(7)),
            last_check_attempt: Some(now - chrono::Duration::minutes(1)),
            check_error: Some(check_failed_text("HTTP 503".to_owned())),
            ..UpdateState::default()
        };
        assert!(!check_due(&state, interval, now));

        state.last_check_attempt = Some(now - chrono::Duration::minutes(16));
        assert!(check_due(&state, interval, now));

        // A short configured interval is never stretched by the back-off.
        state.last_check_attempt = Some(now - chrono::Duration::minutes(6));
        assert!(check_due(&state, 300, now));

        state.check_error = None;
        state.last_check = Some(now - chrono::Duration::hours(1));
        assert!(!check_due(&state, interval, now));
        state.last_check = None;
        assert!(check_due(&state, interval, now));
    }

    #[test]
    fn only_a_transient_download_failure_is_repeated_and_only_when_due() {
        let now = Utc::now();
        let mut state = UpdateState {
            phase: UpdatePhase::Available,
            ..UpdateState::default()
        };
        assert!(download_wanted(&state, now));

        state.phase = UpdatePhase::Failed;
        assert!(
            !download_wanted(&state, now),
            "no retry time: the user decides"
        );
        state.download_progress.retry_at = Some(now + chrono::Duration::minutes(5));
        assert!(!download_wanted(&state, now));
        state.download_progress.retry_at = Some(now - chrono::Duration::seconds(1));
        assert!(download_wanted(&state, now));

        state.phase = UpdatePhase::ReadyToInstall;
        assert!(!download_wanted(&state, now));
    }
}
