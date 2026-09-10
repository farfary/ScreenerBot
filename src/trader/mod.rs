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
//! - `constants`: All trader constants consolidated
//! - `config`: Configuration accessors
//! - `controller`: Start/stop trader control
//! - `service`: Service implementation
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
pub mod policy;
pub mod safety;
pub mod stats;
pub mod templates;
mod types;

// Re-exports for common usage
pub use constants::*;
pub use controller::{
    clear_force_stop, engage_force_stop, is_trader_running, loss_limit_snapshot, monitors_status,
    set_monitor_enabled, start_trader, start_trader_checked, stop_trader_checked,
    stop_trader_gracefully, trader_status, LossLimitSnapshot, Monitor, MonitorsStatus,
    TraderStatus,
};
pub use error::{Error, Result};
pub use executors::execute_trade;
pub use types::{
    FailedTradeStep, TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult, TradeStep,
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
