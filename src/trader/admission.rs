// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-independent entry admission.
//!
//! Everything that decides whether *any* new entry may happen right now, regardless of
//! what produced the signal. Extracted from `evaluators::entry` so a second entry source
//! runs the identical gauntlet instead of a copy of it.

use crate::database::catalog::{StoreSpec, COPY_TRADING, POSITIONS, STRATEGIES};
use crate::database::health::{self, StoreStatus};
use crate::trader::safety;

/// The stores every entry decision reads: open positions and their limits, copy tasks and
/// their budgets, and the strategies that produce signals.
pub const ENTRY_STORES: [&StoreSpec; 3] = [&POSITIONS, &COPY_TRADING, &STRATEGIES];

/// Why an entry may not proceed. Every variant is a value a caller can record, count and
/// render — the checks used to be log lines only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum EntryBlock {
    ForceStopped,
    LossLimit,
    Connectivity(String),
    /// An entry store is not open, or holds restored contents not yet reconciled with
    /// the wallet history. The detail names the store and its state.
    StorageUnavailable(String),
    PositionLimit,
    AlreadyOpen,
    ReentryCooldown,
    OpenCooldown {
        wait_secs: u64,
    },
    EntryReserved,
    Blacklisted,
    /// A check itself failed (DB/state error), not a rejection.
    CheckFailed(String),
}

/// Remaining global entry cooldown, kept separate from the strategy admission path so
/// existing strategy scheduling behavior does not change. Copy uses it to persist an
/// honest typed skip before reaching the string error in `open_position_impl`.
pub fn open_cooldown_wait_secs(
    last_open: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    cooldown_secs: u64,
) -> Option<u64> {
    let last_open = last_open?;
    let elapsed = now.signed_duration_since(last_open).num_seconds().max(0) as u64;
    (elapsed < cooldown_secs).then_some(cooldown_secs - elapsed)
}

/// The block for the first entry store in `statuses` that is not settled. A store the
/// storage phase has not reached (`None`) does not block: its owner opens it on first use
/// and reports its own error.
fn storage_block<'a>(
    statuses: impl IntoIterator<Item = Option<&'a StoreStatus>>,
) -> Option<EntryBlock> {
    statuses
        .into_iter()
        .flatten()
        .find(|status| !status.is_settled())
        .map(|status| {
            let state = status.health.as_str();
            EntryBlock::StorageUnavailable(if status.health.is_open() {
                format!("{}: {state}, not yet reconciled", status.store)
            } else {
                format!("{}: {state}", status.store)
            })
        })
}

/// Refuse an entry while any entry store is not settled.
pub fn check_entry_storage() -> Result<(), EntryBlock> {
    let statuses: Vec<Option<StoreStatus>> = ENTRY_STORES
        .iter()
        .flat_map(|spec| spec.paths())
        .map(|path| health::status(&path))
        .collect();
    storage_block(statuses.iter().map(Option::as_ref)).map_or(Ok(()), Err)
}

pub async fn check_open_cooldown() -> Result<(), EntryBlock> {
    let cooldown_secs = crate::config::with_config(|config| {
        config.positions.position_open_cooldown_secs.max(0) as u64
    });
    let last_open = *crate::positions::state::LAST_OPEN_TIME.read().await;
    match open_cooldown_wait_secs(last_open, chrono::Utc::now(), cooldown_secs) {
        Some(wait_secs) => Err(EntryBlock::OpenCooldown { wait_secs }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use std::path::PathBuf;

    use super::{open_cooldown_wait_secs, storage_block, EntryBlock};
    use crate::database::health::{StoreHealth, StoreStatus};
    use crate::database::recovery::RecoveryAction;
    use crate::errors::DatabaseError;

    fn store(store: &'static str, health: StoreHealth, reconciled: bool) -> StoreStatus {
        StoreStatus {
            store,
            path: PathBuf::from(format!("/admission-test/{store}.db")),
            health,
            changed_at: Utc::now(),
            reconciled,
        }
    }

    fn healthy(store_id: &'static str) -> StoreStatus {
        store(store_id, StoreHealth::Healthy, true)
    }

    #[test]
    fn every_store_state_that_is_not_open_blocks_entries() {
        let disk_full = DatabaseError::DiskFull {
            operation: "open".to_owned(),
            message: "full".to_owned(),
        };
        let closed = [
            StoreHealth::Rebuilding {
                since: Utc::now(),
                phase: "restore".to_owned(),
            },
            StoreHealth::unavailable(&disk_full, RecoveryAction::Unavailable),
            StoreHealth::unavailable(&disk_full, RecoveryAction::Isolate),
        ];
        for health in closed {
            let state = health.as_str();
            let copy = store("copy_trading", health, true);
            let positions = healthy("positions");
            let strategies = healthy("strategies");
            assert_eq!(
                storage_block([Some(&positions), Some(&copy), Some(&strategies)]),
                Some(EntryBlock::StorageUnavailable(format!(
                    "copy_trading: {state}"
                )))
            );
        }
    }

    #[test]
    fn replaced_contents_block_entries_until_reconciled() {
        let quarantine = PathBuf::from("/admission-test/quarantine");
        for health in [
            StoreHealth::Restored {
                snapshot_at: None,
                quarantine: quarantine.clone(),
            },
            StoreHealth::Recovered {
                quarantine: quarantine.clone(),
            },
        ] {
            let state = health.as_str();
            let waiting = store("positions", health.clone(), false);
            let strategies = healthy("strategies");
            assert_eq!(
                storage_block([Some(&waiting), Some(&strategies)]),
                Some(EntryBlock::StorageUnavailable(format!(
                    "positions: {state}, not yet reconciled"
                )))
            );
            let reconciled = store("positions", health, true);
            assert_eq!(storage_block([Some(&reconciled), Some(&strategies)]), None);
        }
    }

    #[test]
    fn settled_or_unopened_stores_never_block_entries() {
        let positions = healthy("positions");
        let copy = healthy("copy_trading");
        let strategies = healthy("strategies");
        assert_eq!(
            storage_block([Some(&positions), Some(&copy), Some(&strategies)]),
            None
        );
        assert_eq!(storage_block([None, None, None]), None);
    }

    #[test]
    fn open_cooldown_reports_the_boundary_without_rounding_it_away() {
        let now = Utc.timestamp_opt(100, 0).unwrap();
        assert_eq!(open_cooldown_wait_secs(None, now, 5), None);
        assert_eq!(open_cooldown_wait_secs(Some(now), now, 5), Some(5));
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(4)), now, 5),
            Some(1)
        );
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(5)), now, 5),
            None
        );
    }

    /// Catalog key of the label for each entry block. The match is exhaustive, so
    /// a new variant fails to compile until it is mapped here and in
    /// `ENTRY_BLOCK_LABELS` (pages/copy/format.js).
    fn block_label_key(block: &EntryBlock) -> &'static str {
        match block {
            EntryBlock::ForceStopped => "copy-entry-block-force-stopped",
            EntryBlock::LossLimit => "copy-entry-block-loss-limit",
            EntryBlock::Connectivity(_) => "copy-entry-block-connectivity",
            EntryBlock::StorageUnavailable(_) => "copy-entry-block-storage-unavailable",
            EntryBlock::PositionLimit => "copy-entry-block-position-limit",
            EntryBlock::AlreadyOpen => "copy-entry-block-already-open",
            EntryBlock::ReentryCooldown => "copy-entry-block-reentry-cooldown",
            EntryBlock::OpenCooldown { .. } => "copy-entry-block-open-cooldown",
            EntryBlock::EntryReserved => "copy-entry-block-entry-reserved",
            EntryBlock::Blacklisted => "copy-entry-block-blacklisted",
            EntryBlock::CheckFailed(_) => "copy-entry-block-check-failed",
        }
    }

    #[test]
    fn entry_block_labels_exist_in_the_catalog() {
        let blocks = [
            EntryBlock::ForceStopped,
            EntryBlock::LossLimit,
            EntryBlock::Connectivity(String::new()),
            EntryBlock::StorageUnavailable(String::new()),
            EntryBlock::PositionLimit,
            EntryBlock::AlreadyOpen,
            EntryBlock::ReentryCooldown,
            EntryBlock::OpenCooldown { wait_secs: 0 },
            EntryBlock::EntryReserved,
            EntryBlock::Blacklisted,
            EntryBlock::CheckFailed(String::new()),
        ];
        for block in &blocks {
            let value = serde_json::to_value(block).unwrap();
            let id = value["kind"].as_str().expect("block kind");
            let key = block_label_key(block);
            assert_eq!(
                key,
                format!("copy-entry-block-{}", id.replace('_', "-")),
                "key does not follow the serialized id {id}"
            );
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
        }
    }
}

/// Run the source-independent admission gauntlet for a potential entry.
///
/// Order: force stop -> loss limit -> entry storage -> connectivity -> position limits ->
/// existing position -> re-entry cooldown -> blacklist.
///
/// `required_endpoints` differs by source on purpose: strategy entries depend on filter
/// freshness and require rpc + dexscreener + rugcheck; a copy entry requires rpc only,
/// matching `execute_buy_managed`.
pub async fn check_entry_admission(
    mint: &str,
    required_endpoints: &[&str],
) -> Result<(), EntryBlock> {
    // Early exit: Force stop is active
    if crate::global::is_force_stopped() {
        crate::logger::debug(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by force_stop",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::ForceStopped);
    }

    // Early exit: Loss limit reached
    if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by loss_limit",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::LossLimit);
    }

    // Entry stores open and reconciled: a restored or recreated ledger would size and
    // count against a past state of the positions, copy budgets or strategies.
    if let Err(block) = check_entry_storage() {
        crate::logger::debug(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by storage: {block:?}",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(block);
    }

    // 1. Connectivity check - no required endpoint may be CONFIRMED down.
    //
    // Confirmed-down, not "not known to be up": an endpoint whose monitor is disabled in
    // config is never probed and stays `Unknown` forever, so the stricter check turned
    // `connectivity.endpoints.rugcheck.enabled = false` into a permanent, silent halt on
    // all automated buying. The executors still refuse to send a swap without a healthy
    // RPC, which is where fail-closed belongs.
    if let Some(unhealthy) = crate::connectivity::check_endpoints_usable(required_endpoints).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by unhealthy: {}",
                &mint[..8.min(mint.len())],
                unhealthy
            ),
        );
        return Err(EntryBlock::Connectivity(unhealthy));
    }

    // 2. Position limits - check if we can open more positions
    match safety::check_position_limits().await {
        Ok(true) => {}
        Ok(false) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by position_limits",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::PositionLimit);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }

    // 3. Existing position check - prevent duplicate entries
    match safety::has_open_position(mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by existing_position",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::AlreadyOpen);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }

    // 4. Re-entry cooldown - prevent immediate re-entry after exit
    match safety::is_in_reentry_cooldown(mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by reentry_cooldown",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::ReentryCooldown);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }

    // 5. Blacklist check - token-level only (not pool-level)
    if safety::is_blacklisted(mint).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by blacklist",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::Blacklisted);
    }

    Ok(())
}
