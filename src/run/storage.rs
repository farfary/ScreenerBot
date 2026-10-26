// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The storage phase of a start: the staged trading-wallet replacement, the wallet data reset and the opening of every store, under the process lock before any service reads a store.

use tokio::task::JoinError;

use crate::chains::ChainId;
use crate::database::open::{reset_wallet_stores, WalletResetFailure};
use crate::errors::StartupError;
use crate::i18n::ids;
use crate::logger::{self, LogTag};

/// Run the storage phase on a blocking thread. A staged trading-wallet replacement is
/// applied first, before any wallet-scoped store has a pool; an apply that cannot finish
/// stops the start, so a half-moved state never boots. Then the one-shot wallet data
/// reset (the GUI's "Reset wallet data & restart" and `--clean-wallet-data`) moves the
/// wallet-scoped stores into a wallet-reset backup, refusing while the trading wallet
/// has open positions. Then every store opens, recovering as its class allows; a store
/// left unavailable is in the health registry and its owner gets the error.
pub(super) async fn run_storage_phase(reset_wallet_data: bool) -> Result<(), StartupError> {
    finish(tokio::task::spawn_blocking(move || open_stores(reset_wallet_data)).await)
}

fn open_stores(reset_wallet_data: bool) -> Result<(), StartupError> {
    crate::wallets::replacement::apply_pending()
        .map_err(StartupError::wallet_replacement_incomplete)?;
    if reset_wallet_data {
        reset_wallet_data_stores()?;
    }
    crate::database::open::open_all();
    Ok(())
}

/// The wallet data reset, refused while a trading wallet of the vault has open
/// positions. A reset that failed and moved its files back lets the start continue on
/// the stores as they were; one that could not move them back stops it.
fn reset_wallet_data_stores() -> Result<(), StartupError> {
    let mut trading_wallets = Vec::new();
    for chain in ChainId::ALL {
        let wallet = crate::wallets::replacement::trading_wallet_address(*chain)
            .map_err(|error| StartupError::failed(ids::STARTUP_FAILURE_WALLETS_INIT, error))?;
        trading_wallets.extend(wallet);
    }
    let directory = crate::database::backup::wallet_reset_directory("reset");
    match reset_wallet_stores(&directory, &trading_wallets) {
        Ok(_) => Ok(()),
        Err(WalletResetFailure::Refused(refusal)) => {
            Err(StartupError::wallet_reset_refused(refusal))
        }
        Err(WalletResetFailure::Restored(error)) => {
            logger::error(
                LogTag::System,
                &format!("Wallet data reset left the stores in place: {error}"),
            );
            Ok(())
        }
        Err(WalletResetFailure::Split {
            directory,
            error,
            restore_error,
        }) => Err(StartupError::wallet_reset_incomplete(
            &directory.display().to_string(),
            format!("{error}; moving the files back failed: {restore_error}"),
        )),
    }
}

/// The outcome of the storage phase's blocking task. A task that panicked stops the
/// start: a later store open would re-create a store at a path a resumed move still has
/// to clear, so continuing could cost the moved history.
fn finish(outcome: Result<Result<(), StartupError>, JoinError>) -> Result<(), StartupError> {
    match outcome {
        Ok(result) => result,
        Err(error) => Err(StartupError::failed(
            ids::STARTUP_FAILURE_STORAGE_PHASE,
            error,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::StartupErrorCode;

    #[tokio::test]
    async fn a_storage_phase_that_panicked_stops_the_start() {
        let outcome = tokio::task::spawn_blocking(|| -> Result<(), StartupError> {
            panic!("the apply panicked after a store moved")
        })
        .await;

        let error = finish(outcome).expect_err("a panic stops the start");

        assert_eq!(error.code, StartupErrorCode::Generic);
        assert_eq!(error.detail.id, ids::STARTUP_FAILURE_STORAGE_PHASE.as_str());
    }

    #[tokio::test]
    async fn a_storage_phase_result_passes_through() {
        let ok = tokio::task::spawn_blocking(|| -> Result<(), StartupError> { Ok(()) }).await;
        assert!(finish(ok).is_ok());

        let refused = tokio::task::spawn_blocking(|| {
            Err(StartupError::wallet_replacement_incomplete("disk is full"))
        })
        .await;
        assert_eq!(
            finish(refused)
                .expect_err("an apply failure stops the start")
                .code,
            StartupErrorCode::WalletReplacementIncomplete
        );
    }
}
