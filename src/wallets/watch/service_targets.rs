//! Per-address runtime state and target-set rebuilding for the observation loop.
//!
//! Split out of `service.rs` to keep that file under the module-size limit;
//! `run()` owns the single event loop, this file owns what one watched
//! address's WS forwarder looks like and how the runtime set follows database
//! changes without interrupting unchanged subscriptions.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::mpsc;

use crate::logger::{self, LogTag};
use crate::transactions::types::Subject;

use super::database::WatchDatabase;
use super::poller;
use super::runtime::WalletWatchRuntime;
use super::types::{WatchNotification, WatchSource, WatchTarget};

/// One watched address's runtime state: its WS forwarder (own-wallet and every
/// target alike subscribe through the same shared transport) and when it was last
/// polled.
pub(super) struct TargetRuntime {
    pub(super) target: WatchTarget,
    pub(super) ws_task: tokio::task::JoinHandle<()>,
    pub(super) last_poll: Instant,
    pub(super) catch_up: Option<poller::CatchUpState>,
    /// First registration establishes a current head without replaying historical
    /// trades as new alerts.
    pub(super) baseline_only: bool,
    /// The open catch-up range was started by a gap-fill.
    pub(super) backfill: bool,
}

/// A non-own target reached its per-wallet page budget before its cursor range
/// completed. Preserve the cursor and pause until the user deliberately resumes.
pub(super) async fn handle_overflow(target_runtime: &mut TargetRuntime, watch_db: &WatchDatabase) {
    let address = target_runtime.target.address.clone();
    let Some(state) = target_runtime.catch_up.take() else {
        return;
    };
    let skipped = state.pending_len();
    let pages = target_runtime.target.page_budget;
    let Some(id) = target_runtime.target.id else {
        return;
    };
    match watch_db.pause_for_budget(id, pages, skipped).await {
        Ok(()) => {
            target_runtime.target.enabled = false;
            target_runtime.target.disable_reason =
                Some(super::types::WatchDisableReason::SignatureBudget {
                    page_budget: pages,
                    signatures_checked: skipped,
                });
            logger::warning(LogTag::WalletWatch, &format!("Paused {address}: reached its {}-signature check limit before catching up; cursor preserved", skipped));
            // Copy's consumer reacts immediately; its own reconciler also retries
            // this state transition if its database is temporarily unavailable.
            super::publish_target_change(address.clone());
            super::service::request_reload();
        }
        Err(e) => logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to pause over-budget target {address}: {e}"),
        ),
    }
}

/// Spawn the per-target WS forwarder: subscribes through the shared transport and
/// funnels every notification into `tx`, tagged with the address, until the
/// subscription ends or the task is aborted (which drops the runtime's
/// subscription handle and unsubscribes, same as any other holder of one).
fn spawn_ws_forwarder(
    address: String,
    tx: mpsc::UnboundedSender<(String, WatchNotification)>,
    runtime: Arc<dyn WalletWatchRuntime>,
) -> (tokio::task::JoinHandle<()>, Arc<AtomicBool>) {
    let active = Arc::new(AtomicBool::new(false));
    let task_active = Arc::clone(&active);
    let task = tokio::spawn(async move {
        let mut retry = Duration::from_secs(1);
        loop {
            if tx.is_closed() {
                break;
            }
            let mut sub = match runtime.subscribe(&address).await {
                Ok(sub) => {
                    task_active.store(true, Ordering::Relaxed);
                    retry = Duration::from_secs(1);
                    sub
                }
                Err(error) => {
                    task_active.store(false, Ordering::Relaxed);
                    logger::warning(
                        LogTag::WalletWatch,
                        &format!("Failed to subscribe to {address}: {error}"),
                    );
                    tokio::time::sleep(retry).await;
                    retry = (retry * 2).min(Duration::from_secs(30));
                    continue;
                }
            };
            while let Some(event) = sub.recv().await {
                if tx.send((address.clone(), event)).is_err() {
                    task_active.store(false, Ordering::Relaxed);
                    return;
                }
            }
            task_active.store(false, Ordering::Relaxed);
            tokio::time::sleep(retry).await;
            retry = (retry * 2).min(Duration::from_secs(30));
        }
    });
    (task, active)
}

fn register(
    runtimes: &mut HashMap<String, TargetRuntime>,
    ws_tx: &mpsc::UnboundedSender<(String, WatchNotification)>,
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    target: WatchTarget,
) {
    let address = target.address.clone();
    let (ws_task, ws_active) =
        spawn_ws_forwarder(address.clone(), ws_tx.clone(), Arc::clone(chain_runtime));
    super::service_state::set_subscription(address.clone(), Arc::clone(&ws_active));
    runtimes.insert(
        address,
        TargetRuntime {
            target,
            ws_task,
            last_poll: Instant::now(),
            catch_up: None,
            baseline_only: false,
            backfill: false,
        },
    );
}

/// Reconcile persisted targets without tearing down subscriptions for unchanged
/// addresses. Each subscription holds a reference to the shared transport; aborting
/// every forwarder during one wallet's pause can close that transport for all.
pub(super) async fn reload_targets(
    runtimes: &mut HashMap<String, TargetRuntime>,
    ws_tx: &mpsc::UnboundedSender<(String, WatchNotification)>,
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    watch_db: &WatchDatabase,
    own_subject: Subject,
    watch_enabled: bool,
) {
    let mut desired = HashMap::new();
    desired.insert(
        own_subject.address(),
        WatchTarget {
            id: None,
            address: own_subject.address(),
            label: Some("Own wallet".to_owned()),
            sources: vec![WatchSource::OwnWallet],
            enabled: true,
            page_budget: super::poller::DEFAULT_PAGE_BUDGET,
            disable_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    );

    if watch_enabled {
        match watch_db.list_targets().await {
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
    for (address, runtime) in runtimes.iter_mut() {
        if let Some(target) = desired.remove(address) {
            runtime.target = target;
        }
    }
    let removed = runtimes
        .keys()
        .filter(|address| !desired_addresses.contains(*address))
        .cloned()
        .collect::<Vec<_>>();
    for address in removed {
        if let Some(runtime) = runtimes.remove(&address) {
            runtime.ws_task.abort();
            super::service_state::remove_subscription(&address);
        }
    }
    // Re-read persisted target addresses so cursor cleanup follows completed target
    // removals. Disabled rows retain their cursors for deliberate resume.
    if let Err(error) = watch_db.purge_orphan_cursors(&own_subject.address()).await {
        logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to purge orphan watch cursors: {error}"),
        );
    }
    for target in desired.into_values() {
        register(runtimes, ws_tx, chain_runtime, target);
    }
}
