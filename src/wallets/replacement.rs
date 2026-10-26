// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Replacing the trading wallet: staged by a request, applied at the next start before any wallet-scoped store opens.
//!
//! The old key lives in process-wide caches (the signer, the main-wallet cache, the
//! position and balance state, the store pools) that cannot be reset safely while
//! services run, so a replacement never changes the running process. [`stage`] refuses
//! while a restart would interrupt something or positions are open, switches the Auto
//! Trader and Copy Trading off, adds the new key to the vault as an other wallet and
//! writes the marker `data/wallet_replacement.json`; the caller then restarts the
//! process. At the next start, under the process lock and before the storage phase,
//! [`apply_pending`] moves the wallet-scoped stores into a wallet-reset backup, makes the
//! new key the vault's main wallet, writes it to the config with both trading switches
//! off and removes the marker. Every apply step is idempotent, so an apply interrupted
//! at any point resumes at the next start; while the marker exists no start skips it.
//!
//! The stores move before the vault changes its main wallet, so at every start the
//! trading key and the wallet-scoped stores belong to the same wallet and the wallet
//! consistency check sees a first run. Nothing is deleted: the old key stays in the
//! vault, active and exportable, and the stores are renamed into the backup.
//!
//! This module receives the new key only as ciphertext, nonce and address; it never
//! holds a decrypted key.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::chains::{active_chain, ChainId};
use crate::database::backup::{self, WalletResetManifest, WalletResetReason};
use crate::logger::{self, LogTag};
use crate::paths::WALLET_RESET_DIRECTORY_PREFIX;
use crate::version::RestartBlocker;

use super::database::WalletsDatabase;
use super::error::Error;
use super::manager::encrypted_key_address;
use super::types::{WalletRole, WalletType};

/// Name the new key carries in the vault.
const MAIN_WALLET_NAME: &str = "Main Wallet";

/// Characters of the previous address that label its backup directory.
const BACKUP_LABEL_LENGTH: usize = 8;

/// The new trading key, encrypted at the route that parsed it.
#[derive(Debug, Clone)]
pub struct StagedKey {
    /// The address the route derived from the key.
    pub address: String,
    pub ciphertext: String,
    pub nonce: String,
    /// The RPC endpoints validated with the key; kept when not empty.
    pub rpc_urls: Vec<String>,
}

/// A replacement written to the marker, applied at the next start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StagedReplacement {
    pub previous_wallet: String,
    pub new_wallet: String,
    /// Where the next start moves the old wallet's stores.
    pub backup_directory: String,
}

/// A replacement applied at this start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedReplacement {
    pub previous_wallet: String,
    pub new_wallet: String,
    pub backup_directory: PathBuf,
}

/// One wallet-reset backup directory and what its manifest records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalletBackup {
    pub directory: String,
    pub reason: WalletResetReason,
    pub previous_wallet: Option<String>,
    pub new_wallet: Option<String>,
    pub created_at_ms: i64,
    pub files: Vec<String>,
}

/// The pending replacement, as `data/wallet_replacement.json` records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Marker {
    pub chain: String,
    pub previous_address: String,
    pub new_address: String,
    /// Name of the wallet-reset directory inside the backups directory.
    pub backup_directory: String,
    pub requested_at: DateTime<Utc>,
}

impl Marker {
    /// The backup directory the marker names, or `None` when the name is not a single
    /// wallet-reset directory name, so a marker can never move a store outside
    /// `backups/`.
    fn backup_path(&self) -> Option<PathBuf> {
        is_backup_directory_name(&self.backup_directory)
            .then(|| crate::paths::get_backups_directory().join(&self.backup_directory))
    }
}

/// Whether `name` is one path component naming a wallet-reset directory.
pub(crate) fn is_backup_directory_name(name: &str) -> bool {
    Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name)
        && name.starts_with(WALLET_RESET_DIRECTORY_PREFIX)
}

// =============================================================================
// REQUEST TIME
// =============================================================================

/// What the refusal checks of a staged replacement read, in the order they are checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageChecks {
    /// Why a restart would interrupt something now, if anything.
    pub blocker: Option<RestartBlocker>,
    /// Open positions of any wallet in the positions store; `None` when it cannot be read.
    pub stored_open_positions: Option<usize>,
    /// The vault's main wallet.
    pub trading_wallet: Option<String>,
    pub new_address: String,
    /// Whether the new address is a watched or copied wallet.
    pub watched: bool,
    /// Whether a replacement marker already exists.
    pub pending: bool,
}

/// The first refusal of a replacement, or the trading wallet it replaces.
pub(crate) fn check_stage(checks: &StageChecks) -> Result<String, Error> {
    if let Some(reason) = checks.blocker {
        return Err(Error::ReplacementBlocked { reason });
    }
    match checks.stored_open_positions {
        None => {
            return Err(Error::ReplacementBlocked {
                reason: RestartBlocker::PositionsUnreadable,
            })
        }
        Some(0) => {}
        Some(count) => {
            return Err(Error::ReplacementBlocked {
                reason: RestartBlocker::OpenPositions { count },
            })
        }
    }
    let Some(trading_wallet) = checks.trading_wallet.clone() else {
        return Err(Error::ReplacementNoTradingWallet);
    };
    if trading_wallet == checks.new_address {
        return Err(Error::ReplacementSameWallet {
            address: checks.new_address.clone(),
        });
    }
    if checks.watched {
        return Err(Error::ReplacementWalletIsWatched {
            address: checks.new_address.clone(),
        });
    }
    if checks.pending {
        return Err(Error::ReplacementPending);
    }
    Ok(trading_wallet)
}

/// The main wallet address of the vault of this process's chain, if it holds one.
pub async fn trading_wallet_address() -> Result<Option<String>, Error> {
    let vault = WalletsDatabase::new(active_chain())?;
    Ok(vault.get_main_wallet()?.map(|wallet| wallet.address))
}

/// Stage the replacement of the trading wallet by `key`. The process must restart
/// afterwards; the restart hold taken here is committed, so no trade or tool starts
/// before it. Every refusal leaves everything unchanged. A failure after the trading
/// switches were turned off leaves them off and says so.
pub async fn stage(key: StagedKey) -> Result<StagedReplacement, Error> {
    let chain = active_chain();
    let address = encrypted_key_address(&key.ciphertext, &key.nonce)?;
    if address != key.address {
        return Err(Error::ReplacementAddressMismatch {
            expected: key.address,
            actual: address,
        });
    }

    let Some(hold) = crate::global::try_hold_activity_for_restart() else {
        return Err(Error::ReplacementRestartInProgress);
    };

    let previous_address = check_stage(&read_stage_checks(chain, &key.address).await?)?;

    if crate::paths::get_config_path().exists() {
        stop_trading(chain, &key.rpc_urls).await?;
    }

    let interrupted = |step: &'static str| {
        move |error: Error| Error::ReplacementInterrupted {
            step,
            detail: error.to_string(),
        }
    };
    let vault = WalletsDatabase::new(chain).map_err(interrupted("open the vault"))?;
    add_new_key(&vault, &key).map_err(interrupted("add the new key to the vault"))?;

    let backup_directory = backup::wallet_reset_directory(&backup_label(&previous_address));
    let marker = Marker {
        chain: chain.as_str().to_owned(),
        previous_address: previous_address.clone(),
        new_address: key.address.clone(),
        backup_directory: backup_directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        requested_at: Utc::now(),
    };
    write_marker(&crate::paths::get_wallet_replacement_marker_path(), &marker)?;
    hold.commit();

    logger::info(
        LogTag::Wallet,
        &format!(
            "Trading wallet replacement staged: {previous_address} -> {}; it applies at the restart",
            key.address
        ),
    );
    Ok(StagedReplacement {
        previous_wallet: previous_address,
        new_wallet: key.address,
        backup_directory: backup_directory.display().to_string(),
    })
}

/// Read every input of the refusal checks.
async fn read_stage_checks(chain: ChainId, new_address: &str) -> Result<StageChecks, Error> {
    let blocker = crate::version::restart_blocker().await;
    let positions = crate::paths::get_positions_db_path();
    let stored_open_positions = tokio::task::spawn_blocking(move || {
        crate::positions::database::count_open_positions_in_file(&positions, chain).ok()
    })
    .await
    .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))?;
    let trading_wallet = WalletsDatabase::new(chain)?
        .get_main_wallet()?
        .map(|wallet| wallet.address);
    let watched = super::watch::WatchDatabase::new(chain)?
        .get_target_by_address(new_address)
        .await?
        .is_some();
    Ok(StageChecks {
        blocker,
        stored_open_positions,
        trading_wallet,
        new_address: new_address.to_owned(),
        watched,
        pending: crate::paths::get_wallet_replacement_marker_path().exists(),
    })
}

/// Switch the Auto Trader and Copy Trading off and keep the validated RPC endpoints.
async fn stop_trading(chain: ChainId, rpc_urls: &[String]) -> Result<(), Error> {
    match crate::trader::stop_trader_gracefully().await {
        Ok(()) | Err(crate::trader::Error::AlreadyStopped) => {}
        Err(error) => {
            return Err(Error::Dependency {
                dependency: "trader",
                detail: error.to_string(),
            })
        }
    }
    crate::trader::copy::control::stop_copy_trading().map_err(|error| {
        Error::ReplacementInterrupted {
            step: "switch Copy Trading off",
            detail: error.to_string(),
        }
    })?;
    if !rpc_urls.is_empty() {
        let urls = rpc_urls.to_vec();
        crate::config::update_config_section(
            move |config| config.chains.rpc_mut(chain).urls = urls,
            true,
        )
        .map_err(|error| Error::ReplacementInterrupted {
            step: "save the RPC endpoints",
            detail: error.to_string(),
        })?;
    }
    Ok(())
}

/// Add the new key to the vault as an other wallet, or keep the row it already has (a
/// wallet replaced earlier, or a retried request). Adding a key snapshots the vault.
fn add_new_key(vault: &WalletsDatabase, key: &StagedKey) -> Result<(), Error> {
    if vault.get_wallet_by_address(&key.address)?.is_some() {
        return Ok(());
    }
    vault.insert_wallet(
        MAIN_WALLET_NAME,
        &key.address,
        &key.ciphertext,
        &key.nonce,
        WalletRole::Secondary,
        WalletType::Imported,
        None,
    )?;
    Ok(())
}

/// The label of a backup directory: the first characters of the previous address.
fn backup_label(previous_address: &str) -> String {
    previous_address.chars().take(BACKUP_LABEL_LENGTH).collect()
}

/// Write `marker` to `path`, create-only and owner read/write only. An existing marker
/// is a replacement already staged.
pub(crate) fn write_marker(path: &Path, marker: &Marker) -> Result<(), Error> {
    let encoded = serde_json::to_vec_pretty(marker).map_err(|error| Error::ReplacementMarker {
        operation: "encode",
        message: error.to_string(),
    })?;
    crate::utils::atomic_file::write_new(path, &encoded, 0o600).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::ReplacementPending
        } else {
            Error::ReplacementMarker {
                operation: "write",
                message: error.to_string(),
            }
        }
    })
}

/// Every wallet-reset backup, newest first.
pub fn list_backups() -> Vec<WalletBackup> {
    backup::wallet_reset_backups()
        .into_iter()
        .map(|(directory, manifest)| WalletBackup {
            directory: directory.display().to_string(),
            reason: manifest.reason,
            previous_wallet: manifest.previous_wallet,
            new_wallet: manifest.new_wallet,
            created_at_ms: manifest.created_at_ms,
            files: manifest.files,
        })
        .collect()
}

// =============================================================================
// BOOT TIME
// =============================================================================

/// Where an interrupted or fresh apply stands, read from the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyState {
    /// The vault still names the previous wallet and holds the new key: apply from the
    /// open-position check.
    NotFlipped,
    /// The vault already names the new wallet: finish the config and the marker.
    Flipped,
    /// The vault matches neither; the marker no longer describes this vault.
    Stale,
}

/// Classify a marker against the vault's main wallet and whether the new key is in it.
pub(crate) fn classify(
    marker: &Marker,
    vault_main: Option<&str>,
    new_key_in_vault: bool,
) -> ApplyState {
    match vault_main {
        Some(main) if main == marker.new_address => ApplyState::Flipped,
        Some(main) if main == marker.previous_address && new_key_in_vault => ApplyState::NotFlipped,
        _ => ApplyState::Stale,
    }
}

/// Apply a staged replacement, if one is pending. Runs once per start under the process
/// lock, before any wallet-scoped store opens. `Ok(None)` when nothing was applied: no
/// marker, an unreadable marker (set aside), a stale marker or an abandoned replacement
/// (each removed, with the reason logged). An error leaves the marker in place, so the
/// next start resumes.
pub fn apply_pending() -> Result<Option<AppliedReplacement>, Error> {
    let path = crate::paths::get_wallet_replacement_marker_path();
    let Some(marker) = read_marker(&path)? else {
        return Ok(None);
    };
    let (Ok(chain), Some(backup_directory)) =
        (marker.chain.parse::<ChainId>(), marker.backup_path())
    else {
        set_aside(
            &path,
            "it names an unknown chain or a backup directory outside backups/",
        );
        return Ok(None);
    };

    let vault = WalletsDatabase::new(chain)?;
    let vault_main = vault.get_main_wallet()?.map(|wallet| wallet.address);
    let new_key_in_vault = vault.get_wallet_by_address(&marker.new_address)?.is_some();
    match classify(&marker, vault_main.as_deref(), new_key_in_vault) {
        ApplyState::Stale => {
            remove_marker(&path)?;
            logger::warning(
                LogTag::Wallet,
                &format!(
                    "Dropped a stale wallet replacement {} -> {}: the vault's trading wallet is {}",
                    marker.previous_address,
                    marker.new_address,
                    vault_main.as_deref().unwrap_or("not set")
                ),
            );
            return Ok(None);
        }
        ApplyState::NotFlipped => {
            if let Some(reason) = open_positions_blocker(chain) {
                remove_marker(&path)?;
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Wallet replacement {} -> {} abandoned because {reason}; the trading wallet is unchanged and {} stays under Other Wallets",
                        marker.previous_address, marker.new_address, marker.new_address
                    ),
                );
                return Ok(None);
            }
            move_stores(&marker, &backup_directory)?;
            vault.promote_to_main(&marker.previous_address, &marker.new_address)?;
        }
        ApplyState::Flipped => {}
    }

    write_trading_key(&vault, &marker.new_address)?;
    remove_marker(&path)?;
    logger::info(
        LogTag::Wallet,
        &format!(
            "Trading wallet replaced: {} -> {}; the previous wallet's stores are in {}",
            marker.previous_address,
            marker.new_address,
            backup_directory.display()
        ),
    );
    Ok(Some(AppliedReplacement {
        previous_wallet: marker.previous_address,
        new_wallet: marker.new_address,
        backup_directory,
    }))
}

/// The marker at `path`, `None` when there is none. A marker that does not parse is
/// set aside, so it never blocks a start, and `None` is returned.
fn read_marker(path: &Path) -> Result<Option<Marker>, Error> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::ReplacementMarker {
                operation: "read",
                message: error.to_string(),
            })
        }
    };
    match serde_json::from_slice::<Marker>(&bytes) {
        Ok(marker) => Ok(Some(marker)),
        Err(error) => {
            set_aside(path, &error.to_string());
            Ok(None)
        }
    }
}

/// Move an unusable marker to `wallet_replacement.invalid-<stamp>.json` beside it.
fn set_aside(path: &Path, reason: &str) {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let target = path.with_file_name(format!("wallet_replacement.invalid-{stamp}.json"));
    match std::fs::rename(path, &target) {
        Ok(()) => logger::error(
            LogTag::Wallet,
            &format!(
                "The wallet replacement marker is unusable ({reason}) and was moved to {}; the trading wallet is unchanged",
                target.display()
            ),
        ),
        Err(error) => logger::error(
            LogTag::Wallet,
            &format!(
                "The wallet replacement marker is unusable ({reason}) and could not be moved aside: {error}"
            ),
        ),
    }
}

/// Why the positions store forbids moving the stores, if it does: an open position of
/// any wallet on `chain`, or a store that cannot be read.
fn open_positions_blocker(chain: ChainId) -> Option<RestartBlocker> {
    match crate::positions::database::count_open_positions_in_file(
        &crate::paths::get_positions_db_path(),
        chain,
    ) {
        Ok(0) => None,
        Ok(count) => Some(RestartBlocker::OpenPositions { count }),
        Err(_) => Some(RestartBlocker::PositionsUnreadable),
    }
}

/// Move every wallet-scoped store into the marker's backup directory with its manifest.
fn move_stores(marker: &Marker, directory: &Path) -> Result<(), Error> {
    backup::move_wallet_stores(
        directory,
        WalletResetManifest {
            reason: WalletResetReason::WalletReplaced,
            chain: marker.chain.clone(),
            previous_wallet: Some(marker.previous_address.clone()),
            new_wallet: Some(marker.new_address.clone()),
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            created_at_ms: Utc::now().timestamp_millis(),
            files: Vec::new(),
        },
    )?;
    Ok(())
}

/// Write the new main wallet's vault key to the config with the Auto Trader and Copy
/// Trading off, when a config file exists. The key is the vault's own ciphertext, so the
/// config and the vault name the same wallet; it is never cleared, because an empty key
/// starts Explore Mode and the config key is what rebuilds a lost vault.
fn write_trading_key(vault: &WalletsDatabase, new_address: &str) -> Result<(), Error> {
    if !crate::paths::get_config_path().exists() {
        return Ok(());
    }
    let wallet =
        vault
            .get_wallet_by_address(new_address)?
            .ok_or_else(|| Error::WalletNotFound {
                address: new_address.to_owned(),
            })?;
    let (ciphertext, nonce) =
        vault
            .get_wallet_encrypted_key(wallet.id)?
            .ok_or_else(|| Error::WalletNotFound {
                address: new_address.to_owned(),
            })?;
    crate::config::update_config_section(
        move |config| {
            config.wallet_encrypted = ciphertext;
            config.wallet_nonce = nonce;
            config.trader.enabled = false;
            config.copy_trading.enabled = false;
        },
        true,
    )
    .map_err(|error| Error::Dependency {
        dependency: "config",
        detail: error.to_string(),
    })
}

/// Remove the marker and persist the directory entry.
fn remove_marker(path: &Path) -> Result<(), Error> {
    crate::utils::atomic_file::remove(path).map_err(|error| Error::ReplacementMarker {
        operation: "remove",
        message: error.to_string(),
    })
}

#[cfg(test)]
#[path = "replacement_tests.rs"]
mod tests;
