// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trader module - Core trading functionality orchestration
//!
//! ## Architecture
//!
//! ```text
//! Monitor → Safety → Evaluator → Executor → Result
//! ```
//!
//! **Monitors:** Orchestration loops (entry/exit monitoring)  
//! **Safety:** Guards (limits, blacklist, cooldown, risk)  
//! **Evaluators:** Business logic (strategies, exit conditions, DCA)  
//! **Executors:** Trade execution (buy/sell/DCA, retry)
//!
//! ## Module Structure
//!
//! - `monitors/`: Entry and exit monitoring loops (orchestration only)
//! - `evaluators/`: Entry/exit evaluation logic, DCA, strategies
//! - `executors/`: Trade execution, retry mechanism, decision cache
//! - `safety/`: Safety checks (limits, blacklist, cooldown, risk)
//! - `manual/`: Manual trading API (normal + force operations)
//! - `paper/`: Shared paper fill model, trading mode and readiness evidence
//! - `constants`: All trader constants consolidated
//! - `config`: Configuration accessors
//! - `controller`: Start/stop trader control
//! - `status`: Auto Trader status and the entry state of each book
//! - `service`: Service implementation
//! - `trade_reason`: Trade and close reasons and their display text
//! - `types`: Trader types

pub mod actions;
pub mod admission;
pub mod config;
mod constants;
mod controller;
pub mod copy;
pub mod entry;
mod error;
pub mod evaluators;
pub mod executors;
pub mod llm_analysis;
pub mod manual;
pub mod monitors;
pub mod paper;
pub mod policy;
pub mod safety;
pub mod stats;
mod status;
pub mod templates;
mod trade_reason;
mod types;

// Re-exports for common usage
pub use constants::*;
pub use controller::{
    clear_force_stop, engage_force_stop, is_trader_running, live_block, loss_limit_snapshot,
    monitors_status, run_state, set_monitor_enabled, start_trader_checked, stop_trader_checked,
    stop_trader_gracefully, LiveBlock, LossLimitSnapshot, Monitor, MonitorState, MonitorsStatus,
    TraderRunState,
};
pub use error::{Error, Result};
pub use executors::execute_trade;
pub use status::{entry_idle, trader_status, EntryIdle, TraderStatus};
pub use trade_reason::{
    closed_reason_group_text, closed_reason_key, closed_reason_text, trade_reason_text,
    TradeReason, FORCE_CLOSED_KEY, UNKNOWN_REASON_KEY,
};
pub use types::{
    FailedTradeStep, TradeAction, TradeDecision, TradePriority, TradeResult, TradeStep, Withdrawal,
};

use crate::logger::{self, LogTag};

/// Initialize the trader system
pub async fn init_trader_system() -> Result<()> {
    logger::info(LogTag::Trader, "Initializing trader system...");

    // Initialize subsystems
    executors::init_execution_system().await?;
    safety::init_safety_system().await?;

    logger::info(LogTag::Trader, "Trader system initialized");
    Ok(())
}
