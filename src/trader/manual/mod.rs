// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Manual trading operations.

mod api;
mod detach;
mod force;
pub mod guard;
mod paper;
mod tracking;

pub use api::{manual_add, manual_buy, manual_sell};
pub(in crate::trader) use detach::detached;
pub use force::force_sell;
pub use paper::manual_paper_close;
pub use tracking::{get_manual_trade_history, record_manual_trade};
