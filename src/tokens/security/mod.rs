// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Security data fetching from multiple sources
//!
//! Each module handles one security analysis source:
//! - rugcheck: Rugcheck API (comprehensive security analysis)
//! - risk_band: the low / medium / high bands of the normalised score

pub mod risk_band;
pub mod rugcheck;
pub mod rugcheck_server;

pub use risk_band::SecurityRiskBand;
pub use rugcheck::fetch_rugcheck_data;
