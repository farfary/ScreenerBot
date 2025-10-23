//! Trader module - Core trading functionality orchestration
//!
//! The trader module is responsible for:
//! 1. Automated trading via strategies
//! 2. Manual trading operations
//! 3. Position management and exit strategies
//! 4. Trade execution and retry mechanisms

mod service;
mod types;
mod config;
mod controller;
pub mod auto;
pub mod manual;
pub mod execution;
pub mod safety;
pub mod exit;

// Re-exports for common usage
pub use controller::{
    is_trader_running, start_trader, stop_trader_gracefully, TraderControlError,
};
pub use service::TraderService;
pub use types::{
    TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult,
};

// Constants for webserver/external modules
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;
pub const DEBUG_FORCE_SELL_MODE: bool = false;
pub const DEBUG_FORCE_BUY_MODE: bool = false;

use crate::logger::{log, LogTag};

/// Initialize the trader system
pub async fn init_trader_system() -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Initializing trader system...");

    // Initialize subsystems
    execution::init_execution_system().await?;
    safety::init_safety_system().await?;

    log(LogTag::Trader, "INFO", "Trader system initialized");
    Ok(())
}
