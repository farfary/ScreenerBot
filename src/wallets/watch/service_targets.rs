//! Per-address runtime state and target-set rebuilding for the observation loop.
//!
//! Split out of `service.rs` to keep that file under the module-size limit;
//! `run()` owns the single event loop, this file owns what one watched
//! address's WS forwarder looks like and how the whole set gets rebuilt from
//! the database.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::mpsc;

use crate::config::with_config;
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
    /// Consecutive polls whose range did not fit in one tick's page budget.
    pub(super) overflow_streak: u32,
}

/// Consecutive over-budget polls after which a non-own target is disabled.
const SATURATION_STREAK: u32 = 2;

/// A non-own target produced more signatures than one tick can page
/// (`MAX_PAGES` x `PAGE_SIZE`). Holding the range open would starve it forever and
/// replaying it later only yields stale history, so the first overflow skips the
/// gap to the current head, and a second one in a row means the wallet out-trades
/// the poll budget: it is disabled with the reason surfaced in its status.
pub(super) async fn handle_overflow(target_runtime: &mut TargetRuntime, watch_db: &WatchDatabase) {
    let address = target_runtime.target.address.clone();
    let Some(state) = target_runtime.catch_up.take() else {
        return;
    };
    target_runtime.overflow_streak += 1;
    let skipped = state.pending_len();

    if target_runtime.overflow_streak >= SATURATION_STREAK {
        let reason = format!(
            "Disabled: more than {skipped} transactions per poll interval in {} consecutive polls exceeds the watch budget",
            target_runtime.overflow_streak
        );
        logger::warning(LogTag::WalletWatch, &format!("{address}: {reason}"));
        if let Some(id) = target_runtime.target.id {
            match watch_db.set_enabled(id, false).await {
                Ok(()) => {
                    super::service_state::mark_saturated(&address, reason);
                    super::service::request_reload();
                }
                Err(e) => logger::warning(
                    LogTag::WalletWatch,
                    &format!("Failed to disable saturated target {address}: {e}"),
                ),
            }
        }
        return;
    }

    if let Some(newest) = state.newest_signature() {
        match watch_db.set_cursor(&address, newest).await {
            Ok(()) => logger::warning(
                LogTag::WalletWatch,
                &format!("{address} exceeded the poll budget; skipped {skipped}+ older transactions to the current head"),
            ),
            Err(e) => logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to re-baseline overflowing target {address}: {e}"),
            ),
        }
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
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sub = match runtime.subscribe(&address).await {
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
    ws_tx: &mpsc::UnboundedSender<(String, WatchNotification)>,
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    target: WatchTarget,
) {
    let address = target.address.clone();
    let ws_task = spawn_ws_forwarder(address.clone(), ws_tx.clone(), Arc::clone(chain_runtime));
    runtimes.insert(
        address,
        TargetRuntime {
            target,
            ws_task,
            last_poll: Instant::now(),
            catch_up: None,
            baseline_only: false,
            overflow_streak: 0,
        },
    );
}

/// Rebuild the runtime target set from the database: the own wallet (always present,
/// never persisted) plus every enabled row in `watch_targets`. Simple full-rebuild
/// rather than a diff -- target management is a low-frequency, human-driven action
/// (`watch_max_targets` caps the whole set to single digits/low tens by default), so
/// a brief resubscribe-everything on change is not a real cost.
pub(super) async fn reload_targets(
    runtimes: &mut HashMap<String, TargetRuntime>,
    ws_tx: &mpsc::UnboundedSender<(String, WatchNotification)>,
    chain_runtime: &Arc<dyn WalletWatchRuntime>,
    watch_db: &WatchDatabase,
    own_subject: Subject,
) {
    for runtime in runtimes.values() {
        runtime.ws_task.abort();
    }
    runtimes.clear();

    // Runs after any in-flight poll has finished, so a cursor such a poll wrote for
    // a target that was just removed is swept here rather than left behind.
    if let Err(e) = watch_db.purge_orphan_cursors(&own_subject.address()).await {
        logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to purge orphan watch cursors: {e}"),
        );
    }

    // The own wallet is always watched, regardless of `wallet.watch_enabled` -- that
    // switch is the master control for TARGET watching (pasted addresses), not for
    // the own-wallet observation `TransactionsService` now structurally depends on.
    register(
        runtimes,
        ws_tx,
        chain_runtime,
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
        super::service_state::set_observed(runtimes.keys().cloned());
        return;
    }

    match watch_db.list_targets().await {
        Ok(targets) => {
            for target in targets.into_iter().filter(|t| t.enabled) {
                // Enabled again (by the user or a re-added source): any earlier
                // saturation verdict no longer describes it.
                super::service_state::clear_saturation(&target.address);
                register(runtimes, ws_tx, chain_runtime, target);
            }
        }
        Err(e) => logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to load watch targets: {e}"),
        ),
    }
    super::service_state::set_observed(runtimes.keys().cloned());
}
