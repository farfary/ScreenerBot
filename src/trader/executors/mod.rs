//! Trade execution system

mod buy;
mod sell;

pub use buy::{execute_buy, execute_buy_managed, execute_dca};
pub use sell::execute_sell;

use crate::logger::{self, LogTag};
use crate::trader::types::{TradeAction, TradeDecision, TradeResult};

/// Initialize the execution system
pub async fn init_execution_system() -> crate::trader::Result<()> {
    logger::info(LogTag::Trader, "Initializing execution system...");
    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}

/// Execute a trade decision
pub async fn execute_trade(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}
