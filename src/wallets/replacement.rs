// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Replacing the trading wallet: staged by a request, applied at the next start before any wallet-scoped store opens.
//!
//! The old key lives in process-wide caches (the signer, the main-wallet cache, the
//! position and balance state, the store pools) that cannot be reset safely while
//! services run, so a replacement never changes the running process. [`stage`] refuses
//! while a restart would interrupt something or the trading wallet has open positions,
//! switches the Auto Trader and Copy Trading off, adds the new key to the vault as an
//! other wallet and writes the marker `data/wallet_replacement.json`; the caller then
//! restarts the process. At the next start, under the process lock and before the storage
//! phase, [`apply_pending`] moves the wallet-scoped stores into a wallet-reset backup,
//! makes the new key the vault's main wallet, writes it to the config with both trading
//! switches off and removes the marker. Every apply step is idempotent, so an apply
//! interrupted at any point resumes at the next start; while the marker exists no start
//! skips it. A replacement is abandoned only before its store move begins; the outcome is
//! recorded for the Wallets page.
//!
//! The stores move before the vault changes its main wallet, so at every start the
//! trading key and the wallet-scoped stores belong to the same wallet and the wallet
//! consistency check sees a first run. Nothing is deleted: the old key stays in the
//! vault, active and exportable, and the stores are renamed into the backup without ever
//! replacing a file there.
//!
//! This module receives the new key only as ciphertext, nonce and address; it never
//! holds a decrypted key.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::chains::ChainId;
use crate::database::backup::{self, WalletResetManifest, WalletResetReason};
use crate::logger::{self, LogTag};
use crate::paths::{
    WALLET_REPLACEMENT_SET_ASIDE_PREFIX, WALLET_REPLACEMENT_SET_ASIDE_SUFFIX,
    WALLET_RESET_DIRECTORY_PREFIX,
};
use crate::process::restart_safety::RestartBlocker;

use super::database::WalletsDatabase;
use super::error::Error;
use super::manager::encrypted_key_address;
use super::types::{WalletRole, WalletType};

/// Characters of the previous address that label its backup directory.
const BACKUP_LABEL_LENGTH: usize = 8;

/// The new trading key, encrypted at the route that parsed it.
#[derive(Clone)]
pub struct StagedKey {
    /// The chain whose key format the route parsed the key in.
    pub chain: ChainId,
    /// The address the route derived from the key.
    pub address: String,
    pub ciphertext: String,
    pub nonce: String,
    /// The chain access validated with the key, saved with it.
    pub access: crate::chains::access::ChainAccessMode,
    /// The RPC endpoints validated with the key; empty keeps the configured ones.
    pub rpc_urls: Vec<String>,
}

impl std::fmt::Debug for StagedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagedKey")
            .field("chain", &self.chain)
            .field("access", &self.access)
            .field("address", &self.address)
            .field("ciphertext", &"<redacted>")
            .field("nonce", &"<redacted>")
            .field("rpc_urls", &self.rpc_urls.len())
            .finish()
    }
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

/// Why a start abandoned a staged replacement before moving anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum AbandonReason {
    /// The previous trading wallet had open positions.
    OpenPositions { count: usize },
    /// The positions store could not be read, so open positions could not be ruled out.
    PositionsUnreadable,
    /// The vault no longer held exactly one trading wallet equal to the previous one, or
    /// the new key no longer decrypted to the new wallet.
    VaultChanged,
}

impl std::fmt::Display for AbandonReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenPositions { count } => {
                write!(
                    f,
                    "the previous trading wallet has {count} open position(s)"
                )
            }
            Self::PositionsUnreadable => write!(f, "the positions store could not be read"),
            Self::VaultChanged => write!(f, "the vault no longer matches the replacement"),
        }
    }
}

/// A staged replacement a start abandoned, kept for the Wallets page until it is shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbandonedReplacement {
    pub previous_wallet: String,
    pub new_wallet: String,
    #[serde(flatten)]
    pub reason: AbandonReason,
    pub abandoned_at: DateTime<Utc>,
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
            .then(|| crate::paths::get_backup_path(&self.backup_directory))
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
    /// Open positions of the trading wallet on every chain in the positions store;
    /// `None` when it cannot be read.
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

/// The main wallet address of the vault of `chain`, if it holds one. Reads the vault
/// synchronously; async callers run it on a blocking thread.
pub fn trading_wallet_address(chain: ChainId) -> Result<Option<String>, Error> {
    let vault = WalletsDatabase::new(chain)?;
    Ok(vault.get_main_wallet()?.map(|wallet| wallet.address))
}

/// Run blocking vault, store or file work off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))?
}

/// Stage the replacement of the trading wallet by `key`. The process must restart
/// afterwards; the restart hold taken here is committed, so no trade or tool starts
/// before it. Every refusal leaves everything unchanged. A failure after a trading
/// switch was turned off leaves it off and says which switches are off.
pub async fn stage(key: StagedKey) -> Result<StagedReplacement, Error> {
    let chain = key.chain;
    let (ciphertext, nonce) = (key.ciphertext.clone(), key.nonce.clone());
    let address = blocking(move || encrypted_key_address(chain, &ciphertext, &nonce)).await?;
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
        stop_trading(chain, key.access, &key.rpc_urls).await?;
    }

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
    let new_address = key.address.clone();
    blocking(move || {
        let interrupted = |step: &'static str| {
            move |error: Error| Error::ReplacementInterrupted {
                step,
                copy_trading_off: true,
                detail: error.to_string(),
            }
        };
        let vault = WalletsDatabase::new(chain).map_err(interrupted("open the vault"))?;
        add_new_key(&vault, &key).map_err(interrupted("add the new key to the vault"))?;
        write_marker(&crate::paths::get_wallet_replacement_marker_path(), &marker).map_err(
            |error| match error {
                Error::ReplacementPending => error,
                other => interrupted("write the replacement marker")(other),
            },
        )
    })
    .await?;
    hold.commit();

    logger::info(
        LogTag::Wallet,
        &format!(
            "Trading wallet replacement staged: {previous_address} -> {new_address}; it applies at the restart"
        ),
    );
    Ok(StagedReplacement {
        previous_wallet: previous_address,
        new_wallet: new_address,
        backup_directory: backup_directory.display().to_string(),
    })
}

/// Read every input of the refusal checks.
async fn read_stage_checks(chain: ChainId, new_address: &str) -> Result<StageChecks, Error> {
    let blocker = crate::process::restart_safety::restart_blocker().await;
    let (trading_wallet, stored_open_positions, pending) = blocking(move || {
        let trading_wallet = trading_wallet_address(chain)?;
        let stored_open_positions = match &trading_wallet {
            Some(wallet) => crate::positions::database::count_open_positions_in_file(
                &crate::paths::get_positions_db_path(),
                wallet,
            )
            .ok(),
            None => Some(0),
        };
        let pending = crate::paths::get_wallet_replacement_marker_path().exists();
        Ok((trading_wallet, stored_open_positions, pending))
    })
    .await?;
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
        pending,
    })
}

/// Switch the Auto Trader and Copy Trading off and keep the validated RPC endpoints,
/// when any were given.
async fn stop_trading(
    chain: ChainId,
    access: crate::chains::access::ChainAccessMode,
    rpc_urls: &[String],
) -> Result<(), Error> {
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
            copy_trading_off: false,
            detail: error.to_string(),
        }
    })?;
    // Nothing is written when the replacement keeps both the mode and the endpoints.
    let access_changes = crate::config::with_config(|config| {
        config.chains.access(chain) != access.as_config_value()
    });
    if rpc_urls.is_empty() && !access_changes {
        return Ok(());
    }
    let urls = rpc_urls.to_vec();
    blocking(move || {
        crate::config::update_config_section(
            move |config| {
                *config.chains.access_mut(chain) = access.as_config_value().to_owned();
                if !urls.is_empty() {
                    config.chains.rpc_mut(chain).urls = urls;
                }
            },
            true,
        )
        .map_err(|error| Error::ReplacementInterrupted {
            step: "save the chain access settings",
            copy_trading_off: true,
            detail: error.to_string(),
        })
    })
    .await?;
    Ok(())
}

/// Add the new key to the vault as an other wallet with no name of its own, or keep the
/// row the address already has (a wallet replaced earlier, or a retried request). A kept
/// row whose key no longer decrypts to its address takes the validated key instead. Every
/// key change snapshots the vault.
fn add_new_key(vault: &WalletsDatabase, key: &StagedKey) -> Result<(), Error> {
    if let Some((ciphertext, nonce)) = vault.get_wallet_key_by_address(&key.address)? {
        let usable = encrypted_key_address(vault.chain(), &ciphertext, &nonce)
            .is_ok_and(|address| address == key.address);
        if !usable {
            vault.replace_wallet_key(&key.address, &key.ciphertext, &key.nonce)?;
            logger::warning(
                LogTag::Wallet,
                &format!(
                    "The vault's key of {} did not decrypt to it and was replaced by the validated key",
                    key.address
                ),
            );
        }
        return Ok(());
    }
    vault.insert_wallet(
        "",
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

/// The replacement a start abandoned and the Wallets page has not shown yet. A record
/// that cannot be read is logged and reads as none.
pub fn abandoned_replacement() -> Option<AbandonedReplacement> {
    let path = crate::paths::get_wallet_replacement_outcome_path();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            logger::warning(
                LogTag::Wallet,
                &format!("Could not read {}: {error}", path.display()),
            );
            return None;
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(outcome) => Some(outcome),
        Err(error) => {
            logger::warning(
                LogTag::Wallet,
                &format!("Could not decode {}: {error}", path.display()),
            );
            None
        }
    }
}

/// Remove the record of an abandoned replacement once the Wallets page showed it.
pub fn dismiss_abandoned_replacement() -> Result<(), Error> {
    crate::utils::atomic_file::remove(&crate::paths::get_wallet_replacement_outcome_path()).map_err(
        |error| Error::ReplacementOutcome {
            message: error.to_string(),
        },
    )
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
/// marker, an unreadable marker (set aside), a stale marker (removed, with the reason
/// logged) or a replacement abandoned before its store move began (removed, logged and
/// recorded for the Wallets page). An error leaves the marker in place, so the next start
/// resumes.
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
            // The backup directory is created right before the first file moves. Once
            // it exists the replacement only resumes: an abandonment would open stores
            // missing what already moved, and the stage counted no open position for the
            // hold it committed.
            let move_begun = backup_directory.exists();
            if !move_begun {
                if let Some(reason) = abandon_reason(&vault, &marker) {
                    remove_marker(&path)?;
                    record_abandoned(&marker, reason.clone());
                    logger::warning(
                        LogTag::Wallet,
                        &format!(
                            "Wallet replacement {} -> {} abandoned because {reason}; the trading wallet is unchanged and {} stays under Other Wallets",
                            marker.previous_address, marker.new_address, marker.new_address
                        ),
                    );
                    return Ok(None);
                }
            } else if let Some(detail) = vault_mismatch(&vault, &marker) {
                return Err(Error::ReplacementVaultChanged { detail });
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
    let target = path.with_file_name(format!(
        "{WALLET_REPLACEMENT_SET_ASIDE_PREFIX}{stamp}{WALLET_REPLACEMENT_SET_ASIDE_SUFFIX}"
    ));
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

/// Why the replacement must be abandoned before its store move begins, if it must: the
/// vault no longer matches it, or the previous trading wallet has open positions on any
/// chain, or they cannot be ruled out.
fn abandon_reason(vault: &WalletsDatabase, marker: &Marker) -> Option<AbandonReason> {
    if let Some(detail) = vault_mismatch(vault, marker) {
        logger::warning(
            LogTag::Wallet,
            &format!("The vault does not match the staged wallet replacement: {detail}"),
        );
        return Some(AbandonReason::VaultChanged);
    }
    match crate::positions::database::count_open_positions_in_file(
        &crate::paths::get_positions_db_path(),
        &marker.previous_address,
    ) {
        Ok(0) => None,
        Ok(count) => Some(AbandonReason::OpenPositions { count }),
        Err(_) => Some(AbandonReason::PositionsUnreadable),
    }
}

/// What keeps the vault from taking the replacement, if anything: it must hold exactly
/// one active main wallet, the previous one, and the new wallet's key must decrypt to
/// the new address, so the promotion after the store move cannot fail.
fn vault_mismatch(vault: &WalletsDatabase, marker: &Marker) -> Option<String> {
    match vault.active_main_addresses() {
        Ok(mains) if mains == [marker.previous_address.as_str()] => {}
        Ok(mains) => return Some(format!("the active main wallets are {mains:?}")),
        Err(error) => return Some(format!("the main wallets could not be read: {error}")),
    }
    match vault.get_wallet_key_by_address(&marker.new_address) {
        Ok(Some((ciphertext, nonce))) => {
            match encrypted_key_address(vault.chain(), &ciphertext, &nonce) {
                Ok(address) if address == marker.new_address => None,
                Ok(address) => Some(format!(
                    "the key of {} decrypts to {address}",
                    marker.new_address
                )),
                Err(error) => Some(format!(
                    "the key of {} cannot be read: {error}",
                    marker.new_address
                )),
            }
        }
        Ok(None) => Some(format!("{} is not in the vault", marker.new_address)),
        Err(error) => Some(format!(
            "the key of {} could not be read: {error}",
            marker.new_address
        )),
    }
}

/// Record the abandoned replacement for the Wallets page. A failed write is logged; the
/// log line of the abandonment stays the record.
fn record_abandoned(marker: &Marker, reason: AbandonReason) {
    let outcome = AbandonedReplacement {
        previous_wallet: marker.previous_address.clone(),
        new_wallet: marker.new_address.clone(),
        reason,
        abandoned_at: Utc::now(),
    };
    let path = crate::paths::get_wallet_replacement_outcome_path();
    let written = serde_json::to_vec_pretty(&outcome)
        .map_err(|error| error.to_string())
        .and_then(|encoded| {
            crate::utils::atomic_file::replace(&path, &encoded, 0o600)
                .map_err(|error| error.to_string())
        });
    if let Err(error) = written {
        logger::warning(
            LogTag::Wallet,
            &format!(
                "Could not record the abandoned wallet replacement in {}: {error}",
                path.display()
            ),
        );
    }
}

/// Move every wallet-scoped store into the marker's backup directory with its manifest.
fn move_stores(marker: &Marker, directory: &Path) -> Result<(), Error> {
    let manifest = backup::move_wallet_stores(
        directory,
        WalletResetManifest {
            reason: WalletResetReason::WalletReplaced,
            chain: marker.chain.clone(),
            previous_wallet: Some(marker.previous_address.clone()),
            new_wallet: Some(marker.new_address.clone()),
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            created_at_ms: Utc::now().timestamp_millis(),
            files: Vec::new(),
            moved_open_positions: Default::default(),
        },
    )?;
    if !manifest.moved_open_positions.is_empty() {
        logger::info(
            LogTag::Wallet,
            &format!(
                "The moved positions store holds open positions by wallet: {:?}",
                manifest.moved_open_positions
            ),
        );
    }
    Ok(())
}

/// Write the new main wallet's vault key to the config with the Auto Trader and Copy
/// Trading off, when a config file exists. The key is the vault's own ciphertext, so the
/// config and the vault name the same wallet; it is never cleared, because an empty key
/// starts Explore Mode and the config key is what rebuilds a lost vault. Only these four
/// keys are written, so a load-time config migration still waits for startup to complete.
fn write_trading_key(vault: &WalletsDatabase, new_address: &str) -> Result<(), Error> {
    if !crate::paths::get_config_path().exists() {
        return Ok(());
    }
    let (ciphertext, nonce) = vault
        .get_wallet_key_by_address(new_address)?
        .ok_or_else(|| Error::WalletNotFound {
            address: new_address.to_owned(),
        })?;
    crate::config::set_config_keys(&[
        ("wallet_encrypted", toml::Value::String(ciphertext)),
        ("wallet_nonce", toml::Value::String(nonce)),
        ("trader.enabled", toml::Value::Boolean(false)),
        ("copy_trading.enabled", toml::Value::Boolean(false)),
    ])
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
