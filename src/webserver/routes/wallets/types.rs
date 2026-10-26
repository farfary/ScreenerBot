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

// =============================================================================
// QUERY PARAMS
// =============================================================================

#[derive(Deserialize)]
pub struct ListWalletsQuery {
    #[serde(default)]
    pub include_inactive: bool,
}
