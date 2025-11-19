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

mod config;
mod constants;
mod controller;
pub mod evaluators;
pub mod executors;
pub mod manual;
pub mod monitors;
pub mod safety;
mod service;
mod types;

// Re-exports for common usage
pub use constants::*;
pub use controller::{is_trader_running, start_trader, stop_trader_gracefully, TraderControlError};
pub use executors::execute_trade;
pub use service::TraderService;
pub use types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};

use crate::logger::{self, LogTag};

/// Initialize the trader system
pub async fn init_trader_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing trader system...");

    // Initialize subsystems
    executors::init_execution_system().await?;
    safety::init_safety_system().await?;

    logger::info(LogTag::Trader, "Trader system initialized");
    Ok(())
}
