//! Force trading operations (bypass safety checks)
//!
//! Emergency operations that bypass normal safety checks.
//! Use with caution - these operations ignore:
//! - Position limits
//! - Blacklist checks
//! - Cooldown periods
//! - Other safety constraints

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::executors;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use chrono::Utc;

/// Execute a force buy (bypass safety checks)
///
/// Creates a high-priority buy decision with ForceBuy reason.
/// **WARNING:** Bypasses all safety checks including position limits and blacklist.
/// Records the trade for tracking purposes.
pub async fn force_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    // Validate SOL amount (even for force operations)
    if !size_sol.is_finite() {
        return Err("Invalid SOL amount: must be finite".to_string());
    }
    if size_sol <= 0.0 {
        return Err(format!(
            "Invalid SOL amount: {}. Must be positive",
            size_sol
        ));
    }

    // Check against reasonable upper bound (100x default trade size)
    let default_trade_size = with_config(|cfg| cfg.trader.trade_size_sol);
    let max_trade_size = default_trade_size * 100.0;
    if size_sol > max_trade_size {
        return Err(format!(
            "SOL amount {:.4} exceeds maximum trade size of {:.4} SOL (100x default)",
            size_sol, max_trade_size
        ));
    }

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

    let result = executors::execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}

/// Execute a force sell (bypass safety checks)
///
/// Supports both full and partial exits via percentage parameter.
/// Creates an emergency-priority sell decision with ForceSell reason.
/// **WARNING:** Bypasses all safety checks.
///
/// # Parameters
/// - `mint`: Token mint address
/// - `percentage`: Exit percentage (None = 100% full exit, Some(50.0) = 50% partial)
///
/// # Returns
/// TradeResult with transaction details
pub async fn force_sell(mint: &str, percentage: Option<f64>) -> Result<TradeResult, String> {
    // Validate position exists
    let position = positions::get_position_by_mint(mint)
        .await
        .ok_or_else(|| format!("No open position for token: {}", mint))?;

    let exit_percentage = percentage.unwrap_or(100.0);

    // Validate percentage range (even for force operations)
    if !exit_percentage.is_finite() || exit_percentage <= 0.0 || exit_percentage > 100.0 {
        return Err(format!(
            "Invalid exit percentage: {}. Must be in range (0, 100]",
            exit_percentage
        ));
    }

    logger::warning(
        LogTag::Trader,
        &format!(
            "Processing FORCE sell (safety checks bypassed): mint={}, percentage={}%",
            mint, exit_percentage
        ),
    );

    let decision = TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: mint.to_string(),
        action: TradeAction::Sell,
        reason: TradeReason::ForceSell,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::Emergency,
        price_sol: None,
        size_sol: Some(exit_percentage), // Use size_sol for percentage
    };

    let result = executors::execute_trade(&decision).await?;

    // Record manual trade
    super::tracking::record_manual_trade(&result).await?;

    Ok(result)
}
