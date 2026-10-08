// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Force sell: an emergency exit of an open position that bypasses the normal sell
//! safety checks, tracked through the actions system for dashboard visibility.

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::actions::ManualSellAction;
use crate::trader::error::Error;
use crate::trader::executors;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};

use super::detach::detached;
use chrono::Utc;

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
///
/// Runs detached from the caller: dropping the returned future never
/// cancels a trade whose swap may already be sent.
pub async fn force_sell(
    mint: &str,
    percentage: Option<f64>,
    slippage_pct: Option<f64>,
) -> Result<TradeResult, Error> {
    let mint = mint.to_owned();
    detached(async move { sell_forced(&mint, percentage, slippage_pct).await }).await
}

async fn sell_forced(
    mint: &str,
    percentage: Option<f64>,
    slippage_pct: Option<f64>,
) -> Result<TradeResult, Error> {
    let exit_percentage = percentage.unwrap_or(100.0);

    // Get token symbol and position for action display
    let symbol = match crate::chains::chain_for_address(mint) {
        Ok(chain) => crate::tokens::get_full_token_async(chain, mint)
            .await
            .ok()
            .flatten(),
        Err(_) => None,
    }
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
            let error = format!("No open position for token: {mint}");
            action.fail_validation(&error).await;
            return Err(Error::NoOpenPosition {
                mint: mint.to_owned(),
            });
        }
    };

    // Validate percentage range (even for force operations)
    if !exit_percentage.is_finite() || exit_percentage <= 0.0 || exit_percentage > 100.0 {
        let error = format!(
            "Invalid exit percentage: {}. Must be in range (0, 100]",
            exit_percentage
        );
        action.fail_validation(&error).await;
        return Err(positions::Error::InvalidExitPercentage {
            percent: exit_percentage,
            reason: "must be in range (0, 100]".to_owned(),
        }
        .into());
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
        price_native: None,
        size_native: None,
        exit_percentage: Some(exit_percentage),
        // Manual trade: honour the user's slippage override (None = config).
        slippage_pct,
    };

    // Execute trade (includes quote + swap)
    let result = match crate::swaps::with_swap_stage_listener(
        action.swap_stage_listener(),
        executors::execute_trade(&decision),
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            crate::trader::actions::fail_from_error(&action, &e).await;
            return Err(e);
        }
    };

    // Check if trade succeeded
    if !result.success {
        let error = result.error.as_deref().unwrap_or("Trade failed");
        crate::trader::actions::fail_at_step(&action, result.failed_step, error).await;
        return Ok(result);
    }

    // Mark quote and swap as complete
    action.complete_quote(None).await;
    action.start_swap().await;

    if let Some(ref sig) = result.tx_signature {
        action.complete_swap(sig, result.executed_size_native).await;
        action.await_verification(Some(sig)).await;
    } else {
        action.complete_swap("unknown", None).await;
        action.await_verification(None).await;
    }

    // Record manual trade
    if let Err(e) = super::tracking::record_manual_trade(&result).await {
        logger::warning(
            LogTag::Trader,
            &format!("Failed to record manual trade: {e}"),
        );
    }

    Ok(result)
}
