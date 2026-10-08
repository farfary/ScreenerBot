// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! DeFiLlama API response types.

use serde::{Deserialize, Serialize};

// ============================================================================
// DEFILLAMA PROTOCOLS RESPONSE
// ============================================================================

/// DeFiLlama protocol from /protocols
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefiLlamaProtocol {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub symbol: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub chain: Option<String>, // Made optional - can be missing in some responses
    pub logo: Option<String>,
    #[serde(default)]
    pub chains: Option<Vec<String>>,
    pub category: Option<String>,
    #[serde(default)]
    pub tvl: Option<f64>,
}
