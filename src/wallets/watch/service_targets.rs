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
        return;
    }

    match watch_db.list_targets().await {
        Ok(targets) => {
            for target in targets.into_iter().filter(|t| t.enabled) {
                register(runtimes, ws_tx, chain_runtime, target);
            }
        }
        Err(e) => logger::warning(
            LogTag::WalletWatch,
            &format!("Failed to load watch targets: {e}"),
        ),
    }
}
