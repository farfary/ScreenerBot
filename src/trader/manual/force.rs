//! Force trading operations (bypass safety checks)
//!
//! Emergency operations that bypass normal safety checks.
//! All force operations are tracked through the actions system for dashboard visibility.
//! Use with caution - these operations ignore:
//! - Position limits
//! - Blacklist checks
//! - Cooldown periods
//! - Other safety constraints

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::actions::{ManualBuyAction, ManualSellAction};
use crate::trader::executors;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use chrono::Utc;

/// Execute a force buy (bypass safety checks)
///
/// Creates a high-priority buy decision with ForceBuy reason.
/// **WARNING:** Bypasses all safety checks including position limits and blacklist.
/// Action progress is broadcast to dashboard via SSE.
pub async fn force_buy(mint: &str, size_sol: f64) -> Result<TradeResult, String> {
    // Get token symbol for action display
    let symbol = crate::tokens::get_full_token_async(mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    // Create action tracker
    let action = ManualBuyAction::new(mint, symbol.as_deref(), size_sol).await?;

    // Step 1: Validation
    action.start_validation().await;

    // Validate SOL amount (even for force operations)
    if !size_sol.is_finite() {
        let error = "Invalid SOL amount: must be finite";
        action.fail_validation(error).await;
        return Err(error.to_string());
    }
    if size_sol <= 0.0 {
        let error = format!("Invalid SOL amount: {}. Must be positive", size_sol);
        action.fail_validation(&error).await;
        return Err(error);
    }

    // Check against reasonable upper bound
    use crate::trader::constants::MAX_TRADE_SIZE_MULTIPLIER;
    let default_trade_size = with_config(|cfg| cfg.trader.trade_size_sol);
    let max_trade_size = default_trade_size * MAX_TRADE_SIZE_MULTIPLIER;
    if size_sol > max_trade_size {
        let error = format!(
            "SOL amount {:.4} exceeds maximum trade size of {:.4} SOL ({}x default)",
            size_sol, max_trade_size, MAX_TRADE_SIZE_MULTIPLIER as u32
        );
        action.fail_validation(&error).await;
        return Err(error);
    }

    action.complete_validation().await;

    logger::warning(
        LogTag::Trader,
        &format!(
            "Processing FORCE buy (safety checks bypassed): mint={}, size={} SOL",
            mint, size_sol
        ),
    );

    // Step 2: Quote
    action.start_quote().await;

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

    // Execute trade (includes quote + swap)
    let result = match executors::execute_trade(&decision).await {
        Ok(result) => result,
        Err(e) => {
            if e.contains("Quote") || e.contains("quote") {
                action.fail_quote(&e).await;
            } else {
                action.fail(&e).await;
            }
            return Err(e);
        }
    };

    // Check if trade succeeded
    if !result.success {
        let error = result.error.as_deref().unwrap_or("Trade failed");
        if error.contains("Unhealthy") || error.contains("connectivity") {
            action.fail_validation(error).await;
        } else if error.contains("Quote") || error.contains("quote") || error.contains("No routes")
        {
            action.fail_quote(error).await;
        } else {
            action.fail_swap(error).await;
        }
        return Ok(result);
    }

    // Mark quote and swap as complete
    action.complete_quote(None).await;
    action.start_swap().await;

    if let Some(ref sig) = result.tx_signature {
        action.complete_swap(sig).await;
        action.skip_verify_async(sig).await;
    } else {
        action.complete_swap("unknown").await;
        action.skip_verify_async("unknown").await;
    }

    // Record manual trade
    if let Err(e) = super::tracking::record_manual_trade(&result).await {
        logger::warning(
            LogTag::Trader,
            &format!("Failed to record manual trade: {}", e),
        );
    }

    Ok(result)
}

/// Execute a force sell (bypass safety checks)
///
/// Supports both full and partial exits via percentage parameter.
/// Creates an emergency-priority sell decision with ForceSell reason.
/// **WARNING:** Bypasses all safety checks.
/// Action progress is broadcast to dashboard via SSE.
///
/// # Parameters
/// - `mint`: Token mint address
/// - `percentage`: Exit percentage (None = 100% full exit, Some(50.0) = 50% partial)
///
/// # Returns
/// TradeResult with transaction details
pub async fn force_sell(mint: &str, percentage: Option<f64>) -> Result<TradeResult, String> {
    let exit_percentage = percentage.unwrap_or(100.0);

    // Get token symbol and position for action display
    let symbol = crate::tokens::get_full_token_async(mint)
        .await
        .ok()
        .flatten()
        .map(|t| t.symbol);

    // Validate position exists first (needed for action metadata)
    let position = positions::get_position_by_mint(mint).await;
    let position_id = position.as_ref().and_then(|p| p.id);

    // Create action tracker
    let action =
        ManualSellAction::new(mint, symbol.as_deref(), exit_percentage, position_id).await?;

    // Step 1: Validation
    action.start_validation().await;

    // Validate position exists
    let position = match position {
        Some(p) => p,
        None => {
            let error = format!("No open position for token: {}", mint);
            action.fail_validation(&error).await;
            return Err(error);
        }
    };

    // Validate percentage range (even for force operations)
    if !exit_percentage.is_finite() || exit_percentage <= 0.0 || exit_percentage > 100.0 {
        let error = format!(
            "Invalid exit percentage: {}. Must be in range (0, 100]",
            exit_percentage
        );
        action.fail_validation(&error).await;
        return Err(error);
    }

    action.complete_validation().await;

    logger::warning(
        LogTag::Trader,
        &format!(
            "Processing FORCE sell (safety checks bypassed): mint={}, percentage={}%",
            mint, exit_percentage
        ),
    );

    // Step 2: Quote
    action.start_quote().await;

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

    // Execute trade (includes quote + swap)
    let result = match executors::execute_trade(&decision).await {
        Ok(result) => result,
        Err(e) => {
            if e.contains("Quote") || e.contains("quote") {
                action.fail_quote(&e).await;
            } else {
                action.fail(&e).await;
            }
            return Err(e);
        }
    };

    // Check if trade succeeded
    if !result.success {
        let error = result.error.as_deref().unwrap_or("Trade failed");
        if error.contains("Unhealthy") || error.contains("connectivity") {
            action.fail_validation(error).await;
        } else if error.contains("Quote") || error.contains("quote") || error.contains("No routes")
        {
            action.fail_quote(error).await;
        } else {
            action.fail_swap(error).await;
        }
        return Ok(result);
    }

    // Mark quote and swap as complete
    action.complete_quote(None).await;
    action.start_swap().await;

    if let Some(ref sig) = result.tx_signature {
        action.complete_swap(sig, result.executed_size_sol).await;
        action.skip_verify_async(sig).await;
    } else {
        action.complete_swap("unknown", None).await;
        action.skip_verify_async("unknown").await;
    }

    // Record manual trade
    if let Err(e) = super::tracking::record_manual_trade(&result).await {
        logger::warning(
            LogTag::Trader,
            &format!("Failed to record manual trade: {}", e),
        );
    }

    Ok(result)
}
