//! The observation loop: one funnel, three triggers (§6.2).
//!
//! ```text
//! WS logsNotification ─┐
//! baseline poll        ├─→ process_signature(subject, signature)
//! escalated poll        │     dedupe -> decode -> drop-if-failed -> classify
//! gap-fill (on connect) ┘     -> record -> dedupe.commit -> broadcast
//! ```
//!
//! WS is delivered per-target via a small forwarding task, backed by the injected
//! `runtime::WalletWatchRuntime::subscribe` -- this module never touches a chain
//! wire format directly. Poll/gap-fill both page `WalletWatchRuntime::
//! fetch_signatures_page` with `until = watch_cursors` so a restart resumes
//! instead of re-reading history.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{broadcast, mpsc, Notify};
use tokio::time::interval;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::transactions::types::Subject;
use crate::transactions::utils::{
    add_pending_transaction_globally, remove_pending_transaction_globally,
};

use super::database::WatchDatabase;
use super::dedupe;
use super::poller;
use super::recorder;
use super::runtime::WalletWatchRuntime;
use super::types::{WalletActivity, WatchNotification, WatchSource, WatchTarget};

/// Bound generous enough that a burst across every watched target cannot fill the
/// channel before the slowest consumer (a Telegram send) catches up. Bounded so a
/// lagging consumer drops rather than stalls the pipeline -- a slow Telegram send
/// must never delay a decision downstream (plan §6).
const ACTIVITY_CHANNEL_CAPACITY: usize = 1024;

static ACTIVITY_CHANNEL: LazyLock<broadcast::Sender<WalletActivity>> =
    LazyLock::new(|| broadcast::channel(ACTIVITY_CHANNEL_CAPACITY).0);

/// Subscribe to every piece of activity the service detects, across every subject.
/// Consumers filter by `subject` and by which `WatchSource` they care about (the
/// own-wallet balance-refresh hook and the Telegram alert consumer both do exactly
/// this, filtered to disjoint sources, from the same feed).
pub fn subscribe_activity() -> broadcast::Receiver<WalletActivity> {
    ACTIVITY_CHANNEL.subscribe()
}

fn publish(activity: WalletActivity) {
    // Err only means "no receivers right now", not a pipeline error.
    let _ = ACTIVITY_CHANNEL.send(activity);
}

/// Ask the running service to resync its target set from the database. Cheap and
/// idempotent -- called by the CRUD API after add/remove/enable/disable.
static RELOAD_NOTIFY: LazyLock<Notify> = LazyLock::new(Notify::new);

pub(super) fn request_reload() {
    RELOAD_NOTIFY.notify_one();
}

static SERVICE_STARTED_AT: LazyLock<std::sync::RwLock<Option<Instant>>> =
    LazyLock::new(|| std::sync::RwLock::new(None));

/// `Healthy` once the shared subscription transport is `Connected`, or still inside
/// the startup grace window (one baseline poll interval) so a normal cold-start
/// reconnect is not reported as degraded before it has had a fair chance. `Degraded`
/// once that window has passed and the transport is still down -- detection is
/// running on polling alone.
pub(super) fn is_healthy() -> bool {
    if super::runtime::try_get_runtime().is_some_and(|runtime| runtime.is_connected()) {
        return true;
    }
    let baseline_secs = with_config(|cfg| cfg.wallet.watch_poll_interval_secs);
    match *SERVICE_STARTED_AT.read().unwrap_or_else(|p| p.into_inner()) {
        Some(started) => started.elapsed() < Duration::from_secs(baseline_secs.max(1)),
        None => false,
    }
}

use super::service_state::{self, WsRetry};
use super::service_targets::{handle_overflow, reload_targets, TargetRuntime};

/// Poll one target's cursor forward. Used for the baseline poll, the escalated poll
/// and gap-fill alike -- they differ only in WHEN this is called, never in what it
/// does (§6.2: "every path converges on the same funnel").
async fn poll_target(
    target_runtime: &mut TargetRuntime,
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    watch_db: &WatchDatabase,
    own_subject: Subject,
    backfill: bool,
) {
    if chain_runtime
        .resolve_subject(&target_runtime.target.address)
        .is_err()
    {
        return;
    }

    if target_runtime.catch_up.is_none() {
        let cursor = watch_db
            .get_cursor(&target_runtime.target.address)
            .await
            .unwrap_or_default();
        target_runtime.baseline_only = !target_runtime
            .target
            .sources
            .contains(&WatchSource::OwnWallet)
            && !watch_db
                .has_cursor_row(&target_runtime.target.address)
                .await
                .unwrap_or(false);
        target_runtime.catch_up = Some(poller::CatchUpState::new(cursor));
        target_runtime.backfill = backfill;
    } else {
        // A range a gap-fill opened stays a backfill when a cadence tick finishes it.
        target_runtime.backfill |= backfill;
    }

    let completed = match poller::advance_catch_up(
        chain_runtime.as_ref(),
        &target_runtime.target.address,
        target_runtime
            .catch_up
            .as_mut()
            .expect("catch-up initialized"),
    )
    .await
    {
        Ok(Some(completed)) => completed,
        Ok(None) => {
            // The own wallet's history is the trading ledger: it is never skipped,
            // it keeps paging on every tick until the range completes.
            if !target_runtime
                .target
                .sources
                .contains(&WatchSource::OwnWallet)
            {
                handle_overflow(target_runtime, watch_db).await;
            }
            return;
        }
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Poll failed for {}: {e}", target_runtime.target.address),
            );
            return;
        }
    };

    if target_runtime.baseline_only {
        let baseline_result = match completed.newest_signature.as_deref() {
            Some(newest) => {
                watch_db
                    .set_cursor(&target_runtime.target.address, newest)
                    .await
            }
            None => {
                watch_db
                    .mark_cursor_initialized(&target_runtime.target.address)
                    .await
            }
        };
        if let Err(e) = baseline_result {
            logger::warning(
                LogTag::WalletWatch,
                &format!(
                    "Failed to establish watch baseline for {}: {e}",
                    target_runtime.target.address
                ),
            );
            return;
        }
        target_runtime.catch_up = None;
        target_runtime.baseline_only = false;
        target_runtime.overflow_streak = 0;
        return;
    }
    target_runtime.overflow_streak = 0;

    let mut replay_complete = true;
    for seen in &completed.signatures {
        if process_signature(
            chain_runtime,
            &target_runtime.target,
            own_subject.clone(),
            &seen.signature,
            seen.detected_at,
            target_runtime.backfill,
        )
        .await
            == ProcessOutcome::Retryable
        {
            replay_complete = false;
        }
    }

    if !replay_complete {
        return;
    }

    if let Some(newest) = completed.newest_signature.as_deref() {
        if let Err(e) = watch_db
            .set_cursor(&target_runtime.target.address, newest)
            .await
        {
            logger::warning(
                LogTag::WalletWatch,
                &format!(
                    "Failed to advance watch cursor for {}: {e}",
                    target_runtime.target.address
                ),
            );
            return;
        }
    }

    target_runtime.catch_up = None;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessOutcome {
    Terminal,
    Retryable,
}

async fn mark_pending(subject: Subject, signature: &str, detected_at: chrono::DateTime<Utc>) {
    add_pending_transaction_globally(subject.clone(), signature.to_owned(), detected_at).await;

    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        let pending = HashMap::from([(signature.to_owned(), detected_at)]);
        if let Err(e) = db
            .save_pending_transactions(subject.clone(), &pending)
            .await
        {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to persist pending signature {signature} for {subject}: {e}"),
            );
        }
    }
}

async fn clear_pending(subject: Subject, signature: &str) {
    remove_pending_transaction_globally(subject.clone(), signature).await;

    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        if let Err(e) = db
            .remove_pending_transaction(subject.clone(), signature)
            .await
        {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to clear pending signature {signature} for {subject}: {e}"),
            );
        }
    }
}

/// The one funnel every trigger feeds: durable dedupe, pure decode, classify,
/// record, dedupe-commit, broadcast.
async fn process_signature(
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    target: &WatchTarget,
    _own_subject: Subject,
    signature: &str,
    detected_at: chrono::DateTime<Utc>,
    backfill: bool,
) -> ProcessOutcome {
    let Ok(subject) = chain_runtime.resolve_subject(&target.address) else {
        return ProcessOutcome::Terminal;
    };

    let already_seen = match dedupe::has_seen(subject.clone(), signature).await {
        Ok(seen) => seen,
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed dedupe admission for {signature} on {subject}: {e}"),
            );
            return ProcessOutcome::Retryable;
        }
    };
    if already_seen {
        // Reconcile a crash between durable dedupe commit and pending cleanup.
        clear_pending(subject.clone(), signature).await;
        return ProcessOutcome::Terminal;
    }

    mark_pending(subject.clone(), signature, detected_at).await;

    let is_own = target.sources.contains(&WatchSource::OwnWallet);

    let transaction = match chain_runtime
        .decode_transaction(&target.address, signature, is_own)
        .await
    {
        Ok(tx) => tx,
        Err(e) => {
            if matches!(
                &e,
                crate::wallets::Error::ChainExecution(
                    crate::chains::ExecutionFailure::IndexingDelay { .. }
                        | crate::chains::ExecutionFailure::NotFound { .. }
                )
            ) {
                // Keep both pending records intact. A completed poll range stays in
                // memory and is replayed next cadence; a WS-first notification is
                // picked up by the baseline poll. The durable cursor remains behind
                // it, so restart re-pages instead of skipping it.
                return ProcessOutcome::Retryable;
            }

            logger::debug(
                LogTag::WalletWatch,
                &format!(
                    "Decode failed permanently for {} on {}: {e}",
                    short(signature),
                    target.address
                ),
            );
            if dedupe::commit(subject.clone(), signature).await.is_err() {
                return ProcessOutcome::Retryable;
            }
            clear_pending(subject, signature).await;
            return ProcessOutcome::Terminal;
        }
    };
    let decoded_at = Utc::now();

    if !transaction.success {
        // Never act on a failed transaction (plan §6.2). The own wallet still keeps
        // its existing behaviour of recording failures for history/debugging;
        // targets get nothing recorded and no broadcast -- there is nothing to
        // alert on and nothing that moved.
        if is_own {
            if recorder::record(subject.clone(), &target.sources, &transaction)
                .await
                .is_err()
            {
                return ProcessOutcome::Retryable;
            }
        }
        if dedupe::commit(subject.clone(), signature).await.is_err() {
            return ProcessOutcome::Retryable;
        }
        clear_pending(subject, signature).await;
        return ProcessOutcome::Terminal;
    }

    let Some((kind, skip_reason)) = chain_runtime.classify(&target.address, &transaction) else {
        if dedupe::commit(subject.clone(), signature).await.is_err() {
            return ProcessOutcome::Retryable;
        }
        clear_pending(subject, signature).await;
        return ProcessOutcome::Terminal;
    };
    if let Some(reason) = skip_reason {
        logger::debug(
            LogTag::WalletWatch,
            &format!(
                "{} on {} classified as Other: {reason}",
                short(signature),
                target.address
            ),
        );
    }

    if recorder::record(subject.clone(), &target.sources, &transaction)
        .await
        .is_err()
    {
        // Do not make a failed persistence attempt durable in dedupe or visible to
        // consumers. The retained poll range retries it without losing ordering.
        return ProcessOutcome::Retryable;
    }
    if dedupe::commit(subject.clone(), signature).await.is_err() {
        return ProcessOutcome::Retryable;
    }
    clear_pending(subject, signature).await;

    publish(WalletActivity {
        subject: target.address.clone(),
        signature: signature.to_owned(),
        slot: transaction.slot.unwrap_or_default(),
        block_time: transaction.block_time,
        detected_at,
        decoded_at,
        success: true,
        kind,
        sources: target.sources.clone(),
        backfill,
    });

    ProcessOutcome::Terminal
}

fn short(signature: &str) -> &str {
    &signature[..signature.len().min(8)]
}

/// The service loop. Runs until `shutdown` is notified.
pub(super) async fn run(
    shutdown: Arc<Notify>,
    chain_runtime: Arc<dyn WalletWatchRuntime>,
    watch_db: WatchDatabase,
    own_subject: Subject,
) {
    *SERVICE_STARTED_AT
        .write()
        .unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());

    let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<(String, WatchNotification)>();
    let (retry_tx, mut retry_rx) = mpsc::unbounded_channel::<WsRetry>();
    let mut runtimes: HashMap<String, TargetRuntime> = HashMap::new();
    reload_targets(
        &mut runtimes,
        &ws_tx,
        &chain_runtime,
        &watch_db,
        own_subject.clone(),
    )
    .await;

    // Gap-fill at service start: catch up on anything that landed while the bot was
    // down, for every registered target including the own wallet.
    let startup_addresses: Vec<String> = runtimes.keys().cloned().collect();
    for address in startup_addresses {
        if let Some(target_runtime) = runtimes.get_mut(&address) {
            poll_target(
                target_runtime,
                &chain_runtime,
                &watch_db,
                own_subject.clone(),
                true,
            )
            .await;
        }
    }

    let mut connection_watch = chain_runtime.connection_watch();
    let mut tick = interval(Duration::from_secs(1));
    let mut retention_tick = interval(Duration::from_secs(3600));
    retention_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // The first tick fires immediately; the interval above already ran the real
    // gap-fill pass, so skip straight to waiting a full hour before the first cleanup.
    retention_tick.reset();

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::WalletWatch, "Wallet watch service shutting down");
                break;
            }
            _ = retention_tick.tick() => {
                run_retention_cleanup(own_subject.clone()).await;
            }
            Some((address, event)) = ws_rx.recv() => {
                if let Some(target_runtime) = runtimes.get(&address) {
                    if event.failed {
                        logger::debug(
                            LogTag::WalletWatch,
                            &format!("Notification for a failed transaction on {address}, decoding anyway to confirm"),
                        );
                    }
                    let detected_at = Utc::now();
                    let outcome = process_signature(&chain_runtime, &target_runtime.target, own_subject.clone(), &event.signature, detected_at, false).await;
                    if outcome == ProcessOutcome::Retryable {
                        // Usually the RPC has not indexed the transaction yet. Retry
                        // on a short backoff instead of waiting a full poll interval.
                        service_state::schedule_ws_retry(&retry_tx, WsRetry {
                            address,
                            signature: event.signature,
                            detected_at,
                            attempt: 0,
                        });
                    }
                }
            }
            Some(retry) = retry_rx.recv() => {
                if let Some(target_runtime) = runtimes.get(&retry.address) {
                    let outcome = process_signature(&chain_runtime, &target_runtime.target, own_subject.clone(), &retry.signature, retry.detected_at, false).await;
                    if outcome == ProcessOutcome::Retryable
                        && !service_state::schedule_ws_retry(&retry_tx, retry.clone())
                    {
                        logger::debug(
                            LogTag::WalletWatch,
                            &format!("{} on {} still undecodable after WS retries; the poll will pick it up", short(&retry.signature), retry.address),
                        );
                    }
                }
            }
            result = connection_watch.changed() => {
                if result.is_err() {
                    // Sender half of the transport's watch channel is gone (process
                    // shutting down) -- nothing left to react to.
                    continue;
                }
                if poller::needs_gap_fill(
                    false,
                    false,
                    connection_watch.is_connected(),
                ) {
                    logger::debug(LogTag::WalletWatch, "Subscription transport reconnected - gap-filling every target");
                    let addresses: Vec<String> = runtimes.keys().cloned().collect();
                    for address in addresses {
                        if let Some(target_runtime) = runtimes.get_mut(&address) {
                            poll_target(target_runtime, &chain_runtime, &watch_db, own_subject.clone(), true).await;
                        }
                    }
                }
            }
            _ = RELOAD_NOTIFY.notified() => {
                reload_targets(&mut runtimes, &ws_tx, &chain_runtime, &watch_db, own_subject.clone()).await;
            }
            _ = tick.tick() => {
                let connected = connection_watch.is_connected();
                let (baseline_secs, fallback_secs) = with_config(|cfg| {
                    (cfg.wallet.watch_poll_interval_secs, cfg.wallet.watch_poll_fallback_secs)
                });
                let now = Instant::now();
                let due: Vec<String> = runtimes
                    .iter_mut()
                    .filter_map(|runtime| {
                        let (address, runtime) = runtime;
                        let interval_secs = poller::cadence_secs(
                            connected,
                            baseline_secs,
                            fallback_secs,
                        );
                        if now.duration_since(runtime.last_poll) >= Duration::from_secs(interval_secs) {
                            runtime.last_poll = now;
                            Some(address.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for address in due {
                    if let Some(target_runtime) = runtimes.get_mut(&address) {
                        poll_target(target_runtime, &chain_runtime, &watch_db, own_subject.clone(), false).await;
                    }
                }
            }
        }
    }

    for runtime in runtimes.values() {
        runtime.ws_task.abort();
    }
    *SERVICE_STARTED_AT
        .write()
        .unwrap_or_else(|p| p.into_inner()) = None;
}

/// Periodic maintenance: purge watched-target rows past their retention window. Not
/// part of the hot `run()` loop -- called on a slow interval by whatever wires the
/// service up, matching how `TransactionsService` already separates its own periodic
/// health/cleanup pass from message handling.
pub(super) async fn run_retention_cleanup(own_subject: Subject) {
    let retention_days = with_config(|cfg| cfg.wallet.watch_retention_days);
    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        if let Err(e) = db
            .cleanup_stale_target_transactions(&own_subject.address(), retention_days)
            .await
        {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Watch retention cleanup failed: {e}"),
            );
        }
    }
}

#[cfg(test)]
#[path = "service/tests.rs"]
mod tests;
