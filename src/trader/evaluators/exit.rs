//! Exit evaluation coordinator with priority-based checks and AI analysis
//!
//! Evaluates whether an exit should be made for a position by checking in priority order:
//! 1. Blacklist (emergency - sync)
//! 2. Risk limits (>90% loss - emergency)
//! 3. AI exit analysis (high priority - if enabled)
//! 4. Stop loss (high priority - fixed threshold from entry)
//! 5. Trailing stop (high priority - from peak)
//! 6. ROI target (normal priority)
//! 7. Time override (normal priority)
//! 8. Strategy exit (normal priority)

use crate::pools;
use crate::positions::Position;
use crate::trader::types::TradeDecision;
use crate::trader::{ai_analysis, evaluators, safety};

/// Evaluate exit opportunity for a position
///
/// Checks exit conditions in priority order. First matching condition returns immediately.
///
/// Priority order (matching current implementation + AI + risk check):
/// 1. **Blacklist** (emergency - sync): Token blacklisted → immediate exit
/// 2. **Risk limits** (emergency): >90% loss → emergency exit
/// 3. **AI exit analysis** (high priority): AI suggests exit → prioritized exit
/// 4. **Stop loss** (high priority): Fixed threshold from entry price
/// 5. **Trailing stop** (high priority): Price dropped from peak by threshold
/// 6. **ROI target** (normal): Target profit reached
/// 7. **Time override** (normal): Position held too long
/// 8. **Strategy exit** (normal): Strategy signals exit
///
/// Returns:
/// - Ok(Some(TradeDecision)) if exit should be made
/// - Ok(None) if no exit signal
/// - Err(String) if evaluation failed
pub async fn evaluate_exit_for_position(
    position: Position,
) -> Result<Option<TradeDecision>, String> {
    // Early exit: Force stop is active (even exits are halted during force stop)
    if crate::global::is_force_stopped() {
        return Ok(None);
    }

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

    // Priority 3: AI exit analysis (high priority - if enabled)
    if ai_analysis::should_analyze_exit() {
        // Get token data for AI analysis
        match crate::tokens::get_full_token_async(&fresh_position.mint).await {
            Ok(Some(token)) => {
                match ai_analysis::analyze_exit(&fresh_position, &token).await {
                    Some(result) => {
                        if result.action == ai_analysis::ExitAction::Exit {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "AI suggests exit for {} (confidence: {}%, urgency: {:?}, reason: {})",
                                    fresh_position.symbol,
                                    result.confidence,
                                    result.urgency,
                                    result.reasoning
                                ),
                            );

                            // Create exit decision
                            let trade_decision = TradeDecision {
                                position_id: fresh_position.id.map(|id| id.to_string()),
                                mint: fresh_position.mint.clone(),
                                action: crate::trader::types::TradeAction::Sell,
                                reason: crate::trader::types::TradeReason::AiExit,
                                strategy_id: Some("ai_exit".to_string()),
                                timestamp: chrono::Utc::now(),
                                priority: match result.urgency {
                                    ai_analysis::ExitUrgency::Immediate => {
                                        crate::trader::types::TradePriority::Emergency
                                    }
                                    ai_analysis::ExitUrgency::High => {
                                        crate::trader::types::TradePriority::High
                                    }
                                    _ => crate::trader::types::TradePriority::Normal,
                                },
                                price_sol: Some(current_price),
                                size_sol: None, // Let executor determine size
                            };

                            // Record AI exit signal event
                            crate::events::record_trader_event(
                                "exit_signal_ai",
                                crate::events::Severity::Info,
                                Some(&fresh_position.mint),
                                None,
                                serde_json::json!({
                                    "exit_type": "ai_exit",
                                    "mint": fresh_position.mint,
                                    "symbol": fresh_position.symbol,
                                    "current_price": current_price,
                                    "confidence": result.confidence,
                                    "urgency": format!("{:?}", result.urgency),
                                    "reasoning": result.reasoning,
                                    "provider": result.provider,
                                }),
                            )
                            .await;

                            return Ok(Some(trade_decision));
                        } else if result.action == ai_analysis::ExitAction::Hold {
                            crate::logger::debug(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "AI suggests holding {} (confidence: {}%, reason: {})",
                                    fresh_position.symbol, result.confidence, result.reasoning
                                ),
                            );
                        }
                    }
                    None => {
                        // AI analysis failed or is unavailable
                        crate::logger::debug(
                            crate::logger::LogTag::Trader,
                            &format!("AI exit analysis unavailable for {}", fresh_position.symbol),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!(
                        "Token data not found for AI exit analysis: {}",
                        fresh_position.mint
                    ),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for AI exit analysis: {}", e),
                );
            }
        }
    }

    // Priority 4: Stop loss (high priority - fixed threshold from entry)
    match evaluators::exit_stop_loss::check_stop_loss(&fresh_position, current_price).await {
        Ok(Some(decision)) => {
            // Log already done in check_stop_loss with full context

            // Record stop loss exit signal event
            crate::events::record_trader_event(
        "exit_signal_stop_loss",
        crate::events::Severity::Warn,
        Some(&fresh_position.mint),
        None,
        serde_json::json!({
          "exit_type": "stop_loss",
          "mint": fresh_position.mint,
          "symbol": fresh_position.symbol,
          "entry_price": fresh_position.average_entry_price,
          "current_price": current_price,
          "loss_pct": ((fresh_position.average_entry_price - current_price) / fresh_position.average_entry_price) * 100.0,
        }),
      )
      .await;

            return Ok(Some(decision));
        }
        Ok(None) => {}
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Error checking stop loss for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Priority 5: Trailing stop (high priority)
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
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Error checking trailing stop for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Priority 6: ROI target (normal priority)
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
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Error checking ROI exit for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Priority 7: Time override (normal priority)
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
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Error checking time override for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Priority 8: Strategy exit (normal priority)
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
            crate::logger::warning(
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
