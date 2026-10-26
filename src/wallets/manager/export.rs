// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet key export — the one wallet action the dashboard offers on a listed wallet.

use super::super::error::Error;
use super::super::types::ExportWalletResponse;
use crate::logger::{self, LogTag};

/// Export a wallet's private key
pub async fn export_wallet(wallet_id: i64) -> Result<ExportWalletResponse, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

    let wallet = db.get_wallet(wallet_id)?.ok_or(Error::WalletNotFound {
        address: format!("id={wallet_id}"),
    })?;

    let (encrypted, nonce) =
        db.get_wallet_encrypted_key(wallet_id)?
            .ok_or(Error::WalletNotFound {
                address: format!("id={wallet_id}"),
            })?;

    let private_key = crate::chains::solana::accounts::export_private_key(&encrypted, &nonce)
        .map_err(|reason| Error::InvalidPrivateKey {
            reason: reason.to_string(),
        })?;

    logger::warning(
        LogTag::Wallet,
        &format!("Wallet exported: {} ({})", wallet.name, wallet.address),
    );

    Ok(ExportWalletResponse {
        address: wallet.address,
        private_key,
        warning: "NEVER share this private key. Anyone with access can steal your funds."
            .to_string(),
    })
}
