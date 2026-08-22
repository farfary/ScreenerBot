//! The observation loop: one funnel, three triggers (§6.2).
//!
//! ```text
//! WS logsNotification ─┐
//! baseline poll        ├─→ process_signature(subject, signature)
//! escalated poll        │     dedupe -> decode -> drop-if-failed -> classify
//! gap-fill (on connect) ┘     -> record -> dedupe.commit -> broadcast
//! ```
//!
//! WS is delivered per-target via a small forwarding task (the shared, multiplexed
//! transport in `rpc::subscriptions` does the actual socket work -- this module never
//! touches a socket). Poll/gap-fill both page `TransactionFetcher::fetch_signatures_page`
//! with `until = watch_cursors` so a restart resumes instead of re-reading history.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use chrono::Utc;
use tokio::sync::{broadcast, mpsc, Notify};
use tokio::time::interval;

use crate::chains::solana::rpc::{self, ConnectionState, SubscriptionEvent};
use crate::chains::solana::transactions::fetcher::TransactionFetcher;
use crate::chains::solana::transactions::processor::TransactionProcessor;
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
use super::types::{WalletActivity, WatchSource, WatchTarget};
use crate::chains::solana::wallets::classify;

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
    if *rpc::connection_state().borrow() == ConnectionState::Connected {
        return true;
    }
    let baseline_secs = with_config(|cfg| cfg.wallet.watch_poll_interval_secs);
    match *SERVICE_STARTED_AT.read().unwrap_or_else(|p| p.into_inner()) {
        Some(started) => started.elapsed() < Duration::from_secs(baseline_secs.max(1)),
        None => false,
    }
}

/// One watched address's runtime state: its WS forwarder (own-wallet and every
/// target alike subscribe through the same shared transport) and when it was last
/// polled.
struct TargetRuntime {
    target: WatchTarget,
    ws_task: tokio::task::JoinHandle<()>,
    last_poll: Instant,
    catch_up: Option<poller::CatchUpState>,
    /// First registration establishes a current head without replaying historical
    /// trades as new alerts.
    baseline_only: bool,
}

/// Spawn the per-target WS forwarder: subscribes through the shared transport and
/// funnels every notification into `tx`, tagged with the address, until the
/// subscription ends or the task is aborted (which drops the `LogsSubscription` and
/// unsubscribes, same as any other holder of one).
fn spawn_ws_forwarder(
    address: String,
    tx: mpsc::UnboundedSender<(String, SubscriptionEvent)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sub = match rpc::subscribe_logs_mentions(&address).await {
            Ok(sub) => sub,
            Err(e) => {
                logger::warning(
                    LogTag::WalletWatch,
                    &format!("Failed to subscribe to {address}: {e}"),
                );
                return;
            }
        };
        while let Some(event) = sub.recv().await {
            if tx.send((address.clone(), event)).is_err() {
                break;
            }
        }
    })
}

fn register(
    runtimes: &mut HashMap<String, TargetRuntime>,
    ws_tx: &mpsc::UnboundedSender<(String, SubscriptionEvent)>,
    target: WatchTarget,
) {
    let address = target.address.clone();
    let ws_task = spawn_ws_forwarder(address.clone(), ws_tx.clone());
    runtimes.insert(
        address,
        TargetRuntime {
            target,
            ws_task,
            last_poll: Instant::now(),
            catch_up: None,
            baseline_only: false,
        },
    );
}

/// Rebuild the runtime target set from the database: the own wallet (always present,
/// never persisted) plus every enabled row in `watch_targets`. Simple full-rebuild
/// rather than a diff -- target management is a low-frequency, human-driven action
/// (`watch_max_targets` caps the whole set to single digits/low tens by default), so
/// a brief resubscribe-everything on change is not a real cost.
async fn reload_targets(
    runtimes: &mut HashMap<String, TargetRuntime>,
    ws_tx: &mpsc::UnboundedSender<(String, SubscriptionEvent)>,
    watch_db: &WatchDatabase,
    own_subject: Subject,
) {
    for runtime in runtimes.values() {
        runtime.ws_task.abort();
    }
    runtimes.clear();

    // The own wallet is always watched, regardless of `wallet.watch_enabled` -- that
    // switch is the master control for TARGET watching (pasted addresses), not for
    // the own-wallet observation `TransactionsService` now structurally depends on.
    register(
        runtimes,
        ws_tx,
        WatchTarget {
            id: None,
            address: own_subject.address(),
            label: Some("Own wallet".to_owned()),
            sources: vec![WatchSource::OwnWallet],
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );

    if !with_config(|cfg| cfg.wallet.watch_enabled) {
        return;
    }

    match watch_db.list_targets().await {
        Ok(targets) => {
            for target in targets.into_iter().filter(|t| t.enabled) {
                register(runtimes, ws_tx, target);
            }
        }
        Err(e) => logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to load watch targets: {e}"),
        ),
    }
}

/// Poll one target's cursor forward. Used for the baseline poll, the escalated poll
/// and gap-fill alike -- they differ only in WHEN this is called, never in what it
/// does (§6.2: "every path converges on the same funnel").
async fn poll_target(runtime: &mut TargetRuntime, watch_db: &WatchDatabase, own_subject: Subject) {
    let Ok(pubkey) = Pubkey::from_str(&runtime.target.address) else {
        return;
    };

    if runtime.catch_up.is_none() {
        let cursor = watch_db
            .get_cursor(&runtime.target.address)
            .await
            .unwrap_or_default();
        runtime.baseline_only = !runtime.target.sources.contains(&WatchSource::OwnWallet)
            && !watch_db
                .has_cursor_row(&runtime.target.address)
                .await
                .unwrap_or(false);
        runtime.catch_up = Some(poller::CatchUpState::new(cursor));
    }

    let fetcher = TransactionFetcher::new();
    let completed = match poller::advance_catch_up(
        &fetcher,
        pubkey,
        runtime.catch_up.as_mut().expect("catch-up initialized"),
    )
    .await
    {
        Ok(Some(completed)) => completed,
        Ok(None) => return,
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Poll failed for {}: {e}", runtime.target.address),
            );
            return;
        }
    };

    if runtime.baseline_only {
        let baseline_result = match completed.newest_signature.as_deref() {
            Some(newest) => watch_db.set_cursor(&runtime.target.address, newest).await,
            None => {
                watch_db
                    .mark_cursor_initialized(&runtime.target.address)
                    .await
            }
        };
        if let Err(e) = baseline_result {
            logger::warning(
                LogTag::WalletWatch,
                &format!(
                    "Failed to establish watch baseline for {}: {e}",
                    runtime.target.address
                ),
            );
            return;
        }
        runtime.catch_up = None;
        runtime.baseline_only = false;
        return;
    }

    let mut replay_complete = true;
    for signature in &completed.signatures {
        if process_signature(&runtime.target, own_subject.clone(), signature, Utc::now()).await
            == ProcessOutcome::Retryable
        {
            replay_complete = false;
        }
    }

    if !replay_complete {
        return;
    }

    if let Some(newest) = completed.newest_signature.as_deref() {
        if let Err(e) = watch_db.set_cursor(&runtime.target.address, newest).await {
            logger::warning(
                LogTag::WalletWatch,
                &format!(
                    "Failed to advance watch cursor for {}: {e}",
                    runtime.target.address
                ),
            );
            return;
        }
    }

    runtime.catch_up = None;
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
    target: &WatchTarget,
    _own_subject: Subject,
    signature: &str,
    detected_at: chrono::DateTime<Utc>,
) -> ProcessOutcome {
    let Ok(pubkey) = Pubkey::from_str(&target.address) else {
        return ProcessOutcome::Terminal;
    };
    let subject = crate::chains::solana::transactions::subject::from_pubkey(pubkey);

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
    let processor = if is_own {
        TransactionProcessor::new(pubkey)
    } else {
        TransactionProcessor::new_for_watch_target(pubkey)
    };

    let transaction = match processor.decode(signature).await {
        Ok(tx) => tx,
        Err(e) => {
            if crate::errors::is_rpc_indexing_delay(&e) || e.contains("RPC indexing delay") {
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

    let Some((kind, skip_reason)) =
        classify::classify_transaction_activity(&target.address, &transaction)
    else {
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
    });

    ProcessOutcome::Terminal
}

fn short(signature: &str) -> &str {
    &signature[..signature.len().min(8)]
}

/// The service loop. Runs until `shutdown` is notified.
pub(super) async fn run(shutdown: Arc<Notify>, watch_db: WatchDatabase, own_subject: Subject) {
    *SERVICE_STARTED_AT
        .write()
        .unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());

    let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<(String, SubscriptionEvent)>();
    let mut runtimes: HashMap<String, TargetRuntime> = HashMap::new();
    reload_targets(&mut runtimes, &ws_tx, &watch_db, own_subject.clone()).await;

    // Gap-fill at service start: catch up on anything that landed while the bot was
    // down, for every registered target including the own wallet.
    let startup_addresses: Vec<String> = runtimes.keys().cloned().collect();
    for address in startup_addresses {
        if let Some(runtime) = runtimes.get_mut(&address) {
            poll_target(runtime, &watch_db, own_subject.clone()).await;
        }
    }

    let mut connection_state = rpc::connection_state();
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
                if let Some(runtime) = runtimes.get(&address) {
                    if event.failed {
                        logger::debug(
                            LogTag::WalletWatch,
                            &format!("Notification for a failed transaction on {address}, decoding anyway to confirm"),
                        );
                    }
                    process_signature(&runtime.target, own_subject.clone(), &event.signature, Utc::now()).await;
                }
            }
            result = connection_state.changed() => {
                if result.is_err() {
                    // Sender half of the transport's watch channel is gone (process
                    // shutting down) -- nothing left to react to.
                    continue;
                }
                if poller::needs_gap_fill(
                    false,
                    false,
                    *connection_state.borrow() == ConnectionState::Connected,
                ) {
                    logger::debug(LogTag::WalletWatch, "Subscription transport reconnected - gap-filling every target");
                    let addresses: Vec<String> = runtimes.keys().cloned().collect();
                    for address in addresses {
                        if let Some(runtime) = runtimes.get_mut(&address) {
                            poll_target(runtime, &watch_db, own_subject.clone()).await;
                        }
                    }
                }
            }
            _ = RELOAD_NOTIFY.notified() => {
                reload_targets(&mut runtimes, &ws_tx, &watch_db, own_subject.clone()).await;
            }
            _ = tick.tick() => {
                let state = *connection_state.borrow();
                let (baseline_secs, fallback_secs) = with_config(|cfg| {
                    (cfg.wallet.watch_poll_interval_secs, cfg.wallet.watch_poll_fallback_secs)
                });
                let now = Instant::now();
                let due: Vec<String> = runtimes
                    .iter_mut()
                    .filter_map(|runtime| {
                        let (address, runtime) = runtime;
                        let interval_secs = poller::cadence_secs(
                            state == ConnectionState::Connected,
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
                    if let Some(runtime) = runtimes.get_mut(&address) {
                        poll_target(runtime, &watch_db, own_subject.clone()).await;
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
