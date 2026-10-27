// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token security data: the Rugcheck fetchers and the normalised risk bands built from them.

pub mod risk_band;
pub mod rugcheck;
pub mod rugcheck_server;

pub use risk_band::SecurityRiskBand;
pub use rugcheck::fetch_rugcheck_data;
