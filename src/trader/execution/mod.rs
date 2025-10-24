//! Trade execution system

mod buy;
mod decision_cache;
mod retry;
mod sell;

pub use buy::{execute_buy, execute_dca};
pub use decision_cache::{
    cache_sell_decision, get_pending_sell_decisions, mark_sell_complete,
};
pub use retry::retry_trade;
pub use sell::execute_sell;

use crate::logger::{self, LogTag};
use crate::trader::types::{TradeAction, TradeDecision, TradeResult};

/// Initialize the execution system
pub async fn init_execution_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing execution system...");

    // Initialize decision cache
    decision_cache::init_cache()?;

    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}

/// Execute a trade decision
pub async fn execute_trade(decision: &TradeDecision) -> Result<TradeResult, String> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}
