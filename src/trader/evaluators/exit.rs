//! Exit evaluation coordinator with priority-based checks
//!
//! Evaluates whether an exit should be made for a position by checking in priority order:
//! 1. Blacklist (emergency - sync)
//! 2. Risk limits (>90% loss - emergency)
//! 3. Trailing stop (high priority)
//! 4. ROI target (normal priority)
//! 5. Time override (normal priority)
//! 6. Strategy exit (normal priority)

use crate::pools;
use crate::positions::Position;
use crate::trader::types::TradeDecision;
use crate::trader::{evaluators, safety};

/// Evaluate exit opportunity for a position
///
/// Checks exit conditions in priority order. First matching condition returns immediately.
///
/// Priority order (matching current implementation + risk check):
/// 1. **Blacklist** (emergency - sync): Token blacklisted → immediate exit
/// 2. **Risk limits** (emergency): >90% loss → emergency exit
/// 3. **Trailing stop** (high priority): Price dropped from peak by threshold
/// 4. **ROI target** (normal): Target profit reached
/// 5. **Time override** (normal): Position held too long
/// 6. **Strategy exit** (normal): Strategy signals exit
///
/// Returns:
/// - Ok(Some(TradeDecision)) if exit should be made
/// - Ok(None) if no exit signal
/// - Err(String) if evaluation failed
pub async fn evaluate_exit_for_position(
  position: Position,
) -> Result<Option<TradeDecision>, String> {
  // Get current price
  let current_price = match pools::get_pool_price(&position.mint) {
    Some(price_info) => {
      if price_info.price_sol > 0.0 && price_info.price_sol.is_finite() {
        price_info.price_sol
      } else {
        return Ok(None); // Invalid price
      }
    }
    None => return Ok(None), // No price data
  };

  // Get fresh position with updated price_highest for accurate trailing stop calculation
  let fresh_position = match crate::positions::get_position_by_mint(&position.mint).await {
    Some(pos) => pos,
    None => return Ok(None), // Position disappeared
  };

  // Priority 1: Blacklist (emergency - sync check)
  if let Some(decision) = safety::check_blacklist_exit(&fresh_position, current_price) {
    crate::logger::info(
      crate::logger::LogTag::Trader,
      &format!(
 "Token {} blacklisted! Emergency exit signal",
        fresh_position.symbol
      ),
    );
    return Ok(Some(decision));
  }

  // Priority 2: Risk limits (>90% loss - emergency)
  if let Some(decision) = safety::check_risk_limits(&fresh_position, current_price).await? {
    crate::logger::info(
      crate::logger::LogTag::Trader,
      &format!(
 "Risk limit triggered for {} (>90% loss)! Emergency exit signal",
        fresh_position.symbol
      ),
    );
    return Ok(Some(decision));
  }

  // Priority 3: Trailing stop (high priority)
  match evaluators::exit_trailing::check_trailing_stop(&fresh_position, current_price).await {
    Ok(Some(decision)) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
 &format!("Trailing stop triggered for {}", fresh_position.symbol),
      );

      // Record exit signal event
      crate::events::record_trader_event(
        "exit_signal_trailing_stop",
        crate::events::Severity::Info,
        Some(&fresh_position.mint),
        None,
        serde_json::json!({
          "exit_type": "trailing_stop",
          "mint": fresh_position.mint,
          "symbol": fresh_position.symbol,
          "current_price": current_price,
        }),
      )
      .await;

      return Ok(Some(decision));
    }
    Ok(None) => {}
    Err(e) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
          "Error checking trailing stop for {}: {}",
          fresh_position.symbol, e
        ),
      );
    }
  }

  // Priority 4: ROI target (normal priority)
  match evaluators::exit_roi::check_roi_exit(&fresh_position, current_price).await {
    Ok(Some(decision)) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
 &format!("ROI target reached for {}", fresh_position.symbol),
      );

      // Record ROI exit signal event
      crate::events::record_trader_event(
        "exit_signal_roi_target",
        crate::events::Severity::Info,
        Some(&fresh_position.mint),
        None,
        serde_json::json!({
          "exit_type": "roi_target",
          "mint": fresh_position.mint,
          "symbol": fresh_position.symbol,
          "current_price": current_price,
        }),
      )
      .await;

      return Ok(Some(decision));
    }
    Ok(None) => {}
    Err(e) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
          "Error checking ROI exit for {}: {}",
          fresh_position.symbol, e
        ),
      );
    }
  }

  // Priority 5: Time override (normal priority)
  match evaluators::exit_time::check_time_override(&fresh_position, current_price).await {
    Ok(Some(decision)) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
 &format!("Time override triggered for {}", fresh_position.symbol),
      );

      // Record time override exit signal event
      crate::events::record_trader_event(
        "exit_signal_time_override",
        crate::events::Severity::Info,
        Some(&fresh_position.mint),
        None,
        serde_json::json!({
          "exit_type": "time_override",
          "mint": fresh_position.mint,
          "symbol": fresh_position.symbol,
          "current_price": current_price,
        }),
      )
      .await;

      return Ok(Some(decision));
    }
    Ok(None) => {}
    Err(e) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
          "Error checking time override for {}: {}",
          fresh_position.symbol, e
        ),
      );
    }
  }

  // Priority 6: Strategy exit (normal priority)
  match evaluators::StrategyEvaluator::check_exit_strategies(&fresh_position, current_price).await
  {
    Ok(Some(decision)) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
 "Strategy exit signal for {} (strategy: {:?})",
          fresh_position.symbol, decision.strategy_id
        ),
      );

      // Record strategy exit signal event
      crate::events::record_trader_event(
        "exit_signal_strategy",
        crate::events::Severity::Info,
        Some(&fresh_position.mint),
        None,
        serde_json::json!({
          "exit_type": "strategy",
          "mint": fresh_position.mint,
          "symbol": fresh_position.symbol,
          "strategy_id": decision.strategy_id,
          "current_price": current_price,
        }),
      )
      .await;

      return Ok(Some(decision));
    }
    Ok(None) => {}
    Err(e) => {
      crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
          "Error checking strategy exit for {}: {}",
          fresh_position.symbol, e
        ),
      );
    }
  }

  // No exit signals
  Ok(None)
}
