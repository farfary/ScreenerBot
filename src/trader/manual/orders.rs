//! Manual order processing

use crate::logger::{self, LogTag};
use crate::trader::execution::execute_trade;
use crate::trader::types::{
    TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult,
};
use chrono::Utc;

/// Execute a manual buy order
pub async fn manual_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    logger::info(
        LogTag::Trader,
        &format!("Processing manual buy: mint={}, size={} SOL", mint, size_sol),
    );

    let decision = TradeDecision {
        position_id: None,
        mint: mint.to_string(),
        action: TradeAction::Buy,
        reason: TradeReason::ManualEntry,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High, // Manual orders have high priority
        price_sol: None,                // Will be determined during execution
        size_sol: Some(size_sol),
    };

    let result = execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}

/// Execute a manual sell order
pub async fn manual_sell(position_id: &str) -> Result<TradeResult, String> {
    logger::info(
        LogTag::Trader,
        &format!("Processing manual sell: position_id={}", position_id),
    );

    // TODO: Get position info when positions module is ready
    // For now, create a stub decision
    let decision = TradeDecision {
        position_id: Some(position_id.to_string()),
        mint: "STUB_MINT".to_string(), // Will be replaced with actual mint
        action: TradeAction::Sell,
        reason: TradeReason::ManualExit,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: None,
        size_sol: None,
    };

    let result = execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}

/// Execute a force buy (bypass safety checks)
pub async fn force_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    logger::warning(
        LogTag::Trader,
        &format!(
            "Processing FORCE buy (safety checks bypassed): mint={}, size={} SOL",
            mint, size_sol
        ),
    );

    let decision = TradeDecision {
        position_id: None,
        mint: mint.to_string(),
        action: TradeAction::Buy,
        reason: TradeReason::ForceBuy,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: None,
        size_sol: Some(size_sol),
    };

    let result = execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}

/// Execute a force sell (bypass safety checks)
pub async fn force_sell(position_id: &str) -> Result<TradeResult, String> {
    logger::warning(
        LogTag::Trader,
        &format!(
            "Processing FORCE sell (safety checks bypassed): position_id={}",
            position_id
        ),
    );

    // TODO: Get position info when positions module is ready
    // For now, create a stub decision
    let decision = TradeDecision {
        position_id: Some(position_id.to_string()),
        mint: "STUB_MINT".to_string(), // Will be replaced with actual mint
        action: TradeAction::Sell,
        reason: TradeReason::ForceSell,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::Emergency,
        price_sol: None,
        size_sol: None,
    };

    let result = execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}
