//! Exit evaluation coordinator with priority-based checks and AI analysis
//!
//! Evaluates whether an exit should be made for a position by checking in priority order,
//! split into two groups:
//!
//! Safety (1-2, always run, regardless of who owns this position's exits):
//! 1. Blacklist (emergency - sync)
//! 2. Risk limits (>90% loss - emergency)
//!
//! Policy (3-8, only when the auto-trader owns this position's exits):
//! 3. AI exit analysis (high priority - if enabled)
//! 4. Stop loss (high priority - fixed threshold from entry)
//! 5. Trailing stop (high priority - from peak)
//! 6. ROI target (normal priority)
//! 7. Time override (normal priority)
//! 8. Strategy exit (normal priority)

use crate::positions::price_resolution::get_price_with_api_fallback;
use crate::positions::Position;
use crate::trader::policy::ExitPolicy;
use crate::trader::types::TradeDecision;
use crate::trader::{ai_analysis, evaluators, safety};

/// Force stop, the price cascade and the fresh position re-read — everything every exit
/// rule needs before any of them can run. `None` means nothing is evaluable this tick.
///
/// Force stop lives here on purpose: it is the global halt, so no caller can reach an
/// exit rule without passing it, whichever of the two evaluators below it chooses.
async fn resolve_evaluation_input(position: &Position) -> Option<(Position, f64)> {
    // Early exit: Force stop is active (even exits are halted during force stop)
    if crate::global::is_force_stopped() {
        return None;
    }

    // Get current price with fallback cascade:
    // 1. Pool price (real-time on-chain) — preferred
    // 2. API price from token database (DexScreener/GeckoTerminal) — if fresh
    // 3. Force-fetch fresh API price if stale
    // This matches the price resolution used by position open/close/DCA operations.
    let current_price = match get_price_with_api_fallback(&position.mint).await {
        Some((price_result, _source)) => {
            if price_result.price_sol > 0.0 && price_result.price_sol.is_finite() {
                // Apply pool price bias correction (BUG-31: DAMM pools underestimate ~5-6%)
                crate::positions::price_resolution::apply_pool_bias_correction(
                    price_result.price_sol,
                    position.entry_price,
                    position.effective_entry_price,
                )
            } else {
                return None; // Invalid price
            }
        }
        None => return None, // No price data from any source
    };

    // Get fresh position with updated price_highest for accurate trailing stop calculation
    let fresh_position = match crate::positions::get_position_by_mint(&position.mint).await {
        Some(pos) => pos,
        None => return None, // Position disappeared
    };

    Some((fresh_position, current_price))
}

/// Priorities 1-2: blacklist and the >90% loss risk limit. These are SAFETY, not policy:
/// the token turned out to be a rug, or the position is already destroyed. They apply to
/// every position the bot is permitted to act on, whatever owns its exits.
pub(crate) async fn evaluate_safety_exit(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Priority 1: Blacklist (emergency - token-level DB check only)
    if let Some(decision) = safety::check_blacklist_exit(position, current_price).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "Token {} blacklisted! Emergency exit signal",
                position.symbol
            ),
        );
        return Ok(Some(decision));
    }

    // Priority 2: Risk limits (>90% loss - emergency)
    if let Some(decision) = safety::check_risk_limits(position, current_price).await? {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "Risk limit triggered for {} (>90% loss)! Emergency exit signal",
                position.symbol
            ),
        );
        return Ok(Some(decision));
    }

    Ok(None)
}

/// Priorities 3-8: AI, stop loss, trailing stop, ROI target, time override, strategy exit.
/// These are POLICY — opinions about when to take a trade off — and only run when the
/// auto-trader owns this position's exits.
pub(crate) async fn evaluate_policy_exit(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    let policy = ExitPolicy::from_config();

    // Priority 3: AI exit analysis (high priority - if enabled)
    if ai_analysis::should_analyze_exit() {
        // Get token data for AI analysis
        match crate::tokens::get_full_token_async(&position.mint).await {
            Ok(Some(token)) => {
                match ai_analysis::analyze_exit(position, &token).await {
                    Some(result) => {
                        if result.action == ai_analysis::ExitAction::Exit {
                            crate::logger::info(
                                crate::logger::LogTag::Trader,
                                &format!(
                                    "AI suggests exit for {} (confidence: {}%, urgency: {:?}, reason: {})",
                                    position.symbol,
                                    result.confidence,
                                    result.urgency,
                                    result.reasoning
                                ),
                            );

                            // Create exit decision
                            let trade_decision = TradeDecision {
                                position_id: position.id.map(|id| id.to_string()),
                                mint: position.mint.clone(),
                                action: crate::trader::types::TradeAction::Sell,
                                reason: crate::trader::types::TradeReason::AiExit,
                                strategy_id: Some("ai_exit".to_owned()),
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
                                size_sol: None,
                                exit_percentage: None, // Full exit
                                // Auto-trader slippage always follows config.
                                slippage_pct: None,
                            };

                            // Record AI exit signal event
                            crate::events::record_trader_event(
                                "exit_signal_ai",
                                crate::events::Severity::Info,
                                Some(&position.mint),
                                None,
                                serde_json::json!({
                                    "exit_type": "ai_exit",
                                    "mint": position.mint,
                                    "symbol": position.symbol,
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
                                    position.symbol, result.confidence, result.reasoning
                                ),
                            );
                        }
                    }
                    None => {
                        // AI analysis failed or is unavailable
                        crate::logger::debug(
                            crate::logger::LogTag::Trader,
                            &format!("AI exit analysis unavailable for {}", position.symbol),
                        );
                    }
                }
            }
            Ok(None) => {
                crate::logger::debug(
                    crate::logger::LogTag::Trader,
                    &format!(
                        "Token data not found for AI exit analysis: {}",
                        position.mint
                    ),
                );
            }
            Err(e) => {
                crate::logger::warning(
                    crate::logger::LogTag::Trader,
                    &format!("Failed to fetch token data for AI exit analysis: {e}"),
                );
            }
        }
    }

    // Priority 4: Stop loss (high priority - fixed threshold from entry)
    match evaluators::exit_stop_loss::check_stop_loss(position, current_price, &policy.stop_loss)
        .await
    {
        Ok(Some(decision)) => {
            // Log already done in check_stop_loss with full context

            // Record stop loss exit signal event
            crate::events::record_trader_event(
        "exit_signal_stop_loss",
        crate::events::Severity::Warn,
        Some(&position.mint),
        None,
        serde_json::json!({
          "exit_type": "stop_loss",
          "mint": position.mint,
          "symbol": position.symbol,
          "entry_price": position.average_entry_price,
          "current_price": current_price,
          "loss_pct": ((position.average_entry_price - current_price) / position.average_entry_price) * 100.0,
        }),
      )
      .await;

            return Ok(Some(decision));
        }
        Ok(None) => {}
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!("Error checking stop loss for {}: {}", position.symbol, e),
            );
        }
    }

    // Priority 5: Trailing stop (high priority)
    match evaluators::exit_trailing::check_trailing_stop(position, current_price, &policy.trailing)
        .await
    {
        Ok(Some(decision)) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("Trailing stop triggered for {}", position.symbol),
            );

            // Record exit signal event
            crate::events::record_trader_event(
                "exit_signal_trailing_stop",
                crate::events::Severity::Info,
                Some(&position.mint),
                None,
                serde_json::json!({
                  "exit_type": "trailing_stop",
                  "mint": position.mint,
                  "symbol": position.symbol,
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
                    position.symbol, e
                ),
            );
        }
    }

    // Priority 6: ROI target (normal priority)
    match evaluators::exit_roi::check_roi_exit(position, current_price, &policy.roi).await {
        Ok(Some(decision)) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("ROI target reached for {}", position.symbol),
            );

            // Record ROI exit signal event
            crate::events::record_trader_event(
                "exit_signal_roi_target",
                crate::events::Severity::Info,
                Some(&position.mint),
                None,
                serde_json::json!({
                  "exit_type": "roi_target",
                  "mint": position.mint,
                  "symbol": position.symbol,
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
                &format!("Error checking ROI exit for {}: {}", position.symbol, e),
            );
        }
    }

    // Priority 7: Time override (normal priority)
    match evaluators::exit_time::check_time_override(position, current_price, &policy.time).await {
        Ok(Some(decision)) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("Time override triggered for {}", position.symbol),
            );

            // Record time override exit signal event
            crate::events::record_trader_event(
                "exit_signal_time_override",
                crate::events::Severity::Info,
                Some(&position.mint),
                None,
                serde_json::json!({
                  "exit_type": "time_override",
                  "mint": position.mint,
                  "symbol": position.symbol,
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
                    position.symbol, e
                ),
            );
        }
    }

    // Priority 8: Strategy exit (normal priority)
    match evaluators::StrategyEvaluator::check_exit_strategies(position, current_price).await {
        Ok(Some(decision)) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Strategy exit signal for {} (strategy: {:?})",
                    position.symbol, decision.strategy_id
                ),
            );

            // Record strategy exit signal event
            crate::events::record_trader_event(
                "exit_signal_strategy",
                crate::events::Severity::Info,
                Some(&position.mint),
                None,
                serde_json::json!({
                  "exit_type": "strategy",
                  "mint": position.mint,
                  "symbol": position.symbol,
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
                    position.symbol, e
                ),
            );
        }
    }

    // No exit signals
    Ok(None)
}

/// Evaluate exit opportunity for a position
///
/// Checks exit conditions in priority order. First matching condition returns immediately.
///
/// Resolves the current price and a fresh position read (see [`resolve_evaluation_input`]),
/// then runs safety checks (priorities 1-2, see [`evaluate_safety_exit`]) followed by policy
/// checks (priorities 3-8, see [`evaluate_policy_exit`]).
///
/// Returns:
/// - Ok(Some(TradeDecision)) if exit should be made
/// - Ok(None) if no exit signal
/// - Err(String) if evaluation failed
pub async fn evaluate_exit_for_position(
    position: Position,
) -> Result<Option<TradeDecision>, String> {
    let Some((fresh_position, current_price)) = resolve_evaluation_input(&position).await else {
        return Ok(None);
    };
    if let Some(decision) = evaluate_safety_exit(&fresh_position, current_price).await? {
        return Ok(Some(decision));
    }
    evaluate_policy_exit(&fresh_position, current_price).await
}
