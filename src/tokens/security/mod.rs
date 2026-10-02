// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Security data fetching from multiple sources
//!
//! Each module handles one security analysis source:
//! - rugcheck: Rugcheck API (comprehensive security analysis)
//! - onchain: Future on-chain verification

pub mod rugcheck;
pub mod rugcheck_server;

pub use rugcheck::fetch_rugcheck_data;
