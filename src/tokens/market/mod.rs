// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Market data fetching from multiple sources
//!
//! Each module handles one API source:
//! - dexscreener: DexScreener API (30s updates, batch endpoint)
//! - geckoterminal: GeckoTerminal API (on-demand refreshes only, batch endpoint)

pub mod dexscreener;
pub mod geckoterminal;
mod persist;
#[cfg(test)]
mod persist_tests;

pub use dexscreener::{fetch_dexscreener_data, fetch_dexscreener_data_batch};
pub use geckoterminal::fetch_geckoterminal_data;
