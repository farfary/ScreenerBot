// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The shared paper owner: the quote-based fill model, its provenance types,
//! the paper or live trading mode with its arming gate, and readiness evidence.
//!
//! Paper code never reaches swap execution, key material or a transaction
//! send: a source-scan guard (`tests/paper_isolation.rs`) keeps the live symbols
//! out of this module, and every paper fill runs inside `global::paper_scope`,
//! which those three seams refuse.

mod error;
pub mod executor;
pub mod fill;
mod mode;
pub mod readiness;
mod run;
mod types;

pub use error::{PaperFillError, PaperModeError};
pub use mode::{confirm_live, TradingMode, LIVE_ARM_CONFIRMATION, STRATEGY_LIVE_CONFIRMATION};
pub use run::{paper_account, reset_paper_run};
pub use types::{PaperFill, PaperModel, PaperQuoteMetadata, PaperSellFill};
