//! Per-target workers and target-set reconciliation for wallet watch.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{mpsc, watch, Notify};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::transactions::types::Subject;

use super::database::WatchDatabase;
use super::poller;
use super::runtime::WalletWatchRuntime;
use super::service::{self, ProcessOutcome};
use super::service_state::{self, WsRetry};
use super::types::{WatchNotification, WatchSource, WatchTarget};

const TARGET_EVENT_CAPACITY: usize = 64;
const TARGET_COMMAND_CAPACITY: usize = 8;

/// Per-target state is only accessed by its worker, preserving its cursor and
/// dedupe ordering while other targets process independently.
pub(super) struct TargetRuntime {
    pub(super) target: WatchTarget,
    pub(super) last_poll: Instant,
    pub(super) catch_up: Option<poller::CatchUpState>,
    pub(super) baseline_only: bool,
    pub(super) backfill: bool,
    pub(super) high_activity: bool,
    pub(super) high_activity_catching_up: bool,
    pub(super) high_activity_dirty: bool,
    pub(super) high_activity_failures: u32,
}

enum WorkerCommand {
    GapFill,
}

struct PendingRetry {
    retry: WsRetry,
    due: Instant,
}

/// The registry owns task handles; the worker owns its `TargetRuntime`.
pub(super) struct TargetWorker {
    command_tx: mpsc::Sender<WorkerCommand>,
    target_tx: watch::Sender<WatchTarget>,
    cancel: Arc<Notify>,
    worker_task: tokio::task::JoinHandle<()>,
    forwarder_task: tokio::task::JoinHandle<()>,
}

impl TargetWorker {
    fn update(&self, target: WatchTarget) {
        self.target_tx.send_replace(target);
    }
    fn approval_changed(&self, target: &WatchTarget) -> bool {
        self.target_tx.borrow().high_activity_approved != target.high_activity_approved
    }
    fn gap_fill(&self) {
        let _ = self.command_tx.try_send(WorkerCommand::GapFill);
    }
    pub(super) async fn stop(self) {
        self.cancel.notify_waiters();
        self.worker_task.abort();
        self.forwarder_task.abort();
        let _ = self.worker_task.await;
        let _ = self.forwarder_task.await;
    }
    #[cfg(test)]
    pub(super) fn forwarder_id(&self) -> tokio::task::Id {
        self.forwarder_task.id()
    }
}

/// A target exceeded its bounded raw cursor range. Its persisted cursor remains
/// intact until the user resumes it with a higher budget or Helius support.
pub(super) async fn handle_overflow(runtime: &mut TargetRuntime, db: &WatchDatabase) {
    let address = runtime.target.address.clone();
    let Some(state) = runtime.catch_up.take() else {
        return;
    };
    let (pages, skipped) = (runtime.target.page_budget, state.pending_len());
    let Some(id) = runtime.target.id else {
        return;
    };
    match db.pause_for_budget(id, pages, skipped).await {
        Ok(()) => {
            runtime.target.enabled = false;
            runtime.target.disable_reason =
                Some(super::types::WatchDisableReason::SignatureBudget {
                    page_budget: pages,
                    signatures_checked: skipped,
                });
            logger::warning(LogTag::WalletWatch, &format!("Paused {address}: reached its {skipped}-signature check limit before catching up; cursor preserved"));
            super::publish_target_change(address);
            service::request_reload();
        }
        Err(error) => logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to pause over-budget target {address}: {error}"),
        ),
    }
}

fn spawn_ws_forwarder(
    address: String,
    is_own: bool,
    tx: mpsc::Sender<WatchNotification>,
    runtime: Arc<dyn WalletWatchRuntime>,
    cancel: Arc<Notify>,
    active: Arc<AtomicBool>,
    overflowed: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut retry = Duration::from_secs(1);
        loop {
            let result = tokio::select! { _ = cancel.notified() => return, result = runtime.subscribe(&address) => result };
            let mut subscription = match result {
                Ok(subscription) => {
                    active.store(true, Ordering::Relaxed);
                    retry = Duration::from_secs(1);
                    subscription
                }
                Err(error) => {
                    active.store(false, Ordering::Relaxed);
                    logger::warning(
                        LogTag::WalletWatch,
                        &format!("Failed to subscribe to {address}: {error}"),
                    );
                    tokio::select! { _ = cancel.notified() => return, _ = tokio::time::sleep(retry) => {} }
                    retry = (retry * 2).min(Duration::from_secs(30));
                    continue;
                }
            };
            loop {
                let event = tokio::select! { _ = cancel.notified() => return, event = subscription.recv() => event };
                let Some(event) = event else {
                    break;
                };
                if event.failed && !is_own {
                    continue;
                }
                match tx.try_send(event) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        overflowed.store(true, Ordering::Relaxed)
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => return,
                }
            }
            active.store(false, Ordering::Relaxed);
            tokio::select! { _ = cancel.notified() => return, _ = tokio::time::sleep(retry) => {} }
            retry = (retry * 2).min(Duration::from_secs(30));
        }
    })
}

fn retry_due(retry: &WsRetry) -> Option<Instant> {
    service_state::ws_retry_delay(retry.attempt).map(|delay| Instant::now() + delay)
}

async fn process_event(
    runtime: &mut TargetRuntime,
    event: WatchNotification,
    chain: &Arc<dyn WalletWatchRuntime>,
    own: &Subject,
    retries: &mut Vec<PendingRetry>,
) {
    if !runtime.target.enabled {
        return;
    }
    if service::skip_known_failed_external(&runtime.target, event.failed) {
        return;
    }
    if runtime.high_activity {
        runtime.high_activity_dirty = true;
        return;
    }
    let detected_at = Utc::now();
    if service::process_signature(
        chain,
        &runtime.target,
        own.clone(),
        &event.signature,
        detected_at,
        false,
        None,
    )
    .await
        == ProcessOutcome::Retryable
    {
        let retry = WsRetry {
            address: runtime.target.address.clone(),
            signature: event.signature,
            detected_at,
            attempt: 0,
        };
        if let Some(due) = retry_due(&retry) {
            retries.push(PendingRetry { retry, due });
        }
    }
}

async fn process_retries(
    runtime: &mut TargetRuntime,
    chain: &Arc<dyn WalletWatchRuntime>,
    own: &Subject,
    retries: &mut Vec<PendingRetry>,
) {
    if !runtime.target.enabled {
        retries.clear();
        return;
    }
    let now = Instant::now();
    let pending = std::mem::take(retries);
    for retry in pending {
        if retry.due > now {
            retries.push(retry);
            continue;
        }
        if service::process_signature(
            chain,
            &runtime.target,
            own.clone(),
            &retry.retry.signature,
            retry.retry.detected_at,
            false,
            None,
        )
        .await
            == ProcessOutcome::Retryable
        {
            let next = WsRetry {
                attempt: retry.retry.attempt + 1,
                ..retry.retry
            };
            if let Some(due) = retry_due(&next) {
                retries.push(PendingRetry { retry: next, due });
            }
        }
    }
}

async fn run_worker(
    mut runtime: TargetRuntime,
    mut target_updates: watch::Receiver<WatchTarget>,
    mut commands: mpsc::Receiver<WorkerCommand>,
    mut events: mpsc::Receiver<WatchNotification>,
    overflowed: Arc<AtomicBool>,
    cancel: Arc<Notify>,
    chain: Arc<dyn WalletWatchRuntime>,
    db: WatchDatabase,
    own: Subject,
) {
    // The forwarder subscribes concurrently and buffers references, but this
    // worker establishes its cursor before it processes any of them.
    service::poll_target(&mut runtime, &chain, &db, own.clone(), true).await;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut retries = Vec::new();
    loop {
        tokio::select! {
            _ = cancel.notified() => break,
            Ok(()) = target_updates.changed() => runtime.target = target_updates.borrow_and_update().clone(),
            Some(command) = commands.recv() => match command {
                WorkerCommand::GapFill => { service::poll_target(&mut runtime, &chain, &db, own.clone(), true).await; runtime.last_poll = Instant::now(); }
            },
            Some(event) = events.recv() => process_event(&mut runtime, event, &chain, &own, &mut retries).await,
            _ = tick.tick() => {
                process_retries(&mut runtime, &chain, &own, &mut retries).await;
                let now = Instant::now();
                let (baseline, fallback, high) = with_config(|cfg| (cfg.wallet.watch_poll_interval_secs, cfg.wallet.watch_poll_fallback_secs, cfg.wallet.watch_high_activity_interval_secs));
                let dropped = overflowed.swap(false, Ordering::Relaxed);
                let elapsed = now.duration_since(runtime.last_poll);
                let due = if runtime.high_activity {
                    service::high_activity_poll_due(elapsed, runtime.high_activity_dirty || runtime.high_activity_catching_up || dropped, high, baseline)
                } else {
                    dropped || elapsed >= Duration::from_secs(poller::cadence_secs(chain.is_connected(), baseline, fallback))
                };
                if due {
                    runtime.last_poll = now;
                    runtime.high_activity_dirty = false;
                    service::poll_target(&mut runtime, &chain, &db, own.clone(), false).await;
                }
            }
            else => break,
        }
    }
}

fn register(
    workers: &mut HashMap<String, TargetWorker>,
    chain: &Arc<dyn WalletWatchRuntime>,
    db: &WatchDatabase,
    own: &Subject,
    target: WatchTarget,
) {
    let address = target.address.clone();
    let cancel = Arc::new(Notify::new());
    let (command_tx, command_rx) = mpsc::channel(TARGET_COMMAND_CAPACITY);
    let (target_tx, target_updates) = watch::channel(target.clone());
    let (event_tx, event_rx) = mpsc::channel(TARGET_EVENT_CAPACITY);
    let active = Arc::new(AtomicBool::new(false));
    let overflowed = Arc::new(AtomicBool::new(false));
    service_state::set_subscription(address.clone(), Arc::clone(&active));
    let worker_task = tokio::spawn(run_worker(
        TargetRuntime {
            target: target.clone(),
            last_poll: Instant::now(),
            catch_up: None,
            baseline_only: false,
            backfill: false,
            high_activity: false,
            high_activity_catching_up: false,
            high_activity_dirty: false,
            high_activity_failures: 0,
        },
        target_updates,
        command_rx,
        event_rx,
        Arc::clone(&overflowed),
        Arc::clone(&cancel),
        Arc::clone(chain),
        db.clone(),
        own.clone(),
    ));
    let forwarder_task = spawn_ws_forwarder(
        address.clone(),
        target.sources.contains(&WatchSource::OwnWallet),
        event_tx,
        Arc::clone(chain),
        Arc::clone(&cancel),
        active,
        overflowed,
    );
    workers.insert(
        address,
        TargetWorker {
            command_tx,
            target_tx,
            cancel,
            worker_task,
            forwarder_task,
        },
    );
}

/// Reconcile persisted targets without restarting unchanged workers. Target
/// metadata and budgets are updated in the worker's sole runtime owner.
pub(super) async fn reload_targets(
    workers: &mut HashMap<String, TargetWorker>,
    chain: &Arc<dyn WalletWatchRuntime>,
    db: &WatchDatabase,
    own: Subject,
    watch_enabled: bool,
) {
    let mut desired = HashMap::new();
    desired.insert(
        own.address(),
        WatchTarget {
            id: None,
            address: own.address(),
            label: Some("Own wallet".to_owned()),
            sources: vec![WatchSource::OwnWallet],
            enabled: true,
            page_budget: poller::DEFAULT_PAGE_BUDGET,
            high_activity_approved: false,
            disable_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );
    if watch_enabled {
        match db.list_targets().await {
            Ok(targets) => {
                for target in targets.into_iter().filter(|target| target.enabled) {
                    desired.insert(target.address.clone(), target);
                }
            }
            Err(error) => {
                logger::warning(
                    LogTag::WalletWatch,
                    &format!(
                        "Failed to load watch targets; retaining current subscriptions: {error}"
                    ),
                );
                return;
            }
        }
    }
    let desired_addresses = desired.keys().cloned().collect::<HashSet<_>>();
    let mut restarts = Vec::new();
    for (address, worker) in workers.iter() {
        if let Some(target) = desired.remove(address) {
            if worker.approval_changed(&target) {
                restarts.push((address.clone(), target));
            } else {
                worker.update(target);
            }
        }
    }
    for (address, target) in restarts {
        if let Some(worker) = workers.remove(&address) {
            worker.stop().await;
            service_state::remove_subscription(&address);
            service_state::remove_runtime_status(&address);
        }
        register(workers, chain, db, &own, target);
    }
    let removed = workers
        .keys()
        .filter(|address| !desired_addresses.contains(*address))
        .cloned()
        .collect::<Vec<_>>();
    for address in removed {
        if let Some(worker) = workers.remove(&address) {
            worker.stop().await;
            service_state::remove_subscription(&address);
            service_state::remove_runtime_status(&address);
        }
    }
    if let Err(error) = db.purge_orphan_cursors(&own.address()).await {
        logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to purge orphan watch cursors: {error}"),
        );
    }
    for target in desired.into_values() {
        register(workers, chain, db, &own, target);
    }
}

pub(super) fn request_gap_fill(workers: &HashMap<String, TargetWorker>) {
    for worker in workers.values() {
        worker.gap_fill();
    }
}

pub(super) async fn stop_all(workers: HashMap<String, TargetWorker>) {
    for (address, worker) in workers {
        worker.stop().await;
        service_state::remove_subscription(&address);
        service_state::remove_runtime_status(&address);
    }
}
