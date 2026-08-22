//! The wallet observation service.
//!
//! Watching a wallet is a feature of the wallet system, and a general one: observe
//! an arbitrary Solana address in real time, decode what it did, record it, and hand
//! it to whoever asked. The own wallet is watch target #1, not a special case --
//! everything a target gets (detection, decoding, recording) the own wallet gets too,
//! through the same funnel (PLAN.md §6).
//!
//! ```text
//! src/wallets/watch/
//!   mod.rs         # this file: public API, target CRUD, service start/stop
//!   types.rs       # WatchTarget, WatchSource, WalletActivity, ActivityKind, WatchStatus
//!   database.rs    # watch_targets + watch_cursors tables (wallets.db)
//!   runtime.rs     # WalletWatchRuntime: the injected chain execution seam
//!   service.rs     # the loop: WS stream + poll fallback + gap-fill + dispatch
//!   poller.rs      # signature-page cursor paging (the HTTP fallback), chain-neutral
//!   dedupe.rs      # durable per-subject seen-signature set
//!   recorder.rs    # per-subject persistence policy (§5.4)
//!
//! Everything that has to know a chain's wire format -- address parsing, the
//! realtime subscription transport, signature paging, transaction decode, and
//! decoded-transaction -> ActivityKind classification (§6.4) -- lives behind
//! `runtime::WalletWatchRuntime`, implemented for Solana by
//! `crate::chains::solana::wallets::runtime` and registered once by the
//! composition root (`crate::run::services`) before this service can start.
//! ```

mod database;
mod dedupe;
mod migrations;
mod poller;
mod recorder;
pub mod runtime;
mod service;
mod service_targets;
mod source_registry;
mod types;

pub use poller::{cadence_secs, needs_gap_fill, CatchUpState, CompletedCatchUp};
pub use service::subscribe_activity;
pub use types::{
    ActivityKind, SwapSide, TransferDirection, WalletActivity, WatchNotification, WatchSource,
    WatchStatus, WatchTarget,
};

use std::sync::{Arc, OnceLock};

use tokio::sync::Notify;

use crate::transactions::types::Subject;
use crate::{chains::active_chain, config::with_config};

use database::WatchDatabase;

/// The global watch database instance, set once by `start()`.
static GLOBAL_WATCH_DB: OnceLock<WatchDatabase> = OnceLock::new();

fn watch_db() -> Result<WatchDatabase, String> {
    GLOBAL_WATCH_DB
        .get()
        .cloned()
        .ok_or_else(|| "Watch database not initialized".to_owned())
}

/// Start the wallet observation service: open `watch_targets` / `watch_cursors`,
/// resolve the own wallet, and spawn the loop. Called by `WalletWatchService::start()`.
pub async fn start(shutdown: Arc<Notify>) -> Result<tokio::task::JoinHandle<()>, String> {
    let chain_runtime = runtime::get_runtime().map_err(|e| e.to_string())?;
    let own_subject =
        Subject::own().map_err(|e| format!("Cannot resolve own wallet for watch service: {e}"))?;

    let db = WatchDatabase::new(active_chain())?;
    let _ = GLOBAL_WATCH_DB.set(db.clone());

    Ok(tokio::spawn(service::run(
        shutdown,
        chain_runtime,
        db,
        own_subject,
    )))
}

/// `Healthy` when the shared subscription transport is connected or the service is
/// still inside its startup grace window; `Degraded` while running on polling alone.
pub fn is_healthy() -> bool {
    service::is_healthy()
}

// =============================================================================
// TARGET CRUD -- consumed by `webserver/routes/wallets/watch.rs`
// =============================================================================

/// Every watched target (alert-only in this phase). Does not include the own
/// wallet -- it is not a row in `watch_targets`, and is surfaced separately by
/// whatever UI wants to show it is always-on.
pub async fn list_targets() -> Result<Vec<WatchTarget>, String> {
    watch_db()?.list_targets().await
}

pub async fn get_target(id: i64) -> Result<Option<WatchTarget>, String> {
    watch_db()?.get_target(id).await
}

/// Resolve a watched target by its public address. Used by subject-scoped read APIs
/// to ensure arbitrary addresses cannot be queried merely by knowing the endpoint.
pub async fn get_target_by_address(address: &str) -> Result<Option<WatchTarget>, String> {
    watch_db()?.get_target_by_address(address).await
}

/// Add a new watch target. Every target added through this API is alert-only in
/// this phase (`WatchSource::Alert`) -- there is no copy-task source to choose
/// between yet. Rejects an address already in `wallets::list_wallets` (watching our
/// own wallet as a "target" is a different thing, handled implicitly by
/// `WatchSource::OwnWallet`) and an address already watched, and enforces
/// `wallet.watch_max_targets`.
pub async fn add_target(address: &str, label: Option<&str>) -> Result<WatchTarget, String> {
    if !with_config(|cfg| cfg.wallet.watch_enabled) {
        return Err(
            "Wallet watching is disabled (wallet.watch_enabled) -- enable it in Config first"
                .to_owned(),
        );
    }

    runtime::get_runtime()
        .map_err(|e| e.to_string())?
        .resolve_subject(address)
        .map_err(|e| format!("Invalid watch address: {e}"))?;

    let own_wallets = crate::wallets::list_wallets(true).await?;
    let db = watch_db()?;
    let current = db.list_targets().await?;
    let max_targets = with_config(|cfg| cfg.wallet.watch_max_targets);
    validate_target_constraints(
        address,
        &own_wallets
            .iter()
            .map(|wallet| wallet.address.clone())
            .collect::<Vec<_>>(),
        &current,
        max_targets,
    )?;

    let target = db.insert_alert_target(address, label).await?;
    service::request_reload();
    Ok(target)
}

fn validate_target_constraints(
    address: &str,
    own_addresses: &[String],
    current: &[WatchTarget],
    max_targets: usize,
) -> Result<(), String> {
    if own_addresses.iter().any(|own| own == address) {
        return Err(
            "This address is already one of your own wallets, not a target to watch".to_owned(),
        );
    }
    if current.iter().any(|target| target.address == address) {
        return Err(format!("{address} is already watched"));
    }
    if current.len() >= max_targets {
        return Err(format!(
            "Watch target limit reached ({max_targets}); remove one before adding another"
        ));
    }
    Ok(())
}

pub async fn remove_target(id: i64) -> Result<(), String> {
    let db = watch_db()?;
    let target = db
        .get_target(id)
        .await?
        .ok_or_else(|| format!("Watch target {id} not found"))?;
    db.remove_source(&target.address, WatchSource::Alert { rule_id: id })
        .await?;
    service::request_reload();
    Ok(())
}

/// Attach a paper-copy consumer to an address, sharing an existing alert target
/// when present. Copy tasks never create a second detection path.
pub async fn add_copy_source(
    task_id: i64,
    address: &str,
    label: Option<&str>,
) -> Result<WatchTarget, String> {
    runtime::get_runtime()
        .map_err(|e| e.to_string())?
        .resolve_subject(address)
        .map_err(|e| format!("Invalid watch address: {e}"))?;
    let own_wallets = crate::wallets::list_wallets(true).await?;
    if own_wallets.iter().any(|wallet| wallet.address == address) {
        return Err("This address is one of your own wallets and cannot be copied".to_owned());
    }
    let db = watch_db()?;
    let current = db.list_targets().await?;
    if !current.iter().any(|target| target.address == address) {
        let max_targets = with_config(|cfg| cfg.wallet.watch_max_targets);
        if current.len() >= max_targets {
            return Err(format!("Watch target limit reached ({max_targets})"));
        }
    }
    let target = db
        .upsert_source(address, label, WatchSource::Copy { task_id })
        .await?;
    service::request_reload();
    Ok(target)
}

pub async fn remove_copy_source(task_id: i64, address: &str) -> Result<(), String> {
    watch_db()?
        .remove_source(address, WatchSource::Copy { task_id })
        .await?;
    service::request_reload();
    Ok(())
}

pub async fn set_target_enabled(id: i64, enabled: bool) -> Result<(), String> {
    watch_db()?.set_enabled(id, enabled).await?;
    service::request_reload();
    Ok(())
}

/// Per-target status: whether the shared transport is currently connected, when its
/// cursor last advanced, and what it last advanced to.
pub async fn get_status(id: i64) -> Result<WatchStatus, String> {
    let db = watch_db()?;
    let target = db
        .get_target(id)
        .await?
        .ok_or_else(|| format!("Watch target {id} not found"))?;

    let last_signature = db.get_cursor(&target.address).await?;
    let last_activity_at = db.get_cursor_updated_at(&target.address).await?;
    let subscribed = runtime::try_get_runtime().is_some_and(|runtime| runtime.is_connected());

    Ok(WatchStatus {
        target,
        subscribed,
        last_activity_at,
        last_signature,
        last_error: None,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn target(address: &str) -> WatchTarget {
        WatchTarget {
            id: Some(1),
            address: address.to_owned(),
            label: None,
            sources: vec![WatchSource::Alert { rule_id: 1 }],
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn self_owned_and_duplicate_watch_targets_are_rejected() {
        assert!(validate_target_constraints("own", &["own".to_owned()], &[], 10).is_err());
        assert!(validate_target_constraints("watched", &[], &[target("watched")], 10).is_err());
    }

    #[test]
    fn watch_target_limit_is_enforced_before_insert() {
        assert!(validate_target_constraints("next", &[], &[target("existing")], 1).is_err());
        assert!(validate_target_constraints("next", &[], &[target("existing")], 2).is_ok());
    }
}
