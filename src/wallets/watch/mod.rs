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
mod service_state;
mod service_targets;
mod source_registry;
mod types;

pub use poller::{cadence_secs, needs_gap_fill, CatchUpState, CompletedCatchUp, PAGE_SIZE};
pub use service::subscribe_activity;
pub use types::{
    ActivityKind, SignaturePageItem, SuccessfulTransactionPageItem, SuccessfulTransactionsPage,
    SwapSide, TransferDirection, WalletActivity, WatchDisableReason, WatchMode, WatchNotification,
    WatchSource, WatchStatus, WatchTarget,
};

use std::sync::{Arc, OnceLock};

use tokio::sync::Notify;

use crate::errors::InternalError;
use crate::transactions::types::Subject;
use crate::wallets::Error;
use crate::{chains::active_chain, config::with_config};

use database::WatchDatabase;

/// The global watch database instance, set once by `start()`.
static GLOBAL_WATCH_DB: OnceLock<WatchDatabase> = OnceLock::new();
static TARGET_CHANGE_CHANNEL: std::sync::LazyLock<tokio::sync::broadcast::Sender<String>> =
    std::sync::LazyLock::new(|| tokio::sync::broadcast::channel(32).0);

pub fn subscribe_target_changes() -> tokio::sync::broadcast::Receiver<String> {
    TARGET_CHANGE_CHANNEL.subscribe()
}

pub(super) fn publish_target_change(address: String) {
    let _ = TARGET_CHANGE_CHANNEL.send(address);
}

fn watch_db() -> Result<WatchDatabase, Error> {
    GLOBAL_WATCH_DB.get().cloned().ok_or_else(|| {
        Error::Internal(InternalError::InvariantViolation {
            message: "watch database not initialized".to_owned(),
        })
    })
}

/// Start the wallet observation service: open `watch_targets` / `watch_cursors`,
/// resolve the own wallet, and spawn the loop. Called by `WalletWatchService::start()`.
pub async fn start(shutdown: Arc<Notify>) -> Result<tokio::task::JoinHandle<()>, Error> {
    let chain_runtime = runtime::get_runtime().map_err(|e| Error::ChainRuntime {
        operation: "get_runtime",
        detail: e.to_string(),
    })?;
    let own_subject = Subject::own().map_err(|e| Error::ChainRuntime {
        operation: "resolve_own_subject",
        detail: e.to_string(),
    })?;

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
pub async fn list_targets() -> Result<Vec<WatchTarget>, Error> {
    watch_db()?.list_targets().await
}

pub async fn get_target(id: i64) -> Result<Option<WatchTarget>, Error> {
    watch_db()?.get_target(id).await
}

/// Resolve a watched target by its public address. Used by subject-scoped read APIs
/// to ensure arbitrary addresses cannot be queried merely by knowing the endpoint.
pub async fn get_target_by_address(address: &str) -> Result<Option<WatchTarget>, Error> {
    watch_db()?.get_target_by_address(address).await
}

/// Add a new watch target. Every target added through this API is alert-only in
/// this phase (`WatchSource::Alert`) -- there is no copy-task source to choose
/// between yet. Rejects an address already in `wallets::list_wallets` (watching our
/// own wallet as a "target" is a different thing, handled implicitly by
/// `WatchSource::OwnWallet`) and an address already watched, and enforces
/// `wallet.watch_max_targets`.
pub async fn add_target(address: &str, label: Option<&str>) -> Result<WatchTarget, Error> {
    if !with_config(|cfg| cfg.wallet.watch_enabled) {
        return Err(Error::WatchDisabled);
    }

    runtime::get_runtime()
        .map_err(|e| Error::ChainRuntime {
            operation: "get_runtime",
            detail: e.to_string(),
        })?
        .resolve_subject(address)
        .map_err(|e| Error::InvalidWatchAddress {
            value: format!("{address}: {e}"),
        })?;

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
) -> Result<(), Error> {
    if own_addresses.iter().any(|own| own == address) {
        return Err(Error::WatchTargetIsOwnWallet {
            address: address.to_owned(),
        });
    }
    if current.iter().any(|target| target.address == address) {
        return Err(Error::WatchTargetAlreadyWatched {
            address: address.to_owned(),
        });
    }
    if current.len() >= max_targets {
        return Err(Error::WatchTargetLimitReached { max: max_targets });
    }
    Ok(())
}

pub async fn remove_target(id: i64) -> Result<(), Error> {
    let db = watch_db()?;
    let target = db.get_target(id).await?.ok_or(Error::WatchTargetNotFound {
        address: format!("id={id}"),
    })?;
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
) -> Result<WatchTarget, Error> {
    runtime::get_runtime()
        .map_err(|e| Error::ChainRuntime {
            operation: "get_runtime",
            detail: e.to_string(),
        })?
        .resolve_subject(address)
        .map_err(|e| Error::InvalidWatchAddress {
            value: format!("{address}: {e}"),
        })?;
    let own_wallets = crate::wallets::list_wallets(true).await?;
    if own_wallets.iter().any(|wallet| wallet.address == address) {
        return Err(Error::WatchTargetIsOwnWallet {
            address: address.to_owned(),
        });
    }
    let db = watch_db()?;
    let current = db.list_targets().await?;
    if !current.iter().any(|target| target.address == address) {
        let max_targets = with_config(|cfg| cfg.wallet.watch_max_targets);
        if current.len() >= max_targets {
            return Err(Error::WatchTargetLimitReached { max: max_targets });
        }
    }
    let target = db
        .upsert_source(address, label, WatchSource::Copy { task_id })
        .await?;
    service::request_reload();
    Ok(target)
}

pub async fn remove_copy_source(task_id: i64, address: &str) -> Result<(), Error> {
    watch_db()?
        .remove_source(address, WatchSource::Copy { task_id })
        .await?;
    service::request_reload();
    Ok(())
}

pub async fn set_target_enabled(id: i64, enabled: bool) -> Result<(), Error> {
    let db = watch_db()?;
    if enabled {
        let target = db.get_target(id).await?.ok_or(Error::WatchTargetNotFound {
            address: format!("id={id}"),
        })?;
        if target.high_activity_approved
            && matches!(
                target.disable_reason,
                Some(WatchDisableReason::HeliusUnavailable)
            )
        {
            let runtime = runtime::get_runtime().map_err(|error| Error::ChainRuntime {
                operation: "get_runtime",
                detail: error.to_string(),
            })?;
            validate_helius_retry(&target, runtime.supports_high_activity_mode().await)?;
        }
    }
    db.set_enabled(id, enabled).await?;
    service::request_reload();
    Ok(())
}

fn validate_helius_retry(target: &WatchTarget, high_activity_supported: bool) -> Result<(), Error> {
    if target.high_activity_approved
        && matches!(
            target.disable_reason,
            Some(WatchDisableReason::HeliusUnavailable)
        )
        && !high_activity_supported
    {
        return Err(Error::WatchHeliusUnavailable {
            address: target.address.clone(),
        });
    }
    Ok(())
}

pub const MAX_PAGE_BUDGET: usize = poller::MAX_PAGE_BUDGET;
pub const DEFAULT_PAGE_BUDGET: usize = poller::DEFAULT_PAGE_BUDGET;

pub async fn resume_target(
    id: i64,
    page_budget: usize,
    acknowledge_missed_activity: bool,
) -> Result<(), Error> {
    if !acknowledge_missed_activity {
        return Err(Error::WatchResumeAcknowledgementRequired);
    }
    validate_page_budget(page_budget)?;
    let target = get_target(id).await?.ok_or(Error::WatchTargetNotFound {
        address: format!("id={id}"),
    })?;
    if target.enabled
        || !matches!(
            target.disable_reason,
            Some(WatchDisableReason::SignatureBudget { .. })
        )
    {
        return Err(Error::WatchResumeNotBudgetPaused { id });
    }
    let runtime = runtime::get_runtime().map_err(|e| Error::ChainRuntime {
        operation: "get_runtime",
        detail: e.to_string(),
    })?;
    let head = runtime
        .fetch_signatures_page(&target.address, 1, None, None)
        .await?;
    watch_db()?
        .resume_target_from_head(
            id,
            page_budget,
            head.first().map(|item| item.signature.clone()),
        )
        .await?;
    service::request_reload();
    Ok(())
}

pub async fn update_target_page_budget(id: i64, page_budget: usize) -> Result<(), Error> {
    validate_page_budget(page_budget)?;
    watch_db()?.set_page_budget(id, page_budget).await?;
    service::request_reload();
    Ok(())
}

/// Persist or revoke per-wallet approval for the Helius high-activity fallback.
/// Approval validates provider capability and resumes a budget-paused watch from
/// its saved cursor rather than rebasing at the current head.
pub async fn set_target_high_activity_approved(
    id: i64,
    approved: bool,
    acknowledge_provider_usage: bool,
) -> Result<(), Error> {
    let target = get_target(id).await?.ok_or(Error::WatchTargetNotFound {
        address: format!("id={id}"),
    })?;
    if approved {
        if !acknowledge_provider_usage {
            return Err(Error::WatchHeliusApprovalAcknowledgementRequired);
        }
        let runtime = runtime::get_runtime().map_err(|error| Error::ChainRuntime {
            operation: "get_runtime",
            detail: error.to_string(),
        })?;
        if !runtime.supports_high_activity_mode().await {
            return Err(Error::WatchHeliusUnavailable {
                address: target.address,
            });
        }
    }
    watch_db()?.set_high_activity_approved(id, approved).await?;
    service::request_reload();
    Ok(())
}

fn validate_page_budget(page_budget: usize) -> Result<(), Error> {
    if !(DEFAULT_PAGE_BUDGET..=MAX_PAGE_BUDGET).contains(&page_budget) {
        return Err(Error::InvalidWatchBudget {
            requested: page_budget,
            min: DEFAULT_PAGE_BUDGET,
            max: MAX_PAGE_BUDGET,
        });
    }
    Ok(())
}

pub async fn copy_source_disable_reason(
    task_id: i64,
    address: &str,
) -> Result<Option<WatchDisableReason>, Error> {
    Ok(watch_db()?
        .get_target_by_address(address)
        .await?
        .filter(|target| target.sources.contains(&WatchSource::Copy { task_id }))
        .and_then(|target| target.disable_reason))
}

/// Per-target status: whether the shared transport is currently connected, when its
/// cursor last advanced, and what it last advanced to.
pub async fn get_status(id: i64) -> Result<WatchStatus, Error> {
    let db = watch_db()?;
    let target = db.get_target(id).await?.ok_or(Error::WatchTargetNotFound {
        address: format!("id={id}"),
    })?;

    let last_signature = db.get_cursor(&target.address).await?;
    let last_activity_at = db.get_cursor_updated_at(&target.address).await?;
    let subscribed = runtime::try_get_runtime().is_some_and(|runtime| runtime.is_connected())
        && service_state::subscription_active(&target.address);
    let last_error = target
        .disable_reason
        .as_ref()
        .map(WatchDisableReason::summary)
        .or_else(|| service_state::runtime_error(&target.address));
    let mode = service_state::watch_mode(&target.address);
    let catching_up = service_state::catching_up(&target.address);
    let last_checked_at = service_state::last_checked_at(&target.address);

    Ok(WatchStatus {
        target,
        subscribed,
        last_activity_at,
        last_signature,
        last_error,
        mode,
        catching_up,
        last_checked_at,
    })
}

/// Whether `address` is persisted as an enabled watch target carrying this copy
/// task as a source. Persisted state, not the loop's runtime set, so a reload in
/// flight is never mistaken for a missing target.
pub async fn copy_source_active(task_id: i64, address: &str) -> Result<bool, Error> {
    Ok(watch_db()?
        .get_target_by_address(address)
        .await?
        .is_some_and(|target| {
            target.enabled && target.sources.contains(&WatchSource::Copy { task_id })
        }))
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
            page_budget: DEFAULT_PAGE_BUDGET,
            high_activity_approved: false,
            disable_reason: None,
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

    #[test]
    fn helius_retry_requires_a_configured_high_activity_provider() {
        let mut paused = target("helius");
        paused.enabled = false;
        paused.high_activity_approved = true;
        paused.disable_reason = Some(WatchDisableReason::HeliusUnavailable);

        assert!(matches!(
            validate_helius_retry(&paused, false),
            Err(Error::WatchHeliusUnavailable { .. })
        ));
        assert!(validate_helius_retry(&paused, true).is_ok());
        paused.high_activity_approved = false;
        assert!(validate_helius_retry(&paused, false).is_ok());
    }
}
