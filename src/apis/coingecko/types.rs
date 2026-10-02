// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! CoinGecko API response types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// COINGECKO COINS LIST RESPONSE
// ============================================================================

/// CoinGecko coin from /api/v3/coins/list
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoinGeckoCoin {
    pub id: String,
    pub symbol: String,
    pub name: String,
    #[serde(default)]
    pub platforms: Option<HashMap<String, String>>,
}
