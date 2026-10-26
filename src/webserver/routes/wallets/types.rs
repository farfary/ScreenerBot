// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Type definitions for wallet API
//!
//! Response/request structs and query parameters for wallet endpoints.

use serde::{Deserialize, Serialize};

use crate::wallets::Wallet;

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Serialize)]
pub struct WalletListResponse {
    pub wallets: Vec<WalletListEntry>,
    pub total: usize,
}

/// One listed wallet with its SOL balance, which the server reads for every row so
/// the page only formats it. `None` when the balance could not be read.
#[derive(Serialize)]
pub struct WalletListEntry {
    #[serde(flatten)]
    pub wallet: Wallet,
    pub balance: Option<f64>,
}

/// A staged trading-wallet replacement; the process restarts to apply it.
#[derive(Serialize)]
pub struct WalletReplaceResponse {
    pub restart_required: bool,
    /// The instance the dashboard waits to see replaced by the restarted one.
    pub instance_id: String,
    pub previous_wallet: String,
    pub new_wallet: String,
    /// Where the restart moves the previous wallet's stores.
    pub backup_directory: String,
}

#[derive(Serialize)]
pub struct WalletBackupsResponse {
    pub backups: Vec<crate::wallets::replacement::WalletBackup>,
}

// =============================================================================
// QUERY PARAMS
// =============================================================================

#[derive(Deserialize)]
pub struct ListWalletsQuery {
    #[serde(default)]
    pub include_inactive: bool,
}
