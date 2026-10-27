// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Market data fetching: one module per provider, each storing its market rows through `persist`.

pub mod dexscreener;
pub mod geckoterminal;
mod persist;
#[cfg(test)]
mod persist_tests;
